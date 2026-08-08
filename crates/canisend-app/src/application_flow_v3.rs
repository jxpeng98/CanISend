use std::path::Path;

use canisend_contracts::{
    ApplicationId, ApplicationPackBindingV3, NextAction, Revision, SafeRelativePath,
    WORKSPACE_V3_FORMAT, WorkflowPackId,
};
use canisend_core::VerifiedWorkflowPackBundle;
use canisend_store::ApplicationFlowServiceV3;
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateExportConsent,
    application::open_workspace, built_in_academic_job_pack, built_in_generic_application_pack,
    built_in_workflow_pack_registry,
};

pub use canisend_store::{
    APPLICATION_FLOW_EXPORT_FORMAT_V3, ApplicationFlowApproveRequestV3,
    ApplicationFlowCommitReadModelV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowCreateRequestV3, ApplicationFlowDeliverableDraftV3,
    ApplicationFlowExportManifestV3, ApplicationFlowExportReadModelV3,
    ApplicationFlowPlanRequestV3, ApplicationFlowPlannedDeliverableV3, ApplicationFlowReadModelV3,
    ApplicationFlowRenderedDeliverableV3, ApplicationFlowRequirementDraftV3,
    ApplicationFlowReviewDeliverableV3, ApplicationFlowReviewReadModelV3,
    ApplicationFlowStageReadModelV3, ApplicationFlowStageStateV3,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowExportRequestV3 {
    pub application_id: ApplicationId,
    pub expected_revision: Revision,
    pub destination: SafeRelativePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowCreateRequestV4 {
    pub pack_id: WorkflowPackId,
    pub application: ApplicationFlowCreateRequestV3,
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
    pub fn create_application_flow_v4(
        workspace_root: &Path,
        request: ApplicationFlowCreateRequestV4,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        Self::workspace_status_v4(workspace_root)?;
        let pack = requested_built_in_pack(&request.pack_id)?;
        let mut workspace = crate::application::open_workspace_v4(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create(&pack, request.application)?;
        Ok(ActionReceipt::new(
            "application.create",
            "created",
            "Created an Application with an exact Application-level Pack binding",
            result,
        ))
    }

    pub fn application_flow_v3(
        workspace_root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let pack = exact_application_pack(workspace_root, application_id.as_str())?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .status(&pack, &application_id)?;
        Ok(ActionReceipt::new(
            "application-flow-v3.status",
            "current",
            "Loaded the current exact-Pack Application flow",
            result,
        ))
    }

    pub fn create_application_flow_v3(
        workspace_root: &Path,
        request: ApplicationFlowCreateRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        let pack = exact_workspace_pack(workspace_root)?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create(&pack, request)?;
        Ok(ActionReceipt::new(
            "application-flow-v3.create",
            "created",
            "Created an exact-Pack Application intake",
            result,
        ))
    }

    pub fn plan_application_flow_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowPlanRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let pack = exact_application_pack(workspace_root, application_id.as_str())?;
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

    pub fn compose_application_flow_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowComposeRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let pack = exact_application_pack(workspace_root, application_id.as_str())?;
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

    pub fn approve_application_flow_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowApproveRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let pack = exact_application_pack(workspace_root, application_id.as_str())?;
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

    pub fn review_application_flow_v3(
        workspace_root: &Path,
        application_id: &str,
        consent: Option<crate::PrivateReadConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowReviewReadModelV3>, ApplicationError> {
        if consent.is_none() {
            return Err(ApplicationError::ConsentRequired {
                message: "Review reads private Pack-bound Deliverable bodies".to_owned(),
                remediation: NextAction {
                    action: "grant private read consent".to_owned(),
                    description:
                        "Confirm that the current Deliverable bodies may be opened locally for review"
                            .to_owned(),
                },
            });
        }
        let application_id = parse_application_id(application_id)?;
        let pack = exact_application_pack(workspace_root, application_id.as_str())?;
        let mut workspace = open_workspace(workspace_root)?;
        let root = workspace.paths.root.clone();
        let result =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .review(&pack, &application_id)?;
        Ok(ActionReceipt::new(
            "application-flow-v3.review",
            "private-content-available",
            "Loaded current Deliverable bodies for explicit local review",
            result,
        ))
    }

    pub fn export_application_flow_v3(
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
        let pack = exact_application_pack(workspace_root, request.application_id.as_str())?;
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

    #[deprecated(note = "use the exact-Pack application_flow_v3 facade")]
    pub fn generic_application_flow_v3(
        workspace_root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        Self::application_flow_v3(workspace_root, application_id)
    }

    #[deprecated(note = "use the exact-Pack create_application_flow_v3 facade")]
    pub fn create_generic_application_v3(
        workspace_root: &Path,
        request: ApplicationFlowCreateRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        Self::create_application_flow_v3(workspace_root, request)
    }

    #[deprecated(note = "use the exact-Pack plan_application_flow_v3 facade")]
    pub fn plan_generic_application_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowPlanRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        Self::plan_application_flow_v3(workspace_root, application_id, request)
    }

    #[deprecated(note = "use the exact-Pack compose_application_flow_v3 facade")]
    pub fn compose_generic_application_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowComposeRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        Self::compose_application_flow_v3(workspace_root, application_id, request)
    }

    #[deprecated(note = "use the exact-Pack approve_application_flow_v3 facade")]
    pub fn approve_generic_application_v3(
        workspace_root: &Path,
        application_id: &str,
        request: ApplicationFlowApproveRequestV3,
    ) -> Result<ActionReceipt<ApplicationFlowCommitReadModelV3>, ApplicationError> {
        Self::approve_application_flow_v3(workspace_root, application_id, request)
    }

    #[deprecated(note = "use the exact-Pack review_application_flow_v3 facade")]
    pub fn review_generic_application_v3(
        workspace_root: &Path,
        application_id: &str,
        consent: Option<crate::PrivateReadConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowReviewReadModelV3>, ApplicationError> {
        Self::review_application_flow_v3(workspace_root, application_id, consent)
    }

    #[deprecated(note = "use the exact-Pack export_application_flow_v3 facade")]
    pub fn export_generic_application_v3(
        workspace_root: &Path,
        request: ApplicationFlowExportRequestV3,
        consent: Option<PrivateExportConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowExportReadModelV3>, ApplicationError> {
        Self::export_application_flow_v3(workspace_root, request, consent)
    }
}

pub(crate) fn exact_workspace_pack(
    root: &Path,
) -> Result<VerifiedWorkflowPackBundle, ApplicationError> {
    let workspace = Application::workspace_status(root)?.data;
    if workspace.status.workspace_format != WORKSPACE_V3_FORMAT {
        return Err(ApplicationError::CompatibilityUnavailable {
            message: "Canonical Application v3 operations require Workspace v3 authority"
                .to_owned(),
            details: serde_json::json!({
                "workspace_format": workspace.status.workspace_format,
                "pack_id": workspace.pack_id,
            }),
            remediation: NextAction {
                action: "preview and approve Workspace v2 to v3 migration".to_owned(),
                description: "Create a verified backup and migrate before using canonical Application operations"
                    .to_owned(),
            },
        });
    }
    match workspace.pack_id.as_str() {
        crate::ACADEMIC_JOB_WORKFLOW_PACK_ID => built_in_academic_job_pack(),
        crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID => built_in_generic_application_pack(),
        _ => Err(ApplicationError::CompatibilityUnavailable {
            message: "Workspace references an unavailable workflow Pack".to_owned(),
            details: serde_json::json!({ "pack_id": workspace.pack_id }),
            remediation: NextAction {
                action: "restore the exact workflow Pack".to_owned(),
                description: "Do not reinterpret an Application under a different Pack".to_owned(),
            },
        }),
    }
}

pub(crate) fn exact_application_pack(
    root: &Path,
    application_id: &str,
) -> Result<VerifiedWorkflowPackBundle, ApplicationError> {
    let stored = Application::application_model_v3(root, application_id)?.data;
    resolve_exact_pack(&stored.snapshot.pack)
}

fn resolve_exact_pack(
    binding: &ApplicationPackBindingV3,
) -> Result<VerifiedWorkflowPackBundle, ApplicationError> {
    let registry = built_in_workflow_pack_registry()?;
    registry
        .resolve_exact(&binding.id, &binding.version, &binding.content_digest)
        .cloned()
        .map_err(|error| {
            ApplicationError::ResourceIntegrity(format!(
                "Application references an unavailable or substituted workflow Pack: {error}"
            ))
        })
}

pub(crate) fn requested_built_in_pack(
    pack_id: &WorkflowPackId,
) -> Result<VerifiedWorkflowPackBundle, ApplicationError> {
    match pack_id.as_str() {
        crate::ACADEMIC_JOB_WORKFLOW_PACK_ID => built_in_academic_job_pack(),
        crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID => built_in_generic_application_pack(),
        _ => Err(ApplicationError::InvalidInput(format!(
            "unknown built-in workflow Pack: {pack_id}"
        ))),
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
        let created = Application::create_application_flow_v3(
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

        let stale = Application::plan_application_flow_v3(
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

        let planned = Application::plan_application_flow_v3(
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

        let composed = Application::compose_application_flow_v3(
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

        Application::review_application_flow_v3(&root, application_id.as_str(), None)
            .expect_err("private Deliverable review requires consent");
        let review = Application::review_application_flow_v3(
            &root,
            application_id.as_str(),
            Some(crate::PrivateReadConsent::granted_by_user()),
        )
        .expect("review current Deliverables");
        assert_eq!(review.data.deliverables.len(), 2);
        assert_eq!(
            review.data.deliverables[0].content,
            "A literal #read(\"/private/canisend-sentinel\") remains text."
        );

        let approved = Application::approve_application_flow_v3(
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
        Application::export_application_flow_v3(&root, export_request.clone(), None)
            .expect_err("export consent required");
        assert!(!root.join(&destination).exists());

        let exported = Application::export_application_flow_v3(
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

    #[test]
    fn workspace_v4_creates_both_built_in_application_flows() {
        let root = temporary_root("v4-mixed-pack");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");

        let generic_source = "Applicants must provide a project narrative.";
        let generic = Application::create_application_flow_v4(
            &root,
            ApplicationFlowCreateRequestV4 {
                pack_id: WorkflowPackId::try_new(crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                    .expect("generic Pack ID"),
                application: ApplicationFlowCreateRequestV3 {
                    title: "Synthetic community project".to_owned(),
                    opportunity_metadata: BTreeMap::from([
                        (
                            item("organization"),
                            ApplicationFieldValueV3::ShortText("Example Foundation".to_owned()),
                        ),
                        (
                            item("reference"),
                            ApplicationFieldValueV3::ShortText("SYN-V4-001".to_owned()),
                        ),
                    ]),
                    application_metadata: BTreeMap::from([(
                        item("status"),
                        ApplicationFieldValueV3::Choice(item("planning")),
                    )]),
                    source_text: generic_source.to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: item("format"),
                        statement: generic_source.to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: u64::try_from(generic_source.len()).expect("source length"),
                    }],
                },
            },
        )
        .expect("generic v4 Application");

        let academic_source = "Applicants must submit a cover letter and academic CV.";
        let academic = Application::create_application_flow_v4(
            &root,
            ApplicationFlowCreateRequestV4 {
                pack_id: WorkflowPackId::try_new(crate::ACADEMIC_JOB_WORKFLOW_PACK_ID)
                    .expect("academic Pack ID"),
                application: ApplicationFlowCreateRequestV3 {
                    title: "Synthetic academic opportunity".to_owned(),
                    opportunity_metadata: BTreeMap::from([(
                        item("institution"),
                        ApplicationFieldValueV3::ShortText("Example University".to_owned()),
                    )]),
                    application_metadata: BTreeMap::new(),
                    source_text: academic_source.to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: item("qualification"),
                        statement: academic_source.to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: u64::try_from(academic_source.len()).expect("source length"),
                    }],
                },
            },
        )
        .expect("academic v4 Application");

        assert_eq!(
            generic.data.stored.snapshot.pack.id.as_str(),
            crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID
        );
        assert_eq!(
            academic.data.stored.snapshot.pack.id.as_str(),
            crate::ACADEMIC_JOB_WORKFLOW_PACK_ID
        );
        let applications = Application::list_application_models_v3(&root)
            .expect("mixed Application collection")
            .data;
        assert_eq!(applications.len(), 2);
        assert_eq!(
            Application::workspace_status_v4(&root)
                .expect("v4 status")
                .data
                .status
                .application_count,
            2
        );

        let unknown_pack = Application::create_application_flow_v4(
            &root,
            ApplicationFlowCreateRequestV4 {
                pack_id: WorkflowPackId::try_new("org.example.unavailable")
                    .expect("synthetic Pack ID"),
                application: ApplicationFlowCreateRequestV3 {
                    title: "Must not be created".to_owned(),
                    opportunity_metadata: BTreeMap::new(),
                    application_metadata: BTreeMap::new(),
                    source_text: "No mutation.".to_owned(),
                    requirements: Vec::new(),
                },
            },
        )
        .expect_err("unknown Pack must fail before Application mutation");
        assert!(matches!(unknown_pack, ApplicationError::InvalidInput(_)));
        assert_eq!(
            Application::workspace_status_v4(&root)
                .expect("unchanged v4 status")
                .data
                .status
                .application_count,
            2
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn migrated_academic_pack_completes_the_same_neutral_v3_flow() {
        let root = temporary_root("academic-complete");
        let backup = temporary_root("academic-complete-backup");
        Application::initialize_workspace(&root).expect("Workspace v2");
        Application::create_job(&root, "Research Fellow", "Example University")
            .expect("legacy academic Application");
        let preview = Application::preview_workspace_v3_migration(&root)
            .expect("migration preview")
            .data;
        Application::migrate_workspace_v3(
            &root,
            crate::WorkspaceV3MigrationRequest {
                expected_plan_sha256: preview.migration_plan_sha256,
                backup_destination: backup.clone(),
            },
        )
        .expect("Workspace v3 migration");

        let source = "Applicants must submit a cover letter and academic CV.";
        let created = Application::create_application_flow_v3(
            &root,
            ApplicationFlowCreateRequestV3 {
                title: "Synthetic academic opportunity".to_owned(),
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
        )
        .expect("academic Application v3");
        let application_id = created.data.stored.snapshot.application.id;
        assert_eq!(
            created.data.stored.snapshot.pack.id.as_str(),
            crate::ACADEMIC_JOB_WORKFLOW_PACK_ID
        );

        Application::plan_application_flow_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowPlanRequestV3 {
                expected_revision: Revision::try_new(1).expect("revision"),
                decision: item("proceed"),
                deliverables: vec![
                    ApplicationFlowPlannedDeliverableV3 {
                        kind: item("cover-letter"),
                        disposition: PlannedDeliverableDispositionV3::Required,
                        rationale: "Required by the reviewed opportunity".to_owned(),
                        constraints: Vec::new(),
                        execution_mode: Some(ExecutionMode::HostAgent),
                    },
                    ApplicationFlowPlannedDeliverableV3 {
                        kind: item("cv"),
                        disposition: PlannedDeliverableDispositionV3::Required,
                        rationale: "Required by the reviewed opportunity".to_owned(),
                        constraints: Vec::new(),
                        execution_mode: Some(ExecutionMode::HostAgent),
                    },
                ],
            },
        )
        .expect("academic Plan");
        Application::compose_application_flow_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowComposeRequestV3 {
                expected_revision: Revision::try_new(2).expect("revision"),
                deliverables: vec![
                    ApplicationFlowDeliverableDraftV3 {
                        kind: item("cover-letter"),
                        title: "Cover letter".to_owned(),
                        media_type: "text/markdown".to_owned(),
                        content: "Evidence-bound synthetic cover letter.".to_owned(),
                    },
                    ApplicationFlowDeliverableDraftV3 {
                        kind: item("cv"),
                        title: "Academic CV".to_owned(),
                        media_type: "text/markdown".to_owned(),
                        content: "Evidence-bound synthetic academic record.".to_owned(),
                    },
                ],
            },
        )
        .expect("academic Deliverables");
        let review = Application::review_application_flow_v3(
            &root,
            application_id.as_str(),
            Some(crate::PrivateReadConsent::granted_by_user()),
        )
        .expect("academic review");
        assert_eq!(review.data.deliverables.len(), 2);
        Application::approve_application_flow_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowApproveRequestV3 {
                expected_revision: Revision::try_new(3).expect("revision"),
            },
        )
        .expect("academic approval");

        let destination = format!("applications/{application_id}/exports/academic-v3");
        let exported = Application::export_application_flow_v3(
            &root,
            ApplicationFlowExportRequestV3::try_new(application_id.as_str(), 4, &destination)
                .expect("export request"),
            Some(PrivateExportConsent::granted_by_user()),
        )
        .expect("academic export");
        assert_eq!(exported.data.render.documents.len(), 2);
        assert!(!exported.data.render.submission_performed);
        assert_eq!(exported.data.stages.len(), 10);
        assert!(
            exported
                .data
                .stages
                .iter()
                .all(|stage| stage.state == ApplicationFlowStageStateV3::Complete)
        );

        fs::remove_dir_all(root).expect("remove fixture");
        fs::remove_dir_all(backup).expect("remove backup");
    }
}
