use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, DocumentWorkspaceReadModel, PackageExportRequest,
    PrivateExportConsent, PrivateReadConsent, ProjectionCopyAsNewRequest, ProjectionReplaceRequest,
    RenderExportReadModel, RenderExportRequest, ReviewWorkspaceReadModel,
};
use canisend_contracts::{
    PackageExportManifestRecord, PackageManifestRecord, ProjectionReconcileRecord,
    RenderManifestRecord, ReviewFindingsRecord,
};
use serde::Deserialize;
use serde_json::Value;

use crate::commands::{DesktopCommandError, run_worker};

const MAX_REVIEW_CANDIDATE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeliveryJobRequest {
    workspace: PathBuf,
    job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivateDeliveryJobRequest {
    workspace: PathBuf,
    job_id: String,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewConfirmRequest {
    workspace: PathBuf,
    job_id: String,
    candidate: Value,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivateExportRequest {
    workspace: PathBuf,
    job_id: String,
    destination: String,
    confirmed_private_export: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectionRequest {
    workspace: PathBuf,
    job_id: String,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectionCopyRequest {
    workspace: PathBuf,
    job_id: String,
    path: String,
    destination: String,
}

fn require_private_read(confirmed: bool) -> Result<PrivateReadConsent, DesktopCommandError> {
    confirmed
        .then(PrivateReadConsent::granted_by_user)
        .ok_or_else(|| {
            DesktopCommandError::consent(
                "Confirm private workspace access before loading document or review bodies.",
            )
        })
}

fn require_private_export(confirmed: bool) -> Result<PrivateExportConsent, DesktopCommandError> {
    confirmed
        .then(PrivateExportConsent::granted_by_user)
        .ok_or_else(|| {
            DesktopCommandError::consent(
                "Confirm the scoped private export before writing application bodies.",
            )
        })
}

fn validate_review_candidate(candidate: &Value) -> Result<(), DesktopCommandError> {
    let size = serde_json::to_vec(candidate)
        .map_err(|error| DesktopCommandError::state(format!("Cannot encode candidate: {error}")))?
        .len();
    if size > MAX_REVIEW_CANDIDATE_BYTES {
        return Err(DesktopCommandError::state(format!(
            "Review candidate exceeds the {MAX_REVIEW_CANDIDATE_BYTES}-byte desktop limit"
        )));
    }
    Ok(())
}

fn document_workspace_impl(
    request: PrivateDeliveryJobRequest,
) -> Result<ActionReceipt<DocumentWorkspaceReadModel>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::document_workspace(&request.workspace, &request.job_id, consent)
        .map_err(DesktopCommandError::application)
}

fn review_workspace_impl(
    request: PrivateDeliveryJobRequest,
) -> Result<ActionReceipt<ReviewWorkspaceReadModel>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::review_workspace(&request.workspace, &request.job_id, consent)
        .map_err(DesktopCommandError::application)
}

fn confirm_review_impl(
    request: ReviewConfirmRequest,
) -> Result<ActionReceipt<ReviewFindingsRecord>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    validate_review_candidate(&request.candidate)?;
    Application::confirm_review_dispositions(
        &request.workspace,
        &request.job_id,
        &request.candidate,
        consent,
    )
    .map_err(DesktopCommandError::application)
}

fn export_package_impl(
    request: PrivateExportRequest,
) -> Result<ActionReceipt<PackageExportManifestRecord>, DesktopCommandError> {
    let consent = require_private_export(request.confirmed_private_export)?;
    let application_request = PackageExportRequest::try_new(&request.job_id, &request.destination)
        .map_err(DesktopCommandError::application)?;
    Application::export_package(&request.workspace, application_request, Some(consent))
        .map_err(DesktopCommandError::application)
}

fn export_render_impl(
    request: PrivateExportRequest,
) -> Result<ActionReceipt<RenderExportReadModel>, DesktopCommandError> {
    let consent = require_private_export(request.confirmed_private_export)?;
    let application_request = RenderExportRequest::try_new(&request.job_id, &request.destination)
        .map_err(DesktopCommandError::application)?;
    Application::export_render(&request.workspace, application_request, Some(consent))
        .map_err(DesktopCommandError::application)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn document_workspace(
    request: PrivateDeliveryJobRequest,
) -> Result<ActionReceipt<DocumentWorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || document_workspace_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn review_workspace(
    request: PrivateDeliveryJobRequest,
) -> Result<ActionReceipt<ReviewWorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || review_workspace_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn confirm_review(
    request: ReviewConfirmRequest,
) -> Result<ActionReceipt<ReviewFindingsRecord>, DesktopCommandError> {
    run_worker(move || confirm_review_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn check_package(
    request: DeliveryJobRequest,
) -> Result<ActionReceipt<PackageManifestRecord>, DesktopCommandError> {
    run_worker(move || {
        Application::check_package(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn current_package(
    request: DeliveryJobRequest,
) -> Result<ActionReceipt<PackageManifestRecord>, DesktopCommandError> {
    run_worker(move || {
        Application::current_package(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn export_package(
    request: PrivateExportRequest,
) -> Result<ActionReceipt<PackageExportManifestRecord>, DesktopCommandError> {
    run_worker(move || export_package_impl(request)).await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn current_package_export(
    request: DeliveryJobRequest,
) -> Result<ActionReceipt<PackageExportManifestRecord>, DesktopCommandError> {
    run_worker(move || {
        Application::current_package_export(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn reconcile_package(
    request: DeliveryJobRequest,
) -> Result<ActionReceipt<Vec<ProjectionReconcileRecord>>, DesktopCommandError> {
    run_worker(move || {
        Application::reconcile_package_projections(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn replace_package_projection(
    request: ProjectionRequest,
) -> Result<ActionReceipt<ProjectionReconcileRecord>, DesktopCommandError> {
    run_worker(move || {
        let application_request = ProjectionReplaceRequest::try_new(&request.job_id, &request.path)
            .map_err(DesktopCommandError::application)?;
        Application::replace_package_projection(&request.workspace, application_request)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn copy_package_projection(
    request: ProjectionCopyRequest,
) -> Result<ActionReceipt<ProjectionReconcileRecord>, DesktopCommandError> {
    run_worker(move || {
        let application_request = ProjectionCopyAsNewRequest::try_new(
            &request.job_id,
            &request.path,
            &request.destination,
        )
        .map_err(DesktopCommandError::application)?;
        Application::copy_package_projection_as_new(&request.workspace, application_request)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn build_render(
    request: DeliveryJobRequest,
) -> Result<ActionReceipt<RenderManifestRecord>, DesktopCommandError> {
    run_worker(move || {
        Application::build_render(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn current_render(
    request: DeliveryJobRequest,
) -> Result<ActionReceipt<RenderManifestRecord>, DesktopCommandError> {
    run_worker(move || {
        Application::current_render(&request.workspace, &request.job_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn export_render(
    request: PrivateExportRequest,
) -> Result<ActionReceipt<RenderExportReadModel>, DesktopCommandError> {
    run_worker(move || export_render_impl(request)).await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use canisend_app::Application;
    use serde_json::json;

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-desktop-delivery-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn private_delivery_operations_require_consent_before_workspace_access() {
        let missing = temporary_root("missing");
        let document = document_workspace_impl(PrivateDeliveryJobRequest {
            workspace: missing.clone(),
            job_id: "not-an-id".to_owned(),
            confirmed_private_read: false,
        })
        .expect_err("document read needs consent");
        assert_eq!(document.code, "consent-required");

        let review = confirm_review_impl(ReviewConfirmRequest {
            workspace: missing.clone(),
            job_id: "not-an-id".to_owned(),
            candidate: json!({"unexpected": true}),
            confirmed_private_read: false,
        })
        .expect_err("review confirmation needs consent");
        assert_eq!(review.code, "consent-required");

        let export = export_render_impl(PrivateExportRequest {
            workspace: missing,
            job_id: "not-an-id".to_owned(),
            destination: "jobs/not-an-id/rendered".to_owned(),
            confirmed_private_export: false,
        })
        .expect_err("render export needs consent");
        assert_eq!(export.code, "consent-required");
    }

    #[test]
    fn document_workspace_delegates_to_the_shared_application_facade() {
        let workspace = temporary_root("workspace");
        Application::initialize_workspace(&workspace).expect("initialize workspace");
        let job = Application::create_job(&workspace, "Lecturer", "University")
            .expect("create job")
            .data;
        let receipt = document_workspace_impl(PrivateDeliveryJobRequest {
            workspace: workspace.clone(),
            job_id: job.id.to_string(),
            confirmed_private_read: true,
        })
        .expect("load document workspace");
        assert_eq!(receipt.operation, "document.workspace");
        assert!(receipt.data.documents.is_empty());
        std::fs::remove_dir_all(workspace).expect("remove workspace");
    }

    #[test]
    fn oversized_review_candidate_is_rejected_at_the_desktop_boundary() {
        let candidate = json!({"value": "x".repeat(MAX_REVIEW_CANDIDATE_BYTES)});
        assert!(validate_review_candidate(&candidate).is_err());
    }
}
