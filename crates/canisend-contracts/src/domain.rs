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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileSourceKind {
    Markdown,
    PlainText,
    Json,
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
    pub archived: bool,
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
    pub final_url: Option<String>,
    pub content_type: String,
    pub redirect_chain: Vec<String>,
    pub retrieved_at: UtcTimestamp,
    pub privacy: PrivacyClassification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileSourceRecord {
    pub id: EntityId,
    pub kind: ProfileSourceKind,
    pub original: ArtifactReference,
    pub normalized_text: ArtifactReference,
    pub content_type: String,
    pub sensitivity: PrivacyClassification,
    pub created_at: UtcTimestamp,
    pub revision: Revision,
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
    pub source_quote: String,
    pub source_span: SourceTextSpan,
    pub confirmed: bool,
    pub excluded: bool,
    pub sensitivity: PrivacyClassification,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceProposalRecord {
    pub kind: EvidenceKind,
    pub summary: String,
    pub source_quote: String,
    pub source_span: SourceTextSpan,
    pub sensitivity: PrivacyClassification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceProposalSet {
    pub profile_revision: Revision,
    pub proposals: Vec<EvidenceProposalRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCatalogRecord {
    pub id: EntityId,
    pub profile_revision: Revision,
    pub items: Vec<EvidenceRecord>,
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
    pub source_span: SourceTextSpan,
    pub confidence_milli: u16,
    pub confirmed: bool,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceTextSpan {
    pub source: ArtifactReference,
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParsedJobRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub title: String,
    pub institution: String,
    pub summary: String,
    pub responsibilities: Vec<String>,
    pub criteria: Vec<CriterionRecord>,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriteriaSetRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub criteria: Vec<CriterionRecord>,
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
pub struct CriterionRevisionReference {
    pub id: EntityId,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRevisionReference {
    pub id: EntityId,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMatchRecord {
    pub id: EntityId,
    pub criterion: CriterionRevisionReference,
    pub evidence: Vec<EvidenceRevisionReference>,
    pub strength: MatchStrength,
    pub rationale: String,
    pub gap: Option<String>,
    pub prohibited_claims: Vec<String>,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMatchProposalRecord {
    pub criterion: CriterionRevisionReference,
    pub evidence: Vec<EvidenceRevisionReference>,
    pub strength: MatchStrength,
    pub rationale: String,
    pub gap: Option<String>,
    pub prohibited_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMatchProposalSet {
    pub job_id: EntityId,
    pub criteria_artifact: ArtifactReference,
    pub evidence_artifact: ArtifactReference,
    pub proposals: Vec<EvidenceMatchProposalRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMatchSetRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub criteria_artifact: ArtifactReference,
    pub evidence_artifact: ArtifactReference,
    pub matches: Vec<EvidenceMatchRecord>,
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
