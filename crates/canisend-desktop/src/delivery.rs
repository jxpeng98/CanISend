use std::{
    fs,
    path::{Path, PathBuf},
};

use canisend_app::{
    ActionReceipt, Application, DocumentWorkspaceReadModel, PackageExportRequest,
    PrivateExportConsent, PrivateReadConsent, ProjectionCopyAsNewRequest, ProjectionReplaceRequest,
    RenderExportReadModel, RenderExportRequest, ReviewWorkspaceReadModel,
};
use canisend_contracts::{
    DocumentKind, PackageExportManifestRecord, PackageManifestRecord, ProjectionReconcileRecord,
    RenderManifestRecord, ReviewFindingsRecord,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenderPreviewRequest {
    workspace: PathBuf,
    job_id: String,
    kind: DocumentKind,
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
pub(crate) struct OpenRenderRequest {
    workspace: PathBuf,
    job_id: String,
    destination: String,
    kind: DocumentKind,
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

fn validated_exported_pdf_path(
    workspace: &Path,
    export: &RenderExportReadModel,
    kind: DocumentKind,
) -> Result<PathBuf, DesktopCommandError> {
    let document = export
        .render_manifest
        .documents
        .iter()
        .find(|document| document.kind == kind)
        .ok_or_else(|| {
            DesktopCommandError::state("The current render does not contain the requested PDF")
        })?;
    let expected_relative = format!(
        "{}/{}.pdf",
        export.destination.as_str(),
        document_kind_slug(kind)
    );
    let relative = export
        .files
        .iter()
        .find(|path| path.as_str() == expected_relative)
        .ok_or_else(|| {
            DesktopCommandError::state(
                "The render export did not report the requested job-scoped PDF",
            )
        })?;

    let workspace_metadata = fs::symlink_metadata(workspace).map_err(|error| {
        DesktopCommandError::state(format!("Cannot inspect the selected workspace: {error}"))
    })?;
    if workspace_metadata.file_type().is_symlink() || !workspace_metadata.is_dir() {
        return Err(DesktopCommandError::state(
            "The selected workspace must be a real directory",
        ));
    }

    let exported_path = workspace.join(relative.as_str());
    let exported_metadata = fs::symlink_metadata(&exported_path).map_err(|error| {
        DesktopCommandError::state(format!("Cannot inspect the exported PDF: {error}"))
    })?;
    if exported_metadata.file_type().is_symlink() || !exported_metadata.is_file() {
        return Err(DesktopCommandError::state(
            "The exported PDF must be a regular file",
        ));
    }
    if exported_metadata.len() != document.byte_count {
        return Err(DesktopCommandError::state(
            "The exported PDF size no longer matches the validated render",
        ));
    }

    let canonical_workspace = fs::canonicalize(workspace).map_err(|error| {
        DesktopCommandError::state(format!("Cannot resolve the selected workspace: {error}"))
    })?;
    let canonical_export = fs::canonicalize(&exported_path).map_err(|error| {
        DesktopCommandError::state(format!("Cannot resolve the exported PDF: {error}"))
    })?;
    if !canonical_export.starts_with(&canonical_workspace) {
        return Err(DesktopCommandError::state(
            "The exported PDF resolved outside the selected workspace",
        ));
    }

    let bytes = fs::read(&canonical_export).map_err(|error| {
        DesktopCommandError::state(format!("Cannot verify the exported PDF: {error}"))
    })?;
    let actual_sha256 = hex::encode(Sha256::digest(&bytes));
    if actual_sha256 != document.pdf_artifact.sha256.as_str() {
        return Err(DesktopCommandError::state(
            "The exported PDF no longer matches the validated render",
        ));
    }
    Ok(canonical_export)
}

fn export_render_and_open_impl<F>(
    request: OpenRenderRequest,
    opener: F,
) -> Result<ActionReceipt<RenderExportReadModel>, DesktopCommandError>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    let workspace = request.workspace.clone();
    let kind = request.kind;
    let mut receipt = export_render_impl(PrivateExportRequest {
        workspace,
        job_id: request.job_id,
        destination: request.destination,
        confirmed_private_export: request.confirmed_private_export,
    })?;
    let path = validated_exported_pdf_path(&request.workspace, &receipt.data, kind)?;
    opener(&path).map_err(|error| {
        DesktopCommandError::system_open(format!(
            "The PDF was exported, but the system viewer could not be opened: {error}"
        ))
    })?;
    receipt.status = "opened".to_owned();
    receipt.summary = format!(
        "Exported validated render files and opened the {} PDF in the system viewer",
        document_kind_slug(kind)
    );
    Ok(receipt)
}

const fn document_kind_slug(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::CoverLetter => "cover-letter",
        DocumentKind::ResearchStatement => "research-statement",
        DocumentKind::TeachingStatement => "teaching-statement",
        DocumentKind::Cv => "cv",
    }
}

fn preview_render_impl(
    request: RenderPreviewRequest,
) -> Result<ActionReceipt<canisend_app::RenderPreviewReadModel>, DesktopCommandError> {
    let consent = require_private_read(request.confirmed_private_read)?;
    Application::preview_render(&request.workspace, &request.job_id, request.kind, consent)
        .map_err(DesktopCommandError::application)
}

#[tauri::command]
pub(crate) async fn document_workspace(
    request: PrivateDeliveryJobRequest,
) -> Result<ActionReceipt<DocumentWorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || document_workspace_impl(request)).await
}

#[tauri::command]
pub(crate) async fn review_workspace(
    request: PrivateDeliveryJobRequest,
) -> Result<ActionReceipt<ReviewWorkspaceReadModel>, DesktopCommandError> {
    run_worker(move || review_workspace_impl(request)).await
}

#[tauri::command]
pub(crate) async fn confirm_review(
    request: ReviewConfirmRequest,
) -> Result<ActionReceipt<ReviewFindingsRecord>, DesktopCommandError> {
    run_worker(move || confirm_review_impl(request)).await
}

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

#[tauri::command]
pub(crate) async fn export_package(
    request: PrivateExportRequest,
) -> Result<ActionReceipt<PackageExportManifestRecord>, DesktopCommandError> {
    run_worker(move || export_package_impl(request)).await
}

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

#[tauri::command]
pub(crate) async fn preview_render(
    request: RenderPreviewRequest,
) -> Result<tauri::ipc::Response, DesktopCommandError> {
    let receipt = run_worker(move || preview_render_impl(request)).await?;
    Ok(tauri::ipc::Response::new(receipt.data.pdf_bytes))
}

#[tauri::command]
pub(crate) async fn export_render(
    request: PrivateExportRequest,
) -> Result<ActionReceipt<RenderExportReadModel>, DesktopCommandError> {
    run_worker(move || export_render_impl(request)).await
}

#[tauri::command]
pub(crate) async fn export_render_and_open(
    request: OpenRenderRequest,
) -> Result<ActionReceipt<RenderExportReadModel>, DesktopCommandError> {
    run_worker(move || {
        export_render_and_open_impl(request, |path| {
            open::that_detached(path).map_err(|error| error.to_string())
        })
    })
    .await
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

        let preview = preview_render_impl(RenderPreviewRequest {
            workspace: missing.clone(),
            job_id: "not-an-id".to_owned(),
            kind: DocumentKind::Cv,
            confirmed_private_read: false,
        })
        .expect_err("render preview needs consent");
        assert_eq!(preview.code, "consent-required");

        let export = export_render_impl(PrivateExportRequest {
            workspace: missing,
            job_id: "not-an-id".to_owned(),
            destination: "jobs/not-an-id/rendered".to_owned(),
            confirmed_private_export: false,
        })
        .expect_err("render export needs consent");
        assert_eq!(export.code, "consent-required");

        let open = export_render_and_open_impl(
            OpenRenderRequest {
                workspace: temporary_root("open-missing"),
                job_id: "not-an-id".to_owned(),
                destination: "jobs/not-an-id/rendered".to_owned(),
                kind: DocumentKind::Cv,
                confirmed_private_export: false,
            },
            |_| panic!("viewer must not open without private-export consent"),
        )
        .expect_err("system viewer needs export consent");
        assert_eq!(open.code, "consent-required");
    }

    #[test]
    fn system_viewer_path_must_match_the_exact_exported_pdf() {
        let workspace = temporary_root("viewer-path");
        let job_id = "019f2f55-7c00-7000-8000-000000000101";
        let directory = workspace.join(format!("jobs/{job_id}/rendered"));
        fs::create_dir_all(&directory).expect("create export fixture");
        let bytes = b"%PDF exact validated fixture";
        let sha256 = hex::encode(Sha256::digest(bytes));
        let pdf_path = directory.join("cv.pdf");
        fs::write(&pdf_path, bytes).expect("write PDF fixture");
        let export: RenderExportReadModel = serde_json::from_value(json!({
            "render_manifest": {
                "id": "019f2f55-7c00-7000-8000-000000000800",
                "job_id": job_id,
                "package_artifact": {
                    "kind": "package-manifest",
                    "id": "019f2f55-7c00-7000-8000-000000000700",
                    "revision": 1,
                    "sha256": "a".repeat(64)
                },
                "documents": [{
                    "kind": "cv",
                    "document_artifact": {
                        "kind": "cv",
                        "id": "019f2f55-7c00-7000-8000-000000000701",
                        "revision": 1,
                        "sha256": "b".repeat(64)
                    },
                    "typst_artifact": {
                        "kind": "typst-source",
                        "id": "019f2f55-7c00-7000-8000-000000000702",
                        "revision": 1,
                        "sha256": "c".repeat(64)
                    },
                    "pdf_artifact": {
                        "kind": "pdf",
                        "id": "019f2f55-7c00-7000-8000-000000000703",
                        "revision": 1,
                        "sha256": sha256
                    },
                    "page_count": 1,
                    "byte_count": bytes.len(),
                    "warning_count": 0,
                    "elapsed_millis": 1
                }],
                "rendered_at": "2026-08-01T12:00:00Z",
                "submission_performed": false,
                "revision": 1
            },
            "destination": format!("jobs/{job_id}/rendered"),
            "files": [
                format!("jobs/{job_id}/rendered/cv.pdf"),
                format!("jobs/{job_id}/rendered/render-manifest.json")
            ],
            "submission_performed": false
        }))
        .expect("valid render export fixture");

        assert_eq!(
            validated_exported_pdf_path(&workspace, &export, DocumentKind::Cv)
                .expect("exact exported PDF"),
            fs::canonicalize(&pdf_path).expect("canonical PDF fixture")
        );

        let mut changed = bytes.to_vec();
        changed[0] ^= 1;
        fs::write(&pdf_path, changed).expect("corrupt PDF fixture");
        assert!(validated_exported_pdf_path(&workspace, &export, DocumentKind::Cv).is_err());
        fs::remove_dir_all(workspace).expect("remove export fixture");
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
