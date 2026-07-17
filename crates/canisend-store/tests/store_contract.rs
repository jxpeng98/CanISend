use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, EntityId, Revision, SafeRelativePath, Sha256Digest,
};
use canisend_store::{
    ArtifactService, DEFAULT_MAX_BLOB_BYTES, StoreError, Workspace, WorkspacePaths, verify_backup,
};

static NEXT: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "canisend-store-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove stale test directory");
        }
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn workspace_init_discovery_status_and_check_are_consistent() {
    let root = TestDirectory::new("workspace");
    let workspace = Workspace::init(root.path()).expect("workspace initializes");
    let nested = root.path().join("jobs/example/workspace");
    fs::create_dir_all(&nested).expect("nested directory");
    let discovered = WorkspacePaths::discover(None, &nested).expect("workspace discovery");
    assert_eq!(discovered.root, root.path());
    assert_eq!(
        workspace.status().expect("status").database_schema_version,
        1
    );
    let check = workspace.check().expect("workspace check");
    assert!(check.ok);
    assert_eq!(check.database_integrity, "ok");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(root.path().join(".canisend"))
                .expect("internal metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(root.path().join("canisend.toml"))
                .expect("config metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn blobs_are_bounded_immutable_verified_and_auditable() {
    let root = TestDirectory::new("blob");
    let workspace = Workspace::init(root.path()).expect("workspace");
    let digest = workspace
        .blobs
        .put_bytes(b"evidence")
        .expect("blob publish");
    assert_eq!(
        workspace
            .blobs
            .read_verified(&digest, DEFAULT_MAX_BLOB_BYTES)
            .expect("verified read"),
        b"evidence"
    );
    let check = workspace.check().expect("check");
    assert_eq!(check.unreferenced_blobs, vec![digest.clone()]);
    assert!(Sha256Digest::try_new("../../escape").is_err());

    let destination = workspace.blobs.path_for(&digest);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&destination)
                .expect("blob metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    fs::write(&destination, b"collision").expect("replace with collision fixture");
    assert!(matches!(
        workspace.blobs.put_bytes(b"evidence"),
        Err(StoreError::BlobCollision(_))
    ));

    let mut reader = FailingReader { emitted: false };
    assert!(
        workspace
            .blobs
            .put_reader(&mut reader, DEFAULT_MAX_BLOB_BYTES)
            .is_err()
    );
    assert_eq!(
        fs::read_dir(root.path().join(".canisend/tmp"))
            .expect("temporary directory")
            .count(),
        0
    );
}

#[cfg(unix)]
#[test]
fn workspace_and_blob_symlinks_fail_closed() {
    use std::os::unix::fs::symlink;

    let root = TestDirectory::new("symlink");
    let workspace = Workspace::init(root.path()).expect("workspace");
    let digest = workspace.blobs.put_bytes(b"private").expect("blob");
    let blob_path = workspace.blobs.path_for(&digest);
    fs::remove_file(&blob_path).expect("remove blob");
    symlink("/tmp", &blob_path).expect("blob symlink");
    assert!(
        workspace
            .blobs
            .verify(&digest, DEFAULT_MAX_BLOB_BYTES)
            .is_err()
    );

    fs::remove_file(&blob_path).expect("remove blob symlink");
    fs::remove_dir_all(&workspace.paths.blob_container).expect("remove blob container");
    symlink("/tmp", &workspace.paths.blob_container).expect("blob container symlink");
    assert!(
        workspace
            .blobs
            .put_bytes(b"must not escape the workspace")
            .is_err()
    );
    fs::remove_file(&workspace.paths.blob_container).expect("remove blob container symlink");

    let internal = root.path().join(".canisend");
    fs::remove_dir_all(&internal).expect("remove internal");
    symlink("/tmp", &internal).expect("internal symlink");
    assert!(Workspace::open_from(Some(root.path()), root.path()).is_err());
}

#[test]
fn artifact_commit_stales_dependents_and_projection_repairs() {
    let root = TestDirectory::new("artifact");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let (source, derived) = {
        let mut service = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        );
        let source = service
            .commit(
                None,
                ArtifactKind::SourceNormalizedText,
                b"source v1",
                &[],
                ActorKind::User,
                "import source",
            )
            .expect("source commit");
        let source_reference = service
            .reference(&source.artifact_id)
            .expect("source reference");
        let derived = service
            .commit(
                None,
                ArtifactKind::CoverLetter,
                b"derived v1",
                &[source_reference],
                ActorKind::HostAgent,
                "draft from evidence",
            )
            .expect("derived commit");

        let missing_dependency = ArtifactReference {
            kind: ArtifactKind::EvidenceCatalog,
            id: EntityId::try_new("019f2f55-7c00-7000-8000-000000009999").expect("fixture id"),
            revision: Revision::try_new(1).expect("fixture revision"),
            sha256: Sha256Digest::try_new("a".repeat(64)).expect("fixture digest"),
        };
        assert!(matches!(
            service.commit(
                None,
                ArtifactKind::CoverLetter,
                b"published before rejected transaction",
                &[missing_dependency],
                ActorKind::HostAgent,
                "exercise transaction rollback",
            ),
            Err(StoreError::DependencyConflict(_))
        ));

        service
            .commit(
                Some(source.artifact_id.clone()),
                ArtifactKind::SourceNormalizedText,
                b"source v2",
                &[],
                ActorKind::User,
                "correct source",
            )
            .expect("source update");

        let collision = root.path().join("jobs/example");
        fs::write(&collision, b"not a directory").expect("projection collision");
        assert!(
            service
                .project(
                    &derived.artifact_id,
                    derived.revision,
                    &SafeRelativePath::try_new("jobs/example/cover-letter.md")
                        .expect("projection path"),
                )
                .is_err()
        );
        assert_eq!(
            service
                .read(&derived.artifact_id, derived.revision)
                .expect("authoritative artifact survives projection failure"),
            b"derived v1"
        );
        fs::remove_file(&collision).expect("remove collision");
        assert_eq!(service.repair_projections().expect("repair projection"), 1);
        (source, derived)
    };
    let check = workspace.check().expect("workspace check");
    assert!(check.stale_artifact_ids.contains(&derived.artifact_id));
    assert!(check.projection_repairs_required.is_empty());
    assert_eq!(check.unreferenced_blobs.len(), 1);
    assert_eq!(
        fs::read(root.path().join("jobs/example/cover-letter.md")).expect("projection"),
        b"derived v1"
    );
    assert!(!check.unreferenced_blobs.contains(&source.sha256));
}

#[test]
fn verified_backup_restores_into_new_workspace() {
    let root = TestDirectory::new("backup-source");
    let backup = TestDirectory::new("backup-destination");
    let restore = TestDirectory::new("restore-destination");
    let backup_path = backup.path().join("snapshot");
    let restore_path = restore.path().join("workspace");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    {
        let mut service = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        );
        service
            .commit(
                None,
                ArtifactKind::EvidenceCatalog,
                b"private evidence",
                &[],
                ActorKind::User,
                "import evidence",
            )
            .expect("artifact commit");
    }
    let result = workspace.backup(&backup_path).expect("backup");
    assert_eq!(result.manifest.blobs.len(), 1);
    verify_backup(&backup_path).expect("backup verifies");
    let restored = Workspace::restore(&backup_path, &restore_path).expect("restore");
    assert_eq!(restored.config.workspace_id, workspace.config.workspace_id);
    assert!(restored.check().expect("restored check").ok);
}

struct FailingReader {
    emitted: bool,
}

impl Read for FailingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.emitted {
            return Err(io::Error::other("synthetic interruption"));
        }
        self.emitted = true;
        buffer[..4].copy_from_slice(b"part");
        Ok(4)
    }
}
