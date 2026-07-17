#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    AgentStageCapability, ArtifactKind, Capability, CapabilityStatus, ExecutionMode,
    SemanticVersion, StageDescriptor, WorkflowStage,
};
use thiserror::Error;

#[derive(Debug, Default, Clone, Copy)]
pub struct CapabilityRegistry;

impl CapabilityRegistry {
    #[must_use]
    pub fn built_in() -> Vec<Capability> {
        vec![
            available("product.version"),
            available("product.doctor"),
            available("agent.capabilities"),
            available("agent.context"),
            available("resources.manifest"),
            available("resource.list"),
            available("schema.list"),
            available("workspace.lifecycle"),
            available("job.intake"),
            available("discovery.refresh"),
            available("task.lifecycle"),
            available("criteria.lifecycle"),
            available("workflow.execute"),
            planned("render.pdf"),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StageGraphError {
    #[error("workflow stage is declared more than once: {0}")]
    DuplicateStage(&'static str),
    #[error("workflow stage has a duplicate dependency: {stage} -> {dependency}")]
    DuplicateDependency {
        stage: &'static str,
        dependency: &'static str,
    },
    #[error("workflow stage depends on an undeclared stage: {stage} -> {dependency}")]
    MissingDependency {
        stage: &'static str,
        dependency: &'static str,
    },
    #[error("workflow artifact kind has multiple producers: {0:?}")]
    DuplicateOutput(ArtifactKind),
    #[error("workflow stage graph contains a cycle")]
    Cycle,
}

#[derive(Debug, Clone)]
pub struct StageGraph {
    descriptors: BTreeMap<WorkflowStage, StageDescriptor>,
    topological_order: Vec<WorkflowStage>,
}

impl StageGraph {
    #[must_use]
    pub fn built_in() -> Self {
        Self::try_new(built_in_descriptors()).expect("compiled stage graph must be valid")
    }

    pub fn try_new(descriptors: Vec<StageDescriptor>) -> Result<Self, StageGraphError> {
        let mut by_stage = BTreeMap::new();
        let mut output_owners = BTreeMap::new();
        for descriptor in descriptors {
            if by_stage.contains_key(&descriptor.stage) {
                return Err(StageGraphError::DuplicateStage(descriptor.stage.as_str()));
            }
            if output_owners
                .insert(descriptor.output_kind, descriptor.stage)
                .is_some()
            {
                return Err(StageGraphError::DuplicateOutput(descriptor.output_kind));
            }
            by_stage.insert(descriptor.stage, descriptor);
        }
        for descriptor in by_stage.values() {
            let mut unique = BTreeSet::new();
            for dependency in &descriptor.depends_on {
                if !unique.insert(*dependency) {
                    return Err(StageGraphError::DuplicateDependency {
                        stage: descriptor.stage.as_str(),
                        dependency: dependency.as_str(),
                    });
                }
                if !by_stage.contains_key(dependency) {
                    return Err(StageGraphError::MissingDependency {
                        stage: descriptor.stage.as_str(),
                        dependency: dependency.as_str(),
                    });
                }
            }
        }
        let topological_order = topological_order(&by_stage)?;
        Ok(Self {
            descriptors: by_stage,
            topological_order,
        })
    }

    #[must_use]
    pub fn descriptor(&self, stage: WorkflowStage) -> &StageDescriptor {
        self.descriptors
            .get(&stage)
            .expect("typed workflow stage is declared")
    }

    #[must_use]
    pub fn descriptors(&self) -> Vec<StageDescriptor> {
        self.topological_order
            .iter()
            .map(|stage| self.descriptor(*stage).clone())
            .collect()
    }

    #[must_use]
    pub fn topological_order(&self) -> &[WorkflowStage] {
        &self.topological_order
    }

    #[must_use]
    pub fn descendants(&self, stage: WorkflowStage) -> Vec<WorkflowStage> {
        let mut descendants = BTreeSet::new();
        let mut frontier = vec![stage];
        while let Some(parent) = frontier.pop() {
            for descriptor in self.descriptors.values() {
                if descriptor.depends_on.contains(&parent) && descendants.insert(descriptor.stage) {
                    frontier.push(descriptor.stage);
                }
            }
        }
        self.topological_order
            .iter()
            .copied()
            .filter(|candidate| descendants.contains(candidate))
            .collect()
    }

    #[must_use]
    pub fn supports_mode(&self, stage: WorkflowStage, mode: ExecutionMode) -> bool {
        self.descriptor(stage).execution_modes.contains(&mode)
    }
}

fn topological_order(
    descriptors: &BTreeMap<WorkflowStage, StageDescriptor>,
) -> Result<Vec<WorkflowStage>, StageGraphError> {
    let mut incoming = descriptors
        .iter()
        .map(|(stage, descriptor)| (*stage, descriptor.depends_on.len()))
        .collect::<BTreeMap<_, _>>();
    let mut ready = incoming
        .iter()
        .filter_map(|(stage, count)| (*count == 0).then_some(*stage))
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(descriptors.len());
    while let Some(stage) = ready.pop_first() {
        ordered.push(stage);
        for descriptor in descriptors.values() {
            if descriptor.depends_on.contains(&stage) {
                let count = incoming
                    .get_mut(&descriptor.stage)
                    .expect("declared stage has an incoming count");
                *count -= 1;
                if *count == 0 {
                    ready.insert(descriptor.stage);
                }
            }
        }
    }
    if ordered.len() == descriptors.len() {
        Ok(ordered)
    } else {
        Err(StageGraphError::Cycle)
    }
}

fn built_in_descriptors() -> Vec<StageDescriptor> {
    vec![
        descriptor(
            WorkflowStage::Intake,
            &[],
            ArtifactKind::SourceNormalizedText,
            &[ExecutionMode::ManualImport],
        ),
        descriptor(
            WorkflowStage::Parse,
            &[WorkflowStage::Intake],
            ArtifactKind::ParsedJob,
            &[ExecutionMode::HostAgent, ExecutionMode::ConfiguredProvider],
        ),
        descriptor(
            WorkflowStage::Criteria,
            &[WorkflowStage::Parse],
            ArtifactKind::Criteria,
            &[ExecutionMode::UserDecision],
        ),
        descriptor(
            WorkflowStage::Evidence,
            &[],
            ArtifactKind::EvidenceCatalog,
            &[
                ExecutionMode::ManualImport,
                ExecutionMode::HostAgent,
                ExecutionMode::ConfiguredProvider,
            ],
        ),
        descriptor(
            WorkflowStage::Match,
            &[WorkflowStage::Criteria, WorkflowStage::Evidence],
            ArtifactKind::EvidenceMatches,
            &[ExecutionMode::HostAgent, ExecutionMode::ConfiguredProvider],
        ),
        descriptor(
            WorkflowStage::Plan,
            &[WorkflowStage::Match],
            ArtifactKind::ApplicationPlan,
            &[ExecutionMode::UserDecision],
        ),
        descriptor(
            WorkflowStage::Draft,
            &[WorkflowStage::Plan],
            ArtifactKind::CoverLetter,
            &[ExecutionMode::HostAgent, ExecutionMode::ConfiguredProvider],
        ),
        descriptor(
            WorkflowStage::Review,
            &[WorkflowStage::Draft],
            ArtifactKind::ReviewFindings,
            &[
                ExecutionMode::HostAgent,
                ExecutionMode::ConfiguredProvider,
                ExecutionMode::UserDecision,
            ],
        ),
        descriptor(
            WorkflowStage::Package,
            &[WorkflowStage::Review],
            ArtifactKind::PackageManifest,
            &[ExecutionMode::Deterministic],
        ),
        descriptor(
            WorkflowStage::Render,
            &[WorkflowStage::Package],
            ArtifactKind::Pdf,
            &[ExecutionMode::Deterministic],
        ),
    ]
}

fn descriptor(
    stage: WorkflowStage,
    depends_on: &[WorkflowStage],
    output_kind: ArtifactKind,
    execution_modes: &[ExecutionMode],
) -> StageDescriptor {
    StageDescriptor {
        stage,
        depends_on: depends_on.to_vec(),
        output_kind,
        execution_modes: execution_modes.to_vec(),
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StageRegistry;

impl StageRegistry {
    #[must_use]
    pub const fn is_available(stage: WorkflowStage) -> bool {
        matches!(
            stage,
            WorkflowStage::Intake | WorkflowStage::Parse | WorkflowStage::Criteria
        )
    }

    #[must_use]
    pub fn built_in() -> Vec<AgentStageCapability> {
        let graph = StageGraph::built_in();
        let mut capabilities = Vec::with_capacity(graph.descriptors.len() + 1);
        for descriptor in graph.descriptors() {
            capabilities.push(AgentStageCapability {
                id: descriptor.stage.as_str().to_owned(),
                status: if Self::is_available(descriptor.stage) {
                    CapabilityStatus::Available
                } else {
                    CapabilityStatus::Planned
                },
                execution_modes: descriptor.execution_modes,
            });
            if descriptor.stage == WorkflowStage::Intake {
                capabilities.push(AgentStageCapability {
                    id: "discovery".to_owned(),
                    status: CapabilityStatus::Available,
                    execution_modes: vec![ExecutionMode::Deterministic, ExecutionMode::HostAgent],
                });
            }
        }
        capabilities
    }
}

fn available(id: &str) -> Capability {
    Capability {
        id: id.to_owned(),
        version: SemanticVersion::try_new("2.0.0").expect("static capability version is valid"),
        status: CapabilityStatus::Available,
    }
}

fn planned(id: &str) -> Capability {
    Capability {
        id: id.to_owned(),
        version: SemanticVersion::try_new("2.0.0").expect("static capability version is valid"),
        status: CapabilityStatus::Planned,
    }
}

#[cfg(test)]
mod tests {
    use canisend_contracts::{
        ArtifactKind, CapabilityStatus, ExecutionMode, StageDescriptor, WorkflowStage,
    };

    use super::{CapabilityRegistry, StageGraph, StageGraphError, StageRegistry};

    #[test]
    fn compiled_graph_is_acyclic_unique_and_mode_complete() {
        let graph = StageGraph::built_in();
        assert_eq!(graph.topological_order().len(), WorkflowStage::ALL.len());
        assert_eq!(graph.topological_order()[0], WorkflowStage::Intake);
        assert_eq!(
            graph.descendants(WorkflowStage::Criteria),
            vec![
                WorkflowStage::Match,
                WorkflowStage::Plan,
                WorkflowStage::Draft,
                WorkflowStage::Review,
                WorkflowStage::Package,
                WorkflowStage::Render,
            ]
        );
        for mode in [
            ExecutionMode::Deterministic,
            ExecutionMode::HostAgent,
            ExecutionMode::ConfiguredProvider,
            ExecutionMode::UserDecision,
        ] {
            assert!(
                graph
                    .descriptors()
                    .iter()
                    .any(|descriptor| descriptor.execution_modes.contains(&mode)),
                "missing workflow mode {mode:?}"
            );
        }
    }

    #[test]
    fn invalid_graphs_reject_cycles_and_duplicate_outputs() {
        let cycle = vec![
            StageDescriptor {
                stage: WorkflowStage::Intake,
                depends_on: vec![WorkflowStage::Parse],
                output_kind: ArtifactKind::SourceNormalizedText,
                execution_modes: vec![ExecutionMode::ManualImport],
            },
            StageDescriptor {
                stage: WorkflowStage::Parse,
                depends_on: vec![WorkflowStage::Intake],
                output_kind: ArtifactKind::ParsedJob,
                execution_modes: vec![ExecutionMode::HostAgent],
            },
        ];
        assert!(matches!(
            StageGraph::try_new(cycle),
            Err(StageGraphError::Cycle)
        ));

        let duplicate_output = vec![
            StageDescriptor {
                stage: WorkflowStage::Intake,
                depends_on: vec![],
                output_kind: ArtifactKind::ParsedJob,
                execution_modes: vec![ExecutionMode::ManualImport],
            },
            StageDescriptor {
                stage: WorkflowStage::Parse,
                depends_on: vec![WorkflowStage::Intake],
                output_kind: ArtifactKind::ParsedJob,
                execution_modes: vec![ExecutionMode::HostAgent],
            },
        ];
        assert!(matches!(
            StageGraph::try_new(duplicate_output),
            Err(StageGraphError::DuplicateOutput(ArtifactKind::ParsedJob))
        ));
    }

    #[test]
    fn registries_are_unique_and_truthful() {
        let capabilities = CapabilityRegistry::built_in();
        let mut ids = capabilities.iter().map(|item| &item.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), capabilities.len());
        for available_id in [
            "workspace.lifecycle",
            "job.intake",
            "discovery.refresh",
            "task.lifecycle",
        ] {
            assert!(capabilities.iter().any(|item| {
                item.id == available_id && item.status == CapabilityStatus::Available
            }));
        }
        let stages = StageRegistry::built_in();
        assert_eq!(
            stages
                .iter()
                .filter(|stage| stage.status == CapabilityStatus::Available)
                .count(),
            4
        );
        let mut stage_ids = stages
            .iter()
            .map(|stage| stage.id.as_str())
            .collect::<Vec<_>>();
        stage_ids.sort_unstable();
        stage_ids.dedup();
        assert_eq!(stage_ids.len(), stages.len());
    }
}
