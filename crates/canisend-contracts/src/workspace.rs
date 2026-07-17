use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{EntityId, Revision, Sha256Digest, UtcTimestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceStatusData {
    pub workspace_id: EntityId,
    pub workspace_format: String,
    pub created_at: UtcTimestamp,
    pub database_schema_version: u32,
    pub sqlite_version: String,
    pub journal_mode: String,
    pub job_count: u64,
    pub artifact_count: u64,
    pub referenced_blob_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CheckSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCheckIssue {
    pub code: String,
    pub severity: CheckSeverity,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCheckData {
    pub workspace_id: EntityId,
    pub ok: bool,
    pub database_integrity: String,
    pub checked_referenced_blobs: u64,
    pub unreferenced_blobs: Vec<Sha256Digest>,
    pub stale_artifact_ids: Vec<EntityId>,
    pub projection_repairs_required: Vec<EntityId>,
    pub issues: Vec<WorkspaceCheckIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupBlobEntry {
    pub sha256: Sha256Digest,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupManifestData {
    pub format: String,
    pub workspace_id: EntityId,
    pub created_at: UtcTimestamp,
    pub database_sha256: Sha256Digest,
    pub configuration_sha256: Sha256Digest,
    pub blobs: Vec<BackupBlobEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRevisionData {
    pub artifact_id: EntityId,
    pub revision: Revision,
    pub sha256: Sha256Digest,
    pub created_at: UtcTimestamp,
    pub stale: bool,
}
