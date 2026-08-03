#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use canisend_app::{
    Application, ApplicationError, ApplicationFlowApproveRequestV3,
    ApplicationFlowComposeRequestV3, ApplicationFlowCreateRequestV3,
    ApplicationFlowDeliverableDraftV3, ApplicationFlowExportRequestV3,
    ApplicationFlowPlanRequestV3, ApplicationFlowPlannedDeliverableV3,
    ApplicationFlowRequirementDraftV3, ApprovalBinding, ApprovalBroker, ApprovalBrokerError,
    ApprovalDisposition, ApprovalGrant, ApprovalKind, ApprovalScope, ApprovalSourceVersion,
    NetworkFetchConsent, PreparedJobSource, PrivateExportConsent, PrivateReadConsent,
    ProviderSendConsent, TaskExecutionMode, TaskInputExportRequest, TaskOperation,
    TaskPrepareRequest, approval_disposition_for_application_error,
};
use canisend_contracts::{
    ApplicationFieldValueV3, ExecutionMode, PlannedDeliverableDispositionV3, RequirementPriorityV3,
    Revision, Sha256Digest, TaskCompletionRequest, WorkflowPackItemId,
};
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
const MAX_APPLICATION_ID_BYTES: usize = 128;
const MAX_TASK_ID_BYTES: usize = 128;
const MAX_PREVIEW_TOKEN_BYTES: usize = 80;
const MAX_SESSION_INPUT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOOL_RESULT_BYTES: usize = 2 * 1024 * 1024;

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
    previews: Arc<ApprovalBroker<PendingMutation>>,
}

#[derive(Debug, Clone)]
enum PendingMutation {
    JobIntake(Box<PreparedJobSource>),
    TaskCompletion(TaskCompletionRequest),
    ApplicationApproval(ApplicationApprovalPreview),
}

#[derive(Debug, Clone)]
struct ApplicationApprovalPreview {
    application_id: String,
    expected_revision: Revision,
    snapshot_sha256: Sha256Digest,
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

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentV3ContextParameters {
    #[schemars(description = "Optional exact-Pack Application ID to resume")]
    pub application_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationParameters {
    #[schemars(description = "Exact-Pack Application ID")]
    pub application_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationRequirementParameters {
    pub category: String,
    pub statement: String,
    pub priority: RequirementPriorityV3,
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationCreateParameters {
    pub title: String,
    #[serde(default)]
    pub opportunity_metadata: BTreeMap<String, ApplicationFieldValueV3>,
    #[serde(default)]
    pub application_metadata: BTreeMap<String, ApplicationFieldValueV3>,
    #[schemars(description = "Reviewed UTF-8 source body; never returned by routine context")]
    pub source_text: String,
    pub requirements: Vec<ApplicationRequirementParameters>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPlannedDeliverableParameters {
    pub kind: String,
    pub disposition: PlannedDeliverableDispositionV3,
    pub rationale: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub execution_mode: Option<ExecutionMode>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPlanParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub decision: String,
    pub deliverables: Vec<ApplicationPlannedDeliverableParameters>,
    #[serde(default)]
    #[schemars(description = "User explicitly confirmed Requirements and this exact Plan")]
    pub confirmed_user_decision: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDeliverableParameters {
    pub kind: String,
    pub title: String,
    pub media_type: String,
    #[schemars(description = "Private Deliverable body to commit for explicit review")]
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationComposeParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub deliverables: Vec<ApplicationDeliverableParameters>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationReviewParameters {
    pub application_id: String,
    #[serde(default)]
    #[schemars(description = "User explicitly allowed local private Deliverable body access")]
    pub confirmed_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationApprovalParameters {
    #[schemars(description = "Opaque session-local token returned by application review")]
    pub review_token: String,
    #[serde(default)]
    #[schemars(description = "User explicitly approved every body in the exact reviewed snapshot")]
    pub confirmed_user_approval: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationExportParameters {
    pub application_id: String,
    pub expected_revision: u64,
    #[schemars(description = "Safe workspace-relative output directory")]
    pub destination: String,
    #[serde(default)]
    #[schemars(description = "User explicitly authorized this local private export")]
    pub confirmed_private_export: bool,
}

impl ApplicationCreateParameters {
    fn try_into_request(self) -> Result<ApplicationFlowCreateRequestV3, ApplicationError> {
        Ok(ApplicationFlowCreateRequestV3 {
            title: self.title,
            opportunity_metadata: pack_metadata(self.opportunity_metadata)?,
            application_metadata: pack_metadata(self.application_metadata)?,
            source_text: self.source_text,
            requirements: self
                .requirements
                .into_iter()
                .map(|requirement| {
                    Ok(ApplicationFlowRequirementDraftV3 {
                        category: pack_item(&requirement.category)?,
                        statement: requirement.statement,
                        priority: requirement.priority,
                        start_byte: requirement.start_byte,
                        end_byte: requirement.end_byte,
                    })
                })
                .collect::<Result<Vec<_>, ApplicationError>>()?,
        })
    }
}

impl ApplicationPlanParameters {
    fn try_into_request(self) -> Result<ApplicationFlowPlanRequestV3, ApplicationError> {
        Ok(ApplicationFlowPlanRequestV3 {
            expected_revision: revision(self.expected_revision)?,
            decision: pack_item(&self.decision)?,
            deliverables: self
                .deliverables
                .into_iter()
                .map(|deliverable| {
                    Ok(ApplicationFlowPlannedDeliverableV3 {
                        kind: pack_item(&deliverable.kind)?,
                        disposition: deliverable.disposition,
                        rationale: deliverable.rationale,
                        constraints: deliverable.constraints,
                        execution_mode: deliverable.execution_mode,
                    })
                })
                .collect::<Result<Vec<_>, ApplicationError>>()?,
        })
    }
}

impl ApplicationComposeParameters {
    fn try_into_request(self) -> Result<ApplicationFlowComposeRequestV3, ApplicationError> {
        Ok(ApplicationFlowComposeRequestV3 {
            expected_revision: revision(self.expected_revision)?,
            deliverables: self
                .deliverables
                .into_iter()
                .map(|deliverable| {
                    Ok(ApplicationFlowDeliverableDraftV3 {
                        kind: pack_item(&deliverable.kind)?,
                        title: deliverable.title,
                        media_type: deliverable.media_type,
                        content: deliverable.content,
                    })
                })
                .collect::<Result<Vec<_>, ApplicationError>>()?,
        })
    }
}

fn pack_metadata(
    values: BTreeMap<String, ApplicationFieldValueV3>,
) -> Result<BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>, ApplicationError> {
    values
        .into_iter()
        .map(|(key, value)| Ok((pack_item(&key)?, value)))
        .collect()
}

fn pack_item(value: &str) -> Result<WorkflowPackItemId, ApplicationError> {
    WorkflowPackItemId::try_new(value)
        .map_err(|error| ApplicationError::InvalidInput(error.to_string()))
}

fn revision(value: u64) -> Result<Revision, ApplicationError> {
    Revision::try_new(value).map_err(|error| ApplicationError::InvalidInput(error.to_string()))
}

impl CanISendMcpServer {
    pub fn open(workspace: &Path) -> Result<Self, ApplicationError> {
        let workspace = Application::resolve_workspace_root(Some(workspace))?;
        Ok(Self {
            workspace: Arc::new(workspace),
            previews: Arc::new(ApprovalBroker::default()),
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

    fn validate_application_id(application_id: &str) -> Result<(), McpError> {
        if application_id.is_empty() || application_id.len() > MAX_APPLICATION_ID_BYTES {
            return Err(McpError::invalid_params(
                format!("application_id must contain 1 to {MAX_APPLICATION_ID_BYTES} bytes"),
                None,
            ));
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
        grant: ApprovalGrant<PendingMutation>,
    ) -> Result<Json<Value>, McpError> {
        match result {
            Ok(value) => {
                self.previews
                    .resolve(grant, ApprovalDisposition::Consume)
                    .map_err(Self::approval_error)?;
                Self::application_result(Ok(value))
            }
            Err(error) => {
                let disposition = approval_disposition_for_application_error(&error);
                self.previews
                    .resolve(grant, disposition)
                    .map_err(Self::approval_error)?;
                Self::application_result::<T>(Err(error))
            }
        }
    }

    fn approval_scope(&self) -> Result<ApprovalScope, McpError> {
        ApprovalScope::for_workspace(self.workspace()).map_err(|error| {
            let failure = error.classify();
            let data = serde_json::to_value(&failure).ok();
            McpError::invalid_params(failure.message, data)
        })
    }

    fn insert_preview(
        &self,
        kind: ApprovalKind,
        application_id: Option<String>,
        source: ApprovalSourceVersion,
        pending: PendingMutation,
    ) -> Result<canisend_app::ApprovalLease, McpError> {
        let binding = ApprovalBinding::new(kind, self.approval_scope()?, application_id, source);
        self.previews
            .insert(binding, pending)
            .map_err(Self::approval_error)
    }

    fn take_preview(
        &self,
        token: &str,
        kind: ApprovalKind,
    ) -> Result<ApprovalGrant<PendingMutation>, McpError> {
        if token.len() > MAX_PREVIEW_TOKEN_BYTES {
            return Err(McpError::invalid_params("preview_token is malformed", None));
        }
        self.previews
            .take(token, kind, &self.approval_scope()?)
            .map_err(Self::approval_error)
    }

    fn approval_error(error: ApprovalBrokerError) -> McpError {
        let code = match &error {
            ApprovalBrokerError::InvalidConfiguration(_) => "approval.invalid-configuration",
            ApprovalBrokerError::Unavailable => "approval.unavailable",
            ApprovalBrokerError::TokenGeneration(_) | ApprovalBrokerError::TokenCollision => {
                "approval.token-generation-failed"
            }
            ApprovalBrokerError::CapacityFull { .. } => "approval.capacity-full",
            ApprovalBrokerError::MalformedToken => "approval.token-malformed",
            ApprovalBrokerError::Missing => "approval.missing-or-replayed",
            ApprovalBrokerError::Expired => "approval.expired",
            ApprovalBrokerError::WrongKind { .. } => "approval.wrong-kind",
            ApprovalBrokerError::WrongContext => "approval.wrong-context",
            ApprovalBrokerError::RestoreCollision => "approval.restore-collision",
        };
        McpError::invalid_params(
            error.to_string(),
            Some(serde_json::json!({
                "status": "approval-rejected",
                "code": code,
                "retryable": matches!(
                    error,
                    ApprovalBrokerError::Unavailable
                        | ApprovalBrokerError::TokenGeneration(_)
                        | ApprovalBrokerError::RestoreCollision
                )
            })),
        )
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
        description = "Return the canonical neutral Agent v3 operation registry and exact Workspace Pack binding",
        annotations(
            title = "Inspect Agent v3 capabilities",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_agent_v3_capabilities(&self) -> Result<Json<Value>, McpError> {
        Self::application_result(Application::agent_v3_capabilities(self.workspace()))
    }

    #[tool(
        description = "Return body-free exact-Pack Application summaries, blockers, and revision-bound next actions",
        annotations(
            title = "Inspect Agent v3 context",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_agent_v3_context(
        &self,
        Parameters(parameters): Parameters<AgentV3ContextParameters>,
    ) -> Result<Json<Value>, McpError> {
        if let Some(application_id) = parameters.application_id.as_deref() {
            Self::validate_application_id(application_id)?;
        }
        Self::application_result(Application::agent_v3_context(
            self.workspace(),
            parameters.application_id.as_deref(),
        ))
    }

    #[tool(
        description = "List body-free exact-Pack Application summaries from the authoritative v3 workspace",
        annotations(
            title = "List Applications",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_applications_list(&self) -> Result<Json<Value>, McpError> {
        let context = match Application::agent_v3_context(self.workspace(), None) {
            Ok(context) => context,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        Self::application_result(Ok(serde_json::json!({
            "operation": "application.list",
            "status": "available",
            "pack": context.data.pack,
            "applications": context.data.applications,
            "privacy": "public",
            "submission_supported": false
        })))
    }

    #[tool(
        description = "Create one exact-Pack Application from reviewed source text and UTF-8 Requirement spans after host write approval",
        annotations(
            title = "Create Application",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_application_create(
        &self,
        Parameters(parameters): Parameters<ApplicationCreateParameters>,
    ) -> Result<Json<Value>, McpError> {
        let request = match parameters.try_into_request() {
            Ok(request) => request,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        Self::application_result(Application::agent_v3_create_application(
            self.workspace(),
            request,
        ))
    }

    #[tool(
        description = "Confirm exact Requirements and a Pack-qualified Plan at the expected Application revision after explicit user decision",
        annotations(
            title = "Confirm Application Plan",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_application_plan(
        &self,
        Parameters(parameters): Parameters<ApplicationPlanParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_application_id(&parameters.application_id)?;
        if !parameters.confirmed_user_decision {
            return Err(Self::consent_required(
                "The user must explicitly confirm the exact Requirements and Plan before commit.",
            ));
        }
        let application_id = parameters.application_id.clone();
        let request = match parameters.try_into_request() {
            Ok(request) => request,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        Self::application_result(Application::agent_v3_plan_application(
            self.workspace(),
            &application_id,
            request,
        ))
    }

    #[tool(
        description = "Commit exact Pack-qualified Deliverable bodies at the expected Application revision for later private review",
        annotations(
            title = "Compose Deliverables",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_application_compose(
        &self,
        Parameters(parameters): Parameters<ApplicationComposeParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_application_id(&parameters.application_id)?;
        let application_id = parameters.application_id.clone();
        let request = match parameters.try_into_request() {
            Ok(request) => request,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        Self::application_result(Application::agent_v3_compose_application(
            self.workspace(),
            &application_id,
            request,
        ))
    }

    #[tool(
        description = "Read verified private Deliverable bodies after explicit consent and return a session-local token bound to the exact reviewed revision and snapshot digest",
        annotations(
            title = "Review Deliverables",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_application_review(
        &self,
        Parameters(parameters): Parameters<ApplicationReviewParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_application_id(&parameters.application_id)?;
        if !parameters.confirmed_private_read {
            return Err(Self::consent_required(
                "The user must explicitly confirm private Deliverable access before review.",
            ));
        }
        let review = match Application::review_application_flow_v3(
            self.workspace(),
            &parameters.application_id,
            Some(PrivateReadConsent::granted_by_user()),
        ) {
            Ok(review) => review,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        let preview = ApplicationApprovalPreview {
            application_id: parameters.application_id,
            expected_revision: review.data.stored.snapshot.application.revision,
            snapshot_sha256: review.data.stored.snapshot_sha256.clone(),
        };
        let lease = self.insert_preview(
            ApprovalKind::ApplicationApproval,
            Some(preview.application_id.clone()),
            ApprovalSourceVersion::RevisionAndSnapshot {
                revision: preview.expected_revision,
                snapshot_sha256: preview.snapshot_sha256.clone(),
            },
            PendingMutation::ApplicationApproval(preview),
        )?;
        Self::application_result(Ok(serde_json::json!({
            "review_token": lease.token,
            "expires_at_unix_ms": lease.expires_at_unix_ms,
            "remaining_ttl_seconds": lease.remaining_ttl_seconds,
            "review": review,
            "next_action": {
                "action": "canisend_application_approve",
                "description": "After the user approves every returned body, commit with this single-use review_token"
            }
        })))
    }

    #[tool(
        description = "Approve only the exact Deliverable snapshot previously returned by review, using its single-use session token and explicit user approval",
        annotations(
            title = "Approve Deliverables",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_application_approve(
        &self,
        Parameters(parameters): Parameters<ApplicationApprovalParameters>,
    ) -> Result<Json<Value>, McpError> {
        if !parameters.confirmed_user_approval {
            return Err(Self::consent_required(
                "The user must explicitly approve every body in the reviewed snapshot.",
            ));
        }
        let token = parameters.review_token;
        let grant = self.take_preview(&token, ApprovalKind::ApplicationApproval)?;
        let PendingMutation::ApplicationApproval(preview) = grant.payload().clone() else {
            self.previews
                .resolve(grant, ApprovalDisposition::Consume)
                .map_err(Self::approval_error)?;
            return Err(McpError::invalid_params(
                "Approval payload does not match Application approval.",
                None,
            ));
        };
        let result = (|| {
            let current =
                Application::application_flow_v3(self.workspace(), &preview.application_id)?;
            if current.data.stored.snapshot.application.revision != preview.expected_revision
                || current.data.stored.snapshot_sha256 != preview.snapshot_sha256
            {
                return Err(ApplicationError::InvalidInput(
                    "The Application changed after review; refresh context and review the current bodies again"
                        .to_owned(),
                ));
            }
            Application::agent_v3_approve_application(
                self.workspace(),
                &preview.application_id,
                ApplicationFlowApproveRequestV3 {
                    expected_revision: preview.expected_revision,
                },
            )
        })();
        self.mutation_result(result, grant)
    }

    #[tool(
        description = "Render and export approved Deliverables to a safe workspace-relative directory after explicit private-export consent; never uploads or submits",
        annotations(
            title = "Export Application",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_application_export(
        &self,
        Parameters(parameters): Parameters<ApplicationExportParameters>,
    ) -> Result<Json<Value>, McpError> {
        Self::validate_application_id(&parameters.application_id)?;
        if !parameters.confirmed_private_export {
            return Err(Self::consent_required(
                "The user must explicitly authorize this local private export.",
            ));
        }
        let request = match ApplicationFlowExportRequestV3::try_new(
            &parameters.application_id,
            parameters.expected_revision,
            &parameters.destination,
        ) {
            Ok(request) => request,
            Err(error) => return Self::application_result::<Value>(Err(error)),
        };
        Self::application_result(Application::agent_v3_export_application(
            self.workspace(),
            request,
            Some(PrivateExportConsent::granted_by_user()),
        ))
    }

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
        let lease = self.insert_preview(
            ApprovalKind::JobIntake,
            Some(preview.data.job.id.to_string()),
            ApprovalSourceVersion::RevisionAndSnapshot {
                revision: preview.data.expected_job_revision,
                snapshot_sha256: preview.data.provenance.original_sha256.clone(),
            },
            PendingMutation::JobIntake(Box::new(prepared)),
        )?;
        Self::application_result(Ok(serde_json::json!({
            "preview_token": lease.token,
            "expires_at_unix_ms": lease.expires_at_unix_ms,
            "remaining_ttl_seconds": lease.remaining_ttl_seconds,
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
        let grant = self.take_preview(&token, ApprovalKind::JobIntake)?;
        let PendingMutation::JobIntake(prepared) = grant.payload().clone() else {
            self.previews
                .resolve(grant, ApprovalDisposition::Consume)
                .map_err(Self::approval_error)?;
            return Err(McpError::invalid_params(
                "Approval payload does not match job intake.",
                None,
            ));
        };
        self.mutation_result(Application::commit_prepared_job_source(*prepared), grant)
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
        let lease = self.insert_preview(
            ApprovalKind::TaskCompletion,
            Some(preview.data.state.descriptor.job_id.to_string()),
            ApprovalSourceVersion::Revision(preview.data.request.expected_job_revision),
            PendingMutation::TaskCompletion(preview.data.request.clone()),
        )?;
        Self::application_result(Ok(serde_json::json!({
            "preview_token": lease.token,
            "expires_at_unix_ms": lease.expires_at_unix_ms,
            "remaining_ttl_seconds": lease.remaining_ttl_seconds,
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
        let grant = self.take_preview(&token, ApprovalKind::TaskCompletion)?;
        let PendingMutation::TaskCompletion(request) = grant.payload().clone() else {
            self.previews
                .resolve(grant, ApprovalDisposition::Consume)
                .map_err(Self::approval_error)?;
            return Err(McpError::invalid_params(
                "Approval payload does not match task completion.",
                None,
            ));
        };
        self.mutation_result(
            Application::commit_task_completion(self.workspace(), request),
            grant,
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
    instructions = "Canonical Agent v3 tools resolve the exact Workspace/Application Pack binding and use Application, Requirement, Plan, and Deliverable nouns. Routine context is body-free. Guarded mutations require host approval and exact expected revisions. Plan confirmation, private review, approval, and export require explicit user authorization; approval accepts only a session-local single-use token bound to the exact reviewed revision and snapshot digest. CanISend never uploads or submits an Application. Agent v2 tools remain a deprecated compatibility surface bounded to the exact academic Pack. Never edit .canisend, SQLite, immutable blobs, or managed projections directly."
)]
impl ServerHandler for CanISendMcpServer {}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_app::{
        Application, ApplicationModelCommitRequestV3, PrivateReadConsent,
        WorkspaceV3MigrationRequest,
    };
    use canisend_contracts::{
        ApplicationFieldValueV3, ExecutionMode, PlannedDeliverableDispositionV3,
        PrivacyClassification, RequirementPriorityV3, Revision,
    };
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::Value;

    use super::{
        AgentV3ContextParameters, ApplicationApprovalParameters, ApplicationComposeParameters,
        ApplicationCreateParameters, ApplicationDeliverableParameters, ApplicationExportParameters,
        ApplicationPlanParameters, ApplicationPlannedDeliverableParameters,
        ApplicationRequirementParameters, ApplicationReviewParameters, CanISendMcpServer,
        ContextParameters, JobListParameters, JobParameters, MAX_JOB_ID_BYTES,
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
        for response in &responses {
            assert_eq!(response["compatibility"]["surface"], "agent-v2");
            assert_eq!(response["compatibility"]["deprecated"], true);
            assert_eq!(
                response["compatibility"]["pack"]["id"],
                "org.canisend.academic-job"
            );
            assert!(
                response["compatibility"]["canonical_v3_operation"]
                    .as_str()
                    .is_some_and(|operation| !operation.is_empty())
            );
        }

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

    #[test]
    fn agent_v3_runs_new_resume_review_approval_and_stale_recovery() {
        let root = temporary_root("agent-v3-lifecycle");
        Application::initialize_workspace_v3(&root).expect("initialize v3 workspace");
        let server = CanISendMcpServer::open(&root).expect("open MCP server");
        let capabilities = server
            .canisend_agent_v3_capabilities()
            .expect("Agent v3 capabilities")
            .0;
        assert_eq!(
            capabilities["data"]["pack"]["id"],
            "org.canisend.generic-application"
        );
        let initially_listed = server
            .canisend_applications_list()
            .expect("initial Application list")
            .0;
        assert_eq!(
            initially_listed["applications"].as_array().map(Vec::len),
            Some(0)
        );

        let source = "Provide one primary narrative.";
        let created = server
            .canisend_application_create(Parameters(ApplicationCreateParameters {
                title: "Neutral fixture".to_owned(),
                opportunity_metadata: BTreeMap::from([(
                    "organization".to_owned(),
                    ApplicationFieldValueV3::ShortText("Example Organization".to_owned()),
                )]),
                application_metadata: BTreeMap::new(),
                source_text: source.to_owned(),
                requirements: vec![ApplicationRequirementParameters {
                    category: "format".to_owned(),
                    statement: source.to_owned(),
                    priority: RequirementPriorityV3::Mandatory,
                    start_byte: 0,
                    end_byte: source.len() as u64,
                }],
            }))
            .expect("create")
            .0;
        let application_id = created["data"]["stored"]["snapshot"]["application"]["id"]
            .as_str()
            .expect("Application ID")
            .to_owned();
        let listed = server
            .canisend_applications_list()
            .expect("Application list")
            .0;
        assert_eq!(listed["applications"].as_array().map(Vec::len), Some(1));

        let resumed = server
            .canisend_agent_v3_context(Parameters(AgentV3ContextParameters {
                application_id: Some(application_id.clone()),
            }))
            .expect("resume context")
            .0;
        let context_json = serde_json::to_string(&resumed).expect("context JSON");
        assert_eq!(
            resumed["data"]["next_actions"][0]["action"],
            "canisend_application_plan"
        );
        assert!(!context_json.contains(source));
        assert!(!context_json.contains("Example Organization"));

        let denied_plan = server.canisend_application_plan(Parameters(ApplicationPlanParameters {
            application_id: application_id.clone(),
            expected_revision: 1,
            decision: "proceed".to_owned(),
            deliverables: vec![ApplicationPlannedDeliverableParameters {
                kind: "primary-document".to_owned(),
                disposition: PlannedDeliverableDispositionV3::Required,
                rationale: "Required by source".to_owned(),
                constraints: Vec::new(),
                execution_mode: Some(ExecutionMode::HostAgent),
            }],
            confirmed_user_decision: false,
        }));
        assert!(denied_plan.is_err());
        assert_eq!(
            Application::application_model_v3(&root, &application_id)
                .expect("unchanged after denied Plan")
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );

        server
            .canisend_application_plan(Parameters(ApplicationPlanParameters {
                application_id: application_id.clone(),
                expected_revision: 1,
                decision: "proceed".to_owned(),
                deliverables: vec![ApplicationPlannedDeliverableParameters {
                    kind: "primary-document".to_owned(),
                    disposition: PlannedDeliverableDispositionV3::Required,
                    rationale: "Required by source".to_owned(),
                    constraints: Vec::new(),
                    execution_mode: Some(ExecutionMode::HostAgent),
                }],
                confirmed_user_decision: true,
            }))
            .expect("Plan");
        server
            .canisend_application_compose(Parameters(ApplicationComposeParameters {
                application_id: application_id.clone(),
                expected_revision: 2,
                deliverables: vec![ApplicationDeliverableParameters {
                    kind: "primary-document".to_owned(),
                    title: "Primary document".to_owned(),
                    media_type: "text/markdown".to_owned(),
                    content: "PRIVATE-AGENT-V3-DELIVERABLE".to_owned(),
                }],
            }))
            .expect("compose");

        assert!(
            server
                .canisend_application_review(Parameters(ApplicationReviewParameters {
                    application_id: application_id.clone(),
                    confirmed_private_read: false,
                }))
                .is_err()
        );
        let reviewed = server
            .canisend_application_review(Parameters(ApplicationReviewParameters {
                application_id: application_id.clone(),
                confirmed_private_read: true,
            }))
            .expect("review")
            .0;
        assert!(
            serde_json::to_string(&reviewed)
                .expect("review JSON")
                .contains("PRIVATE-AGENT-V3-DELIVERABLE")
        );
        let stale_token = reviewed["review_token"]
            .as_str()
            .expect("review token")
            .to_owned();
        assert_eq!(reviewed["remaining_ttl_seconds"], 600);
        assert!(reviewed["expires_at_unix_ms"].as_u64().is_some());

        let current = Application::application_model_v3(&root, &application_id)
            .expect("current model")
            .data;
        let mut concurrent = current.snapshot;
        concurrent.application.revision = Revision::try_new(4).expect("revision");
        concurrent.application.updated_at =
            canisend_store::current_utc_timestamp().expect("current timestamp");
        concurrent.application.metadata.insert(
            canisend_contracts::WorkflowPackItemId::try_new("notes").expect("notes key"),
            ApplicationFieldValueV3::LongText("Concurrent reviewed note".to_owned()),
        );
        Application::commit_application_model_v3(
            &root,
            &application_id,
            ApplicationModelCommitRequestV3 {
                expected_revision: Revision::try_new(3).expect("revision"),
                snapshot: concurrent,
                reason: "concurrent-user-note".to_owned(),
            },
        )
        .expect("concurrent commit");
        std::thread::sleep(std::time::Duration::from_millis(2));

        let stale =
            server.canisend_application_approve(Parameters(ApplicationApprovalParameters {
                review_token: stale_token.clone(),
                confirmed_user_approval: true,
            }));
        assert!(stale.is_err());
        let replayed_stale =
            server.canisend_application_approve(Parameters(ApplicationApprovalParameters {
                review_token: stale_token,
                confirmed_user_approval: true,
            }));
        let replayed_stale = match replayed_stale {
            Ok(_) => panic!("stale approval must be consumed"),
            Err(error) => error,
        };
        assert!(replayed_stale.message.contains("already consumed"));

        let refreshed = server
            .canisend_application_review(Parameters(ApplicationReviewParameters {
                application_id: application_id.clone(),
                confirmed_private_read: true,
            }))
            .expect("refresh review")
            .0;
        let fresh_token = refreshed["review_token"]
            .as_str()
            .expect("fresh token")
            .to_owned();
        assert!(
            server
                .canisend_application_approve(Parameters(ApplicationApprovalParameters {
                    review_token: fresh_token.clone(),
                    confirmed_user_approval: false,
                }))
                .is_err()
        );
        server
            .canisend_application_approve(Parameters(ApplicationApprovalParameters {
                review_token: fresh_token.clone(),
                confirmed_user_approval: true,
            }))
            .expect("approve refreshed review");
        assert!(
            server
                .canisend_application_approve(Parameters(ApplicationApprovalParameters {
                    review_token: fresh_token,
                    confirmed_user_approval: true,
                }))
                .is_err(),
            "approval token must be single-use"
        );

        let ready = server
            .canisend_agent_v3_context(Parameters(AgentV3ContextParameters {
                application_id: Some(application_id.clone()),
            }))
            .expect("approved context")
            .0;
        assert_eq!(
            ready["data"]["next_actions"][0]["action"],
            "canisend_application_export"
        );
        assert_eq!(ready["data"]["submission_supported"], false);

        let destination = format!("applications/{application_id}/exports/mcp-semantic-parity");
        assert!(
            server
                .canisend_application_export(Parameters(ApplicationExportParameters {
                    application_id: application_id.clone(),
                    expected_revision: 4,
                    destination: destination.clone(),
                    confirmed_private_export: true,
                }))
                .is_err(),
            "stale export must fail"
        );
        assert!(!root.join(&destination).exists());
        let exported = server
            .canisend_application_export(Parameters(ApplicationExportParameters {
                application_id,
                expected_revision: 5,
                destination,
                confirmed_private_export: true,
            }))
            .expect("export")
            .0;
        assert_eq!(exported["data"]["render"]["submission_performed"], false);

        let generic_before = Application::workspace_status(&root)
            .expect("generic status before compatibility MCP request")
            .data
            .status;
        assert!(
            server
                .canisend_context(Parameters(ContextParameters { job_id: None }))
                .is_err(),
            "academic compatibility MCP must fail closed on the generic Pack"
        );
        assert_eq!(
            Application::workspace_status(&root)
                .expect("generic status after compatibility MCP request")
                .data
                .status,
            generic_before
        );

        let academic = temporary_root("agent-v3-wrong-pack");
        Application::initialize_workspace(&academic).expect("academic Workspace");
        let before = Application::workspace_status(&academic)
            .expect("academic status before")
            .data
            .status;
        let academic_server = CanISendMcpServer::open(&academic).expect("open academic MCP");
        let wrong_pack_source = "Wrong Pack request must not persist.";
        assert!(
            academic_server
                .canisend_application_create(Parameters(ApplicationCreateParameters {
                    title: "Wrong Pack Application".to_owned(),
                    opportunity_metadata: BTreeMap::new(),
                    application_metadata: BTreeMap::new(),
                    source_text: wrong_pack_source.to_owned(),
                    requirements: vec![ApplicationRequirementParameters {
                        category: "format".to_owned(),
                        statement: wrong_pack_source.to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: wrong_pack_source.len() as u64,
                    }],
                }))
                .is_err()
        );
        assert_eq!(
            Application::workspace_status(&academic)
                .expect("academic status after")
                .data
                .status,
            before
        );

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_dir_all(academic).expect("remove academic workspace");
    }

    #[test]
    fn agent_v3_mcp_completes_the_migrated_academic_pack_flow() {
        let root = temporary_root("agent-v3-academic");
        let backup = temporary_root("agent-v3-academic-backup");
        Application::initialize_workspace(&root).expect("Workspace v2");
        Application::create_job(&root, "Research Fellow", "Example University")
            .expect("legacy academic Application");
        let preview = Application::preview_workspace_v3_migration(&root)
            .expect("migration preview")
            .data;
        Application::migrate_workspace_v3(
            &root,
            WorkspaceV3MigrationRequest {
                expected_plan_sha256: preview.migration_plan_sha256,
                backup_destination: backup.clone(),
            },
        )
        .expect("migration");

        let server = CanISendMcpServer::open(&root).expect("academic MCP server");
        let capabilities = server
            .canisend_agent_v3_capabilities()
            .expect("academic capabilities")
            .0;
        assert_eq!(
            capabilities["data"]["pack"]["id"],
            "org.canisend.academic-job"
        );
        let resumed = server
            .canisend_applications_list()
            .expect("migrated academic Application")
            .0;
        assert_eq!(resumed["applications"].as_array().map(Vec::len), Some(1));

        let source = "Applicants must submit a cover letter and academic CV.";
        let created = server
            .canisend_application_create(Parameters(ApplicationCreateParameters {
                title: "Academic Agent v3 fixture".to_owned(),
                opportunity_metadata: BTreeMap::from([(
                    "institution".to_owned(),
                    ApplicationFieldValueV3::ShortText("Example University".to_owned()),
                )]),
                application_metadata: BTreeMap::new(),
                source_text: source.to_owned(),
                requirements: vec![ApplicationRequirementParameters {
                    category: "qualification".to_owned(),
                    statement: source.to_owned(),
                    priority: RequirementPriorityV3::Mandatory,
                    start_byte: 0,
                    end_byte: source.len() as u64,
                }],
            }))
            .expect("academic create")
            .0;
        let application_id = created["data"]["stored"]["snapshot"]["application"]["id"]
            .as_str()
            .expect("Application ID")
            .to_owned();
        let planned = ["cover-letter", "cv"]
            .into_iter()
            .map(|kind| ApplicationPlannedDeliverableParameters {
                kind: kind.to_owned(),
                disposition: PlannedDeliverableDispositionV3::Required,
                rationale: "Required by the reviewed opportunity".to_owned(),
                constraints: Vec::new(),
                execution_mode: Some(ExecutionMode::HostAgent),
            })
            .collect();
        server
            .canisend_application_plan(Parameters(ApplicationPlanParameters {
                application_id: application_id.clone(),
                expected_revision: 1,
                decision: "proceed".to_owned(),
                deliverables: planned,
                confirmed_user_decision: true,
            }))
            .expect("academic Plan");
        server
            .canisend_application_compose(Parameters(ApplicationComposeParameters {
                application_id: application_id.clone(),
                expected_revision: 2,
                deliverables: vec![
                    ApplicationDeliverableParameters {
                        kind: "cover-letter".to_owned(),
                        title: "Cover letter".to_owned(),
                        media_type: "text/markdown".to_owned(),
                        content: "Synthetic evidence-bound cover letter.".to_owned(),
                    },
                    ApplicationDeliverableParameters {
                        kind: "cv".to_owned(),
                        title: "Academic CV".to_owned(),
                        media_type: "text/markdown".to_owned(),
                        content: "Synthetic evidence-bound academic record.".to_owned(),
                    },
                ],
            }))
            .expect("academic compose");
        let reviewed = server
            .canisend_application_review(Parameters(ApplicationReviewParameters {
                application_id: application_id.clone(),
                confirmed_private_read: true,
            }))
            .expect("academic review")
            .0;
        let token = reviewed["review_token"]
            .as_str()
            .expect("review token")
            .to_owned();
        server
            .canisend_application_approve(Parameters(ApplicationApprovalParameters {
                review_token: token,
                confirmed_user_approval: true,
            }))
            .expect("academic approval");
        let destination = format!("applications/{application_id}/exports/academic-mcp");
        let exported = server
            .canisend_application_export(Parameters(ApplicationExportParameters {
                application_id,
                expected_revision: 4,
                destination,
                confirmed_private_export: true,
            }))
            .expect("academic export")
            .0;
        assert_eq!(
            exported["data"]["render"]["documents"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(exported["data"]["render"]["submission_performed"], false);

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_dir_all(backup).expect("remove backup");
    }
}
