use std::path::Path;

use canisend_contracts::{
    DocumentKind, EntityId, RenderManifestRecord, RenderedDocumentRecord, SafeRelativePath,
};
use canisend_io::EmbeddedTypstCompiler;
use canisend_store::RenderService;
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateExportConsent, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
    package::{parse_job_path, private_export_consent_required},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderExportRequest {
    pub job_id: EntityId,
    pub destination: SafeRelativePath,
}

impl RenderExportRequest {
    pub fn try_new(job_id: &str, destination: &str) -> Result<Self, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let destination = parse_job_path(&job_id, destination)?;
        Ok(Self {
            job_id,
            destination,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderExportReadModel {
    pub render_manifest: RenderManifestRecord,
    pub destination: SafeRelativePath,
    pub files: Vec<SafeRelativePath>,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderPreviewReadModel {
    pub document: RenderedDocumentRecord,
    pub pdf_bytes: Vec<u8>,
}

impl Application {
    pub fn build_render(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<RenderManifestRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let mut executor = EmbeddedTypstCompiler::new();
        let (artifact, manifest) =
            RenderService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .build(&job_id, &mut executor)?;
        Ok(render_receipt(
            "render.build",
            "rendered",
            "Built and validated current PDF artifacts",
            manifest,
        )
        .with_artifacts([artifact]))
    }

    pub fn current_render(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<RenderManifestRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let (artifact, manifest) =
            RenderService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .current(&job_id)?;
        Ok(render_receipt(
            "render.show",
            "available",
            "Loaded current render manifest",
            manifest,
        )
        .with_artifacts([artifact]))
    }

    pub fn preview_render(
        root: &Path,
        job_id: &str,
        kind: DocumentKind,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<RenderPreviewReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let mut executor = EmbeddedTypstCompiler::new();
        let (document, pdf_bytes) =
            RenderService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .preview(&job_id, kind, &mut executor)?;
        let artifact = document.pdf_artifact.clone();
        Ok(ActionReceipt::new(
            "render.preview",
            "available",
            format!(
                "Loaded the exact validated {} PDF for local preview",
                document_kind_name(kind)
            ),
            RenderPreviewReadModel {
                document,
                pdf_bytes,
            },
        )
        .with_artifacts([artifact]))
    }

    pub fn export_render(
        root: &Path,
        request: RenderExportRequest,
        consent: Option<PrivateExportConsent>,
    ) -> Result<ActionReceipt<RenderExportReadModel>, ApplicationError> {
        if consent.is_none() {
            return Err(private_export_consent_required(
                "The operation writes private PDFs and their exact manifest under jobs/JOB_ID/",
            ));
        }
        let mut workspace = open_workspace(root)?;
        let workspace_root = workspace.paths.root.clone();
        let mut executor = EmbeddedTypstCompiler::new();
        let (artifact, manifest, files) = RenderService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .export(&request.job_id, &request.destination, &mut executor)?;
        let count = files.len();
        Ok(ActionReceipt::new(
            "render.export",
            "exported",
            format!(
                "Exported {count} validated render file(s) under {}; submission performed: no",
                request.destination
            ),
            RenderExportReadModel {
                render_manifest: manifest,
                destination: request.destination,
                files,
                submission_performed: false,
            },
        )
        .with_artifacts([artifact]))
    }
}

const fn document_kind_name(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::CoverLetter => "cover-letter",
        DocumentKind::ResearchStatement => "research-statement",
        DocumentKind::TeachingStatement => "teaching-statement",
        DocumentKind::Cv => "cv",
    }
}

fn render_receipt(
    operation: &'static str,
    status: &'static str,
    summary: &'static str,
    manifest: RenderManifestRecord,
) -> ActionReceipt<RenderManifestRecord> {
    let documents = manifest.documents.len();
    let pages = manifest
        .documents
        .iter()
        .map(|document| u64::from(document.page_count))
        .sum::<u64>();
    let bytes = manifest
        .documents
        .iter()
        .map(|document| document.byte_count)
        .sum::<u64>();
    ActionReceipt::new(
        operation,
        status,
        format!(
            "{summary}: {documents} document(s), {pages} page(s), {bytes} byte(s); \
             submission performed: no"
        ),
        manifest,
    )
}

#[cfg(test)]
mod tests {
    use canisend_contracts::{DocumentKind, ErrorCode};

    use crate::{Application, PrivateExportConsent, PrivateReadConsent, RenderExportRequest};

    const JOB_ID: &str = "019f2f55-7c00-7000-8000-000000000101";

    #[test]
    fn render_export_paths_are_job_scoped() {
        assert!(RenderExportRequest::try_new(JOB_ID, &format!("jobs/{JOB_ID}/rendered")).is_ok());
        let error = RenderExportRequest::try_new(JOB_ID, "jobs/other/rendered")
            .expect_err("outside render path");
        assert_eq!(error.classify().code, ErrorCode::InputPathRejected);
    }

    #[test]
    fn render_export_consent_is_required_before_workspace_access() {
        let missing = std::env::temp_dir().join(format!(
            "canisend-app-render-consent-{}",
            std::process::id()
        ));
        let request = RenderExportRequest::try_new(JOB_ID, &format!("jobs/{JOB_ID}/rendered"))
            .expect("valid render request");
        let error = Application::export_render(&missing, request.clone(), None)
            .expect_err("private render export without consent");
        assert_eq!(error.classify().code, ErrorCode::ConsentRequired);
        assert!(!missing.exists());

        let error = Application::export_render(
            &missing,
            request,
            Some(PrivateExportConsent::granted_by_user()),
        )
        .expect_err("missing workspace with consent");
        assert_eq!(error.classify().code, ErrorCode::WorkspaceNotFound);
    }

    #[test]
    fn render_preview_validates_identity_before_private_workspace_access() {
        let missing = std::env::temp_dir().join(format!(
            "canisend-app-render-preview-{}",
            std::process::id()
        ));
        let consent = PrivateReadConsent::granted_by_user();
        let error =
            Application::preview_render(&missing, "not-an-entity-id", DocumentKind::Cv, consent)
                .expect_err("invalid preview job ID");
        assert_eq!(error.classify().code, ErrorCode::InputInvalid);
        assert!(!missing.exists());

        let error = Application::preview_render(&missing, JOB_ID, DocumentKind::Cv, consent)
            .expect_err("missing preview workspace");
        assert_eq!(error.classify().code, ErrorCode::WorkspaceNotFound);
    }
}
