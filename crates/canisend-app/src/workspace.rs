use std::{
    fs,
    path::{Path, PathBuf},
};

use canisend_contracts::ActorKind;
use canisend_contracts::{
    BackupManifestData, WORKSPACE_V4_FORMAT, WorkspaceCheckData, WorkspaceStatusData,
};
use canisend_store::{
    ApplicationModelRepository, BACKUP_FORMAT, BackupResult, ProjectionService, Workspace,
};
use serde::{Deserialize, Serialize};

use crate::{ACADEMIC_JOB_WORKFLOW_PACK_ID, GENERIC_APPLICATION_WORKFLOW_PACK_ID};
use crate::{
    ActionReceipt, Application, ApplicationError,
    application::{open_workspace, open_workspace_v4},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceReadModel {
    pub path: PathBuf,
    pub pack_id: String,
    pub status: WorkspaceStatusData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceV4ReadModel {
    pub path: PathBuf,
    pub status: WorkspaceStatusData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceHealthReadModel {
    pub path: PathBuf,
    pub check: WorkspaceCheckData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupReadModel {
    pub destination: PathBuf,
    pub format: String,
    pub blob_count: usize,
    pub manifest: BackupManifestData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRestoreReadModel {
    pub backup: PathBuf,
    pub destination: PathBuf,
    pub workspace: WorkspaceStatusData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRepairReadModel {
    pub workspace: PathBuf,
    pub repaired_projections: usize,
    pub check: WorkspaceCheckData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceInitPolicy {
    NewOrEmpty,
    PreserveExistingFiles,
}

impl Application {
    pub fn initialize_workspace_v4(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceV4ReadModel>, ApplicationError> {
        Self::initialize_workspace_v4_with_policy(root, WorkspaceInitPolicy::NewOrEmpty)
    }

    pub fn initialize_workspace_v4_with_policy(
        root: &Path,
        policy: WorkspaceInitPolicy,
    ) -> Result<ActionReceipt<WorkspaceV4ReadModel>, ApplicationError> {
        if policy == WorkspaceInitPolicy::NewOrEmpty {
            require_new_or_empty_directory(root)?;
        }
        let workspace = Workspace::init_v4(root)?;
        let status = workspace.status()?;
        Ok(ActionReceipt::new(
            "workspace.initialize.commit",
            "initialized",
            format!(
                "Initialized neutral Workspace v4 at {}",
                workspace.paths.root.display()
            ),
            WorkspaceV4ReadModel {
                path: workspace.paths.root,
                status,
            },
        ))
    }

    pub fn workspace_status_v4(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceV4ReadModel>, ApplicationError> {
        let workspace = open_workspace_v4(root)?;
        let status = workspace.status()?;
        debug_assert_eq!(status.workspace_format, WORKSPACE_V4_FORMAT);
        Ok(ActionReceipt::new(
            "workspace.status",
            "available",
            format!(
                "Workspace contains {} Application(s)",
                status.application_count
            ),
            WorkspaceV4ReadModel {
                path: workspace.paths.root,
                status,
            },
        ))
    }

    pub fn initialize_workspace_for_pack(
        root: &Path,
        pack_id: &str,
    ) -> Result<ActionReceipt<WorkspaceReadModel>, ApplicationError> {
        match pack_id {
            ACADEMIC_JOB_WORKFLOW_PACK_ID => Self::initialize_workspace(root),
            GENERIC_APPLICATION_WORKFLOW_PACK_ID => Self::initialize_workspace_v3(root),
            _ => Err(ApplicationError::InvalidInput(format!(
                "unknown built-in workflow Pack: {pack_id}"
            ))),
        }
    }

    pub fn initialize_workspace(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceReadModel>, ApplicationError> {
        Self::initialize_workspace_with_policy(root, WorkspaceInitPolicy::NewOrEmpty)
    }

    pub fn initialize_workspace_with_policy(
        root: &Path,
        policy: WorkspaceInitPolicy,
    ) -> Result<ActionReceipt<WorkspaceReadModel>, ApplicationError> {
        if policy == WorkspaceInitPolicy::NewOrEmpty {
            require_new_or_empty_directory(root)?;
        }
        let workspace = Workspace::init(root)?;
        let status = workspace.status()?;
        Ok(ActionReceipt::new(
            "workspace.init",
            "initialized",
            format!(
                "Initialized workspace at {}",
                workspace.paths.root.display()
            ),
            WorkspaceReadModel {
                path: workspace.paths.root,
                pack_id: ACADEMIC_JOB_WORKFLOW_PACK_ID.to_owned(),
                status,
            },
        ))
    }

    pub fn initialize_workspace_v3(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceReadModel>, ApplicationError> {
        require_new_or_empty_directory(root)?;
        let mut workspace = Workspace::init(root)?;
        ApplicationModelRepository::new(&mut workspace.database)
            .activate_empty_workspace(ActorKind::User, "new-workspace-v3")?;
        let status = workspace.status()?;
        Ok(ActionReceipt::new(
            "workspace-v3.init",
            "initialized",
            format!(
                "Initialized canonical v3 workspace at {}",
                workspace.paths.root.display()
            ),
            WorkspaceReadModel {
                path: workspace.paths.root,
                pack_id: GENERIC_APPLICATION_WORKFLOW_PACK_ID.to_owned(),
                status,
            },
        ))
    }

    pub fn workspace_status(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let status = workspace.status()?;
        let pack_id = if status.workspace_format == canisend_contracts::WORKSPACE_V3_FORMAT {
            let authority = ApplicationModelRepository::new(&mut workspace.database).authority()?;
            if authority.reason == "new-workspace-v3" {
                GENERIC_APPLICATION_WORKFLOW_PACK_ID
            } else {
                ACADEMIC_JOB_WORKFLOW_PACK_ID
            }
        } else {
            ACADEMIC_JOB_WORKFLOW_PACK_ID
        };
        Ok(ActionReceipt::new(
            "workspace.status",
            "available",
            format!("Workspace has {} job(s)", status.job_count),
            WorkspaceReadModel {
                path: workspace.paths.root,
                pack_id: pack_id.to_owned(),
                status,
            },
        ))
    }

    pub fn check_workspace(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceHealthReadModel>, ApplicationError> {
        let workspace = open_workspace(root)?;
        workspace_check_receipt(workspace, "workspace.check")
    }

    pub fn check_workspace_v4(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceHealthReadModel>, ApplicationError> {
        let workspace = open_workspace_v4(root)?;
        workspace_check_receipt(workspace, "workspace.check")
    }

    pub fn backup_workspace_v4(
        root: &Path,
        destination: &Path,
    ) -> Result<ActionReceipt<BackupReadModel>, ApplicationError> {
        let workspace = open_workspace_v4(root)?;
        workspace_backup_receipt(workspace, destination, "workspace.backup.commit")
    }

    pub fn restore_workspace_v4(
        backup: &Path,
        destination: &Path,
    ) -> Result<ActionReceipt<WorkspaceRestoreReadModel>, ApplicationError> {
        let workspace = Workspace::restore_v4(backup, destination)?;
        workspace_restore_receipt(workspace, backup, "workspace.restore.commit")
    }

    pub fn repair_workspace_v4(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceRepairReadModel>, ApplicationError> {
        let workspace = open_workspace_v4(root)?;
        workspace_repair_receipt(workspace, "workspace.repair.commit")
    }

    pub fn backup_workspace(
        root: &Path,
        destination: &Path,
    ) -> Result<ActionReceipt<BackupReadModel>, ApplicationError> {
        let workspace = open_workspace(root)?;
        workspace_backup_receipt(workspace, destination, "workspace.backup")
    }

    pub fn restore_workspace(
        backup: &Path,
        destination: &Path,
    ) -> Result<ActionReceipt<WorkspaceRestoreReadModel>, ApplicationError> {
        let workspace = Workspace::restore(backup, destination)?;
        workspace_restore_receipt(workspace, backup, "workspace.restore")
    }

    pub fn repair_workspace(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceRepairReadModel>, ApplicationError> {
        let workspace = open_workspace(root)?;
        workspace_repair_receipt(workspace, "workspace.repair")
    }
}

fn workspace_check_receipt(
    workspace: Workspace,
    operation: &'static str,
) -> Result<ActionReceipt<WorkspaceHealthReadModel>, ApplicationError> {
    let check = workspace.check()?;
    let status = if check.ok { "healthy" } else { "issues-found" };
    Ok(ActionReceipt::new(
        operation,
        status,
        if check.ok {
            "Workspace integrity check passed".to_owned()
        } else {
            format!("Workspace check found {} issue(s)", check.issues.len())
        },
        WorkspaceHealthReadModel {
            path: workspace.paths.root,
            check,
        },
    ))
}

fn workspace_backup_receipt(
    mut workspace: Workspace,
    destination: &Path,
    operation: &'static str,
) -> Result<ActionReceipt<BackupReadModel>, ApplicationError> {
    let BackupResult {
        directory,
        manifest,
    } = workspace.backup(destination)?;
    Ok(ActionReceipt::new(
        operation,
        "verified",
        format!("Verified backup created at {}", directory.display()),
        BackupReadModel {
            destination: directory,
            format: BACKUP_FORMAT.to_owned(),
            blob_count: manifest.blobs.len(),
            manifest,
        },
    ))
}

fn workspace_restore_receipt(
    workspace: Workspace,
    backup: &Path,
    operation: &'static str,
) -> Result<ActionReceipt<WorkspaceRestoreReadModel>, ApplicationError> {
    let status = workspace.status()?;
    let destination = workspace.paths.root.clone();
    Ok(ActionReceipt::new(
        operation,
        "restored",
        format!("Restored workspace at {}", destination.display()),
        WorkspaceRestoreReadModel {
            backup: backup.to_path_buf(),
            destination,
            workspace: status,
        },
    ))
}

fn workspace_repair_receipt(
    mut workspace: Workspace,
    operation: &'static str,
) -> Result<ActionReceipt<WorkspaceRepairReadModel>, ApplicationError> {
    let repaired_projections = ProjectionService::new(
        &mut workspace.database,
        &workspace.blobs,
        &workspace.paths.root,
    )
    .repair_all()?;
    let check = workspace.check()?;
    let path = workspace.paths.root;
    Ok(ActionReceipt::new(
        operation,
        "repaired",
        format!("Repaired {repaired_projections} managed projection(s)"),
        WorkspaceRepairReadModel {
            workspace: path,
            repaired_projections,
            check,
        },
    ))
}

fn require_new_or_empty_directory(root: &Path) -> Result<(), ApplicationError> {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(root).map_err(|error| {
        ApplicationError::InvalidInput(format!(
            "cannot inspect new workspace directory {}: {error}",
            root.display()
        ))
    })?;
    if entries.next().is_some() {
        return Err(ApplicationError::InvalidInput(format!(
            "new workspace directory must be empty: {}",
            root.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{ActorKind, ArtifactKind, SafeRelativePath};
    use canisend_store::{ApplicationModelRepository, ArtifactService, Workspace};

    use crate::{
        ACADEMIC_JOB_WORKFLOW_PACK_ID, Application, ApplicationError,
        GENERIC_APPLICATION_WORKFLOW_PACK_ID,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-recovery-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn workspace_read_model_preserves_pack_presentation_across_v3_activation_paths() {
        let generic = temporary_root("generic-pack");
        let migrated = temporary_root("migrated-pack");

        let generic_model = Application::initialize_workspace_v3(&generic)
            .expect("initialize generic v3")
            .data;
        assert_eq!(generic_model.pack_id, GENERIC_APPLICATION_WORKFLOW_PACK_ID);

        Application::initialize_workspace(&migrated).expect("initialize legacy workspace");
        {
            let mut workspace = Workspace::open(Some(&migrated)).expect("open legacy workspace");
            ApplicationModelRepository::new(&mut workspace.database)
                .activate_empty_workspace(ActorKind::User, "migrate-workspace-v2-to-v3")
                .expect("activate migrated authority fixture");
        }
        let migrated_model = Application::workspace_status(&migrated)
            .expect("migrated status")
            .data;
        assert_eq!(migrated_model.pack_id, ACADEMIC_JOB_WORKFLOW_PACK_ID);

        fs::remove_dir_all(generic).expect("remove generic fixture");
        fs::remove_dir_all(migrated).expect("remove migrated fixture");
    }

    #[test]
    fn neutral_workspace_v4_has_no_workspace_pack_selection() {
        let root = temporary_root("neutral-v4");
        let initialized = Application::initialize_workspace_v4(&root)
            .expect("initialize neutral Workspace v4")
            .data;
        assert_eq!(
            initialized.status.workspace_format,
            canisend_contracts::WORKSPACE_V4_FORMAT
        );
        assert_eq!(initialized.status.application_count, 0);

        let reopened = Application::workspace_status_v4(&root)
            .expect("reopen neutral Workspace v4")
            .data;
        assert_eq!(
            reopened.status.workspace_id,
            initialized.status.workspace_id
        );
        assert_eq!(reopened.status.application_count, 0);

        fs::remove_dir_all(root).expect("remove Workspace v4 fixture");
    }

    #[test]
    fn workspace_v4_recovery_surface_round_trips_and_rejects_legacy_backups() {
        let source = temporary_root("v4-recovery-source");
        let backup = temporary_root("v4-recovery-backup");
        let restored = temporary_root("v4-recovery-restored");
        Application::initialize_workspace_v4(&source).expect("initialize Workspace v4");

        assert!(
            Application::check_workspace_v4(&source)
                .expect("check Workspace v4")
                .data
                .check
                .ok
        );
        let backup_receipt =
            Application::backup_workspace_v4(&source, &backup).expect("backup Workspace v4");
        assert_eq!(backup_receipt.operation, "workspace.backup.commit");
        let restore_receipt =
            Application::restore_workspace_v4(&backup, &restored).expect("restore Workspace v4");
        assert_eq!(restore_receipt.operation, "workspace.restore.commit");
        assert_eq!(
            restore_receipt.data.workspace.workspace_format,
            canisend_contracts::WORKSPACE_V4_FORMAT
        );
        let repair_receipt =
            Application::repair_workspace_v4(&restored).expect("repair Workspace v4");
        assert_eq!(repair_receipt.operation, "workspace.repair.commit");
        assert!(repair_receipt.data.check.ok);

        fs::remove_dir_all(source).expect("remove source");
        fs::remove_dir_all(backup).expect("remove backup");
        fs::remove_dir_all(restored).expect("remove restored");

        let legacy = temporary_root("v4-recovery-legacy");
        let legacy_backup = temporary_root("v4-recovery-legacy-backup");
        let rejected = temporary_root("v4-recovery-rejected");
        Application::initialize_workspace(&legacy).expect("initialize legacy Workspace");
        Application::backup_workspace(&legacy, &legacy_backup).expect("backup legacy Workspace");
        let config_before = fs::read(legacy_backup.join("canisend.toml"))
            .expect("read verified legacy backup config");

        assert!(matches!(
            Application::restore_workspace_v4(&legacy_backup, &rejected),
            Err(ApplicationError::Store(
                canisend_store::StoreError::WorkspaceFormatUnsupported { .. }
            ))
        ));
        assert!(!rejected.exists());
        assert_eq!(
            fs::read(legacy_backup.join("canisend.toml")).expect("legacy backup remains readable"),
            config_before
        );

        fs::remove_dir_all(legacy).expect("remove legacy source");
        fs::remove_dir_all(legacy_backup).expect("remove legacy backup");
    }

    #[test]
    fn v4_status_reports_an_actionable_compatibility_boundary_for_legacy_workspaces() {
        let root = temporary_root("legacy-v4-boundary");
        Application::initialize_workspace(&root).expect("initialize legacy Workspace fixture");

        let error = Application::workspace_status_v4(&root)
            .expect_err("legacy Workspace must not enter the v4 surface");

        let ApplicationError::CompatibilityUnavailable {
            details,
            remediation,
            ..
        } = error
        else {
            panic!("expected compatibility-unavailable boundary");
        };
        assert_eq!(details["required"], canisend_contracts::WORKSPACE_V4_FORMAT);
        assert!(remediation.description.contains("does not open"));

        fs::remove_dir_all(root).expect("remove legacy Workspace fixture");
    }

    #[test]
    fn neutral_workspace_v4_can_preserve_user_owned_files_during_cli_initialization() {
        let root = temporary_root("neutral-v4-preserve");
        fs::create_dir_all(&root).expect("create existing project directory");
        let sentinel = root.join("keep.txt");
        fs::write(&sentinel, "user-owned").expect("write user-owned sentinel");

        let initialized = Application::initialize_workspace_v4_with_policy(
            &root,
            super::WorkspaceInitPolicy::PreserveExistingFiles,
        )
        .expect("initialize neutral Workspace v4 around existing files")
        .data;

        assert_eq!(
            initialized.status.workspace_format,
            canisend_contracts::WORKSPACE_V4_FORMAT
        );
        assert_eq!(
            fs::read_to_string(&sentinel).expect("read preserved sentinel"),
            "user-owned"
        );

        fs::remove_dir_all(root).expect("remove Workspace v4 fixture");
    }

    #[test]
    fn verified_backup_restores_atomically_and_rejects_conflicts() {
        let source = temporary_root("source");
        let backup = temporary_root("backup");
        let restored = temporary_root("restored");
        let occupied = temporary_root("occupied");
        Application::initialize_workspace(&source).expect("initialize source");
        Application::create_job(&source, "Lecturer", "University").expect("create job");
        let source_status = Application::workspace_status(&source)
            .expect("source status")
            .data
            .status;
        Application::backup_workspace(&source, &backup).expect("create backup");

        let receipt = Application::restore_workspace(&backup, &restored).expect("restore");
        assert_eq!(receipt.operation, "workspace.restore");
        assert_eq!(
            receipt.data.workspace.workspace_id,
            source_status.workspace_id
        );
        assert_eq!(receipt.data.workspace.job_count, 1);
        assert!(
            Application::check_workspace(&restored)
                .expect("restored check")
                .data
                .check
                .ok
        );

        fs::create_dir_all(&occupied).expect("create occupied destination");
        let sentinel = occupied.join("keep.txt");
        fs::write(&sentinel, "user-owned").expect("write sentinel");
        assert!(Application::restore_workspace(&backup, &occupied).is_err());
        assert_eq!(
            fs::read_to_string(&sentinel).expect("sentinel remains"),
            "user-owned"
        );

        fs::remove_dir_all(source).expect("remove source");
        fs::remove_dir_all(backup).expect("remove backup");
        fs::remove_dir_all(restored).expect("remove restored");
        fs::remove_dir_all(occupied).expect("remove occupied");
    }

    #[test]
    fn malformed_backup_fails_without_creating_destination() {
        let source = temporary_root("malformed-source");
        let backup = temporary_root("malformed-backup");
        let destination = temporary_root("malformed-destination");
        Application::initialize_workspace(&source).expect("initialize source");
        Application::backup_workspace(&source, &backup).expect("create backup");
        fs::write(backup.join("backup-manifest.json"), "{}\n").expect("corrupt manifest");

        assert!(Application::restore_workspace(&backup, &destination).is_err());
        assert!(!destination.exists());

        fs::remove_dir_all(source).expect("remove source");
        fs::remove_dir_all(backup).expect("remove backup");
    }

    #[test]
    fn projection_repair_is_effective_and_idempotent() {
        let root = temporary_root("repair");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let projection_path = root.join("jobs/example/evidence.json");
        {
            let mut workspace = Workspace::open(Some(&root)).expect("open workspace");
            let artifact = ArtifactService::new(
                &mut workspace.database,
                &workspace.blobs,
                &workspace.paths.root,
            )
            .commit(
                None,
                ArtifactKind::EvidenceCatalog,
                b"private evidence",
                &[],
                ActorKind::User,
                "test projection repair",
            )
            .expect("commit artifact");
            ArtifactService::new(
                &mut workspace.database,
                &workspace.blobs,
                &workspace.paths.root,
            )
            .project(
                &artifact.artifact_id,
                artifact.revision,
                &SafeRelativePath::try_new("jobs/example/evidence.json")
                    .expect("safe projection path"),
            )
            .expect("project artifact");
        }
        fs::remove_file(&projection_path).expect("remove managed projection");

        let repaired = Application::repair_workspace(&root).expect("repair");
        assert_eq!(repaired.data.repaired_projections, 1);
        assert!(repaired.data.check.ok);
        assert_eq!(
            fs::read(&projection_path).expect("repaired projection"),
            b"private evidence"
        );

        let repeated = Application::repair_workspace(&root).expect("repeat repair");
        assert_eq!(repeated.data.repaired_projections, 0);
        assert!(repeated.data.check.ok);

        fs::remove_dir_all(root).expect("remove workspace");
    }
}
