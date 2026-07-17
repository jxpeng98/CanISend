#![forbid(unsafe_code)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IoAdapterError {
    #[error("the requested Rust-native I/O adapter is not available yet")]
    NotImplemented,
}
