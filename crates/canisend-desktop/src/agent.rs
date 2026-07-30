use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use canisend_app::{
    ActionReceipt, AgentAssistanceReadModel, AgentCapabilitiesReadModel, AgentContextReadModel,
    AgentHandoffReadModel, AgentHandoffRequest, AgentHost, AgentMcpConfigurationReadModel,
    AgentMcpConfigurationRequest, AgentPackExportReadModel, AgentPackExportRequest,
    AgentSkillsInstallReadModel, AgentSkillsInstallRequest, Application, bundled_cli_path,
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
pub(crate) struct AgentAssistanceRequest {
    workspace: PathBuf,
    job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentExportRequest {
    host: AgentHost,
    destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrepareAgentHandoffRequest {
    host: AgentHost,
    workspace: PathBuf,
    selected_job_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AgentHandoffClipboardField {
    LaunchCommand,
    StartCommand,
    BootstrapPrompt,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CopyAgentHandoffRequest {
    host: AgentHost,
    workspace: PathBuf,
    selected_job_id: Option<String>,
    field: AgentHandoffClipboardField,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrepareAgentMcpConfigurationRequest {
    host: AgentHost,
    workspace: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstallAgentSkillsRequest {
    host: AgentHost,
    workspace: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AgentMcpClipboardField {
    RegistrationCommand,
    ConfigurationSnippet,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CopyAgentMcpConfigurationRequest {
    host: AgentHost,
    workspace: PathBuf,
    field: AgentMcpClipboardField,
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
pub(crate) async fn agent_assistance(
    request: AgentAssistanceRequest,
) -> Result<ActionReceipt<AgentAssistanceReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::agent_assistance(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn prepare_agent_handoff(
    request: PrepareAgentHandoffRequest,
) -> Result<ActionReceipt<AgentHandoffReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::prepare_agent_handoff(&AgentHandoffRequest {
            host: request.host,
            workspace: request.workspace,
            selected_job_id: request.selected_job_id,
        })
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn install_agent_skills(
    request: InstallAgentSkillsRequest,
) -> Result<ActionReceipt<AgentSkillsInstallReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::install_agent_skills(&AgentSkillsInstallRequest {
            host: request.host,
            workspace: request.workspace,
        })
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn copy_agent_handoff(
    request: CopyAgentHandoffRequest,
) -> Result<(), DesktopCommandError> {
    run_worker(move || copy_agent_handoff_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn prepare_agent_mcp_configuration(
    request: PrepareAgentMcpConfigurationRequest,
) -> Result<ActionReceipt<AgentMcpConfigurationReadModel>, DesktopCommandError> {
    run_worker(move || prepare_agent_mcp_configuration_impl(request)).await
}

#[cfg(target_os = "macos")]
fn prepare_agent_mcp_configuration_impl(
    request: PrepareAgentMcpConfigurationRequest,
) -> Result<ActionReceipt<AgentMcpConfigurationReadModel>, DesktopCommandError> {
    let executable = bundled_cli_path().ok_or_else(|| {
        DesktopCommandError::state(
            "The version-matched CanISend CLI is not available inside this App",
        )
    })?;
    Application::prepare_agent_mcp_configuration(&AgentMcpConfigurationRequest {
        host: request.host,
        workspace: request.workspace,
        executable,
    })
    .map_err(DesktopCommandError::application)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn copy_agent_mcp_configuration(
    request: CopyAgentMcpConfigurationRequest,
) -> Result<(), DesktopCommandError> {
    run_worker(move || {
        let prepared = prepare_agent_mcp_configuration_impl(PrepareAgentMcpConfigurationRequest {
            host: request.host,
            workspace: request.workspace,
        })?
        .data;
        let text = match request.field {
            AgentMcpClipboardField::RegistrationCommand => {
                prepared.registration_command.ok_or_else(|| {
                    DesktopCommandError::state("This host has no registration command")
                })?
            }
            AgentMcpClipboardField::ConfigurationSnippet => prepared.configuration_snippet,
        };
        copy_to_macos_clipboard(&text)
    })
    .await
}

#[cfg(target_os = "macos")]
fn copy_agent_handoff_impl(request: CopyAgentHandoffRequest) -> Result<(), DesktopCommandError> {
    let handoff = Application::prepare_agent_handoff(&AgentHandoffRequest {
        host: request.host,
        workspace: request.workspace,
        selected_job_id: request.selected_job_id,
    })
    .map_err(DesktopCommandError::application)?
    .data;
    let text = match request.field {
        AgentHandoffClipboardField::LaunchCommand => handoff.launch_command,
        AgentHandoffClipboardField::StartCommand => handoff.start_command,
        AgentHandoffClipboardField::BootstrapPrompt => handoff.bootstrap_prompt,
    };
    copy_to_macos_clipboard(&text)
}

#[cfg(target_os = "macos")]
fn copy_to_macos_clipboard(text: &str) -> Result<(), DesktopCommandError> {
    if text.len() > 32 * 1024 {
        return Err(DesktopCommandError::state(
            "Agent integration clipboard content exceeds the 32 KiB limit",
        ));
    }
    let mut child = Command::new("/usr/bin/pbcopy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            DesktopCommandError::state(format!("Cannot start the macOS clipboard service: {error}"))
        })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| DesktopCommandError::state("Clipboard input is unavailable"))?;
    if let Err(error) = stdin.write_all(text.as_bytes()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(DesktopCommandError::state(format!(
            "Cannot write the agent integration content to the clipboard: {error}"
        )));
    }
    drop(stdin);
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => {
                return Err(DesktopCommandError::state(
                    "The macOS clipboard service rejected the agent integration content",
                ));
            }
            Ok(None) if started.elapsed() < Duration::from_secs(2) => {
                thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DesktopCommandError::state(
                    "The macOS clipboard service timed out",
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DesktopCommandError::state(format!(
                    "Cannot monitor the macOS clipboard service: {error}"
                )));
            }
        }
    }
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

    #[test]
    fn assistance_request_requires_an_explicit_workspace_and_job() {
        let request: AgentAssistanceRequest = serde_json::from_value(serde_json::json!({
            "workspace": "/tmp/workspace",
            "job_id": "018f2498-7b2a-7f62-8a5c-5e1e7dfb4e11"
        }))
        .expect("assistance request");
        assert_eq!(request.workspace, PathBuf::from("/tmp/workspace"));
        assert!(
            serde_json::from_value::<AgentAssistanceRequest>(serde_json::json!({
                "workspace": "/tmp/workspace"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<AgentAssistanceRequest>(serde_json::json!({
                "workspace": "/tmp/workspace",
                "job_id": "018f2498-7b2a-7f62-8a5c-5e1e7dfb4e11",
                "include_private_bodies": true
            }))
            .is_err()
        );
    }

    #[test]
    fn clipboard_request_accepts_only_a_regenerated_handoff_field() {
        let request: CopyAgentHandoffRequest = serde_json::from_value(serde_json::json!({
            "host": "codex",
            "workspace": "/tmp/workspace",
            "selected_job_id": null,
            "field": "bootstrap-prompt"
        }))
        .expect("clipboard request");
        assert_eq!(request.field, AgentHandoffClipboardField::BootstrapPrompt);
        let start_request: CopyAgentHandoffRequest = serde_json::from_value(serde_json::json!({
            "host": "codex",
            "workspace": "/tmp/workspace",
            "selected_job_id": "job-id",
            "field": "start-command"
        }))
        .expect("start command request");
        assert_eq!(
            start_request.field,
            AgentHandoffClipboardField::StartCommand
        );
        assert!(
            serde_json::from_value::<CopyAgentHandoffRequest>(serde_json::json!({
                "host": "codex",
                "workspace": "/tmp/workspace",
                "selected_job_id": null,
                "field": "bootstrap-prompt",
                "text": "untrusted arbitrary clipboard body"
            }))
            .is_err()
        );
    }

    #[test]
    fn mcp_clipboard_request_accepts_only_a_regenerated_configuration_field() {
        let request: CopyAgentMcpConfigurationRequest = serde_json::from_value(serde_json::json!({
            "host": "claude",
            "workspace": "/tmp/workspace",
            "field": "configuration-snippet"
        }))
        .expect("MCP clipboard request");
        assert_eq!(request.field, AgentMcpClipboardField::ConfigurationSnippet);
        assert!(
            serde_json::from_value::<CopyAgentMcpConfigurationRequest>(serde_json::json!({
                "host": "claude",
                "workspace": "/tmp/workspace",
                "field": "configuration-snippet",
                "text": "untrusted arbitrary clipboard body"
            }))
            .is_err()
        );
    }
}
