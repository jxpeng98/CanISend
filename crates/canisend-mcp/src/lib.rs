#![forbid(unsafe_code)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use canisend_app::{Application, ApplicationError};
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
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationParameters {
    #[schemars(description = "CanISend Application ID")]
    pub application_id: String,
}

impl CanISendMcpServer {
    pub fn open(workspace: &Path) -> Result<Self, ApplicationError> {
        let workspace = Application::resolve_workspace_root_v4(Some(workspace))?;
        Ok(Self {
            workspace: Arc::new(workspace),
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
}

#[tool_handler(
    name = "canisend",
    instructions = "CanISend opens only clean Workspace v4 state. Applications bind an exact workflow Pack; a Workspace itself is domain-neutral. Routine context is body-free. This Alpha.7 MCP surface is read-only; guarded v4 mutations are exposed only after their preview, approval, revision, and audit contracts are complete. CanISend never uploads or submits an Application. Never edit .canisend, SQLite, immutable Blobs, or managed projections directly."
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
