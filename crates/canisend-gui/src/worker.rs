use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, AgentCapabilitiesReadModel, AgentContextReadModel, AgentPackExportReadModel,
    AgentPackExportRequest, Application, ApplicationFailure, BackupReadModel, CliInstallStatus,
    DiscoveryAdapterCatalogReadModel, DiscoveryImportRequest, DiscoveryLeadListReadModel,
    DiscoveryPromotionReadModel, DiscoveryRefreshRequest, DiscoverySourceListReadModel,
    DiscoverySuggestionReadModel, DoctorSummary, DocumentWorkspaceReadModel, JobDetailReadModel,
    JobListReadModel, NetworkFetchConsent, PrivateReadConsent, ProfileSourceImportReadModel,
    ProfileSourceListReadModel, ProviderSendConsent, SourceImportReadModel,
    TaskCompletionPreviewReadModel, TaskInputExportRequest, TaskPrepareAgainReadModel,
    TaskPrepareRequest, TerminalInstallConsent, UpdateCheckReadModel, WorkflowBeginRequest,
    WorkflowCompleteRequest, WorkflowControlReadModel, WorkflowRerunPreview, WorkflowRerunRequest,
    WorkspaceHealthReadModel, WorkspaceReadModel, WorkspaceRepairReadModel,
    WorkspaceRestoreReadModel,
};
use canisend_contracts::{
    ApplicationPlanCandidate, ApplicationPlanRecord, CriteriaSetRecord, DiscoveryImportReport,
    DiscoveryLeadRecord, EvidenceCatalogRecord, EvidenceMatchSetRecord, JobRecord,
    PrivacyClassification, TaskCommitData, TaskCompletionRequest, TaskDescriptor,
    TaskInputExportData, TaskStateData, WorkflowStage, WorkflowStatusData,
};

#[derive(Debug)]
pub(crate) struct DiscoveryWorkspaceReadModel {
    pub(crate) sources: DiscoverySourceListReadModel,
    pub(crate) leads: DiscoveryLeadListReadModel,
}

#[derive(Debug)]
pub(crate) struct DiscoveryPromotionResult {
    pub(crate) receipt: ActionReceipt<DiscoveryPromotionReadModel>,
    pub(crate) jobs: Result<JobListReadModel, String>,
    pub(crate) discovery: Result<DiscoveryWorkspaceReadModel, String>,
}

#[derive(Debug)]
pub(crate) struct DiscoveryCommitResult {
    pub(crate) receipt: ActionReceipt<DiscoveryImportReport>,
    pub(crate) discovery: Result<DiscoveryWorkspaceReadModel, String>,
}

#[derive(Debug)]
pub(crate) enum WorkerRequest {
    LoadWorkspace {
        path: PathBuf,
    },
    CreateWorkspace {
        alias: String,
        path: PathBuf,
    },
    CheckWorkspace {
        path: PathBuf,
    },
    BackupWorkspace {
        root: PathBuf,
        destination: PathBuf,
    },
    RestoreWorkspace {
        alias: String,
        backup: PathBuf,
        destination: PathBuf,
    },
    RepairWorkspace {
        path: PathBuf,
    },
    LoadJobs {
        path: PathBuf,
        include_archived: bool,
    },
    LoadDiscoveryCatalog,
    LoadDiscoveryWorkspace {
        path: PathBuf,
        include_history: bool,
    },
    LoadDiscoveryLead {
        path: PathBuf,
        lead_id: String,
    },
    PreviewDiscoveryImport {
        request: DiscoveryImportRequest,
    },
    CommitDiscoveryImport {
        path: PathBuf,
        report: DiscoveryImportReport,
        include_history: bool,
    },
    PreviewDiscoveryRefresh {
        request: DiscoveryRefreshRequest,
    },
    CommitDiscoveryRefresh {
        path: PathBuf,
        report: DiscoveryImportReport,
        include_history: bool,
    },
    LoadDiscoverySuggestions {
        path: PathBuf,
        lead_id: String,
        limit: usize,
    },
    PromoteDiscoveryLead {
        path: PathBuf,
        lead_id: String,
        include_history: bool,
        include_archived_jobs: bool,
    },
    CreateJob {
        path: PathBuf,
        title: String,
        institution: String,
    },
    LoadJob {
        path: PathBuf,
        id: String,
    },
    ArchiveJob {
        path: PathBuf,
        id: String,
    },
    ImportLocalSource {
        path: PathBuf,
        id: String,
        source: PathBuf,
    },
    ImportUrlSource {
        path: PathBuf,
        id: String,
        url: String,
    },
    LoadProfileSources {
        path: PathBuf,
    },
    ImportProfileSource {
        path: PathBuf,
        source: PathBuf,
        sensitivity: PrivacyClassification,
    },
    LoadProfileEvidence {
        path: PathBuf,
        job_id: String,
    },
    ConfirmProfileEvidence {
        path: PathBuf,
        job_id: String,
        candidate: EvidenceCatalogRecord,
    },
    LoadCriteriaCandidate {
        path: PathBuf,
        job_id: String,
    },
    ConfirmCriteria {
        path: PathBuf,
        job_id: String,
        candidate: CriteriaSetRecord,
    },
    LoadCurrentMatches {
        path: PathBuf,
        job_id: String,
    },
    LoadPlanCandidate {
        path: PathBuf,
        job_id: String,
    },
    LoadCurrentPlan {
        path: PathBuf,
        job_id: String,
    },
    ConfirmPlan {
        path: PathBuf,
        job_id: String,
        candidate: ApplicationPlanCandidate,
    },
    LoadDocuments {
        path: PathBuf,
        job_id: String,
    },
    StartWorkflow {
        path: PathBuf,
        id: String,
    },
    LoadWorkflowControls {
        path: PathBuf,
        id: String,
    },
    BeginWorkflowStage {
        path: PathBuf,
        request: WorkflowBeginRequest,
    },
    CompleteWorkflowStage {
        path: PathBuf,
        request: WorkflowCompleteRequest,
    },
    PreviewWorkflowRerun {
        path: PathBuf,
        id: String,
        stage: WorkflowStage,
    },
    RerunWorkflowStage {
        path: PathBuf,
        request: WorkflowRerunRequest,
    },
    LoadLatestTask {
        path: PathBuf,
        job_id: String,
    },
    PrepareTask {
        path: PathBuf,
        request: TaskPrepareRequest,
    },
    ExportTaskInputs {
        path: PathBuf,
        request: TaskInputExportRequest,
        private_read_consent: bool,
        provider_send_consent: bool,
    },
    PreviewTaskCompletion {
        path: PathBuf,
        file: PathBuf,
    },
    CommitTaskCompletion {
        path: PathBuf,
        request: TaskCompletionRequest,
    },
    CancelTask {
        path: PathBuf,
        task_id: String,
    },
    PrepareTaskAgain {
        path: PathBuf,
        task_id: String,
    },
    LoadAgentCapabilities,
    LoadAgentContext {
        root: Option<PathBuf>,
        selected_job_id: Option<String>,
    },
    ExportAgentPack {
        request: AgentPackExportRequest,
    },
    LoadCliStatus {
        source: Option<PathBuf>,
        destination: PathBuf,
    },
    InstallCli {
        source: PathBuf,
        destination: PathBuf,
        replace_existing: bool,
    },
    UninstallCli {
        source: Option<PathBuf>,
        destination: PathBuf,
    },
    CheckForUpdates,
    Doctor,
}

#[derive(Debug)]
pub(crate) enum WorkerEvent {
    WorkspaceLoaded(Result<ActionReceipt<WorkspaceReadModel>, String>),
    WorkspaceCreated {
        alias: String,
        result: Result<ActionReceipt<WorkspaceReadModel>, String>,
    },
    WorkspaceChecked(Result<ActionReceipt<WorkspaceHealthReadModel>, String>),
    BackupCreated(Result<ActionReceipt<BackupReadModel>, String>),
    WorkspaceRestored {
        alias: String,
        result: Result<ActionReceipt<WorkspaceRestoreReadModel>, String>,
    },
    WorkspaceRepaired(Result<ActionReceipt<WorkspaceRepairReadModel>, String>),
    JobsLoaded(Result<ActionReceipt<JobListReadModel>, String>),
    DiscoveryCatalogLoaded(Result<ActionReceipt<DiscoveryAdapterCatalogReadModel>, String>),
    DiscoveryWorkspaceLoaded(Result<DiscoveryWorkspaceReadModel, String>),
    DiscoveryLeadLoaded(Result<ActionReceipt<DiscoveryLeadRecord>, String>),
    DiscoveryImportPreviewed(Result<ActionReceipt<DiscoveryImportReport>, String>),
    DiscoveryImportCommitted(Result<DiscoveryCommitResult, String>),
    DiscoveryRefreshPreviewed(Result<ActionReceipt<DiscoveryImportReport>, String>),
    DiscoveryRefreshCommitted(Result<DiscoveryCommitResult, String>),
    DiscoverySuggestionsLoaded(Result<ActionReceipt<DiscoverySuggestionReadModel>, String>),
    DiscoveryLeadPromoted(Result<DiscoveryPromotionResult, String>),
    JobCreated(Result<ActionReceipt<JobRecord>, String>),
    JobLoaded(Result<ActionReceipt<JobDetailReadModel>, String>),
    JobArchived(Result<ActionReceipt<JobRecord>, String>),
    SourceImported(Result<ActionReceipt<SourceImportReadModel>, String>),
    ProfileSourcesLoaded(Result<ActionReceipt<ProfileSourceListReadModel>, String>),
    ProfileSourceImported(Result<ActionReceipt<ProfileSourceImportReadModel>, String>),
    ProfileEvidenceLoaded {
        job_id: String,
        result: Result<ActionReceipt<EvidenceCatalogRecord>, String>,
    },
    ProfileEvidenceConfirmed {
        job_id: String,
        result: Result<ActionReceipt<EvidenceCatalogRecord>, String>,
    },
    CriteriaCandidateLoaded {
        job_id: String,
        result: Result<ActionReceipt<CriteriaSetRecord>, String>,
    },
    CriteriaConfirmed {
        job_id: String,
        result: Result<ActionReceipt<CriteriaSetRecord>, String>,
    },
    CurrentMatchesLoaded {
        job_id: String,
        result: Result<ActionReceipt<EvidenceMatchSetRecord>, String>,
    },
    PlanCandidateLoaded {
        job_id: String,
        result: Result<ActionReceipt<ApplicationPlanCandidate>, String>,
    },
    CurrentPlanLoaded {
        job_id: String,
        result: Result<ActionReceipt<ApplicationPlanRecord>, String>,
    },
    PlanConfirmed {
        job_id: String,
        result: Result<ActionReceipt<ApplicationPlanRecord>, String>,
    },
    DocumentsLoaded {
        job_id: String,
        result: Result<DocumentWorkspaceReadModel, String>,
    },
    WorkflowLoaded(Result<ActionReceipt<WorkflowStatusData>, String>),
    WorkflowControlsLoaded(Result<ActionReceipt<WorkflowControlReadModel>, String>),
    WorkflowMutated(Result<ActionReceipt<WorkflowControlReadModel>, String>),
    WorkflowRerunPreviewed(Result<ActionReceipt<WorkflowRerunPreview>, String>),
    LatestTaskLoaded {
        job_id: String,
        result: Result<ActionReceipt<Option<TaskStateData>>, ApplicationFailure>,
    },
    TaskPrepared(Result<ActionReceipt<TaskDescriptor>, ApplicationFailure>),
    TaskInputsExported(Result<ActionReceipt<TaskInputExportData>, ApplicationFailure>),
    TaskCompletionPreviewed(
        Result<ActionReceipt<TaskCompletionPreviewReadModel>, ApplicationFailure>,
    ),
    TaskCompleted(Result<ActionReceipt<TaskCommitData>, ApplicationFailure>),
    TaskCancelled(Result<ActionReceipt<TaskStateData>, ApplicationFailure>),
    TaskPreparedAgain(Result<ActionReceipt<TaskPrepareAgainReadModel>, ApplicationFailure>),
    AgentCapabilitiesLoaded(Result<ActionReceipt<AgentCapabilitiesReadModel>, ApplicationFailure>),
    AgentContextLoaded {
        selected_job_id: Option<String>,
        result: Result<ActionReceipt<AgentContextReadModel>, ApplicationFailure>,
    },
    AgentPackExported {
        request: AgentPackExportRequest,
        result: Result<ActionReceipt<AgentPackExportReadModel>, ApplicationFailure>,
    },
    CliStatusLoaded(Result<ActionReceipt<CliInstallStatus>, String>),
    CliInstalled(Result<ActionReceipt<CliInstallStatus>, String>),
    CliUninstalled(Result<ActionReceipt<CliInstallStatus>, String>),
    UpdateCheckFinished(Result<ActionReceipt<UpdateCheckReadModel>, String>),
    DoctorFinished(Result<ActionReceipt<DoctorSummary>, String>),
}

pub(crate) fn execute(request: WorkerRequest) -> WorkerEvent {
    match request {
        WorkerRequest::LoadWorkspace { path } => WorkerEvent::WorkspaceLoaded(
            Application::workspace_status(&path).map_err(|error| error.to_string()),
        ),
        WorkerRequest::CreateWorkspace { alias, path } => WorkerEvent::WorkspaceCreated {
            alias,
            result: Application::initialize_workspace(&path).map_err(|error| error.to_string()),
        },
        WorkerRequest::CheckWorkspace { path } => WorkerEvent::WorkspaceChecked(
            Application::check_workspace(&path).map_err(|error| error.to_string()),
        ),
        WorkerRequest::BackupWorkspace { root, destination } => WorkerEvent::BackupCreated(
            Application::backup_workspace(&root, &destination).map_err(|error| error.to_string()),
        ),
        WorkerRequest::RestoreWorkspace {
            alias,
            backup,
            destination,
        } => WorkerEvent::WorkspaceRestored {
            alias,
            result: Application::restore_workspace(&backup, &destination)
                .map_err(|error| error.to_string()),
        },
        WorkerRequest::RepairWorkspace { path } => WorkerEvent::WorkspaceRepaired(
            Application::repair_workspace(&path).map_err(|error| error.to_string()),
        ),
        WorkerRequest::LoadJobs {
            path,
            include_archived,
        } => WorkerEvent::JobsLoaded(
            Application::list_jobs(&path, include_archived).map_err(|error| error.to_string()),
        ),
        WorkerRequest::LoadDiscoveryCatalog => {
            WorkerEvent::DiscoveryCatalogLoaded(Ok(Application::discovery_adapters()))
        }
        WorkerRequest::LoadDiscoveryWorkspace {
            path,
            include_history,
        } => {
            WorkerEvent::DiscoveryWorkspaceLoaded(load_discovery_workspace(&path, include_history))
        }
        WorkerRequest::LoadDiscoveryLead { path, lead_id } => WorkerEvent::DiscoveryLeadLoaded(
            Application::discovery_lead(&path, &lead_id).map_err(|error| error.to_string()),
        ),
        WorkerRequest::PreviewDiscoveryImport { request } => WorkerEvent::DiscoveryImportPreviewed(
            Application::preview_discovery_import(&request, PrivateReadConsent::granted_by_user())
                .map_err(|error| error.to_string()),
        ),
        WorkerRequest::CommitDiscoveryImport {
            path,
            report,
            include_history,
        } => WorkerEvent::DiscoveryImportCommitted(
            Application::commit_discovery_import(&path, report)
                .map_err(|error| error.to_string())
                .map(|receipt| {
                    let discovery = load_discovery_workspace(&path, include_history);
                    DiscoveryCommitResult { receipt, discovery }
                }),
        ),
        WorkerRequest::PreviewDiscoveryRefresh { request } => {
            WorkerEvent::DiscoveryRefreshPreviewed(
                Application::preview_discovery_refresh(
                    &request,
                    NetworkFetchConsent::granted_by_user(),
                )
                .map_err(|error| error.to_string()),
            )
        }
        WorkerRequest::CommitDiscoveryRefresh {
            path,
            report,
            include_history,
        } => WorkerEvent::DiscoveryRefreshCommitted(
            Application::commit_discovery_refresh(&path, report)
                .map_err(|error| error.to_string())
                .map(|receipt| {
                    let discovery = load_discovery_workspace(&path, include_history);
                    DiscoveryCommitResult { receipt, discovery }
                }),
        ),
        WorkerRequest::LoadDiscoverySuggestions {
            path,
            lead_id,
            limit,
        } => WorkerEvent::DiscoverySuggestionsLoaded(
            Application::discovery_suggestions(&path, &lead_id, limit)
                .map_err(|error| error.to_string()),
        ),
        WorkerRequest::PromoteDiscoveryLead {
            path,
            lead_id,
            include_history,
            include_archived_jobs,
        } => {
            let result = Application::promote_discovery_lead(&path, &lead_id)
                .map_err(|error| error.to_string())
                .map(|receipt| {
                    let jobs = Application::list_jobs(&path, include_archived_jobs)
                        .map(|result| result.data)
                        .map_err(|error| error.to_string());
                    let discovery = load_discovery_workspace(&path, include_history);
                    DiscoveryPromotionResult {
                        receipt,
                        jobs,
                        discovery,
                    }
                });
            WorkerEvent::DiscoveryLeadPromoted(result)
        }
        WorkerRequest::CreateJob {
            path,
            title,
            institution,
        } => WorkerEvent::JobCreated(
            Application::create_job(&path, &title, &institution).map_err(|error| error.to_string()),
        ),
        WorkerRequest::LoadJob { path, id } => WorkerEvent::JobLoaded(
            Application::job_detail(&path, &id).map_err(|error| error.to_string()),
        ),
        WorkerRequest::ArchiveJob { path, id } => WorkerEvent::JobArchived(
            Application::archive_job(&path, &id).map_err(|error| error.to_string()),
        ),
        WorkerRequest::ImportLocalSource { path, id, source } => WorkerEvent::SourceImported(
            Application::import_local_job_source(
                &path,
                &id,
                &source,
                PrivateReadConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string()),
        ),
        WorkerRequest::ImportUrlSource { path, id, url } => WorkerEvent::SourceImported(
            Application::import_url_job_source(
                &path,
                &id,
                &url,
                NetworkFetchConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string()),
        ),
        WorkerRequest::LoadProfileSources { path } => WorkerEvent::ProfileSourcesLoaded(
            Application::list_profile_sources(&path).map_err(|error| error.to_string()),
        ),
        WorkerRequest::ImportProfileSource {
            path,
            source,
            sensitivity,
        } => WorkerEvent::ProfileSourceImported(
            Application::import_profile_source(
                &path,
                &source,
                sensitivity,
                PrivateReadConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string()),
        ),
        WorkerRequest::LoadProfileEvidence { path, job_id } => {
            let result = Application::profile_evidence_template(
                &path,
                &job_id,
                PrivateReadConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string());
            WorkerEvent::ProfileEvidenceLoaded { job_id, result }
        }
        WorkerRequest::ConfirmProfileEvidence {
            path,
            job_id,
            candidate,
        } => {
            let result = serde_json::to_value(candidate)
                .map_err(|error| error.to_string())
                .and_then(|candidate| {
                    Application::confirm_profile_evidence(
                        &path,
                        &job_id,
                        &candidate,
                        PrivateReadConsent::granted_by_user(),
                    )
                    .map_err(|error| error.to_string())
                });
            WorkerEvent::ProfileEvidenceConfirmed { job_id, result }
        }
        WorkerRequest::LoadCriteriaCandidate { path, job_id } => {
            let result = Application::job_criteria_template(
                &path,
                &job_id,
                PrivateReadConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string());
            WorkerEvent::CriteriaCandidateLoaded { job_id, result }
        }
        WorkerRequest::ConfirmCriteria {
            path,
            job_id,
            candidate,
        } => {
            let result = serde_json::to_value(candidate)
                .map_err(|error| error.to_string())
                .and_then(|candidate| {
                    Application::confirm_job_criteria(
                        &path,
                        &job_id,
                        &candidate,
                        PrivateReadConsent::granted_by_user(),
                    )
                    .map_err(|error| error.to_string())
                });
            WorkerEvent::CriteriaConfirmed { job_id, result }
        }
        WorkerRequest::LoadCurrentMatches { path, job_id } => {
            let result = Application::current_evidence_matches(
                &path,
                &job_id,
                PrivateReadConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string());
            WorkerEvent::CurrentMatchesLoaded { job_id, result }
        }
        WorkerRequest::LoadPlanCandidate { path, job_id } => {
            let result = Application::application_plan_template(
                &path,
                &job_id,
                PrivateReadConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string());
            WorkerEvent::PlanCandidateLoaded { job_id, result }
        }
        WorkerRequest::LoadCurrentPlan { path, job_id } => {
            let result = Application::current_application_plan(
                &path,
                &job_id,
                PrivateReadConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string());
            WorkerEvent::CurrentPlanLoaded { job_id, result }
        }
        WorkerRequest::ConfirmPlan {
            path,
            job_id,
            candidate,
        } => {
            let result = serde_json::to_value(candidate)
                .map_err(|error| error.to_string())
                .and_then(|candidate| {
                    Application::confirm_application_plan(
                        &path,
                        &job_id,
                        &candidate,
                        PrivateReadConsent::granted_by_user(),
                    )
                    .map_err(|error| error.to_string())
                });
            WorkerEvent::PlanConfirmed { job_id, result }
        }
        WorkerRequest::LoadDocuments { path, job_id } => WorkerEvent::DocumentsLoaded {
            result: load_document_workspace(&path, &job_id),
            job_id,
        },
        WorkerRequest::StartWorkflow { path, id } => WorkerEvent::WorkflowLoaded(
            Application::start_workflow(&path, &id).map_err(|error| error.to_string()),
        ),
        WorkerRequest::LoadWorkflowControls { path, id } => WorkerEvent::WorkflowControlsLoaded(
            Application::workflow_controls(&path, &id).map_err(|error| error.to_string()),
        ),
        WorkerRequest::BeginWorkflowStage { path, request } => WorkerEvent::WorkflowMutated(
            Application::begin_workflow_stage(&path, request).map_err(|error| error.to_string()),
        ),
        WorkerRequest::CompleteWorkflowStage { path, request } => WorkerEvent::WorkflowMutated(
            Application::complete_workflow_stage(&path, request).map_err(|error| error.to_string()),
        ),
        WorkerRequest::PreviewWorkflowRerun { path, id, stage } => {
            WorkerEvent::WorkflowRerunPreviewed(
                Application::preview_workflow_rerun(&path, &id, stage)
                    .map_err(|error| error.to_string()),
            )
        }
        WorkerRequest::RerunWorkflowStage { path, request } => WorkerEvent::WorkflowMutated(
            Application::rerun_workflow_stage(&path, request).map_err(|error| error.to_string()),
        ),
        WorkerRequest::LoadLatestTask { path, job_id } => WorkerEvent::LatestTaskLoaded {
            result: Application::latest_task_for_job(&path, &job_id)
                .map_err(|error| error.classify()),
            job_id,
        },
        WorkerRequest::PrepareTask { path, request } => WorkerEvent::TaskPrepared(
            Application::prepare_task(&path, request).map_err(|error| error.classify()),
        ),
        WorkerRequest::ExportTaskInputs {
            path,
            request,
            private_read_consent,
            provider_send_consent,
        } => WorkerEvent::TaskInputsExported(
            Application::export_task_inputs(
                &path,
                request,
                private_read_consent.then(PrivateReadConsent::granted_by_user),
                provider_send_consent.then(ProviderSendConsent::granted_by_user),
            )
            .map_err(|error| error.classify()),
        ),
        WorkerRequest::PreviewTaskCompletion { path, file } => {
            WorkerEvent::TaskCompletionPreviewed(
                Application::preview_task_completion_file(
                    &path,
                    &file,
                    PrivateReadConsent::granted_by_user(),
                )
                .map_err(|error| error.classify()),
            )
        }
        WorkerRequest::CommitTaskCompletion { path, request } => WorkerEvent::TaskCompleted(
            Application::commit_task_completion(&path, request).map_err(|error| error.classify()),
        ),
        WorkerRequest::CancelTask { path, task_id } => WorkerEvent::TaskCancelled(
            Application::cancel_task(&path, &task_id).map_err(|error| error.classify()),
        ),
        WorkerRequest::PrepareTaskAgain { path, task_id } => WorkerEvent::TaskPreparedAgain(
            Application::prepare_task_again(&path, &task_id).map_err(|error| error.classify()),
        ),
        WorkerRequest::LoadAgentCapabilities => WorkerEvent::AgentCapabilitiesLoaded(
            Application::agent_capabilities().map_err(|error| error.classify()),
        ),
        WorkerRequest::LoadAgentContext {
            root,
            selected_job_id,
        } => WorkerEvent::AgentContextLoaded {
            result: Application::agent_context(root.as_deref(), selected_job_id.as_deref())
                .map_err(|error| error.classify()),
            selected_job_id,
        },
        WorkerRequest::ExportAgentPack { request } => WorkerEvent::AgentPackExported {
            result: Application::export_agent_assets(&request).map_err(|error| error.classify()),
            request,
        },
        WorkerRequest::LoadCliStatus {
            source,
            destination,
        } => WorkerEvent::CliStatusLoaded(
            Application::cli_install_status(source.as_deref(), &destination)
                .map_err(|error| error.to_string()),
        ),
        WorkerRequest::InstallCli {
            source,
            destination,
            replace_existing,
        } => WorkerEvent::CliInstalled(
            Application::install_cli(
                &source,
                &destination,
                replace_existing,
                TerminalInstallConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string()),
        ),
        WorkerRequest::UninstallCli {
            source,
            destination,
        } => WorkerEvent::CliUninstalled(
            Application::uninstall_cli(
                source.as_deref(),
                &destination,
                TerminalInstallConsent::granted_by_user(),
            )
            .map_err(|error| error.to_string()),
        ),
        WorkerRequest::CheckForUpdates => WorkerEvent::UpdateCheckFinished(
            Application::check_for_updates(NetworkFetchConsent::granted_by_user())
                .map_err(|error| error.to_string()),
        ),
        WorkerRequest::Doctor => {
            WorkerEvent::DoctorFinished(Application::doctor().map_err(|error| error.to_string()))
        }
    }
}

fn load_discovery_workspace(
    path: &std::path::Path,
    include_history: bool,
) -> Result<DiscoveryWorkspaceReadModel, String> {
    let sources = Application::list_discovery_sources(path)
        .map_err(|error| error.to_string())?
        .data;
    let leads = Application::list_discovery_leads(path, include_history)
        .map_err(|error| error.to_string())?
        .data;
    Ok(DiscoveryWorkspaceReadModel { sources, leads })
}

fn load_document_workspace(
    path: &std::path::Path,
    job_id: &str,
) -> Result<DocumentWorkspaceReadModel, String> {
    Ok(
        Application::document_workspace(path, job_id, PrivateReadConsent::granted_by_user())
            .map_err(|error| error.to_string())?
            .data,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use canisend_app::{
        AgentHost, AgentPackExportRequest, Application, DiscoveryImportRequest,
        DiscoveryNetworkAdapter, DiscoveryRefreshRequest, PrivateReadConsent, TaskExecutionMode,
        TaskInputExportRequest, TaskOperation, TaskPrepareRequest, WorkflowBeginRequest,
        WorkflowRerunRequest,
    };
    use canisend_contracts::{
        ApplicationDecision, ArtifactKind, DiscoveryLeadStatus, ErrorCode, ExecutionMode,
        ExpectedInputRevision, TaskCompletionRequest, TaskStatus, WorkflowStage,
    };
    use canisend_store::{CriteriaService, EvidenceService, TaskService, Workspace};
    use serde_json::json;

    use super::{WorkerEvent, WorkerRequest, execute};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-gui-worker-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    use std::path::PathBuf;

    use canisend_contracts::PrivacyClassification;

    #[test]
    fn each_request_produces_one_typed_terminal_event() {
        assert!(matches!(
            execute(WorkerRequest::Doctor),
            WorkerEvent::DoctorFinished(Ok(_))
        ));

        let root = temporary_root("workspace");
        assert!(matches!(
            execute(WorkerRequest::CreateWorkspace {
                alias: "Worker fixture".to_owned(),
                path: root.clone(),
            }),
            WorkerEvent::WorkspaceCreated { result: Ok(_), .. }
        ));
        assert!(matches!(
            execute(WorkerRequest::LoadWorkspace { path: root.clone() }),
            WorkerEvent::WorkspaceLoaded(Ok(_))
        ));
        let job_id = match execute(WorkerRequest::CreateJob {
            path: root.clone(),
            title: "Lecturer".to_owned(),
            institution: "University".to_owned(),
        }) {
            WorkerEvent::JobCreated(Ok(receipt)) => receipt.data.id,
            event => panic!("unexpected job event: {event:?}"),
        };
        match execute(WorkerRequest::LoadDocuments {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::DocumentsLoaded {
                result: Ok(workspace),
                ..
            } => {
                assert!(workspace.documents.is_empty());
                assert!(workspace.accepted_set.is_none());
                assert!(workspace.acceptance_blocker.is_some());
            }
            event => panic!("unexpected documents event: {event:?}"),
        }
        let source = temporary_root("source").with_extension("txt");
        std::fs::write(&source, "bounded job source").expect("write source");
        assert!(matches!(
            execute(WorkerRequest::ImportLocalSource {
                path: root.clone(),
                id: job_id.to_string(),
                source: source.clone(),
            }),
            WorkerEvent::SourceImported(Ok(_))
        ));
        let profile_source = temporary_root("profile-source").with_extension("md");
        std::fs::write(&profile_source, "# Private profile\n\nBounded evidence.")
            .expect("write profile source");
        assert!(matches!(
            execute(WorkerRequest::ImportProfileSource {
                path: root.clone(),
                source: profile_source.clone(),
                sensitivity: PrivacyClassification::PrivateLocal,
            }),
            WorkerEvent::ProfileSourceImported(Ok(_))
        ));
        match execute(WorkerRequest::LoadProfileSources { path: root.clone() }) {
            WorkerEvent::ProfileSourcesLoaded(Ok(receipt)) => {
                assert_eq!(receipt.data.profile_revision, 1);
                assert_eq!(receipt.data.sources.len(), 1);
                assert!(
                    !serde_json::to_string(&receipt)
                        .expect("serialize profile source list")
                        .contains("Bounded evidence")
                );
            }
            event => panic!("unexpected profile event: {event:?}"),
        }
        assert!(matches!(
            execute(WorkerRequest::LoadProfileEvidence {
                path: root.clone(),
                job_id: job_id.to_string(),
            }),
            WorkerEvent::ProfileEvidenceLoaded { result: Err(_), .. }
        ));
        assert!(matches!(
            execute(WorkerRequest::LoadCriteriaCandidate {
                path: root.clone(),
                job_id: job_id.to_string(),
            }),
            WorkerEvent::CriteriaCandidateLoaded { result: Err(_), .. }
        ));
        assert!(matches!(
            execute(WorkerRequest::LoadCurrentMatches {
                path: root.clone(),
                job_id: job_id.to_string(),
            }),
            WorkerEvent::CurrentMatchesLoaded { result: Err(_), .. }
        ));
        assert!(matches!(
            execute(WorkerRequest::LoadPlanCandidate {
                path: root.clone(),
                job_id: job_id.to_string(),
            }),
            WorkerEvent::PlanCandidateLoaded { result: Err(_), .. }
        ));
        assert!(matches!(
            execute(WorkerRequest::LoadCurrentPlan {
                path: root.clone(),
                job_id: job_id.to_string(),
            }),
            WorkerEvent::CurrentPlanLoaded { result: Err(_), .. }
        ));
        assert!(matches!(
            execute(WorkerRequest::StartWorkflow {
                path: root.clone(),
                id: job_id.to_string(),
            }),
            WorkerEvent::WorkflowLoaded(Ok(_))
        ));
        let controls = execute(WorkerRequest::LoadWorkflowControls {
            path: root.clone(),
            id: job_id.to_string(),
        });
        match controls {
            WorkerEvent::WorkflowControlsLoaded(Ok(receipt)) => {
                assert_eq!(
                    receipt.data.stage_descriptors[1].execution_modes,
                    vec![ExecutionMode::HostAgent, ExecutionMode::ConfiguredProvider]
                );
            }
            event => panic!("unexpected controls event: {event:?}"),
        }
        assert!(matches!(
            execute(WorkerRequest::BeginWorkflowStage {
                path: root.clone(),
                request: WorkflowBeginRequest {
                    job_id: job_id.clone(),
                    stage: WorkflowStage::Parse,
                    mode: ExecutionMode::HostAgent,
                },
            }),
            WorkerEvent::WorkflowMutated(Ok(_))
        ));
        assert!(matches!(
            execute(WorkerRequest::PreviewWorkflowRerun {
                path: root.clone(),
                id: job_id.to_string(),
                stage: WorkflowStage::Parse,
            }),
            WorkerEvent::WorkflowRerunPreviewed(Ok(_))
        ));
        assert!(matches!(
            execute(WorkerRequest::RerunWorkflowStage {
                path: root.clone(),
                request: WorkflowRerunRequest {
                    job_id,
                    stage: WorkflowStage::Parse,
                },
            }),
            WorkerEvent::WorkflowMutated(Ok(_))
        ));

        let backup = temporary_root("backup");
        assert!(matches!(
            execute(WorkerRequest::BackupWorkspace {
                root: root.clone(),
                destination: backup.clone(),
            }),
            WorkerEvent::BackupCreated(Ok(_))
        ));
        let restored = temporary_root("restored");
        assert!(matches!(
            execute(WorkerRequest::RestoreWorkspace {
                alias: "Recovered fixture".to_owned(),
                backup: backup.clone(),
                destination: restored.clone(),
            }),
            WorkerEvent::WorkspaceRestored {
                alias,
                result: Ok(_),
            } if alias == "Recovered fixture"
        ));
        assert!(matches!(
            execute(WorkerRequest::RepairWorkspace {
                path: restored.clone(),
            }),
            WorkerEvent::WorkspaceRepaired(Ok(_))
        ));

        std::fs::remove_dir_all(root).expect("remove worker fixture");
        std::fs::remove_file(source).expect("remove source fixture");
        std::fs::remove_file(profile_source).expect("remove profile source fixture");
        std::fs::remove_dir_all(backup).expect("remove backup fixture");
        std::fs::remove_dir_all(restored).expect("remove restored fixture");
    }

    #[test]
    fn agent_worker_events_preserve_body_free_context_and_export_failures() {
        let root = temporary_root("agent-workspace");
        let destination = temporary_root("agent-pack");
        Application::initialize_workspace(&root).expect("initialize Agent workspace");
        let job = Application::create_job(&root, "Lecturer", "University")
            .expect("create Agent job")
            .data;

        match execute(WorkerRequest::LoadAgentCapabilities) {
            WorkerEvent::AgentCapabilitiesLoaded(Ok(receipt)) => {
                assert_eq!(receipt.operation, "agent.capabilities");
                assert!(!receipt.data.capabilities.is_empty());
            }
            event => panic!("unexpected capabilities event: {event:?}"),
        }

        let context = match execute(WorkerRequest::LoadAgentContext {
            root: Some(root.clone()),
            selected_job_id: Some(job.id.to_string()),
        }) {
            WorkerEvent::AgentContextLoaded {
                selected_job_id,
                result: Ok(receipt),
            } => {
                assert_eq!(selected_job_id.as_deref(), Some(job.id.as_str()));
                receipt.data
            }
            event => panic!("unexpected context event: {event:?}"),
        };
        assert_eq!(
            context.selected_job.as_ref().map(|job| job.id.as_str()),
            Some(job.id.as_str())
        );
        let serialized = serde_json::to_string(&context).expect("serialize public Agent context");
        assert_eq!(context.privacy, PrivacyClassification::Public);
        assert!(!serialized.contains("normalized_text"));
        assert!(!serialized.contains("\"body\""));

        let request = AgentPackExportRequest::new(AgentHost::Codex, &destination);
        match execute(WorkerRequest::ExportAgentPack {
            request: request.clone(),
        }) {
            WorkerEvent::AgentPackExported {
                request: returned,
                result: Ok(receipt),
            } => {
                assert_eq!(returned, request);
                assert_eq!(receipt.data.manifest.host, AgentHost::Codex);
                assert_eq!(receipt.data.manifest.files.len(), 31);
                assert!(receipt.data.manifest_path.is_file());
            }
            event => panic!("unexpected Agent export event: {event:?}"),
        }
        match execute(WorkerRequest::ExportAgentPack { request }) {
            WorkerEvent::AgentPackExported {
                result: Err(failure),
                ..
            } => assert_eq!(failure.code, ErrorCode::InputPathRejected),
            event => panic!("unexpected repeated Agent export event: {event:?}"),
        }

        std::fs::remove_dir_all(root).expect("remove Agent workspace");
        std::fs::remove_dir_all(destination).expect("remove Agent pack");
    }

    #[test]
    fn discovery_worker_commits_the_reviewed_batch_and_refreshes_jobs_after_promotion() {
        let root = temporary_root("discovery");
        let source = temporary_root("discovery-batch").with_extension("csv");
        let host_source = temporary_root("host-discovery-batch").with_extension("json");
        std::fs::write(
            &source,
            "title,organization,url,location\nLecturer,University X,https://example.edu/a,London\nLecturer,University X,https://example.edu/b,London\n",
        )
        .expect("write discovery CSV");
        std::fs::write(
            &host_source,
            serde_json::to_vec_pretty(&serde_json::json!({
                "source_kind": "host-agent",
                "source_name": "Reviewed agent leads",
                "source_url": null,
                "cursor": null,
                "observed_at": "2026-07-26T00:00:00Z",
                "leads": [{
                    "external_id": "agent-1",
                    "title": "Reader in Economics",
                    "organization": "University Y",
                    "location": null,
                    "deadline": null,
                    "url": "https://example.edu/jobs/agent-1",
                    "summary": null,
                    "metadata": {}
                }]
            }))
            .expect("serialize host-agent batch"),
        )
        .expect("write host-agent batch");
        assert!(matches!(
            execute(WorkerRequest::CreateWorkspace {
                alias: "Discovery fixture".to_owned(),
                path: root.clone(),
            }),
            WorkerEvent::WorkspaceCreated { result: Ok(_), .. }
        ));
        match execute(WorkerRequest::LoadDiscoveryCatalog) {
            WorkerEvent::DiscoveryCatalogLoaded(Ok(receipt)) => {
                assert_eq!(receipt.data.adapters.len(), 4);
            }
            event => panic!("unexpected discovery catalog event: {event:?}"),
        }
        match execute(WorkerRequest::PreviewDiscoveryImport {
            request: DiscoveryImportRequest {
                path: host_source.clone(),
                source_name: None,
                source_url: None,
                host_agent: true,
            },
        }) {
            WorkerEvent::DiscoveryImportPreviewed(Ok(receipt)) => {
                assert_eq!(receipt.data.accepted, 1);
                assert_eq!(
                    receipt.data.batch.expect("host batch").source_kind,
                    canisend_contracts::DiscoverySourceKind::HostAgent
                );
            }
            event => panic!("unexpected host-agent preview event: {event:?}"),
        }
        assert!(matches!(
            execute(WorkerRequest::PreviewDiscoveryRefresh {
                request: DiscoveryRefreshRequest {
                    adapter: DiscoveryNetworkAdapter::RssAtom,
                    endpoint: "http://127.0.0.1:9/feed".to_owned(),
                    source_name: "Unsafe local feed".to_owned(),
                    organization: None,
                },
            }),
            WorkerEvent::DiscoveryRefreshPreviewed(Err(_))
        ));

        let report = match execute(WorkerRequest::PreviewDiscoveryImport {
            request: DiscoveryImportRequest {
                path: source.clone(),
                source_name: Some("Reviewed CSV".to_owned()),
                source_url: Some("https://example.edu/jobs".to_owned()),
                host_agent: false,
            },
        }) {
            WorkerEvent::DiscoveryImportPreviewed(Ok(receipt)) => {
                assert_eq!(receipt.data.accepted, 2);
                receipt.data
            }
            event => panic!("unexpected discovery preview event: {event:?}"),
        };
        std::fs::write(
            &source,
            "title,organization,url\nChanged,Other,https://example.edu/changed\n",
        )
        .expect("replace source after preview");
        let (lead_id, committed_leads) = match execute(WorkerRequest::CommitDiscoveryImport {
            path: root.clone(),
            report,
            include_history: false,
        }) {
            WorkerEvent::DiscoveryImportCommitted(Ok(committed)) => {
                assert_eq!(committed.receipt.data.accepted, 2);
                let discovery = committed.discovery.expect("reload committed discovery");
                assert_eq!(discovery.sources.sources.len(), 1);
                assert_eq!(discovery.leads.leads.len(), 2);
                assert!(
                    discovery
                        .leads
                        .leads
                        .iter()
                        .all(|lead| lead.title == "Lecturer")
                );
                (
                    discovery.leads.leads[0].id.to_string(),
                    discovery.leads.leads,
                )
            }
            event => panic!("unexpected discovery commit event: {event:?}"),
        };
        assert_eq!(committed_leads.len(), 2);
        assert!(matches!(
            execute(WorkerRequest::LoadDiscoveryLead {
                path: root.clone(),
                lead_id: lead_id.clone(),
            }),
            WorkerEvent::DiscoveryLeadLoaded(Ok(_))
        ));
        match execute(WorkerRequest::LoadDiscoverySuggestions {
            path: root.clone(),
            lead_id: lead_id.clone(),
            limit: 5,
        }) {
            WorkerEvent::DiscoverySuggestionsLoaded(Ok(receipt)) => {
                assert!(!receipt.data.automatic_merge);
                assert_eq!(receipt.data.suggestions.len(), 1);
            }
            event => panic!("unexpected discovery suggestion event: {event:?}"),
        }

        match execute(WorkerRequest::PromoteDiscoveryLead {
            path: root.clone(),
            lead_id,
            include_history: true,
            include_archived_jobs: false,
        }) {
            WorkerEvent::DiscoveryLeadPromoted(Ok(promoted)) => {
                assert_eq!(promoted.jobs.expect("reload promoted jobs").jobs.len(), 1);
                assert_eq!(promoted.receipt.next_actions.len(), 1);
                let discovery = promoted.discovery.expect("reload promoted discovery");
                assert_eq!(discovery.leads.leads.len(), 2);
                assert!(
                    discovery
                        .leads
                        .leads
                        .iter()
                        .any(|lead| lead.status == DiscoveryLeadStatus::Promoted)
                );
            }
            event => panic!("unexpected discovery promotion event: {event:?}"),
        }

        std::fs::remove_dir_all(root).expect("remove discovery workspace");
        std::fs::remove_file(source).expect("remove discovery CSV");
        std::fs::remove_file(host_source).expect("remove host-agent discovery JSON");
    }

    #[test]
    fn task_worker_restores_scoped_preview_commit_and_recovery_state() {
        let root = temporary_root("task-workflow");
        let source = temporary_root("task-source").with_extension("txt");
        let export = temporary_root("task-export");
        let completion = temporary_root("task-completion").with_extension("json");
        std::fs::write(&source, "bounded job source").expect("write task source");
        assert!(matches!(
            execute(WorkerRequest::CreateWorkspace {
                alias: "Task fixture".to_owned(),
                path: root.clone(),
            }),
            WorkerEvent::WorkspaceCreated { result: Ok(_), .. }
        ));
        let job_id = match execute(WorkerRequest::CreateJob {
            path: root.clone(),
            title: "Lecturer".to_owned(),
            institution: "University X".to_owned(),
        }) {
            WorkerEvent::JobCreated(Ok(receipt)) => receipt.data.id,
            event => panic!("unexpected job event: {event:?}"),
        };
        assert!(matches!(
            execute(WorkerRequest::ImportLocalSource {
                path: root.clone(),
                id: job_id.to_string(),
                source: source.clone(),
            }),
            WorkerEvent::SourceImported(Ok(_))
        ));
        assert!(matches!(
            execute(WorkerRequest::StartWorkflow {
                path: root.clone(),
                id: job_id.to_string(),
            }),
            WorkerEvent::WorkflowLoaded(Ok(_))
        ));
        match execute(WorkerRequest::LoadLatestTask {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::LatestTaskLoaded {
                result: Ok(receipt),
                ..
            } => assert!(receipt.data.is_none()),
            event => panic!("unexpected empty latest-task event: {event:?}"),
        }

        let prepared = match execute(WorkerRequest::PrepareTask {
            path: root.clone(),
            request: TaskPrepareRequest {
                job_id: job_id.clone(),
                operation: TaskOperation::JobParse,
                mode: TaskExecutionMode::HostAgent,
            },
        }) {
            WorkerEvent::TaskPrepared(Ok(receipt)) => receipt.data,
            event => panic!("unexpected task prepare event: {event:?}"),
        };
        match execute(WorkerRequest::LoadLatestTask {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::LatestTaskLoaded {
                result: Ok(receipt),
                ..
            } => {
                let latest = receipt.data.expect("latest prepared task");
                assert_eq!(latest.descriptor.id, prepared.id);
                assert_eq!(latest.status, TaskStatus::Prepared);
            }
            event => panic!("unexpected latest-task event: {event:?}"),
        }
        let export_request = TaskInputExportRequest {
            task_id: prepared.id.clone(),
            destination: export.clone(),
        };
        match execute(WorkerRequest::ExportTaskInputs {
            path: root.clone(),
            request: export_request.clone(),
            private_read_consent: false,
            provider_send_consent: false,
        }) {
            WorkerEvent::TaskInputsExported(Err(failure)) => {
                assert_eq!(failure.code, ErrorCode::ConsentRequired);
            }
            event => panic!("unexpected task consent event: {event:?}"),
        }
        assert!(!export.exists());
        match execute(WorkerRequest::ExportTaskInputs {
            path: root.clone(),
            request: export_request,
            private_read_consent: true,
            provider_send_consent: false,
        }) {
            WorkerEvent::TaskInputsExported(Ok(receipt)) => {
                assert_eq!(receipt.data.files.len(), 1);
                assert_eq!(receipt.data.task_id, prepared.id);
            }
            event => panic!("unexpected task export event: {event:?}"),
        }

        assert!(matches!(
            execute(WorkerRequest::CancelTask {
                path: root.clone(),
                task_id: prepared.id.to_string(),
            }),
            WorkerEvent::TaskCancelled(Ok(receipt))
                if receipt.data.status == TaskStatus::Cancelled
        ));
        let replacement = match execute(WorkerRequest::PrepareTaskAgain {
            path: root.clone(),
            task_id: prepared.id.to_string(),
        }) {
            WorkerEvent::TaskPreparedAgain(Ok(receipt)) => {
                assert_eq!(receipt.data.previous.status, TaskStatus::Cancelled);
                receipt.data.descriptor
            }
            event => panic!("unexpected task recovery event: {event:?}"),
        };
        assert_ne!(replacement.id, prepared.id);

        let mut request = TaskCompletionRequest {
            task_id: replacement.id.clone(),
            lease_id: replacement.lease.id.clone(),
            expected_job_revision: replacement.job_revision,
            expected_inputs: expected_inputs(&replacement.input_artifacts),
            candidate: json!({"title": 3}),
        };
        std::fs::write(
            &completion,
            serde_json::to_vec(&request).expect("invalid completion JSON"),
        )
        .expect("write invalid completion");
        match execute(WorkerRequest::PreviewTaskCompletion {
            path: root.clone(),
            file: completion.clone(),
        }) {
            WorkerEvent::TaskCompletionPreviewed(Err(failure)) => {
                assert_eq!(failure.code, ErrorCode::CandidateSchemaInvalid);
                assert!(
                    failure
                        .details
                        .as_ref()
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|details| !details.is_empty())
                );
            }
            event => panic!("unexpected invalid preview event: {event:?}"),
        }
        request.candidate = json!({
            "id": "019f2f55-7c00-7000-8000-000000000801",
            "job_id": job_id,
            "title": "Lecturer",
            "institution": "University X",
            "summary": "bounded job source",
            "responsibilities": ["bounded job source"],
            "criteria": [{
                "id": "019f2f55-7c00-7000-8000-000000000802",
                "job_id": job_id,
                "kind": "teaching",
                "requirement": "bounded job source",
                "importance": "essential",
                "source_quote": "bounded job source",
                "source_span": {
                    "source": replacement.input_artifacts[0],
                    "start_byte": 0,
                    "end_byte": 18
                },
                "confidence_milli": 900,
                "confirmed": false,
                "revision": 1
            }],
            "revision": 1
        });
        std::fs::write(
            &completion,
            serde_json::to_vec(&request).expect("valid completion JSON"),
        )
        .expect("write valid completion");
        let reviewed = match execute(WorkerRequest::PreviewTaskCompletion {
            path: root.clone(),
            file: completion.clone(),
        }) {
            WorkerEvent::TaskCompletionPreviewed(Ok(receipt)) => {
                assert_eq!(receipt.data.state.status, TaskStatus::Prepared);
                receipt.data.request
            }
            event => panic!("unexpected valid preview event: {event:?}"),
        };
        std::fs::write(&completion, b"{}").expect("replace reviewed completion file");
        match execute(WorkerRequest::CommitTaskCompletion {
            path: root.clone(),
            request: reviewed,
        }) {
            WorkerEvent::TaskCompleted(Ok(receipt)) => {
                assert_eq!(receipt.data.status, TaskStatus::Committed);
                assert_eq!(receipt.data.artifact.kind, ArtifactKind::ParsedJob);
                assert!(!receipt.data.idempotent);
            }
            event => panic!("unexpected task commit event: {event:?}"),
        }
        match execute(WorkerRequest::LoadLatestTask {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::LatestTaskLoaded {
                result: Ok(receipt),
                ..
            } => assert_eq!(
                receipt.data.expect("latest committed task").status,
                TaskStatus::Committed
            ),
            event => panic!("unexpected committed latest-task event: {event:?}"),
        }

        std::fs::remove_dir_all(root).expect("remove task workspace");
        std::fs::remove_file(source).expect("remove task source");
        std::fs::remove_dir_all(export).expect("remove task export");
        std::fs::remove_file(completion).expect("remove task completion");
    }

    #[test]
    fn decision_workflow_round_trips_through_worker_and_reopens() {
        let root = temporary_root("decision-reopen");
        let source = temporary_root("decision-source").with_extension("txt");
        let profile_source = temporary_root("decision-profile").with_extension("md");
        std::fs::write(&source, "bounded job source").expect("write job source");
        std::fs::write(&profile_source, "# Private profile\n\nBounded evidence.")
            .expect("write profile source");

        assert!(matches!(
            execute(WorkerRequest::CreateWorkspace {
                alias: "Decision fixture".to_owned(),
                path: root.clone(),
            }),
            WorkerEvent::WorkspaceCreated { result: Ok(_), .. }
        ));
        let job_id = match execute(WorkerRequest::CreateJob {
            path: root.clone(),
            title: "Lecturer".to_owned(),
            institution: "University X".to_owned(),
        }) {
            WorkerEvent::JobCreated(Ok(receipt)) => receipt.data.id,
            event => panic!("unexpected job event: {event:?}"),
        };
        assert!(matches!(
            execute(WorkerRequest::ImportLocalSource {
                path: root.clone(),
                id: job_id.to_string(),
                source: source.clone(),
            }),
            WorkerEvent::SourceImported(Ok(_))
        ));
        assert!(matches!(
            execute(WorkerRequest::ImportProfileSource {
                path: root.clone(),
                source: profile_source.clone(),
                sensitivity: PrivacyClassification::PrivateLocal,
            }),
            WorkerEvent::ProfileSourceImported(Ok(_))
        ));
        assert!(matches!(
            execute(WorkerRequest::StartWorkflow {
                path: root.clone(),
                id: job_id.to_string(),
            }),
            WorkerEvent::WorkflowLoaded(Ok(_))
        ));

        let mut workspace = Workspace::open(Some(&root)).expect("open seed workspace");
        let evidence_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_evidence_normalization(&job_id, ExecutionMode::HostAgent)
            .expect("prepare evidence task");
        let evidence_request = TaskCompletionRequest {
            task_id: evidence_descriptor.id.clone(),
            lease_id: evidence_descriptor.lease.id.clone(),
            expected_job_revision: evidence_descriptor.job_revision,
            expected_inputs: expected_inputs(&evidence_descriptor.input_artifacts),
            candidate: json!({
                "profile_revision": evidence_descriptor
                    .profile_revision
                    .expect("profile revision"),
                "proposals": [{
                    "kind": "qualification",
                    "summary": "Doctorate in economics",
                    "source_quote": "# Private profile",
                    "source_span": {
                        "source": evidence_descriptor.input_artifacts[0],
                        "start_byte": 0,
                        "end_byte": 17
                    },
                    "sensitivity": "private-local"
                }]
            }),
        };
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&evidence_request)
            .expect("complete evidence proposal");

        let parse_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_job_parse(&job_id, ExecutionMode::HostAgent)
            .expect("prepare parse task");
        let parse_request = TaskCompletionRequest {
            task_id: parse_descriptor.id.clone(),
            lease_id: parse_descriptor.lease.id.clone(),
            expected_job_revision: parse_descriptor.job_revision,
            expected_inputs: expected_inputs(&parse_descriptor.input_artifacts),
            candidate: json!({
                "id": "019f2f55-7c00-7000-8000-000000000601",
                "job_id": job_id,
                "title": "Lecturer",
                "institution": "University X",
                "summary": "Teach economics",
                "responsibilities": ["Teach economics"],
                "criteria": [{
                    "id": "019f2f55-7c00-7000-8000-000000000602",
                    "job_id": job_id,
                    "kind": "qualification",
                    "requirement": "Demonstrate economics expertise",
                    "importance": "essential",
                    "source_quote": "bounded job source",
                    "source_span": {
                        "source": parse_descriptor.input_artifacts[0],
                        "start_byte": 0,
                        "end_byte": 18
                    },
                    "confidence_milli": 900,
                    "confirmed": false,
                    "revision": 1
                }],
                "revision": 1
            }),
        };
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&parse_request)
            .expect("complete parse task");
        drop(workspace);

        let mut evidence = match execute(WorkerRequest::LoadProfileEvidence {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::ProfileEvidenceLoaded {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected evidence event: {event:?}"),
        };
        evidence.items[0].confirmed = true;
        let evidence_artifact = match execute(WorkerRequest::ConfirmProfileEvidence {
            path: root.clone(),
            job_id: job_id.to_string(),
            candidate: evidence,
        }) {
            WorkerEvent::ProfileEvidenceConfirmed {
                result: Ok(receipt),
                ..
            } => receipt
                .artifacts
                .first()
                .cloned()
                .expect("confirmed evidence artifact"),
            event => panic!("unexpected evidence confirmation: {event:?}"),
        };

        let mut criteria = match execute(WorkerRequest::LoadCriteriaCandidate {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::CriteriaCandidateLoaded {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected criteria event: {event:?}"),
        };
        criteria.criteria[0].confirmed = true;
        let criterion = criteria.criteria[0].clone();
        let criteria_artifact = match execute(WorkerRequest::ConfirmCriteria {
            path: root.clone(),
            job_id: job_id.to_string(),
            candidate: criteria,
        }) {
            WorkerEvent::CriteriaConfirmed {
                result: Ok(receipt),
                ..
            } => receipt
                .artifacts
                .first()
                .cloned()
                .expect("confirmed criteria artifact"),
            event => panic!("unexpected criteria confirmation: {event:?}"),
        };

        let mut workspace = Workspace::open(Some(&root)).expect("open match workspace");
        let confirmed_evidence = EvidenceService::new(&mut workspace.database, &workspace.blobs)
            .confirmed(&job_id)
            .expect("confirmed evidence");
        let confirmed_criteria = CriteriaService::new(&mut workspace.database, &workspace.blobs)
            .confirmed(&job_id)
            .expect("confirmed criteria");
        let match_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_evidence_match(&job_id, ExecutionMode::HostAgent)
            .expect("prepare match task");
        let match_request = TaskCompletionRequest {
            task_id: match_descriptor.id.clone(),
            lease_id: match_descriptor.lease.id.clone(),
            expected_job_revision: match_descriptor.job_revision,
            expected_inputs: expected_inputs(&match_descriptor.input_artifacts),
            candidate: json!({
                "job_id": job_id,
                "criteria_artifact": criteria_artifact,
                "evidence_artifact": evidence_artifact,
                "proposals": [{
                    "criterion": {
                        "id": confirmed_criteria.criteria[0].id,
                        "revision": confirmed_criteria.criteria[0].revision
                    },
                    "evidence": [{
                        "id": confirmed_evidence.items[0].id,
                        "revision": confirmed_evidence.items[0].revision
                    }],
                    "strength": "strong",
                    "rationale": "The confirmed profile evidence supports the criterion.",
                    "gap": null,
                    "prohibited_claims": ["Do not claim an unverified teaching award."]
                }]
            }),
        };
        let match_artifact = TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&match_request)
            .expect("complete match task")
            .artifact;
        drop(workspace);

        let matches = match execute(WorkerRequest::LoadCurrentMatches {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::CurrentMatchesLoaded {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected current matches: {event:?}"),
        };
        assert_eq!(matches.matches.len(), 1);
        assert_eq!(matches.matches[0].criterion.id, criterion.id);
        assert_eq!(
            matches.matches[0].prohibited_claims,
            vec!["Do not claim an unverified teaching award."]
        );

        let mut plan = match execute(WorkerRequest::LoadPlanCandidate {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::PlanCandidateLoaded {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected plan candidate: {event:?}"),
        };
        assert_eq!(plan.decision, ApplicationDecision::Hold);
        plan.decision = ApplicationDecision::Apply;
        plan.strategy.positioning =
            "Lead with confirmed economics expertise and bounded claims.".to_owned();
        let confirmed_plan = match execute(WorkerRequest::ConfirmPlan {
            path: root.clone(),
            job_id: job_id.to_string(),
            candidate: plan,
        }) {
            WorkerEvent::PlanConfirmed {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected plan confirmation: {event:?}"),
        };
        assert_eq!(confirmed_plan.decision, ApplicationDecision::Apply);
        assert_eq!(confirmed_plan.revision.get(), 1);

        assert!(matches!(
            execute(WorkerRequest::LoadWorkspace { path: root.clone() }),
            WorkerEvent::WorkspaceLoaded(Ok(_))
        ));
        let reopened_evidence = match execute(WorkerRequest::LoadProfileEvidence {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::ProfileEvidenceLoaded {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected reopened evidence: {event:?}"),
        };
        let reopened_criteria = match execute(WorkerRequest::LoadCriteriaCandidate {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::CriteriaCandidateLoaded {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected reopened criteria: {event:?}"),
        };
        let reopened_matches = match execute(WorkerRequest::LoadCurrentMatches {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::CurrentMatchesLoaded {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected reopened matches: {event:?}"),
        };
        let reopened_plan = match execute(WorkerRequest::LoadCurrentPlan {
            path: root.clone(),
            job_id: job_id.to_string(),
        }) {
            WorkerEvent::CurrentPlanLoaded {
                result: Ok(receipt),
                ..
            } => receipt.data,
            event => panic!("unexpected reopened plan: {event:?}"),
        };
        assert_eq!(reopened_evidence.revision.get(), 1);
        assert_eq!(reopened_criteria.revision.get(), 1);
        assert_eq!(reopened_matches.id, matches.id);
        assert_eq!(reopened_matches.revision, matches.revision);
        assert_eq!(reopened_plan.id, confirmed_plan.id);
        assert_eq!(reopened_plan.revision, confirmed_plan.revision);
        assert_eq!(reopened_plan.matches_artifact, match_artifact);
        assert_eq!(
            reopened_plan.strategy.positioning,
            confirmed_plan.strategy.positioning
        );
        let application_plan = Application::current_application_plan(
            &root,
            job_id.as_str(),
            PrivateReadConsent::granted_by_user(),
        )
        .expect("load plan through shared application facade")
        .data;
        assert_eq!(application_plan, reopened_plan);

        std::fs::remove_dir_all(root).expect("remove decision workspace");
        std::fs::remove_file(source).expect("remove job source");
        std::fs::remove_file(profile_source).expect("remove profile source");
    }

    fn expected_inputs(
        artifacts: &[canisend_contracts::ArtifactReference],
    ) -> Vec<ExpectedInputRevision> {
        artifacts
            .iter()
            .map(|artifact| ExpectedInputRevision {
                artifact_id: artifact.id.clone(),
                revision: artifact.revision,
                sha256: artifact.sha256.clone(),
            })
            .collect()
    }
}
