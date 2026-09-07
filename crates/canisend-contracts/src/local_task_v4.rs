use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{AgentApplicationBindingV4, EntityId, Sha256Digest, UtcTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LocalTaskStateV4 {
    Prepared,
    Claimed,
    Submitted,
    Cancelled,
}

/// Body-free local coordination metadata. A lease never authorizes a business mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalTaskV4 {
    pub id: EntityId,
    pub application: AgentApplicationBindingV4,
    pub input_sha256: Sha256Digest,
    pub generation: u64,
    pub state: LocalTaskStateV4,
    pub lease_id: Option<EntityId>,
    pub lease_expires_at: Option<UtcTimestamp>,
    pub candidate_sha256: Option<Sha256Digest>,
    pub candidate_bytes: Option<u64>,
}
