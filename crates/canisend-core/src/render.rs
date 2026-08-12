use canisend_contracts::{
    ApplicationPackBindingV3, ArtifactReference, DeliverableRecordV3, DocumentRecord, Revision,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPdfOutput {
    pub pdf_bytes: Vec<u8>,
    pub page_count: u32,
    pub warning_count: usize,
    pub elapsed_millis: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderExecutionOutput {
    pub typst_source: String,
    pub pdf_bytes: Vec<u8>,
    pub page_count: u32,
    pub warning_count: usize,
    pub elapsed_millis: u128,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RenderError {
    #[error("embedded application document template is not UTF-8")]
    DocumentTemplateEncoding,
    #[error("document contains {count} unresolved template field(s)")]
    UnresolvedTemplateFields { count: usize },
    #[error("generated Typst source exceeds the {max_bytes}-byte render limit")]
    ProjectionSourceTooLarge { max_bytes: usize },
    #[error("workflow Pack template is not UTF-8")]
    PackTemplateEncoding,
    #[error("Deliverable content is not UTF-8")]
    DeliverableContentEncoding,
    #[error("Deliverable must be approved and materialized before rendering")]
    DeliverableNotApproved,
    #[error("Typst source exceeds the {max_bytes}-byte render limit")]
    SourceTooLarge { max_bytes: usize },
    #[error("embedded Typst compilation failed ({kind}, {diagnostic_count} diagnostic(s))")]
    CompileFailed {
        kind: &'static str,
        diagnostic_count: usize,
    },
    #[error("embedded PDF export failed ({diagnostic_count} diagnostic(s))")]
    PdfExportFailed { diagnostic_count: usize },
    #[error("rendered PDF exceeds the {max_bytes}-byte output limit")]
    PdfTooLarge { max_bytes: usize },
    #[error("embedded render exceeded the {max_millis}-millisecond time budget")]
    TimeBudgetExceeded { max_millis: u128 },
    #[error("embedded renderer returned an invalid PDF")]
    InvalidPdf,
    #[error("embedded renderer returned an encrypted PDF")]
    EncryptedPdf,
    #[error("rendered PDF page count is outside the supported range")]
    PageCountInvalid,
}

impl RenderError {
    #[must_use]
    pub const fn is_projection(&self) -> bool {
        matches!(
            self,
            Self::DocumentTemplateEncoding
                | Self::UnresolvedTemplateFields { .. }
                | Self::ProjectionSourceTooLarge { .. }
                | Self::PackTemplateEncoding
                | Self::DeliverableContentEncoding
                | Self::DeliverableNotApproved
        )
    }
}

pub trait RenderExecutor {
    fn project_document(
        &mut self,
        source_artifact: &ArtifactReference,
        document: &DocumentRecord,
    ) -> Result<String, RenderError>;

    fn render_pdf(&mut self, source: &str) -> Result<RenderedPdfOutput, RenderError>;

    fn validate_pdf(&mut self, bytes: &[u8]) -> Result<u32, RenderError>;

    fn project_deliverable(
        &mut self,
        template: &[u8],
        pack: &ApplicationPackBindingV3,
        application_revision: Revision,
        deliverable: &DeliverableRecordV3,
        content: &[u8],
    ) -> Result<String, RenderError>;

    fn render_document(
        &mut self,
        source_artifact: &ArtifactReference,
        document: &DocumentRecord,
    ) -> Result<RenderExecutionOutput, RenderError> {
        let typst_source = self.project_document(source_artifact, document)?;
        let rendered = self.render_pdf(&typst_source)?;
        Ok(RenderExecutionOutput {
            typst_source,
            pdf_bytes: rendered.pdf_bytes,
            page_count: rendered.page_count,
            warning_count: rendered.warning_count,
            elapsed_millis: rendered.elapsed_millis,
        })
    }
}
