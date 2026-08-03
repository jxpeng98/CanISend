use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, ApplicationFlowApproveRequestV3, ApplicationFlowCommitReadModelV3,
    ApplicationFlowComposeRequestV3, ApplicationFlowCreateRequestV3,
    ApplicationFlowExportReadModelV3, ApplicationFlowExportRequestV3, ApplicationFlowPlanRequestV3,
    ApplicationFlowReadModelV3, ApplicationFlowReviewReadModelV3, PrivateExportConsent,
    PrivateReadConsent, StoredApplicationModelV3,
};
use canisend_contracts::Revision;
use serde::Deserialize;

use crate::commands::{DesktopCommandError, run_worker};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenericApplicationWorkspaceRequest {
    workspace: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenericApplicationIdRequest {
    workspace: PathBuf,
    application_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenericApplicationCreateRequest {
    workspace: PathBuf,
    request: ApplicationFlowCreateRequestV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenericApplicationPlanRequest {
    workspace: PathBuf,
    application_id: String,
    request: ApplicationFlowPlanRequestV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenericApplicationComposeRequest {
    workspace: PathBuf,
    application_id: String,
    request: ApplicationFlowComposeRequestV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenericApplicationApproveRequest {
    workspace: PathBuf,
    application_id: String,
    expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenericApplicationReviewRequest {
    workspace: PathBuf,
    application_id: String,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenericApplicationExportRequest {
    workspace: PathBuf,
    application_id: String,
    expected_revision: u64,
    destination: String,
    confirmed_private_export: bool,
}

#[tauri::command]
pub(crate) async fn list_generic_applications(
    request: GenericApplicationWorkspaceRequest,
) -> Result<ActionReceipt<Vec<StoredApplicationModelV3>>, DesktopCommandError> {
    run_worker(move || {
        Application::list_application_models_v3(&request.workspace)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn show_generic_application(
    request: GenericApplicationIdRequest,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, DesktopCommandError> {
    run_worker(move || {
        Application::generic_application_flow_v3(&request.workspace, &request.application_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_generic_application(
    request: GenericApplicationCreateRequest,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, DesktopCommandError> {
    run_worker(move || {
        Application::create_generic_application_v3(&request.workspace, request.request)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn plan_generic_application(
    request: GenericApplicationPlanRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    run_worker(move || {
        Application::plan_generic_application_v3(
            &request.workspace,
            &request.application_id,
            request.request,
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn compose_generic_application(
    request: GenericApplicationComposeRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    run_worker(move || {
        Application::compose_generic_application_v3(
            &request.workspace,
            &request.application_id,
            request.request,
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn approve_generic_application(
    request: GenericApplicationApproveRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    run_worker(move || {
        let expected_revision = Revision::try_new(request.expected_revision)
            .map_err(|error| DesktopCommandError::state(error.to_string()))?;
        Application::approve_generic_application_v3(
            &request.workspace,
            &request.application_id,
            ApplicationFlowApproveRequestV3 { expected_revision },
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn review_generic_application(
    request: GenericApplicationReviewRequest,
) -> Result<ActionReceipt<ApplicationFlowReviewReadModelV3>, DesktopCommandError> {
    run_worker(move || {
        if !request.confirmed_private_read {
            return Err(DesktopCommandError::consent(
                "Confirm private local content access before reviewing Deliverable bodies.",
            ));
        }
        Application::review_generic_application_v3(
            &request.workspace,
            &request.application_id,
            Some(PrivateReadConsent::granted_by_user()),
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn export_generic_application(
    request: GenericApplicationExportRequest,
) -> Result<ActionReceipt<ApplicationFlowExportReadModelV3>, DesktopCommandError> {
    run_worker(move || {
        if !request.confirmed_private_export {
            return Err(DesktopCommandError::consent(
                "Confirm the private local export destination before writing Deliverables and PDFs.",
            ));
        }
        let export = ApplicationFlowExportRequestV3::try_new(
            &request.application_id,
            request.expected_revision,
            &request.destination,
        )
        .map_err(DesktopCommandError::application)?;
        Application::export_generic_application_v3(
            &request.workspace,
            export,
            Some(PrivateExportConsent::granted_by_user()),
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}
