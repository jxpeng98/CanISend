use std::path::Path;

use canisend_contracts::{CriteriaSetRecord, EvidenceMatchSetRecord, ParsedJobRecord};
use canisend_store::{CriteriaService, MatchService};
use serde_json::Value;

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
};

impl Application {
    pub fn proposed_job_criteria(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ParsedJobRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let criteria =
            CriteriaService::new(&mut workspace.database, &workspace.blobs).proposed(&job_id)?;
        let count = criteria.criteria.len();
        Ok(ActionReceipt::new(
            "criteria.proposed",
            "available",
            format!("Loaded proposed criteria: {count} item(s)"),
            criteria,
        ))
    }

    pub fn job_criteria_template(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<CriteriaSetRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let criteria =
            CriteriaService::new(&mut workspace.database, &workspace.blobs).template(&job_id)?;
        Ok(criteria_receipt(
            "criteria.export",
            "available",
            "Prepared criteria candidate",
            criteria,
        ))
    }

    pub fn confirmed_job_criteria(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<CriteriaSetRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let criteria =
            CriteriaService::new(&mut workspace.database, &workspace.blobs).confirmed(&job_id)?;
        Ok(criteria_receipt(
            "criteria.show",
            "available",
            "Loaded confirmed criteria",
            criteria,
        ))
    }

    pub fn confirm_job_criteria(
        root: &Path,
        job_id: &str,
        candidate: &Value,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<CriteriaSetRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let artifact = CriteriaService::new(&mut workspace.database, &workspace.blobs)
            .confirm(&job_id, candidate)?;
        let criteria =
            CriteriaService::new(&mut workspace.database, &workspace.blobs).confirmed(&job_id)?;
        Ok(criteria_receipt(
            "criteria.confirm",
            "confirmed",
            "Confirmed job criteria",
            criteria,
        )
        .with_artifacts([artifact]))
    }

    pub fn current_evidence_matches(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<EvidenceMatchSetRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let matches =
            MatchService::new(&mut workspace.database, &workspace.blobs).current(&job_id)?;
        let count = matches.matches.len();
        Ok(ActionReceipt::new(
            "match.show",
            "available",
            format!("Loaded {count} evidence match(es)"),
            matches,
        ))
    }
}

fn criteria_receipt(
    operation: &'static str,
    status: &'static str,
    summary: &'static str,
    criteria: CriteriaSetRecord,
) -> ActionReceipt<CriteriaSetRecord> {
    let count = criteria.criteria.len();
    ActionReceipt::new(
        operation,
        status,
        format!("{summary}: {count} item(s)"),
        criteria,
    )
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
            "canisend-app-decision-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn decision_ids_are_validated_before_workspace_access() {
        let missing = temporary_root("missing");
        for error in [
            Application::proposed_job_criteria(
                &missing,
                "not-a-uuid",
                PrivateReadConsent::granted_by_user(),
            )
            .expect_err("invalid criteria job ID"),
            Application::current_evidence_matches(
                &missing,
                "not-a-uuid",
                PrivateReadConsent::granted_by_user(),
            )
            .expect_err("invalid match job ID"),
        ] {
            assert!(matches!(error, ApplicationError::InvalidEntityId(_)));
        }
    }

    #[test]
    fn malformed_criteria_candidate_fails_without_workspace_mutation() {
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
        let error = Application::confirm_job_criteria(
            &root,
            job.id.as_str(),
            &json!({"unexpected": true}),
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("malformed criteria");
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
