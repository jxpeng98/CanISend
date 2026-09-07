use std::path::Path;

use canisend_contracts::{EntityId, LocalTaskV4, NextAction, Revision};
use canisend_store::LocalTaskServiceV4;
use serde_json::Value;

use crate::application::{open_workspace_v4, parse_entity_id};
use crate::{ActionReceipt, Application, ApplicationError, PrivateReadConsent};

impl Application {
    pub fn list_local_tasks_v4(
        root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<Vec<LocalTaskV4>>, ApplicationError> {
        let id = canisend_contracts::ApplicationId::try_new(application_id)
            .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))?;
        let mut workspace = open_workspace_v4(root)?;
        let tasks = LocalTaskServiceV4::new(&mut workspace.database, &workspace.blobs).list(&id)?;
        Ok(ActionReceipt::new(
            "local-task.list",
            "current",
            "Loaded up to 100 recent local tasks without candidate bodies",
            tasks,
        ))
    }
    pub fn prepare_local_task_v4(
        root: &Path,
        application_id: &str,
        expected_revision: Revision,
    ) -> Result<ActionReceipt<LocalTaskV4>, ApplicationError> {
        Self::application_pack_manifest_v4(root, application_id)?;
        let id = canisend_contracts::ApplicationId::try_new(application_id)
            .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))?;
        let mut workspace = open_workspace_v4(root)?;
        let task = LocalTaskServiceV4::new(&mut workspace.database, &workspace.blobs)
            .prepare(&id, expected_revision)?;
        Ok(local_task_receipt("local-task.prepare", task))
    }

    pub fn show_local_task_v4(
        root: &Path,
        task_id: &str,
    ) -> Result<ActionReceipt<LocalTaskV4>, ApplicationError> {
        let id = parse_entity_id(task_id)?;
        let mut workspace = open_workspace_v4(root)?;
        let task = LocalTaskServiceV4::new(&mut workspace.database, &workspace.blobs).show(&id)?;
        Ok(local_task_receipt("local-task.show", task))
    }

    pub fn claim_local_task_v4(
        root: &Path,
        task_id: &str,
        generation: u64,
    ) -> Result<ActionReceipt<LocalTaskV4>, ApplicationError> {
        let id = parse_entity_id(task_id)?;
        let mut workspace = open_workspace_v4(root)?;
        let task = LocalTaskServiceV4::new(&mut workspace.database, &workspace.blobs)
            .claim(&id, generation)?;
        Ok(local_task_receipt("local-task.claim", task))
    }

    pub fn submit_local_task_v4(
        root: &Path,
        task_id: &str,
        generation: u64,
        lease_id: &str,
        candidate_path: &Path,
    ) -> Result<ActionReceipt<LocalTaskV4>, ApplicationError> {
        let id = parse_entity_id(task_id)?;
        let lease = EntityId::try_new(lease_id)
            .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))?;
        let candidate = canisend_io::read_structured_candidate_file(candidate_path)?;
        let mut workspace = open_workspace_v4(root)?;
        let task = LocalTaskServiceV4::new(&mut workspace.database, &workspace.blobs)
            .submit(&id, generation, &lease, &candidate)?;
        Ok(local_task_receipt("local-task.submit", task))
    }

    pub fn cancel_local_task_v4(
        root: &Path,
        task_id: &str,
        generation: u64,
        lease_id: &str,
    ) -> Result<ActionReceipt<LocalTaskV4>, ApplicationError> {
        let id = parse_entity_id(task_id)?;
        let lease = parse_entity_id(lease_id)?;
        let mut workspace = open_workspace_v4(root)?;
        let task = LocalTaskServiceV4::new(&mut workspace.database, &workspace.blobs)
            .cancel(&id, generation, &lease)?;
        Ok(local_task_receipt("local-task.cancel", task))
    }

    pub fn local_task_candidate_v4(
        root: &Path,
        task_id: &str,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<Value>, ApplicationError> {
        if consent.is_none() {
            return Err(ApplicationError::ConsentRequired {
                message: "Reading a local task candidate requires explicit private-read consent"
                    .to_owned(),
                remediation: NextAction {
                    action: "review-private-access".to_owned(),
                    description: "Explicitly authorize private local candidate access.".to_owned(),
                },
            });
        }
        let id = parse_entity_id(task_id)?;
        let mut workspace = open_workspace_v4(root)?;
        let candidate =
            LocalTaskServiceV4::new(&mut workspace.database, &workspace.blobs).candidate(&id)?;
        Ok(ActionReceipt::new(
            "local-task.candidate.show",
            "current",
            "Loaded an untrusted local candidate; no Application mutation was authorized",
            candidate,
        ))
    }
}

fn local_task_receipt(operation: &'static str, task: LocalTaskV4) -> ActionReceipt<LocalTaskV4> {
    ActionReceipt::new(
        operation,
        "current",
        "Local coordination only; Application authority is unchanged",
        task,
    )
}
