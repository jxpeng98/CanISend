#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use canisend_app::{
    Application, ApplicationDeliverableReviseRequestV4, ApplicationError,
    ApplicationFlowApproveRequestV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowDeliverableDraftV3, ApplicationFlowExportRequestV3,
    ApplicationFlowPlannedDeliverableV3, ApplicationFlowRequirementDraftV3,
    ApplicationMutationApprovalBrokerV4, ApplicationMutationApprovalErrorV4,
    ApplicationPlanConfirmRequestV4, ApplicationPlanProposeRequestV4,
    ApplicationRequirementConfirmRequestV4, ApplicationRequirementExtractRequestV4,
    ApplicationRequirementReviseRequestV4, ApprovalBrokerError, ApprovalKind,
    AssociationApprovalBrokerV4, AssociationApprovalErrorV4, AssociationChangeV4,
    EvidenceApprovalBrokerV4, EvidenceApprovalErrorV4, EvidenceAssociationPreviewRequestV4,
    PrivateExportConsent, PrivateReadConsent, ProfileAssociationPreviewRequestV4,
    RequirementDecisionV4,
};
use canisend_contracts::{
    ApplicationId, ContentRevisionReferenceV3, DeliverableId, EvidenceProposalSet, ExecutionMode,
    PlannedDeliverableDispositionV3, RequirementId, RequirementPriorityV3, Revision, Sha256Digest,
    WorkflowPackItemId,
};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    handler::server::{
        tool::ToolCallContext,
        wrapper::{Json, Parameters},
    },
    model::{CallToolRequestParams, CallToolResponse, ElicitRequestParams, ElicitationAction},
    service::{ElicitationMode, RequestContext},
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
    application_id: Option<ApplicationId>,
    association_approvals: AssociationApprovalBrokerV4,
    mutation_approvals: ApplicationMutationApprovalBrokerV4,
    evidence_approvals: EvidenceApprovalBrokerV4,
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
pub struct EvidenceConfirmPreviewParameters {
    pub application_id: String,
    pub profile_source: ContentRevisionReferenceV3,
    pub proposals: EvidenceProposalSet,
    #[schemars(
        description = "Request separate native private-read consent for the selected Profile Source; this flag does not grant consent."
    )]
    pub request_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AssociationCommitParameters {
    #[schemars(description = "CanISend Application ID bound to the preview")]
    pub application_id: String,
    #[schemars(description = "Opaque single-use preview token")]
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    #[schemars(
        description = "Request the native confirmation form; only its actual acceptance authorizes this change. False cancels the preview."
    )]
    pub request_confirmation: bool,
    #[schemars(
        description = "Request separate native private-read consent; this flag does not grant consent."
    )]
    pub request_private_read: bool,
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
    #[schemars(
        description = "Request separate native private-read consent; this flag does not grant consent."
    )]
    pub request_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementExtractCommitParameters {
    pub application_id: String,
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    #[schemars(
        description = "Request the native confirmation form; only its actual acceptance authorizes this change. False cancels the preview."
    )]
    pub request_confirmation: bool,
    #[schemars(
        description = "Request separate native private-read consent; this flag does not grant consent."
    )]
    pub request_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementRevisePreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub requirement_id: String,
    pub source: ContentRevisionReferenceV3,
    pub requirement: RequirementExtractInput,
    #[schemars(
        description = "Request separate native private-read consent; this flag does not grant consent."
    )]
    pub request_private_read: bool,
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
pub struct LocalTaskDraftPreviewParameters {
    pub application_id: String,
    pub task_id: String,
    pub expected_generation: u64,
    pub candidate_sha256: Sha256Digest,
    #[schemars(
        description = "Request native private-read consent for the exact stored candidate; this flag does not grant consent."
    )]
    pub request_private_read: bool,
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
    #[schemars(
        description = "Request the native confirmation form; only its actual acceptance authorizes this change. False cancels the preview."
    )]
    pub request_confirmation: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableAuditParameters {
    pub application_id: String,
    #[schemars(
        description = "Request separate native private-read consent; this flag does not grant consent."
    )]
    pub request_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewInspectParameters {
    pub application_id: String,
    #[schemars(
        description = "Request separate native private-read consent; this flag does not grant consent."
    )]
    pub request_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewDispositionPreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    #[schemars(
        description = "Request separate native private-read consent; this flag does not grant consent."
    )]
    pub request_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewDispositionCommitParameters {
    pub application_id: String,
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    #[schemars(
        description = "Request the native confirmation form; only its actual acceptance authorizes this change. False cancels the preview."
    )]
    pub request_confirmation: bool,
    #[schemars(
        description = "Request separate native private-read consent; this flag does not grant consent."
    )]
    pub request_private_read: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportPreparePreviewParameters {
    pub application_id: String,
    pub expected_revision: u64,
    pub destination: String,
    #[schemars(
        description = "Request separate native private-export consent; this flag does not grant consent."
    )]
    pub request_private_export: bool,
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
    #[schemars(
        description = "Request the native confirmation form; only its actual acceptance authorizes this change. False cancels the preview."
    )]
    pub request_confirmation: bool,
    #[schemars(
        description = "Request separate native private-export consent; this flag does not grant consent."
    )]
    pub request_private_export: bool,
}

impl CanISendMcpServer {
    async fn confirm_with_host(
        context: &RequestContext<RoleServer>,
        message: String,
    ) -> Result<(), McpError> {
        let denied = || {
            McpError::invalid_params(
                "Explicit user confirmation through this Host is required; no operation was authorized",
                Some(serde_json::json!({"code": "consent.host-confirmation-required"})),
            )
        };
        if !context
            .peer
            .supported_elicitation_modes()
            .contains(&ElicitationMode::Form)
        {
            return Err(denied());
        }
        let requested_schema = serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {"confirm": {"type": "boolean", "title": "I approve this exact request", "default": false}},
            "required": ["confirm"]
        })).map_err(|_| McpError::internal_error("Invalid confirmation schema", None))?;
        let response = tokio::select! {
            () = context.ct.cancelled() => return Err(denied()),
            response = context.peer.create_elicitation_with_timeout(
                ElicitRequestParams::FormElicitationParams { meta: None, message, requested_schema },
                Some(Duration::from_secs(120)),
            ) => response.map_err(|_| denied())?,
        };
        if context.ct.is_cancelled()
            || response.action != ElicitationAction::Accept
            || response.content != Some(serde_json::json!({"confirm": true}))
        {
            return Err(denied());
        }
        Ok(())
    }

    async fn confirm_request(
        &self,
        request: &CallToolRequestParams,
        context: &RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        let Some(arguments) = request.arguments.as_ref() else {
            return Ok(());
        };
        let Some(tool) = Self::tool_router().get(&request.name).cloned() else {
            return Ok(()); // The existing router owns unknown-tool errors.
        };
        let properties = tool.input_schema.get("properties");
        let has = |field| properties.and_then(|value| value.get(field)).is_some();
        let commit = has("request_confirmation");
        if commit && arguments.get("request_confirmation") != Some(&Value::Bool(true)) {
            return Ok(()); // Explicit cancellation still consumes the existing preview.
        }
        if let Some(application_id) = arguments.get("application_id").and_then(Value::as_str) {
            self.parse_application_id(application_id)?;
        }
        let preview = if commit {
            let mut binding = arguments.clone();
            binding.remove("request_private_read");
            binding.remove("request_private_export");
            let parameters: ApplicationMutationCommitParameters =
                serde_json::from_value(Value::Object(binding))
                    .map_err(|_| McpError::invalid_params("Invalid commit binding", None))?;
            let application_id = self.parse_application_id(&parameters.application_id)?;
            let kind = match request.name.as_ref() {
                "canisend_requirement_extract_commit" => {
                    ApprovalKind::ApplicationRequirementExtraction
                }
                "canisend_requirement_revise_commit" => {
                    ApprovalKind::ApplicationRequirementRevision
                }
                "canisend_requirement_confirm_commit" => {
                    ApprovalKind::ApplicationRequirementConfirmation
                }
                "canisend_plan_propose_commit" => ApprovalKind::ApplicationPlanProposal,
                "canisend_plan_confirm_commit" => ApprovalKind::ApplicationPlanConfirmation,
                "canisend_deliverable_draft_commit" => ApprovalKind::DeliverableDraft,
                "canisend_deliverable_revise_commit" => ApprovalKind::DeliverableRevision,
                "canisend_review_disposition_commit" => ApprovalKind::ReviewDisposition,
                "canisend_export_prepare_commit" => ApprovalKind::ExportPrepare,
                "canisend_profile_association_commit" => ApprovalKind::ProfileAssociation,
                "canisend_evidence_association_commit" => ApprovalKind::EvidenceAssociation,
                "canisend_evidence_confirm_commit" => ApprovalKind::EvidenceConfirmation,
                _ => {
                    return Err(McpError::invalid_params(
                        "Tool has no trusted confirmation binding",
                        None,
                    ));
                }
            };
            let Json(preview) = if kind == ApprovalKind::EvidenceConfirmation {
                let digest = parameters.preview_sha256.clone();
                Self::evidence_result(self.evidence_approvals.confirmation_preview(
                    self.workspace(),
                    &application_id,
                    &parameters.preview_token,
                    &digest,
                ))?
            } else if matches!(
                kind,
                ApprovalKind::ProfileAssociation | ApprovalKind::EvidenceAssociation
            ) {
                Self::association_result(self.association_approvals.confirmation_preview(
                    self.workspace(),
                    &application_id,
                    &parameters.preview_token,
                    &parameters.preview_sha256,
                    kind,
                ))?
            } else {
                Self::mutation_result(self.mutation_approvals.confirmation_preview(
                    self.workspace(),
                    &application_id,
                    &parameters.preview_token,
                    &parameters.preview_sha256,
                    kind,
                ))?
            };
            Some(preview.0)
        } else {
            None
        };
        for (flag, purpose) in [
            (
                "request_private_read",
                "Read the selected private input and return it to this Host/provider",
            ),
            (
                "request_private_export",
                "Read private content for the selected local export; never upload or submit",
            ),
        ] {
            if has(flag) && arguments.get(flag) == Some(&Value::Bool(true)) {
                let subjects = arguments
                    .iter()
                    .filter(|(name, _)| {
                        matches!(
                            name.as_str(),
                            "application_id"
                                | "task_id"
                                | "expected_generation"
                                | "candidate_sha256"
                                | "source"
                                | "profile_source"
                                | "evidence"
                                | "deliverable_id"
                                | "requirement_id"
                                | "destination"
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                let subjects = serde_json::json!(subjects);
                let selected = preview.as_ref().unwrap_or(&subjects);
                Self::confirm_with_host(context, format!(
                    "{purpose}. This consent does not approve a content change.\nWorkspace: {}\nOperation: {}\nSelected inputs: {}",
                    self.workspace().display(), request.name,
                    selected,
                )).await?;
            }
        }
        if let Some(preview) = preview {
            Self::confirm_with_host(context, format!(
                "Approve this exact local change? No submission is performed.\nOperation: {}\nExact change: {}",
                request.name, preview,
            )).await?;
        }
        Ok(())
    }

    pub fn open(workspace: &Path) -> Result<Self, ApplicationError> {
        Self::open_with_application(workspace, None)
    }

    /// Bind all Application tools to one existing Application. This does not grant consent.
    pub fn open_with_application(
        workspace: &Path,
        application_id: Option<&str>,
    ) -> Result<Self, ApplicationError> {
        let workspace = Application::resolve_workspace_root_v4(Some(workspace))?;
        let application_id = application_id
            .map(|id| {
                Self::validate_application_id(id).map_err(|_| {
                    ApplicationError::InvalidInput("Invalid MCP Application binding".to_owned())
                })?;
                Application::application_model_v4(&workspace, id)
                    .map(|receipt| receipt.data.snapshot.application.id)
            })
            .transpose()?;
        Ok(Self {
            workspace: Arc::new(workspace),
            application_id,
            association_approvals: AssociationApprovalBrokerV4::default(),
            mutation_approvals: ApplicationMutationApprovalBrokerV4::default(),
            evidence_approvals: EvidenceApprovalBrokerV4::default(),
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

    fn parse_application_id(&self, application_id: &str) -> Result<ApplicationId, McpError> {
        Self::validate_application_id(application_id)?;
        if self
            .application_id
            .as_ref()
            .is_some_and(|bound| bound.as_str() != application_id)
        {
            return Err(McpError::invalid_params(
                "Tool request does not match the bound Application",
                Some(serde_json::json!({"code": "application.binding-mismatch"})),
            ));
        }
        ApplicationId::try_new(application_id)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))
    }

    fn require_workspace_scope(&self) -> Result<(), McpError> {
        if self.application_id.is_some() {
            return Err(McpError::invalid_params(
                "Workspace-wide results are unavailable in Application-bound MCP sessions",
                Some(serde_json::json!({"code": "application.workspace-scope-required"})),
            ));
        }
        Ok(())
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

    fn evidence_result<T: Serialize>(
        result: Result<T, EvidenceApprovalErrorV4>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        Self::mutation_result(result.map_err(|error| match error {
            EvidenceApprovalErrorV4::Application(error) => {
                ApplicationMutationApprovalErrorV4::Application(error)
            }
            EvidenceApprovalErrorV4::Approval(error) => {
                ApplicationMutationApprovalErrorV4::Approval(error)
            }
            EvidenceApprovalErrorV4::Denied => ApplicationMutationApprovalErrorV4::Denied,
            EvidenceApprovalErrorV4::BindingMismatch => {
                ApplicationMutationApprovalErrorV4::BindingMismatch
            }
        }))
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
    serve_stdio_with_application(workspace, None)
}

pub fn serve_stdio_with_application(
    workspace: Option<&Path>,
    application_id: Option<&str>,
) -> Result<(), McpServerError> {
    let workspace = Application::resolve_workspace_root_v4(workspace)?;
    let server = CanISendMcpServer::open_with_application(&workspace, application_id)?;
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
        self.require_workspace_scope()?;
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
        self.require_workspace_scope()?;
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
        self.require_workspace_scope()?;
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
        self.parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::application_model_v4(
            self.workspace(),
            &parameters.application_id,
        ))
    }

    #[tool(
        description = "Read the complete verified Pack manifest bound to this Application, including every Deliverable kind and minimum/maximum count",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn canisend_application_pack_show(
        &self,
        Parameters(parameters): Parameters<ApplicationParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::application_pack_manifest_v4(
            self.workspace(),
            application_id.as_str(),
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
        self.parse_application_id(&parameters.application_id)?;
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
        self.parse_application_id(&parameters.application_id)?;
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
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
                    .request_private_read
                    .then_some(PrivateReadConsent::granted_by_user()),
            ),
        )
    }

    #[tool(
        description = "Request native confirmation, then commit exact Source-bound Requirement proposals; the preview token is single-use",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.commit_requirement_extraction(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.request_confirmation,
                parameters
                    .request_private_read
                    .then_some(PrivateReadConsent::granted_by_user()),
            ),
        )
    }

    #[tool(
        description = "Preview a Source-bound correction to one existing Requirement and exact downstream stale revisions. Unchanged content returns preview.status=unchanged without an approval token.",
        annotations(
            title = "Preview Requirement revision",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_requirement_revise_preview(
        &self,
        Parameters(parameters): Parameters<RequirementRevisePreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = self.parse_application_id(&parameters.application_id)?;
        let requirement_id = RequirementId::try_new(parameters.requirement_id)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        let draft = parameters.requirement;
        Self::mutation_result(
            self.mutation_approvals.preview_requirement_revision(
                self.workspace(),
                &application_id,
                ApplicationRequirementReviseRequestV4 {
                    expected_revision: Self::revision(parameters.expected_revision)?,
                    requirement_id,
                    source: parameters.source,
                    requirement: ApplicationFlowRequirementDraftV3 {
                        category: Self::pack_item(&draft.category)?,
                        statement: draft.statement,
                        priority: draft.priority,
                        start_byte: draft.start_byte,
                        end_byte: draft.end_byte,
                    },
                },
                parameters
                    .request_private_read
                    .then_some(PrivateReadConsent::granted_by_user()),
            ),
        )
    }

    #[tool(
        description = "Request native confirmation, then revise one Requirement, clear its obsolete decision and stale affected downstream work; the preview token is single-use",
        annotations(
            title = "Commit Requirement revision",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_requirement_revise_commit(
        &self,
        Parameters(parameters): Parameters<RequirementExtractCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.commit_requirement_revision(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.request_confirmation,
                parameters
                    .request_private_read
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
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
        description = "Request native confirmation, then commit Requirement decisions; the preview token is single-use",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_requirement_confirmation(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.request_confirmation,
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
        self.parse_application_id(&parameters.application_id)?;
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
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
        description = "Request native confirmation, then commit one draft Plan proposal; the preview token is single-use",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_plan_proposal(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.request_confirmation,
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.preview_plan_confirmation(
            self.workspace(),
            &application_id,
            ApplicationPlanConfirmRequestV4 {
                expected_revision: Self::revision(parameters.expected_revision)?,
            },
        ))
    }

    #[tool(
        description = "Request native user confirmation of the current Plan, then commit only on acceptance; token is single-use",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_plan_confirmation(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.request_confirmation,
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
        self.parse_application_id(&parameters.application_id)?;
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
        self.parse_application_id(&parameters.application_id)?;
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
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
        description = "Read an exact submitted local task candidate after native private-read consent and preview it for the existing Deliverable draft commit",
        annotations(
            title = "Preview local task drafts",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_local_task_draft_preview(
        &self,
        Parameters(parameters): Parameters<LocalTaskDraftPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = self.parse_application_id(&parameters.application_id)?;
        let task_id = canisend_contracts::EntityId::try_new(parameters.task_id)
            .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        Self::mutation_result(
            self.mutation_approvals.preview_local_task_draft(
                self.workspace(),
                &application_id,
                &task_id,
                parameters.expected_generation,
                &parameters.candidate_sha256,
                parameters
                    .request_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "Request native confirmation, then commit Deliverable drafts; the preview token is single-use",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_deliverable_draft(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.request_confirmation,
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
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
        description = "Request native confirmation, then commit one Deliverable revision; the preview token is single-use",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(self.mutation_approvals.commit_deliverable_revision(
            self.workspace(),
            &application_id,
            &parameters.preview_token,
            &parameters.preview_sha256,
            parameters.request_confirmation,
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::audit_deliverables_v4(
            self.workspace(),
            &application_id,
            parameters
                .request_private_read
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::inspect_review_v4(
            self.workspace(),
            &application_id,
            parameters
                .request_private_read
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.preview_review_disposition(
                self.workspace(),
                &application_id,
                ApplicationFlowApproveRequestV3 {
                    expected_revision: Self::revision(parameters.expected_revision)?,
                },
                parameters
                    .request_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "Request native confirmation, then commit the exact review disposition; the preview token is single-use",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.commit_review_disposition(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.request_confirmation,
                parameters
                    .request_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "Preview one exact local-only export of reviewed Deliverables",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
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
                    .request_private_export
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
        self.parse_application_id(&parameters.application_id)?;
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
        self.parse_application_id(&parameters.application_id)?;
        Self::application_result(Application::show_export_v4(
            self.workspace(),
            &parameters.application_id,
            &parameters.destination,
        ))
    }

    #[tool(
        description = "Request native confirmation, then render and write the exact local export; never upload or submit",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::mutation_result(
            self.mutation_approvals.commit_export_prepare(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.request_confirmation,
                parameters
                    .request_private_export
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
        self.require_workspace_scope()?;
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
        self.parse_application_id(&parameters.application_id)?;
        self.require_workspace_scope()?;
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
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
        description = "Request native confirmation, then commit one Profile Source link preview; the token is single-use",
        annotations(
            title = "Commit an Application Profile Source link change",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::association_result(
            self.association_approvals.commit_profile(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.request_confirmation,
                parameters
                    .request_private_read
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
        self.parse_application_id(&parameters.application_id)?;
        self.require_workspace_scope()?;
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
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
        description = "Request native confirmation, then commit one Evidence link preview; the token is single-use",
        annotations(
            title = "Commit an Application Evidence link change",
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
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::association_result(
            self.association_approvals.commit_evidence(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &parameters.preview_sha256,
                parameters.request_confirmation,
                parameters
                    .request_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "Preview source-bound Workspace Evidence from the selected Profile Source; requests native private-read consent and does not confirm or associate Evidence",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_evidence_confirm_preview(
        &self,
        Parameters(parameters): Parameters<EvidenceConfirmPreviewParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = self.parse_application_id(&parameters.application_id)?;
        Self::evidence_result(
            self.evidence_approvals.preview(
                self.workspace(),
                &application_id,
                parameters.profile_source,
                parameters.proposals,
                parameters
                    .request_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }

    #[tool(
        description = "Request native confirmation, then create the exact previewed Workspace Evidence; does not associate it with an Application",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn canisend_evidence_confirm_commit(
        &self,
        Parameters(parameters): Parameters<ReviewDispositionCommitParameters>,
    ) -> Result<Json<McpStructuredOutput>, McpError> {
        let application_id = self.parse_application_id(&parameters.application_id)?;
        let digest = parameters.preview_sha256;
        Self::evidence_result(
            self.evidence_approvals.commit(
                self.workspace(),
                &application_id,
                &parameters.preview_token,
                &digest,
                parameters.request_confirmation,
                parameters
                    .request_private_read
                    .then(PrivateReadConsent::granted_by_user),
            ),
        )
    }
}

#[tool_handler(
    name = "canisend",
    instructions = "CanISend opens only clean Workspace v4 state. Applications bind an exact workflow Pack; a Workspace itself is domain-neutral. Routine context is body-free. Guarded changes require an exact preview and a single-use token. request_confirmation and request_private_read/export request native forms; they never assert user approval. Only the actual native form response authorizes the selected scope. Never answer it for the user. CanISend never uploads or submits an Application. Never edit .canisend, SQLite, immutable Blobs, or managed projections directly."
)]
impl ServerHandler for CanISendMcpServer {
    // The router is entered only after native consent succeeds, or to consume a cancellation.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        if let Err(error) = self.confirm_request(&request, &context).await {
            // Reuse the owning cancellation path; do not leave a declined commit reusable.
            if request
                .arguments
                .as_ref()
                .is_some_and(|args| args.contains_key("request_confirmation"))
            {
                let mut cancelled = request.clone();
                if let Some(arguments) = cancelled.arguments.as_mut() {
                    arguments.insert("request_confirmation".to_owned(), Value::Bool(false));
                }
                let _ = Self::tool_router()
                    .call(ToolCallContext::new(self, cancelled, context))
                    .await;
            }
            return Err(error);
        }
        Self::tool_router()
            .call(ToolCallContext::new(self, request, context))
            .await
    }
}

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
