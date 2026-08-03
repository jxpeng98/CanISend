use std::path::{Path, PathBuf};

use canisend_contracts::{
    DocumentKind, EntityId, ExecutionMode, NextAction, TaskCompletionRequest, TaskDescriptor,
    TaskInputExportData, TaskStateData, TaskStatus,
};
use canisend_io::read_task_completion_file;
use canisend_store::{StoreError, TaskService, WorkflowService, Workspace};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, PrivateReadConsent, ProviderSendConsent,
    application::{open_workspace, parse_entity_id},
    compatibility::{
        LegacyCompatibilityAccess, LegacyCompatibilityOperation, job_compatibility_notice,
        task_compatibility_notice,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskOperation {
    JobParse,
    EvidenceNormalize,
    EvidenceMatch,
    CoverLetterDraft,
    ResearchStatementDraft,
    TeachingStatementDraft,
    CvDraft,
    DocumentReview,
}

impl TaskOperation {
    pub const ALL: [Self; 8] = [
        Self::JobParse,
        Self::EvidenceNormalize,
        Self::EvidenceMatch,
        Self::CoverLetterDraft,
        Self::ResearchStatementDraft,
        Self::TeachingStatementDraft,
        Self::CvDraft,
        Self::DocumentReview,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::JobParse => "job-parse",
            Self::EvidenceNormalize => "evidence-normalize",
            Self::EvidenceMatch => "evidence-match",
            Self::CoverLetterDraft => "cover-letter-draft",
            Self::ResearchStatementDraft => "research-statement-draft",
            Self::TeachingStatementDraft => "teaching-statement-draft",
            Self::CvDraft => "cv-draft",
            Self::DocumentReview => "document-review",
        }
    }

    const fn descriptor_operation(self) -> &'static str {
        match self {
            Self::JobParse => "job.parse",
            Self::EvidenceNormalize => "profile.evidence.normalize",
            Self::EvidenceMatch => "evidence.match",
            Self::CoverLetterDraft => "document.draft.cover-letter",
            Self::ResearchStatementDraft => "document.draft.research-statement",
            Self::TeachingStatementDraft => "document.draft.teaching-statement",
            Self::CvDraft => "document.draft.cv",
            Self::DocumentReview => "document.review",
        }
    }

    fn from_descriptor(operation: &str) -> Result<Self, ApplicationError> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.descriptor_operation() == operation)
            .ok_or_else(|| {
                StoreError::Invariant(format!(
                    "task descriptor has unsupported operation: {operation}"
                ))
                .into()
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskExecutionMode {
    HostAgent,
    ConfiguredProvider,
}

impl TaskExecutionMode {
    #[must_use]
    pub const fn as_execution_mode(self) -> ExecutionMode {
        match self {
            Self::HostAgent => ExecutionMode::HostAgent,
            Self::ConfiguredProvider => ExecutionMode::ConfiguredProvider,
        }
    }

    fn from_execution_mode(mode: ExecutionMode) -> Result<Self, ApplicationError> {
        match mode {
            ExecutionMode::HostAgent => Ok(Self::HostAgent),
            ExecutionMode::ConfiguredProvider => Ok(Self::ConfiguredProvider),
            _ => Err(StoreError::Invariant(
                "task descriptor has an unsupported execution mode".to_owned(),
            )
            .into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskPrepareRequest {
    pub job_id: EntityId,
    pub operation: TaskOperation,
    pub mode: TaskExecutionMode,
}

impl TaskPrepareRequest {
    pub fn try_new(
        job_id: &str,
        operation: TaskOperation,
        mode: TaskExecutionMode,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            job_id: parse_entity_id(job_id)?,
            operation,
            mode,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskInputExportRequest {
    pub task_id: EntityId,
    pub destination: PathBuf,
}

impl TaskInputExportRequest {
    pub fn try_new(task_id: &str, destination: PathBuf) -> Result<Self, ApplicationError> {
        Ok(Self {
            task_id: parse_entity_id(task_id)?,
            destination,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCompletionPreviewReadModel {
    pub request: TaskCompletionRequest,
    pub state: TaskStateData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskPrepareAgainReadModel {
    pub previous: TaskStateData,
    pub descriptor: TaskDescriptor,
}

impl Application {
    pub fn prepare_task(
        root: &Path,
        request: TaskPrepareRequest,
    ) -> Result<ActionReceipt<TaskDescriptor>, ApplicationError> {
        let compatibility = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::TaskPrepare,
            LegacyCompatibilityAccess::Write,
            &request.job_id,
        )?;
        let mut workspace = open_workspace(root)?;
        WorkflowService::new(&mut workspace.database).start(&request.job_id)?;
        let descriptor = prepare_descriptor(
            &mut workspace,
            &request.job_id,
            request.operation,
            request.mode,
        )?;
        Ok(prepared_receipt("task.prepare", descriptor).with_compatibility(compatibility))
    }

    pub fn task_state(
        root: &Path,
        task_id: &str,
    ) -> Result<ActionReceipt<TaskStateData>, ApplicationError> {
        let task_id = parse_entity_id(task_id)?;
        let compatibility = task_compatibility_notice(
            root,
            LegacyCompatibilityOperation::TaskShow,
            LegacyCompatibilityAccess::Read,
            &task_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let state = TaskService::new(&mut workspace.database, &workspace.blobs).get(&task_id)?;
        Ok(task_state_receipt("task.show", state).with_compatibility(compatibility))
    }

    pub fn latest_task_for_job(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<Option<TaskStateData>>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let compatibility = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::TaskLatest,
            LegacyCompatibilityAccess::Read,
            &job_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let state =
            TaskService::new(&mut workspace.database, &workspace.blobs).latest_for_job(&job_id)?;
        let status = if state.is_some() {
            "available"
        } else {
            "empty"
        };
        let summary = state.as_ref().map_or_else(
            || format!("No task has been prepared for job {job_id}"),
            |state| {
                format!(
                    "Latest task {} is {}",
                    state.descriptor.id,
                    task_status_text(state.status)
                )
            },
        );
        Ok(ActionReceipt::new("task.latest", status, summary, state)
            .with_compatibility(compatibility))
    }

    pub fn export_task_inputs(
        root: &Path,
        request: TaskInputExportRequest,
        private_read_consent: Option<PrivateReadConsent>,
        provider_send_consent: Option<ProviderSendConsent>,
    ) -> Result<ActionReceipt<TaskInputExportData>, ApplicationError> {
        if private_read_consent.is_none() {
            return Err(read_private_consent_required());
        }
        let compatibility = task_compatibility_notice(
            root,
            LegacyCompatibilityOperation::TaskInputs,
            LegacyCompatibilityAccess::Read,
            &request.task_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let state =
            TaskService::new(&mut workspace.database, &workspace.blobs).get(&request.task_id)?;
        let provider_send_allowed = provider_send_consent.is_some();
        if state.descriptor.execution_mode == ExecutionMode::ConfiguredProvider
            && !provider_send_allowed
        {
            return Err(provider_send_consent_required());
        }
        let exported = TaskService::new(&mut workspace.database, &workspace.blobs).export_inputs(
            &request.task_id,
            &request.destination,
            provider_send_allowed,
        )?;
        let artifacts = exported
            .files
            .iter()
            .map(|file| file.artifact.clone())
            .collect::<Vec<_>>();
        Ok(ActionReceipt::new(
            "task.inputs",
            "exported",
            format!(
                "Exported {} declared task input(s) with manifest {}",
                exported.files.len(),
                exported.manifest_sha256
            ),
            exported,
        )
        .with_artifacts(artifacts)
        .with_compatibility(compatibility))
    }

    pub fn preview_task_completion_file(
        root: &Path,
        file: &Path,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<TaskCompletionPreviewReadModel>, ApplicationError> {
        let request = read_task_completion_file(file)?;
        let compatibility = task_compatibility_notice(
            root,
            LegacyCompatibilityOperation::TaskCompletionPreview,
            LegacyCompatibilityAccess::Write,
            &request.task_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let state = TaskService::new(&mut workspace.database, &workspace.blobs)
            .validate_completion(&request)?;
        Ok(ActionReceipt::new(
            "task.complete.preview",
            "validated",
            format!(
                "Validated completion for task {} without committing",
                request.task_id
            ),
            TaskCompletionPreviewReadModel { request, state },
        )
        .with_next_actions([NextAction {
            action: "commit the exact reviewed task completion request".to_owned(),
            description:
                "CanISend will revalidate the lease, job revision, input revisions/hashes, candidate schema, and source spans"
                    .to_owned(),
        }])
        .with_compatibility(compatibility))
    }

    pub fn commit_task_completion(
        root: &Path,
        request: TaskCompletionRequest,
    ) -> Result<ActionReceipt<canisend_contracts::TaskCommitData>, ApplicationError> {
        let compatibility = task_compatibility_notice(
            root,
            LegacyCompatibilityOperation::TaskCompletionCommit,
            LegacyCompatibilityAccess::Write,
            &request.task_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let committed =
            TaskService::new(&mut workspace.database, &workspace.blobs).complete(&request)?;
        let artifact = committed.artifact.clone();
        Ok(ActionReceipt::new(
            "task.complete",
            "committed",
            format!(
                "Committed task {} as artifact {}",
                committed.task_id, committed.artifact.id
            ),
            committed,
        )
        .with_artifacts([artifact])
        .with_compatibility(compatibility))
    }

    pub fn cancel_task(
        root: &Path,
        task_id: &str,
    ) -> Result<ActionReceipt<TaskStateData>, ApplicationError> {
        let task_id = parse_entity_id(task_id)?;
        let compatibility = task_compatibility_notice(
            root,
            LegacyCompatibilityOperation::TaskCancel,
            LegacyCompatibilityAccess::Write,
            &task_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let state = TaskService::new(&mut workspace.database, &workspace.blobs).cancel(&task_id)?;
        Ok(task_state_receipt("task.cancel", state).with_compatibility(compatibility))
    }

    pub fn prepare_task_again(
        root: &Path,
        task_id: &str,
    ) -> Result<ActionReceipt<TaskPrepareAgainReadModel>, ApplicationError> {
        let task_id = parse_entity_id(task_id)?;
        let compatibility = task_compatibility_notice(
            root,
            LegacyCompatibilityOperation::TaskPrepareAgain,
            LegacyCompatibilityAccess::Write,
            &task_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let previous = TaskService::new(&mut workspace.database, &workspace.blobs).get(&task_id)?;
        if !matches!(previous.status, TaskStatus::Cancelled | TaskStatus::Stale) {
            return Err(StoreError::TaskConflict(format!(
                "task {} must be cancelled or stale before preparing again",
                previous.descriptor.id
            ))
            .into());
        }
        let operation = TaskOperation::from_descriptor(&previous.descriptor.operation)?;
        let mode = TaskExecutionMode::from_execution_mode(previous.descriptor.execution_mode)?;
        WorkflowService::new(&mut workspace.database).status(&previous.descriptor.job_id)?;
        let descriptor =
            prepare_descriptor(&mut workspace, &previous.descriptor.job_id, operation, mode)?;
        let required_consents = descriptor.required_consents.clone();
        Ok(ActionReceipt::new(
            "task.prepare-again",
            "prepared",
            format!(
                "Prepared replacement task {} for {}",
                descriptor.id, previous.descriptor.id
            ),
            TaskPrepareAgainReadModel {
                previous,
                descriptor,
            },
        )
        .with_required_consents(required_consents)
        .with_next_actions([completion_next_action()])
        .with_compatibility(compatibility))
    }
}

fn prepare_descriptor(
    workspace: &mut Workspace,
    job_id: &EntityId,
    operation: TaskOperation,
    mode: TaskExecutionMode,
) -> Result<TaskDescriptor, ApplicationError> {
    let mode = mode.as_execution_mode();
    let descriptor = match operation {
        TaskOperation::JobParse => TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_job_parse(job_id, mode)?,
        TaskOperation::EvidenceNormalize => {
            TaskService::new(&mut workspace.database, &workspace.blobs)
                .prepare_evidence_normalization(job_id, mode)?
        }
        TaskOperation::EvidenceMatch => TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_evidence_match(job_id, mode)?,
        TaskOperation::CoverLetterDraft => TaskService::new(
            &mut workspace.database,
            &workspace.blobs,
        )
        .prepare_document_draft(job_id, DocumentKind::CoverLetter, mode)?,
        TaskOperation::ResearchStatementDraft => TaskService::new(
            &mut workspace.database,
            &workspace.blobs,
        )
        .prepare_document_draft(job_id, DocumentKind::ResearchStatement, mode)?,
        TaskOperation::TeachingStatementDraft => TaskService::new(
            &mut workspace.database,
            &workspace.blobs,
        )
        .prepare_document_draft(job_id, DocumentKind::TeachingStatement, mode)?,
        TaskOperation::CvDraft => TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_document_draft(job_id, DocumentKind::Cv, mode)?,
        TaskOperation::DocumentReview => {
            TaskService::new(&mut workspace.database, &workspace.blobs)
                .prepare_document_review(job_id, mode)?
        }
    };
    if descriptor.operation != operation.descriptor_operation() {
        return Err(StoreError::Invariant(format!(
            "task registry returned {} for {}",
            descriptor.operation,
            operation.as_str()
        ))
        .into());
    }
    Ok(descriptor)
}

fn prepared_receipt(
    operation: &'static str,
    descriptor: TaskDescriptor,
) -> ActionReceipt<TaskDescriptor> {
    let required_consents = descriptor.required_consents.clone();
    ActionReceipt::new(
        operation,
        "prepared",
        format!(
            "Prepared {} task {} with {} immutable input(s)",
            descriptor.operation,
            descriptor.id,
            descriptor.input_artifacts.len()
        ),
        descriptor,
    )
    .with_required_consents(required_consents)
    .with_next_actions([completion_next_action()])
}

fn task_state_receipt(
    operation: &'static str,
    state: TaskStateData,
) -> ActionReceipt<TaskStateData> {
    let required_consents = if state.status == TaskStatus::Prepared {
        state.descriptor.required_consents.clone()
    } else {
        Vec::new()
    };
    let artifacts = state.result.clone().into_iter().collect::<Vec<_>>();
    let next_actions = match state.status {
        TaskStatus::Stale | TaskStatus::Cancelled => vec![NextAction {
            action: format!("prepare task {} again", state.descriptor.id),
            description:
                "Prepare a new lease against the current job, profile, and artifact revisions"
                    .to_owned(),
        }],
        TaskStatus::Prepared => vec![completion_next_action()],
        TaskStatus::Committed => Vec::new(),
    };
    ActionReceipt::new(
        operation,
        task_status_text(state.status),
        format!(
            "Task {} is {}",
            state.descriptor.id,
            task_status_text(state.status)
        ),
        state,
    )
    .with_required_consents(required_consents)
    .with_artifacts(artifacts)
    .with_next_actions(next_actions)
}

const fn task_status_text(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Prepared => "prepared",
        TaskStatus::Committed => "committed",
        TaskStatus::Cancelled => "cancelled",
        TaskStatus::Stale => "stale",
    }
}

fn completion_next_action() -> NextAction {
    NextAction {
        action: "create a canisend.task-completion/v2 JSON file and preview it before committing"
            .to_owned(),
        description:
            "Repeat the task ID, lease ID, job revision, and every exact input revision/hash"
                .to_owned(),
    }
}

fn read_private_consent_required() -> ApplicationError {
    ApplicationError::ConsentRequired {
        message: "read-private-inputs consent must be explicitly confirmed".to_owned(),
        remediation: NextAction {
            action: "obtain user approval, then retry the scoped input export".to_owned(),
            description:
                "Only artifacts declared in the task's private read scope will be exported"
                    .to_owned(),
        },
    }
}

fn provider_send_consent_required() -> ApplicationError {
    ApplicationError::ConsentRequired {
        message: "send-to-configured-provider consent must be explicitly confirmed".to_owned(),
        remediation: NextAction {
            action: "obtain user approval, then retry the configured-provider export".to_owned(),
            description: "Only the exact artifact revisions declared by the task may be sent"
                .to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ArtifactKind, ConsentScope, EntityId, ErrorCode, ExpectedInputRevision, Revision,
        Sha256Digest, TaskCompletionRequest, TaskStatus,
    };
    use serde_json::json;

    use super::{TaskExecutionMode, TaskInputExportRequest, TaskOperation, TaskPrepareRequest};
    use crate::{Application, PrivateReadConsent, ProviderSendConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-task-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn expected_inputs(
        descriptor: &canisend_contracts::TaskDescriptor,
    ) -> Vec<ExpectedInputRevision> {
        descriptor
            .input_artifacts
            .iter()
            .map(|input| ExpectedInputRevision {
                artifact_id: input.id.clone(),
                revision: input.revision,
                sha256: input.sha256.clone(),
            })
            .collect()
    }

    fn parsed_job_candidate(descriptor: &canisend_contracts::TaskDescriptor) -> serde_json::Value {
        json!({
            "id": "019f2f55-7c00-7000-8000-000000000701",
            "job_id": descriptor.job_id,
            "title": "Lecturer in Economics",
            "institution": "University X",
            "summary": "Teach economics",
            "responsibilities": ["Teach economics"],
            "criteria": [{
                "id": "019f2f55-7c00-7000-8000-000000000702",
                "job_id": descriptor.job_id,
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
        })
    }

    fn completion_request(
        descriptor: &canisend_contracts::TaskDescriptor,
    ) -> TaskCompletionRequest {
        TaskCompletionRequest {
            task_id: descriptor.id.clone(),
            lease_id: descriptor.lease.id.clone(),
            expected_job_revision: descriptor.job_revision,
            expected_inputs: expected_inputs(descriptor),
            candidate: parsed_job_candidate(descriptor),
        }
    }

    #[test]
    fn task_registry_is_typed_bounded_and_stable() {
        assert_eq!(TaskOperation::ALL.len(), 8);
        for operation in TaskOperation::ALL {
            assert_eq!(
                TaskOperation::from_descriptor(operation.descriptor_operation())
                    .expect("compiled descriptor operation"),
                operation
            );
        }
        assert_eq!(TaskOperation::JobParse.as_str(), "job-parse");
        assert_eq!(
            serde_json::to_string(&TaskOperation::ResearchStatementDraft)
                .expect("serialize operation"),
            r#""research-statement-draft""#
        );
        assert_eq!(
            TaskExecutionMode::ConfiguredProvider.as_execution_mode(),
            canisend_contracts::ExecutionMode::ConfiguredProvider
        );

        let missing = temporary_root("missing");
        let error = TaskPrepareRequest::try_new(
            "not-a-uuid",
            TaskOperation::JobParse,
            TaskExecutionMode::HostAgent,
        )
        .expect_err("invalid task job ID");
        assert_eq!(error.classify().code, ErrorCode::InputInvalid);
        assert!(!missing.exists());
    }

    #[test]
    fn task_facade_preserves_reviewed_completion_and_independent_consents() {
        let root = temporary_root("workspace");
        let source = temporary_root("source").with_extension("txt");
        let host_export = temporary_root("host-export");
        let provider_export = temporary_root("provider-export");
        let completion_file = temporary_root("completion").with_extension("json");
        fs::write(&source, "Teach economics").expect("write source");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer in Economics", "University X")
            .expect("create job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import source");

        let prepared = Application::prepare_task(
            &root,
            TaskPrepareRequest::try_new(
                job.id.as_str(),
                TaskOperation::JobParse,
                TaskExecutionMode::HostAgent,
            )
            .expect("host prepare request"),
        )
        .expect("prepare host task");
        assert_eq!(prepared.operation, "task.prepare");
        assert_eq!(prepared.data.operation, "job.parse");
        assert_eq!(prepared.required_consents.len(), 1);
        assert_eq!(
            prepared.required_consents[0].scope,
            ConsentScope::ReadPrivateInputs
        );
        let shown =
            Application::task_state(&root, prepared.data.id.as_str()).expect("show prepared task");
        assert_eq!(shown.data.descriptor, prepared.data);
        assert_eq!(shown.data.status, TaskStatus::Prepared);
        let latest = Application::latest_task_for_job(&root, job.id.as_str())
            .expect("latest task for job")
            .data
            .expect("prepared task is discoverable");
        assert_eq!(latest.descriptor.id, shown.data.descriptor.id);
        assert_eq!(latest.status, TaskStatus::Prepared);

        let host_export_request =
            TaskInputExportRequest::try_new(prepared.data.id.as_str(), host_export.clone())
                .expect("host export request");
        let error = Application::export_task_inputs(&root, host_export_request.clone(), None, None)
            .expect_err("private consent required");
        let failure = error.classify();
        assert_eq!(failure.status, "consent-required");
        assert_eq!(failure.code, ErrorCode::ConsentRequired);
        assert!(failure.remediation.is_some());
        assert!(!host_export.exists());
        let exported = Application::export_task_inputs(
            &root,
            host_export_request,
            Some(PrivateReadConsent::granted_by_user()),
            None,
        )
        .expect("export exact host scope");
        assert_eq!(exported.data.files.len(), 1);
        assert_eq!(exported.artifacts, prepared.data.private_read_scope);

        let cancelled =
            Application::cancel_task(&root, prepared.data.id.as_str()).expect("cancel task");
        assert_eq!(cancelled.data.status, TaskStatus::Cancelled);
        assert_eq!(cancelled.next_actions.len(), 1);
        let replacement = Application::prepare_task_again(&root, prepared.data.id.as_str())
            .expect("prepare cancelled task again");
        assert_eq!(replacement.data.previous.status, TaskStatus::Cancelled);
        assert_ne!(replacement.data.descriptor.id, prepared.data.id);
        assert_eq!(
            replacement.data.descriptor.execution_mode,
            canisend_contracts::ExecutionMode::HostAgent
        );
        Application::cancel_task(&root, replacement.data.descriptor.id.as_str())
            .expect("cancel replacement");

        let provider = Application::prepare_task(
            &root,
            TaskPrepareRequest::try_new(
                job.id.as_str(),
                TaskOperation::JobParse,
                TaskExecutionMode::ConfiguredProvider,
            )
            .expect("provider prepare request"),
        )
        .expect("prepare provider task");
        assert_eq!(provider.required_consents.len(), 2);
        assert_eq!(
            provider.required_consents[0].scope,
            ConsentScope::ReadPrivateInputs
        );
        assert_eq!(
            provider.required_consents[1].scope,
            ConsentScope::SendToConfiguredProvider
        );
        let provider_export_request =
            TaskInputExportRequest::try_new(provider.data.id.as_str(), provider_export.clone())
                .expect("provider export request");
        let error = Application::export_task_inputs(
            &root,
            provider_export_request.clone(),
            Some(PrivateReadConsent::granted_by_user()),
            None,
        )
        .expect_err("provider send consent required independently");
        assert_eq!(error.classify().code, ErrorCode::ConsentRequired);
        assert!(error.to_string().contains("send-to-configured-provider"));
        assert!(!provider_export.exists());
        Application::export_task_inputs(
            &root,
            provider_export_request,
            Some(PrivateReadConsent::granted_by_user()),
            Some(ProviderSendConsent::granted_by_user()),
        )
        .expect("export provider scope after both consents");

        let request = completion_request(&provider.data);
        let mut invalid = request.clone();
        invalid.candidate = json!({"requirement": 3});
        fs::write(
            &completion_file,
            serde_json::to_vec_pretty(&invalid).expect("invalid completion JSON"),
        )
        .expect("write invalid completion");
        let error = Application::preview_task_completion_file(
            &root,
            &completion_file,
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("candidate validation failure");
        let failure = error.classify();
        assert_eq!(failure.code, ErrorCode::CandidateSchemaInvalid);
        assert!(failure.details.is_some());
        assert_eq!(
            Application::task_state(&root, provider.data.id.as_str())
                .expect("state after invalid preview")
                .data
                .status,
            TaskStatus::Prepared
        );

        let mut wrong_inputs = request.clone();
        wrong_inputs.expected_inputs[0].revision =
            Revision::try_new(wrong_inputs.expected_inputs[0].revision.get() + 1)
                .expect("different revision");
        wrong_inputs.expected_inputs[0].sha256 =
            Sha256Digest::try_new("0".repeat(64)).expect("different digest");
        fs::write(
            &completion_file,
            serde_json::to_vec_pretty(&wrong_inputs).expect("stale completion JSON"),
        )
        .expect("write stale completion");
        let error = Application::preview_task_completion_file(
            &root,
            &completion_file,
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("exact inputs required");
        assert_eq!(error.classify().code, ErrorCode::TaskStale);
        assert_eq!(
            Application::task_state(&root, provider.data.id.as_str())
                .expect("state after stale preview")
                .data
                .status,
            TaskStatus::Prepared
        );

        let mut wrong_lease = request.clone();
        wrong_lease.lease_id =
            EntityId::try_new("019f2f55-7c00-7000-8000-000000000799").expect("different lease");
        fs::write(
            &completion_file,
            serde_json::to_vec_pretty(&wrong_lease).expect("wrong lease JSON"),
        )
        .expect("write wrong lease completion");
        let error = Application::preview_task_completion_file(
            &root,
            &completion_file,
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("exact lease required");
        assert_eq!(error.classify().code, ErrorCode::TaskConflict);
        assert_eq!(
            Application::task_state(&root, provider.data.id.as_str())
                .expect("state after lease validation")
                .data
                .status,
            TaskStatus::Prepared
        );

        fs::write(
            &completion_file,
            serde_json::to_vec_pretty(&request).expect("completion JSON"),
        )
        .expect("write valid completion");
        let preview = Application::preview_task_completion_file(
            &root,
            &completion_file,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("preview valid completion");
        assert_eq!(preview.status, "validated");
        assert_eq!(preview.data.state.status, TaskStatus::Prepared);
        assert_eq!(preview.data.request, request);
        fs::write(&completion_file, b"{}").expect("replace completion after preview");
        let committed = Application::commit_task_completion(&root, preview.data.request.clone())
            .expect("commit exact preview");
        assert_eq!(committed.data.status, TaskStatus::Committed);
        assert_eq!(committed.data.artifact.kind, ArtifactKind::ParsedJob);
        assert!(!committed.data.idempotent);
        assert_eq!(committed.artifacts, vec![committed.data.artifact.clone()]);
        let replay = Application::commit_task_completion(&root, preview.data.request)
            .expect("idempotent replay");
        assert!(replay.data.idempotent);
        assert_eq!(replay.data.artifact, committed.data.artifact);
        assert_eq!(
            Application::latest_task_for_job(&root, job.id.as_str())
                .expect("latest committed task")
                .data
                .expect("task remains discoverable")
                .status,
            TaskStatus::Committed
        );

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
        fs::remove_dir_all(host_export).expect("remove host export");
        fs::remove_dir_all(provider_export).expect("remove provider export");
        fs::remove_file(completion_file).expect("remove completion");
    }

    #[test]
    fn stale_completion_becomes_prepare_again_recovery() {
        let root = temporary_root("stale-workspace");
        let source = temporary_root("stale-source").with_extension("txt");
        let completion_file = temporary_root("stale-completion").with_extension("json");
        fs::write(&source, "Teach economics").expect("write source");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer in Economics", "University X")
            .expect("create job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import source");
        let descriptor = Application::prepare_task(
            &root,
            TaskPrepareRequest::try_new(
                job.id.as_str(),
                TaskOperation::JobParse,
                TaskExecutionMode::HostAgent,
            )
            .expect("prepare request"),
        )
        .expect("prepare task")
        .data;
        let request = completion_request(&descriptor);
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("change job revision");
        fs::write(
            &completion_file,
            serde_json::to_vec_pretty(&request).expect("stale completion JSON"),
        )
        .expect("write completion");
        let preview_error = Application::preview_task_completion_file(
            &root,
            &completion_file,
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("preview detects stale inputs");
        assert_eq!(preview_error.classify().code, ErrorCode::TaskStale);
        assert_eq!(
            Application::task_state(&root, descriptor.id.as_str())
                .expect("read still-prepared state")
                .data
                .status,
            TaskStatus::Prepared
        );
        let commit_error = Application::commit_task_completion(&root, request)
            .expect_err("commit records stale state");
        assert_eq!(commit_error.classify().code, ErrorCode::TaskStale);
        let stale =
            Application::task_state(&root, descriptor.id.as_str()).expect("read stale task");
        assert_eq!(stale.data.status, TaskStatus::Stale);
        assert_eq!(stale.next_actions.len(), 1);
        let recovered = Application::prepare_task_again(&root, descriptor.id.as_str())
            .expect("prepare against current revision");
        assert_eq!(recovered.data.previous.status, TaskStatus::Stale);
        assert_ne!(recovered.data.descriptor.id, descriptor.id);
        assert!(recovered.data.descriptor.job_revision > descriptor.job_revision);

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
        fs::remove_file(completion_file).expect("remove completion");
    }
}
