use std::path::Path;

use canisend_contracts::{ApplicationId, ApplicationModelSnapshotV3, Revision};
use canisend_store::ApplicationModelRepository;
use serde::{Deserialize, Serialize};

use crate::{ActionReceipt, Application, ApplicationError, application::open_workspace};

pub use canisend_store::{
    ApplicationModelCommitResultV3, ApplicationModelRevisionV3, StoredApplicationModelV3,
    WorkspaceV3AuthorityState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationModelCreateRequestV3 {
    pub snapshot: ApplicationModelSnapshotV3,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationModelCommitRequestV3 {
    pub expected_revision: Revision,
    pub snapshot: ApplicationModelSnapshotV3,
    pub reason: String,
}

impl Application {
    pub fn application_model_authority_v3(
        workspace_root: &Path,
    ) -> Result<ActionReceipt<WorkspaceV3AuthorityState>, ApplicationError> {
        let mut workspace = open_workspace(workspace_root)?;
        let authority = ApplicationModelRepository::new(&mut workspace.database).authority()?;
        Ok(ActionReceipt::new(
            "application-v3.authority",
            "current",
            "Loaded the Workspace v3 authority boundary",
            authority,
        ))
    }

    pub fn create_application_model_v3(
        workspace_root: &Path,
        request: ApplicationModelCreateRequestV3,
    ) -> Result<ActionReceipt<ApplicationModelCommitResultV3>, ApplicationError> {
        let mut workspace = open_workspace(workspace_root)?;
        let result = ApplicationModelRepository::new(&mut workspace.database).create(
            request.snapshot,
            canisend_contracts::ActorKind::User,
            &request.reason,
        )?;
        Ok(ActionReceipt::new(
            "application-v3.create",
            "created",
            "Created a revision-bound neutral Application model",
            result,
        ))
    }

    pub fn application_model_v3(
        workspace_root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let mut workspace = open_workspace(workspace_root)?;
        let stored =
            ApplicationModelRepository::new(&mut workspace.database).get(&application_id)?;
        Ok(ActionReceipt::new(
            "application-v3.show",
            "current",
            "Loaded the current neutral Application model",
            stored,
        ))
    }

    pub fn list_application_models_v3(
        workspace_root: &Path,
    ) -> Result<ActionReceipt<Vec<StoredApplicationModelV3>>, ApplicationError> {
        let mut workspace = open_workspace(workspace_root)?;
        let applications = ApplicationModelRepository::new(&mut workspace.database).list()?;
        Ok(ActionReceipt::new(
            "application-v3.list",
            "current",
            format!("Loaded {} neutral Application model(s)", applications.len()),
            applications,
        ))
    }

    pub fn application_model_history_v3(
        workspace_root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<Vec<ApplicationModelRevisionV3>>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let mut workspace = open_workspace(workspace_root)?;
        let revisions =
            ApplicationModelRepository::new(&mut workspace.database).history(&application_id)?;
        Ok(ActionReceipt::new(
            "application-v3.history",
            "current",
            format!("Loaded {} Application revision(s)", revisions.len()),
            revisions,
        ))
    }

    pub fn commit_application_model_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationModelCommitRequestV3,
    ) -> Result<ActionReceipt<ApplicationModelCommitResultV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let mut workspace = open_workspace(workspace_root)?;
        let result = ApplicationModelRepository::new(&mut workspace.database).commit(
            &application_id,
            request.expected_revision,
            request.snapshot,
            canisend_contracts::ActorKind::User,
            &request.reason,
        )?;
        Ok(ActionReceipt::new(
            "application-v3.commit",
            "committed",
            "Committed the next neutral Application revision",
            result,
        ))
    }
}

fn parse_application_id(value: &str) -> Result<ApplicationId, ApplicationError> {
    ApplicationId::try_new(value)
        .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-v3-authority-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn current_v2_workspace_exposes_no_mixed_v3_authority() {
        let root = temporary_root();
        Application::initialize_workspace(&root).expect("initialize v2 workspace");
        let error = Application::list_application_models_v3(&root)
            .expect_err("v2 workspace must fail closed");
        assert!(matches!(
            error,
            ApplicationError::Store(canisend_store::StoreError::ApplicationModelUnavailable)
        ));
        fs::remove_dir_all(root).expect("remove workspace");
    }
}
