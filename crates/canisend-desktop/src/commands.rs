use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use canisend_app::{
    ACADEMIC_JOB_WORKFLOW_PACK_ID, ActionReceipt, AgentHost, AgentMcpConfigurationReadModel,
    AgentMcpConfigurationRequest, AgentSkillsInstallReadModel, AgentSkillsInstallRequest,
    Application, ApplicationDossierListReadModel, ApplicationDossierReadModel, ApplicationError,
    ApprovalBrokerError, BackupReadModel, ContentCatalogFilter, ContentCatalogReadModel,
    ContentSearchReadModel, ContentSearchRequest, DoctorSummary,
    GENERIC_APPLICATION_WORKFLOW_PACK_ID, NetworkFetchConsent, PrivateReadConsent, ProductSummary,
    SourceImportReadModel, WorkflowPackPresentationLocale, WorkflowPackPresentationReadModel,
    WorkspaceHealthReadModel, WorkspaceRegistry, WorkspaceRepairReadModel,
    WorkspaceRestoreReadModel, WorkspaceV4ReadModel, default_registry_path,
    desktop_cli_source_path, validate_workspace_alias,
};
use canisend_contracts::ApplicationPackBindingV3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesktopCommandError {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) retryable: bool,
}

impl DesktopCommandError {
    pub(crate) fn application(error: ApplicationError) -> Self {
        let failure = error.classify();
        let code = serde_json::to_value(failure.code)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| "application-failure".to_owned());
        Self {
            code,
            message: failure.message,
            retryable: failure.retryable,
        }
    }

    fn registry(message: String) -> Self {
        Self {
            code: "workspace-registry-failure".to_owned(),
            message,
            retryable: false,
        }
    }

    pub(crate) fn consent(message: &str) -> Self {
        Self {
            code: "consent-required".to_owned(),
            message: message.to_owned(),
            retryable: false,
        }
    }

    pub(crate) fn state(message: impl Into<String>) -> Self {
        Self {
            code: "desktop-state-failure".to_owned(),
            message: message.into(),
            retryable: false,
        }
    }

    pub(crate) fn approval(error: ApprovalBrokerError) -> Self {
        Self {
            code: crate::approval::approval_error_code(&error).to_owned(),
            message: error.to_string(),
            retryable: matches!(
                error,
                ApprovalBrokerError::Unavailable
                    | ApprovalBrokerError::TokenGeneration(_)
                    | ApprovalBrokerError::RestoreCollision
            ),
        }
    }

    pub(crate) fn system_open(message: impl Into<String>) -> Self {
        Self {
            code: "system-open-failure".to_owned(),
            message: message.into(),
            retryable: true,
        }
    }

    pub(crate) fn worker(message: String) -> Self {
        Self {
            code: "desktop-worker-failure".to_owned(),
            message,
            retryable: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistrySnapshot {
    registry_path: PathBuf,
    registry: WorkspaceRegistry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisteredAction<T> {
    action: ActionReceipt<T>,
    registry: RegistrySnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceBootstrapHostReadModel {
    host: AgentHost,
    skills: AgentSkillsInstallReadModel,
    mcp: AgentMcpConfigurationReadModel,
    configuration_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceBootstrapBoundaryReadModel {
    workspace_alias: String,
    application_count: usize,
    profile_initialized: bool,
    private_bodies_written: bool,
    workspace_modes_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceBootstrapReadModel {
    action: ActionReceipt<WorkspaceV4ReadModel>,
    registry: RegistrySnapshot,
    validated_packs: Vec<ApplicationPackBindingV3>,
    hosts: Vec<WorkspaceBootstrapHostReadModel>,
    boundary: WorkspaceBootstrapBoundaryReadModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceCreateRequest {
    alias: String,
    path: PathBuf,
    #[serde(default)]
    hosts: Vec<AgentHost>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspacePathRequest {
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceBackupRequest {
    workspace: PathBuf,
    destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceRestoreRequest {
    alias: String,
    backup: PathBuf,
    destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobListRequest {
    workspace: PathBuf,
    include_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobRequest {
    workspace: PathBuf,
    job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocalSourceImportRequest {
    workspace: PathBuf,
    job_id: String,
    source: PathBuf,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UrlSourceImportRequest {
    workspace: PathBuf,
    job_id: String,
    url: String,
    confirmed_network_fetch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentCatalogRequest {
    workspace: PathBuf,
    #[serde(default)]
    filter: ContentCatalogFilter,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentSearchCommandRequest {
    workspace: PathBuf,
    query: String,
    #[serde(default)]
    filter: ContentCatalogFilter,
    include_private_bodies: bool,
    confirmed_private_read: bool,
    limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkflowPackPresentationRequest {
    locale: WorkflowPackPresentationLocale,
    #[serde(default)]
    pack_id: Option<String>,
}

pub(crate) fn product_summary_impl() -> ProductSummary {
    Application::product_summary()
}

pub(crate) fn doctor_impl() -> Result<ActionReceipt<DoctorSummary>, DesktopCommandError> {
    Application::doctor().map_err(DesktopCommandError::application)
}

fn workflow_pack_presentation_impl(
    request: WorkflowPackPresentationRequest,
) -> Result<ActionReceipt<WorkflowPackPresentationReadModel>, DesktopCommandError> {
    Application::built_in_pack_presentation(
        request
            .pack_id
            .as_deref()
            .unwrap_or(ACADEMIC_JOB_WORKFLOW_PACK_ID),
        request.locale,
    )
    .map_err(DesktopCommandError::application)
}

fn registry_snapshot_impl(registry_path: &Path) -> Result<RegistrySnapshot, DesktopCommandError> {
    let registry = WorkspaceRegistry::load(registry_path).map_err(DesktopCommandError::registry)?;
    Ok(RegistrySnapshot {
        registry_path: registry_path.to_path_buf(),
        registry,
    })
}

fn save_registry(
    registry_path: &Path,
    registry: WorkspaceRegistry,
) -> Result<RegistrySnapshot, DesktopCommandError> {
    registry
        .save(registry_path)
        .map_err(DesktopCommandError::registry)?;
    Ok(RegistrySnapshot {
        registry_path: registry_path.to_path_buf(),
        registry,
    })
}

fn create_workspace_impl(
    registry_path: &Path,
    request: WorkspaceCreateRequest,
    desktop_executable: Option<PathBuf>,
) -> Result<WorkspaceBootstrapReadModel, DesktopCommandError> {
    validate_workspace_alias(request.alias.trim()).map_err(DesktopCommandError::registry)?;
    validate_bootstrap_hosts(&request.hosts)?;
    let validated_packs = [
        GENERIC_APPLICATION_WORKFLOW_PACK_ID,
        ACADEMIC_JOB_WORKFLOW_PACK_ID,
    ]
    .into_iter()
    .map(|pack_id| {
        Application::built_in_pack_presentation(pack_id, WorkflowPackPresentationLocale::English)
            .map(|receipt| receipt.data.pack)
            .map_err(DesktopCommandError::application)
    })
    .collect::<Result<Vec<_>, _>>()?;
    let root_existed = match fs::symlink_metadata(&request.path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(DesktopCommandError::state(
                    "Workspace setup requires a new or empty regular directory",
                ));
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(DesktopCommandError::state(format!(
                "Cannot inspect the Workspace setup directory: {error}"
            )));
        }
    };
    let action = Application::initialize_workspace_v4(&request.path)
        .map_err(DesktopCommandError::application)?;
    let mut rollback = WorkspaceBootstrapRollback::new(action.data.path.clone(), root_existed);
    let executable = if request.hosts.is_empty() {
        None
    } else {
        Some(desktop_executable.ok_or_else(|| {
            DesktopCommandError::state(
                "The version-matched CanISend desktop host is not available inside this App",
            )
        })?)
    };
    let mut hosts = Vec::with_capacity(request.hosts.len());
    for host in request.hosts.iter().copied() {
        let executable = executable.as_ref().ok_or_else(|| {
            DesktopCommandError::state(
                "The version-matched CanISend desktop host is not available inside this App",
            )
        })?;
        let mcp = Application::prepare_agent_mcp_configuration(&AgentMcpConfigurationRequest {
            host,
            workspace: action.data.path.clone(),
            executable: executable.clone(),
        })
        .map_err(DesktopCommandError::application)?
        .data;
        let skills = Application::install_agent_skills(&AgentSkillsInstallRequest {
            host,
            workspace: action.data.path.clone(),
        })
        .map_err(DesktopCommandError::application)?
        .data;
        let configuration_path = write_bootstrap_mcp_configuration(&action.data.path, host, &mcp)?;
        hosts.push(WorkspaceBootstrapHostReadModel {
            host,
            skills,
            mcp,
            configuration_path,
        });
    }
    let mut registry =
        WorkspaceRegistry::load(registry_path).map_err(DesktopCommandError::registry)?;
    registry
        .register(request.alias.trim(), &action.data.path)
        .map_err(DesktopCommandError::registry)?;
    let registry = save_registry(registry_path, registry)?;
    rollback.commit();
    Ok(WorkspaceBootstrapReadModel {
        action,
        registry,
        validated_packs,
        hosts,
        boundary: WorkspaceBootstrapBoundaryReadModel {
            workspace_alias: request.alias.trim().to_owned(),
            application_count: 0,
            profile_initialized: false,
            private_bodies_written: false,
            workspace_modes_enabled: false,
        },
    })
}

fn validate_bootstrap_hosts(hosts: &[AgentHost]) -> Result<(), DesktopCommandError> {
    let unique = hosts
        .iter()
        .map(|host| host.as_str())
        .collect::<BTreeSet<_>>();
    if unique.len() != hosts.len() {
        return Err(DesktopCommandError::state(
            "Agent hosts cannot be repeated during Workspace setup",
        ));
    }
    Ok(())
}

fn write_bootstrap_mcp_configuration(
    workspace: &Path,
    host: AgentHost,
    configuration: &AgentMcpConfigurationReadModel,
) -> Result<PathBuf, DesktopCommandError> {
    const MAX_CONFIGURATION_BYTES: usize = 64 * 1024;
    let expected_target = match host {
        AgentHost::Codex => ".codex/config.toml",
        AgentHost::Claude => ".mcp.json",
        AgentHost::Generic => "mcp.json",
    };
    if configuration.host != host || configuration.configuration_target != expected_target {
        return Err(DesktopCommandError::state(
            "Prepared MCP configuration does not match the selected Agent host",
        ));
    }
    let bytes = configuration.configuration_snippet.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_CONFIGURATION_BYTES {
        return Err(DesktopCommandError::state(
            "Prepared MCP configuration is empty or exceeds the 64 KiB setup limit",
        ));
    }
    let destination = workspace.join(expected_target);
    let parent = destination.parent().ok_or_else(|| {
        DesktopCommandError::state("Prepared MCP configuration has no parent directory")
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        DesktopCommandError::state(format!(
            "Cannot create the Agent host configuration directory: {error}"
        ))
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| {
            DesktopCommandError::state(format!(
                "Cannot create the Agent host configuration without overwriting a file: {error}"
            ))
        })?;
    file.write_all(bytes).map_err(|error| {
        DesktopCommandError::state(format!(
            "Cannot write the Agent host configuration: {error}"
        ))
    })?;
    file.sync_all().map_err(|error| {
        DesktopCommandError::state(format!("Cannot sync the Agent host configuration: {error}"))
    })?;
    Ok(destination)
}

struct WorkspaceBootstrapRollback {
    root: PathBuf,
    recreate_empty: bool,
    committed: bool,
}

impl WorkspaceBootstrapRollback {
    fn new(root: PathBuf, recreate_empty: bool) -> Self {
        Self {
            root,
            recreate_empty,
            committed: false,
        }
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for WorkspaceBootstrapRollback {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let _ = fs::remove_dir_all(&self.root);
        if self.recreate_empty {
            let _ = fs::create_dir(&self.root);
        }
    }
}

fn connect_workspace_impl(
    registry_path: &Path,
    request: WorkspaceCreateRequest,
) -> Result<RegisteredAction<WorkspaceV4ReadModel>, DesktopCommandError> {
    validate_workspace_alias(request.alias.trim()).map_err(DesktopCommandError::registry)?;
    let action = Application::workspace_status_v4(&request.path)
        .map_err(DesktopCommandError::application)?;
    let mut registry =
        WorkspaceRegistry::load(registry_path).map_err(DesktopCommandError::registry)?;
    registry
        .register(request.alias.trim(), &action.data.path)
        .map_err(DesktopCommandError::registry)?;
    let registry = save_registry(registry_path, registry)?;
    Ok(RegisteredAction { action, registry })
}

fn select_workspace_impl(
    registry_path: &Path,
    request: WorkspacePathRequest,
) -> Result<RegisteredAction<WorkspaceV4ReadModel>, DesktopCommandError> {
    let action = Application::workspace_status_v4(&request.path)
        .map_err(DesktopCommandError::application)?;
    let mut registry =
        WorkspaceRegistry::load(registry_path).map_err(DesktopCommandError::registry)?;
    registry
        .touch(&action.data.path)
        .map_err(DesktopCommandError::registry)?;
    let registry = save_registry(registry_path, registry)?;
    Ok(RegisteredAction { action, registry })
}

fn remove_workspace_impl(
    registry_path: &Path,
    request: WorkspacePathRequest,
) -> Result<RegistrySnapshot, DesktopCommandError> {
    let mut registry =
        WorkspaceRegistry::load(registry_path).map_err(DesktopCommandError::registry)?;
    registry.remove(&request.path);
    save_registry(registry_path, registry)
}

fn restore_workspace_impl(
    registry_path: &Path,
    request: WorkspaceRestoreRequest,
) -> Result<RegisteredAction<WorkspaceRestoreReadModel>, DesktopCommandError> {
    validate_workspace_alias(request.alias.trim()).map_err(DesktopCommandError::registry)?;
    let action = Application::restore_workspace(&request.backup, &request.destination)
        .map_err(DesktopCommandError::application)?;
    let mut registry =
        WorkspaceRegistry::load(registry_path).map_err(DesktopCommandError::registry)?;
    registry
        .register(request.alias.trim(), &action.data.destination)
        .map_err(DesktopCommandError::registry)?;
    let registry = save_registry(registry_path, registry)?;
    Ok(RegisteredAction { action, registry })
}

fn import_local_job_source_impl(
    request: LocalSourceImportRequest,
) -> Result<ActionReceipt<SourceImportReadModel>, DesktopCommandError> {
    if !request.confirmed_private_read {
        return Err(DesktopCommandError::consent(
            "Confirm access to the selected private local file before importing it.",
        ));
    }
    Application::import_local_job_source(
        &request.workspace,
        &request.job_id,
        &request.source,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)
}

fn import_url_job_source_impl(
    request: UrlSourceImportRequest,
) -> Result<ActionReceipt<SourceImportReadModel>, DesktopCommandError> {
    if !request.confirmed_network_fetch {
        return Err(DesktopCommandError::consent(
            "Confirm the network request before fetching this URL.",
        ));
    }
    Application::import_url_job_source(
        &request.workspace,
        &request.job_id,
        &request.url,
        NetworkFetchConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)
}

fn list_application_dossiers_impl(
    request: JobListRequest,
) -> Result<ActionReceipt<ApplicationDossierListReadModel>, DesktopCommandError> {
    Application::list_application_dossiers(&request.workspace, request.include_archived)
        .map_err(DesktopCommandError::application)
}

fn application_dossier_impl(
    request: JobRequest,
) -> Result<ActionReceipt<ApplicationDossierReadModel>, DesktopCommandError> {
    Application::application_dossier(&request.workspace, &request.job_id)
        .map_err(DesktopCommandError::application)
}

fn content_catalog_impl(
    request: ContentCatalogRequest,
) -> Result<ActionReceipt<ContentCatalogReadModel>, DesktopCommandError> {
    Application::content_catalog(&request.workspace, request.filter)
        .map_err(DesktopCommandError::application)
}

fn search_content_impl(
    request: ContentSearchCommandRequest,
) -> Result<ActionReceipt<ContentSearchReadModel>, DesktopCommandError> {
    if request.include_private_bodies && !request.confirmed_private_read {
        return Err(DesktopCommandError::consent(
            "Confirm private local content access before searching artifact bodies.",
        ));
    }
    let consent = (request.include_private_bodies && request.confirmed_private_read)
        .then(PrivateReadConsent::granted_by_user);
    Application::search_content(
        &request.workspace,
        ContentSearchRequest {
            query: request.query,
            filter: request.filter,
            include_private_bodies: request.include_private_bodies,
            limit: request.limit,
        },
        consent,
    )
    .map_err(DesktopCommandError::application)
}

pub(crate) async fn run_worker<T, F>(task: F) -> Result<T, DesktopCommandError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, DesktopCommandError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| DesktopCommandError::worker(error.to_string()))?
}

pub(crate) enum ApplicationWorkerError {
    Application(ApplicationError),
    Worker(String),
}

pub(crate) async fn run_application_worker<T, F>(task: F) -> Result<T, ApplicationWorkerError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ApplicationError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| ApplicationWorkerError::Worker(error.to_string()))?
        .map_err(ApplicationWorkerError::Application)
}

#[tauri::command]
pub(crate) fn product_summary() -> ProductSummary {
    product_summary_impl()
}

#[tauri::command]
pub(crate) async fn run_doctor() -> Result<ActionReceipt<DoctorSummary>, DesktopCommandError> {
    run_worker(doctor_impl).await
}

#[tauri::command]
pub(crate) async fn workflow_pack_presentation(
    request: WorkflowPackPresentationRequest,
) -> Result<ActionReceipt<WorkflowPackPresentationReadModel>, DesktopCommandError> {
    run_worker(move || workflow_pack_presentation_impl(request)).await
}

#[tauri::command]
pub(crate) async fn list_workspaces() -> Result<RegistrySnapshot, DesktopCommandError> {
    run_worker(|| registry_snapshot_impl(&default_registry_path())).await
}

#[tauri::command]
pub(crate) async fn create_workspace(
    request: WorkspaceCreateRequest,
) -> Result<WorkspaceBootstrapReadModel, DesktopCommandError> {
    let executable = desktop_cli_source_path();
    run_worker(move || create_workspace_impl(&default_registry_path(), request, executable)).await
}

#[tauri::command]
pub(crate) async fn connect_workspace(
    request: WorkspaceCreateRequest,
) -> Result<RegisteredAction<WorkspaceV4ReadModel>, DesktopCommandError> {
    run_worker(move || connect_workspace_impl(&default_registry_path(), request)).await
}

#[tauri::command]
pub(crate) async fn select_workspace(
    request: WorkspacePathRequest,
) -> Result<RegisteredAction<WorkspaceV4ReadModel>, DesktopCommandError> {
    run_worker(move || select_workspace_impl(&default_registry_path(), request)).await
}

#[tauri::command]
pub(crate) async fn remove_workspace(
    request: WorkspacePathRequest,
) -> Result<RegistrySnapshot, DesktopCommandError> {
    run_worker(move || remove_workspace_impl(&default_registry_path(), request)).await
}

#[tauri::command]
pub(crate) async fn workspace_status(
    request: WorkspacePathRequest,
) -> Result<ActionReceipt<WorkspaceV4ReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::workspace_status_v4(&request.path).map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn check_workspace(
    request: WorkspacePathRequest,
) -> Result<ActionReceipt<WorkspaceHealthReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::check_workspace(&request.path).map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn backup_workspace(
    request: WorkspaceBackupRequest,
) -> Result<ActionReceipt<BackupReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::backup_workspace(&request.workspace, &request.destination)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn restore_workspace(
    request: WorkspaceRestoreRequest,
) -> Result<RegisteredAction<WorkspaceRestoreReadModel>, DesktopCommandError> {
    run_worker(move || restore_workspace_impl(&default_registry_path(), request)).await
}

#[tauri::command]
pub(crate) async fn repair_workspace(
    request: WorkspacePathRequest,
) -> Result<ActionReceipt<WorkspaceRepairReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::repair_workspace(&request.path).map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn list_application_dossiers(
    request: JobListRequest,
) -> Result<ActionReceipt<ApplicationDossierListReadModel>, DesktopCommandError> {
    run_worker(move || list_application_dossiers_impl(request)).await
}

#[tauri::command]
pub(crate) async fn application_dossier(
    request: JobRequest,
) -> Result<ActionReceipt<ApplicationDossierReadModel>, DesktopCommandError> {
    run_worker(move || application_dossier_impl(request)).await
}

#[tauri::command]
pub(crate) async fn content_catalog(
    request: ContentCatalogRequest,
) -> Result<ActionReceipt<ContentCatalogReadModel>, DesktopCommandError> {
    run_worker(move || content_catalog_impl(request)).await
}

#[tauri::command]
pub(crate) async fn search_content(
    request: ContentSearchCommandRequest,
) -> Result<ActionReceipt<ContentSearchReadModel>, DesktopCommandError> {
    run_worker(move || search_content_impl(request)).await
}

#[tauri::command]
pub(crate) async fn import_local_job_source(
    request: LocalSourceImportRequest,
) -> Result<ActionReceipt<SourceImportReadModel>, DesktopCommandError> {
    run_worker(move || import_local_job_source_impl(request)).await
}

#[tauri::command]
pub(crate) async fn import_url_job_source(
    request: UrlSourceImportRequest,
) -> Result<ActionReceipt<SourceImportReadModel>, DesktopCommandError> {
    run_worker(move || import_url_job_source_impl(request)).await
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-desktop-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn product_summary_exposes_the_shared_rust_contract() {
        let summary = product_summary_impl();

        assert_eq!(summary.product, "canisend");
        assert_eq!(summary.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(summary.protocol, "canisend.agent/v2");
        assert_eq!(summary.workspace_format, "canisend.workspace/v2");
        assert_eq!(summary.resource_format, "canisend.resources/v2");
        assert!(!summary.target_os.is_empty());
        assert!(!summary.target_arch.is_empty());
    }

    #[test]
    fn workflow_pack_presentation_resolves_host_locale_through_verified_pack() {
        let presentation = workflow_pack_presentation_impl(WorkflowPackPresentationRequest {
            locale: WorkflowPackPresentationLocale::SimplifiedChinese,
            pack_id: None,
        })
        .expect("Pack presentation");

        assert_eq!(presentation.operation, "workflow-pack.presentation");
        assert_eq!(presentation.data.requested_locale.as_str(), "zh-CN");
        assert_eq!(presentation.data.selected_locale.as_str(), "zh-Hans");
        assert_eq!(presentation.data.deliverables.len(), 4);
        assert_eq!(presentation.data.deliverables[3].label.value, "学术简历");
    }

    #[test]
    fn desktop_creates_neutral_workspace_v4_and_resolves_both_pack_presentations() {
        let root = temporary_root("neutral-workspace");
        let registry_path = temporary_root("neutral-registry").join("workspaces.json");
        let presentation = workflow_pack_presentation_impl(WorkflowPackPresentationRequest {
            locale: WorkflowPackPresentationLocale::English,
            pack_id: Some(canisend_app::GENERIC_APPLICATION_WORKFLOW_PACK_ID.to_owned()),
        })
        .expect("generic Pack presentation");
        assert_eq!(
            presentation.data.pack.id.as_str(),
            canisend_app::GENERIC_APPLICATION_WORKFLOW_PACK_ID
        );
        assert_eq!(presentation.data.deliverables.len(), 2);

        let created = create_workspace_impl(
            &registry_path,
            WorkspaceCreateRequest {
                alias: "Mixed applications".to_owned(),
                path: root.clone(),
                hosts: Vec::new(),
            },
            None,
        )
        .expect("neutral Workspace v4");
        assert_eq!(
            created.action.data.status.workspace_format,
            canisend_contracts::WORKSPACE_V4_FORMAT
        );
        assert_eq!(created.action.data.status.application_count, 0);
        let academic = workflow_pack_presentation_impl(WorkflowPackPresentationRequest {
            locale: WorkflowPackPresentationLocale::English,
            pack_id: Some(canisend_app::ACADEMIC_JOB_WORKFLOW_PACK_ID.to_owned()),
        })
        .expect("academic Pack presentation");
        assert_eq!(
            academic.data.pack.id.as_str(),
            canisend_app::ACADEMIC_JOB_WORKFLOW_PACK_ID
        );

        fs::remove_dir_all(root).expect("remove Workspace");
        fs::remove_dir_all(registry_path.parent().expect("registry parent"))
            .expect("remove registry");
    }

    #[test]
    fn desktop_errors_are_stable_serializable_envelopes() {
        let error = DesktopCommandError {
            code: "fixture".to_owned(),
            message: "bounded message".to_owned(),
            retryable: false,
        };
        let json = serde_json::to_value(error).expect("desktop error must serialize");

        assert_eq!(json["code"], "fixture");
        assert_eq!(json["message"], "bounded message");
        assert_eq!(json["retryable"], false);
    }

    #[test]
    fn shared_registry_commands_create_and_reopen_neutral_workspace_v4() {
        let root = temporary_root("workspace");
        let registry_path = temporary_root("registry").join("workspaces.json");

        let created = create_workspace_impl(
            &registry_path,
            WorkspaceCreateRequest {
                alias: "Mixed applications".to_owned(),
                path: root.clone(),
                hosts: Vec::new(),
            },
            None,
        )
        .expect("create registered workspace");
        assert_eq!(created.registry.registry.entries.len(), 1);

        let selected =
            select_workspace_impl(&registry_path, WorkspacePathRequest { path: root.clone() })
                .expect("select workspace");
        assert_eq!(
            selected.action.data.status.workspace_format,
            canisend_contracts::WORKSPACE_V4_FORMAT
        );
        assert_eq!(selected.action.data.status.application_count, 0);
        let canonical_root = fs::canonicalize(&root).expect("canonical Workspace path");
        assert_eq!(
            selected.registry.registry.default_path.as_deref(),
            Some(canonical_root.as_path())
        );

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_dir_all(registry_path.parent().expect("registry parent"))
            .expect("remove registry");
    }

    #[test]
    fn desktop_bootstrap_installs_selected_v4_hosts_and_records_only_basic_boundaries() {
        let sandbox = temporary_root("bootstrap-hosts");
        let root = sandbox.join("workspace");
        let registry_path = sandbox.join("registry/workspaces.json");
        let executable = sandbox.join("canisend-gui");
        fs::create_dir_all(&sandbox).expect("create sandbox");
        fs::write(&executable, b"bounded desktop executable fixture")
            .expect("write executable fixture");

        let created = create_workspace_impl(
            &registry_path,
            WorkspaceCreateRequest {
                alias: "One library".to_owned(),
                path: root.clone(),
                hosts: vec![AgentHost::Codex, AgentHost::Claude],
            },
            Some(executable.clone()),
        )
        .expect("clean App bootstrap");

        assert_eq!(created.hosts.len(), 2);
        assert_eq!(created.boundary.workspace_alias, "One library");
        assert_eq!(created.boundary.application_count, 0);
        assert!(!created.boundary.profile_initialized);
        assert!(!created.boundary.private_bodies_written);
        assert!(!created.boundary.workspace_modes_enabled);
        assert_eq!(created.validated_packs.len(), 2);
        assert_eq!(
            created
                .validated_packs
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                GENERIC_APPLICATION_WORKFLOW_PACK_ID,
                ACADEMIC_JOB_WORKFLOW_PACK_ID,
            ])
        );
        assert_eq!(created.registry.registry.entries.len(), 1);
        for host in &created.hosts {
            assert!(host.skills.manifest_path.is_file());
            assert!(!host.skills.files.is_empty());
            assert_eq!(host.mcp.tools.len(), 5);
            assert_eq!(host.mcp.read_only_tools, host.mcp.tools);
            assert!(host.mcp.guarded_write_tools.is_empty());
            assert_eq!(
                fs::read_to_string(&host.configuration_path).expect("read MCP configuration"),
                host.mcp.configuration_snippet
            );
            assert_eq!(host.mcp.executable, fs::canonicalize(&executable).unwrap());
        }
        assert!(root.join(".codex/config.toml").is_file());
        assert!(root.join(".mcp.json").is_file());

        fs::remove_dir_all(sandbox).expect("remove bootstrap sandbox");
    }

    #[test]
    fn desktop_bootstrap_rejects_duplicate_hosts_before_creating_workspace() {
        let root = temporary_root("duplicate-hosts");
        let registry_path = temporary_root("duplicate-registry").join("workspaces.json");

        let error = create_workspace_impl(
            &registry_path,
            WorkspaceCreateRequest {
                alias: "Duplicate hosts".to_owned(),
                path: root.clone(),
                hosts: vec![AgentHost::Codex, AgentHost::Codex],
            },
            None,
        )
        .expect_err("duplicate hosts must fail");

        assert_eq!(error.code, "desktop-state-failure");
        assert!(!root.exists());
        assert!(!registry_path.exists());
    }

    #[test]
    fn desktop_bootstrap_rolls_back_a_new_workspace_when_registry_commit_fails() {
        let sandbox = temporary_root("bootstrap-rollback-new");
        let root = sandbox.join("workspace");
        let registry_path = sandbox.join("registry-as-directory");
        fs::create_dir_all(&registry_path).expect("create invalid registry target");

        let error = create_workspace_impl(
            &registry_path,
            WorkspaceCreateRequest {
                alias: "Rollback".to_owned(),
                path: root.clone(),
                hosts: Vec::new(),
            },
            None,
        )
        .expect_err("registry commit must fail");

        assert_eq!(error.code, "workspace-registry-failure");
        assert!(!root.exists());
        fs::remove_dir_all(sandbox).expect("remove rollback sandbox");
    }

    #[test]
    fn desktop_bootstrap_restores_an_existing_empty_directory_after_failure() {
        let sandbox = temporary_root("bootstrap-rollback-empty");
        let root = sandbox.join("workspace");
        let registry_path = sandbox.join("registry-as-directory");
        fs::create_dir_all(&root).expect("create selected empty directory");
        fs::create_dir_all(&registry_path).expect("create invalid registry target");

        create_workspace_impl(
            &registry_path,
            WorkspaceCreateRequest {
                alias: "Rollback".to_owned(),
                path: root.clone(),
                hosts: Vec::new(),
            },
            None,
        )
        .expect_err("registry commit must fail");

        assert!(root.is_dir());
        assert_eq!(
            fs::read_dir(&root)
                .expect("read restored directory")
                .count(),
            0
        );
        fs::remove_dir_all(sandbox).expect("remove rollback sandbox");
    }

    #[test]
    fn source_commands_require_explicit_consent_before_io() {
        let local_error = import_local_job_source_impl(LocalSourceImportRequest {
            workspace: PathBuf::from("/missing/workspace"),
            job_id: "missing".to_owned(),
            source: PathBuf::from("/missing/private.pdf"),
            confirmed_private_read: false,
        })
        .expect_err("private read must require consent");
        assert_eq!(local_error.code, "consent-required");

        let network_error = import_url_job_source_impl(UrlSourceImportRequest {
            workspace: PathBuf::from("/missing/workspace"),
            job_id: "missing".to_owned(),
            url: "https://example.invalid/job.pdf".to_owned(),
            confirmed_network_fetch: false,
        })
        .expect_err("network fetch must require consent");
        assert_eq!(network_error.code, "consent-required");
    }

    #[test]
    fn content_search_requires_private_consent_before_workspace_access() {
        let error = search_content_impl(ContentSearchCommandRequest {
            workspace: PathBuf::from("/missing/workspace"),
            query: "private evidence".to_owned(),
            filter: ContentCatalogFilter::default(),
            include_private_bodies: true,
            confirmed_private_read: false,
            limit: 25,
        })
        .expect_err("private body search must require consent");

        assert_eq!(error.code, "consent-required");
    }
}
