#![forbid(unsafe_code)]

use thiserror::Error;

pub const STORAGE_ARCHITECTURE: &str = "sqlite-plus-content-addressed-blobs";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("the Rust-native storage implementation is not available yet")]
    NotImplemented,
}
