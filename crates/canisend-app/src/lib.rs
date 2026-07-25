#![forbid(unsafe_code)]

mod cli_install;
mod update;

use std::{
    fs,
    path::{Path, PathBuf},
};

use canisend_contracts::{
    AGENT_PROTOCOL, ActorKind, EntityId, JobRecord, PUBLIC_SCHEMA_VERSION, PrivacyClassification,
    RESOURCE_FORMAT, SourceKind, SourceRecord, WORKSPACE_FORMAT, WorkflowStatusData,
    WorkspaceCheckData, WorkspaceStatusData,
};
use canisend_io::{
    EmbeddedRenderError, HttpFetcher, IoAdapterError, RemoteDocumentKind, extract_pdf_text,
    read_local_pdf, read_local_text, render_acceptance_probe,
};
use canisend_store::{
    BACKUP_FORMAT, BackupResult, JobService, NewSource, StoreError, WorkflowService, Workspace,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use cli_install::{
    CliInstallState, CliInstallStatus, CliVersionRelation, TerminalInstallConsent,
};
pub use update::UpdateCheckReadModel;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("{0}")]
    Store(#[from] StoreError),
    #[error("{0}")]
    Input(#[from] IoAdapterError),
    #[error("{0}")]
    Render(#[from] EmbeddedRenderError),
    #[error("invalid entity ID: {0}")]
    InvalidEntityId(String),
    #[error("input is invalid: {0}")]
    InvalidInput(String),
    #[error("embedded resources failed verification: {0}")]
    ResourceIntegrity(String),
    #[error("CLI installation failed: {0}")]
    CliInstall(String),
    #[error("update check failed: {0}")]
    UpdateCheck(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionReceipt<T> {
    pub operation: String,
    pub status: String,
    pub summary: String,
    pub data: T,
}

impl<T> ActionReceipt<T> {
    pub(crate) fn new(
        operation: &'static str,
        status: &'static str,
        summary: impl Into<String>,
        data: T,
    ) -> Self {
        Self {
            operation: operation.to_owned(),
            status: status.to_owned(),
            summary: summary.into(),
            data,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductSummary {
    pub product: String,
    pub version: String,
    pub protocol: String,
    pub workspace_format: String,
    pub resource_format: String,
    pub public_schema_version: String,
    pub target_os: String,
    pub target_arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorSummary {
    pub healthy: bool,
    pub embedded_resources: usize,
    pub embedded_renderer: bool,
    pub rendered_pages: usize,
    pub render_warning_count: usize,
    pub python_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceReadModel {
    pub path: PathBuf,
    pub status: WorkspaceStatusData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceHealthReadModel {
    pub path: PathBuf,
    pub check: WorkspaceCheckData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupReadModel {
    pub destination: PathBuf,
    pub format: String,
    pub blob_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobListReadModel {
    pub workspace: PathBuf,
    pub include_archived: bool,
    pub jobs: Vec<JobRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobDetailReadModel {
    pub workspace: PathBuf,
    pub job: JobRecord,
    pub sources: Vec<SourceRecord>,
    pub workflow: Option<WorkflowStatusData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceImportReadModel {
    pub job: JobRecord,
    pub source: SourceRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateReadConsent(());

impl PrivateReadConsent {
    #[must_use]
    pub const fn granted_by_user() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkFetchConsent(());

impl NetworkFetchConsent {
    #[must_use]
    pub const fn granted_by_user() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Application;

impl Application {
    #[must_use]
    pub fn product_summary() -> ProductSummary {
        ProductSummary {
            product: "canisend".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol: AGENT_PROTOCOL.to_owned(),
            workspace_format: WORKSPACE_FORMAT.to_owned(),
            resource_format: RESOURCE_FORMAT.to_owned(),
            public_schema_version: PUBLIC_SCHEMA_VERSION.to_owned(),
            target_os: std::env::consts::OS.to_owned(),
            target_arch: std::env::consts::ARCH.to_owned(),
        }
    }

    pub fn doctor() -> Result<ActionReceipt<DoctorSummary>, ApplicationError> {
        canisend_resources::verify().map_err(ApplicationError::ResourceIntegrity)?;
        let rendered = render_acceptance_probe()?;
        let data = DoctorSummary {
            healthy: true,
            embedded_resources: canisend_resources::manifest().len(),
            embedded_renderer: true,
            rendered_pages: rendered.page_count() as usize,
            render_warning_count: rendered.warning_count(),
            python_required: false,
        };
        Ok(ActionReceipt::new(
            "product.doctor",
            "healthy",
            "Native resources and embedded PDF renderer verified",
            data,
        ))
    }

    pub fn cli_install_status(
        source: Option<&Path>,
        destination: &Path,
    ) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
        cli_install::inspect(source, destination)
    }

    pub fn install_cli(
        source: &Path,
        destination: &Path,
        replace_existing: bool,
        consent: TerminalInstallConsent,
    ) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
        cli_install::install(source, destination, replace_existing, consent)
    }

    pub fn uninstall_cli(
        source: Option<&Path>,
        destination: &Path,
        consent: TerminalInstallConsent,
    ) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
        cli_install::uninstall(source, destination, consent)
    }

    pub fn check_for_updates(
        consent: NetworkFetchConsent,
    ) -> Result<ActionReceipt<UpdateCheckReadModel>, ApplicationError> {
        update::check(consent)
    }

    pub fn initialize_workspace(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceReadModel>, ApplicationError> {
        require_new_or_empty_directory(root)?;
        let workspace = Workspace::init(root)?;
        let status = workspace.status()?;
        Ok(ActionReceipt::new(
            "workspace.init",
            "initialized",
            format!(
                "Initialized workspace at {}",
                workspace.paths.root.display()
            ),
            WorkspaceReadModel {
                path: workspace.paths.root,
                status,
            },
        ))
    }

    pub fn workspace_status(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceReadModel>, ApplicationError> {
        let workspace = open_workspace(root)?;
        let status = workspace.status()?;
        Ok(ActionReceipt::new(
            "workspace.status",
            "available",
            format!("Workspace has {} job(s)", status.job_count),
            WorkspaceReadModel {
                path: workspace.paths.root,
                status,
            },
        ))
    }

    pub fn check_workspace(
        root: &Path,
    ) -> Result<ActionReceipt<WorkspaceHealthReadModel>, ApplicationError> {
        let workspace = open_workspace(root)?;
        let check = workspace.check()?;
        let status = if check.ok { "healthy" } else { "issues-found" };
        Ok(ActionReceipt::new(
            "workspace.check",
            status,
            if check.ok {
                "Workspace integrity check passed".to_owned()
            } else {
                format!("Workspace check found {} issue(s)", check.issues.len())
            },
            WorkspaceHealthReadModel {
                path: workspace.paths.root,
                check,
            },
        ))
    }

    pub fn backup_workspace(
        root: &Path,
        destination: &Path,
    ) -> Result<ActionReceipt<BackupReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let BackupResult {
            directory,
            manifest,
        } = workspace.backup(destination)?;
        Ok(ActionReceipt::new(
            "workspace.backup",
            "verified",
            format!("Verified backup created at {}", directory.display()),
            BackupReadModel {
                destination: directory,
                format: BACKUP_FORMAT.to_owned(),
                blob_count: manifest.blobs.len(),
            },
        ))
    }

    pub fn list_jobs(
        root: &Path,
        include_archived: bool,
    ) -> Result<ActionReceipt<JobListReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let jobs =
            JobService::new(&mut workspace.database, &workspace.blobs).list(include_archived)?;
        Ok(ActionReceipt::new(
            "job.list",
            "available",
            format!("Loaded {} job(s)", jobs.len()),
            JobListReadModel {
                workspace: workspace.paths.root,
                include_archived,
                jobs,
            },
        ))
    }

    pub fn create_job(
        root: &Path,
        title: &str,
        institution: &str,
    ) -> Result<ActionReceipt<JobRecord>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let job = JobService::new(&mut workspace.database, &workspace.blobs).create(
            title,
            institution,
            ActorKind::User,
        )?;
        Ok(ActionReceipt::new(
            "job.create",
            "created",
            format!("Created {} at {}", job.title, job.institution),
            job,
        ))
    }

    pub fn archive_job(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<JobRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .archive(&job_id, ActorKind::User)?;
        Ok(ActionReceipt::new(
            "job.archive",
            "archived",
            format!("Archived {}", job.title),
            job,
        ))
    }

    pub fn job_detail(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<JobDetailReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let service = JobService::new(&mut workspace.database, &workspace.blobs);
        let job = service.get(&job_id)?;
        let sources = service.sources(&job_id)?;
        let workflow = match WorkflowService::new(&mut workspace.database).status(&job_id) {
            Ok(status) => Some(status),
            Err(StoreError::WorkflowNotFound(_)) => None,
            Err(error) => return Err(error.into()),
        };
        Ok(ActionReceipt::new(
            "job.show",
            "available",
            format!("{} source(s) attached", sources.len()),
            JobDetailReadModel {
                workspace: workspace.paths.root,
                job,
                sources,
                workflow,
            },
        ))
    }

    pub fn import_local_job_source(
        root: &Path,
        job_id: &str,
        path: &Path,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<SourceImportReadModel>, ApplicationError> {
        let source = local_source(path)?;
        Self::commit_job_source(root, job_id, source)
    }

    pub fn import_url_job_source(
        root: &Path,
        job_id: &str,
        url: &str,
        _consent: NetworkFetchConsent,
    ) -> Result<ActionReceipt<SourceImportReadModel>, ApplicationError> {
        if url.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "URL cannot be empty".to_owned(),
            ));
        }
        let document = HttpFetcher::new().fetch(url.trim())?;
        let normalized_text = if document.kind == RemoteDocumentKind::Pdf {
            extract_pdf_text(document.original_bytes.clone())?.normalized_text
        } else {
            document
                .normalized_text
                .ok_or(IoAdapterError::TextUnavailable)?
        };
        let source = NewSource {
            kind: SourceKind::UserUrl,
            original_bytes: document.original_bytes,
            normalized_text,
            source_url: Some(document.source_url),
            final_url: Some(document.final_url),
            content_type: document.content_type,
            redirect_chain: document.redirect_chain,
            privacy: PrivacyClassification::PrivateLocal,
        };
        Self::commit_job_source(root, job_id, source)
    }

    pub fn start_workflow(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<WorkflowStatusData>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).start(&job_id)?;
        Ok(ActionReceipt::new(
            "workflow.start",
            "started",
            "Workflow is ready for its next action",
            status,
        ))
    }

    pub fn workflow_status(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<WorkflowStatusData>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).status(&job_id)?;
        Ok(ActionReceipt::new(
            "workflow.status",
            "available",
            format!("{} blocker(s)", status.blockers.len()),
            status,
        ))
    }

    fn commit_job_source(
        root: &Path,
        job_id: &str,
        source: NewSource,
    ) -> Result<ActionReceipt<SourceImportReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let mut service = JobService::new(&mut workspace.database, &workspace.blobs);
        let source = service.import_source(&job_id, source, ActorKind::User)?;
        let job = service.get(&job_id)?;
        Ok(ActionReceipt::new(
            "job.import",
            "imported",
            format!("Imported {} source", source.content_type),
            SourceImportReadModel { job, source },
        ))
    }
}

fn open_workspace(root: &Path) -> Result<Workspace, StoreError> {
    Workspace::open(Some(root))
}

fn require_new_or_empty_directory(root: &Path) -> Result<(), ApplicationError> {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(root).map_err(|error| {
        ApplicationError::InvalidInput(format!(
            "cannot inspect new workspace directory {}: {error}",
            root.display()
        ))
    })?;
    if entries.next().is_some() {
        return Err(ApplicationError::InvalidInput(format!(
            "new workspace directory must be empty: {}",
            root.display()
        )));
    }
    Ok(())
}

fn parse_entity_id(value: &str) -> Result<EntityId, ApplicationError> {
    EntityId::try_new(value).map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))
}

fn local_source(path: &Path) -> Result<NewSource, ApplicationError> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        let document = read_local_pdf(path)?;
        Ok(NewSource {
            kind: SourceKind::LocalFile,
            original_bytes: document.original_bytes,
            normalized_text: document.normalized_text,
            source_url: None,
            final_url: None,
            content_type: "application/pdf".to_owned(),
            redirect_chain: Vec::new(),
            privacy: PrivacyClassification::PrivateLocal,
        })
    } else {
        let document = read_local_text(path)?;
        Ok(NewSource {
            kind: SourceKind::LocalFile,
            original_bytes: document.original_bytes,
            normalized_text: document.normalized_text,
            source_url: None,
            final_url: None,
            content_type: document.content_type.to_owned(),
            redirect_chain: Vec::new(),
            privacy: PrivacyClassification::PrivateLocal,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{Application, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn facade_completes_workspace_job_import_and_workflow_slice() {
        let root = temporary_root("vertical");
        let source = temporary_root("advert").with_extension("md");
        fs::write(
            &source,
            "# Lecturer in Economics\n\nTeach and publish research.\n",
        )
        .expect("write fixture");

        let initialized = Application::initialize_workspace(&root).expect("initialize workspace");
        assert_eq!(initialized.operation, "workspace.init");
        assert_eq!(initialized.data.status.job_count, 0);

        let job = Application::create_job(&root, "Lecturer in Economics", "University X")
            .expect("create job")
            .data;
        let imported = Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import source");
        assert_eq!(imported.data.job.revision.get(), 2);
        assert_eq!(
            imported.data.source.content_type,
            "text/markdown; charset=utf-8"
        );

        let workflow = Application::start_workflow(&root, job.id.as_str()).expect("start workflow");
        assert_eq!(workflow.data.stages.len(), 10);
        assert_eq!(
            workflow.data.stages[0].status,
            canisend_contracts::StageExecutionStatus::Complete
        );

        let jobs = Application::list_jobs(&root, false).expect("list jobs");
        assert_eq!(jobs.data.jobs.len(), 1);
        let detail = Application::job_detail(&root, job.id.as_str()).expect("job detail");
        assert_eq!(detail.data.sources.len(), 1);
        assert!(detail.data.workflow.is_some());

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove fixture");
    }

    #[test]
    fn routine_read_models_do_not_contain_private_source_bodies() {
        let root = temporary_root("privacy");
        let source = temporary_root("private").with_extension("txt");
        let sentinel = "PRIVATE-SENTINEL-DO-NOT-LEAK";
        fs::write(&source, sentinel).expect("write fixture");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Reader", "University Y")
            .expect("create job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import source");

        let detail = Application::job_detail(&root, job.id.as_str()).expect("job detail");
        let serialized = serde_json::to_string(&detail).expect("serialize read model");
        assert!(!serialized.contains(sentinel));

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove fixture");
    }

    #[test]
    fn facade_rejects_initializing_over_existing_user_files() {
        let root = temporary_root("non-empty");
        fs::create_dir_all(&root).expect("create directory");
        let sentinel = root.join("keep-me.txt");
        fs::write(&sentinel, "user-owned").expect("write sentinel");

        assert!(Application::initialize_workspace(&root).is_err());
        assert_eq!(
            fs::read_to_string(&sentinel).expect("sentinel remains"),
            "user-owned"
        );
        assert!(!root.join("canisend.toml").exists());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
