#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use canisend_app::{
    Application, ApplicationDeliverableReviseRequestV4, ApplicationError,
    ApplicationFlowApproveRequestV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowDeliverableDraftV3, ApplicationFlowExportRequestV3,
    ApplicationFlowPlannedDeliverableV3, ApplicationFlowRequirementDraftV3,
    ApplicationMutationApprovalBrokerV4, ApplicationMutationApprovalErrorV4,
    ApplicationPlanConfirmRequestV4, ApplicationPlanProposeRequestV4,
    ApplicationRequirementConfirmRequestV4, ApplicationRequirementExtractRequestV4,
    ApprovalBrokerError, AssociationApprovalBrokerV4, AssociationApprovalErrorV4,
    AssociationChangeV4, EvidenceAssociationPreviewRequestV4, PrivateExportConsent,
    PrivateReadConsent, ProfileAssociationPreviewRequestV4, RequirementDecisionV4,
};
use canisend_contracts::{
    ApplicationId, ContentRevisionReferenceV3, DeliverableId, ExecutionMode,
    PlannedDeliverableDispositionV3, RequirementId, RequirementPriorityV3, Revision, Sha256Digest,
    WorkflowPackItemId,
};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::wrapper::{Json, Parameters},
    tool, tool_handler, tool_router,
};
use schemars::{JsonSchema, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const MAX_APPLICATION_ID_BYTES: usize = 128;
const MAX_SESSION_INPUT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOOL_RESULT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
struct McpStructuredOutput(Value);

impl JsonSchema for McpStructuredOutput {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("McpStructuredOutput")
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "type": "object",
            "additionalProperties": true
        })
    }
}

impl std::ops::Deref for McpStructuredOutput {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    association_approvals: AssociationApprovalBrokerV4,
    mutation_approvals: ApplicationMutationApprovalBrokerV4,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationParameters {
    #[schemars(description = "CanISend Application ID")]
    pub application_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationRequirementParameters {
    #[schemars(description = "CanISend Application ID")]
    pub application_id: String,
    #[schemars(description = "Requirement ID owned by the selected Application")]
    pub requirement_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDeliverableParameters {
    #[schemars(description = "CanISend Application ID")]
    pub application_id: String,
    #[schemars(description = "Deliverable ID owned by the selected Application")]
    pub deliverable_id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AssociationChangeParameters {
    Associate,
    Unlink,
}

impl From<AssociationChangeParameters> for AssociationChangeV4 {
    fn from(value: AssociationChangeParameters) -> Self {
        match value {
            AssociationChangeParameters::Associate => Self::Associate,
            AssociationChangeParameters::Unlink => Self::Unlink,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileAssociationPreviewParameters {
    #[schemars(description = "CanISend Application ID")]
    pub application_id: String,
    pub profile_source: ContentRevisionReferenceV3,
    pub change: AssociationChangeParameters,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAssociationPreviewParameters {
    #[schemars(description = "CanISend Application ID")]
    pub application_id: String,
    pub evidence: ContentRevisionReferenceV3,
    pub change: AssociationChangeParameters,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AssociationCommitParameters {
    #[schemars(description = "CanISend Application ID bound to the preview")]
    pub application_id: String,
    #[schemars(description = "Opaque single-use preview token")]
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    #[schemars(description = "True only after the user explicitly approves this exact preview")]
    pub approved: bool,
    #[schemars(
        description = "True only after explicit consent to read the selected private input"
    )]
    pub confirmed_private_read: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementDecisionParameters {
    Confirm,
    Exclude,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementDecisionInput {
    pub requirement_id: String,
    pub decision: RequirementDecisionParameters,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementConfirmPreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub decisions: Vec<RequirementDecisionInput>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementExtractInput {
    pub category: String,
    pub statement: String,
    pub priority: RequirementPriorityV3,
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementExtractPreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub source: ContentRevisionReferenceV3,
    pub requirements: Vec<RequirementExtractInput>,
    #[schemars(description = "True only after explicit consent to read a private local Source")]
    pub confirmed_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementExtractCommitParameters {
    pub application_id: String,
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    pub approved: bool,
    #[schemars(description = "True only after explicit consent to read a private local Source")]
    pub confirmed_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedDeliverableInput {
    pub kind: String,
    pub disposition: PlannedDeliverableDispositionV3,
    pub rationale: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub execution_mode: Option<ExecutionMode>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanProposePreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub decision: String,
    pub deliverables: Vec<PlannedDeliverableInput>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RevisionPreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableDraftInput {
    pub kind: String,
    pub title: String,
    pub media_type: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableDraftPreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub deliverables: Vec<DeliverableDraftInput>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableRevisePreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub deliverable_id: String,
    pub title: String,
    pub media_type: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationMutationCommitParameters {
    pub application_id: String,
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    #[schemars(description = "True only after explicit user approval of the exact preview")]
    pub approved: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableAuditParameters {
    pub application_id: String,
    #[schemars(
        description = "True only after explicit consent to read private Deliverable bodies"
    )]
    pub confirmed_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewInspectParameters {
    pub application_id: String,
    #[schemars(
        description = "True only after explicit consent to read private Deliverable bodies"
    )]
    pub confirmed_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewDispositionPreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    #[schemars(
        description = "True only after explicit consent to read private Deliverable bodies"
    )]
    pub confirmed_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewDispositionCommitParameters {
    pub application_id: String,
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    pub approved: bool,
    pub confirmed_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportPreparePreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub destination: String,
    #[schemars(description = "True only after explicit consent to export private artifacts")]
    pub confirmed_private_export: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportShowParameters {
    pub application_id: String,
    pub destination: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportPrepareCommitParameters {
    pub application_id: String,
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    pub approved: bool,
    pub confirmed_private_export: bool,
}

impl CanISendMcpServer {
    pub fn open(workspace: &Path) -> Result<Self, ApplicationError> {
        let workspace = Application::resolve_workspace_root_v4(Some(workspace))?;
        Ok(Self {
            workspace: Arc::new(workspace),
            association_approvals: AssociationApprovalBrokerV4::default(),
            mutation_approvals: ApplicationMutationApprovalBrokerV4::default(),
        })
    }

    #[must_use]
    pub fn workspace(&self) -> &Path {
        self.workspace.as_path()
    }

    fn application_result<T: Serialize>(
        result: Result<T, ApplicationError>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
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
                Ok(Json(McpStructuredOutput(value)))
            }
            Err(error) => {
                let failure = error.classify();
                let data = serde_json::to_value(&failure).ok();
                Err(McpError::invalid_params(failure.message, data))
            }
        }
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

    fn parse_application_id(application_id: &str) -> Result<ApplicationId, McpError> {
        Self::validate_application_id(application_id)?;
        ApplicationId::try_new(application_id)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))
    }

    fn validate_requirement_id(requirement_id: &str) -> Result<(), McpError> {
        RequirementId::try_new(requirement_id)
            .map(|_| ())
            .map_err(|error| McpError::invalid_params(error.to_string(), None))
    }

    fn validate_deliverable_id(deliverable_id: &str) -> Result<(), McpError> {
        DeliverableId::try_new(deliverable_id)
            .map(|_| ())
            .map_err(|error| McpError::invalid_params(error.to_string(), None))
    }

    fn association_result<T: Serialize>(
        result: Result<T, AssociationApprovalErrorV4>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        match result {
            Ok(value) => Self::application_result(Ok(value)),
            Err(AssociationApprovalErrorV4::Application(error)) => {
                Self::application_result::<T>(Err(error))
            }
            Err(AssociationApprovalErrorV4::Approval(error)) => Err(McpError::invalid_params(
                error.to_string(),
                Some(serde_json::json!({"code": approval_error_code(&error)})),
            )),
            Err(AssociationApprovalErrorV4::Denied) => Err(McpError::invalid_params(
                "association approval was denied",
                Some(serde_json::json!({"code": "approval.denied"})),
            )),
            Err(AssociationApprovalErrorV4::BindingMismatch) => Err(McpError::invalid_params(
                "association approval does not match the reviewed Application or preview",
                Some(serde_json::json!({"code": "approval.binding-mismatch"})),
            )),
        }
    }

    fn mutation_result<T: Serialize>(
        result: Result<T, ApplicationMutationApprovalErrorV4>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        match result {
            Ok(value) => Self::application_result(Ok(value)),
            Err(ApplicationMutationApprovalErrorV4::Application(error)) => {
                Self::application_result::<T>(Err(error))
            }
            Err(ApplicationMutationApprovalErrorV4::Approval(error)) => {
                Err(McpError::invalid_params(
                    error.to_string(),
                    Some(serde_json::json!({"code": approval_error_code(&error)})),
                ))
            }
            Err(ApplicationMutationApprovalErrorV4::Denied) => Err(McpError::invalid_params(
                "Application mutation approval was denied",
                Some(serde_json::json!({"code": "approval.denied"})),
            )),
            Err(ApplicationMutationApprovalErrorV4::BindingMismatch) => {
                Err(McpError::invalid_params(
                    "Application mutation approval does not match the reviewed operation or preview",
                    Some(serde_json::json!({"code": "approval.binding-mismatch"})),
                ))
            }
        }
    }

    fn revision(value: u64) -> Result<Revision, McpError> {
        Revision::try_new(value).map_err(|error| McpError::invalid_params(error.to_string(), None))
    }

    fn pack_item(value: &str) -> Result<WorkflowPackItemId, McpError> {
        WorkflowPackItemId::try_new(value)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))
    }
}

fn approval_error_code(error: &ApprovalBrokerError) -> &'static str {
    match error {
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
    }
}

pub fn serve_stdio(workspace: Option<&Path>) -> Result<(), McpServerError> {
    let workspace = Application::resolve_workspace_root_v4(workspace)?;
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
        description = "Return authoritative Workspace v4 status without private bodies",
        annotations(
            title = "Inspect Workspace status",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_workspace_status(&self) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::application_result(Application::workspace_status_v4(self.workspace()))
    }

    #[tool(
        description = "Check Workspace v4 database, Blob, freshness, and projection invariants",
        annotations(
            title = "Check Workspace integrity",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_workspace_check(&self) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::application_result(Application::check_workspace_v4(self.workspace()))
    }

    #[tool(
        description = "List Pack-bound Applications from authoritative Workspace v4 state",
        annotations(
            title = "List Workspace Applications",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_application_list(&self) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::application_result(Application::list_application_models_v4(self.workspace()))
    }

    #[tool(
        description = "Show one Pack-bound Application from authoritative Workspace v4 state",
        annotations(
            title = "Show Workspace Application",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_application_show(
        &self,
        Parameters(parameters): Parameters<ApplicationParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::validate_application_id(&parameters.application_id)?;
        Self::application_result(Application::application_model_v4(
            self.workspace(),
            &parameters.application_id,
        ))
    }

    #[tool(
        description = "List Pack-bound Requirements for one exact Application revision",
        annotations(
            title = "List Application Requirements",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_requirement_list(
        &self,
        Parameters(parameters): Parameters<ApplicationParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::list_requirements_v4(
            self.workspace(),
            &parameters.application_id,
        ))
    }

    #[tool(
        description = "Show one Requirement owned by the selected Application and Pack binding",
        annotations(
            title = "Show Application Requirement",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_requirement_show(
        &self,
        Parameters(parameters): Parameters<ApplicationRequirementParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::parse_application_id(&parameters.application_id)?;
        Self::validate_requirement_id(&parameters.requirement_id)?;
        Self::application_result(Application::show_requirement_v4(
            self.workspace(),
            &parameters.application_id,
            &parameters.requirement_id,
        ))
    }

    #[tool(
        description = "Preview exact Source-bound Requirement proposals and issue a single-use approval token",
        annotations(
            title = "Preview Requirement extraction",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_requirement_extract_preview(
        &self,
        Parameters(parameters): Parameters<RequirementExtractPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        let requirements = parameters
            .requirements
            .into_iter()
            .map(|requirement| {
                Ok(ApplicationFlowRequirementDraftV3 {
                    category: Self::pack_item(&requirement.category)?,
                    statement: requirement.statement,
                    priority: requirement.priority,
                    start_byte: requirement.start_byte,
                    end_byte: requirement.end_byte,
                })
            })
            .collect::<Result<Vec<_>, McpError>>()?;
        Self::mutation_result(
            self.mutation_approvals.preview_requirement_extraction(
                self.workspace(),
                &application_id,
                ApplicationRequirementExtractRequestV4 {
                    expected_revision: Self::revision(parameters.expected_revision)?,
                    source: parameters.source,
                    requirements,
                },
                parameters
                    .confirmed_private_read
                    .then_some(PrivateReadConsent::granted_by_user()),
            ),
        )
    }

    #[tool(
        description = "Commit approved exact Source-bound Requirement proposals; the preview token is single-use",
        annotations(
            title = "Commit Requirement extraction",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_requirement_extract_commit(
        &self,
        Parameters(parameters): Parameters<RequirementExtractCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.commit_requirement_extraction(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.approved,
                parameters
                    .confirmed_private_read
                    .then_some(PrivateReadConsent::granted_by_user()),
            ),
        )
    }

    #[tool(
        description = "Preview explicit decisions for every current Requirement and issue a single-use approval token",
        annotations(
            title = "Preview Requirement decisions",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_requirement_confirm_preview(
        &self,
        Parameters(parameters): Parameters<RequirementConfirmPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        let mut decisions = BTreeMap::new();
        for decision in parameters.decisions {
            let requirement_id = RequirementId::try_new(decision.requirement_id)
                .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
            let decision = match decision.decision {
                RequirementDecisionParameters::Confirm => RequirementDecisionV4::Confirm,
                RequirementDecisionParameters::Exclude => RequirementDecisionV4::Exclude,
            };
            if decisions.insert(requirement_id, decision).is_some() {
                return Err(McpError::invalid_params(
                    "Requirement decisions contain a duplicate Requirement ID",
                    None,
                ));
            }
        }
        Self::mutation_result(self.mutation_approvals.preview_requirement_confirmation(
            self.workspace(),
            &application_id,
            ApplicationRequirementConfirmRequestV4 {
                expected_revision: Self::revision(parameters.expected_revision)?,
                decisions,
            },
        ))
    }

    #[tool(
        description = "Commit explicitly approved Requirement decisions; the preview token is single-use",
        annotations(
            title = "Commit Requirement decisions",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_requirement_confirm_commit(
        &self,
        Parameters(parameters): Parameters<ApplicationMutationCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_requirement_confirmation(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.approved,
        ))
    }

    #[tool(
        description = "Show the current Pack-bound Plan or an explicit not-created state for one Application",
        annotations(
            title = "Show Application Plan",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_plan_show(
        &self,
        Parameters(parameters): Parameters<ApplicationParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::show_plan_v4(
            self.workspace(),
            &parameters.application_id,
        ))
    }

    #[tool(
        description = "Preview a Pack-qualified draft Plan after all Requirements have explicit decisions",
        annotations(
            title = "Preview a Plan proposal",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_plan_propose_preview(
        &self,
        Parameters(parameters): Parameters<PlanProposePreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        let deliverables = parameters
            .deliverables
            .into_iter()
            .map(|deliverable| {
                Ok(ApplicationFlowPlannedDeliverableV3 {
                    kind: Self::pack_item(&deliverable.kind)?,
                    disposition: deliverable.disposition,
                    rationale: deliverable.rationale,
                    constraints: deliverable.constraints,
                    execution_mode: deliverable.execution_mode,
                })
            })
            .collect::<Result<Vec<_>, McpError>>()?;
        Self::mutation_result(self.mutation_approvals.preview_plan_proposal(
            self.workspace(),
            &application_id,
            ApplicationPlanProposeRequestV4 {
                expected_revision: Self::revision(parameters.expected_revision)?,
                decision: Self::pack_item(&parameters.decision)?,
                deliverables,
            },
        ))
    }

    #[tool(
        description = "Commit one approved draft Plan proposal; the preview token is single-use",
        annotations(
            title = "Commit a Plan proposal",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_plan_propose_commit(
        &self,
        Parameters(parameters): Parameters<ApplicationMutationCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_plan_proposal(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.approved,
        ))
    }

    #[tool(
        description = "Preview explicit user confirmation of the exact current draft Plan",
        annotations(
            title = "Preview Plan confirmation",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_plan_confirm_preview(
        &self,
        Parameters(parameters): Parameters<RevisionPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.preview_plan_confirmation(
            self.workspace(),
            &application_id,
            ApplicationPlanConfirmRequestV4 {
                expected_revision: Self::revision(parameters.expected_revision)?,
            },
        ))
    }

    #[tool(
        description = "Commit explicit user confirmation of the current Plan; the preview token is single-use",
        annotations(
            title = "Commit Plan confirmation",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_plan_confirm_commit(
        &self,
        Parameters(parameters): Parameters<ApplicationMutationCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_plan_confirmation(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.approved,
        ))
    }

    #[tool(
        description = "List body-free Pack-bound Deliverable metadata for one exact Application revision",
        annotations(
            title = "List Application Deliverables",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_deliverable_list(
        &self,
        Parameters(parameters): Parameters<ApplicationParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::list_deliverables_v4(
            self.workspace(),
            &parameters.application_id,
        ))
    }

    #[tool(
        description = "Show one body-free Deliverable metadata record owned by the selected Application",
        annotations(
            title = "Show Application Deliverable",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_deliverable_show(
        &self,
        Parameters(parameters): Parameters<ApplicationDeliverableParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::parse_application_id(&parameters.application_id)?;
        Self::validate_deliverable_id(&parameters.deliverable_id)?;
        Self::application_result(Application::show_deliverable_v4(
            self.workspace(),
            &parameters.application_id,
            &parameters.deliverable_id,
        ))
    }

    #[tool(
        description = "Preview Pack-qualified private Deliverable drafts without mutating the Application",
        annotations(
            title = "Preview Deliverable drafts",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_deliverable_draft_preview(
        &self,
        Parameters(parameters): Parameters<DeliverableDraftPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        let deliverables = parameters
            .deliverables
            .into_iter()
            .map(|deliverable| {
                Ok(ApplicationFlowDeliverableDraftV3 {
                    kind: Self::pack_item(&deliverable.kind)?,
                    title: deliverable.title,
                    media_type: deliverable.media_type,
                    content: deliverable.content,
                })
            })
            .collect::<Result<Vec<_>, McpError>>()?;
        Self::mutation_result(self.mutation_approvals.preview_deliverable_draft(
            self.workspace(),
            &application_id,
            ApplicationFlowComposeRequestV3 {
                expected_revision: Self::revision(parameters.expected_revision)?,
                deliverables,
            },
        ))
    }

    #[tool(
        description = "Commit explicitly approved Deliverable drafts; the preview token is single-use",
        annotations(
            title = "Commit Deliverable drafts",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_deliverable_draft_commit(
        &self,
        Parameters(parameters): Parameters<ApplicationMutationCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_deliverable_draft(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.approved,
        ))
    }

    #[tool(
        description = "Preview a private Deliverable content revision for one exact Application",
        annotations(
            title = "Preview a Deliverable revision",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_deliverable_revise_preview(
        &self,
        Parameters(parameters): Parameters<DeliverableRevisePreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        let deliverable_id = DeliverableId::try_new(parameters.deliverable_id)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        Self::mutation_result(self.mutation_approvals.preview_deliverable_revision(
            self.workspace(),
            &application_id,
            ApplicationDeliverableReviseRequestV4 {
                expected_revision: Self::revision(parameters.expected_revision)?,
                deliverable_id,
                title: parameters.title,
                media_type: parameters.media_type,
                content: parameters.content,
            },
        ))
    }

    #[tool(
        description = "Commit one explicitly approved Deliverable revision; the preview token is single-use",
        annotations(
            title = "Commit a Deliverable revision",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_deliverable_revise_commit(
        &self,
        Parameters(parameters): Parameters<ApplicationMutationCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_deliverable_revision(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.approved,
        ))
    }

    #[tool(
        description = "Read current private Deliverable bodies only after explicit local private-read consent",
        annotations(
            title = "Audit Deliverable bodies",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_deliverable_audit(
        &self,
        Parameters(parameters): Parameters<DeliverableAuditParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::audit_deliverables_v4(
            self.workspace(),
            &application_id,
            parameters
                .confirmed_private_read
                .then(PrivateReadConsent::granted_by_user),
        ))
    }

    #[tool(
        description = "Inspect exact current Deliverables for evidence-bound review after private-read consent",
        annotations(
            title = "Inspect Application review",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_review_inspect(
        &self,
        Parameters(parameters): Parameters<ReviewInspectParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::inspect_review_v4(
            self.workspace(),
            &application_id,
            parameters
                .confirmed_private_read
                .then(PrivateReadConsent::granted_by_user),
        ))
    }

    #[tool(
        description = "Preview approval of all exact current review-required Deliverables",
        annotations(
            title = "Preview review disposition",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_review_disposition_preview(
        &self,
        Parameters(parameters): Parameters<ReviewDispositionPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.preview_review_disposition(
                self.workspace(),
                &application_id,
                ApplicationFlowApproveRequestV3 {
                    expected_revision: Self::revision(parameters.expected_revision)?,
                },
                parameters
                    .confirmed_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "Commit the exact approved review disposition; the preview token is single-use",
        annotations(
            title = "Commit review disposition",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_review_disposition_commit(
        &self,
        Parameters(parameters): Parameters<ReviewDispositionCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.commit_review_disposition(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.approved,
                parameters
                    .confirmed_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "Preview one exact local-only export of approved Deliverables",
        annotations(
            title = "Preview local export",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_export_prepare_preview(
        &self,
        Parameters(parameters): Parameters<ExportPreparePreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        let request = ApplicationFlowExportRequestV3::try_new(
            &parameters.application_id,
            parameters.expected_revision,
            &parameters.destination,
        )
        .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        Self::mutation_result(
            self.mutation_approvals.preview_export_prepare(
                self.workspace(),
                &application_id,
                request,
                parameters
                    .confirmed_private_export
                    .then(PrivateExportConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "List verified local exports for one exact Application",
        annotations(
            title = "List local exports",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_export_list(
        &self,
        Parameters(parameters): Parameters<ApplicationParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::list_exports_v4(
            self.workspace(),
            &parameters.application_id,
        ))
    }

    #[tool(
        description = "Load and verify one exact local export manifest and every document digest",
        annotations(
            title = "Show local export",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_export_show(
        &self,
        Parameters(parameters): Parameters<ExportShowParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::show_export_v4(
            self.workspace(),
            &parameters.application_id,
            &parameters.destination,
        ))
    }

    #[tool(
        description = "Render and write the exact approved local export; never upload or submit",
        annotations(
            title = "Commit local export",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_export_prepare_commit(
        &self,
        Parameters(parameters): Parameters<ExportPrepareCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.commit_export_prepare(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.approved,
                parameters
                    .confirmed_private_export
                    .then(PrivateExportConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "List body-free Workspace Profile Source metadata from authoritative Workspace v4 state",
        annotations(
            title = "List Workspace Profile Sources",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_profile_source_list(&self) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::application_result(Application::list_profile_sources_v4(self.workspace()))
    }

    #[tool(
        description = "List body-free Workspace Profile Sources and explicit links for one Application",
        annotations(
            title = "List Application Profile Source links",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_profile_association_list(
        &self,
        Parameters(parameters): Parameters<ApplicationParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::validate_application_id(&parameters.application_id)?;
        Self::application_result(Application::list_profile_associations_v4(
            self.workspace(),
            &parameters.application_id,
        ))
    }

    #[tool(
        description = "Preview an exact Application Profile Source link change and issue a bounded single-use approval token",
        annotations(
            title = "Preview an Application Profile Source link change",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_profile_association_preview(
        &self,
        Parameters(parameters): Parameters<ProfileAssociationPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::association_result(self.association_approvals.preview_profile(
            self.workspace(),
            ProfileAssociationPreviewRequestV4 {
                application_id,
                profile_source: parameters.profile_source,
                change: parameters.change.into(),
            },
        ))
    }

    #[tool(
        description = "Commit one explicitly approved Profile Source link preview; the token is single-use",
        annotations(
            title = "Commit an approved Application Profile Source link change",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_profile_association_commit(
        &self,
        Parameters(parameters): Parameters<AssociationCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::association_result(
            self.association_approvals.commit_profile(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.approved,
                parameters
                    .confirmed_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "List body-free confirmed Workspace Evidence and explicit links for one Application",
        annotations(
            title = "List Application Evidence links",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_evidence_association_list(
        &self,
        Parameters(parameters): Parameters<ApplicationParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::validate_application_id(&parameters.application_id)?;
        Self::application_result(Application::list_evidence_associations_v4(
            self.workspace(),
            &parameters.application_id,
        ))
    }

    #[tool(
        description = "Preview an exact Application Evidence link change and issue a bounded single-use approval token",
        annotations(
            title = "Preview an Application Evidence link change",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_evidence_association_preview(
        &self,
        Parameters(parameters): Parameters<EvidenceAssociationPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::association_result(self.association_approvals.preview_evidence(
            self.workspace(),
            EvidenceAssociationPreviewRequestV4 {
                application_id,
                evidence: parameters.evidence,
                change: parameters.change.into(),
            },
        ))
    }

    #[tool(
        description = "Commit one explicitly approved Evidence link preview; the token is single-use",
        annotations(
            title = "Commit an approved Application Evidence link change",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_evidence_association_commit(
        &self,
        Parameters(parameters): Parameters<AssociationCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = Self::parse_application_id(&parameters.application_id)?;
        Self::association_result(
            self.association_approvals.commit_evidence(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.approved,
                parameters
                    .confirmed_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }
}

#[tool_handler(
    name = "canisend",
    instructions = "CanISend opens only clean Workspace v4 state. Applications bind an exact workflow Pack; a Workspace itself is domain-neutral. Routine context is body-free. Guarded association changes require preview, exact digest review, explicit approval and consent, and a single-use token. CanISend never uploads or submits an Application. Never edit .canisend, SQLite, immutable Blobs, or managed projections directly."
)]
impl ServerHandler for CanISendMcpServer {}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_app::{Application, CANISEND_MCP_TOOLS};

    use super::CanISendMcpServer;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-mcp-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn public_router_is_exactly_the_clean_v4_read_surface() {
        let names = CanISendMcpServer::tool_router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, CANISEND_MCP_TOOLS);
    }

    #[test]
    fn opens_v4_and_refuses_legacy_workspace_formats() {
        let root = temporary_root("v4");
        Application::initialize_workspace_v4(&root).expect("initialize Workspace v4");
        let server = CanISendMcpServer::open(&root).expect("open Workspace v4");
        assert_eq!(server.workspace(), root.as_path());
        fs::remove_dir_all(root).expect("remove Workspace v4");

        let legacy = temporary_root("legacy");
        Application::initialize_workspace_v3(&legacy).expect("initialize Workspace v3");
        assert!(CanISendMcpServer::open(&legacy).is_err());
        fs::remove_dir_all(legacy).expect("remove Workspace v3");
    }

    #[test]
    fn application_ids_are_bounded_before_storage_access() {
        assert!(CanISendMcpServer::validate_application_id("").is_err());
        assert!(CanISendMcpServer::validate_application_id(&"a".repeat(129)).is_err());
        assert!(CanISendMcpServer::validate_application_id("app-123").is_ok());
    }
}
