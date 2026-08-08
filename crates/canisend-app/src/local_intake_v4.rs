use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use canisend_contracts::{
    ApplicationFieldValueV3, ConsentScope, ContentRevisionReferenceV3, NextAction,
    PrivacyClassification, RequirementPriorityV3, Sha256Digest, WorkflowPackId, WorkflowPackItemId,
    WorkspaceSourceKindV4,
};
use canisend_io::{read_local_pdf, read_local_text};
use canisend_store::{
    ApplicationAssociationServiceV4, ApplicationFlowCreateRequestV3, ApplicationFlowReadModelV3,
    ApplicationFlowServiceV3, NewWorkspaceSourceV4, validate_application_flow_create_request,
};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, PastedTextIntakePreviewRequestV4,
    PrivateReadConsent, application::open_workspace, application_flow_v3::requested_built_in_pack,
    intake_v4::digest,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFileIntakePreviewRequestV4 {
    pub pack_id: WorkflowPackId,
    pub title: String,
    pub opportunity_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub application_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub path: PathBuf,
    pub requirement_category: WorkflowPackItemId,
    pub requirement_priority: RequirementPriorityV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceDuplicateSignalV4 {
    pub source: ContentRevisionReferenceV3,
    pub original_bytes_match: bool,
    pub normalized_text_match: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFileIntakePreviewReadModelV4 {
    pub preview_sha256: Sha256Digest,
    pub original_sha256: Sha256Digest,
    pub normalized_sha256: Sha256Digest,
    pub source_kind: WorkspaceSourceKindV4,
    pub content_type: String,
    pub original_bytes: u64,
    pub normalized_text_bytes: u64,
    pub normalized_lines: u64,
    pub pdf_page_count: Option<u64>,
    pub duplicates: Vec<SourceDuplicateSignalV4>,
    pub application: ApplicationFlowCreateRequestV3,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFileIntakeCommitRequestV4 {
    pub preview: LocalFileIntakePreviewRequestV4,
    pub expected_preview_sha256: Sha256Digest,
}

struct PreparedLocalFileIntakeV4 {
    preview: LocalFileIntakePreviewReadModelV4,
    source: NewWorkspaceSourceV4,
}

impl Application {
    pub fn preview_local_file_intake_v4(
        workspace_root: &Path,
        request: LocalFileIntakePreviewRequestV4,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<LocalFileIntakePreviewReadModelV4>, ApplicationError> {
        require_private_read_consent(consent)?;
        let prepared = prepare_local_file_intake(workspace_root, &request)?;
        Ok(ActionReceipt::new(
            "application.intake.local-file.preview",
            "previewed",
            "Read one consented bounded local Source and proposed exact-span Requirements without Workspace mutation",
            prepared.preview,
        ))
    }

    pub fn commit_local_file_intake_v4(
        workspace_root: &Path,
        request: LocalFileIntakeCommitRequestV4,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        require_private_read_consent(consent)?;
        let pack = requested_built_in_pack(&request.preview.pack_id)?;
        let prepared = prepare_local_file_intake(workspace_root, &request.preview)?;
        if prepared.preview.preview_sha256 != request.expected_preview_sha256 {
            return Err(ApplicationError::InvalidInput(
                "local-file intake preview is stale or the file bytes no longer match the reviewed digest"
                    .to_owned(),
            ));
        }
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let committed =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create_with_source(
                    &pack,
                    prepared.preview.application,
                    prepared.source,
                    Some(ConsentScope::ReadPrivateInputs),
                )?;
        Ok(ActionReceipt::new(
            "application.intake.local-file.commit",
            "created",
            "Committed the reviewed local Source, private-read consent, explicit Application link, and proposed Requirements",
            committed,
        ))
    }
}

fn prepare_local_file_intake(
    workspace_root: &Path,
    request: &LocalFileIntakePreviewRequestV4,
) -> Result<PreparedLocalFileIntakeV4, ApplicationError> {
    Application::workspace_status_v4(workspace_root)?;
    let locator = request.path.to_str().ok_or_else(|| {
        ApplicationError::InvalidInput("local Source path must be valid UTF-8".to_owned())
    })?;
    let (kind, content_type, original_bytes, normalized_text, pdf_page_count) = if request
        .path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        let document = read_local_pdf(&request.path)?;
        (
            WorkspaceSourceKindV4::TextPdf,
            "application/pdf".to_owned(),
            document.original_bytes,
            document.normalized_text,
            Some(u64::try_from(document.page_count).map_err(|_| {
                ApplicationError::InvalidInput("PDF page count overflow".to_owned())
            })?),
        )
    } else {
        let document = read_local_text(&request.path)?;
        (
            WorkspaceSourceKindV4::LocalFile,
            document.content_type.to_owned(),
            document.original_bytes,
            document.normalized_text,
            None,
        )
    };
    let pasted = PastedTextIntakePreviewRequestV4 {
        pack_id: request.pack_id.clone(),
        title: request.title.clone(),
        opportunity_metadata: request.opportunity_metadata.clone(),
        application_metadata: request.application_metadata.clone(),
        source_text: normalized_text.clone(),
        requirement_category: request.requirement_category.clone(),
        requirement_priority: request.requirement_priority,
    };
    let mut application = Application::preview_pasted_text_intake_v4(workspace_root, pasted)?
        .data
        .application;
    if kind == WorkspaceSourceKindV4::TextPdf {
        application
            .requirements
            .retain(|requirement| !is_pdf_page_marker(&requirement.statement));
        let pack = requested_built_in_pack(&request.pack_id)?;
        validate_application_flow_create_request(&pack, &application)?;
    }
    let original_sha256 = digest(&original_bytes)?;
    let normalized_sha256 = digest(normalized_text.as_bytes())?;
    let duplicates = {
        let mut workspace = open_workspace(workspace_root)?;
        ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs)
            .source_duplicates(&original_sha256, &normalized_sha256)?
            .into_iter()
            .map(|record| SourceDuplicateSignalV4 {
                source: ContentRevisionReferenceV3 {
                    id: record.id,
                    revision: record.revision,
                    sha256: record.normalized_sha256.clone(),
                },
                original_bytes_match: record.original_sha256 == original_sha256,
                normalized_text_match: record.normalized_sha256 == normalized_sha256,
            })
            .collect::<Vec<_>>()
    };
    let preview_bytes = serde_json::to_vec(&(
        "canisend.local-file-intake-preview/v4",
        request,
        kind,
        &content_type,
        &original_sha256,
        &normalized_sha256,
        pdf_page_count,
        &duplicates,
        &application,
    ))
    .map_err(|error| {
        ApplicationError::InvalidInput(format!("could not encode local intake preview: {error}"))
    })?;
    let preview_sha256 = digest(&preview_bytes)?;
    let original_byte_count = u64::try_from(original_bytes.len())
        .map_err(|_| ApplicationError::InvalidInput("Source byte count overflow".to_owned()))?;
    let normalized_text_bytes = u64::try_from(normalized_text.len())
        .map_err(|_| ApplicationError::InvalidInput("Source byte count overflow".to_owned()))?;
    let normalized_lines = u64::try_from(normalized_text.lines().count())
        .map_err(|_| ApplicationError::InvalidInput("Source line count overflow".to_owned()))?;
    Ok(PreparedLocalFileIntakeV4 {
        preview: LocalFileIntakePreviewReadModelV4 {
            preview_sha256,
            original_sha256,
            normalized_sha256,
            source_kind: kind,
            content_type: content_type.clone(),
            original_bytes: original_byte_count,
            normalized_text_bytes,
            normalized_lines,
            pdf_page_count,
            duplicates,
            application,
            submission_performed: false,
        },
        source: NewWorkspaceSourceV4 {
            kind,
            locator: locator.to_owned(),
            final_locator: None,
            redirect_chain: Vec::new(),
            content_type,
            original_bytes,
            normalized_text,
            privacy: PrivacyClassification::PrivateLocal,
        },
    })
}

fn require_private_read_consent(
    consent: Option<PrivateReadConsent>,
) -> Result<(), ApplicationError> {
    if consent.is_some() {
        return Ok(());
    }
    Err(ApplicationError::ConsentRequired {
        message: "Local-file intake reads private bytes from an explicit user-selected path"
            .to_owned(),
        remediation: NextAction {
            action: "grant private read consent".to_owned(),
            description:
                "Confirm this one bounded local-file read, then repeat the preview or commit"
                    .to_owned(),
        },
    })
}

fn is_pdf_page_marker(statement: &str) -> bool {
    statement
        .strip_prefix("--- Page ")
        .and_then(|value| value.strip_suffix(" ---"))
        .is_some_and(|page| !page.is_empty() && page.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use canisend_contracts::{ApplicationFieldValueV3, Revision};
    use canisend_io::EmbeddedTypstCompiler;
    use canisend_store::{ApplicationAssociationServiceV4, Workspace};

    use super::*;

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-local-intake-v4-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn request(path: PathBuf) -> LocalFileIntakePreviewRequestV4 {
        LocalFileIntakePreviewRequestV4 {
            pack_id: WorkflowPackId::try_new(crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            title: "Community programme".to_owned(),
            opportunity_metadata: BTreeMap::from([(
                item("organization"),
                ApplicationFieldValueV3::ShortText("Example Foundation".to_owned()),
            )]),
            application_metadata: BTreeMap::from([(
                item("status"),
                ApplicationFieldValueV3::Choice(item("planning")),
            )]),
            path,
            requirement_category: item("format"),
            requirement_priority: RequirementPriorityV3::Mandatory,
        }
    }

    fn academic_request(path: PathBuf) -> LocalFileIntakePreviewRequestV4 {
        LocalFileIntakePreviewRequestV4 {
            pack_id: WorkflowPackId::try_new(crate::ACADEMIC_JOB_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            title: "Research fellowship".to_owned(),
            opportunity_metadata: BTreeMap::from([(
                item("institution"),
                ApplicationFieldValueV3::ShortText("Example University".to_owned()),
            )]),
            application_metadata: BTreeMap::new(),
            path,
            requirement_category: item("qualification"),
            requirement_priority: RequirementPriorityV3::Mandatory,
        }
    }

    #[test]
    fn consent_precedes_workspace_and_file_access() {
        let root = root("consent");
        let source = root.join("missing.txt");
        let error = Application::preview_local_file_intake_v4(&root, request(source), None)
            .expect_err("consent required before access");
        assert!(matches!(error, ApplicationError::ConsentRequired { .. }));
        assert!(!root.exists());
    }

    #[test]
    fn local_text_commit_preserves_provenance_consent_and_duplicate_signal() {
        let root = root("text");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let source_path = root.join("requirements.md");
        fs::write(
            &source_path,
            b"First requirement  \r\nSecond requirement\r\n",
        )
        .expect("write local Source");
        let request = request(source_path.clone());
        let consent = PrivateReadConsent::granted_by_user();
        let preview =
            Application::preview_local_file_intake_v4(&root, request.clone(), Some(consent))
                .expect("local preview")
                .data;
        assert_eq!(preview.source_kind, WorkspaceSourceKindV4::LocalFile);
        assert_eq!(preview.application.requirements.len(), 2);
        assert!(preview.duplicates.is_empty());
        assert_ne!(preview.original_sha256, preview.normalized_sha256);

        let committed = Application::commit_local_file_intake_v4(
            &root,
            LocalFileIntakeCommitRequestV4 {
                preview: request.clone(),
                expected_preview_sha256: preview.preview_sha256,
            },
            Some(consent),
        )
        .expect("local commit")
        .data;
        let duplicate_preview =
            Application::preview_local_file_intake_v4(&root, request, Some(consent))
                .expect("duplicate preview")
                .data;
        assert_eq!(duplicate_preview.duplicates.len(), 1);
        assert!(duplicate_preview.duplicates[0].original_bytes_match);
        assert!(duplicate_preview.duplicates[0].normalized_text_match);

        let mut workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        let associations =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let links = associations
            .source_associations(&committed.stored.snapshot.application.id)
            .expect("Source association");
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].consent_scope,
            Some(ConsentScope::ReadPrivateInputs)
        );
        let source = associations
            .source(&links[0].source.id, Revision::try_new(1).expect("revision"))
            .expect("Source revision");
        assert_eq!(source.kind, WorkspaceSourceKindV4::LocalFile);
        assert_eq!(source.locator, source_path.to_str().expect("UTF-8 path"));
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn text_pdf_commit_preserves_original_pdf_and_ignores_page_markers() {
        let root = root("pdf");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let source_path = root.join("requirements.pdf");
        let pdf = EmbeddedTypstCompiler::new()
            .compile_pdf(
                "= Programme requirements\n\nDoctorate required.\n\nResearch plan required.",
            )
            .expect("render text PDF")
            .into_bytes();
        fs::write(&source_path, &pdf).expect("write PDF Source");
        let request = academic_request(source_path);
        let consent = PrivateReadConsent::granted_by_user();
        let preview =
            Application::preview_local_file_intake_v4(&root, request.clone(), Some(consent))
                .expect("PDF preview")
                .data;
        assert_eq!(preview.source_kind, WorkspaceSourceKindV4::TextPdf);
        assert_eq!(preview.pdf_page_count, Some(1));
        assert_eq!(
            preview.original_bytes,
            u64::try_from(pdf.len()).expect("PDF size")
        );
        assert!(
            preview
                .application
                .requirements
                .iter()
                .all(|requirement| !is_pdf_page_marker(&requirement.statement))
        );
        let committed = Application::commit_local_file_intake_v4(
            &root,
            LocalFileIntakeCommitRequestV4 {
                preview: request,
                expected_preview_sha256: preview.preview_sha256,
            },
            Some(consent),
        )
        .expect("PDF commit")
        .data;
        let mut workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        let associations =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let links = associations
            .source_associations(&committed.stored.snapshot.application.id)
            .expect("Source association");
        let source = associations
            .source(&links[0].source.id, links[0].source.revision)
            .expect("PDF Source revision");
        assert_eq!(source.kind, WorkspaceSourceKindV4::TextPdf);
        assert_eq!(source.original_sha256, preview.original_sha256);
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn changed_file_rejects_stale_commit_without_authority_mutation() {
        let root = root("stale");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let source_path = root.join("requirements.txt");
        fs::write(&source_path, b"Original requirement.\n").expect("write original Source");
        let request = request(source_path.clone());
        let consent = PrivateReadConsent::granted_by_user();
        let preview =
            Application::preview_local_file_intake_v4(&root, request.clone(), Some(consent))
                .expect("preview")
                .data;
        fs::write(&source_path, b"Changed requirement.\n").expect("change Source");
        let error = Application::commit_local_file_intake_v4(
            &root,
            LocalFileIntakeCommitRequestV4 {
                preview: request,
                expected_preview_sha256: preview.preview_sha256,
            },
            Some(consent),
        )
        .expect_err("stale local Source");
        assert!(matches!(error, ApplicationError::InvalidInput(_)));
        assert!(
            Application::list_application_models_v3(&root)
                .expect("Applications")
                .data
                .is_empty()
        );
        let mut workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        let duplicates =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs)
                .source_duplicates(&preview.original_sha256, &preview.normalized_sha256)
                .expect("Source duplicate query");
        assert!(duplicates.is_empty());
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
