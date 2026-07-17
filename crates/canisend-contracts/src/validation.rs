use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;

use crate::{
    ApplicationPlanRecord, CriteriaSetRecord, CriterionRecord, DocumentRecord,
    EvidenceCatalogRecord, EvidenceMatchRecord, EvidenceProposalRecord, EvidenceProposalSet,
    EvidenceRecord, FindingRecord, JobRecord, ParsedJobRecord, ProfileSourceRecord,
    ReadinessRecord, SourceRecord,
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
        let mut violations = Vec::new();
        required_text(&self.rationale, "/rationale", &mut violations);
        if !matches!(
            self.strength,
            crate::MatchStrength::Gap | crate::MatchStrength::Unknown
        ) && self.evidence_ids.is_empty()
        {
            violations.push(ContractViolation::new(
                "match.evidence_required",
                "/evidence_ids",
                "strong and partial matches require at least one evidence ID",
            ));
        }
        violations
    }
}

impl SemanticValidate for ApplicationPlanRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.strategy, "/strategy", &mut violations);
        violations
    }
}

impl SemanticValidate for DocumentRecord {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.title, "/title", &mut violations);
        if matches!(self.status, crate::DocumentStatus::Current) && self.artifact.is_none() {
            violations.push(ContractViolation::new(
                "document.artifact_required",
                "/artifact",
                "a current document must reference an artifact",
            ));
        }
        violations
    }
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
    use crate::JobRecord;

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
}
