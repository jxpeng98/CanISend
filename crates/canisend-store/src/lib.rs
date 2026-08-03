#![forbid(unsafe_code)]

mod application_flow_v3;
mod application_projection_v3;
mod application_v3;
mod artifact;
mod backup;
mod blob;
mod catalog;
mod compatibility_v3;
mod context;
mod criteria;
mod database;
mod discovery;
mod document;
mod evidence;
mod job;
mod matching;
mod migration_v3;
mod pack_migration_v3;
mod package;
mod plan;
mod profile;
mod projection;
mod render;
mod review;
mod task;
mod workflow;
mod workspace;

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub use application_flow_v3::{
    APPLICATION_FLOW_EXPORT_FORMAT_V3, ApplicationFlowApproveRequestV3,
    ApplicationFlowCommitReadModelV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowCreateRequestV3, ApplicationFlowDeliverableDraftV3,
    ApplicationFlowExportManifestV3, ApplicationFlowExportReadModelV3,
    ApplicationFlowPlanRequestV3, ApplicationFlowPlannedDeliverableV3, ApplicationFlowReadModelV3,
    ApplicationFlowRenderedDeliverableV3, ApplicationFlowRequirementDraftV3,
    ApplicationFlowReviewDeliverableV3, ApplicationFlowReviewReadModelV3, ApplicationFlowServiceV3,
    ApplicationFlowStageReadModelV3, ApplicationFlowStageStateV3,
    MAX_APPLICATION_FLOW_SOURCE_BYTES_V3,
};
pub use application_projection_v3::{
    APPLICATION_PROJECTION_FORMAT_V3, ApplicationLegacyProjectionV3,
    ApplicationProjectionCatalogV3, ApplicationProjectionKindV3, ApplicationProjectionReconcileV3,
    ApplicationProjectionRecordV3, ApplicationProjectionService,
};
pub use application_v3::{
    ApplicationModelCommitResultV3, ApplicationModelRepository, ApplicationModelRevisionV3,
    StoredApplicationModelV3, WORKSPACE_V3_FORMAT, WorkspaceV3AuthorityState,
};
pub use artifact::ArtifactService;
pub use backup::{BackupResult, verify_backup};
pub use blob::{BlobAudit, BlobStore, DEFAULT_MAX_BLOB_BYTES};
pub use catalog::{
    CatalogArtifactMetadata, CatalogSourceMetadata, CatalogSourceRole, CatalogSourceScope,
    CatalogSubjectJob, ContentCatalogService, MAX_CONTENT_CATALOG_ENTRIES,
};
pub use compatibility_v3::{
    LegacyApplicationBindingV3, LegacyCompatibilityAuthority, LegacyCompatibilityContextV3,
    LegacyCompatibilityService,
};
pub use context::AgentContextService;
pub use criteria::CriteriaService;
pub use database::{DATABASE_SCHEMA_VERSION, Database};
pub use discovery::DiscoveryService;
pub use document::DocumentService;
pub use evidence::EvidenceService;
pub use job::{JobService, NewSource};
pub use matching::MatchService;
pub use migration_v3::{
    ACADEMIC_JOB_PACK_ID, LEGACY_WORKSPACE_SCHEMA_VERSION, WORKSPACE_V3_MIGRATION_PREVIEW_FORMAT,
    WORKSPACE_V3_MIGRATION_RESULT_FORMAT, WorkspaceV3MigrationPreview, WorkspaceV3MigrationResult,
    WorkspaceV3MigrationService,
};
pub use pack_migration_v3::{
    APPLICATION_PACK_MIGRATION_FORMAT_V3, ApplicationPackMigrationImpactV3,
    ApplicationPackMigrationPreviewV3, ApplicationPackMigrationResultV3,
    ApplicationPackMigrationService,
};
pub use package::PackageService;
pub use plan::PlanService;
pub use profile::{NewProfileSource, ProfileService};
pub use projection::ProjectionService;
pub use render::RenderService;
pub use review::ReviewService;
pub use task::TaskService;
pub use workflow::WorkflowService;
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
    #[error(
        "workspace database schema {found} is newer than supported {supported}; upgrade CanISend or restore a verified pre-upgrade backup to a new path"
    )]
    WorkspaceVersionUnsupported { found: u32, supported: u32 },
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
    #[error("Workspace v3 application authority is not active")]
    ApplicationModelUnavailable,
    #[error("application model was not found: {0}")]
    ApplicationModelNotFound(String),
    #[error("application model operation conflicts with current state: {0}")]
    ApplicationModelConflict(String),
    #[error("application model integrity verification failed: {0}")]
    ApplicationModelIntegrity(String),
    #[error("Workspace v2 to v3 migration conflicts with current state: {0}")]
    WorkspaceMigrationConflict(String),
    #[error("Workspace v2 to v3 migration integrity verification failed: {0}")]
    WorkspaceMigrationIntegrity(String),
    #[error("profile source was not found: {0}")]
    ProfileSourceNotFound(String),
    #[error("discovery source was not found: {0}")]
    DiscoverySourceNotFound(String),
    #[error("discovery lead was not found: {0}")]
    DiscoveryLeadNotFound(String),
    #[error("discovery operation conflicts with current state: {0}")]
    DiscoveryConflict(String),
    #[error("task was not found: {0}")]
    TaskNotFound(String),
    #[error("task inputs or lease are stale: {0}")]
    TaskStale(String),
    #[error("task operation conflicts with current state: {0}")]
    TaskConflict(String),
    #[error("candidate does not satisfy its JSON Schema")]
    CandidateStructural(Vec<canisend_contracts::ContractViolation>),
    #[error("candidate violates semantic contract rules")]
    CandidateSemantic(Vec<canisend_contracts::ContractViolation>),
    #[error("workflow was not found for job: {0}")]
    WorkflowNotFound(String),
    #[error("workflow operation conflicts with current state: {0}")]
    WorkflowConflict(String),
    #[error("input is invalid: {0}")]
    InvalidInput(String),
    #[error("artifact dependency is not current: {0}")]
    DependencyConflict(String),
    #[error("projection path is outside the managed application, job, or profile tree")]
    ProjectionPathRejected,
    #[error("managed projection contains user edits: {0}")]
    ProjectionEdited(String),
    #[error("projection destination is an unmanaged existing file: {0}")]
    ProjectionUnmanagedConflict(String),
    #[error("managed projection was not found: {0}")]
    ProjectionNotFound(String),
    #[error("document contains {count} unresolved template field(s)")]
    TemplateFieldsUnresolved { count: usize },
    #[error("embedded Typst projection invariant failed")]
    TypstProjectionInvariant,
    #[error("embedded render failed: {0}")]
    EmbeddedRender(#[from] canisend_io::EmbeddedRenderError),
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

pub fn current_utc_timestamp() -> Result<UtcTimestamp, StoreError> {
    now_utc()
}
