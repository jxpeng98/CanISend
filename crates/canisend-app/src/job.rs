use std::{
    fs,
    path::{Path, PathBuf},
};

use canisend_contracts::{
    ActorKind, JobRecord, NextAction, PrivacyClassification, Revision, Sha256Digest, SourceKind,
    SourceRecord, WorkflowStatusData,
};
use canisend_io::{
    HttpFetcher, IoAdapterError, RemoteDocumentKind, extract_pdf_text, read_local_pdf,
    read_local_text,
};
use canisend_store::{JobService, NewSource, StoreError, WorkflowService};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ActionReceipt, Application, ApplicationError, NetworkFetchConsent, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
    compatibility::{
        LegacyCompatibilityAccess, LegacyCompatibilityOperation, job_compatibility_notice,
        workspace_compatibility_notice,
    },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobIntakeSourceKind {
    LocalFile,
    Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobIntakeIssueSeverity {
    Information,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobIntakeValidationIssue {
    pub code: String,
    pub severity: JobIntakeIssueSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobIntakeExtractionReadModel {
    pub content_type: String,
    pub original_bytes: u64,
    pub normalized_text_bytes: u64,
    pub normalized_lines: u64,
    pub pdf_pages: Option<u64>,
    pub semantic_fields_pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobIntakeProvenanceReadModel {
    pub source_kind: JobIntakeSourceKind,
    pub requested_locator: String,
    pub final_url: Option<String>,
    pub redirect_chain: Vec<String>,
    pub original_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobIntakeMutationReadModel {
    pub subject: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobIntakePreviewReadModel {
    pub workspace: PathBuf,
    pub job: JobRecord,
    pub expected_job_revision: Revision,
    pub provenance: JobIntakeProvenanceReadModel,
    pub extraction: JobIntakeExtractionReadModel,
    pub validation_issues: Vec<JobIntakeValidationIssue>,
    pub intended_mutations: Vec<JobIntakeMutationReadModel>,
}

#[derive(Debug, Clone)]
pub struct PreparedJobSource {
    workspace: PathBuf,
    job_id: canisend_contracts::EntityId,
    expected_job_revision: Revision,
    source: NewSource,
    preview: ActionReceipt<JobIntakePreviewReadModel>,
}

impl PreparedJobSource {
    #[must_use]
    pub fn preview(&self) -> &ActionReceipt<JobIntakePreviewReadModel> {
        &self.preview
    }
}

#[derive(Debug)]
struct PreparedSourceCandidate {
    source: NewSource,
    kind: JobIntakeSourceKind,
    requested_locator: String,
    pdf_pages: Option<u64>,
}

impl Application {
    pub fn list_jobs(
        root: &Path,
        include_archived: bool,
    ) -> Result<ActionReceipt<JobListReadModel>, ApplicationError> {
        let compatibility = workspace_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobList,
            LegacyCompatibilityAccess::Read,
        )?;
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
        )
        .with_compatibility(compatibility))
    }

    pub fn create_job(
        root: &Path,
        title: &str,
        institution: &str,
    ) -> Result<ActionReceipt<JobRecord>, ApplicationError> {
        let compatibility = workspace_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobCreate,
            LegacyCompatibilityAccess::Write,
        )?;
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
        )
        .with_compatibility(compatibility))
    }

    pub fn archive_job(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<JobRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let compatibility = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobArchive,
            LegacyCompatibilityAccess::Write,
            &job_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .archive(&job_id, ActorKind::User)?;
        Ok(ActionReceipt::new(
            "job.archive",
            "archived",
            format!("Archived {}", job.title),
            job,
        )
        .with_compatibility(compatibility))
    }

    pub fn job_detail(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<JobDetailReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let compatibility = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobShow,
            LegacyCompatibilityAccess::Read,
            &job_id,
        )?;
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
        )
        .with_compatibility(compatibility))
    }

    pub fn import_local_job_source(
        root: &Path,
        job_id: &str,
        path: &Path,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<SourceImportReadModel>, ApplicationError> {
        let job_id_value = parse_entity_id(job_id)?;
        let _ = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobImport,
            LegacyCompatibilityAccess::Write,
            &job_id_value,
        )?;
        let source = local_source(path)?;
        Self::commit_job_source(root, job_id, source)
    }

    pub fn prepare_local_job_source(
        root: &Path,
        job_id: &str,
        path: &Path,
        _consent: PrivateReadConsent,
    ) -> Result<PreparedJobSource, ApplicationError> {
        let job_id_value = parse_entity_id(job_id)?;
        let _ = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobIntakePreview,
            LegacyCompatibilityAccess::Write,
            &job_id_value,
        )?;
        let canonical_path = fs::canonicalize(path).map_err(|source| IoAdapterError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let (source, pdf_pages) = local_source_with_page_count(&canonical_path)?;
        Self::prepare_job_source(
            root,
            job_id,
            PreparedSourceCandidate {
                source,
                kind: JobIntakeSourceKind::LocalFile,
                requested_locator: canonical_path.to_string_lossy().into_owned(),
                pdf_pages,
            },
        )
    }

    pub fn import_url_job_source(
        root: &Path,
        job_id: &str,
        url: &str,
        _consent: NetworkFetchConsent,
    ) -> Result<ActionReceipt<SourceImportReadModel>, ApplicationError> {
        let job_id_value = parse_entity_id(job_id)?;
        let _ = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobImport,
            LegacyCompatibilityAccess::Write,
            &job_id_value,
        )?;
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

    pub fn prepare_url_job_source(
        root: &Path,
        job_id: &str,
        url: &str,
        _consent: NetworkFetchConsent,
    ) -> Result<PreparedJobSource, ApplicationError> {
        let job_id_value = parse_entity_id(job_id)?;
        let _ = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobIntakePreview,
            LegacyCompatibilityAccess::Write,
            &job_id_value,
        )?;
        if url.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "URL cannot be empty".to_owned(),
            ));
        }
        let requested_locator = url.trim().to_owned();
        let document = HttpFetcher::new().fetch(&requested_locator)?;
        let (normalized_text, pdf_pages) = if document.kind == RemoteDocumentKind::Pdf {
            let extracted = extract_pdf_text(document.original_bytes.clone())?;
            (
                extracted.normalized_text,
                Some(u64::try_from(extracted.page_count).map_err(|_| {
                    ApplicationError::InvalidInput("PDF page count is too large".to_owned())
                })?),
            )
        } else {
            (
                document
                    .normalized_text
                    .ok_or(IoAdapterError::TextUnavailable)?,
                None,
            )
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
        Self::prepare_job_source(
            root,
            job_id,
            PreparedSourceCandidate {
                source,
                kind: JobIntakeSourceKind::Url,
                requested_locator,
                pdf_pages,
            },
        )
    }

    pub fn commit_prepared_job_source(
        prepared: PreparedJobSource,
    ) -> Result<ActionReceipt<SourceImportReadModel>, ApplicationError> {
        let compatibility = job_compatibility_notice(
            &prepared.workspace,
            LegacyCompatibilityOperation::JobIntakeCommit,
            LegacyCompatibilityAccess::Write,
            &prepared.job_id,
        )?;
        let mut workspace = open_workspace(&prepared.workspace)?;
        let mut service = JobService::new(&mut workspace.database, &workspace.blobs);
        let current = service.get(&prepared.job_id)?;
        if current.revision != prepared.expected_job_revision {
            return Err(StoreError::DependencyConflict(format!(
                "job {} changed from revision {} to {} after the intake preview",
                current.id,
                prepared.expected_job_revision.get(),
                current.revision.get()
            ))
            .into());
        }
        let source = service.import_source(&prepared.job_id, prepared.source, ActorKind::User)?;
        let job = service.get(&prepared.job_id)?;
        let artifacts = std::iter::once(source.original.clone())
            .chain(source.normalized_text.clone())
            .collect::<Vec<_>>();
        Ok(ActionReceipt::new(
            "job.intake.commit",
            "imported",
            format!("Imported reviewed {} source", source.content_type),
            SourceImportReadModel { job, source },
        )
        .with_artifacts(artifacts)
        .with_compatibility(compatibility))
    }

    fn prepare_job_source(
        root: &Path,
        job_id: &str,
        candidate: PreparedSourceCandidate,
    ) -> Result<PreparedJobSource, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let compatibility = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobIntakePreview,
            LegacyCompatibilityAccess::Write,
            &job_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let service = JobService::new(&mut workspace.database, &workspace.blobs);
        let job = service.get(&job_id)?;
        if job.archived {
            return Err(StoreError::JobArchived(job_id.to_string()).into());
        }

        let original_sha256 = Sha256Digest::try_new(hex::encode(Sha256::digest(
            &candidate.source.original_bytes,
        )))
        .map_err(StoreError::from)?;
        let existing_sources = service.sources(&job_id)?;
        let mut validation_issues = Vec::new();
        if existing_sources
            .iter()
            .any(|source| source.original.sha256 == original_sha256)
        {
            validation_issues.push(JobIntakeValidationIssue {
                code: "source.duplicate-content".to_owned(),
                severity: JobIntakeIssueSeverity::Warning,
                message:
                    "This exact source content is already attached to the selected application."
                        .to_owned(),
            });
        }
        validation_issues.push(JobIntakeValidationIssue {
            code: "source.semantic-fields-pending".to_owned(),
            severity: JobIntakeIssueSeverity::Information,
            message:
                "Title, criteria, responsibilities, and deadlines remain unconfirmed until the job-parse task is reviewed."
                    .to_owned(),
        });

        let original_bytes = u64::try_from(candidate.source.original_bytes.len())
            .map_err(|_| ApplicationError::InvalidInput("source is too large".to_owned()))?;
        let normalized_text_bytes = u64::try_from(candidate.source.normalized_text.len())
            .map_err(|_| ApplicationError::InvalidInput("source text is too large".to_owned()))?;
        let normalized_lines = u64::try_from(candidate.source.normalized_text.lines().count())
            .map_err(|_| ApplicationError::InvalidInput("source has too many lines".to_owned()))?;
        let preview = JobIntakePreviewReadModel {
            workspace: workspace.paths.root.clone(),
            job: job.clone(),
            expected_job_revision: job.revision,
            provenance: JobIntakeProvenanceReadModel {
                source_kind: candidate.kind,
                requested_locator: candidate.requested_locator,
                final_url: candidate.source.final_url.clone(),
                redirect_chain: candidate.source.redirect_chain.clone(),
                original_sha256,
            },
            extraction: JobIntakeExtractionReadModel {
                content_type: candidate.source.content_type.clone(),
                original_bytes,
                normalized_text_bytes,
                normalized_lines,
                pdf_pages: candidate.pdf_pages,
                semantic_fields_pending: true,
            },
            validation_issues,
            intended_mutations: vec![
                JobIntakeMutationReadModel {
                    subject: format!("job:{}", job.id),
                    action: "attach-source".to_owned(),
                    description: format!(
                        "Attach one immutable {} source and its normalized text",
                        candidate.source.content_type
                    ),
                },
                JobIntakeMutationReadModel {
                    subject: format!("job:{}", job.id),
                    action: "advance-revision".to_owned(),
                    description: format!(
                        "Advance the selected job from revision {} after confirmation",
                        job.revision.get()
                    ),
                },
            ],
        };
        let warnings = preview
            .validation_issues
            .iter()
            .filter(|issue| issue.severity == JobIntakeIssueSeverity::Warning)
            .map(|issue| issue.message.clone())
            .collect::<Vec<_>>();
        let receipt = ActionReceipt::new(
            "job.intake.preview",
            "awaiting-user",
            format!(
                "Prepared a body-free preview for {} without changing the workspace",
                preview.extraction.content_type
            ),
            preview,
        )
        .with_warnings(warnings)
        .with_next_actions([NextAction {
            action: "commit the exact reviewed source preview".to_owned(),
            description: "CanISend will reject the preview if the selected job revision changed."
                .to_owned(),
        }])
        .with_compatibility(compatibility);
        Ok(PreparedJobSource {
            workspace: workspace.paths.root,
            job_id,
            expected_job_revision: job.revision,
            source: candidate.source,
            preview: receipt,
        })
    }

    fn commit_job_source(
        root: &Path,
        job_id: &str,
        source: NewSource,
    ) -> Result<ActionReceipt<SourceImportReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let compatibility = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::JobImport,
            LegacyCompatibilityAccess::Write,
            &job_id,
        )?;
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
        .with_artifacts(artifacts)
        .with_compatibility(compatibility))
    }
}

fn local_source(path: &Path) -> Result<NewSource, ApplicationError> {
    local_source_with_page_count(path).map(|(source, _)| source)
}

fn local_source_with_page_count(path: &Path) -> Result<(NewSource, Option<u64>), ApplicationError> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        let document = read_local_pdf(path)?;
        let pages = u64::try_from(document.page_count).map_err(|_| {
            ApplicationError::InvalidInput("PDF page count is too large".to_owned())
        })?;
        Ok((
            NewSource {
                kind: SourceKind::LocalFile,
                original_bytes: document.original_bytes,
                normalized_text: document.normalized_text,
                source_url: None,
                final_url: None,
                content_type: "application/pdf".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            Some(pages),
        ))
    } else {
        let document = read_local_text(path)?;
        Ok((
            NewSource {
                kind: SourceKind::LocalFile,
                original_bytes: document.original_bytes,
                normalized_text: document.normalized_text,
                source_url: None,
                final_url: None,
                content_type: document.content_type.to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::ErrorCode;

    use crate::{Application, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-job-intake-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn intake_preview_is_body_free_and_commits_the_exact_reviewed_source() {
        let root = temporary_root("preview");
        let source = temporary_root("advert").with_extension("md");
        let sentinel = "PRIVATE-JOB-INTAKE-SENTINEL";
        fs::write(&source, format!("# Lecturer\n\n{sentinel}\n")).expect("write source");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer", "University")
            .expect("create job")
            .data;

        let prepared = Application::prepare_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("prepare source");
        let preview_json =
            serde_json::to_string(prepared.preview()).expect("serialize body-free preview");
        assert!(!preview_json.contains(sentinel));
        assert_eq!(prepared.preview().operation, "job.intake.preview");
        assert_eq!(prepared.preview().data.expected_job_revision, job.revision);
        assert_eq!(prepared.preview().data.extraction.pdf_pages, None);
        assert!(prepared.preview().data.extraction.semantic_fields_pending);

        let unchanged = Application::job_detail(&root, job.id.as_str()).expect("unchanged job");
        assert!(unchanged.data.sources.is_empty());
        assert_eq!(unchanged.data.job.revision, job.revision);

        let committed =
            Application::commit_prepared_job_source(prepared).expect("commit reviewed source");
        assert_eq!(committed.operation, "job.intake.commit");
        assert_eq!(committed.data.job.source_ids.len(), 1);
        assert_eq!(committed.data.job.revision.get(), job.revision.get() + 1);

        let duplicate = Application::prepare_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("prepare duplicate");
        assert!(
            duplicate
                .preview()
                .data
                .validation_issues
                .iter()
                .any(|issue| issue.code == "source.duplicate-content")
        );

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
    }

    #[test]
    fn intake_commit_rejects_a_stale_job_revision() {
        let root = temporary_root("stale");
        let reviewed = temporary_root("reviewed").with_extension("txt");
        let intervening = temporary_root("intervening").with_extension("txt");
        fs::write(&reviewed, "Reviewed advert").expect("write reviewed source");
        fs::write(&intervening, "Different advert").expect("write intervening source");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer", "University")
            .expect("create job")
            .data;
        let prepared = Application::prepare_local_job_source(
            &root,
            job.id.as_str(),
            &reviewed,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("prepare source");
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &intervening,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("intervening import");

        let error =
            Application::commit_prepared_job_source(prepared).expect_err("stale preview must fail");
        assert_eq!(error.classify().code, ErrorCode::WorkspaceConflict);
        let detail = Application::job_detail(&root, job.id.as_str()).expect("job detail");
        assert_eq!(detail.data.sources.len(), 1);

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(reviewed).expect("remove reviewed source");
        fs::remove_file(intervening).expect("remove intervening source");
    }
}
