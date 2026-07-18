use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

use canisend_contracts::{
    ActorKind, ApplicationDecision, ArtifactKind, ArtifactReference, DocumentKind, EntityId,
    ExecutionMode, ExpectedInputRevision, PlannedDocumentRecord, PrivacyClassification,
    ProfileSourceKind, Revision, SafeRelativePath, Sha256Digest, SourceKind, StageExecutionStatus,
    TaskCompletionRequest, TaskStatus, WorkflowStage,
};
use canisend_store::{
    ArtifactService, CriteriaService, DEFAULT_MAX_BLOB_BYTES, DocumentService, EvidenceService,
    JobService, MatchService, NewProfileSource, NewSource, PackageService, PlanService,
    ProfileService, ProjectionService, RenderService, ReviewService, StoreError, TaskService,
    WorkflowService, Workspace, WorkspacePaths, verify_backup,
};
use serde_json::{Value, json};

static NEXT: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "canisend-store-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove stale test directory");
        }
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn workspace_init_discovery_status_and_check_are_consistent() {
    let root = TestDirectory::new("workspace");
    let workspace = Workspace::init(root.path()).expect("workspace initializes");
    let nested = root.path().join("jobs/example/workspace");
    fs::create_dir_all(&nested).expect("nested directory");
    let discovered = WorkspacePaths::discover(None, &nested).expect("workspace discovery");
    assert_eq!(discovered.root, root.path());
    assert_eq!(
        workspace.status().expect("status").database_schema_version,
        13
    );
    let check = workspace.check().expect("workspace check");
    assert!(check.ok);
    assert_eq!(check.database_integrity, "ok");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(root.path().join(".canisend"))
                .expect("internal metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(root.path().join("canisend.toml"))
                .expect("config metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn jobs_and_local_sources_are_revisioned_without_identity_merging() {
    let root = TestDirectory::new("job-intake");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let (job, first, second) = {
        let mut jobs = JobService::new(&mut workspace.database, &workspace.blobs);
        let job = jobs
            .create(" Lecturer in Economics ", " University X ", ActorKind::User)
            .expect("job create");
        assert_eq!(job.title, "Lecturer in Economics");
        assert_eq!(job.revision.get(), 1);

        let source = || NewSource {
            kind: SourceKind::LocalFile,
            original_bytes: b"Title  \r\nBody\r\n".to_vec(),
            normalized_text: "Title\nBody\n".to_owned(),
            source_url: None,
            final_url: None,
            content_type: "text/markdown; charset=utf-8".to_owned(),
            redirect_chain: Vec::new(),
            privacy: PrivacyClassification::PrivateLocal,
        };
        let first = jobs
            .import_source(&job.id, source(), ActorKind::User)
            .expect("first source");
        let second = jobs
            .import_source(&job.id, source(), ActorKind::User)
            .expect("second source");
        assert_ne!(first.id, second.id);
        assert_ne!(first.original.id, second.original.id);
        assert_eq!(first.original.sha256, second.original.sha256);
        let current = jobs.get(&job.id).expect("updated job");
        assert_eq!(current.revision.get(), 3);
        assert_eq!(current.source_ids.len(), 2);
        assert_eq!(jobs.sources(&job.id).expect("sources").len(), 2);
        (job, first, second)
    };

    {
        let artifacts = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        );
        assert_eq!(
            artifacts
                .read(&first.original.id, first.original.revision)
                .expect("original bytes"),
            b"Title  \r\nBody\r\n"
        );
        let normalized = first.normalized_text.expect("normalized reference");
        assert_eq!(
            artifacts
                .read(&normalized.id, normalized.revision)
                .expect("normalized bytes"),
            b"Title\nBody\n"
        );
    }

    let mut jobs = JobService::new(&mut workspace.database, &workspace.blobs);
    let archived = jobs.archive(&job.id, ActorKind::User).expect("archive job");
    assert!(archived.archived);
    assert!(jobs.list(false).expect("active jobs").is_empty());
    assert_eq!(jobs.list(true).expect("all jobs").len(), 1);
    assert!(matches!(
        jobs.import_source(
            &job.id,
            NewSource {
                kind: SourceKind::LocalFile,
                original_bytes: b"later".to_vec(),
                normalized_text: "later\n".to_owned(),
                source_url: None,
                final_url: None,
                content_type: "text/plain; charset=utf-8".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        ),
        Err(StoreError::JobArchived(_))
    ));
    assert_ne!(first.id, second.id);
}

#[test]
fn profile_sources_are_private_revisioned_and_invalidate_evidence_only() {
    let root = TestDirectory::new("profile-source");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    assert_eq!(
        ProfileService::new(&mut workspace.database, &workspace.blobs)
            .revision()
            .expect("empty profile revision"),
        0
    );
    let imported = ProfileService::new(&mut workspace.database, &workspace.blobs)
        .import_source(
            NewProfileSource {
                kind: ProfileSourceKind::Markdown,
                original_bytes: b"# Profile\nPhD in Economics\n".to_vec(),
                normalized_text: "# Profile\nPhD in Economics\n".to_owned(),
                content_type: "text/markdown; charset=utf-8".to_owned(),
                sensitivity: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        )
        .expect("profile source");
    assert_eq!(imported.revision.get(), 1);
    assert_eq!(
        ProfileService::new(&mut workspace.database, &workspace.blobs)
            .revision()
            .expect("profile revision"),
        1
    );
    let listed = ProfileService::new(&mut workspace.database, &workspace.blobs)
        .list_sources()
        .expect("profile sources");
    assert_eq!(listed, vec![imported.clone()]);
    assert!(
        !serde_json::to_string(&listed)
            .expect("profile metadata JSON")
            .contains("PhD in Economics")
    );

    let job = JobService::new(&mut workspace.database, &workspace.blobs)
        .create("Lecturer", "University X", ActorKind::User)
        .expect("job");
    JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(
            &job.id,
            NewSource {
                kind: SourceKind::LocalFile,
                original_bytes: b"Teach economics".to_vec(),
                normalized_text: "Teach economics\n".to_owned(),
                source_url: None,
                final_url: None,
                content_type: "text/plain; charset=utf-8".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        )
        .expect("job source");
    WorkflowService::new(&mut workspace.database)
        .start(&job.id)
        .expect("workflow");
    WorkflowService::new(&mut workspace.database)
        .begin_stage(
            &job.id,
            WorkflowStage::Evidence,
            ExecutionMode::HostAgent,
            ActorKind::HostAgent,
        )
        .expect("running evidence");
    ProfileService::new(&mut workspace.database, &workspace.blobs)
        .import_source(
            NewProfileSource {
                kind: ProfileSourceKind::Json,
                original_bytes: b"{\"teaching\":\"Econometrics\"}\n".to_vec(),
                normalized_text: "{\"teaching\":\"Econometrics\"}\n".to_owned(),
                content_type: "application/json; charset=utf-8".to_owned(),
                sensitivity: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        )
        .expect("second profile source");
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow status");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Evidence),
        StageExecutionStatus::Ready
    );
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Parse),
        StageExecutionStatus::Ready,
        "profile changes must not invalidate the independent job parse branch"
    );
}

#[test]
fn evidence_and_match_tasks_enforce_stable_revision_bound_identities() {
    let root = TestDirectory::new("evidence-workflow");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let job = JobService::new(&mut workspace.database, &workspace.blobs)
        .create("Lecturer", "University X", ActorKind::User)
        .expect("job");
    JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(
            &job.id,
            NewSource {
                kind: SourceKind::ManualText,
                original_bytes: b"Teach economics".to_vec(),
                normalized_text: "Teach economics\n".to_owned(),
                source_url: None,
                final_url: None,
                content_type: "text/plain; charset=utf-8".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        )
        .expect("job source");
    ProfileService::new(&mut workspace.database, &workspace.blobs)
        .import_source(
            NewProfileSource {
                kind: ProfileSourceKind::Markdown,
                original_bytes: b"# Profile\nPhD in Economics\n".to_vec(),
                normalized_text: "# Profile\nPhD in Economics\n".to_owned(),
                content_type: "text/markdown; charset=utf-8".to_owned(),
                sensitivity: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        )
        .expect("profile source");
    WorkflowService::new(&mut workspace.database)
        .start(&job.id)
        .expect("workflow");
    let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_evidence_normalization(&job.id, ExecutionMode::HostAgent)
        .expect("evidence task");
    assert_eq!(
        descriptor.profile_revision.expect("profile revision").get(),
        1
    );
    assert_eq!(descriptor.input_artifacts.len(), 1);
    let candidate = json!({
        "profile_revision": 1,
        "proposals": [{
            "kind": "qualification",
            "summary": "Doctorate in economics",
            "source_quote": "PhD in Economics",
            "source_span": {
                "source": descriptor.input_artifacts[0],
                "start_byte": 10,
                "end_byte": 26
            },
            "sensitivity": "private-local"
        }]
    });
    let request = TaskCompletionRequest {
        task_id: descriptor.id.clone(),
        lease_id: descriptor.lease.id.clone(),
        expected_job_revision: descriptor.job_revision,
        expected_inputs: descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate: candidate.clone(),
    };
    let mut agent_invented_id = request.clone();
    agent_invented_id.candidate["proposals"][0]["id"] =
        json!("019f2f55-7c00-7000-8000-000000000401");
    assert!(matches!(
        TaskService::new(&mut workspace.database, &workspace.blobs).complete(&agent_invented_id),
        Err(StoreError::CandidateStructural(_))
    ));
    let proposal_artifact = TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&request)
        .expect("evidence proposals");
    assert_eq!(
        proposal_artifact.artifact.kind,
        ArtifactKind::EvidenceCatalog
    );
    assert!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&request)
            .expect("evidence replay")
            .idempotent
    );
    let proposed = EvidenceService::new(&mut workspace.database, &workspace.blobs)
        .proposed(&job.id)
        .expect("proposed evidence");
    assert_eq!(proposed.items.len(), 1);
    assert!(!proposed.items[0].confirmed);
    let generated_catalog_id = proposed.id.clone();
    let generated_item_id = proposed.items[0].id.clone();

    let mut confirmation = EvidenceService::new(&mut workspace.database, &workspace.blobs)
        .template(&job.id)
        .expect("confirmation template");
    confirmation.items[0].summary = "Corrected doctorate in economics".to_owned();
    confirmation.items[0].excluded = true;
    let mut wrong_span = confirmation.clone();
    wrong_span.items[0].source_span.end_byte = 25;
    assert!(matches!(
        EvidenceService::new(&mut workspace.database, &workspace.blobs).confirm(
            &job.id,
            &serde_json::to_value(wrong_span).expect("invalid evidence JSON"),
        ),
        Err(StoreError::CandidateSemantic(_))
    ));
    let confirmed_artifact = EvidenceService::new(&mut workspace.database, &workspace.blobs)
        .confirm(
            &job.id,
            &serde_json::to_value(&confirmation).expect("evidence JSON"),
        )
        .expect("confirm evidence");
    assert_eq!(confirmed_artifact.revision.get(), 1);
    let confirmed = EvidenceService::new(&mut workspace.database, &workspace.blobs)
        .confirmed(&job.id)
        .expect("confirmed catalog");
    assert_eq!(confirmed.id, generated_catalog_id);
    assert_eq!(confirmed.items[0].id, generated_item_id);
    assert!(confirmed.items[0].excluded);

    let parse_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_job_parse(&job.id, ExecutionMode::HostAgent)
        .expect("parse task");
    let parse_request = TaskCompletionRequest {
        task_id: parse_descriptor.id.clone(),
        lease_id: parse_descriptor.lease.id.clone(),
        expected_job_revision: parse_descriptor.job_revision,
        expected_inputs: parse_descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate: json!({
            "id": "019f2f55-7c00-7000-8000-000000000501",
            "job_id": job.id,
            "title": "Lecturer",
            "institution": "University X",
            "summary": "Teach economics",
            "responsibilities": ["Teach economics"],
            "criteria": [{
                "id": "019f2f55-7c00-7000-8000-000000000502",
                "job_id": job.id,
                "kind": "qualification",
                "requirement": "Demonstrate economics expertise",
                "importance": "essential",
                "source_quote": "Teach economics",
                "source_span": {
                    "source": parse_descriptor.input_artifacts[0],
                    "start_byte": 0,
                    "end_byte": 15
                },
                "confidence_milli": 800,
                "confirmed": false,
                "revision": 1
            }],
            "revision": 1
        }),
    };
    TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&parse_request)
        .expect("parsed job");
    let criteria = CriteriaService::new(&mut workspace.database, &workspace.blobs)
        .template(&job.id)
        .expect("criteria template");
    let criteria_artifact = CriteriaService::new(&mut workspace.database, &workspace.blobs)
        .confirm(
            &job.id,
            &serde_json::to_value(&criteria).expect("criteria JSON"),
        )
        .expect("criteria confirmation");
    let excluded_match_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_evidence_match(&job.id, ExecutionMode::HostAgent)
        .expect("match task");
    assert_eq!(excluded_match_descriptor.input_artifacts.len(), 2);
    let excluded_match_request = TaskCompletionRequest {
        task_id: excluded_match_descriptor.id.clone(),
        lease_id: excluded_match_descriptor.lease.id.clone(),
        expected_job_revision: excluded_match_descriptor.job_revision,
        expected_inputs: excluded_match_descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate: json!({
            "job_id": job.id,
            "criteria_artifact": criteria_artifact,
            "evidence_artifact": confirmed_artifact,
            "proposals": [{
                "criterion": {
                    "id": criteria.criteria[0].id,
                    "revision": criteria.criteria[0].revision
                },
                "evidence": [{
                    "id": confirmed.items[0].id,
                    "revision": confirmed.items[0].revision
                }],
                "strength": "strong",
                "rationale": "The doctorate demonstrates economics expertise.",
                "gap": null,
                "prohibited_claims": ["Do not claim a teaching qualification."]
            }]
        }),
    };
    assert!(matches!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&excluded_match_request),
        Err(StoreError::CandidateSemantic(_))
    ));

    let mut revision = EvidenceService::new(&mut workspace.database, &workspace.blobs)
        .template(&job.id)
        .expect("revision template");
    revision.items[0].summary = "Reviewed doctorate in economics".to_owned();
    revision.items[0].excluded = false;
    let revised_artifact = EvidenceService::new(&mut workspace.database, &workspace.blobs)
        .confirm(
            &job.id,
            &serde_json::to_value(revision).expect("revision JSON"),
        )
        .expect("revise evidence");
    assert_eq!(revised_artifact.id, confirmed_artifact.id);
    assert_eq!(revised_artifact.revision.get(), 2);
    let revised = EvidenceService::new(&mut workspace.database, &workspace.blobs)
        .confirmed(&job.id)
        .expect("revised catalog");
    assert_eq!(revised.revision.get(), 2);
    assert_eq!(revised.items[0].revision.get(), 2);
    assert_eq!(revised.items[0].id, generated_item_id);
    assert!(!revised.items[0].excluded);
    assert_eq!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .get(&excluded_match_descriptor.id)
            .expect("stale match task")
            .status,
        TaskStatus::Stale
    );
    assert!(matches!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&excluded_match_request),
        Err(StoreError::TaskStale(_))
    ));

    let match_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_evidence_match(&job.id, ExecutionMode::ConfiguredProvider)
        .expect("recomputed match task");
    assert_eq!(match_descriptor.required_consents.len(), 2);
    let match_request = TaskCompletionRequest {
        task_id: match_descriptor.id.clone(),
        lease_id: match_descriptor.lease.id.clone(),
        expected_job_revision: match_descriptor.job_revision,
        expected_inputs: match_descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate: json!({
            "job_id": job.id,
            "criteria_artifact": criteria_artifact,
            "evidence_artifact": revised_artifact,
            "proposals": [{
                "criterion": {
                    "id": criteria.criteria[0].id,
                    "revision": criteria.criteria[0].revision
                },
                "evidence": [{
                    "id": revised.items[0].id,
                    "revision": revised.items[0].revision
                }],
                "strength": "strong",
                "rationale": "The doctorate demonstrates economics expertise.",
                "gap": null,
                "prohibited_claims": ["Do not claim a teaching qualification."]
            }]
        }),
    };
    let match_artifact = TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&match_request)
        .expect("validated matches");
    assert_eq!(match_artifact.artifact.kind, ArtifactKind::EvidenceMatches);
    assert!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&match_request)
            .expect("match replay")
            .idempotent
    );
    let matches = MatchService::new(&mut workspace.database, &workspace.blobs)
        .current(&job.id)
        .expect("current matches");
    assert_eq!(matches.matches.len(), 1);
    assert_ne!(matches.matches[0].id, revised.items[0].id);
    assert_eq!(matches.matches[0].criterion.id, criteria.criteria[0].id);
    assert_eq!(matches.matches[0].evidence[0].id, revised.items[0].id);
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow after matching");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Plan),
        StageExecutionStatus::Ready
    );

    let hold_candidate = PlanService::new(&mut workspace.database, &workspace.blobs)
        .template(&job.id)
        .expect("application plan template");
    assert_eq!(hold_candidate.decision, ApplicationDecision::Hold);
    assert!(hold_candidate.blockers.is_empty());
    let mut invented_blocker = hold_candidate.clone();
    invented_blocker
        .blockers
        .push(canisend_contracts::PlanBlockerRecord {
            code: "plan.invented".to_owned(),
            criterion: canisend_contracts::CriterionRevisionReference {
                id: criteria.criteria[0].id.clone(),
                revision: criteria.criteria[0].revision,
            },
            severity: canisend_contracts::PlanBlockerSeverity::Warning,
            description: "Invented blocker".to_owned(),
        });
    assert!(matches!(
        PlanService::new(&mut workspace.database, &workspace.blobs).confirm(
            &job.id,
            &serde_json::to_value(invented_blocker).expect("invented blocker JSON"),
        ),
        Err(StoreError::CandidateSemantic(_))
    ));
    let hold_artifact = PlanService::new(&mut workspace.database, &workspace.blobs)
        .confirm(
            &job.id,
            &serde_json::to_value(&hold_candidate).expect("hold plan JSON"),
        )
        .expect("confirm hold plan");
    let hold_plan = PlanService::new(&mut workspace.database, &workspace.blobs)
        .current(&job.id)
        .expect("current hold plan");
    assert_eq!(hold_plan.decision, ApplicationDecision::Hold);
    assert_eq!(hold_plan.revision.get(), 1);
    let hold_document_ids = hold_plan
        .documents
        .iter()
        .map(|document| (document.kind, document.id.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow after hold decision");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Draft),
        StageExecutionStatus::Blocked
    );
    assert!(
        status
            .blockers
            .iter()
            .any(|blocker| blocker.code == "workflow.decision_not_apply")
    );

    let mut apply_candidate = PlanService::new(&mut workspace.database, &workspace.blobs)
        .template(&job.id)
        .expect("plan revision template");
    apply_candidate.decision = ApplicationDecision::Apply;
    apply_candidate
        .documents
        .iter_mut()
        .find(|document| document.kind == DocumentKind::ResearchStatement)
        .expect("research statement plan")
        .executor = Some(ExecutionMode::ConfiguredProvider);
    let apply_artifact = PlanService::new(&mut workspace.database, &workspace.blobs)
        .confirm(
            &job.id,
            &serde_json::to_value(apply_candidate).expect("apply plan JSON"),
        )
        .expect("confirm apply plan");
    assert_eq!(apply_artifact.id, hold_artifact.id);
    assert_eq!(apply_artifact.revision.get(), 2);
    let apply_plan = PlanService::new(&mut workspace.database, &workspace.blobs)
        .current(&job.id)
        .expect("current apply plan");
    assert_eq!(apply_plan.id, hold_plan.id);
    assert_eq!(apply_plan.revision.get(), 2);
    assert!(apply_plan.documents.iter().all(|document| {
        hold_document_ids.get(&document.kind) == Some(&document.id) && document.revision.get() == 2
    }));
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow after apply decision");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Draft),
        StageExecutionStatus::Ready
    );
    assert!(status.next_actions.iter().any(|action| {
        action
            .action
            .contains("--operation cover-letter-draft --mode host-agent")
    }));

    let planned_documents = apply_plan
        .documents
        .iter()
        .filter(|document| document.requirement != canisend_contracts::DocumentRequirement::Omitted)
        .cloned()
        .collect::<Vec<_>>();
    let mut document_artifacts = Vec::new();
    for (index, planned) in planned_documents.iter().enumerate() {
        let mode = planned.executor.expect("planned executor");
        let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_document_draft(&job.id, planned.kind, mode)
            .expect("document draft task");
        assert_eq!(descriptor.input_artifacts.len(), 5);
        assert_eq!(
            descriptor.required_consents.len(),
            if mode == ExecutionMode::ConfiguredProvider {
                2
            } else {
                1
            }
        );
        let candidate = document_candidate(
            &job.id,
            &apply_artifact,
            planned,
            (&criteria.criteria[0].id, criteria.criteria[0].revision),
            (&revised.items[0].id, revised.items[0].revision),
            false,
        );
        if index == 0 {
            let mut invented_id = candidate.clone();
            invented_id["sections"][0]["id"] = json!("019f2f55-7c00-7000-8000-000000000901");
            let invalid_request = document_request(&descriptor, invented_id);
            assert!(matches!(
                TaskService::new(&mut workspace.database, &workspace.blobs)
                    .complete(&invalid_request),
                Err(StoreError::CandidateStructural(_))
            ));

            let mut unknown_evidence = candidate.clone();
            unknown_evidence["sections"][1]["claims"][0]["citations"][0]["target"]["evidence"]["id"] =
                json!("019f2f55-7c00-7000-8000-000000000902");
            let invalid_request = document_request(&descriptor, unknown_evidence);
            assert!(matches!(
                TaskService::new(&mut workspace.database, &workspace.blobs)
                    .complete(&invalid_request),
                Err(StoreError::CandidateSemantic(_))
            ));
        }
        let request = document_request(&descriptor, candidate);
        let committed = TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&request)
            .expect("structured document");
        assert_eq!(
            committed.artifact.kind,
            document_artifact_kind(planned.kind)
        );
        document_artifacts.push(committed.artifact.clone());
        let current = DocumentService::new(&workspace.database, &workspace.blobs)
            .current(&job.id, planned.kind)
            .expect("current document");
        assert_eq!(current.generation.task_id, descriptor.id);
        assert_eq!(current.generation.execution_mode, mode);
        assert_eq!(current.planned_document.id, planned.id);
        let status = WorkflowService::new(&mut workspace.database)
            .status(&job.id)
            .expect("workflow during drafting");
        assert_eq!(
            workflow_stage_status(&status, WorkflowStage::Draft),
            if index + 1 == planned_documents.len() {
                StageExecutionStatus::Complete
            } else {
                StageExecutionStatus::Ready
            }
        );
    }
    let documents = DocumentService::new(&workspace.database, &workspace.blobs)
        .list(&job.id)
        .expect("current documents");
    assert_eq!(documents.len(), planned_documents.len());
    let document_set = DocumentService::new(&workspace.database, &workspace.blobs)
        .set(&job.id)
        .expect("complete document set");
    assert_eq!(document_set.plan_artifact, apply_artifact);
    assert_eq!(document_set.documents.len(), planned_documents.len());
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow after drafting");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Review),
        StageExecutionStatus::Ready
    );
    let document_set_artifact = status
        .stages
        .iter()
        .find(|state| state.stage == WorkflowStage::Draft)
        .and_then(|state| state.output.clone())
        .expect("document set artifact");
    let cover_artifact = document_artifacts
        .iter()
        .find(|artifact| artifact.kind == ArtifactKind::CoverLetter)
        .expect("cover artifact")
        .clone();
    let cover = documents
        .iter()
        .find(|document| document.kind == DocumentKind::CoverLetter)
        .expect("cover document");
    let review_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_document_review(&job.id, ExecutionMode::ConfiguredProvider)
        .expect("review task");
    assert_eq!(review_descriptor.input_artifacts.len(), 9);
    assert_eq!(review_descriptor.required_consents.len(), 2);
    let review_candidate = json!({
        "job_id": job.id,
        "document_set_artifact": document_set_artifact,
        "findings": [{
            "code": "review.motivation",
            "category": "human-judgement",
            "severity": "warning",
            "message": "Ask the user to confirm that the closing reflects genuine intent.",
            "target": {
                "kind": "document",
                "document": cover_artifact,
                "document_id": cover.id
            },
            "related_targets": [],
            "suggested_resolution": "Confirm or revise the closing with the user."
        }]
    });
    let mut wrong_target = review_candidate.clone();
    wrong_target["findings"][0]["target"]["document_id"] =
        json!("019f2f55-7c00-7000-8000-000000000903");
    assert!(matches!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&document_request(&review_descriptor, wrong_target)),
        Err(StoreError::CandidateSemantic(_))
    ));
    let committed_review = TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&document_request(&review_descriptor, review_candidate))
        .expect("review findings");
    assert_eq!(committed_review.artifact.kind, ArtifactKind::ReviewFindings);
    let review = ReviewService::new(&mut workspace.database, &workspace.blobs)
        .current(&job.id)
        .expect("current review");
    let deterministic = review
        .findings
        .iter()
        .find(|finding| finding.authority == canisend_contracts::FindingAuthority::Deterministic)
        .expect("deterministic placeholder blocker");
    assert_eq!(deterministic.code, "review.placeholder-required");
    assert_eq!(
        deterministic.severity,
        canisend_contracts::FindingSeverity::Blocker
    );
    let human = review
        .findings
        .iter()
        .find(|finding| finding.authority == canisend_contracts::FindingAuthority::HumanReview)
        .expect("human review finding");
    assert_eq!(human.status, canisend_contracts::FindingStatus::Open);
    let deterministic_disposition = json!({
        "job_id": job.id,
        "review_artifact": committed_review.artifact,
        "decisions": [{
            "finding_id": deterministic.id,
            "expected_revision": deterministic.revision,
            "disposition": "dismissed",
            "rationale": "Attempt to bypass a deterministic blocker."
        }]
    });
    assert!(matches!(
        ReviewService::new(&mut workspace.database, &workspace.blobs)
            .confirm(&job.id, &deterministic_disposition,),
        Err(StoreError::CandidateSemantic(_))
    ));
    let mut disposition = ReviewService::new(&mut workspace.database, &workspace.blobs)
        .template(&job.id)
        .expect("review disposition template");
    assert_eq!(disposition.decisions.len(), 1);
    disposition.decisions[0].disposition =
        Some(canisend_contracts::FindingDisposition::AcceptedRisk);
    disposition.decisions[0].rationale =
        Some("The user reviewed and accepts the motivation wording risk.".to_owned());
    let revised_review_artifact = ReviewService::new(&mut workspace.database, &workspace.blobs)
        .confirm(
            &job.id,
            &serde_json::to_value(disposition).expect("disposition JSON"),
        )
        .expect("confirm review disposition");
    assert_eq!(revised_review_artifact.id, committed_review.artifact.id);
    assert_eq!(revised_review_artifact.revision.get(), 2);
    let revised_review = ReviewService::new(&mut workspace.database, &workspace.blobs)
        .current(&job.id)
        .expect("revised review");
    let revised_human = revised_review
        .findings
        .iter()
        .find(|finding| finding.id == human.id)
        .expect("stable human finding");
    assert_eq!(
        revised_human.status,
        canisend_contracts::FindingStatus::AcceptedRisk
    );
    assert_eq!(revised_human.revision.get(), 2);
    let unchanged_deterministic = revised_review
        .findings
        .iter()
        .find(|finding| finding.id == deterministic.id)
        .expect("stable deterministic finding");
    assert_eq!(unchanged_deterministic.revision.get(), 1);
    assert_eq!(
        unchanged_deterministic.status,
        canisend_contracts::FindingStatus::Open
    );
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow after review");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Review),
        StageExecutionStatus::Complete
    );
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Package),
        StageExecutionStatus::Ready
    );
    assert!(
        status
            .next_actions
            .iter()
            .any(|action| action.action.contains("package check --job"))
    );
    let package_artifact = PackageService::new(&mut workspace.database, &workspace.blobs)
        .check(&job.id)
        .expect("deterministic package readiness");
    assert_eq!(package_artifact.kind, ArtifactKind::PackageManifest);
    let package = PackageService::new(&mut workspace.database, &workspace.blobs)
        .current(&job.id)
        .expect("current package manifest");
    assert_eq!(
        package.readiness.state,
        canisend_contracts::ReadinessState::Blocked
    );
    assert_eq!(package.plan_artifact, apply_artifact);
    assert_eq!(package.evidence_artifact, revised_artifact);
    assert_eq!(package.document_set_artifact, document_set_artifact);
    assert_eq!(package.review_artifact, revised_review_artifact);
    assert_eq!(package.documents, document_set.documents);
    assert!(!package.submission_performed);
    assert!(package.readiness.reasons.iter().any(|reason| {
        reason.code == canisend_contracts::ReadinessReasonCode::OpenDeterministicFinding
            && reason.finding_id.as_ref() == Some(&deterministic.id)
    }));
    assert!(!package.readiness.reasons.iter().any(|reason| {
        reason.code == canisend_contracts::ReadinessReasonCode::PendingHumanFinding
    }));
    assert_eq!(
        PackageService::new(&mut workspace.database, &workspace.blobs)
            .check(&job.id)
            .expect("idempotent readiness check"),
        package_artifact
    );
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow after package readiness");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Package),
        StageExecutionStatus::Complete
    );
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Render),
        StageExecutionStatus::Blocked
    );
    assert!(
        status
            .blockers
            .iter()
            .any(|blocker| blocker.code == "workflow.package_blocked")
    );

    WorkflowService::new(&mut workspace.database)
        .rerun(&job.id, WorkflowStage::Draft, ActorKind::User)
        .expect("rerun drafting to resolve the deterministic blocker");
    for planned in &planned_documents {
        let mode = planned.executor.expect("planned executor");
        let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_document_draft(&job.id, planned.kind, mode)
            .expect("replacement document task");
        let candidate = document_candidate(
            &job.id,
            &apply_artifact,
            planned,
            (&criteria.criteria[0].id, criteria.criteria[0].revision),
            (&revised.items[0].id, revised.items[0].revision),
            true,
        );
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&document_request(&descriptor, candidate))
            .expect("replacement structured document");
    }
    let current_set = DocumentService::new(&workspace.database, &workspace.blobs)
        .set(&job.id)
        .expect("replacement document set");
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow after replacement drafts");
    let current_set_artifact = status
        .stages
        .iter()
        .find(|state| state.stage == WorkflowStage::Draft)
        .and_then(|state| state.output.clone())
        .expect("replacement document-set artifact");
    let review_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_document_review(&job.id, ExecutionMode::HostAgent)
        .expect("replacement review task");
    let clean_review = json!({
        "job_id": job.id,
        "document_set_artifact": current_set_artifact,
        "findings": []
    });
    TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&document_request(&review_descriptor, clean_review))
        .expect("clean replacement review");
    let clean_findings = ReviewService::new(&mut workspace.database, &workspace.blobs)
        .current(&job.id)
        .expect("clean current review");
    assert!(clean_findings.findings.is_empty());
    let ready_package_artifact = PackageService::new(&mut workspace.database, &workspace.blobs)
        .check(&job.id)
        .expect("export-ready package");
    let ready_package = PackageService::new(&mut workspace.database, &workspace.blobs)
        .current(&job.id)
        .expect("current export-ready package");
    assert_eq!(
        ready_package.readiness.state,
        canisend_contracts::ReadinessState::ReadyToExport
    );
    assert!(ready_package.readiness.reasons.is_empty());

    let export_directory = SafeRelativePath::try_new(format!("jobs/{}/application", job.id))
        .expect("safe export directory");
    let workspace_root = workspace.paths.root.clone();
    let (export_artifact, export_receipt) =
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .export(&job.id, &export_directory)
            .expect("structured Markdown and JSON export");
    assert_eq!(export_artifact.kind, ArtifactKind::ExportManifest);
    assert_eq!(export_receipt.package_artifact, ready_package_artifact);
    assert_eq!(
        export_receipt.projections.len(),
        current_set.documents.len() * 3 + 1
    );
    assert!(!export_receipt.submission_performed);
    let cover_markdown = export_receipt
        .projections
        .iter()
        .find(|projection| {
            projection.kind == canisend_contracts::ProjectionKind::Markdown
                && projection.source_artifact.kind == ArtifactKind::CoverLetter
        })
        .expect("cover Markdown projection")
        .relative_path
        .clone();
    let cover_json = export_receipt
        .projections
        .iter()
        .find(|projection| {
            projection.kind == canisend_contracts::ProjectionKind::StructuredJson
                && projection.source_artifact.kind == ArtifactKind::CoverLetter
        })
        .expect("cover JSON projection")
        .relative_path
        .clone();
    let cover_typst = export_receipt
        .projections
        .iter()
        .find(|projection| {
            projection.kind == canisend_contracts::ProjectionKind::TypstSource
                && projection.source_artifact.kind == ArtifactKind::CoverLetter
        })
        .expect("cover Typst projection")
        .relative_path
        .clone();
    let markdown_path = workspace_root.join(cover_markdown.as_str());
    let markdown = fs::read_to_string(&markdown_path).expect("Markdown projection");
    assert!(markdown.contains("canisend-claim"));
    assert!(markdown.contains(revised.items[0].id.as_str()));
    let structured: Value = serde_json::from_slice(
        &fs::read(workspace_root.join(cover_json.as_str())).expect("structured JSON projection"),
    )
    .expect("structured JSON");
    assert_eq!(structured["kind"], "cover-letter");
    let typst = fs::read_to_string(workspace_root.join(cover_typst.as_str()))
        .expect("self-contained Typst projection");
    assert!(typst.contains("canisend_render_document"));
    assert!(typst.contains("Structured artifacts remain authoritative"));
    let typst_path = workspace_root.join(cover_typst.as_str());
    fs::write(&typst_path, format!("{typst}\n// user layout edit\n"))
        .expect("edit managed Typst source");
    let inspection =
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .reconcile(&job.id)
            .expect("detect edited Typst source");
    assert!(inspection.iter().any(|record| {
        record.projection.relative_path == cover_typst
            && record.projection.edit_status == canisend_contracts::ProjectionEditStatus::Edited
            && !record.authoritative_changed
    }));
    let restored =
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .replace(&job.id, &cover_typst)
            .expect("explicitly restore generated Typst source");
    assert!(!restored.authoritative_changed);
    assert!(
        !fs::read_to_string(&typst_path)
            .expect("restored Typst source")
            .contains("user layout edit")
    );
    let (current_export_artifact, current_export) =
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .current(&job.id)
            .expect("current export receipt");
    assert_eq!(current_export_artifact, export_artifact);
    assert_eq!(current_export, export_receipt);

    fs::write(
        &markdown_path,
        format!("{markdown}\nUser-edited closing.\n"),
    )
    .expect("edit managed Markdown");
    let inspection =
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .reconcile(&job.id)
            .expect("detect edited projections");
    assert!(inspection.iter().any(|record| {
        record.projection.relative_path == cover_markdown
            && record.projection.edit_status == canisend_contracts::ProjectionEditStatus::Edited
            && !record.authoritative_changed
    }));
    assert!(matches!(
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .export(&job.id, &export_directory),
        Err(StoreError::ProjectionEdited(_))
    ));
    let preserved_path = SafeRelativePath::try_new(format!(
        "jobs/{}/application/cover-letter.user-edit.md",
        job.id
    ))
    .expect("preserved edit path");
    let copied = ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
        .copy_as_new(&job.id, &cover_markdown, &preserved_path)
        .expect("preserve edit and restore managed projection");
    assert_eq!(
        copied.action,
        canisend_contracts::ProjectionReconcileAction::CopyAsNew
    );
    assert!(!copied.authoritative_changed);
    assert!(
        fs::read_to_string(workspace_root.join(preserved_path.as_str()))
            .expect("preserved edit")
            .contains("User-edited closing")
    );
    assert!(
        !fs::read_to_string(&markdown_path)
            .expect("restored managed Markdown")
            .contains("User-edited closing")
    );

    fs::write(&markdown_path, "temporary replacement edit\n").expect("second edit");
    let replaced =
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .replace(&job.id, &cover_markdown)
            .expect("explicitly discard edit");
    assert_eq!(
        replaced.action,
        canisend_contracts::ProjectionReconcileAction::Replace
    );
    assert!(!replaced.authoritative_changed);
    fs::remove_file(workspace_root.join(cover_json.as_str())).expect("remove managed JSON");
    let inspection =
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .reconcile(&job.id)
            .expect("detect missing projection");
    assert!(inspection.iter().any(|record| {
        record.projection.relative_path == cover_json
            && record.projection.edit_status == canisend_contracts::ProjectionEditStatus::Missing
    }));
    ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
        .replace(&job.id, &cover_json)
        .expect("restore missing JSON projection");
    assert!(workspace_root.join(cover_json.as_str()).is_file());

    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow ready to render");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Render),
        StageExecutionStatus::Ready
    );
    assert!(
        status
            .next_actions
            .iter()
            .any(|action| action.action.contains("render build --job"))
    );
    fs::write(
        &typst_path,
        "#read(\"/private/user-edited-projection-must-not-be-trusted\")\n",
    )
    .expect("edit the non-authoritative Typst projection");
    let (render_artifact, render_manifest) =
        RenderService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .build(&job.id)
            .expect("in-process revision-bound render");
    assert_eq!(render_artifact.kind, ArtifactKind::RenderManifest);
    assert_eq!(render_manifest.package_artifact, ready_package_artifact);
    assert_eq!(render_manifest.documents.len(), current_set.documents.len());
    assert!(!render_manifest.submission_performed);
    for document in &render_manifest.documents {
        assert_eq!(document.typst_artifact.kind, ArtifactKind::TypstSource);
        assert_eq!(document.pdf_artifact.kind, ArtifactKind::Pdf);
        assert!(document.page_count > 0);
        assert!(document.byte_count > 0);
        let pdf = workspace
            .blobs
            .read_verified(&document.pdf_artifact.sha256, DEFAULT_MAX_BLOB_BYTES)
            .expect("validated PDF blob");
        assert_eq!(
            canisend_io::validate_rendered_pdf(&pdf).expect("parse stored PDF"),
            document.page_count
        );
    }
    assert_eq!(
        RenderService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .build(&job.id)
            .expect("idempotent render"),
        (render_artifact.clone(), render_manifest.clone())
    );
    let rendered_directory =
        SafeRelativePath::try_new(format!("jobs/{}/rendered", job.id)).expect("render directory");
    let (exported_artifact, exported_manifest, rendered_paths) =
        RenderService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .export(&job.id, &rendered_directory)
            .expect("explicit PDF export service");
    assert_eq!(exported_artifact, render_artifact);
    assert_eq!(exported_manifest, render_manifest);
    assert_eq!(rendered_paths.len(), current_set.documents.len() + 1);
    assert!(rendered_paths.iter().all(|path| {
        workspace_root.join(path.as_str()).is_file()
            && path
                .as_str()
                .starts_with(&format!("jobs/{}/rendered/", job.id))
    }));
    assert!(matches!(
        RenderService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .export(&job.id, &rendered_directory),
        Err(StoreError::ProjectionUnmanagedConflict(_))
    ));
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow complete after render");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Render),
        StageExecutionStatus::Complete
    );

    ProfileService::new(&mut workspace.database, &workspace.blobs)
        .import_source(
            NewProfileSource {
                kind: ProfileSourceKind::PlainText,
                original_bytes: b"New evidence".to_vec(),
                normalized_text: "New evidence\n".to_owned(),
                content_type: "text/plain; charset=utf-8".to_owned(),
                sensitivity: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        )
        .expect("profile revision");
    assert!(matches!(
        EvidenceService::new(&mut workspace.database, &workspace.blobs).confirmed(&job.id),
        Err(StoreError::WorkflowConflict(_))
    ));
    assert!(matches!(
        MatchService::new(&mut workspace.database, &workspace.blobs).current(&job.id),
        Err(StoreError::WorkflowConflict(_))
    ));
    assert!(matches!(
        PlanService::new(&mut workspace.database, &workspace.blobs).current(&job.id),
        Err(StoreError::WorkflowConflict(_))
    ));
    assert!(
        DocumentService::new(&workspace.database, &workspace.blobs)
            .list(&job.id)
            .expect("stale documents are not current")
            .is_empty()
    );
    assert!(matches!(
        DocumentService::new(&workspace.database, &workspace.blobs).set(&job.id),
        Err(StoreError::WorkflowConflict(_))
    ));
    assert!(matches!(
        ReviewService::new(&mut workspace.database, &workspace.blobs).current(&job.id),
        Err(StoreError::WorkflowConflict(_))
    ));
    assert!(matches!(
        PackageService::new(&mut workspace.database, &workspace.blobs).current(&job.id),
        Err(StoreError::WorkflowConflict(_))
    ));
    assert!(matches!(
        ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .current(&job.id),
        Err(StoreError::WorkflowConflict(_))
    ));
    assert!(matches!(
        RenderService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .current(&job.id),
        Err(StoreError::WorkflowConflict(_))
    ));
    let status = WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("workflow after profile change");
    assert_eq!(
        workflow_stage_status(&status, WorkflowStage::Evidence),
        StageExecutionStatus::Ready
    );
}

#[test]
fn agent_tasks_validate_commit_idempotently_and_detect_changed_jobs() {
    let root = TestDirectory::new("agent-task");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let job = JobService::new(&mut workspace.database, &workspace.blobs)
        .create("Lecturer in Economics", "University X", ActorKind::User)
        .expect("job");
    let source = || NewSource {
        kind: SourceKind::LocalFile,
        original_bytes: b"Teach economics".to_vec(),
        normalized_text: "Teach economics\n".to_owned(),
        source_url: None,
        final_url: None,
        content_type: "text/plain; charset=utf-8".to_owned(),
        redirect_chain: Vec::new(),
        privacy: PrivacyClassification::PrivateLocal,
    };
    JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(&job.id, source(), ActorKind::User)
        .expect("source");
    WorkflowService::new(&mut workspace.database)
        .start(&job.id)
        .expect("workflow");
    let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_job_parse(&job.id, ExecutionMode::HostAgent)
        .expect("prepared task");
    assert_eq!(descriptor.input_artifacts.len(), 1);
    assert_eq!(descriptor.private_read_scope, descriptor.input_artifacts);
    let export_directory = workspace.paths.root.join("agent/task-inputs");
    let exported = TaskService::new(&mut workspace.database, &workspace.blobs)
        .export_inputs(&descriptor.id, &export_directory, false)
        .expect("scoped input export");
    assert_eq!(exported.files.len(), 1);
    assert_eq!(
        fs::read(export_directory.join(exported.files[0].relative_path.as_str()))
            .expect("exported body"),
        b"Teach economics\n"
    );
    assert!(export_directory.join("canisend-task-inputs.json").is_file());
    assert!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .export_inputs(
                &descriptor.id,
                &workspace.paths.root.join(".canisend/forbidden"),
                false,
            )
            .is_err()
    );
    let candidate = json!({
        "id": "019f2f55-7c00-7000-8000-000000000201",
        "job_id": job.id,
        "title": "Lecturer in Economics",
        "institution": "University X",
        "summary": "Teach economics",
        "responsibilities": ["Teach economics"],
        "criteria": [{
            "id": "019f2f55-7c00-7000-8000-000000000202",
            "job_id": job.id,
            "kind": "teaching",
            "requirement": "Evidence of university-level teaching",
            "importance": "essential",
            "source_quote": "Teach economics",
            "source_span": {
                "source": descriptor.input_artifacts[0],
                "start_byte": 0,
                "end_byte": 15
            },
            "confidence_milli": 950,
            "confirmed": false,
            "revision": 1
        }],
        "revision": 1
    });
    let request = TaskCompletionRequest {
        task_id: descriptor.id.clone(),
        lease_id: descriptor.lease.id.clone(),
        expected_job_revision: descriptor.job_revision,
        expected_inputs: descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate: candidate.clone(),
    };
    let mut invalid = request.clone();
    invalid.candidate = json!({"requirement": 3});
    assert!(matches!(
        TaskService::new(&mut workspace.database, &workspace.blobs).complete(&invalid),
        Err(StoreError::CandidateStructural(_))
    ));
    assert_eq!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .get(&descriptor.id)
            .expect("task state")
            .status,
        TaskStatus::Prepared
    );
    let committed = TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&request)
        .expect("commit");
    assert!(!committed.idempotent);
    let replay = TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&request)
        .expect("idempotent replay");
    assert!(replay.idempotent);
    assert_eq!(replay.artifact, committed.artifact);

    let mut template = CriteriaService::new(&mut workspace.database, &workspace.blobs)
        .template(&job.id)
        .expect("editable criteria template");
    assert!(
        template
            .criteria
            .iter()
            .all(|criterion| criterion.confirmed)
    );
    template.criteria[0].requirement = "Corrected evidence of economics teaching".to_owned();
    let mut invalid_template = template.clone();
    invalid_template.criteria[0].source_span.end_byte = 14;
    assert!(matches!(
        CriteriaService::new(&mut workspace.database, &workspace.blobs).confirm(
            &job.id,
            &serde_json::to_value(invalid_template).expect("invalid criteria JSON"),
        ),
        Err(StoreError::CandidateSemantic(_))
    ));
    let criteria_artifact = CriteriaService::new(&mut workspace.database, &workspace.blobs)
        .confirm(
            &job.id,
            &serde_json::to_value(&template).expect("criteria JSON"),
        )
        .expect("confirm criteria");
    assert_eq!(criteria_artifact.kind, ArtifactKind::Criteria);
    assert_eq!(
        CriteriaService::new(&mut workspace.database, &workspace.blobs)
            .confirmed(&job.id)
            .expect("confirmed criteria"),
        template
    );

    WorkflowService::new(&mut workspace.database)
        .rerun(&job.id, WorkflowStage::Parse, ActorKind::User)
        .expect("rerun parse");
    let provider_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_job_parse(&job.id, ExecutionMode::ConfiguredProvider)
        .expect("provider task");
    let provider_directory = workspace.paths.root.join("provider/task-inputs");
    fs::create_dir(workspace.paths.root.join("provider")).expect("provider work directory");
    assert!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .export_inputs(&provider_descriptor.id, &provider_directory, false)
            .is_err()
    );
    assert!(!provider_directory.exists());
    TaskService::new(&mut workspace.database, &workspace.blobs)
        .export_inputs(&provider_descriptor.id, &provider_directory, true)
        .expect("provider-scoped export");
    let provider_request = TaskCompletionRequest {
        task_id: provider_descriptor.id.clone(),
        lease_id: provider_descriptor.lease.id.clone(),
        expected_job_revision: provider_descriptor.job_revision,
        expected_inputs: provider_descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate: candidate.clone(),
    };
    TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&provider_request)
        .expect("provider candidate uses the shared validator");

    WorkflowService::new(&mut workspace.database)
        .rerun(&job.id, WorkflowStage::Parse, ActorKind::User)
        .expect("rerun parse after provider");
    let stale_descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_job_parse(&job.id, ExecutionMode::HostAgent)
        .expect("second task");
    JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(&job.id, source(), ActorKind::User)
        .expect("changed job inputs");
    let stale_request = TaskCompletionRequest {
        task_id: stale_descriptor.id.clone(),
        lease_id: stale_descriptor.lease.id.clone(),
        expected_job_revision: stale_descriptor.job_revision,
        expected_inputs: stale_descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate,
    };
    assert!(matches!(
        TaskService::new(&mut workspace.database, &workspace.blobs).complete(&stale_request),
        Err(StoreError::TaskStale(_))
    ));
    assert_eq!(
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .get(&stale_descriptor.id)
            .expect("stale state")
            .status,
        TaskStatus::Stale
    );

    WorkflowService::new(&mut workspace.database)
        .status(&job.id)
        .expect("reconcile workflow");
    let cancelled = TaskService::new(&mut workspace.database, &workspace.blobs)
        .prepare_job_parse(&job.id, ExecutionMode::ConfiguredProvider)
        .expect("third task");
    assert_eq!(
        cancelled.required_consents.len(),
        2,
        "provider parse requires read and send consent"
    );
    let state = TaskService::new(&mut workspace.database, &workspace.blobs)
        .cancel(&cancelled.id)
        .expect("cancel");
    assert_eq!(state.status, TaskStatus::Cancelled);
}

#[test]
fn recovery_concurrent_host_agents_commit_one_idempotent_result() {
    let root = TestDirectory::new("concurrent-agent-task");
    let request = {
        let mut workspace = Workspace::init(root.path()).expect("workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Lecturer in Economics", "University X", ActorKind::User)
            .expect("job");
        JobService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                &job.id,
                NewSource {
                    kind: SourceKind::LocalFile,
                    original_bytes: b"Teach economics".to_vec(),
                    normalized_text: "Teach economics\n".to_owned(),
                    source_url: None,
                    final_url: None,
                    content_type: "text/plain; charset=utf-8".to_owned(),
                    redirect_chain: Vec::new(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("source");
        WorkflowService::new(&mut workspace.database)
            .start(&job.id)
            .expect("workflow");
        let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_job_parse(&job.id, ExecutionMode::HostAgent)
            .expect("prepared task");
        let candidate = json!({
            "id": "019f2f55-7c00-7000-8000-000000000801",
            "job_id": job.id,
            "title": "Lecturer in Economics",
            "institution": "University X",
            "summary": "Teach economics",
            "responsibilities": ["Teach economics"],
            "criteria": [{
                "id": "019f2f55-7c00-7000-8000-000000000802",
                "job_id": job.id,
                "kind": "teaching",
                "requirement": "Evidence of university-level teaching",
                "importance": "essential",
                "source_quote": "Teach economics",
                "source_span": {
                    "source": descriptor.input_artifacts[0],
                    "start_byte": 0,
                    "end_byte": 15
                },
                "confidence_milli": 950,
                "confirmed": false,
                "revision": 1
            }],
            "revision": 1
        });
        document_request(&descriptor, candidate)
    };

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let root = root.path().to_path_buf();
        let request = request.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let mut workspace =
                Workspace::open_from(Some(&root), &root).map_err(|error| error.to_string())?;
            barrier.wait();
            TaskService::new(&mut workspace.database, &workspace.blobs)
                .complete(&request)
                .map_err(|error| error.to_string())
        }));
    }
    barrier.wait();
    let commits = handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .expect("host-agent thread")
                .expect("concurrent completion")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        commits.iter().filter(|commit| !commit.idempotent).count(),
        1
    );
    assert_eq!(commits.iter().filter(|commit| commit.idempotent).count(), 1);
    assert_eq!(commits[0].artifact, commits[1].artifact);
    let workspace = Workspace::open_from(Some(root.path()), root.path()).expect("reopen workspace");
    assert!(workspace.check().expect("post-concurrency check").ok);
}

#[test]
fn workflow_kernel_enforces_graph_modes_and_scoped_rerun() {
    let root = TestDirectory::new("workflow-kernel");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let job = JobService::new(&mut workspace.database, &workspace.blobs)
        .create("Lecturer", "University X", ActorKind::User)
        .expect("job");
    let source = JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(
            &job.id,
            NewSource {
                kind: SourceKind::LocalFile,
                original_bytes: b"Private sentinel: teach economics".to_vec(),
                normalized_text: "Private sentinel: teach economics\n".to_owned(),
                source_url: None,
                final_url: None,
                content_type: "text/plain; charset=utf-8".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        )
        .expect("source");
    let started = WorkflowService::new(&mut workspace.database)
        .start(&job.id)
        .expect("workflow start");
    let replay = WorkflowService::new(&mut workspace.database)
        .start(&job.id)
        .expect("idempotent start");
    assert_eq!(started.run_id, replay.run_id);
    assert_eq!(
        workflow_stage_status(&started, WorkflowStage::Intake),
        StageExecutionStatus::Complete
    );
    assert_eq!(
        workflow_stage_status(&started, WorkflowStage::Parse),
        StageExecutionStatus::Ready
    );
    assert_eq!(
        workflow_stage_status(&started, WorkflowStage::Evidence),
        StageExecutionStatus::Ready
    );
    assert!(
        !serde_json::to_string(&started)
            .expect("workflow JSON")
            .contains("Private sentinel")
    );

    let running = WorkflowService::new(&mut workspace.database)
        .begin_stage(
            &job.id,
            WorkflowStage::Parse,
            ExecutionMode::HostAgent,
            ActorKind::HostAgent,
        )
        .expect("begin parse");
    assert_eq!(
        workflow_stage_status(&running, WorkflowStage::Parse),
        StageExecutionStatus::Running
    );
    let parsed = {
        let mut artifacts = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        );
        let revision = artifacts
            .commit(
                None,
                ArtifactKind::ParsedJob,
                br#"{"title":"Lecturer"}"#,
                &[source.normalized_text.clone().expect("normalized")],
                ActorKind::HostAgent,
                "workflow kernel fixture",
            )
            .expect("parsed artifact");
        artifacts
            .reference(&revision.artifact_id)
            .expect("reference")
    };
    let parsed_complete = WorkflowService::new(&mut workspace.database)
        .complete_stage(&job.id, WorkflowStage::Parse, &parsed, ActorKind::HostAgent)
        .expect("complete parse");
    assert_eq!(
        workflow_stage_status(&parsed_complete, WorkflowStage::Criteria),
        StageExecutionStatus::Ready
    );
    assert!(matches!(
        WorkflowService::new(&mut workspace.database).begin_stage(
            &job.id,
            WorkflowStage::Criteria,
            ExecutionMode::HostAgent,
            ActorKind::HostAgent,
        ),
        Err(StoreError::WorkflowConflict(_))
    ));
    let awaiting = WorkflowService::new(&mut workspace.database)
        .begin_stage(
            &job.id,
            WorkflowStage::Criteria,
            ExecutionMode::UserDecision,
            ActorKind::User,
        )
        .expect("await criteria decision");
    assert_eq!(
        workflow_stage_status(&awaiting, WorkflowStage::Criteria),
        StageExecutionStatus::AwaitingUser
    );

    let rerun = WorkflowService::new(&mut workspace.database)
        .rerun(&job.id, WorkflowStage::Parse, ActorKind::User)
        .expect("scoped rerun");
    assert_eq!(
        workflow_stage_status(&rerun, WorkflowStage::Parse),
        StageExecutionStatus::Ready
    );
    assert_eq!(
        workflow_stage_status(&rerun, WorkflowStage::Criteria),
        StageExecutionStatus::Stale
    );
    assert_eq!(
        workflow_stage_status(&rerun, WorkflowStage::Evidence),
        StageExecutionStatus::Ready
    );
    assert!(
        workspace
            .check()
            .expect("workspace check")
            .stale_artifact_ids
            .contains(&parsed.id)
    );

    let late_job = JobService::new(&mut workspace.database, &workspace.blobs)
        .create("Late source", "University Y", ActorKind::User)
        .expect("late job");
    let blocked = WorkflowService::new(&mut workspace.database)
        .start(&late_job.id)
        .expect("blocked workflow");
    assert_eq!(
        workflow_stage_status(&blocked, WorkflowStage::Intake),
        StageExecutionStatus::Blocked
    );
    JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(
            &late_job.id,
            NewSource {
                kind: SourceKind::ManualText,
                original_bytes: b"Added later".to_vec(),
                normalized_text: "Added later\n".to_owned(),
                source_url: None,
                final_url: None,
                content_type: "text/plain; charset=utf-8".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            },
            ActorKind::User,
        )
        .expect("late source");
    let reconciled = WorkflowService::new(&mut workspace.database)
        .status(&late_job.id)
        .expect("reconciled workflow");
    assert_eq!(
        workflow_stage_status(&reconciled, WorkflowStage::Intake),
        StageExecutionStatus::Complete
    );
    assert_eq!(
        workflow_stage_status(&reconciled, WorkflowStage::Parse),
        StageExecutionStatus::Ready
    );
}

fn document_candidate(
    job_id: &EntityId,
    plan: &ArtifactReference,
    planned: &PlannedDocumentRecord,
    criterion: (&EntityId, Revision),
    evidence: (&EntityId, Revision),
    resolve_required_placeholders: bool,
) -> Value {
    let (criterion_id, criterion_revision) = criterion;
    let (evidence_id, evidence_revision) = evidence;
    let applicant_claim = || {
        json!({
            "text": "I hold a reviewed doctorate in economics.",
            "classification": "applicant-fact",
            "citations": [{
                "target": {
                    "kind": "evidence",
                    "evidence": {
                        "id": evidence_id,
                        "revision": evidence_revision
                    }
                },
                "purpose": "Support the confirmed economics qualification"
            }]
        })
    };
    let sections = match planned.kind {
        DocumentKind::CoverLetter => vec![
            json!({
                "kind": "opening",
                "heading": null,
                "body": "I am applying for the Lecturer position at University X.",
                "claims": [{
                    "text": "I am applying for the Lecturer position.",
                    "classification": "user-intent",
                    "citations": []
                }]
            }),
            json!({
                "kind": "fit",
                "heading": "Fit",
                "body": "The role requires economics expertise, and I hold a reviewed doctorate in economics.",
                "claims": [
                    applicant_claim(),
                    {
                        "text": "The role requires economics expertise.",
                        "classification": "job-requirement",
                        "citations": [{
                            "target": {
                                "kind": "criterion",
                                "criterion": {
                                    "id": criterion_id,
                                    "revision": criterion_revision
                                }
                            },
                            "purpose": "Repeat the exact confirmed job criterion"
                        }]
                    }
                ]
            }),
            json!({
                "kind": "closing",
                "heading": null,
                "body": "I would welcome the opportunity to discuss my application.",
                "claims": [{
                    "text": "I would welcome a discussion.",
                    "classification": "user-intent",
                    "citations": []
                }]
            }),
        ],
        DocumentKind::ResearchStatement => vec![json!({
            "kind": "research",
            "heading": "Research foundation",
            "body": "My research foundation includes a reviewed doctorate in economics.",
            "claims": [applicant_claim()]
        })],
        DocumentKind::TeachingStatement => vec![json!({
            "kind": "teaching",
            "heading": "Teaching foundation",
            "body": "My subject foundation includes a reviewed doctorate in economics.",
            "claims": [applicant_claim()]
        })],
        DocumentKind::Cv => vec![json!({
            "kind": "education",
            "heading": "Education",
            "body": "Reviewed doctorate in economics.",
            "claims": [applicant_claim()]
        })],
    };
    json!({
        "job_id": job_id,
        "plan_artifact": plan,
        "planned_document": {
            "id": planned.id,
            "revision": planned.revision
        },
        "kind": planned.kind,
        "title": format!("Lecturer application {:?}", planned.kind),
        "sections": sections,
        "placeholders": if planned.kind == DocumentKind::CoverLetter {
            json!([{
                "key": "contact-name",
                "instruction": "Confirm the addressee before packaging",
                "required": true,
                "resolution": if resolve_required_placeholders {
                    json!("Hiring Committee")
                } else {
                    Value::Null
                }
            }])
        } else {
            json!([])
        }
    })
}

fn document_request(
    descriptor: &canisend_contracts::TaskDescriptor,
    candidate: Value,
) -> TaskCompletionRequest {
    TaskCompletionRequest {
        task_id: descriptor.id.clone(),
        lease_id: descriptor.lease.id.clone(),
        expected_job_revision: descriptor.job_revision,
        expected_inputs: descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect(),
        candidate,
    }
}

fn document_artifact_kind(kind: DocumentKind) -> ArtifactKind {
    match kind {
        DocumentKind::CoverLetter => ArtifactKind::CoverLetter,
        DocumentKind::ResearchStatement => ArtifactKind::ResearchStatement,
        DocumentKind::TeachingStatement => ArtifactKind::TeachingStatement,
        DocumentKind::Cv => ArtifactKind::Cv,
    }
}

fn workflow_stage_status(
    workflow: &canisend_contracts::WorkflowStatusData,
    stage: WorkflowStage,
) -> StageExecutionStatus {
    workflow
        .stages
        .iter()
        .find(|state| state.stage == stage)
        .expect("stage state")
        .status
}

#[test]
fn blobs_are_bounded_immutable_verified_and_auditable() {
    let root = TestDirectory::new("blob");
    let workspace = Workspace::init(root.path()).expect("workspace");
    let digest = workspace
        .blobs
        .put_bytes(b"evidence")
        .expect("blob publish");
    assert_eq!(
        workspace
            .blobs
            .read_verified(&digest, DEFAULT_MAX_BLOB_BYTES)
            .expect("verified read"),
        b"evidence"
    );
    let check = workspace.check().expect("check");
    assert_eq!(check.unreferenced_blobs, vec![digest.clone()]);
    assert!(Sha256Digest::try_new("../../escape").is_err());

    let destination = workspace.blobs.path_for(&digest);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&destination)
                .expect("blob metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    fs::write(&destination, b"collision").expect("replace with collision fixture");
    assert!(matches!(
        workspace.blobs.put_bytes(b"evidence"),
        Err(StoreError::BlobCollision(_))
    ));

    let mut reader = FailingReader { emitted: false };
    assert!(
        workspace
            .blobs
            .put_reader(&mut reader, DEFAULT_MAX_BLOB_BYTES)
            .is_err()
    );
    assert_eq!(
        fs::read_dir(root.path().join(".canisend/tmp"))
            .expect("temporary directory")
            .count(),
        0
    );
}

#[cfg(unix)]
#[test]
fn workspace_and_blob_symlinks_fail_closed() {
    use std::os::unix::fs::symlink;

    let root = TestDirectory::new("symlink");
    let workspace = Workspace::init(root.path()).expect("workspace");
    let digest = workspace.blobs.put_bytes(b"private").expect("blob");
    let blob_path = workspace.blobs.path_for(&digest);
    fs::remove_file(&blob_path).expect("remove blob");
    symlink("/tmp", &blob_path).expect("blob symlink");
    assert!(
        workspace
            .blobs
            .verify(&digest, DEFAULT_MAX_BLOB_BYTES)
            .is_err()
    );

    fs::remove_file(&blob_path).expect("remove blob symlink");
    fs::remove_dir_all(&workspace.paths.blob_container).expect("remove blob container");
    symlink("/tmp", &workspace.paths.blob_container).expect("blob container symlink");
    assert!(
        workspace
            .blobs
            .put_bytes(b"must not escape the workspace")
            .is_err()
    );
    fs::remove_file(&workspace.paths.blob_container).expect("remove blob container symlink");

    let internal = root.path().join(".canisend");
    fs::remove_dir_all(&internal).expect("remove internal");
    symlink("/tmp", &internal).expect("internal symlink");
    assert!(Workspace::open_from(Some(root.path()), root.path()).is_err());
}

#[test]
fn artifact_commit_stales_dependents_and_projection_repairs() {
    let root = TestDirectory::new("artifact");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let (source, derived) = {
        let mut service = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        );
        let source = service
            .commit(
                None,
                ArtifactKind::SourceNormalizedText,
                b"source v1",
                &[],
                ActorKind::User,
                "import source",
            )
            .expect("source commit");
        let source_reference = service
            .reference(&source.artifact_id)
            .expect("source reference");
        let derived = service
            .commit(
                None,
                ArtifactKind::CoverLetter,
                b"derived v1",
                &[source_reference],
                ActorKind::HostAgent,
                "draft from evidence",
            )
            .expect("derived commit");

        let missing_dependency = ArtifactReference {
            kind: ArtifactKind::EvidenceCatalog,
            id: EntityId::try_new("019f2f55-7c00-7000-8000-000000009999").expect("fixture id"),
            revision: Revision::try_new(1).expect("fixture revision"),
            sha256: Sha256Digest::try_new("a".repeat(64)).expect("fixture digest"),
        };
        assert!(matches!(
            service.commit(
                None,
                ArtifactKind::CoverLetter,
                b"published before rejected transaction",
                &[missing_dependency],
                ActorKind::HostAgent,
                "exercise transaction rollback",
            ),
            Err(StoreError::DependencyConflict(_))
        ));

        service
            .commit(
                Some(source.artifact_id.clone()),
                ArtifactKind::SourceNormalizedText,
                b"source v2",
                &[],
                ActorKind::User,
                "correct source",
            )
            .expect("source update");

        let collision = root.path().join("jobs/example");
        fs::write(&collision, b"not a directory").expect("projection collision");
        assert!(
            service
                .project(
                    &derived.artifact_id,
                    derived.revision,
                    &SafeRelativePath::try_new("jobs/example/cover-letter.md")
                        .expect("projection path"),
                )
                .is_err()
        );
        assert_eq!(
            service
                .read(&derived.artifact_id, derived.revision)
                .expect("authoritative artifact survives projection failure"),
            b"derived v1"
        );
        fs::remove_file(&collision).expect("remove collision");
        assert_eq!(service.repair_projections().expect("repair projection"), 1);
        (source, derived)
    };
    let check = workspace.check().expect("workspace check");
    assert!(check.stale_artifact_ids.contains(&derived.artifact_id));
    assert!(check.projection_repairs_required.is_empty());
    assert_eq!(check.unreferenced_blobs.len(), 1);
    assert_eq!(
        fs::read(root.path().join("jobs/example/cover-letter.md")).expect("projection"),
        b"derived v1"
    );
    assert!(!check.unreferenced_blobs.contains(&source.sha256));
}

#[test]
fn recovery_verified_backup_restores_into_new_workspace() {
    let root = TestDirectory::new("backup-source");
    let backup = TestDirectory::new("backup-destination");
    let restore = TestDirectory::new("restore-destination");
    let backup_path = backup.path().join("snapshot");
    let restore_path = restore.path().join("workspace");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    {
        let mut service = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        );
        let artifact = service
            .commit(
                None,
                ArtifactKind::EvidenceCatalog,
                b"private evidence",
                &[],
                ActorKind::User,
                "import evidence",
            )
            .expect("artifact commit");
        service
            .project(
                &artifact.artifact_id,
                artifact.revision,
                &SafeRelativePath::try_new("jobs/example/evidence.json").expect("projection path"),
            )
            .expect("raw projection");
    }
    let result = workspace.backup(&backup_path).expect("backup");
    assert_eq!(result.manifest.blobs.len(), 1);
    verify_backup(&backup_path).expect("backup verifies");
    let restored = Workspace::restore(&backup_path, &restore_path).expect("restore");
    assert_eq!(restored.config.workspace_id, workspace.config.workspace_id);
    assert!(restored.check().expect("restored check").ok);
    assert_eq!(
        fs::read(restore_path.join("jobs/example/evidence.json"))
            .expect("projection rebuilt during restore"),
        b"private evidence"
    );
}

#[test]
fn recovery_interrupted_backup_removes_partial_destination() {
    let root = TestDirectory::new("backup-interrupted-source");
    let backup = TestDirectory::new("backup-interrupted-destination");
    let destination = backup.path().join("snapshot");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let artifact = ArtifactService::new(
        &mut workspace.database,
        &workspace.blobs,
        &workspace.paths.root,
    )
    .commit(
        None,
        ArtifactKind::EvidenceCatalog,
        b"private evidence",
        &[],
        ActorKind::User,
        "backup interruption fixture",
    )
    .expect("artifact commit");
    fs::remove_file(workspace.blobs.path_for(&artifact.sha256)).expect("remove referenced blob");
    assert!(workspace.backup(&destination).is_err());
    assert!(!destination.exists());
    if backup.path().exists() {
        assert_eq!(
            fs::read_dir(backup.path()).expect("backup parent").count(),
            0,
            "failed backup must not leave a partial staging directory"
        );
    }
}

#[test]
fn recovery_check_detects_missing_and_corrupted_referenced_blobs() {
    let root = TestDirectory::new("referenced-blob-damage");
    let mut workspace = Workspace::init(root.path()).expect("workspace");
    let missing = ArtifactService::new(
        &mut workspace.database,
        &workspace.blobs,
        &workspace.paths.root,
    )
    .commit(
        None,
        ArtifactKind::SourceOriginal,
        b"missing referenced body",
        &[],
        ActorKind::User,
        "missing recovery fixture",
    )
    .expect("missing artifact");
    let corrupted = ArtifactService::new(
        &mut workspace.database,
        &workspace.blobs,
        &workspace.paths.root,
    )
    .commit(
        None,
        ArtifactKind::SourceNormalizedText,
        b"corrupted referenced body",
        &[],
        ActorKind::User,
        "corrupt recovery fixture",
    )
    .expect("corrupted artifact");
    fs::remove_file(workspace.blobs.path_for(&missing.sha256)).expect("remove referenced blob");
    let corrupt_path = workspace.blobs.path_for(&corrupted.sha256);
    fs::remove_file(&corrupt_path).expect("remove immutable blob before corruption");
    fs::write(&corrupt_path, b"tampered").expect("replace blob with corrupt bytes");

    let check = workspace.check().expect("workspace damage report");
    assert!(!check.ok);
    let invalid_subjects = check
        .issues
        .iter()
        .filter(|issue| issue.code == "blob.reference_invalid")
        .map(|issue| issue.subject.as_str())
        .collect::<Vec<_>>();
    assert!(invalid_subjects.contains(&missing.sha256.as_str()));
    assert!(invalid_subjects.contains(&corrupted.sha256.as_str()));
}

struct FailingReader {
    emitted: bool,
}

impl Read for FailingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.emitted {
            return Err(io::Error::other("synthetic interruption"));
        }
        self.emitted = true;
        buffer[..4].copy_from_slice(b"part");
        Ok(4)
    }
}
