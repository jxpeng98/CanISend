use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{EntityId, Revision, UtcTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoverySourceKind {
    Csv,
    Json,
    HostAgent,
    RssAtom,
    JobsAcUk,
    Greenhouse,
    Lever,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveryLeadStatus {
    Active,
    Removed,
    Expired,
    Promoted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveryFreshness {
    Current,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DiscoveryMetadataValue {
    Text(String),
    Integer(i64),
    Boolean(bool),
    Json(serde_json::Value),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryAdapterCapabilities {
    pub kind: DiscoverySourceKind,
    pub network: bool,
    pub supports_cursor: bool,
    pub preserves_removed: bool,
    pub max_items_per_refresh: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryRefreshPolicy {
    pub max_items: u32,
    pub stale_after_seconds: u64,
    pub mark_missing_as_removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryLeadCandidate {
    pub external_id: Option<String>,
    pub title: String,
    pub organization: String,
    pub location: Option<String>,
    pub deadline: Option<String>,
    pub url: String,
    pub summary: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, DiscoveryMetadataValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryBatch {
    pub source_kind: DiscoverySourceKind,
    pub source_name: String,
    pub source_url: Option<String>,
    pub cursor: Option<String>,
    pub observed_at: UtcTimestamp,
    pub leads: Vec<DiscoveryLeadCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoverySourceRecord {
    pub id: EntityId,
    pub kind: DiscoverySourceKind,
    pub name: String,
    pub endpoint: Option<String>,
    pub enabled: bool,
    pub policy: DiscoveryRefreshPolicy,
    pub cursor: Option<String>,
    pub last_refreshed_at: Option<UtcTimestamp>,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryLeadRecord {
    pub id: EntityId,
    pub source_id: EntityId,
    pub external_id: Option<String>,
    pub canonical_key: String,
    pub title: String,
    pub organization: String,
    pub location: Option<String>,
    pub deadline: Option<String>,
    pub url: String,
    pub summary: Option<String>,
    pub metadata: BTreeMap<String, DiscoveryMetadataValue>,
    pub status: DiscoveryLeadStatus,
    pub freshness: DiscoveryFreshness,
    pub first_seen_at: UtcTimestamp,
    pub last_seen_at: UtcTimestamp,
    pub revision: Revision,
    pub promoted_job_id: Option<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryRefreshReceipt {
    pub id: EntityId,
    pub source_id: EntityId,
    pub observed: u64,
    pub inserted: u64,
    pub updated: u64,
    pub unchanged: u64,
    pub removed: u64,
    pub rejected: u64,
    pub cursor: Option<String>,
    pub started_at: UtcTimestamp,
    pub completed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryImportDiagnostic {
    pub row: u64,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryImportReport {
    pub dry_run: bool,
    pub accepted: u64,
    pub rejected: u64,
    pub diagnostics: Vec<DiscoveryImportDiagnostic>,
    pub batch: Option<DiscoveryBatch>,
    pub receipt: Option<DiscoveryRefreshReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryLeadSuggestion {
    pub lead: DiscoveryLeadRecord,
    pub similarity_percent: u8,
}
