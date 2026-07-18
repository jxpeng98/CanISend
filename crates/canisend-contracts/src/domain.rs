use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ActorKind, ArtifactKind, EntityId, ExecutionMode, PrivacyClassification, Revision,
    SafeRelativePath, Sha256Digest, UtcTimestamp,
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
    Hold,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationStrategyRecord {
    pub positioning: String,
    pub priorities: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentKind {
    CoverLetter,
    ResearchStatement,
    TeachingStatement,
    Cv,
}

impl DocumentKind {
    pub const ALL: [Self; 4] = [
        Self::CoverLetter,
        Self::ResearchStatement,
        Self::TeachingStatement,
        Self::Cv,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentRequirement {
    Required,
    Optional,
    Omitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentPlanCandidateRecord {
    pub kind: DocumentKind,
    pub requirement: DocumentRequirement,
    pub rationale: String,
    pub constraints: Vec<String>,
    pub executor: Option<ExecutionMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedDocumentRecord {
    pub id: EntityId,
    pub kind: DocumentKind,
    pub requirement: DocumentRequirement,
    pub rationale: String,
    pub constraints: Vec<String>,
    pub executor: Option<ExecutionMode>,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlanBlockerSeverity {
    Blocking,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanBlockerRecord {
    pub code: String,
    pub criterion: CriterionRevisionReference,
    pub severity: PlanBlockerSeverity,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPlanCandidate {
    pub job_id: EntityId,
    pub matches_artifact: ArtifactReference,
    pub decision: ApplicationDecision,
    pub strategy: ApplicationStrategyRecord,
    pub documents: Vec<DocumentPlanCandidateRecord>,
    pub blockers: Vec<PlanBlockerRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPlanRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub matches_artifact: ArtifactReference,
    pub decision: ApplicationDecision,
    pub strategy: ApplicationStrategyRecord,
    pub documents: Vec<PlannedDocumentRecord>,
    pub blockers: Vec<PlanBlockerRecord>,
    pub decided_by: ActorKind,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentSectionKind {
    Opening,
    Fit,
    Research,
    Teaching,
    Service,
    Experience,
    Education,
    Publications,
    Skills,
    Closing,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimClassification {
    ApplicantFact,
    JobRequirement,
    UserIntent,
    NonFactual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CitationTarget {
    Evidence {
        evidence: EvidenceRevisionReference,
    },
    Criterion {
        criterion: CriterionRevisionReference,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentCitationRecord {
    pub target: CitationTarget,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentClaimCandidateRecord {
    pub text: String,
    pub classification: ClaimClassification,
    pub citations: Vec<DocumentCitationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentClaimRecord {
    pub id: EntityId,
    pub text: String,
    pub classification: ClaimClassification,
    pub citations: Vec<DocumentCitationRecord>,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentSectionCandidateRecord {
    pub kind: DocumentSectionKind,
    pub heading: Option<String>,
    pub body: String,
    pub claims: Vec<DocumentClaimCandidateRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentSectionRecord {
    pub id: EntityId,
    pub kind: DocumentSectionKind,
    pub heading: Option<String>,
    pub body: String,
    pub claims: Vec<DocumentClaimRecord>,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentPlaceholderCandidateRecord {
    pub key: String,
    pub instruction: String,
    pub required: bool,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentPlaceholderRecord {
    pub id: EntityId,
    pub key: String,
    pub instruction: String,
    pub required: bool,
    pub resolution: Option<String>,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedDocumentRevisionReference {
    pub id: EntityId,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentGenerationMetadata {
    pub actor: ActorKind,
    pub execution_mode: ExecutionMode,
    pub task_id: EntityId,
    pub prompt_resource_id: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentCandidate {
    pub job_id: EntityId,
    pub plan_artifact: ArtifactReference,
    pub planned_document: PlannedDocumentRevisionReference,
    pub kind: DocumentKind,
    pub title: String,
    pub sections: Vec<DocumentSectionCandidateRecord>,
    pub placeholders: Vec<DocumentPlaceholderCandidateRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub plan_artifact: ArtifactReference,
    pub planned_document: PlannedDocumentRevisionReference,
    pub kind: DocumentKind,
    pub title: String,
    pub sections: Vec<DocumentSectionRecord>,
    pub placeholders: Vec<DocumentPlaceholderRecord>,
    pub generation: DocumentGenerationMetadata,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentSetRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub plan_artifact: ArtifactReference,
    pub documents: Vec<ArtifactReference>,
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
pub enum FindingAuthority {
    Deterministic,
    HumanReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FindingCategory {
    CitationInvalid,
    UnclaimedContent,
    ProhibitedClaim,
    UnresolvedPlaceholder,
    CrossDocumentInconsistency,
    HumanJudgement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum FindingTarget {
    DocumentSet {
        document_set: ArtifactReference,
    },
    Document {
        document: ArtifactReference,
        document_id: EntityId,
    },
    Section {
        document: ArtifactReference,
        document_id: EntityId,
        section_id: EntityId,
    },
    Claim {
        document: ArtifactReference,
        document_id: EntityId,
        section_id: EntityId,
        claim_id: EntityId,
    },
    Placeholder {
        document: ArtifactReference,
        document_id: EntityId,
        placeholder_id: EntityId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewFindingCandidateRecord {
    pub code: String,
    pub category: FindingCategory,
    pub severity: FindingSeverity,
    pub message: String,
    pub target: FindingTarget,
    pub related_targets: Vec<FindingTarget>,
    pub suggested_resolution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewCandidate {
    pub job_id: EntityId,
    pub document_set_artifact: ArtifactReference,
    pub findings: Vec<ReviewFindingCandidateRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FindingStatus {
    Open,
    AcceptedRisk,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindingRecord {
    pub id: EntityId,
    pub code: String,
    pub category: FindingCategory,
    pub severity: FindingSeverity,
    pub authority: FindingAuthority,
    pub message: String,
    pub target: FindingTarget,
    pub related_targets: Vec<FindingTarget>,
    pub suggested_resolution: Option<String>,
    pub status: FindingStatus,
    pub disposition_reason: Option<String>,
    pub decided_by: Option<ActorKind>,
    pub decided_at: Option<UtcTimestamp>,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewFindingsRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub document_set_artifact: ArtifactReference,
    pub findings: Vec<FindingRecord>,
    pub reviewed_by: ActorKind,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FindingDisposition {
    AcceptedRisk,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindingDispositionCandidateRecord {
    pub finding_id: EntityId,
    pub expected_revision: Revision,
    pub disposition: Option<FindingDisposition>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewDispositionCandidate {
    pub job_id: EntityId,
    pub review_artifact: ArtifactReference,
    pub decisions: Vec<FindingDispositionCandidateRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessState {
    Blocked,
    NeedsReview,
    ReadyToExport,
    Exported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessReasonCode {
    MissingRequiredDocument,
    StaleDocument,
    DocumentPlanMismatch,
    ReviewDocumentSetMismatch,
    OpenDeterministicFinding,
    PendingHumanFinding,
    MixedPlanRevision,
    MixedEvidenceRevision,
    MixedProfileRevision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadinessReasonRecord {
    pub code: ReadinessReasonCode,
    pub document_kind: Option<DocumentKind>,
    pub artifact: Option<ArtifactReference>,
    pub finding_id: Option<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadinessRecord {
    pub job_id: EntityId,
    pub state: ReadinessState,
    pub reasons: Vec<ReadinessReasonRecord>,
    pub checked_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageManifestRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub plan_artifact: ArtifactReference,
    pub evidence_artifact: ArtifactReference,
    pub profile_revision: Revision,
    pub document_set_artifact: ArtifactReference,
    pub documents: Vec<ArtifactReference>,
    pub review_artifact: ArtifactReference,
    pub readiness: ReadinessRecord,
    pub submission_performed: bool,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionKind {
    Markdown,
    StructuredJson,
    TypstSource,
    PackageManifestJson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionEditStatus {
    Current,
    Edited,
    Missing,
    RepairRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectionRecord {
    pub source_artifact: ArtifactReference,
    pub relative_path: SafeRelativePath,
    pub kind: ProjectionKind,
    pub generated_sha256: Sha256Digest,
    pub observed_sha256: Option<Sha256Digest>,
    pub edit_status: ProjectionEditStatus,
    pub updated_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageExportManifestRecord {
    pub id: EntityId,
    pub job_id: EntityId,
    pub package_artifact: ArtifactReference,
    pub projections: Vec<ProjectionRecord>,
    pub exported_at: UtcTimestamp,
    pub submission_performed: bool,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionReconcileAction {
    Inspect,
    Replace,
    CopyAsNew,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectionReconcileRecord {
    pub job_id: EntityId,
    pub package_artifact: ArtifactReference,
    pub projection: ProjectionRecord,
    pub action: ProjectionReconcileAction,
    pub preserved_copy_path: Option<SafeRelativePath>,
    pub preserved_copy_sha256: Option<Sha256Digest>,
    pub authoritative_changed: bool,
    pub reconciled_at: UtcTimestamp,
}
