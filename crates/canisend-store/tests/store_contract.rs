use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, EntityId, ExpectedInputRevision,
    PrivacyClassification, Revision, SafeRelativePath, Sha256Digest, SourceKind,
    TaskCompletionRequest, TaskStatus,
};
use canisend_store::{
    ArtifactService, DEFAULT_MAX_BLOB_BYTES, JobService, NewSource, StoreError, TaskService,
    Workspace, WorkspacePaths, verify_backup,
};
use serde_json::json;

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
        4
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
fn jobs_and_local_sources_are_revisioned_without_identity_merging() {
    let root = TestDirectory::new("job-intake");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let (job, first, second) = {
        let mut jobs = JobService::new(&mut workspace.database, &workspace.blobs);
        let job = jobs
            .create(" Lecturer in Economics ", " University X ", ActorKind::User)
            .expect("job create");
        assert_eq!(job.title, "Lecturer in Economics");
        assert_eq!(job.revision.get(), 1);

        let source = || NewSource {
            kind: SourceKind::LocalFile,
            original_bytes: b"Title  \r\nBody\r\n".to_vec(),
            normalized_text: "Title\nBody\n".to_owned(),
            source_url: None,
            final_url: None,
            content_type: "text/markdown; charset=utf-8".to_owned(),
            redirect_chain: Vec::new(),
            privacy: PrivacyClassification::PrivateLocal,
        };
        let first = jobs
            .import_source(&job.id, source(), ActorKind::User)
            .expect("first source");
        let second = jobs
            .import_source(&job.id, source(), ActorKind::User)
            .expect("second source");
        assert_ne!(first.id, second.id);
        assert_ne!(first.original.id, second.original.id);
        assert_eq!(first.original.sha256, second.original.sha256);
        let current = jobs.get(&job.id).expect("updated job");
        assert_eq!(current.revision.get(), 3);
        assert_eq!(current.source_ids.len(), 2);
        assert_eq!(jobs.sources(&job.id).expect("sources").len(), 2);
        (job, first, second)
    };

    {
        let artifacts = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        );
        assert_eq!(
            artifacts
                .read(&first.original.id, first.original.revision)
                .expect("original bytes"),
            b"Title  \r\nBody\r\n"
        );
        let normalized = first.normalized_text.expect("normalized reference");
        assert_eq!(
            artifacts
                .read(&normalized.id, normalized.revision)
                .expect("normalized bytes"),
            b"Title\nBody\n"
        );
    }

    let mut jobs = JobService::new(&mut workspace.database, &workspace.blobs);
    let archived = jobs.archive(&job.id, ActorKind::User).expect("archive job");
    assert!(archived.archived);
    assert!(jobs.list(false).expect("active jobs").is_empty());
    assert_eq!(jobs.list(true).expect("all jobs").len(), 1);
    assert!(matches!(
        jobs.import_source(
            &job.id,
            NewSource {
                kind: SourceKind::LocalFile,
                original_bytes: b"later".to_vec(),
                normalized_text: "later\n".to_owned(),
                source_url: None,
                final_url: None,
                content_type: "text/plain; charset=utf-8".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        ),
        Err(StoreError::JobArchived(_))
    ));
    assert_ne!(first.id, second.id);
}

#[test]
fn agent_tasks_validate_commit_idempotently_and_detect_changed_jobs() {
    let root = TestDirectory::new("agent-task");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let job = JobService::new(&mut workspace.database, &workspace.blobs)
        .create("Lecturer in Economics", "University X", ActorKind::User)
        .expect("job");
    let source = || NewSource {
        kind: SourceKind::LocalFile,
        original_bytes: b"Teach economics".to_vec(),
        normalized_text: "Teach economics\n".to_owned(),
        source_url: None,
        final_url: None,
        content_type: "text/plain; charset=utf-8".to_owned(),
        redirect_chain: Vec::new(),
        privacy: PrivacyClassification::PrivateLocal,
    };
    JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(&job.id, source(), ActorKind::User)
        .expect("source");
    let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_job_criterion(&job.id)
        .expect("prepared task");
    assert_eq!(descriptor.input_artifacts.len(), 1);
    assert_eq!(descriptor.private_read_scope, descriptor.input_artifacts);
    let export_directory = workspace.paths.root.join("agent/task-inputs");
    let exported = TaskService::new(&mut workspace.database, &workspace.blobs)
        .export_inputs(&descriptor.id, &export_directory)
        .expect("scoped input export");
    assert_eq!(exported.files.len(), 1);
    assert_eq!(
        fs::read(export_directory.join(exported.files[0].relative_path.as_str()))
            .expect("exported body"),
        b"Teach economics\n"
    );
    assert!(export_directory.join("canisend-task-inputs.json").is_file());
    assert!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .export_inputs(
                &descriptor.id,
                &workspace.paths.root.join(".canisend/forbidden")
            )
            .is_err()
    );
    let candidate = json!({
        "id": "019f2f55-7c00-7000-8000-000000000201",
        "job_id": job.id,
        "kind": "teaching",
        "requirement": "Evidence of university-level teaching",
        "importance": "essential",
        "source_quote": "Teach economics",
        "revision": 1
    });
    let request = TaskCompletionRequest {
        task_id: descriptor.id.clone(),
        lease_id: descriptor.lease.id.clone(),
        expected_job_revision: descriptor.job_revision,
        expected_inputs: descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate: candidate.clone(),
    };
    let mut invalid = request.clone();
    invalid.candidate = json!({"requirement": 3});
    assert!(matches!(
        TaskService::new(&mut workspace.database, &workspace.blobs).complete(&invalid),
        Err(StoreError::CandidateStructural(_))
    ));
    assert_eq!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .get(&descriptor.id)
            .expect("task state")
            .status,
        TaskStatus::Prepared
    );
    let committed = TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&request)
        .expect("commit");
    assert!(!committed.idempotent);
    let replay = TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&request)
        .expect("idempotent replay");
    assert!(replay.idempotent);
    assert_eq!(replay.artifact, committed.artifact);

    let stale_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_job_criterion(&job.id)
        .expect("second task");
    JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(&job.id, source(), ActorKind::User)
        .expect("changed job inputs");
    let stale_request = TaskCompletionRequest {
        task_id: stale_descriptor.id.clone(),
        lease_id: stale_descriptor.lease.id.clone(),
        expected_job_revision: stale_descriptor.job_revision,
        expected_inputs: stale_descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate,
    };
    assert!(matches!(
        TaskService::new(&mut workspace.database, &workspace.blobs).complete(&stale_request),
        Err(StoreError::TaskStale(_))
    ));
    assert_eq!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .get(&stale_descriptor.id)
            .expect("stale state")
            .status,
        TaskStatus::Stale
    );

    let cancelled = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_job_criterion(&job.id)
        .expect("third task");
    let state = TaskService::new(&mut workspace.database, &workspace.blobs)
        .cancel(&cancelled.id)
        .expect("cancel");
    assert_eq!(state.status, TaskStatus::Cancelled);
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
