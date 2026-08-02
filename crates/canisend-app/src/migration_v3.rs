use std::path::{Path, PathBuf};

use canisend_contracts::Sha256Digest;
use canisend_core::VerifiedWorkflowPackBundle;
use canisend_store::WorkspaceV3MigrationService;
use serde::{Deserialize, Serialize};

use crate::{ActionReceipt, Application, ApplicationError, application::open_workspace};

pub use canisend_store::{WorkspaceV3MigrationPreview, WorkspaceV3MigrationResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceV3MigrationRequest {
    pub expected_plan_sha256: Sha256Digest,
    pub backup_destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceV3MigrationReadModel {
    pub backup_destination: PathBuf,
    pub migration: WorkspaceV3MigrationResult,
}

impl Application {
    pub fn preview_workspace_v3_migration(
        workspace_root: &Path,
        academic_pack: &VerifiedWorkflowPackBundle,
    ) -> Result<ActionReceipt<WorkspaceV3MigrationPreview>, ApplicationError> {
        let mut workspace = open_workspace(workspace_root)?;
        let preview = WorkspaceV3MigrationService::new(&mut workspace).preview(academic_pack)?;
        Ok(ActionReceipt::new(
            "workspace-v3.migration-preview",
            "ready",
            format!(
                "Migration dry-run is ready for {} Application(s)",
                preview.application_count
            ),
            preview,
        ))
    }

    pub fn migrate_workspace_v3(
        workspace_root: &Path,
        academic_pack: &VerifiedWorkflowPackBundle,
        request: WorkspaceV3MigrationRequest,
    ) -> Result<ActionReceipt<WorkspaceV3MigrationReadModel>, ApplicationError> {
        let mut workspace = open_workspace(workspace_root)?;
        let migration = WorkspaceV3MigrationService::new(&mut workspace).migrate(
            academic_pack,
            &request.expected_plan_sha256,
            &request.backup_destination,
        )?;
        Ok(ActionReceipt::new(
            "workspace-v3.migrate",
            "migrated",
            format!(
                "Migrated {} Application(s) with a verified backup",
                migration.application_ids.len()
            ),
            WorkspaceV3MigrationReadModel {
                backup_destination: request.backup_destination,
                migration,
            },
        ))
    }
}
