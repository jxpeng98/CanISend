use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, BackupReadModel, CliInstallStatus, DoctorSummary,
    JobDetailReadModel, JobListReadModel, NetworkFetchConsent, PrivateReadConsent,
    SourceImportReadModel, TerminalInstallConsent, UpdateCheckReadModel, WorkspaceHealthReadModel,
    WorkspaceReadModel, WorkspaceRepairReadModel, WorkspaceRestoreReadModel,
};
use canisend_contracts::{JobRecord, WorkflowStatusData};

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
    StartWorkflow {
        path: PathBuf,
        id: String,
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
    WorkflowLoaded(Result<ActionReceipt<WorkflowStatusData>, String>),
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
        WorkerRequest::StartWorkflow { path, id } => WorkerEvent::WorkflowLoaded(
            Application::start_workflow(&path, &id).map_err(|error| error.to_string()),
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
        std::fs::remove_dir_all(backup).expect("remove backup fixture");
        std::fs::remove_dir_all(restored).expect("remove restored fixture");
    }
}
