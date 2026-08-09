#![forbid(unsafe_code)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use canisend_app::{
    Application, ApplicationError, ApprovalBrokerError, AssociationApprovalBrokerV4,
    AssociationApprovalErrorV4, AssociationChangeV4, EvidenceAssociationPreviewRequestV4,
    PrivateReadConsent, ProfileAssociationPreviewRequestV4,
};
use canisend_contracts::{
    ApplicationId, ContentRevisionReferenceV3, DeliverableId, RequirementId, Sha256Digest,
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

impl CanISendMcpServer {
    pub fn open(workspace: &Path) -> Result<Self, ApplicationError> {
        let workspace = Application::resolve_workspace_root_v4(Some(workspace))?;
        Ok(Self {
            workspace: Arc::new(workspace),
            association_approvals: AssociationApprovalBrokerV4::default(),
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
