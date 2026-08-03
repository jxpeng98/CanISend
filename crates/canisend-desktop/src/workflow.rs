use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, ApprovalBinding, ApprovalDisposition, ApprovalKind, ApprovalScope,
    ApprovalSourceVersion, PrivateReadConsent, ProviderSendConsent, TaskCompletionPreviewReadModel,
    TaskExecutionMode, TaskInputExportRequest, TaskPrepareAgainReadModel, TaskPrepareRequest,
    WorkflowBeginRequest, WorkflowCompleteRequest, WorkflowControlReadModel, WorkflowRerunPreview,
    WorkflowRerunRequest, approval_disposition_for_application_error,
};
use canisend_contracts::{
    ExecutionMode, TaskCommitData, TaskDescriptor, TaskInputExportData, TaskStateData,
    WorkflowStage,
};
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskCompletionTokenReadModel {
    preview_token: String,
    expires_at_unix_ms: u64,
    remaining_ttl_seconds: u64,
    preview: ActionReceipt<TaskCompletionPreviewReadModel>,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskPrepareTransport {
    workspace: PathBuf,
    job_id: String,
    operation: canisend_app::TaskOperation,
    mode: TaskExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskExportTransport {
    workspace: PathBuf,
    task_id: String,
    destination: PathBuf,
    confirmed_private_read: bool,
    confirmed_provider_send: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskCompletionPreviewTransport {
    workspace: PathBuf,
    file: PathBuf,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskRequest {
    workspace: PathBuf,
    task_id: String,
}

fn task_export_impl(
    request: TaskExportTransport,
) -> Result<ActionReceipt<TaskInputExportData>, DesktopCommandError> {
    let private_read = request
        .confirmed_private_read
        .then(PrivateReadConsent::granted_by_user);
    if private_read.is_none() {
        return Err(DesktopCommandError::consent(
            "Confirm private task input access before exporting task inputs.",
        ));
    }
    let provider_send = request
        .confirmed_provider_send
        .then(ProviderSendConsent::granted_by_user);
    let application_request =
        TaskInputExportRequest::try_new(&request.task_id, request.destination)
            .map_err(DesktopCommandError::application)?;
    Application::export_task_inputs(
        &request.workspace,
        application_request,
        private_read,
        provider_send,
    )
    .map_err(DesktopCommandError::application)
}

fn task_completion_preview_impl(
    request: TaskCompletionPreviewTransport,
) -> Result<ActionReceipt<TaskCompletionPreviewReadModel>, DesktopCommandError> {
    if !request.confirmed_private_read {
        return Err(DesktopCommandError::consent(
            "Confirm private file access before previewing the task completion.",
        ));
    }
    Application::preview_task_completion_file(
        &request.workspace,
        &request.file,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)
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

#[tauri::command]
pub(crate) async fn prepare_task(
    request: TaskPrepareTransport,
) -> Result<ActionReceipt<TaskDescriptor>, DesktopCommandError> {
    run_worker(move || {
        let application_request =
            TaskPrepareRequest::try_new(&request.job_id, request.operation, request.mode)
                .map_err(DesktopCommandError::application)?;
        Application::prepare_task(&request.workspace, application_request)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn export_task_inputs(
    request: TaskExportTransport,
) -> Result<ActionReceipt<TaskInputExportData>, DesktopCommandError> {
    run_worker(move || task_export_impl(request)).await
}

#[tauri::command]
pub(crate) async fn preview_task_completion(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: TaskCompletionPreviewTransport,
) -> Result<TaskCompletionTokenReadModel, DesktopCommandError> {
    let workspace = request.workspace.clone();
    let preview = run_worker(move || task_completion_preview_impl(request)).await?;
    let scope =
        ApprovalScope::for_workspace(&workspace).map_err(DesktopCommandError::application)?;
    let binding = ApprovalBinding::new(
        ApprovalKind::TaskCompletion,
        scope,
        Some(preview.data.state.descriptor.job_id.to_string()),
        ApprovalSourceVersion::Revision(preview.data.request.expected_job_revision),
    );
    let (preview_token, expires_at_unix_ms, remaining_ttl_seconds) = lease_fields(state.insert(
        binding,
        DesktopPendingApproval::TaskCompletion {
            workspace,
            request: preview.data.request.clone(),
        },
    )?);
    Ok(TaskCompletionTokenReadModel {
        preview_token,
        expires_at_unix_ms,
        remaining_ttl_seconds,
        preview,
    })
}

#[tauri::command]
pub(crate) async fn commit_task_completion_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: PreviewTokenRequest,
) -> Result<ActionReceipt<TaskCommitData>, DesktopCommandError> {
    let scope = ApprovalScope::for_workspace(&request.workspace)
        .map_err(DesktopCommandError::application)?;
    let grant = state.take(&request.preview_token, ApprovalKind::TaskCompletion, &scope)?;
    let DesktopPendingApproval::TaskCompletion {
        workspace,
        request: application_request,
    } = grant.payload().clone()
    else {
        state.resolve(grant, ApprovalDisposition::Consume)?;
        return Err(DesktopCommandError::state(
            "Approval payload does not match task completion.",
        ));
    };
    match run_application_worker(move || {
        Application::commit_task_completion(&workspace, application_request)
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
pub(crate) async fn cancel_task(
    request: TaskRequest,
) -> Result<ActionReceipt<TaskStateData>, DesktopCommandError> {
    run_worker(move || {
        Application::cancel_task(&request.workspace, &request.task_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn prepare_task_again(
    request: TaskRequest,
) -> Result<ActionReceipt<TaskPrepareAgainReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::prepare_task_again(&request.workspace, &request.task_id)
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
    fn workflow_and_task_families_share_one_context_bound_broker() {
        let (workspace, job_id, source) = {
            let workspace = temporary_root("shared-broker");
            let source = temporary_root("shared-source").with_extension("txt");
            std::fs::write(&source, "Lecturer job fixture").expect("write source");
            Application::initialize_workspace(&workspace).expect("initialize workspace");
            let job = Application::create_job(&workspace, "Lecturer", "University")
                .expect("create job")
                .data;
            (workspace, job.id.to_string(), source)
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

        let task_request = canisend_contracts::TaskCompletionRequest {
            task_id: canisend_contracts::EntityId::try_new("019f2f55-7c00-7000-8000-000000000201")
                .expect("task ID"),
            lease_id: canisend_contracts::EntityId::try_new("019f2f55-7c00-7000-8000-000000000202")
                .expect("lease ID"),
            expected_job_revision: revision,
            expected_inputs: Vec::new(),
            candidate: serde_json::json!({}),
        };
        let task_lease = store
            .insert(
                ApprovalBinding::new(
                    ApprovalKind::TaskCompletion,
                    scope.clone(),
                    Some(job_id),
                    ApprovalSourceVersion::Revision(revision),
                ),
                DesktopPendingApproval::TaskCompletion {
                    workspace: workspace.clone(),
                    request: task_request,
                },
            )
            .expect("task approval");
        let task = store
            .take(&task_lease.token, ApprovalKind::TaskCompletion, &scope)
            .expect("take task approval");
        assert!(matches!(
            task.payload(),
            DesktopPendingApproval::TaskCompletion { .. }
        ));
        store
            .resolve(task, ApprovalDisposition::Consume)
            .expect("consume task approval");

        std::fs::remove_dir_all(workspace).expect("remove workspace");
        std::fs::remove_file(source).expect("remove source");
    }

    #[test]
    fn task_io_requires_consent_before_workspace_or_file_access() {
        let missing = temporary_root("missing");
        let export = task_export_impl(TaskExportTransport {
            workspace: missing.clone(),
            task_id: "not-an-id".to_owned(),
            destination: missing.join("inputs"),
            confirmed_private_read: false,
            confirmed_provider_send: false,
        })
        .expect_err("task export needs consent");
        assert_eq!(export.code, "consent-required");

        let completion = task_completion_preview_impl(TaskCompletionPreviewTransport {
            workspace: missing,
            file: PathBuf::from("/missing/completion.json"),
            confirmed_private_read: false,
        })
        .expect_err("task completion needs consent");
        assert_eq!(completion.code, "consent-required");
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
