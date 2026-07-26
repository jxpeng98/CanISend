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

    use canisend_app::{
        Application, PrivateReadConsent, WorkflowBeginRequest, WorkflowRerunRequest,
    };
    use canisend_contracts::{
        ApplicationDecision, ExecutionMode, ExpectedInputRevision, TaskCompletionRequest,
        WorkflowStage,
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
