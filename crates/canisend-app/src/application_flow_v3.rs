use std::path::Path;

use canisend_contracts::{ApplicationId, NextAction, Revision, SafeRelativePath};
use canisend_store::ApplicationFlowServiceV3;
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateExportConsent,
    application::open_workspace, built_in_generic_application_pack,
};

pub use canisend_store::{
    APPLICATION_FLOW_EXPORT_FORMAT_V3, ApplicationFlowApproveRequestV3,
    ApplicationFlowCommitReadModelV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowCreateRequestV3, ApplicationFlowDeliverableDraftV3,
    ApplicationFlowExportManifestV3, ApplicationFlowExportReadModelV3,
    ApplicationFlowPlanRequestV3, ApplicationFlowPlannedDeliverableV3, ApplicationFlowReadModelV3,
    ApplicationFlowRenderedDeliverableV3, ApplicationFlowRequirementDraftV3,
    ApplicationFlowStageReadModelV3, ApplicationFlowStageStateV3,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowExportRequestV3 {
    pub application_id: ApplicationId,
    pub expected_revision: Revision,
    pub destination: SafeRelativePath,
}

impl ApplicationFlowExportRequestV3 {
    pub fn try_new(
        application_id: &str,
        expected_revision: u64,
        destination: &str,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            application_id: parse_application_id(application_id)?,
            expected_revision: Revision::try_new(expected_revision)
                .map_err(|error| ApplicationError::InvalidInput(error.to_string()))?,
            destination: SafeRelativePath::try_new(destination)
                .map_err(|error| ApplicationError::InvalidInput(error.to_string()))?,
        })
    }
}

impl Application {
    pub fn create_generic_application_v3(
        workspace_root: &Path,
        request: ApplicationFlowCreateRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        let pack = built_in_generic_application_pack()?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create(&pack, request)?;
        Ok(ActionReceipt::new(
            "application-flow-v3.create",
            "created",
            "Created a Pack-bound generic Application intake",
            result,
        ))
    }

    pub fn plan_generic_application_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowPlanRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let pack = built_in_generic_application_pack()?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .confirm_requirements_and_plan(&pack, &application_id, request)?;
        Ok(ActionReceipt::new(
            "application-flow-v3.plan",
            "confirmed",
            "Confirmed Requirements and committed the user-approved Plan",
            result,
        ))
    }

    pub fn compose_generic_application_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowComposeRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let pack = built_in_generic_application_pack()?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .compose(&pack, &application_id, request)?;
        Ok(ActionReceipt::new(
            "application-flow-v3.compose",
            "review-required",
            "Committed Pack-qualified Deliverables for explicit review",
            result,
        ))
    }

    pub fn approve_generic_application_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowApproveRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let pack = built_in_generic_application_pack()?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .approve(&pack, &application_id, request)?;
        Ok(ActionReceipt::new(
            "application-flow-v3.approve",
            "approved",
            "Recorded explicit user approval for all current Deliverables",
            result,
        ))
    }

    pub fn export_generic_application_v3(
        workspace_root: &Path,
        request: ApplicationFlowExportRequestV3,
        consent: Option<PrivateExportConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowExportReadModelV3>, ApplicationError> {
        if consent.is_none() {
            return Err(ApplicationError::ConsentRequired {
                message: "Export writes private Pack-bound Deliverable bodies and PDFs".to_owned(),
                remediation: NextAction {
                    action: "grant private export consent".to_owned(),
                    description:
                        "Review the destination and explicitly authorize this local-only export"
                            .to_owned(),
                },
            });
        }
        let pack = built_in_generic_application_pack()?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .export(
                    &pack,
                    &request.application_id,
                    request.expected_revision,
                    &request.destination,
                )?;
        Ok(ActionReceipt::new(
            "application-flow-v3.export",
            "exported",
            format!(
                "Exported {} validated PDF(s); submission performed: no",
                result.render.documents.len()
            ),
            result,
        ))
    }
}

fn parse_application_id(value: &str) -> Result<ApplicationId, ApplicationError> {
    ApplicationId::try_new(value)
        .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use canisend_contracts::{
        ApplicationFieldValueV3, ExecutionMode, PlannedDeliverableDispositionV3,
        RequirementPriorityV3, WorkflowPackItemId,
    };
    use canisend_io::validate_rendered_pdf;

    use super::*;

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-generic-flow-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    #[test]
    fn generic_fixture_completes_two_deliverables_without_submission() {
        let root = temporary_root("complete");
        let initialized = Application::initialize_workspace_v3(&root).expect("v3 workspace");
        assert_eq!(
            initialized.data.status.workspace_format,
            canisend_contracts::WORKSPACE_V3_FORMAT
        );

        let source = "Applicants must provide a project narrative and a budget appendix.";
        let created = Application::create_generic_application_v3(
            &root,
            ApplicationFlowCreateRequestV3 {
                title: "Synthetic community project".to_owned(),
                opportunity_metadata: BTreeMap::from([
                    (
                        item("organization"),
                        ApplicationFieldValueV3::ShortText("Example Foundation".to_owned()),
                    ),
                    (
                        item("reference"),
                        ApplicationFieldValueV3::ShortText("SYN-001".to_owned()),
                    ),
                ]),
                application_metadata: BTreeMap::from([(
                    item("status"),
                    ApplicationFieldValueV3::Choice(item("planning")),
                )]),
                source_text: source.to_owned(),
                requirements: vec![ApplicationFlowRequirementDraftV3 {
                    category: item("format"),
                    statement: source.to_owned(),
                    priority: RequirementPriorityV3::Mandatory,
                    start_byte: 0,
                    end_byte: u64::try_from(source.len()).expect("source length"),
                }],
            },
        )
        .expect("create generic Application");
        let application_id = created.data.stored.snapshot.application.id.clone();
        assert_eq!(created.data.stored.snapshot.application.revision.get(), 1);

        let stale = Application::plan_generic_application_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowPlanRequestV3 {
                expected_revision: Revision::try_new(2).expect("revision"),
                decision: item("proceed"),
                deliverables: Vec::new(),
            },
        )
        .expect_err("stale Plan request");
        assert!(matches!(stale, ApplicationError::Store(_)));
        assert_eq!(
            Application::application_model_v3(&root, application_id.as_str())
                .expect("unchanged Application")
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );

        let planned = Application::plan_generic_application_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowPlanRequestV3 {
                expected_revision: Revision::try_new(1).expect("revision"),
                decision: item("proceed"),
                deliverables: vec![
                    ApplicationFlowPlannedDeliverableV3 {
                        kind: item("primary-document"),
                        disposition: PlannedDeliverableDispositionV3::Required,
                        rationale: "The Pack requires one primary document".to_owned(),
                        constraints: vec!["Use only confirmed local evidence".to_owned()],
                        execution_mode: Some(ExecutionMode::ManualImport),
                    },
                    ApplicationFlowPlannedDeliverableV3 {
                        kind: item("supporting-document"),
                        disposition: PlannedDeliverableDispositionV3::Optional,
                        rationale: "The source requests a budget appendix".to_owned(),
                        constraints: vec!["Keep figures synthetic".to_owned()],
                        execution_mode: Some(ExecutionMode::ManualImport),
                    },
                ],
            },
        )
        .expect("confirm and plan");
        assert_eq!(
            planned
                .data
                .commit
                .stored
                .snapshot
                .application
                .revision
                .get(),
            2
        );

        let composed = Application::compose_generic_application_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowComposeRequestV3 {
                expected_revision: Revision::try_new(2).expect("revision"),
                deliverables: vec![
                    ApplicationFlowDeliverableDraftV3 {
                        kind: item("primary-document"),
                        title: "Project narrative".to_owned(),
                        media_type: "text/markdown".to_owned(),
                        content: "A literal #read(\"/private/canisend-sentinel\") remains text."
                            .to_owned(),
                    },
                    ApplicationFlowDeliverableDraftV3 {
                        kind: item("supporting-document"),
                        title: "Budget appendix".to_owned(),
                        media_type: "text/plain".to_owned(),
                        content: "Synthetic total: 100 units.".to_owned(),
                    },
                ],
            },
        )
        .expect("compose custom Deliverables");
        assert_eq!(composed.data.commit.stored.snapshot.deliverables.len(), 2);
        assert_eq!(
            composed
                .data
                .commit
                .stored
                .snapshot
                .application
                .revision
                .get(),
            3
        );

        let approved = Application::approve_generic_application_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowApproveRequestV3 {
                expected_revision: Revision::try_new(3).expect("revision"),
            },
        )
        .expect("approve Deliverables");
        assert_eq!(
            approved
                .data
                .commit
                .stored
                .snapshot
                .application
                .revision
                .get(),
            4
        );

        let destination = format!("applications/{application_id}/exports/alpha-fixture");
        let export_request =
            ApplicationFlowExportRequestV3::try_new(application_id.as_str(), 4, &destination)
                .expect("export request");
        Application::export_generic_application_v3(&root, export_request.clone(), None)
            .expect_err("export consent required");
        assert!(!root.join(&destination).exists());

        let exported = Application::export_generic_application_v3(
            &root,
            export_request,
            Some(PrivateExportConsent::granted_by_user()),
        )
        .expect("render and export");
        assert_eq!(exported.data.render.documents.len(), 2);
        assert!(!exported.data.render.submission_performed);
        assert!(!exported.data.package.submission_performed);
        assert!(
            exported
                .data
                .stages
                .iter()
                .all(|stage| stage.state == ApplicationFlowStageStateV3::Complete)
        );
        for document in &exported.data.render.documents {
            let bytes = fs::read(root.join(document.relative_path.as_str())).expect("PDF bytes");
            assert_eq!(
                validate_rendered_pdf(&bytes).expect("validated PDF"),
                document.page_count
            );
        }
        assert!(
            root.join(&destination)
                .join("render-manifest.json")
                .is_file()
        );
        assert_eq!(
            Application::application_model_history_v3(&root, application_id.as_str())
                .expect("history")
                .data
                .len(),
            4
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }
}
