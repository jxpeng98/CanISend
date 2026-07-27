use std::path::Path;

use canisend_contracts::{ReviewDispositionCandidate, ReviewFindingsRecord};
use canisend_store::ReviewService;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewWorkspaceReadModel {
    pub current: ReviewFindingsRecord,
    pub disposition_candidate: ReviewDispositionCandidate,
}

impl Application {
    pub fn current_review(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ReviewFindingsRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let review =
            ReviewService::new(&mut workspace.database, &workspace.blobs).current(&job_id)?;
        Ok(review_receipt(
            "review.show",
            "available",
            "Loaded current review findings",
            review,
        ))
    }

    pub fn review_disposition_template(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ReviewDispositionCandidate>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let candidate =
            ReviewService::new(&mut workspace.database, &workspace.blobs).template(&job_id)?;
        let count = candidate.decisions.len();
        Ok(ActionReceipt::new(
            "review.export",
            "available",
            format!("Prepared {count} human-review disposition candidate(s)"),
            candidate,
        ))
    }

    pub fn review_workspace(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ReviewWorkspaceReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let service = ReviewService::new(&mut workspace.database, &workspace.blobs);
        let current = service.current(&job_id)?;
        let disposition_candidate = service.template(&job_id)?;
        let deterministic = deterministic_open_blockers(&current);
        let pending_human = pending_human_findings(&current);
        Ok(ActionReceipt::new(
            "review.workspace",
            "available",
            format!(
                "Loaded review workspace: {deterministic} deterministic blocker(s), \
                 {pending_human} pending human finding(s)"
            ),
            ReviewWorkspaceReadModel {
                current,
                disposition_candidate,
            },
        ))
    }

    pub fn confirm_review_dispositions(
        root: &Path,
        job_id: &str,
        candidate: &Value,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ReviewFindingsRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let artifact = ReviewService::new(&mut workspace.database, &workspace.blobs)
            .confirm(&job_id, candidate)?;
        let review =
            ReviewService::new(&mut workspace.database, &workspace.blobs).current(&job_id)?;
        Ok(review_receipt(
            "review.confirm",
            "confirmed",
            "Confirmed human-review finding dispositions",
            review,
        )
        .with_artifacts([artifact]))
    }
}

fn review_receipt(
    operation: &'static str,
    status: &'static str,
    summary: &'static str,
    review: ReviewFindingsRecord,
) -> ActionReceipt<ReviewFindingsRecord> {
    let findings = review.findings.len();
    let deterministic = deterministic_open_blockers(&review);
    let pending_human = pending_human_findings(&review);
    ActionReceipt::new(
        operation,
        status,
        format!(
            "{summary}: {findings} finding(s), {deterministic} deterministic blocker(s), \
             {pending_human} pending human finding(s)"
        ),
        review,
    )
}

fn deterministic_open_blockers(review: &ReviewFindingsRecord) -> usize {
    use canisend_contracts::{FindingAuthority, FindingSeverity, FindingStatus};

    review
        .findings
        .iter()
        .filter(|finding| {
            finding.authority == FindingAuthority::Deterministic
                && finding.severity == FindingSeverity::Blocker
                && finding.status == FindingStatus::Open
        })
        .count()
}

fn pending_human_findings(review: &ReviewFindingsRecord) -> usize {
    use canisend_contracts::{FindingAuthority, FindingStatus};

    review
        .findings
        .iter()
        .filter(|finding| {
            finding.authority == FindingAuthority::HumanReview
                && finding.status == FindingStatus::Open
        })
        .count()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use canisend_contracts::ErrorCode;
    use serde_json::json;

    use crate::{Application, ApplicationError, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-review-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn review_ids_are_validated_before_workspace_access() {
        let missing = temporary_root("missing");
        let consent = PrivateReadConsent::granted_by_user();
        for error in [
            Application::current_review(&missing, "not-a-uuid", consent)
                .expect_err("invalid review job ID"),
            Application::review_disposition_template(&missing, "not-a-uuid", consent)
                .expect_err("invalid disposition job ID"),
            Application::review_workspace(&missing, "not-a-uuid", consent)
                .expect_err("invalid workspace job ID"),
        ] {
            assert!(matches!(error, ApplicationError::InvalidEntityId(_)));
        }
    }

    #[test]
    fn malformed_review_candidate_fails_without_workspace_mutation() {
        let root = temporary_root("candidate");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer", "University X")
            .expect("create job")
            .data;
        let before = Application::workspace_status(&root)
            .expect("status before")
            .data
            .status
            .artifact_count;
        let error = Application::confirm_review_dispositions(
            &root,
            job.id.as_str(),
            &json!({"unexpected": true}),
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("malformed review disposition");
        assert_eq!(error.classify().code, ErrorCode::CandidateSchemaInvalid);
        let after = Application::workspace_status(&root)
            .expect("status after")
            .data
            .status
            .artifact_count;
        assert_eq!(before, after);
        std::fs::remove_dir_all(root).expect("remove workspace");
    }
}
