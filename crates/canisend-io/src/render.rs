use std::{
    fmt::Write as _,
    time::{Duration, Instant},
};

use canisend_contracts::{ArtifactReference, DocumentKind, DocumentRecord};
use canisend_resources::{ResourceDescriptor, ResourceId, get};
use thiserror::Error;
use typst_as_lib::{TypstAsLibError, TypstEngine};

pub const MAX_TYPST_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_RENDER_PDF_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RENDER_MILLIS: u128 = 10_000;

const CROSS_PLATFORM_RENDER_PROBE: &str = r#"
#set page(paper: "a4", margin: 22mm)
#set text(font: "Libertinus Serif", size: 10.5pt)

= CanISend cross-platform acceptance

Unicode: résumé — Ελληνικά — Кириллица — naïve façade.

Mathematics: $ sum_(i=1)^n i = (n(n+1))/2 $ and $ integral_0^1 x^2 dif x = 1/3 $.

#link("https://example.edu/jobs?id=42&lang=en")[https://example.edu/jobs?id=42&lang=en]

- Evidence-backed claim
- Revision-bound citation
  - Nested application detail

#pagebreak()

= Explicit second page

#table(
  columns: (1fr, 1fr),
  [Field], [Value],
  [Runtime], [Rust-native],
  [Renderer], [Embedded Typst],
)
"#;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TypstProjectionError {
    #[error("embedded application document template is not UTF-8")]
    TemplateEncoding,
    #[error("document contains {count} unresolved template field(s)")]
    UnresolvedTemplateFields { count: usize },
    #[error("generated Typst source exceeds the {max_bytes}-byte render limit")]
    SourceTooLarge { max_bytes: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentTemplateDescriptor {
    pub bundle_id: &'static str,
    pub entrypoint: &'static str,
    pub resource: ResourceDescriptor,
}

const fn document_template_resource(kind: DocumentKind) -> ResourceId {
    match kind {
        DocumentKind::Cv => ResourceId::TemplateModernproCv,
        DocumentKind::CoverLetter
        | DocumentKind::ResearchStatement
        | DocumentKind::TeachingStatement => ResourceId::TemplateModernproCoverletter,
    }
}

const fn document_template_bundle(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::Cv => "modernpro-cv",
        DocumentKind::CoverLetter
        | DocumentKind::ResearchStatement
        | DocumentKind::TeachingStatement => "modernpro-coverletter",
    }
}

#[must_use]
pub fn document_template_descriptor(kind: DocumentKind) -> DocumentTemplateDescriptor {
    DocumentTemplateDescriptor {
        bundle_id: document_template_bundle(kind),
        entrypoint: "canisend_render_document",
        resource: get(document_template_resource(kind)).descriptor,
    }
}

pub fn project_document_typst(
    source_artifact: &ArtifactReference,
    document: &DocumentRecord,
) -> Result<String, TypstProjectionError> {
    let unresolved = document
        .placeholders
        .iter()
        .filter(|placeholder| placeholder.resolution.is_none())
        .count();
    if unresolved > 0 {
        return Err(TypstProjectionError::UnresolvedTemplateFields { count: unresolved });
    }
    let template_descriptor = document_template_descriptor(document.kind);
    let template_resource = document_template_resource(document.kind);
    let template = std::str::from_utf8(get(template_resource).bytes)
        .map_err(|_| TypstProjectionError::TemplateEncoding)?;
    let mut output = String::with_capacity(template.len() + 2048);
    output.push_str(template);
    output.push_str(
        "\n\n// Managed CanISend Typst projection. Structured artifacts remain authoritative.\n",
    );
    writeln!(
        output,
        "// template: bundle:{} resource:{} version:{} sha256:{} entrypoint:{}",
        template_descriptor.bundle_id,
        template_descriptor.resource.id,
        template_descriptor.resource.version,
        template_descriptor.resource.sha256,
        template_descriptor.entrypoint,
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "// source-artifact: {}@{} sha256:{}",
        source_artifact.id,
        source_artifact.revision.get(),
        source_artifact.sha256
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "// document: {}@{} job:{}",
        document.id,
        document.revision.get(),
        document.job_id
    )
    .expect("writing to String cannot fail");
    output.push_str("#let canisend_document_data = (\n");
    writeln!(output, "  id: {},", typst_string(document.id.as_str()))
        .expect("writing to String cannot fail");
    writeln!(output, "  revision: {},", document.revision.get())
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "  kind: {},",
        typst_string(document_kind(document.kind))
    )
    .expect("writing to String cannot fail");
    writeln!(output, "  title: {},", typst_string(&document.title))
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "  generated-date: {},",
        typst_string(&document.generation.created_at.as_str()[..10])
    )
    .expect("writing to String cannot fail");
    output.push_str("  sections: (\n");
    for section in &document.sections {
        writeln!(
            output,
            "    // section: {}@{}",
            section.id,
            section.revision.get()
        )
        .expect("writing to String cannot fail");
        for claim in &section.claims {
            writeln!(
                output,
                "    // claim: {}@{} citations:{}",
                claim.id,
                claim.revision.get(),
                claim.citations.len()
            )
            .expect("writing to String cannot fail");
        }
        output.push_str("    (\n");
        writeln!(output, "      id: {},", typst_string(section.id.as_str()))
            .expect("writing to String cannot fail");
        match &section.heading {
            Some(heading) => writeln!(output, "      heading: {},", typst_string(heading)),
            None => writeln!(output, "      heading: none,"),
        }
        .expect("writing to String cannot fail");
        writeln!(output, "      body: {},", typst_string(section.body.trim()))
            .expect("writing to String cannot fail");
        output.push_str("    ),\n");
    }
    output.push_str("  ),\n  fields: (\n");
    for placeholder in &document.placeholders {
        let value = placeholder
            .resolution
            .as_deref()
            .expect("unresolved fields rejected above");
        writeln!(
            output,
            "    (key: {}, value: {}),",
            typst_string(&placeholder.key),
            typst_string(value)
        )
        .expect("writing to String cannot fail");
    }
    output.push_str("  ),\n)\n\n#canisend_render_document(canisend_document_data)\n");
    if output.len() > MAX_TYPST_SOURCE_BYTES {
        return Err(TypstProjectionError::SourceTooLarge {
            max_bytes: MAX_TYPST_SOURCE_BYTES,
        });
    }
    Ok(output)
}

fn typst_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control.is_control() => {
                write!(output, "\\u{{{:X}}}", control as u32)
                    .expect("writing to String cannot fail");
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

const fn document_kind(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::CoverLetter => "cover-letter",
        DocumentKind::ResearchStatement => "research-statement",
        DocumentKind::TeachingStatement => "teaching-statement",
        DocumentKind::Cv => "cv",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPdf {
    bytes: Vec<u8>,
    page_count: u32,
    warning_count: usize,
    elapsed: Duration,
}

impl RenderedPdf {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    #[must_use]
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    #[must_use]
    pub const fn warning_count(&self) -> usize {
        self.warning_count
    }

    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EmbeddedRenderError {
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

#[derive(Debug, Default, Clone, Copy)]
pub struct EmbeddedTypstCompiler;

impl EmbeddedTypstCompiler {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub fn compile_pdf(&self, source: &str) -> Result<RenderedPdf, EmbeddedRenderError> {
        if source.len() > MAX_TYPST_SOURCE_BYTES {
            return Err(EmbeddedRenderError::SourceTooLarge {
                max_bytes: MAX_TYPST_SOURCE_BYTES,
            });
        }

        let started = Instant::now();
        let engine = TypstEngine::builder()
            .main_file(source)
            .fonts(typst_assets::fonts())
            .build();
        let compiled = engine.compile();
        let warning_count = compiled.warnings.len();
        let document = compiled.output.map_err(safe_compile_error)?;
        enforce_time_budget(started.elapsed())?;
        let bytes = typst_pdf::pdf(&document, &Default::default()).map_err(|diagnostics| {
            EmbeddedRenderError::PdfExportFailed {
                diagnostic_count: diagnostics.len(),
            }
        })?;
        let elapsed = started.elapsed();
        enforce_time_budget(elapsed)?;
        if bytes.len() > MAX_RENDER_PDF_BYTES {
            return Err(EmbeddedRenderError::PdfTooLarge {
                max_bytes: MAX_RENDER_PDF_BYTES,
            });
        }
        let page_count = validate_rendered_pdf(&bytes)?;
        Ok(RenderedPdf {
            bytes,
            page_count,
            warning_count,
            elapsed,
        })
    }
}

pub fn render_acceptance_probe() -> Result<RenderedPdf, EmbeddedRenderError> {
    EmbeddedTypstCompiler::new().compile_pdf(CROSS_PLATFORM_RENDER_PROBE)
}

pub fn validate_rendered_pdf(bytes: &[u8]) -> Result<u32, EmbeddedRenderError> {
    if bytes.len() > MAX_RENDER_PDF_BYTES || !bytes.starts_with(b"%PDF-") {
        return Err(EmbeddedRenderError::InvalidPdf);
    }
    let document = lopdf::Document::load_mem(bytes).map_err(|_| EmbeddedRenderError::InvalidPdf)?;
    if document.is_encrypted() {
        return Err(EmbeddedRenderError::EncryptedPdf);
    }
    let page_count = document.get_pages().len();
    if page_count == 0 || page_count > crate::pdf::MAX_PDF_PAGES {
        return Err(EmbeddedRenderError::PageCountInvalid);
    }
    u32::try_from(page_count).map_err(|_| EmbeddedRenderError::PageCountInvalid)
}

fn safe_compile_error(error: TypstAsLibError) -> EmbeddedRenderError {
    let (kind, diagnostic_count) = match error {
        TypstAsLibError::TypstSource(diagnostics) => ("source", diagnostics.len()),
        TypstAsLibError::TypstFile(_) => ("file", 1),
        TypstAsLibError::MainSourceFileDoesNotExist(_) => ("main-source", 1),
        TypstAsLibError::HintedString(_) => ("hinted", 1),
        TypstAsLibError::Unspecified(_) => ("unspecified", 1),
    };
    EmbeddedRenderError::CompileFailed {
        kind,
        diagnostic_count,
    }
}

fn enforce_time_budget(elapsed: Duration) -> Result<(), EmbeddedRenderError> {
    if elapsed.as_millis() > MAX_RENDER_MILLIS {
        Err(EmbeddedRenderError::TimeBudgetExceeded {
            max_millis: MAX_RENDER_MILLIS,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use canisend_contracts::{
        ActorKind, ArtifactKind, ArtifactReference, DocumentGenerationMetadata, DocumentKind,
        DocumentPlaceholderRecord, DocumentRecord, DocumentSectionKind, DocumentSectionRecord,
        EntityId, ExecutionMode, Revision, Sha256Digest, UtcTimestamp,
    };
    use canisend_resources::{ResourceId, get};

    use super::{
        EmbeddedRenderError, EmbeddedTypstCompiler, MAX_RENDER_PDF_BYTES, MAX_TYPST_SOURCE_BYTES,
        TypstProjectionError, document_template_descriptor, project_document_typst,
        render_acceptance_probe, validate_rendered_pdf,
    };

    #[test]
    fn embedded_template_and_fonts_render_without_external_files() {
        let template = std::str::from_utf8(get(ResourceId::TemplateModernproCoverletter).bytes)
            .expect("embedded Typst template is UTF-8");
        let source = format!(
            r#"{template}
#canisend_render_document((
  title: "Application for Example Role",
  kind: "cover-letter",
  generated-date: "2026-08-01",
  sections: ((heading: none, body: "Evidence-backed application."),),
  fields: (
    (key: "candidate-name", value: "Ada Lovelace"),
    (key: "recipient-institution", value: "University X"),
  ),
))"#
        );
        let rendered = EmbeddedTypstCompiler::new()
            .compile_pdf(&source)
            .expect("embedded renderer");

        assert!(rendered.bytes().starts_with(b"%PDF-"));
        assert!(rendered.bytes().len() < MAX_RENDER_PDF_BYTES);
        assert_eq!(rendered.page_count(), 1);
        assert_eq!(rendered.warning_count(), 0);
        assert!(!rendered.elapsed().is_zero());
        let text = pdf_extract::extract_text_from_mem(rendered.bytes()).expect("extract PDF text");
        assert!(text.contains("Ada Lovelace"));
        assert!(text.contains("Evidence-backed application"));
    }

    #[test]
    fn rendered_pdf_validation_rejects_malformed_inputs() {
        assert_eq!(
            validate_rendered_pdf(b"%PDF-not-a-document"),
            Err(EmbeddedRenderError::InvalidPdf)
        );
        assert_eq!(
            validate_rendered_pdf(b"not-a-pdf"),
            Err(EmbeddedRenderError::InvalidPdf)
        );
    }

    #[test]
    fn cross_platform_probe_covers_unicode_math_urls_lists_and_page_breaks() {
        let rendered = render_acceptance_probe().expect("cross-platform render probe");
        assert_eq!(rendered.page_count(), 2);
        assert_eq!(rendered.warning_count(), 0);
        let text = pdf_extract::extract_text_from_mem(rendered.bytes()).expect("probe PDF text");
        assert!(text.contains("CanISend cross-platform acceptance"));
        assert!(text.contains("résumé"));
        assert!(text.contains("Ελληνικά"));
        assert!(text.contains("Кириллица"));
        assert!(text.contains("example.edu/jobs"));
        assert!(text.contains("Explicit second page"));
    }

    #[test]
    fn missing_user_font_is_a_bounded_warning_with_embedded_fallback() {
        let rendered = EmbeddedTypstCompiler::new()
            .compile_pdf(
                r#"#set text(font: ("CanISend Missing User Font", "Libertinus Serif"))
Missing user font behavior remains deterministic."#,
            )
            .expect("embedded fallback renders without system-font access");
        assert!(rendered.warning_count() > 0);
        assert_eq!(rendered.page_count(), 1);
        let text = pdf_extract::extract_text_from_mem(rendered.bytes()).expect("fallback PDF text");
        assert!(text.contains("Missing user font behavior"));
    }

    #[test]
    fn source_and_file_world_are_fail_closed_and_diagnostics_are_body_free() {
        let oversized = "x".repeat(MAX_TYPST_SOURCE_BYTES + 1);
        assert_eq!(
            EmbeddedTypstCompiler::new().compile_pdf(&oversized),
            Err(EmbeddedRenderError::SourceTooLarge {
                max_bytes: MAX_TYPST_SOURCE_BYTES
            })
        );

        let private_path = "/private/canisend-render-sentinel";
        let error = EmbeddedTypstCompiler::new()
            .compile_pdf(&format!("#read(\"{private_path}\")"))
            .expect_err("restricted world rejects filesystem reads");
        assert!(matches!(error, EmbeddedRenderError::CompileFailed { .. }));
        assert!(!error.to_string().contains(private_path));

        let package = "@preview/canisend-network-sentinel:0.1.0";
        let error = EmbeddedTypstCompiler::new()
            .compile_pdf(&format!("#import \"{package}\": *"))
            .expect_err("restricted world has no package resolver");
        assert!(!error.to_string().contains(package));
    }

    #[test]
    fn every_document_kind_projects_escaped_self_contained_typst() {
        let source_artifact = artifact_reference(ArtifactKind::CoverLetter, 10);
        for kind in DocumentKind::ALL {
            let document = document(kind, true);
            let source = project_document_typst(&source_artifact, &document)
                .expect("self-contained Typst projection");
            assert!(source.contains("Structured artifacts remain authoritative"));
            assert!(source.contains("\\\"/private/canisend-injection-sentinel\\\""));
            let rendered = EmbeddedTypstCompiler::new()
                .compile_pdf(&source)
                .expect("escaped projection compiles");
            assert_eq!(rendered.warning_count(), 0);
            let text = pdf_extract::extract_text_from_mem(rendered.bytes())
                .expect("extract projected PDF text");
            assert!(
                text.to_ascii_lowercase()
                    .contains("canisend injection sentinel")
            );
            assert!(text.contains("#read"));
        }
    }

    #[test]
    fn document_kinds_select_the_pinned_modernpro_bundle_entrypoints() {
        for kind in DocumentKind::ALL {
            let descriptor = document_template_descriptor(kind);
            assert_eq!(descriptor.entrypoint, "canisend_render_document");
            let (expected_bundle, expected_resource, expected_version) = match kind {
                DocumentKind::Cv => ("modernpro-cv", "template.modernpro-cv", "2.0.0"),
                DocumentKind::CoverLetter
                | DocumentKind::ResearchStatement
                | DocumentKind::TeachingStatement => (
                    "modernpro-coverletter",
                    "template.modernpro-coverletter",
                    "1.0.0",
                ),
            };
            assert_eq!(descriptor.bundle_id, expected_bundle);
            assert_eq!(descriptor.resource.id, expected_resource);
            assert_eq!(descriptor.resource.version, expected_version);

            let source = project_document_typst(
                &artifact_reference(ArtifactKind::CoverLetter, 10),
                &document(kind, true),
            )
            .expect("template-routed projection");
            assert!(source.contains(&format!("resource:{expected_resource}")));
            assert!(source.contains(&format!(
                "version:{} sha256:{}",
                descriptor.resource.version, descriptor.resource.sha256
            )));
            assert!(source.contains("generated-date: \"2026-07-18\""));
            assert!(!source.contains("#import \"@preview/"));
        }
    }

    #[test]
    fn active_statement_template_renders_every_section_kind_without_warnings() {
        const SECTION_KINDS: [DocumentSectionKind; 11] = [
            DocumentSectionKind::Opening,
            DocumentSectionKind::Fit,
            DocumentSectionKind::Research,
            DocumentSectionKind::Teaching,
            DocumentSectionKind::Service,
            DocumentSectionKind::Experience,
            DocumentSectionKind::Education,
            DocumentSectionKind::Publications,
            DocumentSectionKind::Skills,
            DocumentSectionKind::Closing,
            DocumentSectionKind::Other,
        ];

        let mut document = document(DocumentKind::ResearchStatement, true);
        document.title = "Latest template contract — résumé".to_owned();
        document.sections = SECTION_KINDS
            .into_iter()
            .enumerate()
            .map(|(index, kind)| DocumentSectionRecord {
                id: entity(100 + index as u64),
                kind,
                heading: Some(format!("Section {}", index + 1)),
                body: format!(
                    "Evidence {} covers résumé, Ελληνικά, Кириллица, symbols § ±, and https://example.edu/jobs/{}.",
                    index + 1,
                    index + 1
                ),
                claims: Vec::new(),
                revision: Revision::try_new(1).expect("revision"),
            })
            .collect();
        let source = project_document_typst(
            &artifact_reference(ArtifactKind::ResearchStatement, 20),
            &document,
        )
        .expect("latest application template projection");
        let rendered = EmbeddedTypstCompiler::new()
            .compile_pdf(&source)
            .expect("latest application template render");

        assert_eq!(rendered.warning_count(), 0);
        assert!(rendered.page_count() >= 1);
        let text = pdf_extract::extract_text_from_mem(rendered.bytes())
            .expect("extract latest-template contract text");
        assert!(text.contains("Latest template contract"));
        assert!(text.contains("résumé"));
        assert!(text.contains("Ελληνικά"));
        assert!(text.contains("Кириллица"));
        assert!(text.contains("https://example.edu/jobs/11"));
    }

    #[test]
    fn modernpro_projection_is_byte_stable_for_identical_structured_input() {
        let source_artifact = artifact_reference(ArtifactKind::CoverLetter, 40);
        for kind in DocumentKind::ALL {
            let source = project_document_typst(&source_artifact, &document(kind, true))
                .expect("deterministic ModernPro projection");
            let first = EmbeddedTypstCompiler::new()
                .compile_pdf(&source)
                .expect("first deterministic render");
            let second = EmbeddedTypstCompiler::new()
                .compile_pdf(&source)
                .expect("second deterministic render");
            assert_eq!(first.bytes(), second.bytes(), "render drift for {kind:?}");
        }
    }

    #[test]
    fn unresolved_template_fields_fail_before_source_generation() {
        let error = project_document_typst(
            &artifact_reference(ArtifactKind::CoverLetter, 10),
            &document(DocumentKind::CoverLetter, false),
        )
        .expect_err("unresolved field");
        assert_eq!(
            error,
            TypstProjectionError::UnresolvedTemplateFields { count: 1 }
        );
    }

    fn document(kind: DocumentKind, resolved: bool) -> DocumentRecord {
        DocumentRecord {
            id: entity(1),
            job_id: entity(2),
            plan_artifact: artifact_reference(ArtifactKind::ApplicationPlan, 3),
            planned_document: canisend_contracts::PlannedDocumentRevisionReference {
                id: entity(4),
                revision: Revision::try_new(1).expect("revision"),
            },
            kind,
            title: "CanISend injection sentinel".to_owned(),
            sections: vec![DocumentSectionRecord {
                id: entity(5),
                kind: DocumentSectionKind::Other,
                heading: Some("Quoted heading".to_owned()),
                body: "Literal #read(\"/private/canisend-injection-sentinel\") \\\\ [safe] résumé"
                    .to_owned(),
                claims: Vec::new(),
                revision: Revision::try_new(1).expect("revision"),
            }],
            placeholders: vec![DocumentPlaceholderRecord {
                id: entity(6),
                key: "addressee".to_owned(),
                instruction: "Confirm the addressee".to_owned(),
                required: true,
                resolution: resolved.then(|| "Selection Committee".to_owned()),
                revision: Revision::try_new(1).expect("revision"),
            }],
            generation: DocumentGenerationMetadata {
                actor: ActorKind::HostAgent,
                execution_mode: ExecutionMode::HostAgent,
                task_id: entity(7),
                prompt_resource_id: "prompt.document-draft".to_owned(),
                created_at: UtcTimestamp::try_new("2026-07-18T04:00:00Z").expect("timestamp"),
            },
            revision: Revision::try_new(1).expect("revision"),
        }
    }

    fn artifact_reference(kind: ArtifactKind, suffix: u64) -> ArtifactReference {
        ArtifactReference {
            kind,
            id: entity(suffix),
            revision: Revision::try_new(1).expect("revision"),
            sha256: Sha256Digest::try_new(format!("{suffix:064x}")).expect("digest"),
        }
    }

    fn entity(suffix: u64) -> EntityId {
        EntityId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012x}")).expect("entity ID")
    }
}
