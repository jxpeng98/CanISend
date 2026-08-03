use std::path::Path;

use canisend_contracts::{
    ActorKind, ArtifactReference, EntityId, ExecutionMode, StageDescriptor, WorkflowStage,
    WorkflowStatusData,
};
use canisend_core::StageGraph;
use canisend_store::{ArtifactService, WorkflowService};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError,
    application::{open_workspace, parse_entity_id},
    compatibility::{
        LegacyCompatibilityAccess, LegacyCompatibilityOperation, job_compatibility_notice,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowControlReadModel {
    pub status: WorkflowStatusData,
    pub stage_descriptors: Vec<StageDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBeginRequest {
    pub job_id: EntityId,
    pub stage: WorkflowStage,
    pub mode: ExecutionMode,
}

impl WorkflowBeginRequest {
    pub fn try_new(
        job_id: &str,
        stage: WorkflowStage,
        mode: ExecutionMode,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            job_id: parse_entity_id(job_id)?,
            stage,
            mode,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowCompleteRequest {
    pub job_id: EntityId,
    pub stage: WorkflowStage,
    pub artifact_id: EntityId,
}

impl WorkflowCompleteRequest {
    pub fn try_new(
        job_id: &str,
        stage: WorkflowStage,
        artifact_id: &str,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            job_id: parse_entity_id(job_id)?,
            stage,
            artifact_id: parse_entity_id(artifact_id)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowRerunPreview {
    pub job_id: EntityId,
    pub target: WorkflowStage,
    pub affected_stages: Vec<WorkflowStage>,
    pub affected_outputs: Vec<ArtifactReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowRerunRequest {
    pub job_id: EntityId,
    pub stage: WorkflowStage,
}

impl WorkflowRerunRequest {
    pub fn try_new(job_id: &str, stage: WorkflowStage) -> Result<Self, ApplicationError> {
        Ok(Self {
            job_id: parse_entity_id(job_id)?,
            stage,
        })
    }
}

impl Application {
    pub fn start_workflow(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<WorkflowStatusData>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).start(&job_id)?;
        let next_actions = status.next_actions.clone();
        Ok(ActionReceipt::new(
            "workflow.start",
            "started",
            "Workflow is ready for its next action",
            status,
        )
        .with_next_actions(next_actions))
    }

    pub fn workflow_status(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<WorkflowStatusData>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let compatibility = job_compatibility_notice(
            root,
            LegacyCompatibilityOperation::WorkflowStatus,
            LegacyCompatibilityAccess::Read,
            &job_id,
        )?;
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).status(&job_id)?;
        let next_actions = status.next_actions.clone();
        Ok(ActionReceipt::new(
            "workflow.status",
            "available",
            format!("{} blocker(s)", status.blockers.len()),
            status,
        )
        .with_next_actions(next_actions)
        .with_compatibility(compatibility))
    }

    pub fn workflow_controls(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<WorkflowControlReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).status(&job_id)?;
        workflow_control_receipt("workflow.controls", "available", status)
    }

    pub fn begin_workflow_stage(
        root: &Path,
        request: WorkflowBeginRequest,
    ) -> Result<ActionReceipt<WorkflowControlReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).begin_stage(
            &request.job_id,
            request.stage,
            request.mode,
            ActorKind::User,
        )?;
        workflow_control_receipt("workflow.begin", "begun", status)
    }

    pub fn complete_workflow_stage(
        root: &Path,
        request: WorkflowCompleteRequest,
    ) -> Result<ActionReceipt<WorkflowControlReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let artifact = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        )
        .reference(&request.artifact_id)?;
        let status = WorkflowService::new(&mut workspace.database).complete_stage(
            &request.job_id,
            request.stage,
            &artifact,
            ActorKind::User,
        )?;
        Ok(
            workflow_control_receipt("workflow.complete", "complete", status)?
                .with_artifacts(vec![artifact]),
        )
    }

    pub fn preview_workflow_rerun(
        root: &Path,
        job_id: &str,
        stage: WorkflowStage,
    ) -> Result<ActionReceipt<WorkflowRerunPreview>, ApplicationError> {
        if stage == WorkflowStage::Intake {
            return Err(ApplicationError::InvalidInput(
                "intake changes must use job import rather than workflow rerun".to_owned(),
            ));
        }
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).status(&job_id)?;
        let graph = StageGraph::built_in();
        let affected_stages = std::iter::once(stage)
            .chain(graph.descendants(stage))
            .collect::<Vec<_>>();
        let affected_outputs = status
            .stages
            .iter()
            .filter(|state| affected_stages.contains(&state.stage))
            .filter_map(|state| state.output.clone())
            .collect::<Vec<_>>();
        Ok(ActionReceipt::new(
            "workflow.rerun.preview",
            "available",
            format!(
                "{} stage(s) and {} current output(s) may be invalidated",
                affected_stages.len(),
                affected_outputs.len()
            ),
            WorkflowRerunPreview {
                job_id,
                target: stage,
                affected_stages,
                affected_outputs,
            },
        ))
    }

    pub fn rerun_workflow_stage(
        root: &Path,
        request: WorkflowRerunRequest,
    ) -> Result<ActionReceipt<WorkflowControlReadModel>, ApplicationError> {
        if request.stage == WorkflowStage::Intake {
            return Err(ApplicationError::InvalidInput(
                "intake changes must use job import rather than workflow rerun".to_owned(),
            ));
        }
        let mut workspace = open_workspace(root)?;
        let status = WorkflowService::new(&mut workspace.database).rerun(
            &request.job_id,
            request.stage,
            ActorKind::User,
        )?;
        workflow_control_receipt("workflow.rerun", "ready", status)
    }
}

fn workflow_control_receipt(
    operation: &'static str,
    receipt_status: &'static str,
    status: WorkflowStatusData,
) -> Result<ActionReceipt<WorkflowControlReadModel>, ApplicationError> {
    let next_actions = status.next_actions.clone();
    let blockers = status.blockers.len();
    Ok(ActionReceipt::new(
        operation,
        receipt_status,
        format!("{blockers} blocker(s)"),
        WorkflowControlReadModel {
            status,
            stage_descriptors: StageGraph::built_in().descriptors(),
        },
    )
    .with_next_actions(next_actions))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ActorKind, ArtifactKind, ExecutionMode, StageExecutionStatus, WorkflowStage,
    };
    use canisend_store::{ArtifactService, Workspace};

    use crate::{
        Application, PrivateReadConsent, WorkflowBeginRequest, WorkflowCompleteRequest,
        WorkflowRerunRequest,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-workflow-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn workflow_fixture(label: &str) -> (PathBuf, String, PathBuf) {
        let root = temporary_root(label);
        let source = temporary_root("advert").with_extension("txt");
        fs::write(&source, "Lecturer job fixture").expect("write job source");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Lecturer", "University")
            .expect("create job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import source");
        Application::start_workflow(&root, job.id.as_str()).expect("start workflow");
        (root, job.id.to_string(), source)
    }

    fn commit_artifact(root: &Path, kind: ArtifactKind) -> String {
        let mut workspace = Workspace::open(Some(root)).expect("open workspace");
        ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        )
        .commit(
            None,
            kind,
            b"bounded workflow output",
            &[],
            ActorKind::User,
            "workflow application test",
        )
        .expect("commit artifact")
        .artifact_id
        .to_string()
    }

    fn cleanup(root: PathBuf, source: PathBuf) {
        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
    }

    #[test]
    fn controls_expose_authoritative_descriptors_and_validate_ids() {
        let (root, job_id, source) = workflow_fixture("controls");
        let controls = Application::workflow_controls(&root, &job_id).expect("controls");
        assert_eq!(
            controls.data.stage_descriptors.len(),
            WorkflowStage::ALL.len()
        );
        assert_eq!(
            controls.data.stage_descriptors[1].execution_modes,
            vec![ExecutionMode::HostAgent, ExecutionMode::ConfiguredProvider]
        );
        assert_eq!(
            controls.data.status.stages[1].status,
            StageExecutionStatus::Ready
        );
        assert!(
            WorkflowBeginRequest::try_new(
                "not-an-id",
                WorkflowStage::Parse,
                ExecutionMode::HostAgent
            )
            .is_err()
        );
        assert!(
            WorkflowCompleteRequest::try_new(&job_id, WorkflowStage::Parse, "not-an-id").is_err()
        );
        assert!(Application::workflow_controls(&root, "not-an-id").is_err());
        cleanup(root, source);
    }

    #[test]
    fn begin_and_complete_enforce_mode_state_and_artifact_kind() {
        let (root, job_id, source) = workflow_fixture("begin-complete");
        let unsupported = WorkflowBeginRequest::try_new(
            &job_id,
            WorkflowStage::Parse,
            ExecutionMode::Deterministic,
        )
        .expect("typed request");
        assert!(Application::begin_workflow_stage(&root, unsupported).is_err());
        assert_eq!(
            Application::workflow_controls(&root, &job_id)
                .expect("unchanged controls")
                .data
                .status
                .stages[1]
                .status,
            StageExecutionStatus::Ready
        );

        let begin =
            WorkflowBeginRequest::try_new(&job_id, WorkflowStage::Parse, ExecutionMode::HostAgent)
                .expect("begin request");
        let running = Application::begin_workflow_stage(&root, begin).expect("begin parse");
        assert_eq!(
            running.data.status.stages[1].status,
            StageExecutionStatus::Running
        );
        let duplicate =
            WorkflowBeginRequest::try_new(&job_id, WorkflowStage::Parse, ExecutionMode::HostAgent)
                .expect("duplicate request");
        assert!(Application::begin_workflow_stage(&root, duplicate).is_err());

        let missing_complete = WorkflowCompleteRequest::try_new(
            &job_id,
            WorkflowStage::Parse,
            "019f2f55-7c00-7000-8000-000000000002",
        )
        .expect("missing complete request");
        assert!(Application::complete_workflow_stage(&root, missing_complete).is_err());

        let wrong_artifact = commit_artifact(&root, ArtifactKind::EvidenceCatalog);
        let wrong_complete =
            WorkflowCompleteRequest::try_new(&job_id, WorkflowStage::Parse, &wrong_artifact)
                .expect("wrong complete request");
        assert!(Application::complete_workflow_stage(&root, wrong_complete).is_err());
        assert_eq!(
            Application::workflow_controls(&root, &job_id)
                .expect("still running")
                .data
                .status
                .stages[1]
                .status,
            StageExecutionStatus::Running
        );

        let artifact = commit_artifact(&root, ArtifactKind::ParsedJob);
        let complete = WorkflowCompleteRequest::try_new(&job_id, WorkflowStage::Parse, &artifact)
            .expect("complete request");
        let completed =
            Application::complete_workflow_stage(&root, complete).expect("complete parse");
        assert_eq!(completed.operation, "workflow.complete");
        assert_eq!(completed.artifacts.len(), 1);
        assert_eq!(
            completed.data.status.stages[1].status,
            StageExecutionStatus::Complete
        );
        assert_eq!(
            completed.data.status.stages[2].status,
            StageExecutionStatus::Ready
        );

        let decide = WorkflowBeginRequest::try_new(
            &job_id,
            WorkflowStage::Criteria,
            ExecutionMode::UserDecision,
        )
        .expect("decision request");
        let awaiting =
            Application::begin_workflow_stage(&root, decide).expect("begin user decision");
        assert_eq!(
            awaiting.data.status.stages[2].status,
            StageExecutionStatus::AwaitingUser
        );
        cleanup(root, source);
    }

    #[test]
    fn rerun_preview_and_mutation_use_graph_descendants() {
        let (root, job_id, source) = workflow_fixture("rerun");
        let begin =
            WorkflowBeginRequest::try_new(&job_id, WorkflowStage::Parse, ExecutionMode::HostAgent)
                .expect("begin request");
        Application::begin_workflow_stage(&root, begin).expect("begin parse");
        let artifact = commit_artifact(&root, ArtifactKind::ParsedJob);
        let complete = WorkflowCompleteRequest::try_new(&job_id, WorkflowStage::Parse, &artifact)
            .expect("complete request");
        Application::complete_workflow_stage(&root, complete).expect("complete parse");

        let preview = Application::preview_workflow_rerun(&root, &job_id, WorkflowStage::Parse)
            .expect("rerun preview");
        assert_eq!(preview.data.target, WorkflowStage::Parse);
        assert_eq!(
            preview.data.affected_stages.first(),
            Some(&WorkflowStage::Parse)
        );
        assert!(
            preview
                .data
                .affected_stages
                .contains(&WorkflowStage::Render)
        );
        assert_eq!(preview.data.affected_outputs.len(), 1);
        assert!(
            Application::preview_workflow_rerun(&root, &job_id, WorkflowStage::Intake).is_err()
        );

        let request =
            WorkflowRerunRequest::try_new(&job_id, WorkflowStage::Parse).expect("rerun request");
        let rerun = Application::rerun_workflow_stage(&root, request).expect("rerun parse");
        assert_eq!(
            rerun.data.status.stages[1].status,
            StageExecutionStatus::Ready
        );
        assert_eq!(
            rerun.data.status.stages[2].status,
            StageExecutionStatus::Stale
        );
        assert!(WorkflowRerunRequest::try_new("not-an-id", WorkflowStage::Parse).is_err());
        cleanup(root, source);
    }
}
