use std::path::Path;

use canisend_contracts::{
    ApplicationId, ApplicationPackBindingV3, DeliverableId, DeliverableRecordV3, PlanRecordV3,
    RequirementId, RequirementRecordV3, Revision, Sha256Digest,
};
use serde::{Deserialize, Serialize};

use crate::{ActionReceipt, Application, ApplicationError, StoredApplicationModelV3};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationResourceContextV4 {
    pub application_id: ApplicationId,
    pub pack: ApplicationPackBindingV3,
    pub application_revision: Revision,
    pub snapshot_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementListReadModelV4 {
    pub context: ApplicationResourceContextV4,
    pub requirements: Vec<RequirementRecordV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementShowReadModelV4 {
    pub context: ApplicationResourceContextV4,
    pub requirement: RequirementRecordV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanShowReadModelV4 {
    pub context: ApplicationResourceContextV4,
    pub plan: Option<PlanRecordV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliverableListReadModelV4 {
    pub context: ApplicationResourceContextV4,
    pub deliverables: Vec<DeliverableRecordV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliverableShowReadModelV4 {
    pub context: ApplicationResourceContextV4,
    pub deliverable: DeliverableRecordV3,
}

impl Application {
    pub fn list_requirements_v4(
        root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<RequirementListReadModelV4>, ApplicationError> {
        let stored = application(root, application_id)?;
        let context = context(&stored);
        let requirements = stored.snapshot.requirements;
        Ok(ActionReceipt::new(
            "requirement.list",
            "current",
            format!("Loaded {} Requirement(s)", requirements.len()),
            RequirementListReadModelV4 {
                context,
                requirements,
            },
        ))
    }

    pub fn show_requirement_v4(
        root: &Path,
        application_id: &str,
        requirement_id: &str,
    ) -> Result<ActionReceipt<RequirementShowReadModelV4>, ApplicationError> {
        let requirement_id = RequirementId::try_new(requirement_id)
            .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))?;
        let stored = application(root, application_id)?;
        let context = context(&stored);
        let requirement = stored
            .snapshot
            .requirements
            .into_iter()
            .find(|requirement| requirement.id == requirement_id)
            .ok_or_else(|| {
                ApplicationError::InvalidInput(
                    "Requirement does not belong to the selected Application".to_owned(),
                )
            })?;
        Ok(ActionReceipt::new(
            "requirement.show",
            "current",
            "Loaded one exact Application Requirement",
            RequirementShowReadModelV4 {
                context,
                requirement,
            },
        ))
    }

    pub fn show_plan_v4(
        root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<PlanShowReadModelV4>, ApplicationError> {
        let stored = application(root, application_id)?;
        let context = context(&stored);
        let plan = stored.snapshot.plan;
        let status = if plan.is_some() {
            "current"
        } else {
            "not-created"
        };
        Ok(ActionReceipt::new(
            "plan.show",
            status,
            if plan.is_some() {
                "Loaded the current Application Plan"
            } else {
                "The selected Application has no Plan"
            },
            PlanShowReadModelV4 { context, plan },
        ))
    }

    pub fn list_deliverables_v4(
        root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<DeliverableListReadModelV4>, ApplicationError> {
        let stored = application(root, application_id)?;
        let context = context(&stored);
        let deliverables = stored.snapshot.deliverables;
        Ok(ActionReceipt::new(
            "deliverable.list",
            "current",
            format!("Loaded {} Deliverable(s)", deliverables.len()),
            DeliverableListReadModelV4 {
                context,
                deliverables,
            },
        ))
    }

    pub fn show_deliverable_v4(
        root: &Path,
        application_id: &str,
        deliverable_id: &str,
    ) -> Result<ActionReceipt<DeliverableShowReadModelV4>, ApplicationError> {
        let deliverable_id = DeliverableId::try_new(deliverable_id)
            .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))?;
        let stored = application(root, application_id)?;
        let context = context(&stored);
        let deliverable = stored
            .snapshot
            .deliverables
            .into_iter()
            .find(|deliverable| deliverable.id == deliverable_id)
            .ok_or_else(|| {
                ApplicationError::InvalidInput(
                    "Deliverable does not belong to the selected Application".to_owned(),
                )
            })?;
        Ok(ActionReceipt::new(
            "deliverable.show",
            "current",
            "Loaded one exact Application Deliverable metadata record",
            DeliverableShowReadModelV4 {
                context,
                deliverable,
            },
        ))
    }
}

fn application(
    root: &Path,
    application_id: &str,
) -> Result<StoredApplicationModelV3, ApplicationError> {
    ApplicationId::try_new(application_id)
        .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))?;
    Ok(Application::application_model_v4(root, application_id)?.data)
}

fn context(stored: &StoredApplicationModelV3) -> ApplicationResourceContextV4 {
    ApplicationResourceContextV4 {
        application_id: stored.snapshot.application.id.clone(),
        pack: stored.snapshot.pack.clone(),
        application_revision: stored.snapshot.application.revision,
        snapshot_sha256: stored.snapshot_sha256.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ExecutionMode, PlannedDeliverableDispositionV3, RequirementPriorityV3, WorkflowPackId,
        WorkflowPackItemId,
    };
    use canisend_store::{
        ApplicationFlowComposeRequestV3, ApplicationFlowCreateRequestV3,
        ApplicationFlowDeliverableDraftV3, ApplicationFlowPlanRequestV3,
        ApplicationFlowPlannedDeliverableV3, ApplicationFlowRequirementDraftV3,
    };

    use super::*;
    use crate::{ApplicationFlowCreateRequestV4, GENERIC_APPLICATION_WORKFLOW_PACK_ID};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-application-resources-v4-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn exact_application_resource_reads_are_pack_revision_bound_and_body_free() {
        let root = root();
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let created = Application::create_application_flow_v4(
            &root,
            ApplicationFlowCreateRequestV4 {
                pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                    .expect("Pack ID"),
                application: ApplicationFlowCreateRequestV3 {
                    title: "Resource read fixture".to_owned(),
                    opportunity_metadata: Default::default(),
                    application_metadata: Default::default(),
                    source_text: "Provide one concise narrative.".to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: WorkflowPackItemId::try_new("format").expect("category"),
                        statement: "Provide one concise narrative.".to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: 30,
                    }],
                },
            },
        )
        .expect("Application");
        let application_id = created.data.stored.snapshot.application.id;
        let requirement_id = created.data.stored.snapshot.requirements[0].id.clone();

        let requirements = Application::list_requirements_v4(&root, application_id.as_str())
            .expect("Requirement list");
        assert_eq!(requirements.operation, "requirement.list");
        assert_eq!(requirements.data.requirements.len(), 1);
        assert_eq!(requirements.data.context.application_id, application_id);
        assert_eq!(
            requirements.data.context.pack.id.as_str(),
            GENERIC_APPLICATION_WORKFLOW_PACK_ID
        );
        assert_eq!(
            Application::show_requirement_v4(
                &root,
                application_id.as_str(),
                requirement_id.as_str(),
            )
            .expect("Requirement show")
            .data
            .requirement
            .id,
            requirement_id
        );

        let plan = Application::show_plan_v4(&root, application_id.as_str()).expect("Plan show");
        assert_eq!(plan.status, "not-created");
        assert!(plan.data.plan.is_none());
        let deliverables =
            Application::list_deliverables_v4(&root, application_id.as_str()).expect("list");
        assert!(deliverables.data.deliverables.is_empty());

        let item = |value| WorkflowPackItemId::try_new(value).expect("Pack item ID");
        Application::plan_application_flow_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowPlanRequestV3 {
                expected_revision: Revision::try_new(1).expect("revision"),
                decision: item("proceed"),
                deliverables: vec![ApplicationFlowPlannedDeliverableV3 {
                    kind: item("primary-document"),
                    disposition: PlannedDeliverableDispositionV3::Required,
                    rationale: "Required by the reviewed source".to_owned(),
                    constraints: Vec::new(),
                    execution_mode: Some(ExecutionMode::ManualImport),
                }],
            },
        )
        .expect("Plan");
        Application::compose_application_flow_v3(
            &root,
            application_id.as_str(),
            ApplicationFlowComposeRequestV3 {
                expected_revision: Revision::try_new(2).expect("revision"),
                deliverables: vec![ApplicationFlowDeliverableDraftV3 {
                    kind: item("primary-document"),
                    title: "Private narrative".to_owned(),
                    media_type: "text/markdown".to_owned(),
                    content: "DELIVERABLE-PRIVATE-BODY-MUST-NOT-LEAK".to_owned(),
                }],
            },
        )
        .expect("Deliverable draft");
        let deliverables =
            Application::list_deliverables_v4(&root, application_id.as_str()).expect("list");
        let deliverable_id = deliverables.data.deliverables[0].id.clone();
        let shown_deliverable = Application::show_deliverable_v4(
            &root,
            application_id.as_str(),
            deliverable_id.as_str(),
        )
        .expect("show Deliverable");
        assert_eq!(shown_deliverable.data.deliverable.id, deliverable_id);
        assert!(
            !serde_json::to_string(&shown_deliverable)
                .expect("serialize Deliverable metadata")
                .contains("DELIVERABLE-PRIVATE-BODY-MUST-NOT-LEAK")
        );

        let serialized = serde_json::to_string(&requirements).expect("serialize receipt");
        assert!(!serialized.contains("normalized_text"));
        assert!(!serialized.contains("original_bytes"));

        let other = Application::create_application_flow_v4(
            &root,
            ApplicationFlowCreateRequestV4 {
                pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                    .expect("Pack ID"),
                application: ApplicationFlowCreateRequestV3 {
                    title: "Other Application".to_owned(),
                    opportunity_metadata: Default::default(),
                    application_metadata: Default::default(),
                    source_text: "Provide a different narrative.".to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: WorkflowPackItemId::try_new("format").expect("category"),
                        statement: "Provide a different narrative.".to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: 30,
                    }],
                },
            },
        )
        .expect("other Application");
        assert!(
            Application::show_requirement_v4(
                &root,
                other.data.stored.snapshot.application.id.as_str(),
                requirement_id.as_str(),
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
