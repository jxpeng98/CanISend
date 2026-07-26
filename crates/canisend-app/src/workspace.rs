use std::{
    fs,
    path::{Path, PathBuf},
};

use canisend_contracts::{BackupManifestData, WorkspaceCheckData, WorkspaceStatusData};
use canisend_store::{BACKUP_FORMAT, BackupResult, Workspace};
use serde::{Deserialize, Serialize};

use crate::{ActionReceipt, Application, ApplicationError, application::open_workspace};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceReadModel {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceInitPolicy {
    NewOrEmpty,
    PreserveExistingFiles,
}

impl Application {
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
                status,
            },
        ))
    }

    pub fn workspace_status(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceReadModel>, ApplicationError> {
        let workspace = open_workspace(root)?;
        let status = workspace.status()?;
        Ok(ActionReceipt::new(
            "workspace.status",
            "available",
            format!("Workspace has {} job(s)", status.job_count),
            WorkspaceReadModel {
                path: workspace.paths.root,
                status,
            },
        ))
    }

    pub fn check_workspace(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceHealthReadModel>, ApplicationError> {
        let workspace = open_workspace(root)?;
        let check = workspace.check()?;
        let status = if check.ok { "healthy" } else { "issues-found" };
        Ok(ActionReceipt::new(
            "workspace.check",
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

    pub fn backup_workspace(
        root: &Path,
        destination: &Path,
    ) -> Result<ActionReceipt<BackupReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let BackupResult {
            directory,
            manifest,
        } = workspace.backup(destination)?;
        Ok(ActionReceipt::new(
            "workspace.backup",
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
