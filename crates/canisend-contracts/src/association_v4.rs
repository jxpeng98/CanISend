use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ApplicationId, ConsentScope, ContentRevisionReferenceV3, EntityId, PrivacyClassification,
    Revision, Sha256Digest, UtcTimestamp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceSourceKindV4 {
    PastedText,
    LocalFile,
    TextPdf,
    Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSourceRevisionV4 {
    pub id: EntityId,
    pub revision: Revision,
    pub kind: WorkspaceSourceKindV4,
    pub locator: String,
    pub content_type: String,
    pub original_sha256: Sha256Digest,
    pub normalized_sha256: Sha256Digest,
    pub original_bytes: u64,
    pub normalized_text_bytes: u64,
    pub privacy: PrivacyClassification,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationSourceAssociationV4 {
    pub application_id: ApplicationId,
    pub source: ContentRevisionReferenceV3,
    pub consent_scope: Option<ConsentScope>,
    pub associated_at: UtcTimestamp,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationProfileAssociationV4 {
    pub application_id: ApplicationId,
    pub profile_source: ContentRevisionReferenceV3,
    pub consent_scope: Option<ConsentScope>,
    pub associated_at: UtcTimestamp,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationEvidenceAssociationV4 {
    pub application_id: ApplicationId,
    pub evidence: ContentRevisionReferenceV3,
    pub consent_scope: Option<ConsentScope>,
    pub associated_at: UtcTimestamp,
    pub stale: bool,
}
