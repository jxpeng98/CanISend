use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, ApprovalBinding, ApprovalDisposition, ApprovalKind, ApprovalScope,
    ApprovalSourceVersion, WorkflowBeginRequest, WorkflowCompleteRequest, WorkflowControlReadModel,
    WorkflowRerunPreview, WorkflowRerunRequest, approval_disposition_for_application_error,
};
use canisend_contracts::{ExecutionMode, TaskStateData, WorkflowStage};
use serde::{Deserialize, Serialize};

use crate::{
    approval::{DesktopApprovalStore, DesktopPendingApproval, lease_fields},
    commands::{ApplicationWorkerError, DesktopCommandError, run_application_worker, run_worker},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowRerunPreviewReadModel {
    preview_token: String,
    expires_at_unix_ms: u64,
    remaining_ttl_seconds: u64,
    preview: ActionReceipt<WorkflowRerunPreview>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceJobRequest {
    workspace: PathBuf,
    job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowBeginTransport {
    workspace: PathBuf,
    job_id: String,
    stage: WorkflowStage,
    mode: ExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowCompleteTransport {
    workspace: PathBuf,
    job_id: String,
    stage: WorkflowStage,
    artifact_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowRerunTransport {
    workspace: PathBuf,
    job_id: String,
    stage: WorkflowStage,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewTokenRequest {
    workspace: PathBuf,
    preview_token: String,
}

#[tauri::command]
pub(crate) async fn start_workflow(
    request: WorkspaceJobRequest,
) -> Result<ActionReceipt<canisend_contracts::WorkflowStatusData>, DesktopCommandError> {
    run_worker(move || {
        Application::start_workflow(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn workflow_controls(
    request: WorkspaceJobRequest,
) -> Result<ActionReceipt<WorkflowControlReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::workflow_controls(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn begin_workflow_stage(
    request: WorkflowBeginTransport,
) -> Result<ActionReceipt<WorkflowControlReadModel>, DesktopCommandError> {
    run_worker(move || {
        let application_request =
            WorkflowBeginRequest::try_new(&request.job_id, request.stage, request.mode)
                .map_err(DesktopCommandError::application)?;
        Application::begin_workflow_stage(&request.workspace, application_request)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn complete_workflow_stage(
    request: WorkflowCompleteTransport,
) -> Result<ActionReceipt<WorkflowControlReadModel>, DesktopCommandError> {
    run_worker(move || {
        let application_request =
            WorkflowCompleteRequest::try_new(&request.job_id, request.stage, &request.artifact_id)
                .map_err(DesktopCommandError::application)?;
        Application::complete_workflow_stage(&request.workspace, application_request)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn preview_workflow_rerun(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: WorkflowRerunTransport,
) -> Result<WorkflowRerunPreviewReadModel, DesktopCommandError> {
    let workspace = request.workspace;
    let job_id = request.job_id;
    let stage = request.stage;
    let preview_workspace = workspace.clone();
    let preview_job = job_id.clone();
    let (preview, job_revision) = run_worker(move || {
        Application::preview_workflow_rerun_with_revision(&preview_workspace, &preview_job, stage)
            .map_err(DesktopCommandError::application)
    })
    .await?;
    let application_request =
        WorkflowRerunRequest::try_new(&job_id, stage).map_err(DesktopCommandError::application)?;
    let scope =
        ApprovalScope::for_workspace(&workspace).map_err(DesktopCommandError::application)?;
    let binding = ApprovalBinding::new(
        ApprovalKind::WorkflowRerun,
        scope,
        Some(job_id.clone()),
        ApprovalSourceVersion::Revision(job_revision),
    );
    let (preview_token, expires_at_unix_ms, remaining_ttl_seconds) = lease_fields(state.insert(
        binding,
        DesktopPendingApproval::WorkflowRerun {
            workspace,
            request: application_request,
        },
    )?);
    Ok(WorkflowRerunPreviewReadModel {
        preview_token,
        expires_at_unix_ms,
        remaining_ttl_seconds,
        preview,
    })
}

#[tauri::command]
pub(crate) async fn commit_workflow_rerun(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: PreviewTokenRequest,
) -> Result<ActionReceipt<WorkflowControlReadModel>, DesktopCommandError> {
    let scope = ApprovalScope::for_workspace(&request.workspace)
        .map_err(DesktopCommandError::application)?;
    let grant = state.take(&request.preview_token, ApprovalKind::WorkflowRerun, &scope)?;
    let ApprovalSourceVersion::Revision(expected_job_revision) = grant.binding().source else {
        state.resolve(grant, ApprovalDisposition::Consume)?;
        return Err(DesktopCommandError::state(
            "Workflow rerun approval is missing its source revision.",
        ));
    };
    let DesktopPendingApproval::WorkflowRerun {
        workspace,
        request: application_request,
    } = grant.payload().clone()
    else {
        state.resolve(grant, ApprovalDisposition::Consume)?;
        return Err(DesktopCommandError::state(
            "Approval payload does not match workflow rerun.",
        ));
    };
    match run_application_worker(move || {
        Application::rerun_workflow_stage_at_revision(
            &workspace,
            application_request,
            expected_job_revision,
        )
    })
    .await
    {
        Ok(receipt) => {
            state.resolve(grant, ApprovalDisposition::Consume)?;
            Ok(receipt)
        }
        Err(ApplicationWorkerError::Application(error)) => {
            let disposition = approval_disposition_for_application_error(&error);
            state.resolve(grant, disposition)?;
            Err(DesktopCommandError::application(error))
        }
        Err(ApplicationWorkerError::Worker(message)) => {
            state.resolve(grant, ApprovalDisposition::Consume)?;
            Err(DesktopCommandError::worker(message))
        }
    }
}

#[tauri::command]
pub(crate) fn discard_workflow_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: PreviewTokenRequest,
) -> Result<(), DesktopCommandError> {
    let scope = ApprovalScope::for_workspace(&request.workspace)
        .map_err(DesktopCommandError::application)?;
    state.discard(&request.preview_token, ApprovalKind::WorkflowRerun, &scope)
}

#[tauri::command]
pub(crate) async fn latest_task(
    request: WorkspaceJobRequest,
) -> Result<ActionReceipt<Option<TaskStateData>>, DesktopCommandError> {
    run_worker(move || {
        Application::latest_task_for_job(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use canisend_app::Application;

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-desktop-workflow-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn workflow_rerun_uses_the_shared_context_bound_broker() {
        let (workspace, job_id) = {
            let workspace = temporary_root("shared-broker");
            Application::initialize_workspace(&workspace).expect("initialize workspace");
            let job = Application::create_job(&workspace, "Lecturer", "University")
                .expect("create job")
                .data;
            (workspace, job.id.to_string())
        };
        let revision = Application::job_detail(&workspace, &job_id)
            .expect("job")
            .data
            .job
            .revision;
        let scope = ApprovalScope::for_workspace(&workspace).expect("approval scope");
        let store = DesktopApprovalStore::default();
        let workflow_request =
            WorkflowRerunRequest::try_new(&job_id, WorkflowStage::Parse).expect("rerun request");
        let workflow_lease = store
            .insert(
                ApprovalBinding::new(
                    ApprovalKind::WorkflowRerun,
                    scope.clone(),
                    Some(job_id.clone()),
                    ApprovalSourceVersion::Revision(revision),
                ),
                DesktopPendingApproval::WorkflowRerun {
                    workspace: workspace.clone(),
                    request: workflow_request,
                },
            )
            .expect("workflow approval");
        let workflow = store
            .take(&workflow_lease.token, ApprovalKind::WorkflowRerun, &scope)
            .expect("take workflow approval");
        assert!(matches!(
            workflow.payload(),
            DesktopPendingApproval::WorkflowRerun { .. }
        ));
        store
            .resolve(workflow, ApprovalDisposition::Consume)
            .expect("consume workflow approval");

        std::fs::remove_dir_all(workspace).expect("remove workspace");
    }

    #[test]
    fn workflow_commands_share_the_application_facade() {
        let workspace = temporary_root("workspace");
        Application::initialize_workspace(&workspace).expect("initialize workspace");
        let job = Application::create_job(&workspace, "Lecturer", "University")
            .expect("create job")
            .data;
        let started =
            Application::start_workflow(&workspace, job.id.as_str()).expect("start workflow");
        assert_eq!(started.operation, "workflow.start");
        let controls =
            Application::workflow_controls(&workspace, job.id.as_str()).expect("workflow controls");
        assert_eq!(controls.data.status.job_id, job.id);
        assert_eq!(
            controls.data.stage_descriptors.len(),
            WorkflowStage::ALL.len()
        );
        std::fs::remove_dir_all(workspace).expect("remove workspace");
    }
}
