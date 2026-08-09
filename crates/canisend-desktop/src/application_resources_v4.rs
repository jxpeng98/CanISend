use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, DeliverableListReadModelV4, DeliverableShowReadModelV4,
    ExportListReadModelV4, ExportShowReadModelV4, PlanShowReadModelV4, RequirementListReadModelV4,
    RequirementShowReadModelV4,
};
use serde::Deserialize;

use crate::commands::{DesktopCommandError, run_worker};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationResourceListRequestV4 {
    workspace: PathBuf,
    application_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationRequirementShowRequestV4 {
    workspace: PathBuf,
    application_id: String,
    requirement_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationDeliverableShowRequestV4 {
    workspace: PathBuf,
    application_id: String,
    deliverable_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationExportShowRequestV4 {
    workspace: PathBuf,
    application_id: String,
    destination: String,
}

#[tauri::command]
pub(crate) async fn requirement_list(
    request: ApplicationResourceListRequestV4,
) -> Result<ActionReceipt<RequirementListReadModelV4>, DesktopCommandError> {
    run_worker(move || {
        Application::list_requirements_v4(&request.workspace, &request.application_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn requirement_show(
    request: ApplicationRequirementShowRequestV4,
) -> Result<ActionReceipt<RequirementShowReadModelV4>, DesktopCommandError> {
    run_worker(move || {
        Application::show_requirement_v4(
            &request.workspace,
            &request.application_id,
            &request.requirement_id,
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn plan_show(
    request: ApplicationResourceListRequestV4,
) -> Result<ActionReceipt<PlanShowReadModelV4>, DesktopCommandError> {
    run_worker(move || {
        Application::show_plan_v4(&request.workspace, &request.application_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn deliverable_list(
    request: ApplicationResourceListRequestV4,
) -> Result<ActionReceipt<DeliverableListReadModelV4>, DesktopCommandError> {
    run_worker(move || {
        Application::list_deliverables_v4(&request.workspace, &request.application_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn deliverable_show(
    request: ApplicationDeliverableShowRequestV4,
) -> Result<ActionReceipt<DeliverableShowReadModelV4>, DesktopCommandError> {
    run_worker(move || {
        Application::show_deliverable_v4(
            &request.workspace,
            &request.application_id,
            &request.deliverable_id,
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn export_list(
    request: ApplicationResourceListRequestV4,
) -> Result<ActionReceipt<ExportListReadModelV4>, DesktopCommandError> {
    run_worker(move || {
        Application::list_exports_v4(&request.workspace, &request.application_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn export_show(
    request: ApplicationExportShowRequestV4,
) -> Result<ActionReceipt<ExportShowReadModelV4>, DesktopCommandError> {
    run_worker(move || {
        Application::show_export_v4(
            &request.workspace,
            &request.application_id,
            &request.destination,
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}
