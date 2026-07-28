use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, AgentCapabilitiesReadModel, AgentContextReadModel, AgentHost,
    AgentPackExportReadModel, AgentPackExportRequest, Application,
};
use serde::Deserialize;

use crate::commands::{DesktopCommandError, run_worker};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentContextRequest {
    workspace: Option<PathBuf>,
    selected_job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentExportRequest {
    host: AgentHost,
    destination: PathBuf,
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn agent_capabilities()
-> Result<ActionReceipt<AgentCapabilitiesReadModel>, DesktopCommandError> {
    run_worker(|| Application::agent_capabilities().map_err(DesktopCommandError::application)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn agent_context(
    request: AgentContextRequest,
) -> Result<ActionReceipt<AgentContextReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::agent_context(
            request.workspace.as_deref(),
            request.selected_job_id.as_deref(),
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn export_agent_pack(
    request: AgentExportRequest,
) -> Result<ActionReceipt<AgentPackExportReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::export_agent_assets(&AgentPackExportRequest::new(
            request.host,
            request.destination,
        ))
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_commands_preserve_body_free_shared_contracts() {
        let capabilities = Application::agent_capabilities().expect("agent capabilities");
        assert_eq!(capabilities.operation, "agent.capabilities");
        assert!(!capabilities.data.capabilities.is_empty());

        let context = Application::agent_context(None, None).expect("agent context");
        assert_eq!(context.operation, "agent.context");
        assert!(context.data.workspace.is_none());
        assert!(!context.data.blockers.is_empty());
    }
}
