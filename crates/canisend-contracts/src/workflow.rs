use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ArtifactKind, ArtifactReference, EntityId, ExecutionMode, NextAction, UtcTimestamp};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowStage {
    Intake,
    Parse,
    Criteria,
    Evidence,
    Match,
    Plan,
    Draft,
    Review,
    Package,
    Render,
}

impl WorkflowStage {
    pub const ALL: [Self; 10] = [
        Self::Intake,
        Self::Parse,
        Self::Criteria,
        Self::Evidence,
        Self::Match,
        Self::Plan,
        Self::Draft,
        Self::Review,
        Self::Package,
        Self::Render,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intake => "intake",
            Self::Parse => "parse",
            Self::Criteria => "criteria",
            Self::Evidence => "evidence",
            Self::Match => "match",
            Self::Plan => "plan",
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Package => "package",
            Self::Render => "render",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StageDescriptor {
    pub stage: WorkflowStage,
    pub depends_on: Vec<WorkflowStage>,
    pub output_kind: ArtifactKind,
    pub execution_modes: Vec<ExecutionMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum StageExecutionStatus {
    Blocked,
    Ready,
    Running,
    AwaitingUser,
    Complete,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowRunStatus {
    Active,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBlocker {
    pub code: String,
    pub stage: WorkflowStage,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStageState {
    pub stage: WorkflowStage,
    pub status: StageExecutionStatus,
    pub execution_mode: Option<ExecutionMode>,
    pub output: Option<ArtifactReference>,
    pub updated_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStatusData {
    pub run_id: EntityId,
    pub job_id: EntityId,
    pub status: WorkflowRunStatus,
    pub stages: Vec<WorkflowStageState>,
    pub blockers: Vec<WorkflowBlocker>,
    pub next_actions: Vec<NextAction>,
}
