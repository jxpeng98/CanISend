use canisend_contracts::{
    ConsentScope, DiscoveryImportReport, DiscoverySourceKind, EntityId, Sha256Digest,
};
use serde::{Deserialize, Serialize};

use crate::{ApplicationError, JobIntakePreviewReadModel, JobIntakeSourceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntakeSourceKind {
    Url,
    Pdf,
    LocalFile,
    Csv,
    Json,
    Agent,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntakeDuplicateState {
    NoneKnown,
    ExactMatch,
    ReviewAfterCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntakeTargetKind {
    Application,
    OpportunityLibrary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntakeCommitBoundary {
    ExactPreparedBytes,
    ExactNormalizedReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeSourceIdentityReadModel {
    pub kind: IntakeSourceKind,
    pub locator: String,
    pub detected_type: String,
    pub sha256: Option<Sha256Digest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeExtractionReadModel {
    pub original_bytes: Option<u64>,
    pub normalized_text_bytes: Option<u64>,
    pub normalized_lines: Option<u64>,
    pub pdf_pages: Option<u64>,
    pub accepted_items: u64,
    pub rejected_items: u64,
    pub semantic_fields_pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeDuplicateSignalReadModel {
    pub state: IntakeDuplicateState,
    pub count: u64,
    pub automatic_merge: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeTargetReadModel {
    pub kind: IntakeTargetKind,
    pub id: Option<EntityId>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeMutationReadModel {
    pub subject: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeReviewReadModel {
    pub source: IntakeSourceIdentityReadModel,
    pub extraction: IntakeExtractionReadModel,
    pub duplicate_signal: IntakeDuplicateSignalReadModel,
    pub target: IntakeTargetReadModel,
    pub intended_mutations: Vec<IntakeMutationReadModel>,
    pub required_consent: ConsentScope,
    pub consent_confirmed: bool,
    pub commit_boundary: IntakeCommitBoundary,
}

#[must_use]
pub fn job_intake_review(preview: &JobIntakePreviewReadModel) -> IntakeReviewReadModel {
    let exact_duplicates = preview
        .validation_issues
        .iter()
        .filter(|issue| issue.code == "source.duplicate-content")
        .count();
    let kind = if preview.extraction.content_type == "application/pdf" {
        IntakeSourceKind::Pdf
    } else {
        match preview.provenance.source_kind {
            JobIntakeSourceKind::LocalFile => IntakeSourceKind::LocalFile,
            JobIntakeSourceKind::Url => IntakeSourceKind::Url,
        }
    };
    let required_consent = match preview.provenance.source_kind {
        JobIntakeSourceKind::LocalFile => ConsentScope::ReadPrivateInputs,
        JobIntakeSourceKind::Url => ConsentScope::FetchUserSuppliedUrl,
    };
    IntakeReviewReadModel {
        source: IntakeSourceIdentityReadModel {
            kind,
            locator: preview
                .provenance
                .final_url
                .clone()
                .unwrap_or_else(|| preview.provenance.requested_locator.clone()),
            detected_type: preview.extraction.content_type.clone(),
            sha256: Some(preview.provenance.original_sha256.clone()),
        },
        extraction: IntakeExtractionReadModel {
            original_bytes: Some(preview.extraction.original_bytes),
            normalized_text_bytes: Some(preview.extraction.normalized_text_bytes),
            normalized_lines: Some(preview.extraction.normalized_lines),
            pdf_pages: preview.extraction.pdf_pages,
            accepted_items: 1,
            rejected_items: 0,
            semantic_fields_pending: preview.extraction.semantic_fields_pending,
        },
        duplicate_signal: IntakeDuplicateSignalReadModel {
            state: if exact_duplicates == 0 {
                IntakeDuplicateState::NoneKnown
            } else {
                IntakeDuplicateState::ExactMatch
            },
            count: u64::try_from(exact_duplicates).unwrap_or(u64::MAX),
            automatic_merge: false,
        },
        target: IntakeTargetReadModel {
            kind: IntakeTargetKind::Application,
            id: Some(preview.job.id.clone()),
            label: format!("{} — {}", preview.job.title, preview.job.institution),
        },
        intended_mutations: preview
            .intended_mutations
            .iter()
            .map(|mutation| IntakeMutationReadModel {
                subject: mutation.subject.clone(),
                action: mutation.action.clone(),
                description: mutation.description.clone(),
            })
            .collect(),
        required_consent,
        consent_confirmed: true,
        commit_boundary: IntakeCommitBoundary::ExactPreparedBytes,
    }
}

pub fn discovery_intake_review(
    report: &DiscoveryImportReport,
    locator: impl Into<String>,
    required_consent: ConsentScope,
) -> Result<IntakeReviewReadModel, ApplicationError> {
    let batch = report.batch.as_ref().ok_or_else(|| {
        ApplicationError::InvalidInput(
            "validated discovery preview does not contain its normalized batch".to_owned(),
        )
    })?;
    let source_kind = match batch.source_kind {
        DiscoverySourceKind::Csv => IntakeSourceKind::Csv,
        DiscoverySourceKind::Json => IntakeSourceKind::Json,
        DiscoverySourceKind::HostAgent => IntakeSourceKind::Agent,
        DiscoverySourceKind::RssAtom
        | DiscoverySourceKind::JobsAcUk
        | DiscoverySourceKind::Greenhouse
        | DiscoverySourceKind::Lever => IntakeSourceKind::Network,
    };
    let detected_type = match batch.source_kind {
        DiscoverySourceKind::Csv => "text/csv",
        DiscoverySourceKind::Json | DiscoverySourceKind::HostAgent => "application/json",
        DiscoverySourceKind::RssAtom => "application/rss+xml",
        DiscoverySourceKind::JobsAcUk => "text/html",
        DiscoverySourceKind::Greenhouse | DiscoverySourceKind::Lever => "application/json",
    };
    Ok(IntakeReviewReadModel {
        source: IntakeSourceIdentityReadModel {
            kind: source_kind,
            locator: locator.into(),
            detected_type: detected_type.to_owned(),
            sha256: None,
        },
        extraction: IntakeExtractionReadModel {
            original_bytes: None,
            normalized_text_bytes: None,
            normalized_lines: None,
            pdf_pages: None,
            accepted_items: report.accepted,
            rejected_items: report.rejected,
            semantic_fields_pending: false,
        },
        duplicate_signal: IntakeDuplicateSignalReadModel {
            state: IntakeDuplicateState::ReviewAfterCommit,
            count: 0,
            automatic_merge: false,
        },
        target: IntakeTargetReadModel {
            kind: IntakeTargetKind::OpportunityLibrary,
            id: None,
            label: batch.source_name.clone(),
        },
        intended_mutations: vec![
            IntakeMutationReadModel {
                subject: format!("discovery-source:{}", batch.source_name),
                action: "register-or-refresh-source".to_owned(),
                description: "Commit the reviewed source identity and refresh metadata".to_owned(),
            },
            IntakeMutationReadModel {
                subject: "opportunity-library".to_owned(),
                action: "upsert-reviewed-leads".to_owned(),
                description: format!(
                    "Commit {} accepted lead(s); keep {} rejected row diagnostic(s) body-free",
                    report.accepted, report.rejected
                ),
            },
        ],
        required_consent,
        consent_confirmed: true,
        commit_boundary: IntakeCommitBoundary::ExactNormalizedReport,
    })
}

#[cfg(test)]
mod tests {
    use canisend_contracts::{
        ConsentScope, DiscoveryBatch, DiscoveryImportReport, DiscoverySourceKind, UtcTimestamp,
    };

    use super::{
        IntakeCommitBoundary, IntakeDuplicateState, IntakeSourceKind, discovery_intake_review,
    };

    #[test]
    fn discovery_review_keeps_agent_identity_and_exact_report_boundary() {
        let report = DiscoveryImportReport {
            dry_run: true,
            accepted: 2,
            rejected: 1,
            diagnostics: Vec::new(),
            batch: Some(DiscoveryBatch {
                source_kind: DiscoverySourceKind::HostAgent,
                source_name: "Codex leads".to_owned(),
                source_url: None,
                cursor: None,
                observed_at: UtcTimestamp::try_new("2026-07-30T12:00:00Z").expect("timestamp"),
                leads: Vec::new(),
            }),
            receipt: None,
        };
        let review =
            discovery_intake_review(&report, "/tmp/leads.json", ConsentScope::ReadPrivateInputs)
                .expect("review");

        assert_eq!(review.source.kind, IntakeSourceKind::Agent);
        assert_eq!(
            review.duplicate_signal.state,
            IntakeDuplicateState::ReviewAfterCommit
        );
        assert_eq!(
            review.commit_boundary,
            IntakeCommitBoundary::ExactNormalizedReport
        );
        assert_eq!(review.extraction.accepted_items, 2);
    }
}
