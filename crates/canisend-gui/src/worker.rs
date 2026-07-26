use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, BackupReadModel, CliInstallStatus, DoctorSummary,
    JobDetailReadModel, JobListReadModel, NetworkFetchConsent, PrivateReadConsent,
    ProfileSourceImportReadModel, ProfileSourceListReadModel, SourceImportReadModel,
    TerminalInstallConsent, UpdateCheckReadModel, WorkflowBeginRequest, WorkflowCompleteRequest,
    WorkflowControlReadModel, WorkflowRerunPreview, WorkflowRerunRequest, WorkspaceHealthReadModel,
    WorkspaceReadModel, WorkspaceRepairReadModel, WorkspaceRestoreReadModel,
};
use canisend_contracts::{
    ApplicationPlanCandidate, ApplicationPlanRecord, CriteriaSetRecord, EvidenceCatalogRecord,
    EvidenceMatchSetRecord, JobRecord, PrivacyClassification, WorkflowStage, WorkflowStatusData,
};

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
    WorkflowLoaded(Result<ActionReceipt<WorkflowStatusData>, String>),
    WorkflowControlsLoaded(Result<ActionReceipt<WorkflowControlReadModel>, String>),
    WorkflowMutated(Result<ActionReceipt<WorkflowControlReadModel>, String>),
    WorkflowRerunPreviewed(Result<ActionReceipt<WorkflowRerunPreview>, String>),
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use canisend_app::{WorkflowBeginRequest, WorkflowRerunRequest};
    use canisend_contracts::{ExecutionMode, WorkflowStage};

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
}
