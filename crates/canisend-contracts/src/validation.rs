use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;

use crate::{
    ApplicationPlanCandidate, ApplicationPlanRecord, ApplicationStrategyRecord, CriteriaSetRecord,
    CriterionRecord, DocumentCandidate, DocumentClaimCandidateRecord,
    DocumentPlaceholderCandidateRecord, DocumentPlanCandidateRecord, DocumentRecord,
    DocumentSectionCandidateRecord, DocumentSetRecord, EvidenceCatalogRecord,
    EvidenceMatchProposalRecord, EvidenceMatchProposalSet, EvidenceMatchRecord,
    EvidenceMatchSetRecord, EvidenceProposalRecord, EvidenceProposalSet, EvidenceRecord,
    FindingRecord, JobRecord, ParsedJobRecord, ProfileSourceRecord, ReadinessRecord, SourceRecord,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractViolation {
    pub code: String,
    pub json_pointer: String,
    pub message: String,
}

impl ContractViolation {
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        json_pointer: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            json_pointer: json_pointer.into(),
            message: message.into(),
        }
    }
}

pub trait SemanticValidate {
    fn validate_semantics(&self) -> Vec<ContractViolation>;
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CandidateValidationError {
    #[error("candidate does not satisfy its JSON Schema")]
    Structural(Vec<ContractViolation>),
    #[error("candidate violates semantic contract rules")]
    Semantic(Vec<ContractViolation>),
}

impl CandidateValidationError {
    #[must_use]
    pub fn violations(&self) -> &[ContractViolation] {
        match self {
            Self::Structural(violations) | Self::Semantic(violations) => violations,
        }
    }
}

pub fn validate_external_candidate<T>(value: &Value) -> Result<T, CandidateValidationError>
where
    T: DeserializeOwned + JsonSchema + SemanticValidate,
{
    let schema = schemars::schema_for!(T);
    let schema_value = serde_json::to_value(schema).expect("generated schema serializes");
    if !jsonschema::draft202012::is_valid(&schema_value, value) {
        return Err(CandidateValidationError::Structural(vec![
            ContractViolation::new(
                "candidate.schema_invalid",
                "",
                "candidate does not satisfy the generated JSON Schema",
            ),
        ]));
    }
    let candidate: T = serde_json::from_value(value.clone()).map_err(|error| {
        CandidateValidationError::Semantic(vec![ContractViolation::new(
            "candidate.primitive_invalid",
            "",
            error.to_string(),
        )])
    })?;
    let violations = candidate.validate_semantics();
    if violations.is_empty() {
        Ok(candidate)
    } else {
        Err(CandidateValidationError::Semantic(violations))
    }
}

fn required_text(value: &str, pointer: &str, violations: &mut Vec<ContractViolation>) {
    if value.trim().is_empty() {
        violations.push(ContractViolation::new(
            "value.required",
            pointer,
            "value cannot be empty or whitespace",
        ));
    } else if value.len() > 16_384 {
        violations.push(ContractViolation::new(
            "value.too_long",
            pointer,
            "value exceeds the 16384-byte contract limit",
        ));
    }
}

impl SemanticValidate for JobRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.title, "/title", &mut violations);
        required_text(&self.institution, "/institution", &mut violations);
        if self.source_ids.is_empty() {
            violations.push(ContractViolation::new(
                "job.source_required",
                "/source_ids",
                "a job must reference at least one source",
            ));
        }
        violations
    }
}

impl SemanticValidate for SourceRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if matches!(self.kind, crate::SourceKind::UserUrl)
            && !self
                .source_url
                .as_deref()
                .is_some_and(|url| url.starts_with("https://") || url.starts_with("http://"))
        {
            violations.push(ContractViolation::new(
                "source.url_required",
                "/source_url",
                "a user URL source requires an http or https URL",
            ));
        }
        violations
    }
}

impl SemanticValidate for ProfileSourceRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.content_type, "/content_type", &mut violations);
        if self.original.kind != crate::ArtifactKind::SourceOriginal {
            violations.push(ContractViolation::new(
                "profile_source.original_kind_invalid",
                "/original/kind",
                "profile original must reference source-original",
            ));
        }
        if self.normalized_text.kind != crate::ArtifactKind::SourceNormalizedText {
            violations.push(ContractViolation::new(
                "profile_source.normalized_kind_invalid",
                "/normalized_text/kind",
                "profile text must reference source-normalized-text",
            ));
        }
        violations
    }
}

impl SemanticValidate for EvidenceRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.summary, "/summary", &mut violations);
        required_text(&self.source_quote, "/source_quote", &mut violations);
        validate_source_span(&self.source_span, "/source_span", &mut violations);
        if self.excluded && !self.confirmed {
            violations.push(ContractViolation::new(
                "evidence.exclusion_unconfirmed",
                "/excluded",
                "only a confirmed evidence decision may exclude an item",
            ));
        }
        violations
    }
}

impl SemanticValidate for EvidenceProposalRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.summary, "/summary", &mut violations);
        required_text(&self.source_quote, "/source_quote", &mut violations);
        validate_source_span(&self.source_span, "/source_span", &mut violations);
        violations
    }
}

impl SemanticValidate for EvidenceProposalSet {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if self.proposals.is_empty() || self.proposals.len() > 1_000 {
            violations.push(ContractViolation::new(
                "evidence_proposals.count_invalid",
                "/proposals",
                "evidence proposal set must contain between 1 and 1000 items",
            ));
        }
        for (index, proposal) in self.proposals.iter().enumerate() {
            for mut violation in proposal.validate_semantics() {
                violation.json_pointer = format!("/proposals/{index}{}", violation.json_pointer);
                violations.push(violation);
            }
        }
        violations
    }
}

impl SemanticValidate for EvidenceCatalogRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if self.items.is_empty() || self.items.len() > 1_000 {
            violations.push(ContractViolation::new(
                "evidence_catalog.count_invalid",
                "/items",
                "evidence catalog must contain between 1 and 1000 items",
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        let confirmation = self.items.first().map(|item| item.confirmed);
        for (index, item) in self.items.iter().enumerate() {
            for mut violation in item.validate_semantics() {
                violation.json_pointer = format!("/items/{index}{}", violation.json_pointer);
                violations.push(violation);
            }
            if !ids.insert(item.id.clone()) {
                violations.push(ContractViolation::new(
                    "evidence.id_duplicate",
                    format!("/items/{index}/id"),
                    "evidence IDs must be unique within the catalog",
                ));
            }
            if Some(item.confirmed) != confirmation {
                violations.push(ContractViolation::new(
                    "evidence.confirmation_mixed",
                    format!("/items/{index}/confirmed"),
                    "a catalog cannot mix proposed and confirmed evidence",
                ));
            }
        }
        violations
    }
}

fn validate_source_span(
    span: &crate::SourceTextSpan,
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    if span.source.kind != crate::ArtifactKind::SourceNormalizedText {
        violations.push(ContractViolation::new(
            "source_span.kind_invalid",
            format!("{pointer}/source/kind"),
            "source span must reference normalized source text",
        ));
    }
    if span.start_byte >= span.end_byte {
        violations.push(ContractViolation::new(
            "source_span.range_invalid",
            pointer,
            "source span start must be less than its end",
        ));
    }
}

impl SemanticValidate for CriterionRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.requirement, "/requirement", &mut violations);
        required_text(&self.source_quote, "/source_quote", &mut violations);
        validate_source_span(&self.source_span, "/source_span", &mut violations);
        if self.confidence_milli > 1_000 {
            violations.push(ContractViolation::new(
                "criterion.confidence_invalid",
                "/confidence_milli",
                "confidence_milli must be between 0 and 1000",
            ));
        }
        violations
    }
}

impl SemanticValidate for ParsedJobRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.title, "/title", &mut violations);
        required_text(&self.institution, "/institution", &mut violations);
        required_text(&self.summary, "/summary", &mut violations);
        if self.criteria.is_empty() || self.criteria.len() > 500 {
            violations.push(ContractViolation::new(
                "parsed_job.criteria_count_invalid",
                "/criteria",
                "parsed job must contain between 1 and 500 criterion proposals",
            ));
        }
        validate_criteria(&self.criteria, &self.job_id, false, &mut violations);
        violations
    }
}

impl SemanticValidate for CriteriaSetRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if self.criteria.is_empty() || self.criteria.len() > 500 {
            violations.push(ContractViolation::new(
                "criteria.count_invalid",
                "/criteria",
                "confirmed set must contain between 1 and 500 criteria",
            ));
        }
        validate_criteria(&self.criteria, &self.job_id, true, &mut violations);
        violations
    }
}

fn validate_criteria(
    criteria: &[CriterionRecord],
    job_id: &crate::EntityId,
    must_be_confirmed: bool,
    violations: &mut Vec<ContractViolation>,
) {
    let mut ids = std::collections::BTreeSet::new();
    for (index, criterion) in criteria.iter().enumerate() {
        let pointer = format!("/criteria/{index}");
        for mut violation in criterion.validate_semantics() {
            violation.json_pointer = format!("{pointer}{}", violation.json_pointer);
            violations.push(violation);
        }
        if criterion.job_id != *job_id {
            violations.push(ContractViolation::new(
                "criterion.job_mismatch",
                format!("{pointer}/job_id"),
                "criterion job ID must match its containing record",
            ));
        }
        if !ids.insert(criterion.id.clone()) {
            violations.push(ContractViolation::new(
                "criterion.id_duplicate",
                format!("{pointer}/id"),
                "criterion IDs must be unique within the record",
            ));
        }
        if criterion.confirmed != must_be_confirmed {
            violations.push(ContractViolation::new(
                "criterion.confirmation_invalid",
                format!("{pointer}/confirmed"),
                if must_be_confirmed {
                    "every criterion must be explicitly confirmed"
                } else {
                    "agent-proposed criteria cannot mark themselves confirmed"
                },
            ));
        }
    }
}

impl SemanticValidate for EvidenceMatchRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        validate_match_fields(
            &self.evidence,
            self.strength,
            &self.rationale,
            self.gap.as_deref(),
            &self.prohibited_claims,
        )
    }
}

impl SemanticValidate for EvidenceMatchProposalRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        validate_match_fields(
            &self.evidence,
            self.strength,
            &self.rationale,
            self.gap.as_deref(),
            &self.prohibited_claims,
        )
    }
}

fn validate_match_fields(
    evidence: &[crate::EvidenceRevisionReference],
    strength: crate::MatchStrength,
    rationale: &str,
    gap: Option<&str>,
    prohibited_claims: &[String],
) -> Vec<ContractViolation> {
    let mut violations = Vec::new();
    required_text(rationale, "/rationale", &mut violations);
    if evidence.len() > 100 {
        violations.push(ContractViolation::new(
            "match.evidence_count_invalid",
            "/evidence",
            "a match may cite at most 100 evidence revisions",
        ));
    }
    let mut evidence_ids = std::collections::BTreeSet::new();
    for (index, reference) in evidence.iter().enumerate() {
        if !evidence_ids.insert(&reference.id) {
            violations.push(ContractViolation::new(
                "match.evidence_duplicate",
                format!("/evidence/{index}/id"),
                "a match cannot cite the same evidence identity more than once",
            ));
        }
    }
    match strength {
        crate::MatchStrength::Strong => {
            if evidence.is_empty() {
                violations.push(ContractViolation::new(
                    "match.evidence_required",
                    "/evidence",
                    "a strong match requires at least one evidence revision",
                ));
            }
            if gap.is_some() {
                violations.push(ContractViolation::new(
                    "match.gap_forbidden",
                    "/gap",
                    "a strong match cannot declare a remaining support gap",
                ));
            }
        }
        crate::MatchStrength::Partial => {
            if evidence.is_empty() {
                violations.push(ContractViolation::new(
                    "match.evidence_required",
                    "/evidence",
                    "a partial match requires at least one evidence revision",
                ));
            }
            if let Some(gap) = gap {
                required_text(gap, "/gap", &mut violations);
            } else {
                violations.push(ContractViolation::new(
                    "match.gap_required",
                    "/gap",
                    "a partial match must state what remains unsupported",
                ));
            }
        }
        crate::MatchStrength::Gap | crate::MatchStrength::Unknown => {
            if !evidence.is_empty() {
                violations.push(ContractViolation::new(
                    "match.evidence_forbidden",
                    "/evidence",
                    "gap and unknown matches cannot claim supporting evidence",
                ));
            }
            if let Some(gap) = gap {
                required_text(gap, "/gap", &mut violations);
            } else {
                violations.push(ContractViolation::new(
                    "match.gap_required",
                    "/gap",
                    "gap and unknown matches must state the unresolved support issue",
                ));
            }
        }
    }
    if prohibited_claims.len() > 100 {
        violations.push(ContractViolation::new(
            "match.prohibited_claim_count_invalid",
            "/prohibited_claims",
            "a match may declare at most 100 prohibited claims",
        ));
    }
    let mut claims = std::collections::BTreeSet::new();
    for (index, claim) in prohibited_claims.iter().enumerate() {
        required_text(
            claim,
            &format!("/prohibited_claims/{index}"),
            &mut violations,
        );
        if !claims.insert(claim.trim()) {
            violations.push(ContractViolation::new(
                "match.prohibited_claim_duplicate",
                format!("/prohibited_claims/{index}"),
                "prohibited claims must be unique within a match",
            ));
        }
    }
    violations
}

impl SemanticValidate for EvidenceMatchProposalSet {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations =
            validate_match_set_artifacts(&self.criteria_artifact, &self.evidence_artifact);
        if self.proposals.is_empty() || self.proposals.len() > 500 {
            violations.push(ContractViolation::new(
                "match_proposals.count_invalid",
                "/proposals",
                "match proposals must contain between 1 and 500 records",
            ));
        }
        let mut criteria = std::collections::BTreeSet::new();
        for (index, proposal) in self.proposals.iter().enumerate() {
            for mut violation in proposal.validate_semantics() {
                violation.json_pointer = format!("/proposals/{index}{}", violation.json_pointer);
                violations.push(violation);
            }
            if !criteria.insert(&proposal.criterion.id) {
                violations.push(ContractViolation::new(
                    "match.criterion_duplicate",
                    format!("/proposals/{index}/criterion/id"),
                    "each criterion may appear exactly once",
                ));
            }
        }
        violations
    }
}

impl SemanticValidate for EvidenceMatchSetRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations =
            validate_match_set_artifacts(&self.criteria_artifact, &self.evidence_artifact);
        if self.matches.is_empty() || self.matches.len() > 500 {
            violations.push(ContractViolation::new(
                "matches.count_invalid",
                "/matches",
                "a match set must contain between 1 and 500 records",
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut criteria = std::collections::BTreeSet::new();
        for (index, record) in self.matches.iter().enumerate() {
            for mut violation in record.validate_semantics() {
                violation.json_pointer = format!("/matches/{index}{}", violation.json_pointer);
                violations.push(violation);
            }
            if !ids.insert(&record.id) {
                violations.push(ContractViolation::new(
                    "match.id_duplicate",
                    format!("/matches/{index}/id"),
                    "core-generated match IDs must be unique",
                ));
            }
            if !criteria.insert(&record.criterion.id) {
                violations.push(ContractViolation::new(
                    "match.criterion_duplicate",
                    format!("/matches/{index}/criterion/id"),
                    "each criterion may appear exactly once",
                ));
            }
        }
        violations
    }
}

fn validate_match_set_artifacts(
    criteria_artifact: &crate::ArtifactReference,
    evidence_artifact: &crate::ArtifactReference,
) -> Vec<ContractViolation> {
    let mut violations = Vec::new();
    if criteria_artifact.kind != crate::ArtifactKind::Criteria {
        violations.push(ContractViolation::new(
            "matches.criteria_kind_invalid",
            "/criteria_artifact/kind",
            "matches must reference a criteria artifact",
        ));
    }
    if evidence_artifact.kind != crate::ArtifactKind::EvidenceCatalog {
        violations.push(ContractViolation::new(
            "matches.evidence_kind_invalid",
            "/evidence_artifact/kind",
            "matches must reference an evidence catalog artifact",
        ));
    }
    violations
}

impl SemanticValidate for ApplicationPlanRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let documents = self
            .documents
            .iter()
            .map(|document| DocumentPlanCandidateRecord {
                kind: document.kind,
                requirement: document.requirement,
                rationale: document.rationale.clone(),
                constraints: document.constraints.clone(),
                executor: document.executor,
            })
            .collect::<Vec<_>>();
        validate_plan(
            self.matches_artifact.kind,
            self.decision,
            &self.strategy,
            &documents,
            &self.blockers,
        )
    }
}

impl SemanticValidate for ApplicationPlanCandidate {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        validate_plan(
            self.matches_artifact.kind,
            self.decision,
            &self.strategy,
            &self.documents,
            &self.blockers,
        )
    }
}

fn validate_plan(
    matches_kind: crate::ArtifactKind,
    decision: crate::ApplicationDecision,
    strategy: &ApplicationStrategyRecord,
    documents: &[DocumentPlanCandidateRecord],
    blockers: &[crate::PlanBlockerRecord],
) -> Vec<ContractViolation> {
    let mut violations = Vec::new();
    if matches_kind != crate::ArtifactKind::EvidenceMatches {
        violations.push(ContractViolation::new(
            "plan.matches_kind_invalid",
            "/matches_artifact/kind",
            "application plan must reference an evidence-matches artifact",
        ));
    }
    required_text(
        &strategy.positioning,
        "/strategy/positioning",
        &mut violations,
    );
    validate_text_list(
        &strategy.priorities,
        "/strategy/priorities",
        1,
        50,
        &mut violations,
    );
    validate_text_list(&strategy.risks, "/strategy/risks", 0, 100, &mut violations);
    if documents.len() != 4 {
        violations.push(ContractViolation::new(
            "plan.documents_count_invalid",
            "/documents",
            "plan must contain exactly one entry for every supported document kind",
        ));
    }
    let mut kinds = std::collections::BTreeSet::new();
    for (index, document) in documents.iter().enumerate() {
        if !kinds.insert(document.kind) {
            violations.push(ContractViolation::new(
                "plan.document_kind_duplicate",
                format!("/documents/{index}/kind"),
                "each supported document kind must appear exactly once",
            ));
        }
        required_text(
            &document.rationale,
            &format!("/documents/{index}/rationale"),
            &mut violations,
        );
        validate_text_list(
            &document.constraints,
            &format!("/documents/{index}/constraints"),
            0,
            50,
            &mut violations,
        );
        match (document.requirement, document.executor) {
            (crate::DocumentRequirement::Omitted, None) => {}
            (crate::DocumentRequirement::Omitted, Some(_)) => {
                violations.push(ContractViolation::new(
                    "plan.omitted_executor_forbidden",
                    format!("/documents/{index}/executor"),
                    "an omitted document cannot have an executor",
                ))
            }
            (
                _,
                Some(crate::ExecutionMode::HostAgent | crate::ExecutionMode::ConfiguredProvider),
            ) => {}
            (_, Some(_)) => violations.push(ContractViolation::new(
                "plan.executor_invalid",
                format!("/documents/{index}/executor"),
                "planned documents support host-agent or configured-provider execution",
            )),
            (_, None) => violations.push(ContractViolation::new(
                "plan.executor_required",
                format!("/documents/{index}/executor"),
                "a required or optional document needs an executor",
            )),
        }
    }
    if decision == crate::ApplicationDecision::Skip
        && documents
            .iter()
            .any(|document| document.requirement != crate::DocumentRequirement::Omitted)
    {
        violations.push(ContractViolation::new(
            "plan.skip_documents_forbidden",
            "/documents",
            "a skipped application must omit every document",
        ));
    }
    if blockers.len() > 500 {
        violations.push(ContractViolation::new(
            "plan.blocker_count_invalid",
            "/blockers",
            "a plan may contain at most 500 derived blockers",
        ));
    }
    let mut blocker_keys = std::collections::BTreeSet::new();
    for (index, blocker) in blockers.iter().enumerate() {
        required_text(
            &blocker.code,
            &format!("/blockers/{index}/code"),
            &mut violations,
        );
        required_text(
            &blocker.description,
            &format!("/blockers/{index}/description"),
            &mut violations,
        );
        if !blocker_keys.insert((&blocker.criterion.id, blocker.code.trim())) {
            violations.push(ContractViolation::new(
                "plan.blocker_duplicate",
                format!("/blockers/{index}"),
                "derived blockers must be unique per criterion and code",
            ));
        }
    }
    if decision == crate::ApplicationDecision::Apply
        && blockers
            .iter()
            .any(|blocker| blocker.severity == crate::PlanBlockerSeverity::Blocking)
    {
        violations.push(ContractViolation::new(
            "plan.apply_blocked",
            "/decision",
            "apply cannot be confirmed while essential evidence blockers remain",
        ));
    }
    violations
}

fn validate_text_list(
    values: &[String],
    pointer: &str,
    minimum: usize,
    maximum: usize,
    violations: &mut Vec<ContractViolation>,
) {
    if values.len() < minimum || values.len() > maximum {
        violations.push(ContractViolation::new(
            "value.count_invalid",
            pointer,
            format!("list must contain between {minimum} and {maximum} items"),
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        required_text(value, &format!("{pointer}/{index}"), violations);
        if !unique.insert(value.trim()) {
            violations.push(ContractViolation::new(
                "value.duplicate",
                format!("{pointer}/{index}"),
                "list items must be unique",
            ));
        }
    }
}

impl SemanticValidate for DocumentCandidate {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        validate_document(
            self.plan_artifact.kind,
            self.kind,
            &self.title,
            &self.sections,
            &self.placeholders,
        )
    }
}

impl SemanticValidate for DocumentRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let sections = self
            .sections
            .iter()
            .map(|section| DocumentSectionCandidateRecord {
                kind: section.kind,
                heading: section.heading.clone(),
                body: section.body.clone(),
                claims: section
                    .claims
                    .iter()
                    .map(|claim| DocumentClaimCandidateRecord {
                        text: claim.text.clone(),
                        classification: claim.classification,
                        citations: claim.citations.clone(),
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        let placeholders = self
            .placeholders
            .iter()
            .map(|placeholder| DocumentPlaceholderCandidateRecord {
                key: placeholder.key.clone(),
                instruction: placeholder.instruction.clone(),
                required: placeholder.required,
                resolution: placeholder.resolution.clone(),
            })
            .collect::<Vec<_>>();
        let mut violations = validate_document(
            self.plan_artifact.kind,
            self.kind,
            &self.title,
            &sections,
            &placeholders,
        );
        let mut ids = std::collections::BTreeSet::new();
        ids.insert(&self.id);
        for (section_index, section) in self.sections.iter().enumerate() {
            if !ids.insert(&section.id) {
                violations.push(ContractViolation::new(
                    "document.id_duplicate",
                    format!("/sections/{section_index}/id"),
                    "document, section, claim, and placeholder IDs must be globally unique",
                ));
            }
            for (claim_index, claim) in section.claims.iter().enumerate() {
                if !ids.insert(&claim.id) {
                    violations.push(ContractViolation::new(
                        "document.id_duplicate",
                        format!("/sections/{section_index}/claims/{claim_index}/id"),
                        "document, section, claim, and placeholder IDs must be globally unique",
                    ));
                }
            }
        }
        for (index, placeholder) in self.placeholders.iter().enumerate() {
            if !ids.insert(&placeholder.id) {
                violations.push(ContractViolation::new(
                    "document.id_duplicate",
                    format!("/placeholders/{index}/id"),
                    "document, section, claim, and placeholder IDs must be globally unique",
                ));
            }
        }
        let generation_pair_valid = matches!(
            (self.generation.actor, self.generation.execution_mode),
            (crate::ActorKind::HostAgent, crate::ExecutionMode::HostAgent)
                | (
                    crate::ActorKind::ConfiguredProvider,
                    crate::ExecutionMode::ConfiguredProvider
                )
        );
        if !generation_pair_valid {
            violations.push(ContractViolation::new(
                "document.generation_mode_invalid",
                "/generation/execution_mode",
                "document generation actor and mode must identify the same bounded executor",
            ));
        }
        required_text(
            &self.generation.prompt_resource_id,
            "/generation/prompt_resource_id",
            &mut violations,
        );
        violations
    }
}

impl SemanticValidate for DocumentSetRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if self.plan_artifact.kind != crate::ArtifactKind::ApplicationPlan {
            violations.push(ContractViolation::new(
                "document_set.plan_kind_invalid",
                "/plan_artifact/kind",
                "document set must reference an application-plan artifact",
            ));
        }
        if self.documents.is_empty() || self.documents.len() > crate::DocumentKind::ALL.len() {
            violations.push(ContractViolation::new(
                "document_set.count_invalid",
                "/documents",
                "document set must contain between one and four structured documents",
            ));
        }
        let mut kinds = std::collections::BTreeSet::new();
        let mut ids = std::collections::BTreeSet::new();
        for (index, document) in self.documents.iter().enumerate() {
            if !matches!(
                document.kind,
                crate::ArtifactKind::CoverLetter
                    | crate::ArtifactKind::ResearchStatement
                    | crate::ArtifactKind::TeachingStatement
                    | crate::ArtifactKind::Cv
            ) {
                violations.push(ContractViolation::new(
                    "document_set.kind_invalid",
                    format!("/documents/{index}/kind"),
                    "document set may reference only supported structured document artifacts",
                ));
            }
            if !kinds.insert(document.kind) {
                violations.push(ContractViolation::new(
                    "document_set.kind_duplicate",
                    format!("/documents/{index}/kind"),
                    "document set may contain each document kind at most once",
                ));
            }
            if !ids.insert(&document.id) {
                violations.push(ContractViolation::new(
                    "document_set.id_duplicate",
                    format!("/documents/{index}/id"),
                    "document set artifact identities must be unique",
                ));
            }
        }
        violations
    }
}

fn validate_document(
    plan_kind: crate::ArtifactKind,
    kind: crate::DocumentKind,
    title: &str,
    sections: &[DocumentSectionCandidateRecord],
    placeholders: &[DocumentPlaceholderCandidateRecord],
) -> Vec<ContractViolation> {
    let mut violations = Vec::new();
    if plan_kind != crate::ArtifactKind::ApplicationPlan {
        violations.push(ContractViolation::new(
            "document.plan_kind_invalid",
            "/plan_artifact/kind",
            "structured document must reference an application-plan artifact",
        ));
    }
    required_text(title, "/title", &mut violations);
    if sections.is_empty() || sections.len() > 50 {
        violations.push(ContractViolation::new(
            "document.sections_count_invalid",
            "/sections",
            "document must contain between 1 and 50 sections",
        ));
    }
    let mut body_bytes = 0_usize;
    let mut claim_count = 0_usize;
    for (section_index, section) in sections.iter().enumerate() {
        if let Some(heading) = &section.heading {
            required_text(
                heading,
                &format!("/sections/{section_index}/heading"),
                &mut violations,
            );
        }
        required_text(
            &section.body,
            &format!("/sections/{section_index}/body"),
            &mut violations,
        );
        body_bytes = body_bytes.saturating_add(section.body.len());
        if section.claims.len() > 200 {
            violations.push(ContractViolation::new(
                "document.section_claims_count_invalid",
                format!("/sections/{section_index}/claims"),
                "a section may contain at most 200 declared claims",
            ));
        }
        claim_count = claim_count.saturating_add(section.claims.len());
        for (claim_index, claim) in section.claims.iter().enumerate() {
            validate_document_claim(
                claim,
                &format!("/sections/{section_index}/claims/{claim_index}"),
                &mut violations,
            );
        }
    }
    if body_bytes > 262_144 {
        violations.push(ContractViolation::new(
            "document.body_too_large",
            "/sections",
            "combined section body text exceeds the 262144-byte contract limit",
        ));
    }
    if claim_count > 1_000 {
        violations.push(ContractViolation::new(
            "document.claims_count_invalid",
            "/sections",
            "document may contain at most 1000 declared claims",
        ));
    }
    validate_document_shape(kind, sections, &mut violations);
    if placeholders.len() > 200 {
        violations.push(ContractViolation::new(
            "document.placeholders_count_invalid",
            "/placeholders",
            "document may contain at most 200 placeholders",
        ));
    }
    let mut keys = std::collections::BTreeSet::new();
    for (index, placeholder) in placeholders.iter().enumerate() {
        let pointer = format!("/placeholders/{index}");
        if !valid_placeholder_key(&placeholder.key) {
            violations.push(ContractViolation::new(
                    "document.placeholder_key_invalid",
                    format!("{pointer}/key"),
                    "placeholder key must be 1-64 lowercase ASCII letters, digits, or hyphens and start with a letter",
                ));
        }
        if !keys.insert(placeholder.key.as_str()) {
            violations.push(ContractViolation::new(
                "document.placeholder_key_duplicate",
                format!("{pointer}/key"),
                "placeholder keys must be unique within a document",
            ));
        }
        required_text(
            &placeholder.instruction,
            &format!("{pointer}/instruction"),
            &mut violations,
        );
        if let Some(resolution) = &placeholder.resolution {
            required_text(
                resolution,
                &format!("{pointer}/resolution"),
                &mut violations,
            );
        }
    }
    violations
}

fn validate_document_claim(
    claim: &DocumentClaimCandidateRecord,
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    required_text(&claim.text, &format!("{pointer}/text"), violations);
    if claim.citations.len() > 50 {
        violations.push(ContractViolation::new(
            "document.claim_citations_count_invalid",
            format!("{pointer}/citations"),
            "a claim may contain at most 50 citations",
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    for (index, citation) in claim.citations.iter().enumerate() {
        required_text(
            &citation.purpose,
            &format!("{pointer}/citations/{index}/purpose"),
            violations,
        );
        let key = match &citation.target {
            crate::CitationTarget::Evidence { evidence } => {
                format!("evidence:{}:{}", evidence.id, evidence.revision.get())
            }
            crate::CitationTarget::Criterion { criterion } => {
                format!("criterion:{}:{}", criterion.id, criterion.revision.get())
            }
        };
        if !unique.insert(key) {
            violations.push(ContractViolation::new(
                "document.claim_citation_duplicate",
                format!("{pointer}/citations/{index}"),
                "claim citations must reference unique target revisions",
            ));
        }
    }
    let all_evidence = claim
        .citations
        .iter()
        .all(|citation| matches!(citation.target, crate::CitationTarget::Evidence { .. }));
    let all_criteria = claim
        .citations
        .iter()
        .all(|citation| matches!(citation.target, crate::CitationTarget::Criterion { .. }));
    match claim.classification {
        crate::ClaimClassification::ApplicantFact
            if claim.citations.is_empty() || !all_evidence =>
        {
            violations.push(ContractViolation::new(
                "document.applicant_fact_evidence_required",
                format!("{pointer}/citations"),
                "an applicant fact must cite one or more exact evidence revisions",
            ));
        }
        crate::ClaimClassification::JobRequirement
            if claim.citations.is_empty() || !all_criteria =>
        {
            violations.push(ContractViolation::new(
                "document.job_requirement_citation_required",
                format!("{pointer}/citations"),
                "a job requirement claim must cite one or more exact criterion revisions",
            ));
        }
        crate::ClaimClassification::UserIntent | crate::ClaimClassification::NonFactual
            if !claim.citations.is_empty() =>
        {
            violations.push(ContractViolation::new(
                "document.non_evidence_citation_forbidden",
                format!("{pointer}/citations"),
                "user-intent and non-factual claims must not masquerade as evidence-backed facts",
            ));
        }
        _ => {}
    }
}

fn validate_document_shape(
    kind: crate::DocumentKind,
    sections: &[DocumentSectionCandidateRecord],
    violations: &mut Vec<ContractViolation>,
) {
    let count = |kind| {
        sections
            .iter()
            .filter(|section| section.kind == kind)
            .count()
    };
    match kind {
        crate::DocumentKind::CoverLetter => {
            if count(crate::DocumentSectionKind::Opening) != 1 {
                violations.push(ContractViolation::new(
                    "document.cover_letter_opening_required",
                    "/sections",
                    "cover letter must contain exactly one opening section",
                ));
            }
            if count(crate::DocumentSectionKind::Closing) != 1 {
                violations.push(ContractViolation::new(
                    "document.cover_letter_closing_required",
                    "/sections",
                    "cover letter must contain exactly one closing section",
                ));
            }
        }
        crate::DocumentKind::ResearchStatement
            if count(crate::DocumentSectionKind::Research) == 0 =>
        {
            violations.push(ContractViolation::new(
                "document.research_section_required",
                "/sections",
                "research statement must contain a research section",
            ));
        }
        crate::DocumentKind::TeachingStatement
            if count(crate::DocumentSectionKind::Teaching) == 0 =>
        {
            violations.push(ContractViolation::new(
                "document.teaching_section_required",
                "/sections",
                "teaching statement must contain a teaching section",
            ));
        }
        crate::DocumentKind::Cv
            if !sections.iter().any(|section| {
                matches!(
                    section.kind,
                    crate::DocumentSectionKind::Education
                        | crate::DocumentSectionKind::Experience
                        | crate::DocumentSectionKind::Publications
                )
            }) =>
        {
            violations.push(ContractViolation::new(
                "document.cv_record_section_required",
                "/sections",
                "CV must contain education, experience, or publications",
            ));
        }
        _ => {}
    }
}

fn valid_placeholder_key(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

impl SemanticValidate for FindingRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.code, "/code", &mut violations);
        required_text(&self.message, "/message", &mut violations);
        violations
    }
}

impl SemanticValidate for ReadinessRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        if matches!(self.state, crate::ReadinessState::Blocked)
            && self.blocker_finding_ids.is_empty()
        {
            vec![ContractViolation::new(
                "readiness.blocker_required",
                "/blocker_finding_ids",
                "blocked readiness must reference at least one blocker finding",
            )]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{CandidateValidationError, validate_external_candidate};
    use crate::{DocumentCandidate, JobRecord};

    #[test]
    fn validation_runs_schema_before_semantics() {
        let structurally_invalid = json!({"title": 3});
        assert!(matches!(
            validate_external_candidate::<JobRecord>(&structurally_invalid),
            Err(CandidateValidationError::Structural(_))
        ));

        let semantically_invalid = json!({
            "id": "019f2f55-7c00-7000-8000-000000000002",
            "title": " ",
            "institution": "Example University",
            "source_ids": [],
            "created_at": "2026-07-17T12:30:00Z",
            "revision": 1,
            "archived": false
        });
        let error = validate_external_candidate::<JobRecord>(&semantically_invalid)
            .expect_err("semantic validation must fail");
        assert!(matches!(error, CandidateValidationError::Semantic(_)));
        assert_eq!(error.violations().len(), 2);
    }

    #[test]
    fn structured_document_claims_require_typed_support_and_document_shape() {
        let valid = json!({
            "job_id": "019f2f55-7c00-7000-8000-000000000002",
            "plan_artifact": {
                "kind": "application-plan",
                "id": "019f2f55-7c00-7000-8000-000000000010",
                "revision": 1,
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "planned_document": {
                "id": "019f2f55-7c00-7000-8000-000000000011",
                "revision": 1
            },
            "kind": "cover-letter",
            "title": "Lecturer in Economics cover letter",
            "sections": [
                {
                    "kind": "opening",
                    "heading": null,
                    "body": "I am applying for the Lecturer in Economics role.",
                    "claims": [{
                        "text": "The role is Lecturer in Economics.",
                        "classification": "job-requirement",
                        "citations": [{
                            "target": {
                                "kind": "criterion",
                                "criterion": {
                                    "id": "019f2f55-7c00-7000-8000-000000000202",
                                    "revision": 1
                                }
                            },
                            "purpose": "Identify the confirmed role requirement"
                        }]
                    }]
                },
                {
                    "kind": "research",
                    "heading": "Research",
                    "body": "My research has produced two peer-reviewed articles.",
                    "claims": [{
                        "text": "My research has produced two peer-reviewed articles.",
                        "classification": "applicant-fact",
                        "citations": [{
                            "target": {
                                "kind": "evidence",
                                "evidence": {
                                    "id": "019f2f55-7c00-7000-8000-000000000103",
                                    "revision": 1
                                }
                            },
                            "purpose": "Support the publication count"
                        }]
                    }]
                },
                {
                    "kind": "closing",
                    "heading": null,
                    "body": "I would welcome the opportunity to contribute.",
                    "claims": [{
                        "text": "I would welcome the opportunity to contribute.",
                        "classification": "user-intent",
                        "citations": []
                    }]
                }
            ],
            "placeholders": [{
                "key": "contact-name",
                "instruction": "Confirm the addressee before export",
                "required": true,
                "resolution": null
            }]
        });
        validate_external_candidate::<DocumentCandidate>(&valid).expect("valid document candidate");

        let mut unsupported = valid.clone();
        unsupported["sections"][1]["claims"][0]["citations"] = json!([]);
        let error = validate_external_candidate::<DocumentCandidate>(&unsupported)
            .expect_err("unsupported applicant fact");
        assert!(
            error
                .violations()
                .iter()
                .any(|violation| { violation.code == "document.applicant_fact_evidence_required" })
        );

        let mut wrong_shape = valid.clone();
        wrong_shape["sections"]
            .as_array_mut()
            .expect("sections")
            .pop();
        let error = validate_external_candidate::<DocumentCandidate>(&wrong_shape)
            .expect_err("cover letter without closing");
        assert!(
            error
                .violations()
                .iter()
                .any(|violation| { violation.code == "document.cover_letter_closing_required" })
        );

        for (kind, expected_code) in [
            ("research-statement", "document.research_section_required"),
            ("teaching-statement", "document.teaching_section_required"),
            ("cv", "document.cv_record_section_required"),
        ] {
            let mut candidate = valid.clone();
            candidate["kind"] = json!(kind);
            for section in candidate["sections"]
                .as_array_mut()
                .expect("candidate sections")
            {
                section["kind"] = json!("fit");
            }
            let error = validate_external_candidate::<DocumentCandidate>(&candidate)
                .expect_err("document-specific section is required");
            assert!(
                error
                    .violations()
                    .iter()
                    .any(|violation| violation.code == expected_code)
            );
        }

        let mut invented_identity = valid;
        invented_identity["sections"][0]["id"] = json!("019f2f55-7c00-7000-8000-000000000099");
        assert!(matches!(
            validate_external_candidate::<DocumentCandidate>(&invented_identity),
            Err(CandidateValidationError::Structural(_))
        ));
    }

    #[test]
    fn synthetic_cover_letter_fixture_uses_the_public_candidate_contract() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/v2-spec/cover-letter-candidate.json"
        ))
        .expect("cover letter fixture JSON");
        validate_external_candidate::<DocumentCandidate>(&fixture)
            .expect("cover letter fixture satisfies the document candidate contract");
    }
}
