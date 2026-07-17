use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, EntityId, ExecutionMode, ExpectedInputRevision,
    PrivacyClassification, ProfileSourceKind, Revision, SafeRelativePath, Sha256Digest, SourceKind,
    StageExecutionStatus, TaskCompletionRequest, TaskStatus, WorkflowStage,
};
use canisend_store::{
    ArtifactService, CriteriaService, DEFAULT_MAX_BLOB_BYTES, EvidenceService, JobService,
    NewProfileSource, NewSource, ProfileService, StoreError, TaskService, WorkflowService,
    Workspace, WorkspacePaths, verify_backup,
};
use serde_json::json;

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
        7
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
fn evidence_tasks_generate_stable_ids_and_require_source_bound_user_revisions() {
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
fn verified_backup_restores_into_new_workspace() {
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
        service
            .commit(
                None,
                ArtifactKind::EvidenceCatalog,
                b"private evidence",
                &[],
                ActorKind::User,
                "import evidence",
            )
            .expect("artifact commit");
    }
    let result = workspace.backup(&backup_path).expect("backup");
    assert_eq!(result.manifest.blobs.len(), 1);
    verify_backup(&backup_path).expect("backup verifies");
    let restored = Workspace::restore(&backup_path, &restore_path).expect("restore");
    assert_eq!(restored.config.workspace_id, workspace.config.workspace_id);
    assert!(restored.check().expect("restored check").ok);
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
