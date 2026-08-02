use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    ExecutionMode, StageId, WORKFLOW_PACK_MAX_STAGES, WorkflowPackId, WorkflowPackItemId,
    WorkflowPackLocalizedText, WorkflowPackStageDefinition, WorkflowPackStageOutput,
    WorkflowPackWorkflowDefinition,
};
use thiserror::Error;

use crate::VerifiedWorkflowPackBundle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPackStageDescriptor {
    stage: StageId,
    local_id: WorkflowPackItemId,
    labels: WorkflowPackLocalizedText,
    depends_on: Vec<StageId>,
    output: WorkflowPackStageOutput,
    execution_modes: Vec<ExecutionMode>,
}

impl WorkflowPackStageDescriptor {
    #[must_use]
    pub const fn stage(&self) -> &StageId {
        &self.stage
    }

    #[must_use]
    pub const fn local_id(&self) -> &WorkflowPackItemId {
        &self.local_id
    }

    #[must_use]
    pub const fn labels(&self) -> &WorkflowPackLocalizedText {
        &self.labels
    }

    #[must_use]
    pub fn depends_on(&self) -> &[StageId] {
        &self.depends_on
    }

    #[must_use]
    pub const fn output(&self) -> WorkflowPackStageOutput {
        self.output
    }

    #[must_use]
    pub fn execution_modes(&self) -> &[ExecutionMode] {
        &self.execution_modes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPackStageGraph {
    pack_id: WorkflowPackId,
    descriptors: BTreeMap<StageId, WorkflowPackStageDescriptor>,
    topological_order: Vec<StageId>,
    terminal_stage: StageId,
}

impl WorkflowPackStageGraph {
    pub fn try_new(
        pack_id: WorkflowPackId,
        workflow: &WorkflowPackWorkflowDefinition,
    ) -> Result<Self, WorkflowPackStageGraphError> {
        validate_stage_count(workflow.stages.len())?;
        let mut descriptors = BTreeMap::new();
        for definition in &workflow.stages {
            let descriptor = compile_descriptor(&pack_id, definition)?;
            let stage = descriptor.stage.clone();
            if descriptors.insert(stage.clone(), descriptor).is_some() {
                return Err(WorkflowPackStageGraphError::DuplicateStage { stage });
            }
        }
        validate_dependencies(&descriptors)?;
        let terminal_stage = StageId::from_parts(&pack_id, &workflow.terminal_stage);
        if !descriptors.contains_key(&terminal_stage) {
            return Err(WorkflowPackStageGraphError::TerminalStageMissing {
                stage: terminal_stage,
            });
        }
        let topological_order = topological_order(&descriptors)?;
        validate_terminal_reachability(&descriptors, &terminal_stage)?;
        Ok(Self {
            pack_id,
            descriptors,
            topological_order,
            terminal_stage,
        })
    }

    pub fn from_verified_bundle(
        bundle: &VerifiedWorkflowPackBundle,
    ) -> Result<Self, WorkflowPackStageGraphError> {
        Self::try_new(bundle.manifest().id.clone(), &bundle.manifest().workflow)
    }

    #[must_use]
    pub const fn pack_id(&self) -> &WorkflowPackId {
        &self.pack_id
    }

    #[must_use]
    pub const fn terminal_stage(&self) -> &StageId {
        &self.terminal_stage
    }

    #[must_use]
    pub fn topological_order(&self) -> &[StageId] {
        &self.topological_order
    }

    #[must_use]
    pub fn stage_id(&self, local_id: &WorkflowPackItemId) -> StageId {
        StageId::from_parts(&self.pack_id, local_id)
    }

    #[must_use]
    pub fn descriptor(&self, stage: &StageId) -> Option<&WorkflowPackStageDescriptor> {
        self.descriptors.get(stage)
    }

    #[must_use]
    pub fn descriptor_for_local_id(
        &self,
        local_id: &WorkflowPackItemId,
    ) -> Option<&WorkflowPackStageDescriptor> {
        self.descriptor(&self.stage_id(local_id))
    }

    #[must_use]
    pub fn descriptors(&self) -> Vec<&WorkflowPackStageDescriptor> {
        self.topological_order
            .iter()
            .map(|stage| {
                self.descriptors
                    .get(stage)
                    .expect("topological order contains only compiled stages")
            })
            .collect()
    }

    #[must_use]
    pub fn contains(&self, stage: &StageId) -> bool {
        self.descriptors.contains_key(stage)
    }

    #[must_use]
    pub fn supports_mode(&self, stage: &StageId, mode: ExecutionMode) -> bool {
        self.descriptor(stage)
            .is_some_and(|descriptor| descriptor.execution_modes.contains(&mode))
    }

    #[must_use]
    pub fn descendants(&self, stage: &StageId) -> Option<Vec<StageId>> {
        if !self.contains(stage) {
            return None;
        }
        let mut descendants = BTreeSet::new();
        let mut frontier = vec![stage.clone()];
        while let Some(parent) = frontier.pop() {
            for descriptor in self.descriptors.values() {
                if descriptor.depends_on.contains(&parent)
                    && descendants.insert(descriptor.stage.clone())
                {
                    frontier.push(descriptor.stage.clone());
                }
            }
        }
        Some(
            self.topological_order
                .iter()
                .filter(|candidate| descendants.contains(*candidate))
                .cloned()
                .collect(),
        )
    }

    #[must_use]
    pub fn ancestors(&self, stage: &StageId) -> Option<Vec<StageId>> {
        if !self.contains(stage) {
            return None;
        }
        let mut ancestors = BTreeSet::new();
        collect_ancestors(&self.descriptors, stage, &mut ancestors);
        ancestors.remove(stage);
        Some(
            self.topological_order
                .iter()
                .filter(|candidate| ancestors.contains(*candidate))
                .cloned()
                .collect(),
        )
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

fn validate_stage_count(actual: usize) -> Result<(), WorkflowPackStageGraphError> {
    if actual == 0 || actual > WORKFLOW_PACK_MAX_STAGES {
        return Err(WorkflowPackStageGraphError::StageCountInvalid {
            minimum: 1,
            maximum: WORKFLOW_PACK_MAX_STAGES,
            actual,
        });
    }
    Ok(())
}

fn compile_descriptor(
    pack_id: &WorkflowPackId,
    definition: &WorkflowPackStageDefinition,
) -> Result<WorkflowPackStageDescriptor, WorkflowPackStageGraphError> {
    let stage = StageId::from_parts(pack_id, &definition.id);
    if definition.execution_modes.is_empty() || definition.execution_modes.len() > 5 {
        return Err(WorkflowPackStageGraphError::ExecutionModeCountInvalid {
            stage,
            minimum: 1,
            maximum: 5,
            actual: definition.execution_modes.len(),
        });
    }
    let mut unique_modes = Vec::new();
    for mode in &definition.execution_modes {
        if unique_modes.contains(mode) {
            return Err(WorkflowPackStageGraphError::DuplicateExecutionMode { stage, mode: *mode });
        }
        unique_modes.push(*mode);
    }
    let mut depends_on = Vec::with_capacity(definition.depends_on.len());
    for local_dependency in &definition.depends_on {
        let dependency = StageId::from_parts(pack_id, local_dependency);
        if dependency == stage {
            return Err(WorkflowPackStageGraphError::SelfDependency { stage });
        }
        if depends_on.contains(&dependency) {
            return Err(WorkflowPackStageGraphError::DuplicateDependency { stage, dependency });
        }
        depends_on.push(dependency);
    }
    Ok(WorkflowPackStageDescriptor {
        stage,
        local_id: definition.id.clone(),
        labels: definition.labels.clone(),
        depends_on,
        output: definition.output,
        execution_modes: definition.execution_modes.clone(),
    })
}

fn validate_dependencies(
    descriptors: &BTreeMap<StageId, WorkflowPackStageDescriptor>,
) -> Result<(), WorkflowPackStageGraphError> {
    for descriptor in descriptors.values() {
        for dependency in &descriptor.depends_on {
            if !descriptors.contains_key(dependency) {
                return Err(WorkflowPackStageGraphError::MissingDependency {
                    stage: descriptor.stage.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    Ok(())
}

fn topological_order(
    descriptors: &BTreeMap<StageId, WorkflowPackStageDescriptor>,
) -> Result<Vec<StageId>, WorkflowPackStageGraphError> {
    let mut incoming = descriptors
        .iter()
        .map(|(stage, descriptor)| (stage.clone(), descriptor.depends_on.len()))
        .collect::<BTreeMap<_, _>>();
    let mut ready = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(stage, _)| stage.clone())
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(descriptors.len());
    while let Some(stage) = ready.pop_first() {
        ordered.push(stage.clone());
        for descriptor in descriptors.values() {
            if descriptor.depends_on.contains(&stage) {
                let count = incoming
                    .get_mut(&descriptor.stage)
                    .expect("compiled stage has an incoming count");
                *count -= 1;
                if *count == 0 {
                    ready.insert(descriptor.stage.clone());
                }
            }
        }
    }
    if ordered.len() == descriptors.len() {
        Ok(ordered)
    } else {
        Err(WorkflowPackStageGraphError::Cycle)
    }
}

fn validate_terminal_reachability(
    descriptors: &BTreeMap<StageId, WorkflowPackStageDescriptor>,
    terminal_stage: &StageId,
) -> Result<(), WorkflowPackStageGraphError> {
    let mut ancestors = BTreeSet::new();
    collect_ancestors(descriptors, terminal_stage, &mut ancestors);
    if ancestors.len() == descriptors.len() {
        return Ok(());
    }
    Err(WorkflowPackStageGraphError::NotTerminalReachable {
        terminal_stage: terminal_stage.clone(),
        stages: descriptors
            .keys()
            .filter(|stage| !ancestors.contains(*stage))
            .cloned()
            .collect(),
    })
}

fn collect_ancestors(
    descriptors: &BTreeMap<StageId, WorkflowPackStageDescriptor>,
    stage: &StageId,
    ancestors: &mut BTreeSet<StageId>,
) {
    if !ancestors.insert(stage.clone()) {
        return;
    }
    if let Some(descriptor) = descriptors.get(stage) {
        for dependency in &descriptor.depends_on {
            collect_ancestors(descriptors, dependency, ancestors);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowPackStageGraphError {
    #[error("workflow-pack stage count must be between {minimum} and {maximum}; found {actual}")]
    StageCountInvalid {
        minimum: usize,
        maximum: usize,
        actual: usize,
    },
    #[error("workflow-pack stage is declared more than once: {stage}")]
    DuplicateStage { stage: StageId },
    #[error(
        "workflow-pack stage {stage} execution-mode count must be between {minimum} and {maximum}; found {actual}"
    )]
    ExecutionModeCountInvalid {
        stage: StageId,
        minimum: usize,
        maximum: usize,
        actual: usize,
    },
    #[error("workflow-pack stage {stage} repeats execution mode {mode:?}")]
    DuplicateExecutionMode { stage: StageId, mode: ExecutionMode },
    #[error("workflow-pack stage cannot depend on itself: {stage}")]
    SelfDependency { stage: StageId },
    #[error("workflow-pack stage repeats dependency: {stage} -> {dependency}")]
    DuplicateDependency { stage: StageId, dependency: StageId },
    #[error("workflow-pack stage depends on an undeclared stage: {stage} -> {dependency}")]
    MissingDependency { stage: StageId, dependency: StageId },
    #[error("workflow-pack terminal stage is not declared: {stage}")]
    TerminalStageMissing { stage: StageId },
    #[error("workflow-pack stage graph contains a cycle")]
    Cycle,
    #[error(
        "workflow-pack stages do not contribute to terminal stage {terminal_stage}: {stages:?}"
    )]
    NotTerminalReachable {
        terminal_stage: StageId,
        stages: Vec<StageId>,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use canisend_contracts::{
        ExecutionMode, StageId, WORKFLOW_PACK_MAX_STAGES, WorkflowPackId, WorkflowPackItemId,
        WorkflowPackLocaleId, WorkflowPackLocalizedText, WorkflowPackStageDefinition,
        WorkflowPackStageOutput, WorkflowPackWorkflowDefinition,
    };

    use super::{WorkflowPackStageGraph, WorkflowPackStageGraphError};

    fn pack(value: &str) -> WorkflowPackId {
        WorkflowPackId::try_new(value).expect("pack ID")
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("item ID")
    }

    fn labels(value: &str) -> WorkflowPackLocalizedText {
        WorkflowPackLocalizedText(BTreeMap::from([(
            WorkflowPackLocaleId::try_new("en").expect("locale ID"),
            value.to_owned(),
        )]))
    }

    fn stage(
        id: &str,
        dependencies: &[&str],
        output: WorkflowPackStageOutput,
        modes: &[ExecutionMode],
    ) -> WorkflowPackStageDefinition {
        WorkflowPackStageDefinition {
            id: item(id),
            labels: labels(id),
            depends_on: dependencies.iter().map(|value| item(value)).collect(),
            output,
            execution_modes: modes.to_vec(),
        }
    }

    fn valid_workflow() -> WorkflowPackWorkflowDefinition {
        WorkflowPackWorkflowDefinition {
            stages: vec![
                stage(
                    "finish",
                    &["review"],
                    WorkflowPackStageOutput::Render,
                    &[ExecutionMode::Deterministic],
                ),
                stage(
                    "intake",
                    &[],
                    WorkflowPackStageOutput::Sources,
                    &[ExecutionMode::ManualImport],
                ),
                stage(
                    "review",
                    &["match"],
                    WorkflowPackStageOutput::Review,
                    &[ExecutionMode::UserDecision],
                ),
                stage(
                    "evidence",
                    &[],
                    WorkflowPackStageOutput::Evidence,
                    &[ExecutionMode::HostAgent],
                ),
                stage(
                    "match",
                    &["intake", "evidence"],
                    WorkflowPackStageOutput::Matches,
                    &[ExecutionMode::ConfiguredProvider, ExecutionMode::HostAgent],
                ),
            ],
            terminal_stage: item("finish"),
        }
    }

    fn id(pack_id: &WorkflowPackId, local_id: &str) -> StageId {
        StageId::from_parts(pack_id, &item(local_id))
    }

    #[test]
    fn compiles_a_deterministic_pack_qualified_graph() {
        let pack_id = pack("org.canisend.graph-test");
        let graph = WorkflowPackStageGraph::try_new(pack_id.clone(), &valid_workflow())
            .expect("valid dynamic graph");
        let mut reordered_workflow = valid_workflow();
        reordered_workflow.stages.reverse();
        let reordered = WorkflowPackStageGraph::try_new(pack_id.clone(), &reordered_workflow)
            .expect("reordered dynamic graph");
        assert_eq!(
            graph, reordered,
            "declaration order must not change the graph"
        );
        assert_eq!(graph.pack_id(), &pack_id);
        assert_eq!(graph.len(), 5);
        assert_eq!(graph.terminal_stage(), &id(&pack_id, "finish"));
        assert_eq!(
            graph.topological_order(),
            [
                id(&pack_id, "evidence"),
                id(&pack_id, "intake"),
                id(&pack_id, "match"),
                id(&pack_id, "review"),
                id(&pack_id, "finish"),
            ]
        );
        assert_eq!(
            graph.descendants(&id(&pack_id, "evidence")),
            Some(vec![
                id(&pack_id, "match"),
                id(&pack_id, "review"),
                id(&pack_id, "finish"),
            ])
        );
        assert_eq!(
            graph.ancestors(&id(&pack_id, "review")),
            Some(vec![
                id(&pack_id, "evidence"),
                id(&pack_id, "intake"),
                id(&pack_id, "match"),
            ])
        );
        assert!(graph.supports_mode(&id(&pack_id, "match"), ExecutionMode::ConfiguredProvider));
        assert_eq!(
            graph
                .descriptor_for_local_id(&item("match"))
                .expect("match descriptor")
                .output(),
            WorkflowPackStageOutput::Matches
        );
    }

    #[test]
    fn stage_identity_is_isolated_between_packs() {
        let first_pack = pack("org.canisend.first-pack");
        let second_pack = pack("org.canisend.second-pack");
        let graph = WorkflowPackStageGraph::try_new(first_pack.clone(), &valid_workflow())
            .expect("first graph");
        let first_stage = id(&first_pack, "review");
        let second_stage = id(&second_pack, "review");
        assert!(graph.contains(&first_stage));
        assert!(!graph.contains(&second_stage));
        assert_eq!(graph.descriptor(&second_stage), None);
        assert_eq!(graph.descendants(&second_stage), None);
    }

    #[test]
    fn duplicate_missing_and_self_dependencies_fail_closed() {
        let pack_id = pack("org.canisend.graph-test");
        let duplicate = WorkflowPackWorkflowDefinition {
            stages: vec![
                stage(
                    "intake",
                    &[],
                    WorkflowPackStageOutput::Sources,
                    &[ExecutionMode::ManualImport],
                ),
                stage(
                    "intake",
                    &[],
                    WorkflowPackStageOutput::Sources,
                    &[ExecutionMode::ManualImport],
                ),
            ],
            terminal_stage: item("intake"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &duplicate),
            Err(WorkflowPackStageGraphError::DuplicateStage { .. })
        ));

        let missing = WorkflowPackWorkflowDefinition {
            stages: vec![stage(
                "finish",
                &["missing"],
                WorkflowPackStageOutput::Render,
                &[ExecutionMode::Deterministic],
            )],
            terminal_stage: item("finish"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &missing),
            Err(WorkflowPackStageGraphError::MissingDependency { .. })
        ));

        let duplicate_dependency = WorkflowPackWorkflowDefinition {
            stages: vec![
                stage(
                    "intake",
                    &[],
                    WorkflowPackStageOutput::Sources,
                    &[ExecutionMode::ManualImport],
                ),
                stage(
                    "finish",
                    &["intake", "intake"],
                    WorkflowPackStageOutput::Render,
                    &[ExecutionMode::Deterministic],
                ),
            ],
            terminal_stage: item("finish"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &duplicate_dependency),
            Err(WorkflowPackStageGraphError::DuplicateDependency { .. })
        ));

        let self_dependency = WorkflowPackWorkflowDefinition {
            stages: vec![stage(
                "finish",
                &["finish"],
                WorkflowPackStageOutput::Render,
                &[ExecutionMode::Deterministic],
            )],
            terminal_stage: item("finish"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id, &self_dependency),
            Err(WorkflowPackStageGraphError::SelfDependency { .. })
        ));
    }

    #[test]
    fn cycle_unreachable_and_unknown_terminal_fail_closed() {
        let pack_id = pack("org.canisend.graph-test");
        let cycle = WorkflowPackWorkflowDefinition {
            stages: vec![
                stage(
                    "first",
                    &["second"],
                    WorkflowPackStageOutput::None,
                    &[ExecutionMode::HostAgent],
                ),
                stage(
                    "second",
                    &["first"],
                    WorkflowPackStageOutput::Render,
                    &[ExecutionMode::Deterministic],
                ),
            ],
            terminal_stage: item("second"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &cycle),
            Err(WorkflowPackStageGraphError::Cycle)
        ));

        let unreachable = WorkflowPackWorkflowDefinition {
            stages: vec![
                stage(
                    "intake",
                    &[],
                    WorkflowPackStageOutput::Sources,
                    &[ExecutionMode::ManualImport],
                ),
                stage(
                    "orphan",
                    &[],
                    WorkflowPackStageOutput::None,
                    &[ExecutionMode::UserDecision],
                ),
                stage(
                    "finish",
                    &["intake"],
                    WorkflowPackStageOutput::Render,
                    &[ExecutionMode::Deterministic],
                ),
            ],
            terminal_stage: item("finish"),
        };
        assert_eq!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &unreachable),
            Err(WorkflowPackStageGraphError::NotTerminalReachable {
                terminal_stage: id(&pack_id, "finish"),
                stages: vec![id(&pack_id, "orphan")],
            })
        );

        let mut unknown_terminal = valid_workflow();
        unknown_terminal.terminal_stage = item("missing");
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id, &unknown_terminal),
            Err(WorkflowPackStageGraphError::TerminalStageMissing { .. })
        ));
    }

    #[test]
    fn stage_and_execution_mode_limits_are_rechecked_by_the_compiler() {
        let pack_id = pack("org.canisend.graph-test");
        let empty = WorkflowPackWorkflowDefinition {
            stages: Vec::new(),
            terminal_stage: item("finish"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &empty),
            Err(WorkflowPackStageGraphError::StageCountInvalid { actual: 0, .. })
        ));

        let oversized = WorkflowPackWorkflowDefinition {
            stages: (0..=WORKFLOW_PACK_MAX_STAGES)
                .map(|index| {
                    stage(
                        &format!("stage-{index}"),
                        &[],
                        WorkflowPackStageOutput::None,
                        &[ExecutionMode::Deterministic],
                    )
                })
                .collect(),
            terminal_stage: item("stage-0"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &oversized),
            Err(WorkflowPackStageGraphError::StageCountInvalid { actual, .. })
                if actual == WORKFLOW_PACK_MAX_STAGES + 1
        ));

        let no_mode = WorkflowPackWorkflowDefinition {
            stages: vec![stage("finish", &[], WorkflowPackStageOutput::Render, &[])],
            terminal_stage: item("finish"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &no_mode),
            Err(WorkflowPackStageGraphError::ExecutionModeCountInvalid { actual: 0, .. })
        ));

        let too_many_modes = WorkflowPackWorkflowDefinition {
            stages: vec![stage(
                "finish",
                &[],
                WorkflowPackStageOutput::Render,
                &[
                    ExecutionMode::Deterministic,
                    ExecutionMode::HostAgent,
                    ExecutionMode::ConfiguredProvider,
                    ExecutionMode::UserDecision,
                    ExecutionMode::ManualImport,
                    ExecutionMode::Deterministic,
                ],
            )],
            terminal_stage: item("finish"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id.clone(), &too_many_modes),
            Err(WorkflowPackStageGraphError::ExecutionModeCountInvalid { actual: 6, .. })
        ));

        let duplicate_mode = WorkflowPackWorkflowDefinition {
            stages: vec![stage(
                "finish",
                &[],
                WorkflowPackStageOutput::Render,
                &[ExecutionMode::Deterministic, ExecutionMode::Deterministic],
            )],
            terminal_stage: item("finish"),
        };
        assert!(matches!(
            WorkflowPackStageGraph::try_new(pack_id, &duplicate_mode),
            Err(WorkflowPackStageGraphError::DuplicateExecutionMode { .. })
        ));
    }
}
