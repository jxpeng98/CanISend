#![forbid(unsafe_code)]

mod local;

use std::path::PathBuf;

use thiserror::Error;

pub use local::{
    LocalTextDocument, LocalTextKind, MAX_LOCAL_SOURCE_BYTES, normalize_utf8_text, read_local_text,
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
}
