use std::path::Path;

use canisend_contracts::WorkflowStatusData;
use canisend_store::WorkflowService;

use crate::{
    ActionReceipt, Application, ApplicationError,
    application::{open_workspace, parse_entity_id},
};

impl Application {
    pub fn start_workflow(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<WorkflowStatusData>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).start(&job_id)?;
        let next_actions = status.next_actions.clone();
        Ok(ActionReceipt::new(
            "workflow.start",
            "started",
            "Workflow is ready for its next action",
            status,
        )
        .with_next_actions(next_actions))
    }

    pub fn workflow_status(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<WorkflowStatusData>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).status(&job_id)?;
        let next_actions = status.next_actions.clone();
        Ok(ActionReceipt::new(
            "workflow.status",
            "available",
            format!("{} blocker(s)", status.blockers.len()),
            status,
        )
        .with_next_actions(next_actions))
    }
}
