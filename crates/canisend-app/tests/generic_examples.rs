#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_app::{
    Application, ApplicationFlowApproveRequestV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowCreateRequestV3, ApplicationFlowDeliverableDraftV3,
    ApplicationFlowExportRequestV3, ApplicationFlowPlanRequestV3,
    ApplicationFlowPlannedDeliverableV3, ApplicationFlowRequirementDraftV3,
    ApplicationFlowStageStateV3, PrivateExportConsent, PrivateReadConsent,
};
use canisend_contracts::{
    ApplicationFieldValueV3, DeliverableStateV3, ExecutionMode, PlannedDeliverableDispositionV3,
    RequirementPriorityV3, WorkflowPackItemId,
};
use canisend_io::validate_rendered_pdf;
use canisend_resources::{GENERIC_APPLICATION_WORKFLOW_PACK_ID, ResourceId, ResourceKind, get};
use serde::{Deserialize, Serialize};

const EXAMPLE_FORMAT: &str = "canisend.generic-application-example/v1";
const EXAMPLE_DATA_POLICY: &str = "fictional-only-no-real-personal-data";
const SYNTHETIC_NOTICE: &str = "All names, organizations, identifiers, and Application content in this example are fictional test data.";
const EXAMPLES: [(&str, &str); 4] = [
    ("example.generic-v3.grant", "grant"),
    ("example.generic-v3.admission", "admission"),
    ("example.generic-v3.tender-proposal", "tender-proposal"),
    ("example.generic-v3.professional-job", "professional-job"),
];

static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GenericExampleV1 {
    format: String,
    scenario_id: String,
    family: String,
    synthetic: bool,
    data_policy: String,
    synthetic_notice: String,
    pack_id: String,
    title: String,
    opportunity_metadata: BTreeMap<String, ApplicationFieldValueV3>,
    application_metadata: BTreeMap<String, ApplicationFieldValueV3>,
    source_text: String,
    requirements: Vec<RequirementExampleV1>,
    decision: String,
    planned_deliverables: Vec<PlannedDeliverableExampleV1>,
    deliverables: Vec<DeliverableExampleV1>,
    export_slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RequirementExampleV1 {
    category: String,
    statement: String,
    priority: RequirementPriorityV3,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlannedDeliverableExampleV1 {
    kind: String,
    disposition: PlannedDeliverableDispositionV3,
    rationale: String,
    constraints: Vec<String>,
    execution_mode: Option<ExecutionMode>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeliverableExampleV1 {
    kind: String,
    title: String,
    media_type: String,
    content: String,
}

#[test]
fn four_domain_families_complete_offline_without_real_data_or_submission() {
    let mut scenario_ids = BTreeSet::new();
    let mut families = BTreeSet::new();
    for (resource_id, expected_family) in EXAMPLES {
        let id = ResourceId::from_str(resource_id).expect("typed example resource ID");
        let resource = get(id);
        assert_eq!(resource.descriptor.kind, ResourceKind::Example);
        assert_eq!(resource.descriptor.version, "1.0.0");
        let scenario: GenericExampleV1 =
            serde_json::from_slice(resource.bytes).expect("valid generic example JSON");
        validate_synthetic_contract(&scenario, expected_family);
        assert!(scenario_ids.insert(scenario.scenario_id.clone()));
        assert!(families.insert(scenario.family.clone()));
        run_complete_offline_flow(&scenario);
    }
    assert_eq!(
        families,
        BTreeSet::from([
            "admission".to_owned(),
            "grant".to_owned(),
            "professional-job".to_owned(),
            "tender-proposal".to_owned(),
        ])
    );
}

fn validate_synthetic_contract(scenario: &GenericExampleV1, expected_family: &str) {
    assert_eq!(scenario.format, EXAMPLE_FORMAT);
    assert_eq!(scenario.family, expected_family);
    assert!(scenario.synthetic);
    assert_eq!(scenario.data_policy, EXAMPLE_DATA_POLICY);
    assert_eq!(scenario.synthetic_notice, SYNTHETIC_NOTICE);
    assert_eq!(scenario.pack_id, GENERIC_APPLICATION_WORKFLOW_PACK_ID);
    assert!(scenario.title.starts_with("Synthetic "));
    assert_eq!(scenario.requirements.len(), 2);
    assert_eq!(scenario.planned_deliverables.len(), 2);
    assert_eq!(scenario.deliverables.len(), 2);
    assert!(!scenario.opportunity_metadata.contains_key("source-url"));
    let encoded = serde_json::to_string(scenario).expect("serialize example for privacy checks");
    assert!(!encoded.contains("https://"));
    assert!(!encoded.contains("http://"));
    assert!(!encoded.contains('@'));
    assert!(!encoded.contains("\".canisend"));
}

fn run_complete_offline_flow(scenario: &GenericExampleV1) {
    let root = temporary_root(&scenario.family);
    Application::initialize_workspace_v3(&root).expect("initialize generic Workspace");
    let created = Application::create_generic_application_v3(
        &root,
        ApplicationFlowCreateRequestV3 {
            title: scenario.title.clone(),
            opportunity_metadata: metadata(&scenario.opportunity_metadata),
            application_metadata: metadata(&scenario.application_metadata),
            source_text: scenario.source_text.clone(),
            requirements: requirement_drafts(scenario),
        },
    )
    .expect("create synthetic Application")
    .data;
    assert_eq!(
        created.stored.snapshot.pack.id.as_str(),
        GENERIC_APPLICATION_WORKFLOW_PACK_ID
    );
    let application_id = created.stored.snapshot.application.id.to_string();

    let planned = Application::plan_generic_application_v3(
        &root,
        &application_id,
        ApplicationFlowPlanRequestV3 {
            expected_revision: created.stored.snapshot.application.revision,
            decision: item(&scenario.decision),
            deliverables: scenario
                .planned_deliverables
                .iter()
                .map(|deliverable| ApplicationFlowPlannedDeliverableV3 {
                    kind: item(&deliverable.kind),
                    disposition: deliverable.disposition,
                    rationale: deliverable.rationale.clone(),
                    constraints: deliverable.constraints.clone(),
                    execution_mode: deliverable.execution_mode,
                })
                .collect(),
        },
    )
    .expect("confirm synthetic Plan")
    .data;

    let composed = Application::compose_generic_application_v3(
        &root,
        &application_id,
        ApplicationFlowComposeRequestV3 {
            expected_revision: planned.commit.stored.snapshot.application.revision,
            deliverables: scenario
                .deliverables
                .iter()
                .map(|deliverable| ApplicationFlowDeliverableDraftV3 {
                    kind: item(&deliverable.kind),
                    title: deliverable.title.clone(),
                    media_type: deliverable.media_type.clone(),
                    content: deliverable.content.clone(),
                })
                .collect(),
        },
    )
    .expect("compose synthetic Deliverables")
    .data;
    assert!(
        composed
            .commit
            .stored
            .snapshot
            .deliverables
            .iter()
            .all(|deliverable| deliverable.state == DeliverableStateV3::ReviewRequired)
    );

    let reviewed = Application::review_generic_application_v3(
        &root,
        &application_id,
        Some(PrivateReadConsent::granted_by_user()),
    )
    .expect("review synthetic Deliverables")
    .data;
    let reviewed_bodies = reviewed
        .deliverables
        .iter()
        .map(|deliverable| deliverable.content.as_str())
        .collect::<BTreeSet<_>>();
    let expected_bodies = scenario
        .deliverables
        .iter()
        .map(|deliverable| deliverable.content.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(reviewed_bodies, expected_bodies);

    let approved = Application::approve_generic_application_v3(
        &root,
        &application_id,
        ApplicationFlowApproveRequestV3 {
            expected_revision: reviewed.stored.snapshot.application.revision,
        },
    )
    .expect("approve synthetic Deliverables")
    .data;
    let approved_revision = approved.commit.stored.snapshot.application.revision;
    let destination = format!(
        "applications/{application_id}/exports/synthetic-{}",
        scenario.export_slug
    );
    let exported = Application::export_generic_application_v3(
        &root,
        ApplicationFlowExportRequestV3::try_new(
            &application_id,
            approved_revision.get(),
            &destination,
        )
        .expect("safe example destination"),
        Some(PrivateExportConsent::granted_by_user()),
    )
    .expect("export synthetic Application")
    .data;
    assert_eq!(exported.render.documents.len(), 2);
    assert!(!exported.render.submission_performed);
    assert!(
        exported
            .stages
            .iter()
            .all(|stage| stage.state == ApplicationFlowStageStateV3::Complete)
    );
    for document in &exported.render.documents {
        let bytes = fs::read(root.join(document.relative_path.as_str()))
            .expect("read rendered example PDF");
        let page_count = validate_rendered_pdf(&bytes).expect("validate rendered example PDF");
        assert_eq!(page_count, document.page_count);
        assert_eq!(bytes.len() as u64, document.byte_count);
    }
    assert!(
        Application::check_workspace(&root)
            .expect("check Workspace")
            .data
            .check
            .ok
    );
    fs::remove_dir_all(root).expect("remove example Workspace");
}

fn metadata(
    values: &BTreeMap<String, ApplicationFieldValueV3>,
) -> BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3> {
    values
        .iter()
        .map(|(key, value)| (item(key), value.clone()))
        .collect()
}

fn requirement_drafts(scenario: &GenericExampleV1) -> Vec<ApplicationFlowRequirementDraftV3> {
    scenario
        .requirements
        .iter()
        .map(|requirement| {
            let matches = scenario
                .source_text
                .match_indices(&requirement.statement)
                .collect::<Vec<_>>();
            assert_eq!(
                matches.len(),
                1,
                "Requirement excerpt must occur exactly once in {}",
                scenario.scenario_id
            );
            let start = matches[0].0;
            ApplicationFlowRequirementDraftV3 {
                category: item(&requirement.category),
                statement: requirement.statement.clone(),
                priority: requirement.priority,
                start_byte: start as u64,
                end_byte: (start + requirement.statement.len()) as u64,
            }
        })
        .collect()
}

fn item(value: &str) -> WorkflowPackItemId {
    WorkflowPackItemId::try_new(value).expect("valid Pack item ID")
}

fn temporary_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "canisend-generic-example-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
