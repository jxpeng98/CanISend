use std::time::{Duration, Instant};

use thiserror::Error;
use typst_as_lib::{TypstAsLibError, TypstEngine, typst_kit_options::TypstKitFontOptions};

pub const MAX_TYPST_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_RENDER_PDF_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RENDER_MILLIS: u128 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPdf {
    bytes: Vec<u8>,
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
            .search_fonts_with(
                TypstKitFontOptions::default()
                    .include_system_fonts(false)
                    .include_embedded_fonts(true),
            )
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
        if !bytes.starts_with(b"%PDF-") {
            return Err(EmbeddedRenderError::InvalidPdf);
        }
        Ok(RenderedPdf {
            bytes,
            warning_count,
            elapsed,
        })
    }
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
    use canisend_resources::{ResourceId, get};

    use super::{
        EmbeddedRenderError, EmbeddedTypstCompiler, MAX_RENDER_PDF_BYTES, MAX_TYPST_SOURCE_BYTES,
    };

    #[test]
    fn embedded_template_and_fonts_render_without_external_files() {
        let template = std::str::from_utf8(get(ResourceId::TemplateCoverLetter).bytes)
            .expect("embedded Typst template is UTF-8");
        let source = format!(
            "{template}\n#application_cover_letter([Ada Lovelace], [University X], [Evidence-backed application.])"
        );
        let rendered = EmbeddedTypstCompiler::new()
            .compile_pdf(&source)
            .expect("embedded renderer");

        assert!(rendered.bytes().starts_with(b"%PDF-"));
        assert!(rendered.bytes().len() < MAX_RENDER_PDF_BYTES);
        assert_eq!(rendered.warning_count(), 0);
        assert!(!rendered.elapsed().is_zero());
        let text = pdf_extract::extract_text_from_mem(rendered.bytes()).expect("extract PDF text");
        assert!(text.contains("Ada Lovelace"));
        assert!(text.contains("Evidence-backed application"));
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
}
