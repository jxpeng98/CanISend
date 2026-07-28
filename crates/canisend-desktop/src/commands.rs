use std::path::{Path, PathBuf};

use canisend_app::{
    ActionReceipt, Application, ApplicationError, BackupReadModel, DoctorSummary,
    JobDetailReadModel, JobListReadModel, NetworkFetchConsent, PrivateReadConsent, ProductSummary,
    SourceImportReadModel, WorkspaceHealthReadModel, WorkspaceReadModel, WorkspaceRegistry,
    WorkspaceRepairReadModel, WorkspaceRestoreReadModel, default_registry_path,
    validate_workspace_alias,
};
use canisend_contracts::JobRecord;
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

    #[cfg(target_os = "macos")]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceCreateRequest {
    alias: String,
    path: PathBuf,
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
pub(crate) struct JobCreateRequest {
    workspace: PathBuf,
    title: String,
    institution: String,
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

pub(crate) fn product_summary_impl() -> ProductSummary {
    Application::product_summary()
}

pub(crate) fn doctor_impl() -> Result<ActionReceipt<DoctorSummary>, DesktopCommandError> {
    Application::doctor().map_err(DesktopCommandError::application)
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
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    validate_workspace_alias(request.alias.trim()).map_err(DesktopCommandError::registry)?;
    let action = Application::initialize_workspace(&request.path)
        .map_err(DesktopCommandError::application)?;
    let mut registry =
        WorkspaceRegistry::load(registry_path).map_err(DesktopCommandError::registry)?;
    registry
        .register(request.alias.trim(), &action.data.path)
        .map_err(DesktopCommandError::registry)?;
    let registry = save_registry(registry_path, registry)?;
    Ok(RegisteredAction { action, registry })
}

fn connect_workspace_impl(
    registry_path: &Path,
    request: WorkspaceCreateRequest,
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    validate_workspace_alias(request.alias.trim()).map_err(DesktopCommandError::registry)?;
    let action =
        Application::workspace_status(&request.path).map_err(DesktopCommandError::application)?;
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
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    let action =
        Application::workspace_status(&request.path).map_err(DesktopCommandError::application)?;
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

#[cfg(target_os = "macos")]
pub(crate) async fn run_worker<T, F>(task: F) -> Result<T, DesktopCommandError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, DesktopCommandError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| DesktopCommandError::worker(error.to_string()))?
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) fn product_summary() -> ProductSummary {
    product_summary_impl()
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn run_doctor() -> Result<ActionReceipt<DoctorSummary>, DesktopCommandError> {
    run_worker(doctor_impl).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn list_workspaces() -> Result<RegistrySnapshot, DesktopCommandError> {
    run_worker(|| registry_snapshot_impl(&default_registry_path())).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn create_workspace(
    request: WorkspaceCreateRequest,
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || create_workspace_impl(&default_registry_path(), request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn connect_workspace(
    request: WorkspaceCreateRequest,
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || connect_workspace_impl(&default_registry_path(), request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn select_workspace(
    request: WorkspacePathRequest,
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || select_workspace_impl(&default_registry_path(), request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn remove_workspace(
    request: WorkspacePathRequest,
) -> Result<RegistrySnapshot, DesktopCommandError> {
    run_worker(move || remove_workspace_impl(&default_registry_path(), request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn workspace_status(
    request: WorkspacePathRequest,
) -> Result<ActionReceipt<WorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::workspace_status(&request.path).map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn check_workspace(
    request: WorkspacePathRequest,
) -> Result<ActionReceipt<WorkspaceHealthReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::check_workspace(&request.path).map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn restore_workspace(
    request: WorkspaceRestoreRequest,
) -> Result<RegisteredAction<WorkspaceRestoreReadModel>, DesktopCommandError> {
    run_worker(move || restore_workspace_impl(&default_registry_path(), request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn repair_workspace(
    request: WorkspacePathRequest,
) -> Result<ActionReceipt<WorkspaceRepairReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::repair_workspace(&request.path).map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn list_jobs(
    request: JobListRequest,
) -> Result<ActionReceipt<JobListReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::list_jobs(&request.workspace, request.include_archived)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn create_job(
    request: JobCreateRequest,
) -> Result<ActionReceipt<JobRecord>, DesktopCommandError> {
    run_worker(move || {
        Application::create_job(&request.workspace, &request.title, &request.institution)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn show_job(
    request: JobRequest,
) -> Result<ActionReceipt<JobDetailReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::job_detail(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn archive_job(
    request: JobRequest,
) -> Result<ActionReceipt<JobRecord>, DesktopCommandError> {
    run_worker(move || {
        Application::archive_job(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn import_local_job_source(
    request: LocalSourceImportRequest,
) -> Result<ActionReceipt<SourceImportReadModel>, DesktopCommandError> {
    run_worker(move || import_local_job_source_impl(request)).await
}

#[cfg(target_os = "macos")]
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
    fn shared_registry_and_job_commands_cover_the_local_ts2_slice() {
        let root = temporary_root("workspace");
        let registry_path = temporary_root("registry").join("workspaces.json");
        let source = temporary_root("advert").with_extension("txt");
        fs::write(&source, "Lecturer in Economics").expect("write source fixture");

        let created = create_workspace_impl(
            &registry_path,
            WorkspaceCreateRequest {
                alias: "Academic applications".to_owned(),
                path: root.clone(),
            },
        )
        .expect("create registered workspace");
        assert_eq!(created.registry.registry.entries.len(), 1);

        let job = Application::create_job(&root, "Lecturer", "University")
            .expect("create job")
            .data;
        let imported = import_local_job_source_impl(LocalSourceImportRequest {
            workspace: root.clone(),
            job_id: job.id.to_string(),
            source: source.clone(),
            confirmed_private_read: true,
        })
        .expect("import local source");
        assert_eq!(imported.data.job.source_ids.len(), 1);

        let selected =
            select_workspace_impl(&registry_path, WorkspacePathRequest { path: root.clone() })
                .expect("select workspace");
        assert_eq!(selected.action.data.status.job_count, 1);

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_dir_all(registry_path.parent().expect("registry parent"))
            .expect("remove registry");
        fs::remove_file(source).expect("remove source fixture");
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
}
