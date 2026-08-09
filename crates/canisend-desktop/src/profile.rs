use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, PrivateReadConsent, ProfileInitializationReadModel,
    ProfileSourceImportReadModel, ProfileSourceListReadModel,
};
use canisend_contracts::{
    ApplicationPlanCandidate, ApplicationPlanRecord, CriteriaSetRecord, EvidenceCatalogRecord,
    EvidenceMatchSetRecord, PrivacyClassification,
};
use serde::Deserialize;
use serde_json::Value;

use crate::commands::{DesktopCommandError, run_worker};

const MAX_CANDIDATE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileWorkspaceRequest {
    workspace: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileSourceImportRequest {
    workspace: PathBuf,
    source: PathBuf,
    sensitivity: PrivacyClassification,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileInitializationRequest {
    workspace: PathBuf,
    markdown: String,
    sensitivity: PrivacyClassification,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivateJobRequest {
    workspace: PathBuf,
    job_id: String,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CandidateConfirmRequest {
    workspace: PathBuf,
    job_id: String,
    candidate: Value,
    confirmed_private_read: bool,
}

fn require_private_read(confirmed: bool) -> Result<PrivateReadConsent, DesktopCommandError> {
    confirmed
        .then(PrivateReadConsent::granted_by_user)
        .ok_or_else(|| {
            DesktopCommandError::consent(
                "Confirm private workspace access before loading or confirming application data.",
            )
        })
}

fn validate_candidate(candidate: &Value) -> Result<(), DesktopCommandError> {
    let size = serde_json::to_vec(candidate)
        .map_err(|error| DesktopCommandError::state(format!("Cannot encode candidate: {error}")))?
        .len();
    if size > MAX_CANDIDATE_BYTES {
        return Err(DesktopCommandError::state(format!(
            "Candidate exceeds the {MAX_CANDIDATE_BYTES}-byte desktop limit"
        )));
    }
    Ok(())
}

fn import_profile_source_impl(
    request: ProfileSourceImportRequest,
) -> Result<ActionReceipt<ProfileSourceImportReadModel>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::import_profile_source(
        &request.workspace,
        &request.source,
        request.sensitivity,
        consent,
    )
    .map_err(DesktopCommandError::application)
}

fn list_profile_sources_impl(
    request: ProfileWorkspaceRequest,
) -> Result<ActionReceipt<ProfileSourceListReadModel>, DesktopCommandError> {
    Application::list_profile_sources_v4(&request.workspace)
        .map_err(DesktopCommandError::application)
}

fn initialize_profile_impl(
    request: ProfileInitializationRequest,
) -> Result<ActionReceipt<ProfileInitializationReadModel>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::initialize_profile(
        &request.workspace,
        &request.markdown,
        request.sensitivity,
        consent,
    )
    .map_err(DesktopCommandError::application)
}

fn profile_evidence_template_impl(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<EvidenceCatalogRecord>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::profile_evidence_template(&request.workspace, &request.job_id, consent)
        .map_err(DesktopCommandError::application)
}

fn confirm_profile_evidence_impl(
    request: CandidateConfirmRequest,
) -> Result<ActionReceipt<EvidenceCatalogRecord>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    validate_candidate(&request.candidate)?;
    Application::confirm_profile_evidence(
        &request.workspace,
        &request.job_id,
        &request.candidate,
        consent,
    )
    .map_err(DesktopCommandError::application)
}

fn criteria_template_impl(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<CriteriaSetRecord>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::job_criteria_template(&request.workspace, &request.job_id, consent)
        .map_err(DesktopCommandError::application)
}

fn confirm_criteria_impl(
    request: CandidateConfirmRequest,
) -> Result<ActionReceipt<CriteriaSetRecord>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    validate_candidate(&request.candidate)?;
    Application::confirm_job_criteria(
        &request.workspace,
        &request.job_id,
        &request.candidate,
        consent,
    )
    .map_err(DesktopCommandError::application)
}

fn current_matches_impl(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<EvidenceMatchSetRecord>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::current_evidence_matches(&request.workspace, &request.job_id, consent)
        .map_err(DesktopCommandError::application)
}

fn plan_template_impl(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<ApplicationPlanCandidate>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::application_plan_template(&request.workspace, &request.job_id, consent)
        .map_err(DesktopCommandError::application)
}

fn current_plan_impl(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<ApplicationPlanRecord>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::current_application_plan(&request.workspace, &request.job_id, consent)
        .map_err(DesktopCommandError::application)
}

fn confirm_plan_impl(
    request: CandidateConfirmRequest,
) -> Result<ActionReceipt<ApplicationPlanRecord>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    validate_candidate(&request.candidate)?;
    Application::confirm_application_plan(
        &request.workspace,
        &request.job_id,
        &request.candidate,
        consent,
    )
    .map_err(DesktopCommandError::application)
}

#[tauri::command]
pub(crate) async fn list_profile_sources(
    request: ProfileWorkspaceRequest,
) -> Result<ActionReceipt<ProfileSourceListReadModel>, DesktopCommandError> {
    run_worker(move || list_profile_sources_impl(request)).await
}

#[tauri::command]
pub(crate) async fn import_profile_source(
    request: ProfileSourceImportRequest,
) -> Result<ActionReceipt<ProfileSourceImportReadModel>, DesktopCommandError> {
    run_worker(move || import_profile_source_impl(request)).await
}

#[tauri::command]
pub(crate) async fn initialize_profile(
    request: ProfileInitializationRequest,
) -> Result<ActionReceipt<ProfileInitializationReadModel>, DesktopCommandError> {
    run_worker(move || initialize_profile_impl(request)).await
}

#[tauri::command]
pub(crate) async fn profile_evidence_template(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<EvidenceCatalogRecord>, DesktopCommandError> {
    run_worker(move || profile_evidence_template_impl(request)).await
}

#[tauri::command]
pub(crate) async fn confirm_profile_evidence(
    request: CandidateConfirmRequest,
) -> Result<ActionReceipt<EvidenceCatalogRecord>, DesktopCommandError> {
    run_worker(move || confirm_profile_evidence_impl(request)).await
}

#[tauri::command]
pub(crate) async fn criteria_template(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<CriteriaSetRecord>, DesktopCommandError> {
    run_worker(move || criteria_template_impl(request)).await
}

#[tauri::command]
pub(crate) async fn confirm_criteria(
    request: CandidateConfirmRequest,
) -> Result<ActionReceipt<CriteriaSetRecord>, DesktopCommandError> {
    run_worker(move || confirm_criteria_impl(request)).await
}

#[tauri::command]
pub(crate) async fn current_matches(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<EvidenceMatchSetRecord>, DesktopCommandError> {
    run_worker(move || current_matches_impl(request)).await
}

#[tauri::command]
pub(crate) async fn plan_template(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<ApplicationPlanCandidate>, DesktopCommandError> {
    run_worker(move || plan_template_impl(request)).await
}

#[tauri::command]
pub(crate) async fn current_plan(
    request: PrivateJobRequest,
) -> Result<ActionReceipt<ApplicationPlanRecord>, DesktopCommandError> {
    run_worker(move || current_plan_impl(request)).await
}

#[tauri::command]
pub(crate) async fn confirm_plan(
    request: CandidateConfirmRequest,
) -> Result<ActionReceipt<ApplicationPlanRecord>, DesktopCommandError> {
    run_worker(move || confirm_plan_impl(request)).await
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_app::Application;
    use serde_json::json;

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-desktop-profile-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn private_profile_and_candidate_calls_require_consent_before_io() {
        let missing = temporary_root("missing");
        let source = temporary_root("source").with_extension("md");
        let import = import_profile_source_impl(ProfileSourceImportRequest {
            workspace: missing.clone(),
            source,
            sensitivity: PrivacyClassification::PrivateLocal,
            confirmed_private_read: false,
        })
        .expect_err("profile import needs consent");
        assert_eq!(import.code, "consent-required");

        let initialize = initialize_profile_impl(ProfileInitializationRequest {
            workspace: missing.clone(),
            markdown: "# Profile\n\nResearcher.\n".to_owned(),
            sensitivity: PrivacyClassification::PrivateLocal,
            confirmed_private_read: false,
        })
        .expect_err("profile initialization needs consent");
        assert_eq!(initialize.code, "consent-required");

        let candidate = confirm_criteria_impl(CandidateConfirmRequest {
            workspace: missing,
            job_id: "not-an-id".to_owned(),
            candidate: json!({"unexpected": true}),
            confirmed_private_read: false,
        })
        .expect_err("candidate confirmation needs consent");
        assert_eq!(candidate.code, "consent-required");
    }

    #[test]
    fn profile_source_commands_delegate_to_the_shared_application_facade() {
        let workspace = temporary_root("workspace");
        let source = temporary_root("source").with_extension("txt");
        fs::write(&source, "Teaching and research experience.").expect("write profile source");
        Application::initialize_workspace_v4(&workspace).expect("initialize Workspace v4");

        let imported = import_profile_source_impl(ProfileSourceImportRequest {
            workspace: workspace.clone(),
            source: source.clone(),
            sensitivity: PrivacyClassification::PrivateLocal,
            confirmed_private_read: true,
        })
        .expect("import profile source");
        assert_eq!(imported.operation, "profile.source.add");
        let listed = list_profile_sources_impl(ProfileWorkspaceRequest {
            workspace: workspace.clone(),
        })
        .expect("list profile sources");
        assert_eq!(listed.operation, "profile-source.list");
        assert_eq!(listed.data.sources.len(), 1);
        assert_eq!(listed.data.sources[0].id, imported.data.source.id);

        fs::remove_dir_all(workspace).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
    }

    #[test]
    fn oversized_candidate_is_rejected_at_the_desktop_boundary() {
        let candidate = json!({"value": "x".repeat(MAX_CANDIDATE_BYTES)});
        assert!(validate_candidate(&candidate).is_err());
    }
}
