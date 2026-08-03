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

fn list_generic_applications_impl(
    request: GenericApplicationWorkspaceRequest,
) -> Result<ActionReceipt<Vec<StoredApplicationModelV3>>, DesktopCommandError> {
    Application::list_application_models_v3(&request.workspace)
        .map_err(DesktopCommandError::application)
}

fn show_generic_application_impl(
    request: GenericApplicationIdRequest,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, DesktopCommandError> {
    Application::application_flow_v3(&request.workspace, &request.application_id)
        .map_err(DesktopCommandError::application)
}

fn create_generic_application_impl(
    request: GenericApplicationCreateRequest,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, DesktopCommandError> {
    Application::create_application_flow_v3(&request.workspace, request.request)
        .map_err(DesktopCommandError::application)
}

fn plan_generic_application_impl(
    request: GenericApplicationPlanRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    Application::plan_application_flow_v3(
        &request.workspace,
        &request.application_id,
        request.request,
    )
    .map_err(DesktopCommandError::application)
}

fn compose_generic_application_impl(
    request: GenericApplicationComposeRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    Application::compose_application_flow_v3(
        &request.workspace,
        &request.application_id,
        request.request,
    )
    .map_err(DesktopCommandError::application)
}

fn approve_generic_application_impl(
    request: GenericApplicationApproveRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    let expected_revision = Revision::try_new(request.expected_revision)
        .map_err(|error| DesktopCommandError::state(error.to_string()))?;
    Application::approve_application_flow_v3(
        &request.workspace,
        &request.application_id,
        ApplicationFlowApproveRequestV3 { expected_revision },
    )
    .map_err(DesktopCommandError::application)
}

fn review_generic_application_impl(
    request: GenericApplicationReviewRequest,
) -> Result<ActionReceipt<ApplicationFlowReviewReadModelV3>, DesktopCommandError> {
    if !request.confirmed_private_read {
        return Err(DesktopCommandError::consent(
            "Confirm private local content access before reviewing Deliverable bodies.",
        ));
    }
    Application::review_application_flow_v3(
        &request.workspace,
        &request.application_id,
        Some(PrivateReadConsent::granted_by_user()),
    )
    .map_err(DesktopCommandError::application)
}

fn export_generic_application_impl(
    request: GenericApplicationExportRequest,
) -> Result<ActionReceipt<ApplicationFlowExportReadModelV3>, DesktopCommandError> {
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
    Application::export_application_flow_v3(
        &request.workspace,
        export,
        Some(PrivateExportConsent::granted_by_user()),
    )
    .map_err(DesktopCommandError::application)
}

#[tauri::command]
pub(crate) async fn list_generic_applications(
    request: GenericApplicationWorkspaceRequest,
) -> Result<ActionReceipt<Vec<StoredApplicationModelV3>>, DesktopCommandError> {
    run_worker(move || list_generic_applications_impl(request)).await
}

#[tauri::command]
pub(crate) async fn show_generic_application(
    request: GenericApplicationIdRequest,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, DesktopCommandError> {
    run_worker(move || show_generic_application_impl(request)).await
}

#[tauri::command]
pub(crate) async fn create_generic_application(
    request: GenericApplicationCreateRequest,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, DesktopCommandError> {
    run_worker(move || create_generic_application_impl(request)).await
}

#[tauri::command]
pub(crate) async fn plan_generic_application(
    request: GenericApplicationPlanRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    run_worker(move || plan_generic_application_impl(request)).await
}

#[tauri::command]
pub(crate) async fn compose_generic_application(
    request: GenericApplicationComposeRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    run_worker(move || compose_generic_application_impl(request)).await
}

#[tauri::command]
pub(crate) async fn approve_generic_application(
    request: GenericApplicationApproveRequest,
) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, DesktopCommandError> {
    run_worker(move || approve_generic_application_impl(request)).await
}

#[tauri::command]
pub(crate) async fn review_generic_application(
    request: GenericApplicationReviewRequest,
) -> Result<ActionReceipt<ApplicationFlowReviewReadModelV3>, DesktopCommandError> {
    run_worker(move || review_generic_application_impl(request)).await
}

#[tauri::command]
pub(crate) async fn export_generic_application(
    request: GenericApplicationExportRequest,
) -> Result<ActionReceipt<ApplicationFlowExportReadModelV3>, DesktopCommandError> {
    run_worker(move || export_generic_application_impl(request)).await
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use canisend_app::{
        ApplicationFlowDeliverableDraftV3, ApplicationFlowPlannedDeliverableV3,
        ApplicationFlowRequirementDraftV3, WorkspaceV3MigrationRequest,
    };
    use canisend_contracts::{
        ApplicationFieldValueV3, ExecutionMode, PlannedDeliverableDispositionV3,
        RequirementPriorityV3, WorkflowPackItemId,
    };

    use super::*;

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-desktop-generic-semantic-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    fn create_request(workspace: PathBuf) -> GenericApplicationCreateRequest {
        let source = "Provide one primary narrative.";
        GenericApplicationCreateRequest {
            workspace,
            request: ApplicationFlowCreateRequestV3 {
                title: "Desktop semantic parity Application".to_owned(),
                opportunity_metadata: BTreeMap::new(),
                application_metadata: BTreeMap::new(),
                source_text: source.to_owned(),
                requirements: vec![ApplicationFlowRequirementDraftV3 {
                    category: item("format"),
                    statement: source.to_owned(),
                    priority: RequirementPriorityV3::Mandatory,
                    start_byte: 0,
                    end_byte: u64::try_from(source.len()).expect("source length"),
                }],
            },
        }
    }

    #[test]
    fn pack_driven_desktop_commands_preserve_full_semantic_lifecycle_and_failures() {
        let workspace = temporary_root("workspace");
        Application::initialize_workspace_v3(&workspace).expect("generic Workspace");
        let created = create_generic_application_impl(create_request(workspace.clone()))
            .expect("create Application");
        let application_id = created.data.stored.snapshot.application.id.to_string();

        let listed = list_generic_applications_impl(GenericApplicationWorkspaceRequest {
            workspace: workspace.clone(),
        })
        .expect("list Applications");
        assert_eq!(listed.data.len(), 1);
        let shown = show_generic_application_impl(GenericApplicationIdRequest {
            workspace: workspace.clone(),
            application_id: application_id.clone(),
        })
        .expect("show Application");
        assert_eq!(shown.data.stored.snapshot.application.revision.get(), 1);

        let plan = |expected_revision| ApplicationFlowPlanRequestV3 {
            expected_revision: Revision::try_new(expected_revision).expect("revision"),
            decision: item("proceed"),
            deliverables: vec![ApplicationFlowPlannedDeliverableV3 {
                kind: item("primary-document"),
                disposition: PlannedDeliverableDispositionV3::Required,
                rationale: "Required by source".to_owned(),
                constraints: Vec::new(),
                execution_mode: Some(ExecutionMode::ManualImport),
            }],
        };
        assert!(
            plan_generic_application_impl(GenericApplicationPlanRequest {
                workspace: workspace.clone(),
                application_id: application_id.clone(),
                request: plan(2),
            })
            .is_err()
        );
        assert_eq!(
            Application::application_model_v3(&workspace, &application_id)
                .expect("unchanged after stale Plan")
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );
        plan_generic_application_impl(GenericApplicationPlanRequest {
            workspace: workspace.clone(),
            application_id: application_id.clone(),
            request: plan(1),
        })
        .expect("Plan");

        compose_generic_application_impl(GenericApplicationComposeRequest {
            workspace: workspace.clone(),
            application_id: application_id.clone(),
            request: ApplicationFlowComposeRequestV3 {
                expected_revision: Revision::try_new(2).expect("revision"),
                deliverables: vec![ApplicationFlowDeliverableDraftV3 {
                    kind: item("primary-document"),
                    title: "Desktop narrative".to_owned(),
                    media_type: "text/markdown".to_owned(),
                    content: "Synthetic desktop semantic parity content.".to_owned(),
                }],
            },
        })
        .expect("compose");

        let denied_review = review_generic_application_impl(GenericApplicationReviewRequest {
            workspace: workspace.clone(),
            application_id: application_id.clone(),
            confirmed_private_read: false,
        })
        .expect_err("review consent required");
        assert_eq!(denied_review.code, "consent-required");
        let reviewed = review_generic_application_impl(GenericApplicationReviewRequest {
            workspace: workspace.clone(),
            application_id: application_id.clone(),
            confirmed_private_read: true,
        })
        .expect("review");
        assert_eq!(reviewed.data.deliverables.len(), 1);

        assert!(
            approve_generic_application_impl(GenericApplicationApproveRequest {
                workspace: workspace.clone(),
                application_id: application_id.clone(),
                expected_revision: 2,
            })
            .is_err()
        );
        assert_eq!(
            Application::application_model_v3(&workspace, &application_id)
                .expect("unchanged after stale approval")
                .data
                .snapshot
                .application
                .revision
                .get(),
            3
        );
        approve_generic_application_impl(GenericApplicationApproveRequest {
            workspace: workspace.clone(),
            application_id: application_id.clone(),
            expected_revision: 3,
        })
        .expect("approve");

        let destination = format!("applications/{application_id}/exports/desktop-semantic-parity");
        assert!(
            export_generic_application_impl(GenericApplicationExportRequest {
                workspace: workspace.clone(),
                application_id: application_id.clone(),
                expected_revision: 3,
                destination: destination.clone(),
                confirmed_private_export: true,
            })
            .is_err()
        );
        assert!(!workspace.join(&destination).exists());
        let exported = export_generic_application_impl(GenericApplicationExportRequest {
            workspace: workspace.clone(),
            application_id,
            expected_revision: 4,
            destination,
            confirmed_private_export: true,
        })
        .expect("export");
        assert!(!exported.data.render.submission_performed);

        let academic = temporary_root("academic");
        Application::initialize_workspace(&academic).expect("academic Workspace");
        let before = Application::workspace_status(&academic)
            .expect("academic status before")
            .data
            .status;
        assert!(create_generic_application_impl(create_request(academic.clone())).is_err());
        assert_eq!(
            Application::workspace_status(&academic)
                .expect("academic status after")
                .data
                .status,
            before
        );

        let backup = temporary_root("academic-backup");
        Application::create_job(&academic, "Research Fellow", "Example University")
            .expect("legacy academic Application");
        let preview = Application::preview_workspace_v3_migration(&academic)
            .expect("migration preview")
            .data;
        Application::migrate_workspace_v3(
            &academic,
            WorkspaceV3MigrationRequest {
                expected_plan_sha256: preview.migration_plan_sha256,
                backup_destination: backup.clone(),
            },
        )
        .expect("migration");
        let source = "Applicants must submit a cover letter and academic CV.";
        let created = create_generic_application_impl(GenericApplicationCreateRequest {
            workspace: academic.clone(),
            request: ApplicationFlowCreateRequestV3 {
                title: "Academic desktop v3 fixture".to_owned(),
                opportunity_metadata: BTreeMap::from([(
                    item("institution"),
                    ApplicationFieldValueV3::ShortText("Example University".to_owned()),
                )]),
                application_metadata: BTreeMap::new(),
                source_text: source.to_owned(),
                requirements: vec![ApplicationFlowRequirementDraftV3 {
                    category: item("qualification"),
                    statement: source.to_owned(),
                    priority: RequirementPriorityV3::Mandatory,
                    start_byte: 0,
                    end_byte: u64::try_from(source.len()).expect("source length"),
                }],
            },
        })
        .expect("academic desktop create");
        let academic_application_id = created.data.stored.snapshot.application.id.to_string();
        assert_eq!(
            created.data.stored.snapshot.pack.id.as_str(),
            "org.canisend.academic-job"
        );
        plan_generic_application_impl(GenericApplicationPlanRequest {
            workspace: academic.clone(),
            application_id: academic_application_id.clone(),
            request: ApplicationFlowPlanRequestV3 {
                expected_revision: Revision::try_new(1).expect("revision"),
                decision: item("proceed"),
                deliverables: vec![
                    ApplicationFlowPlannedDeliverableV3 {
                        kind: item("cover-letter"),
                        disposition: PlannedDeliverableDispositionV3::Required,
                        rationale: "Required by source".to_owned(),
                        constraints: Vec::new(),
                        execution_mode: Some(ExecutionMode::HostAgent),
                    },
                    ApplicationFlowPlannedDeliverableV3 {
                        kind: item("cv"),
                        disposition: PlannedDeliverableDispositionV3::Required,
                        rationale: "Required by source".to_owned(),
                        constraints: Vec::new(),
                        execution_mode: Some(ExecutionMode::HostAgent),
                    },
                ],
            },
        })
        .expect("academic desktop Plan");
        let shown = show_generic_application_impl(GenericApplicationIdRequest {
            workspace: academic.clone(),
            application_id: academic_application_id,
        })
        .expect("academic desktop resume");
        assert_eq!(shown.data.stored.snapshot.application.revision.get(), 2);
        assert_eq!(shown.data.stages.len(), 10);

        fs::remove_dir_all(workspace).expect("remove generic Workspace");
        fs::remove_dir_all(academic).expect("remove academic Workspace");
        fs::remove_dir_all(backup).expect("remove academic backup");
    }
}
