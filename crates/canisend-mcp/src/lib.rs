#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use canisend_app::{
    Application, ApplicationError, NetworkFetchConsent, PreparedJobSource, PrivateReadConsent,
    ProviderSendConsent, TaskExecutionMode, TaskInputExportRequest, TaskOperation,
    TaskPrepareRequest,
};
use canisend_contracts::TaskCompletionRequest;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::wrapper::{Json, Parameters},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const MAX_JOB_ID_BYTES: usize = 128;
const MAX_TASK_ID_BYTES: usize = 128;
const MAX_PREVIEW_TOKEN_BYTES: usize = 192;
const MAX_PENDING_PREVIEWS: usize = 16;
const MAX_SESSION_INPUT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOOL_RESULT_BYTES: usize = 2 * 1024 * 1024;
static NEXT_PREVIEW: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Error)]
pub enum McpServerError {
    #[error("{0}")]
    Application(#[from] ApplicationError),
    #[error("cannot start the MCP runtime: {0}")]
    Runtime(#[from] std::io::Error),
    #[error("MCP transport failed: {0}")]
    Transport(String),
}

#[derive(Debug, Clone)]
pub struct CanISendMcpServer {
    workspace: Arc<PathBuf>,
    previews: Arc<MutationPreviewStore>,
}

#[derive(Debug, Clone)]
enum PendingMutation {
    JobIntake(Box<PreparedJobSource>),
    TaskCompletion(TaskCompletionRequest),
}

#[derive(Debug, Default)]
struct PendingMutationState {
    previews: BTreeMap<String, PendingMutation>,
    order: VecDeque<String>,
}

#[derive(Debug, Default)]
struct MutationPreviewStore {
    state: Mutex<PendingMutationState>,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextParameters {
    #[schemars(description = "Optional CanISend job ID to scope the body-free context")]
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobListParameters {
    #[serde(default)]
    #[schemars(description = "Include archived jobs in the result")]
    pub include_archived: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobParameters {
    #[schemars(description = "CanISend job ID")]
    pub job_id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum JobIntakeSourceKind {
    LocalFile,
    Url,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobIntakePreviewParameters {
    #[schemars(description = "Existing CanISend job ID that will receive the reviewed source")]
    pub job_id: String,
    #[schemars(description = "Whether locator is an absolute local file path or a public URL")]
    pub source_kind: JobIntakeSourceKind,
    #[schemars(description = "Absolute local PDF/text path or public job-advert URL")]
    pub locator: String,
    #[serde(default)]
    #[schemars(description = "User explicitly confirmed private local-file access")]
    pub confirmed_private_read: bool,
    #[serde(default)]
    #[schemars(description = "User explicitly confirmed the bounded URL network request")]
    pub confirmed_network_fetch: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewTokenParameters {
    #[schemars(description = "Opaque single-use token returned by a CanISend preview tool")]
    pub preview_token: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum McpTaskOperation {
    JobParse,
    EvidenceNormalize,
    EvidenceMatch,
    CoverLetterDraft,
    ResearchStatementDraft,
    TeachingStatementDraft,
    CvDraft,
    DocumentReview,
}

impl From<McpTaskOperation> for TaskOperation {
    fn from(value: McpTaskOperation) -> Self {
        match value {
            McpTaskOperation::JobParse => Self::JobParse,
            McpTaskOperation::EvidenceNormalize => Self::EvidenceNormalize,
            McpTaskOperation::EvidenceMatch => Self::EvidenceMatch,
            McpTaskOperation::CoverLetterDraft => Self::CoverLetterDraft,
            McpTaskOperation::ResearchStatementDraft => Self::ResearchStatementDraft,
            McpTaskOperation::TeachingStatementDraft => Self::TeachingStatementDraft,
            McpTaskOperation::CvDraft => Self::CvDraft,
            McpTaskOperation::DocumentReview => Self::DocumentReview,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum McpTaskExecutionMode {
    HostAgent,
    ConfiguredProvider,
}

impl From<McpTaskExecutionMode> for TaskExecutionMode {
    fn from(value: McpTaskExecutionMode) -> Self {
        match value {
            McpTaskExecutionMode::HostAgent => Self::HostAgent,
            McpTaskExecutionMode::ConfiguredProvider => Self::ConfiguredProvider,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskPrepareParameters {
    pub job_id: String,
    pub operation: McpTaskOperation,
    pub mode: McpTaskExecutionMode,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskInputsParameters {
    pub task_id: String,
    #[schemars(description = "Absolute empty output directory for exact task inputs")]
    pub destination: String,
    #[serde(default)]
    pub confirmed_private_read: bool,
    #[serde(default)]
    pub confirmed_provider_send: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskCompletionPreviewParameters {
    #[schemars(description = "Absolute path to a canisend.task-completion/v2 JSON file")]
    pub file: String,
    #[serde(default)]
    pub confirmed_private_read: bool,
}

impl MutationPreviewStore {
    fn insert(&self, mutation: PendingMutation) -> Result<String, McpError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| McpError::internal_error("system clock is before the Unix epoch", None))?
            .as_millis();
        let sequence = NEXT_PREVIEW.fetch_add(1, Ordering::Relaxed);
        let token = format!("mcp-preview-{}-{timestamp}-{sequence}", std::process::id());
        let mut state = self
            .state
            .lock()
            .map_err(|_| McpError::internal_error("MCP preview state is unavailable", None))?;
        while state.previews.len() >= MAX_PENDING_PREVIEWS {
            let Some(oldest) = state.order.pop_front() else {
                break;
            };
            state.previews.remove(&oldest);
        }
        state.order.push_back(token.clone());
        state.previews.insert(token.clone(), mutation);
        Ok(token)
    }

    fn take(&self, token: &str) -> Result<PendingMutation, McpError> {
        CanISendMcpServer::validate_preview_token(token)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| McpError::internal_error("MCP preview state is unavailable", None))?;
        let mutation = state.previews.remove(token).ok_or_else(|| {
            McpError::invalid_params(
                "The reviewed preview is missing, expired, or already committed; prepare it again.",
                None,
            )
        })?;
        state.order.retain(|existing| existing != token);
        Ok(mutation)
    }

    fn restore(&self, token: String, mutation: PendingMutation) -> Result<(), McpError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| McpError::internal_error("MCP preview state is unavailable", None))?;
        state.order.retain(|existing| existing != &token);
        state.order.push_back(token.clone());
        state.previews.insert(token, mutation);
        Ok(())
    }
}

impl CanISendMcpServer {
    pub fn open(workspace: &Path) -> Result<Self, ApplicationError> {
        let workspace = Application::resolve_workspace_root(Some(workspace))?;
        Ok(Self {
            workspace: Arc::new(workspace),
            previews: Arc::new(MutationPreviewStore::default()),
        })
    }

    #[must_use]
    pub fn workspace(&self) -> &Path {
        self.workspace.as_path()
    }

    fn application_result<T: Serialize>(
        result: Result<T, ApplicationError>,
    ) -> Result<Json<Value>, McpError> {
        match result {
            Ok(value) => {
                let value = serde_json::to_value(value).map_err(|error| {
                    McpError::internal_error(
                        format!("failed to serialize CanISend response: {error}"),
                        None,
                    )
                })?;
                let encoded = serde_json::to_vec(&value).map_err(|error| {
                    McpError::internal_error(
                        format!("failed to bound CanISend response: {error}"),
                        None,
                    )
                })?;
                if encoded.len() > MAX_TOOL_RESULT_BYTES {
                    return Err(McpError::internal_error(
                        format!(
                            "CanISend response exceeded the {MAX_TOOL_RESULT_BYTES}-byte MCP limit"
                        ),
                        None,
                    ));
                }
                Ok(Json(value))
            }
            Err(error) => {
                let failure = error.classify();
                let data = serde_json::to_value(&failure).ok();
                Err(McpError::invalid_params(failure.message, data))
            }
        }
    }

    fn validate_job_id(job_id: &str) -> Result<(), McpError> {
        if job_id.is_empty() || job_id.len() > MAX_JOB_ID_BYTES {
            return Err(McpError::invalid_params(
                format!("job_id must contain 1 to {MAX_JOB_ID_BYTES} bytes"),
                None,
            ));
        }
        Ok(())
    }

    fn validate_task_id(task_id: &str) -> Result<(), McpError> {
        if task_id.is_empty() || task_id.len() > MAX_TASK_ID_BYTES {
            return Err(McpError::invalid_params(
                format!("task_id must contain 1 to {MAX_TASK_ID_BYTES} bytes"),
                None,
            ));
        }
        Ok(())
    }

    fn validate_preview_token(token: &str) -> Result<(), McpError> {
        if token.is_empty()
            || token.len() > MAX_PREVIEW_TOKEN_BYTES
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(McpError::invalid_params("preview_token is malformed", None));
        }
        Ok(())
    }

    fn consent_required(message: &str) -> McpError {
        McpError::invalid_params(
            message.to_owned(),
            Some(serde_json::json!({
                "status": "consent-required",
                "code": "consent.required",
                "retryable": false
            })),
        )
    }

    fn mutation_result<T: Serialize>(
        &self,
        result: Result<T, ApplicationError>,
        token: String,
        retry: PendingMutation,
    ) -> Result<Json<Value>, McpError> {
        match result {
            Ok(value) => Self::application_result(Ok(value)),
            Err(error) => {
                self.previews.restore(token, retry)?;
                Self::application_result::<T>(Err(error))
            }
        }
    }
}

pub fn serve_stdio(workspace: Option<&Path>) -> Result<(), McpServerError> {
    let workspace = Application::resolve_workspace_root(workspace)?;
    let server = CanISendMcpServer::open(&workspace)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        use tokio::io::AsyncReadExt as _;

        let input = tokio::io::stdin().take(MAX_SESSION_INPUT_BYTES);
        let service = server
            .serve((input, tokio::io::stdout()))
            .await
            .map_err(|error| McpServerError::Transport(error.to_string()))?;
        service
            .waiting()
            .await
            .map(|_| ())
            .map_err(|error| McpServerError::Transport(error.to_string()))
    })
}

#[tool_router]
impl CanISendMcpServer {
    #[tool(
        description = "Return the versioned CanISend Agent v2 capability catalog without reading a workspace",
        annotations(
            title = "Inspect CanISend capabilities",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_capabilities(&self) -> Result<Json<Value>, McpError> {
        Self::application_result(Application::agent_capabilities())
    }

    #[tool(
        description = "Return body-free workspace context and optional job scope from the authoritative CanISend store",
        annotations(
            title = "Inspect CanISend context",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_context(
        &self,
        Parameters(parameters): Parameters<ContextParameters>,
    ) -> Result<Json<Value>, McpError> {
        if let Some(job_id) = parameters.job_id.as_deref() {
            Self::validate_job_id(job_id)?;
        }
        Self::application_result(Application::agent_context(
            Some(self.workspace()),
            parameters.job_id.as_deref(),
        ))
    }

    #[tool(
        description = "Return one CanISend job and its source and workflow metadata without source bodies",
        annotations(
            title = "Inspect CanISend job",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_job_detail(
        &self,
        Parameters(parameters): Parameters<JobParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_job_id(&parameters.job_id)?;
        Self::application_result(Application::job_detail(
            self.workspace(),
            &parameters.job_id,
        ))
    }

    #[tool(
        description = "Read and validate a user-approved local job file or public URL, then return a body-free single-use preview without changing the workspace",
        annotations(
            title = "Preview CanISend job intake",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn canisend_job_intake_preview(
        &self,
        Parameters(parameters): Parameters<JobIntakePreviewParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_job_id(&parameters.job_id)?;
        if parameters.locator.trim().is_empty() {
            return Err(McpError::invalid_params("locator cannot be empty", None));
        }
        let prepared = match parameters.source_kind {
            JobIntakeSourceKind::LocalFile => {
                if !parameters.confirmed_private_read {
                    return Err(Self::consent_required(
                        "The user must explicitly confirm private local-file access before preview.",
                    ));
                }
                Application::prepare_local_job_source(
                    self.workspace(),
                    &parameters.job_id,
                    Path::new(parameters.locator.trim()),
                    PrivateReadConsent::granted_by_user(),
                )
            }
            JobIntakeSourceKind::Url => {
                if !parameters.confirmed_network_fetch {
                    return Err(Self::consent_required(
                        "The user must explicitly confirm the bounded URL request before preview.",
                    ));
                }
                Application::prepare_url_job_source(
                    self.workspace(),
                    &parameters.job_id,
                    parameters.locator.trim(),
                    NetworkFetchConsent::granted_by_user(),
                )
            }
        };
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        let preview = prepared.preview().clone();
        let preview_token = self
            .previews
            .insert(PendingMutation::JobIntake(Box::new(prepared)))?;
        Self::application_result(Ok(serde_json::json!({
            "preview_token": preview_token,
            "preview": preview
        })))
    }

    #[tool(
        description = "Commit the exact previously reviewed job-intake preview after explicit user approval; rejects stale job revisions",
        annotations(
            title = "Commit CanISend job intake",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_job_intake_commit(
        &self,
        Parameters(parameters): Parameters<PreviewTokenParameters>,
    ) -> Result<Json<Value>, McpError> {
        let token = parameters.preview_token;
        let pending = self.previews.take(&token)?;
        let retry = pending.clone();
        let PendingMutation::JobIntake(prepared) = pending else {
            self.previews.restore(token, retry)?;
            return Err(McpError::invalid_params(
                "The preview token belongs to a task completion, not job intake.",
                None,
            ));
        };
        self.mutation_result(
            Application::commit_prepared_job_source(*prepared),
            token,
            retry,
        )
    }

    #[tool(
        description = "List jobs in the authoritative CanISend workspace without private source bodies",
        annotations(
            title = "List CanISend jobs",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_jobs_list(
        &self,
        Parameters(parameters): Parameters<JobListParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::application_result(Application::list_jobs(
            self.workspace(),
            parameters.include_archived,
        ))
    }

    #[tool(
        description = "List profile-source metadata and revisions without returning profile bodies",
        annotations(
            title = "Inspect CanISend profile sources",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_profile_sources(&self) -> Result<Json<Value>, McpError> {
        Self::application_result(Application::list_profile_sources(self.workspace()))
    }

    #[tool(
        description = "Return the latest versioned task state for one job without private input bodies",
        annotations(
            title = "Inspect latest CanISend task",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_task_latest(
        &self,
        Parameters(parameters): Parameters<JobParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_job_id(&parameters.job_id)?;
        Self::application_result(Application::latest_task_for_job(
            self.workspace(),
            &parameters.job_id,
        ))
    }

    #[tool(
        description = "Prepare a versioned task lease against current job/profile/artifact revisions after host approval",
        annotations(
            title = "Prepare CanISend task",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_task_prepare(
        &self,
        Parameters(parameters): Parameters<TaskPrepareParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_job_id(&parameters.job_id)?;
        let request = match TaskPrepareRequest::try_new(
            &parameters.job_id,
            parameters.operation.into(),
            parameters.mode.into(),
        ) {
            Ok(request) => request,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        Self::application_result(Application::prepare_task(self.workspace(), request))
    }

    #[tool(
        description = "Export only the immutable inputs declared by a prepared task after explicit private-read and, when applicable, provider-send approval",
        annotations(
            title = "Export CanISend task inputs",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_task_inputs(
        &self,
        Parameters(parameters): Parameters<TaskInputsParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_task_id(&parameters.task_id)?;
        if !parameters.confirmed_private_read {
            return Err(Self::consent_required(
                "The user must explicitly confirm private task-input access before export.",
            ));
        }
        let request = match TaskInputExportRequest::try_new(
            &parameters.task_id,
            PathBuf::from(parameters.destination),
        ) {
            Ok(request) => request,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        Self::application_result(Application::export_task_inputs(
            self.workspace(),
            request,
            Some(PrivateReadConsent::granted_by_user()),
            parameters
                .confirmed_provider_send
                .then(ProviderSendConsent::granted_by_user),
        ))
    }

    #[tool(
        description = "Validate a user-approved task-completion JSON file and return a single-use body-free commit preview without changing task state",
        annotations(
            title = "Preview CanISend task completion",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_task_completion_preview(
        &self,
        Parameters(parameters): Parameters<TaskCompletionPreviewParameters>,
    ) -> Result<Json<Value>, McpError> {
        if !parameters.confirmed_private_read {
            return Err(Self::consent_required(
                "The user must explicitly confirm private completion-file access before preview.",
            ));
        }
        let preview = match Application::preview_task_completion_file(
            self.workspace(),
            Path::new(parameters.file.trim()),
            PrivateReadConsent::granted_by_user(),
        ) {
            Ok(preview) => preview,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        let preview_token = self.previews.insert(PendingMutation::TaskCompletion(
            preview.data.request.clone(),
        ))?;
        Self::application_result(Ok(serde_json::json!({
            "preview_token": preview_token,
            "preview": preview
        })))
    }

    #[tool(
        description = "Commit the exact previously reviewed task-completion request after explicit user approval; revalidates lease and every input revision/hash",
        annotations(
            title = "Commit CanISend task completion",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_task_completion_commit(
        &self,
        Parameters(parameters): Parameters<PreviewTokenParameters>,
    ) -> Result<Json<Value>, McpError> {
        let token = parameters.preview_token;
        let pending = self.previews.take(&token)?;
        let retry = pending.clone();
        let PendingMutation::TaskCompletion(request) = pending else {
            self.previews.restore(token, retry)?;
            return Err(McpError::invalid_params(
                "The preview token belongs to job intake, not a task completion.",
                None,
            ));
        };
        self.mutation_result(
            Application::commit_task_completion(self.workspace(), request),
            token,
            retry,
        )
    }

    #[tool(
        description = "Return the current workflow status, blockers, and next actions for one CanISend job",
        annotations(
            title = "Inspect CanISend workflow",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_workflow_status(
        &self,
        Parameters(parameters): Parameters<JobParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_job_id(&parameters.job_id)?;
        Self::application_result(Application::workflow_status(
            self.workspace(),
            &parameters.job_id,
        ))
    }
}

#[tool_handler(
    name = "canisend",
    instructions = "CanISend is the authoritative control plane for academic job application state. Inspection tools return metadata, workflow state, and body-free context. Guarded mutation tools require host approval; job intake and task completion must be previewed first and committed with the returned single-use token. Preview tokens live only for this MCP session. Never edit .canisend, SQLite, immutable blobs, or managed projections directly."
)]
impl ServerHandler for CanISendMcpServer {}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_app::{Application, PrivateReadConsent};
    use canisend_contracts::PrivacyClassification;
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::Value;

    use super::{
        CanISendMcpServer, ContextParameters, JobListParameters, JobParameters, MAX_JOB_ID_BYTES,
        MAX_TOOL_RESULT_BYTES,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-mcp-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn read_only_tools_share_the_application_facade_and_hide_private_bodies() {
        let root = temporary_root("privacy");
        let job_source = temporary_root("job-source").with_extension("md");
        let profile_source = temporary_root("profile-source").with_extension("md");
        let job_sentinel = "MCP-PRIVATE-JOB-SENTINEL";
        let profile_sentinel = "MCP-PRIVATE-PROFILE-SENTINEL";
        fs::write(&job_source, job_sentinel).expect("write job source");
        fs::write(&profile_source, profile_sentinel).expect("write profile source");

        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer", "University X")
            .expect("create job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &job_source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import job source");
        Application::import_profile_source(
            &root,
            &profile_source,
            PrivacyClassification::PrivateLocal,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import profile source");
        Application::start_workflow(&root, job.id.as_str()).expect("start workflow");

        let before = Application::workspace_status(&root)
            .expect("workspace before")
            .data
            .status;
        let server = CanISendMcpServer::open(&root).expect("open MCP server");
        let responses = [
            server.canisend_capabilities().expect("capabilities").0,
            server
                .canisend_context(Parameters(ContextParameters {
                    job_id: Some(job.id.to_string()),
                }))
                .expect("context")
                .0,
            server
                .canisend_job_detail(Parameters(JobParameters {
                    job_id: job.id.to_string(),
                }))
                .expect("job detail")
                .0,
            server
                .canisend_jobs_list(Parameters(JobListParameters::default()))
                .expect("jobs")
                .0,
            server
                .canisend_profile_sources()
                .expect("profile sources")
                .0,
            server
                .canisend_workflow_status(Parameters(JobParameters {
                    job_id: job.id.to_string(),
                }))
                .expect("workflow")
                .0,
        ];
        let serialized = serde_json::to_string(&responses).expect("serialize responses");
        assert!(!serialized.contains(job_sentinel));
        assert!(!serialized.contains(profile_sentinel));

        let after = Application::workspace_status(&root)
            .expect("workspace after")
            .data
            .status;
        assert_eq!(before, after);

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(job_source).expect("remove job source");
        fs::remove_file(profile_source).expect("remove profile source");
    }

    #[test]
    fn adapter_bounds_job_ids_and_serialized_results() {
        let oversized_job_id = "x".repeat(MAX_JOB_ID_BYTES + 1);
        assert!(CanISendMcpServer::validate_job_id(&oversized_job_id).is_err());

        let oversized = Value::String("x".repeat(MAX_TOOL_RESULT_BYTES + 1));
        let error = match CanISendMcpServer::application_result::<Value>(Ok(oversized)) {
            Ok(_) => panic!("oversized result must fail"),
            Err(error) => error,
        };
        assert!(error.message.contains("MCP limit"));
    }
}
