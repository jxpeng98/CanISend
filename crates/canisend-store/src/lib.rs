#![forbid(unsafe_code)]

mod artifact;
mod backup;
mod blob;
mod database;
mod job;
mod workspace;

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub use artifact::ArtifactService;
pub use backup::{BackupResult, verify_backup};
pub use blob::{BlobAudit, BlobStore, DEFAULT_MAX_BLOB_BYTES};
pub use database::{DATABASE_SCHEMA_VERSION, Database};
pub use job::{JobService, NewSource};
pub use workspace::{Workspace, WorkspaceConfig, WorkspacePaths};

use canisend_contracts::{EntityId, PrimitiveError, UtcTimestamp};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub const STORAGE_ARCHITECTURE: &str = "sqlite-plus-content-addressed-blobs";
pub const BACKUP_FORMAT: &str = "canisend.backup/v2";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("workspace was not found from {0}")]
    WorkspaceNotFound(PathBuf),
    #[error("workspace already exists at {0}")]
    WorkspaceExists(PathBuf),
    #[error("unsafe workspace path: {0}")]
    UnsafePath(PathBuf),
    #[error("expected a directory at {0}")]
    NotDirectory(PathBuf),
    #[error("I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("workspace configuration is invalid: {0}")]
    ConfigDecode(#[from] toml::de::Error),
    #[error("workspace configuration could not be encoded: {0}")]
    ConfigEncode(#[from] toml::ser::Error),
    #[error("JSON operation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("contract value is invalid: {0}")]
    Contract(#[from] PrimitiveError),
    #[error("secure random generation failed: {0}")]
    Random(String),
    #[error("system clock is before the Unix epoch")]
    Clock,
    #[error("blob exceeds the configured {limit}-byte limit")]
    BlobTooLarge { limit: u64 },
    #[error("blob is missing: {0}")]
    BlobMissing(String),
    #[error("blob digest verification failed: expected {expected}, found {actual}")]
    BlobDigestMismatch { expected: String, actual: String },
    #[error("immutable blob collision at {0}")]
    BlobCollision(PathBuf),
    #[error("artifact was not found: {0}")]
    ArtifactNotFound(String),
    #[error("job was not found: {0}")]
    JobNotFound(String),
    #[error("job is archived: {0}")]
    JobArchived(String),
    #[error("input is invalid: {0}")]
    InvalidInput(String),
    #[error("artifact dependency is not current: {0}")]
    DependencyConflict(String),
    #[error("projection path must be inside jobs/ or profile/")]
    ProjectionPathRejected,
    #[error("backup is invalid: {0}")]
    BackupInvalid(String),
    #[error("workspace invariant failed: {0}")]
    Invariant(String),
}

pub(crate) fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.into(),
        source,
    }
}

pub(crate) fn generate_id() -> Result<EntityId, StoreError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StoreError::Clock)?
        .as_millis();
    let timestamp = u64::try_from(milliseconds).map_err(|_| StoreError::Clock)?;
    let mut bytes = [0_u8; 16];
    bytes[..6].copy_from_slice(&timestamp.to_be_bytes()[2..]);
    getrandom::fill(&mut bytes[6..]).map_err(|error| StoreError::Random(error.to_string()))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let value = format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    );
    EntityId::try_new(value).map_err(StoreError::from)
}

pub(crate) fn now_utc() -> Result<UtcTimestamp, StoreError> {
    let value = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| StoreError::Invariant(error.to_string()))?;
    UtcTimestamp::try_new(value).map_err(StoreError::from)
}
