use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use canisend_app::{
    ActionReceipt, Application, PrivateReadConsent, ProviderSendConsent,
    TaskCompletionPreviewReadModel, TaskExecutionMode, TaskInputExportRequest,
    TaskPrepareAgainReadModel, TaskPrepareRequest, WorkflowBeginRequest, WorkflowCompleteRequest,
    WorkflowControlReadModel, WorkflowRerunPreview, WorkflowRerunRequest,
};
use canisend_contracts::{
    ExecutionMode, TaskCommitData, TaskCompletionRequest, TaskDescriptor, TaskInputExportData,
    TaskStateData, WorkflowStage,
};
use serde::{Deserialize, Serialize};

use crate::commands::{DesktopCommandError, run_worker};

const MAX_PENDING_PREVIEWS: usize = 8;
static NEXT_PREVIEW: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
enum PendingOperation {
    WorkflowRerun {
        workspace: PathBuf,
        request: WorkflowRerunRequest,
    },
    TaskCompletion {
        workspace: PathBuf,
        request: TaskCompletionRequest,
    },
}

#[derive(Debug, Default)]
struct PendingOperationState {
    operations: BTreeMap<String, PendingOperation>,
    order: VecDeque<String>,
}

#[derive(Debug, Default)]
pub(crate) struct WorkflowPreviewStore {
    state: Mutex<PendingOperationState>,
}

impl WorkflowPreviewStore {
    fn insert(&self, operation: PendingOperation) -> Result<String, DesktopCommandError> {
        let token = operation_token()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Workflow preview state is unavailable"))?;
        while state.operations.len() >= MAX_PENDING_PREVIEWS {
            let Some(oldest) = state.order.pop_front() else {
                break;
            };
            state.operations.remove(&oldest);
        }
        state.order.push_back(token.clone());
        state.operations.insert(token.clone(), operation);
        Ok(token)
    }

    fn take(&self, token: &str) -> Result<PendingOperation, DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Workflow preview state is unavailable"))?;
        let operation = state.operations.remove(token).ok_or_else(|| {
            DesktopCommandError::state(
                "The reviewed operation preview expired; preview the operation again.",
            )
        })?;
        state.order.retain(|existing| existing != token);
        Ok(operation)
    }

    fn restore(
        &self,
        token: String,
        operation: PendingOperation,
    ) -> Result<(), DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Workflow preview state is unavailable"))?;
        state.order.retain(|existing| existing != &token);
        state.order.push_back(token.clone());
        state.operations.insert(token, operation);
        Ok(())
    }

    fn discard(&self, token: &str) -> Result<(), DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Workflow preview state is unavailable"))?;
        state.operations.remove(token);
        state.order.retain(|existing| existing != token);
        Ok(())
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.state
            .lock()
            .expect("workflow preview lock")
            .operations
            .len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowRerunPreviewReadModel {
    preview_token: String,
    preview: ActionReceipt<WorkflowRerunPreview>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskCompletionTokenReadModel {
    preview_token: String,
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

fn operation_token() -> Result<String, DesktopCommandError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DesktopCommandError::state("System clock is before the Unix epoch"))?
        .as_millis();
    let sequence = NEXT_PREVIEW.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "workflow-preview-{}-{timestamp}-{sequence}",
        std::process::id()
    ))
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn preview_workflow_rerun(
    state: tauri::State<'_, WorkflowPreviewStore>,
    request: WorkflowRerunTransport,
) -> Result<WorkflowRerunPreviewReadModel, DesktopCommandError> {
    let workspace = request.workspace;
    let job_id = request.job_id;
    let stage = request.stage;
    let preview_workspace = workspace.clone();
    let preview_job = job_id.clone();
    let preview = run_worker(move || {
        Application::preview_workflow_rerun(&preview_workspace, &preview_job, stage)
            .map_err(DesktopCommandError::application)
    })
    .await?;
    let application_request =
        WorkflowRerunRequest::try_new(&job_id, stage).map_err(DesktopCommandError::application)?;
    let preview_token = state.insert(PendingOperation::WorkflowRerun {
        workspace,
        request: application_request,
    })?;
    Ok(WorkflowRerunPreviewReadModel {
        preview_token,
        preview,
    })
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn commit_workflow_rerun(
    state: tauri::State<'_, WorkflowPreviewStore>,
    request: PreviewTokenRequest,
) -> Result<ActionReceipt<WorkflowControlReadModel>, DesktopCommandError> {
    let pending = state.take(&request.preview_token)?;
    let retry = pending.clone();
    let result = match pending {
        PendingOperation::WorkflowRerun { workspace, request } => {
            run_worker(move || {
                Application::rerun_workflow_stage(&workspace, request)
                    .map_err(DesktopCommandError::application)
            })
            .await
        }
        PendingOperation::TaskCompletion { .. } => Err(DesktopCommandError::state(
            "The preview token belongs to a task completion.",
        )),
    };
    if result.is_err() {
        state.restore(request.preview_token, retry)?;
    }
    result
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) fn discard_workflow_preview(
    state: tauri::State<'_, WorkflowPreviewStore>,
    request: PreviewTokenRequest,
) -> Result<(), DesktopCommandError> {
    state.discard(&request.preview_token)
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn export_task_inputs(
    request: TaskExportTransport,
) -> Result<ActionReceipt<TaskInputExportData>, DesktopCommandError> {
    run_worker(move || task_export_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn preview_task_completion(
    state: tauri::State<'_, WorkflowPreviewStore>,
    request: TaskCompletionPreviewTransport,
) -> Result<TaskCompletionTokenReadModel, DesktopCommandError> {
    let workspace = request.workspace.clone();
    let preview = run_worker(move || task_completion_preview_impl(request)).await?;
    let preview_token = state.insert(PendingOperation::TaskCompletion {
        workspace,
        request: preview.data.request.clone(),
    })?;
    Ok(TaskCompletionTokenReadModel {
        preview_token,
        preview,
    })
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn commit_task_completion_preview(
    state: tauri::State<'_, WorkflowPreviewStore>,
    request: PreviewTokenRequest,
) -> Result<ActionReceipt<TaskCommitData>, DesktopCommandError> {
    let pending = state.take(&request.preview_token)?;
    let retry = pending.clone();
    let result = match pending {
        PendingOperation::TaskCompletion { workspace, request } => {
            run_worker(move || {
                Application::commit_task_completion(&workspace, request)
                    .map_err(DesktopCommandError::application)
            })
            .await
        }
        PendingOperation::WorkflowRerun { .. } => Err(DesktopCommandError::state(
            "The preview token belongs to a workflow rerun.",
        )),
    };
    if result.is_err() {
        state.restore(request.preview_token, retry)?;
    }
    result
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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
    fn workflow_preview_store_is_bounded_and_single_use() {
        let store = WorkflowPreviewStore::default();
        let request = WorkflowRerunRequest::try_new(
            "019f2f55-7c00-7000-8000-000000000101",
            WorkflowStage::Parse,
        )
        .expect("workflow request");
        let mut latest = String::new();
        for _ in 0..(MAX_PENDING_PREVIEWS + 2) {
            latest = store
                .insert(PendingOperation::WorkflowRerun {
                    workspace: PathBuf::from("/tmp/workspace"),
                    request: request.clone(),
                })
                .expect("store preview");
        }
        assert_eq!(store.len(), MAX_PENDING_PREVIEWS);
        assert!(store.take(&latest).is_ok());
        assert!(store.take(&latest).is_err());
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
