use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ActorKind, ArtifactKind, EntityId, ExecutionMode, PrivacyClassification, Revision,
    Sha256Digest, UtcTimestamp,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReference {
    pub kind: ArtifactKind,
    pub id: EntityId,
    pub revision: Revision,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    LocalFile,
    UserUrl,
    DiscoveryLead,
    ManualText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobRecord {
    pub id: EntityId,
    pub title: String,
    pub institution: String,
    pub source_ids: Vec<EntityId>,
    pub created_at: UtcTimestamp,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub kind: SourceKind,
    pub original: ArtifactReference,
    pub normalized_text: Option<ArtifactReference>,
    pub source_url: Option<String>,
    pub retrieved_at: UtcTimestamp,
    pub privacy: PrivacyClassification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    Qualification,
    Teaching,
    Research,
    Communication,
    Leadership,
    Service,
    Employment,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub id: EntityId,
    pub kind: EvidenceKind,
    pub summary: String,
    pub source: ArtifactReference,
    pub confirmed: bool,
    pub privacy: PrivacyClassification,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CriterionImportance {
    Essential,
    Desirable,
    Informational,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriterionRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub kind: EvidenceKind,
    pub requirement: String,
    pub importance: CriterionImportance,
    pub source_quote: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MatchStrength {
    Strong,
    Partial,
    Gap,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMatchRecord {
    pub id: EntityId,
    pub criterion_id: EntityId,
    pub evidence_ids: Vec<EntityId>,
    pub strength: MatchStrength,
    pub rationale: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationDecision {
    Apply,
    DoNotApply,
    Undecided,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPlanRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub decision: ApplicationDecision,
    pub strategy: String,
    pub document_ids: Vec<EntityId>,
    pub decided_by: ActorKind,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentKind {
    CoverLetter,
    ResearchStatement,
    TeachingStatement,
    Cv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentStatus {
    Planned,
    AwaitingInput,
    Draft,
    Reviewed,
    Current,
    Stale,
    Omitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub kind: DocumentKind,
    pub title: String,
    pub executor: Option<ExecutionMode>,
    pub status: DocumentStatus,
    pub artifact: Option<ArtifactReference>,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FindingSeverity {
    Info,
    Warning,
    Blocker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FindingStatus {
    Open,
    Accepted,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindingRecord {
    pub id: EntityId,
    pub code: String,
    pub severity: FindingSeverity,
    pub message: String,
    pub related_ids: Vec<EntityId>,
    pub status: FindingStatus,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessState {
    Blocked,
    NeedsReview,
    ReadyToExport,
    Exported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadinessRecord {
    pub job_id: EntityId,
    pub state: ReadinessState,
    pub blocker_finding_ids: Vec<EntityId>,
    pub checked_at: UtcTimestamp,
}
