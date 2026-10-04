#![forbid(unsafe_code)]

use std::{collections::BTreeMap, fs};

use canisend_app::{
    Application, ApplicationFlowApproveRequestV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowCreateRequestV3, ApplicationFlowCreateRequestV4,
    ApplicationFlowDeliverableDraftV3, ApplicationFlowExportRequestV3,
    ApplicationFlowPlanRequestV3, ApplicationFlowPlannedDeliverableV3,
    ApplicationFlowRequirementDraftV3, AssociationApprovalBrokerV4, AssociationChangeV4,
    EvidenceApprovalBrokerV4, EvidenceAssociationPreviewRequestV4, PrivateExportConsent,
    PrivateReadConsent, ProfileAssociationPreviewRequestV4,
};
use canisend_contracts::{
    ApplicationFieldValueV3, ContentRevisionReferenceV3, DELIVERABLE_DOCUMENT_MEDIA_TYPE_V3,
    EvidenceKind, EvidenceProposalRecord, EvidenceProposalSet, ExecutionMode,
    PlannedDeliverableDispositionV3, PrivacyClassification, RequirementPriorityV3, Revision,
    SourceTextSpan, WorkflowPackItemId,
};
use serde_json::{Value, json};

fn item(value: &str) -> WorkflowPackItemId {
    WorkflowPackItemId::try_new(value).unwrap()
}
fn consent() -> Option<PrivateReadConsent> {
    Some(PrivateReadConsent::granted_by_user())
}

// All confirmations in this test apply only to fictional data in its isolated Workspace.
#[test]
fn sourced_academic_package_requires_complete_plan_and_preserves_identity_and_sections() {
    let root = std::env::temp_dir().join(format!(
        "canisend-academic-package-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    Application::initialize_workspace_v4(&root).unwrap();
    let source_text = "Submit a CV, cover letter, research statement and teaching statement.";
    let created = Application::create_application_flow_v4(
        &root,
        ApplicationFlowCreateRequestV4 {
            pack_id: canisend_contracts::WorkflowPackId::try_new(
                canisend_app::ACADEMIC_JOB_WORKFLOW_PACK_ID,
            )
            .unwrap(),
            application: ApplicationFlowCreateRequestV3 {
                title: "Fictional faculty application".to_owned(),
                opportunity_metadata: BTreeMap::from([(
                    item("institution"),
                    ApplicationFieldValueV3::ShortText("Example University".to_owned()),
                )]),
                application_metadata: BTreeMap::new(),
                source_text: source_text.to_owned(),
                requirements: vec![ApplicationFlowRequirementDraftV3 {
                    category: item("qualification"),
                    statement: source_text.to_owned(),
                    priority: RequirementPriorityV3::Mandatory,
                    start_byte: 0,
                    end_byte: source_text.len() as u64,
                }],
            },
        },
    )
    .unwrap()
    .data;
    let id = created.stored.snapshot.application.id;
    let profile_text =
        "Dr. Nova Example\nnova@candidate.invalid\nCoordinated a fictional research seminar.";
    let profile_path = root.join("fictional-profile.txt");
    fs::write(&profile_path, profile_text).unwrap();
    let imported = Application::import_profile_source_v4(
        &root,
        &profile_path,
        PrivacyClassification::PrivateLocal,
        consent(),
    )
    .unwrap()
    .data;
    let source = imported.source;
    let profile_reference = ContentRevisionReferenceV3 {
        id: source.id,
        revision: source.revision,
        sha256: source.original.sha256,
    };
    let associations = AssociationApprovalBrokerV4::default();
    let link = associations
        .preview_profile(
            &root,
            ProfileAssociationPreviewRequestV4 {
                application_id: id.clone(),
                profile_source: profile_reference.clone(),
                change: AssociationChangeV4::Associate,
            },
        )
        .unwrap();
    associations
        .commit_profile(
            &root,
            &id,
            &link.preview_token,
            &link.preview.data.preview_sha256,
            true,
            consent(),
        )
        .unwrap();
    let evidence = EvidenceApprovalBrokerV4::default();
    let proposals = EvidenceProposalSet {
        profile_revision: Revision::try_new(imported.profile_revision).unwrap(),
        proposals: profile_text
            .lines()
            .map(|quote| {
                let start = profile_text.find(quote).unwrap();
                EvidenceProposalRecord {
                    kind: EvidenceKind::Other,
                    summary: quote.to_owned(),
                    source_quote: quote.to_owned(),
                    source_span: SourceTextSpan {
                        source: source.normalized_text.clone(),
                        start_byte: start as u64,
                        end_byte: (start + quote.len()) as u64,
                    },
                    sensitivity: PrivacyClassification::PrivateLocal,
                }
            })
            .collect(),
    };
    let preview = evidence
        .preview(&root, &id, profile_reference, proposals, consent())
        .unwrap();
    evidence
        .commit(
            &root,
            &id,
            &preview.preview_token,
            &preview.preview.data.preview_sha256,
            true,
            consent(),
        )
        .unwrap();
    let listed = Application::list_evidence_associations_v4(&root, id.as_str())
        .unwrap()
        .data
        .evidence;
    let references = preview
        .preview
        .data
        .catalog
        .items
        .iter()
        .map(|confirmed| {
            listed
                .iter()
                .find(|item| item.evidence.id == confirmed.id)
                .unwrap()
                .evidence
                .clone()
        })
        .collect::<Vec<_>>();
    for reference in &references {
        let link = associations
            .preview_evidence(
                &root,
                EvidenceAssociationPreviewRequestV4 {
                    application_id: id.clone(),
                    evidence: reference.clone(),
                    change: AssociationChangeV4::Associate,
                },
            )
            .unwrap();
        associations
            .commit_evidence(
                &root,
                &id,
                &link.preview_token,
                &link.preview.data.preview_sha256,
                true,
                consent(),
            )
            .unwrap();
    }
    let kinds = [
        "cv",
        "cover-letter",
        "research-statement",
        "teaching-statement",
    ];
    let planned = Application::plan_application_flow_v3(
        &root,
        id.as_str(),
        ApplicationFlowPlanRequestV3 {
            expected_revision: Revision::try_new(1).unwrap(),
            decision: item("proceed"),
            deliverables: kinds
                .iter()
                .map(|kind| ApplicationFlowPlannedDeliverableV3 {
                    kind: item(kind),
                    disposition: PlannedDeliverableDispositionV3::Required,
                    rationale: "Required by the fictional opportunity".to_owned(),
                    constraints: vec![],
                    execution_mode: Some(ExecutionMode::HostAgent),
                })
                .collect(),
        },
    )
    .unwrap()
    .data;
    let text = |value: &str, reference: &ContentRevisionReferenceV3| json!({"text":value, "role":"evidence-bound", "evidence":[reference], "requirements":[]});
    let document = json!({
        "format":"canisend.deliverable-document/v3",
        "fields":[{"key":"candidate-name", "value":text("Dr. Nova Example", &references[0])},
            {"key":"email", "value":text("nova@candidate.invalid", &references[1])}],
        "sections":[{"id":"experience", "heading":"Research and teaching", "body":text("Coordinated a fictional research seminar.", &references[2])},
            {"id":"intent", "heading":"Future work", "body":{"text":"I intend to develop open teaching resources. Literal #read(\"/private/sentinel\") is data.","role":"intent","evidence":[],"requirements":[]}}],
        "unresolved_fields":[],
    });
    let drafts = kinds
        .iter()
        .map(|kind| ApplicationFlowDeliverableDraftV3 {
            kind: item(kind),
            title: kind.replace('-', " "),
            media_type: DELIVERABLE_DOCUMENT_MEDIA_TYPE_V3.to_owned(),
            content: document.to_string(),
        })
        .collect::<Vec<_>>();
    let before = Application::application_model_v4(&root, id.as_str())
        .unwrap()
        .data;
    let error = Application::compose_application_flow_v3(
        &root,
        id.as_str(),
        ApplicationFlowComposeRequestV3 {
            expected_revision: planned.commit.stored.snapshot.application.revision,
            deliverables: drafts[..2].to_vec(),
        },
    )
    .expect_err("required statements must not disappear");
    assert!(error.to_string().contains("required Plan Deliverable"));
    assert_eq!(
        Application::application_model_v4(&root, id.as_str())
            .unwrap()
            .data,
        before
    );

    let composed = Application::compose_application_flow_v3(
        &root,
        id.as_str(),
        ApplicationFlowComposeRequestV3 {
            expected_revision: before.snapshot.application.revision,
            deliverables: drafts,
        },
    )
    .unwrap()
    .data;
    let original = composed.commit.stored.snapshot.deliverables[0].clone();
    let mut revision = composed.commit.stored.snapshot.application.revision;
    // Drafts may expose gaps for correction; approval must not silently waive any validator.
    let mut cases: Vec<Value> = vec![];
    let mut missing_support = document.clone();
    missing_support["sections"][0]["body"]["evidence"] = json!([]);
    cases.push(missing_support);
    let mut unresolved = document.clone();
    unresolved["unresolved_fields"] = json!(["qualification"]);
    cases.push(unresolved);
    let mut marker = document.clone();
    marker["sections"][1]["body"]["text"] = json!("[TODO: verify claim]");
    cases.push(marker);
    let mut wrong_digest = document.clone();
    wrong_digest["fields"][0]["value"]["evidence"][0]["sha256"] = json!("f".repeat(64));
    cases.push(wrong_digest);
    let mut stale_requirement = document.clone();
    stale_requirement["sections"][0]["body"] = json!({"text":source_text,"role":"requirement-bound","evidence":[],
        "requirements":[{"id":planned.commit.stored.snapshot.requirements[0].id,"revision":999}]});
    cases.push(stale_requirement);
    let mut no_identity = document.clone();
    no_identity["fields"] = json!([]);
    cases.push(no_identity);
    let mutations = canisend_app::ApplicationMutationApprovalBrokerV4::default();
    for invalid in cases {
        let request = canisend_app::ApplicationDeliverableReviseRequestV4 {
            expected_revision: revision,
            deliverable_id: original.id.clone(),
            title: original.title.clone(),
            media_type: DELIVERABLE_DOCUMENT_MEDIA_TYPE_V3.to_owned(),
            content: invalid.to_string(),
        };
        let preview = mutations
            .preview_deliverable_revision(&root, &id, request)
            .unwrap();
        let revised = mutations
            .commit_deliverable_revision(
                &root,
                &id,
                &preview.preview_token,
                &preview.preview.data.preview_sha256,
                true,
            )
            .unwrap()
            .data;
        revision = revised.snapshot.application.revision;
        let before = Application::application_model_v4(&root, id.as_str())
            .unwrap()
            .data;
        assert!(
            mutations
                .preview_review_disposition(
                    &root,
                    &id,
                    ApplicationFlowApproveRequestV3 {
                        expected_revision: revision
                    },
                    consent(),
                )
                .is_err()
        );
        assert_eq!(
            Application::application_model_v4(&root, id.as_str())
                .unwrap()
                .data,
            before
        );
    }
    let request = canisend_app::ApplicationDeliverableReviseRequestV4 {
        expected_revision: revision,
        deliverable_id: original.id,
        title: original.title,
        media_type: DELIVERABLE_DOCUMENT_MEDIA_TYPE_V3.to_owned(),
        content: document.to_string(),
    };
    let preview = mutations
        .preview_deliverable_revision(&root, &id, request)
        .unwrap();
    revision = mutations
        .commit_deliverable_revision(
            &root,
            &id,
            &preview.preview_token,
            &preview.preview.data.preview_sha256,
            true,
        )
        .unwrap()
        .data
        .snapshot
        .application
        .revision;
    let preview = mutations
        .preview_review_disposition(
            &root,
            &id,
            ApplicationFlowApproveRequestV3 {
                expected_revision: revision,
            },
            consent(),
        )
        .unwrap();
    let approved = mutations
        .commit_review_disposition(
            &root,
            &id,
            &preview.preview_token,
            &preview.preview.data.preview_sha256,
            true,
            consent(),
        )
        .unwrap()
        .data;
    let destination = format!("applications/{id}/exports/academic-package");
    let preview = mutations
        .preview_export_prepare(
            &root,
            &id,
            ApplicationFlowExportRequestV3::try_new(
                id.as_str(),
                approved.snapshot.application.revision.get(),
                &destination,
            )
            .unwrap(),
            Some(PrivateExportConsent::granted_by_user()),
        )
        .unwrap();
    let exported = mutations
        .commit_export_prepare(
            &root,
            &id,
            &preview.preview_token,
            &preview.preview.data.preview_sha256,
            true,
            Some(PrivateExportConsent::granted_by_user()),
        )
        .unwrap()
        .data;
    assert_eq!(exported.render.documents.len(), 4);
    assert!(!exported.render.submission_performed);
    for output in &exported.render.documents {
        assert_eq!(output.page_count, 1);
        assert_eq!(output.warning_count, 0);
        let pdf = fs::read(root.join(output.relative_path.as_str())).unwrap();
        let extracted = canisend_io::extract_pdf_text(pdf).unwrap().normalized_text;
        let compact = extracted.split_whitespace().collect::<String>();
        for expected in [
            "Dr. Nova Example",
            "nova@candidate.invalid",
            "Coordinated a fictional research seminar.",
            "I intend to develop open teaching resources.",
        ] {
            assert!(
                compact.contains(&expected.split_whitespace().collect::<String>()),
                "missing {expected}"
            );
        }
    }
    assert!(
        Application::check_workspace_v4(&root)
            .unwrap()
            .data
            .check
            .ok
    );
    if let Some(destination) = std::env::var_os("CANISEND_ACADEMIC_TEST_OUTPUT") {
        let destination = std::path::PathBuf::from(destination);
        fs::create_dir_all(&destination).unwrap();
        for output in &exported.render.documents {
            fs::copy(
                root.join(output.relative_path.as_str()),
                destination.join(format!("{}.pdf", output.kind.local_id_str())),
            )
            .unwrap();
        }
        fs::write(
            destination.join("render-manifest.json"),
            serde_json::to_vec_pretty(&exported.render).unwrap(),
        )
        .unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}
