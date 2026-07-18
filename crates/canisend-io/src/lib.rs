#![forbid(unsafe_code)]

mod candidate;
mod discovery;
mod local;
mod pdf;
mod remote;
mod render;

use std::path::PathBuf;

use thiserror::Error;

pub use local::{
    LocalTextDocument, LocalTextKind, MAX_LOCAL_SOURCE_BYTES, normalize_utf8_text, read_local_text,
};
pub use pdf::{MAX_PDF_PAGES, PdfTextDocument, extract_pdf_text, read_local_pdf};
pub use remote::{
    HttpFetcher, MAX_REMOTE_SOURCE_BYTES, RemoteDocument, RemoteDocumentKind, RemotePayload,
    RemotePayloadKind,
};
pub use render::{
    EmbeddedRenderError, EmbeddedTypstCompiler, MAX_RENDER_MILLIS, MAX_RENDER_PDF_BYTES,
    MAX_TYPST_SOURCE_BYTES, RenderedPdf, TypstProjectionError, project_document_typst,
    render_acceptance_probe, validate_rendered_pdf,
};

#[derive(Debug, Error)]
pub enum IoAdapterError {
    #[error("I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unsafe local input path: {0}")]
    UnsafeLocalFile(PathBuf),
    #[error("unsupported local input type: {0}")]
    UnsupportedLocalType(PathBuf),
    #[error("input exceeds the configured {limit}-byte limit")]
    InputTooLarge { limit: u64 },
    #[error("text input must be valid UTF-8, optionally with a UTF-8 BOM")]
    InvalidTextEncoding,
    #[error("text input contains a forbidden control character")]
    UnsafeTextControlCharacter,
    #[error("no usable text was available in the input")]
    TextUnavailable,
    #[error("URL is invalid: {0}")]
    InvalidUrl(String),
    #[error("URL is forbidden by the network policy: {0}")]
    UrlPolicy(String),
    #[error("DNS resolution failed for {0}")]
    DnsResolution(String),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("HTTP response body read failed: {0}")]
    ResponseRead(#[source] std::io::Error),
    #[error("HTTP response returned unsupported status {0}")]
    HttpStatus(u16),
    #[error("HTTP redirect is invalid: {0}")]
    InvalidRedirect(String),
    #[error("HTTP response content type is unsupported or misleading: {0}")]
    UnsupportedContentType(String),
    #[error("HTML parsing failed: {0}")]
    Html(String),
    #[error("PDF is encrypted and cannot be imported without a password")]
    PdfEncrypted,
    #[error("PDF is malformed or extraction failed: {0}")]
    PdfMalformed(String),
    #[error(
        "pdf_text_unavailable: PDF contains no extractable text; provide a text-based PDF, Markdown, or plain text"
    )]
    PdfTextUnavailable,
    #[error("PDF has {actual} pages, exceeding the configured {limit}-page limit")]
    PdfPageLimit { limit: usize, actual: usize },
    #[error("PDF extraction exceeded the configured time budget")]
    PdfTimeBudget,
    #[error("discovery input is invalid: {0}")]
    DiscoveryInput(String),
    #[error("task completion input is invalid: {0}")]
    CandidateInput(String),
}
pub use candidate::{
    MAX_CRITERIA_BYTES, MAX_TASK_COMPLETION_BYTES, read_criteria_file, read_task_completion_file,
    read_task_completion_stdin,
};
pub use discovery::{
    DiscoveryAdapter, DiscoveryFile, DiscoveryFileKind, GreenhouseAdapter, JobsAcUkAdapter,
    LeverAdapter, MAX_DISCOVERY_BATCH_BYTES, MAX_DISCOVERY_LEADS, RssAtomAdapter,
    discovery_adapter_capabilities, parse_csv_batch, parse_host_agent_batch, parse_json_batch,
    read_discovery_file,
};
