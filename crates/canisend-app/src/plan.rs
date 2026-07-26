use std::path::Path;

use canisend_contracts::{ApplicationPlanCandidate, ApplicationPlanRecord};
use canisend_store::PlanService;
use serde_json::Value;

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
};

impl Application {
    pub fn application_plan_template(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ApplicationPlanCandidate>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let plan = PlanService::new(&mut workspace.database, &workspace.blobs).template(&job_id)?;
        Ok(candidate_receipt(
            "plan.export",
            "available",
            "Prepared application plan candidate",
            plan,
        ))
    }

    pub fn current_application_plan(
        root: &Path,
        job_id: &str,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ApplicationPlanRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let plan = PlanService::new(&mut workspace.database, &workspace.blobs).current(&job_id)?;
        Ok(plan_receipt(
            "plan.show",
            "available",
            "Loaded current application plan",
            plan,
        ))
    }

    pub fn confirm_application_plan(
        root: &Path,
        job_id: &str,
        candidate: &Value,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<ApplicationPlanRecord>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let artifact = PlanService::new(&mut workspace.database, &workspace.blobs)
            .confirm(&job_id, candidate)?;
        let plan = PlanService::new(&mut workspace.database, &workspace.blobs).current(&job_id)?;
        Ok(plan_receipt(
            "plan.confirm",
            "confirmed",
            "Confirmed application plan",
            plan,
        )
        .with_artifacts([artifact]))
    }
}

fn candidate_receipt(
    operation: &'static str,
    status: &'static str,
    summary: &'static str,
    plan: ApplicationPlanCandidate,
) -> ActionReceipt<ApplicationPlanCandidate> {
    let documents = plan.documents.len();
    let blockers = plan.blockers.len();
    ActionReceipt::new(
        operation,
        status,
        format!("{summary}: {documents} document(s), {blockers} blocker(s)"),
        plan,
    )
}

fn plan_receipt(
    operation: &'static str,
    status: &'static str,
    summary: &'static str,
    plan: ApplicationPlanRecord,
) -> ActionReceipt<ApplicationPlanRecord> {
    let documents = plan.documents.len();
    let blockers = plan.blockers.len();
    ActionReceipt::new(
        operation,
        status,
        format!("{summary}: {documents} document(s), {blockers} blocker(s)"),
        plan,
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
            "canisend-app-plan-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn plan_ids_are_validated_before_workspace_access() {
        let missing = temporary_root("missing");
        for error in [
            Application::application_plan_template(
                &missing,
                "not-a-uuid",
                PrivateReadConsent::granted_by_user(),
            )
            .expect_err("invalid template job ID"),
            Application::current_application_plan(
                &missing,
                "not-a-uuid",
                PrivateReadConsent::granted_by_user(),
            )
            .expect_err("invalid current-plan job ID"),
        ] {
            assert!(matches!(error, ApplicationError::InvalidEntityId(_)));
        }
    }

    #[test]
    fn malformed_plan_candidate_fails_without_workspace_mutation() {
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
        let error = Application::confirm_application_plan(
            &root,
            job.id.as_str(),
            &json!({"unexpected": true}),
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("malformed plan");
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
