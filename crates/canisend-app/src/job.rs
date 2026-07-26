use std::path::{Path, PathBuf};

use canisend_contracts::{
    ActorKind, JobRecord, PrivacyClassification, SourceKind, SourceRecord, WorkflowStatusData,
};
use canisend_io::{
    HttpFetcher, IoAdapterError, RemoteDocumentKind, extract_pdf_text, read_local_pdf,
    read_local_text,
};
use canisend_store::{JobService, NewSource, StoreError, WorkflowService};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, NetworkFetchConsent, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
};

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

impl Application {
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
        let artifacts = std::iter::once(source.original.clone())
            .chain(source.normalized_text.clone())
            .collect::<Vec<_>>();
        Ok(ActionReceipt::new(
            "job.import",
            "imported",
            format!("Imported {} source", source.content_type),
            SourceImportReadModel { job, source },
        )
        .with_artifacts(artifacts))
    }
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
