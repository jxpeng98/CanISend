use std::path::{Path, PathBuf};

use canisend_app::{
    ACADEMIC_JOB_WORKFLOW_PACK_ID, ActionReceipt, Application, ApplicationDossierListReadModel,
    ApplicationDossierReadModel, ApplicationError, ApprovalBrokerError, BackupReadModel,
    ContentCatalogFilter, ContentCatalogReadModel, ContentSearchReadModel, ContentSearchRequest,
    DoctorSummary, JobDetailReadModel, JobListReadModel, NetworkFetchConsent, PrivateReadConsent,
    ProductSummary, SourceImportReadModel, WorkflowPackPresentationLocale,
    WorkflowPackPresentationReadModel, WorkspaceHealthReadModel, WorkspaceReadModel,
    WorkspaceRegistry, WorkspaceRepairReadModel, WorkspaceRestoreReadModel,
    WorkspaceV3MigrationPreview, WorkspaceV3MigrationReadModel, WorkspaceV3MigrationRequest,
    default_registry_path, validate_workspace_alias,
};
use canisend_contracts::{JobRecord, Sha256Digest};
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceCreateRequest {
    alias: String,
    path: PathBuf,
    #[serde(default)]
    pack_id: Option<String>,
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
pub(crate) struct WorkspaceMigrateRequest {
    workspace: PathBuf,
    expected_plan_sha256: String,
    backup_destination: PathBuf,
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
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    validate_workspace_alias(request.alias.trim()).map_err(DesktopCommandError::registry)?;
    let action = Application::initialize_workspace_for_pack(
        &request.path,
        request
            .pack_id
            .as_deref()
            .unwrap_or(ACADEMIC_JOB_WORKFLOW_PACK_ID),
    )
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
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || create_workspace_impl(&default_registry_path(), request)).await
}

#[tauri::command]
pub(crate) async fn connect_workspace(
    request: WorkspaceCreateRequest,
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || connect_workspace_impl(&default_registry_path(), request)).await
}

#[tauri::command]
pub(crate) async fn select_workspace(
    request: WorkspacePathRequest,
) -> Result<RegisteredAction<WorkspaceReadModel>, DesktopCommandError> {
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
) -> Result<ActionReceipt<WorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::workspace_status(&request.path).map_err(DesktopCommandError::application)
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
pub(crate) async fn preview_workspace_v3_migration(
    request: WorkspacePathRequest,
) -> Result<ActionReceipt<WorkspaceV3MigrationPreview>, DesktopCommandError> {
    run_worker(move || {
        Application::preview_workspace_v3_migration(&request.path)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn migrate_workspace_v3(
    request: WorkspaceMigrateRequest,
) -> Result<ActionReceipt<WorkspaceV3MigrationReadModel>, DesktopCommandError> {
    run_worker(move || {
        let expected_plan_sha256 = Sha256Digest::try_new(request.expected_plan_sha256)
            .map_err(|error| DesktopCommandError::state(error.to_string()))?;
        Application::migrate_workspace_v3(
            &request.workspace,
            WorkspaceV3MigrationRequest {
                expected_plan_sha256,
                backup_destination: request.backup_destination,
            },
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

fn list_jobs_impl(
    request: JobListRequest,
) -> Result<ActionReceipt<JobListReadModel>, DesktopCommandError> {
    Application::list_jobs(&request.workspace, request.include_archived)
        .map_err(DesktopCommandError::application)
}

fn create_job_impl(
    request: JobCreateRequest,
) -> Result<ActionReceipt<JobRecord>, DesktopCommandError> {
    Application::create_job(&request.workspace, &request.title, &request.institution)
        .map_err(DesktopCommandError::application)
}

fn show_job_impl(
    request: JobRequest,
) -> Result<ActionReceipt<JobDetailReadModel>, DesktopCommandError> {
    Application::job_detail(&request.workspace, &request.job_id)
        .map_err(DesktopCommandError::application)
}

fn archive_job_impl(request: JobRequest) -> Result<ActionReceipt<JobRecord>, DesktopCommandError> {
    Application::archive_job(&request.workspace, &request.job_id)
        .map_err(DesktopCommandError::application)
}

#[tauri::command]
pub(crate) async fn list_jobs(
    request: JobListRequest,
) -> Result<ActionReceipt<JobListReadModel>, DesktopCommandError> {
    run_worker(move || list_jobs_impl(request)).await
}

#[tauri::command]
pub(crate) async fn create_job(
    request: JobCreateRequest,
) -> Result<ActionReceipt<JobRecord>, DesktopCommandError> {
    run_worker(move || create_job_impl(request)).await
}

#[tauri::command]
pub(crate) async fn show_job(
    request: JobRequest,
) -> Result<ActionReceipt<JobDetailReadModel>, DesktopCommandError> {
    run_worker(move || show_job_impl(request)).await
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
pub(crate) async fn archive_job(
    request: JobRequest,
) -> Result<ActionReceipt<JobRecord>, DesktopCommandError> {
    run_worker(move || archive_job_impl(request)).await
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
    fn desktop_pack_selection_creates_v3_and_resolves_generic_labels() {
        let root = temporary_root("generic-workspace");
        let registry_path = temporary_root("generic-registry").join("workspaces.json");
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
                alias: "Generic applications".to_owned(),
                path: root.clone(),
                pack_id: Some(canisend_app::GENERIC_APPLICATION_WORKFLOW_PACK_ID.to_owned()),
            },
        )
        .expect("generic v3 Workspace");
        assert_eq!(
            created.action.data.status.workspace_format,
            "canisend.workspace/v3"
        );
        let before = Application::workspace_status(&root)
            .expect("generic status before compatibility command")
            .data
            .status;
        assert!(
            create_job_impl(JobCreateRequest {
                workspace: root.clone(),
                title: "Wrong Pack job".to_owned(),
                institution: "Must not persist".to_owned(),
            })
            .is_err()
        );
        assert_eq!(
            Application::workspace_status(&root)
                .expect("generic status after compatibility command")
                .data
                .status,
            before
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
                pack_id: None,
            },
        )
        .expect("create registered workspace");
        assert_eq!(created.registry.registry.entries.len(), 1);

        let job = create_job_impl(JobCreateRequest {
            workspace: root.clone(),
            title: "Lecturer".to_owned(),
            institution: "University".to_owned(),
        })
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
        let shown = show_job_impl(JobRequest {
            workspace: root.clone(),
            job_id: job.id.to_string(),
        })
        .expect("show job");
        assert_eq!(shown.data.job.id, job.id);
        let listed = list_jobs_impl(JobListRequest {
            workspace: root.clone(),
            include_archived: false,
        })
        .expect("list jobs");
        assert_eq!(listed.data.jobs.len(), 1);
        let dossier = application_dossier_impl(JobRequest {
            workspace: root.clone(),
            job_id: job.id.to_string(),
        })
        .expect("application dossier");
        assert_eq!(dossier.data.source_count, 1);
        let dossiers = list_application_dossiers_impl(JobListRequest {
            workspace: root.clone(),
            include_archived: false,
        })
        .expect("application dossiers");
        assert_eq!(dossiers.data.applications.len(), 1);

        let selected =
            select_workspace_impl(&registry_path, WorkspacePathRequest { path: root.clone() })
                .expect("select workspace");
        assert_eq!(selected.action.data.status.job_count, 1);
        archive_job_impl(JobRequest {
            workspace: root.clone(),
            job_id: job.id.to_string(),
        })
        .expect("archive job");
        assert!(
            list_jobs_impl(JobListRequest {
                workspace: root.clone(),
                include_archived: false,
            })
            .expect("list active jobs")
            .data
            .jobs
            .is_empty()
        );

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
