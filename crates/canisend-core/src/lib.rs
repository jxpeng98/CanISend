#![forbid(unsafe_code)]

use canisend_contracts::{
    AgentStageCapability, Capability, CapabilityStatus, ExecutionMode, SemanticVersion,
};

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
            planned("workflow.execute"),
            planned("render.pdf"),
        ]
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StageRegistry;

impl StageRegistry {
    #[must_use]
    pub fn built_in() -> Vec<AgentStageCapability> {
        vec![
            stage(
                "intake",
                CapabilityStatus::Available,
                &[ExecutionMode::ManualImport],
            ),
            stage(
                "discovery",
                CapabilityStatus::Available,
                &[ExecutionMode::Deterministic, ExecutionMode::HostAgent],
            ),
            stage(
                "parse",
                CapabilityStatus::Planned,
                &[ExecutionMode::HostAgent],
            ),
            stage(
                "criteria",
                CapabilityStatus::Planned,
                &[ExecutionMode::UserDecision],
            ),
            stage(
                "evidence",
                CapabilityStatus::Planned,
                &[ExecutionMode::HostAgent],
            ),
            stage(
                "match",
                CapabilityStatus::Planned,
                &[ExecutionMode::HostAgent],
            ),
            stage(
                "plan",
                CapabilityStatus::Planned,
                &[ExecutionMode::UserDecision],
            ),
            stage(
                "draft",
                CapabilityStatus::Planned,
                &[ExecutionMode::HostAgent],
            ),
            stage(
                "review",
                CapabilityStatus::Planned,
                &[ExecutionMode::HostAgent],
            ),
            stage(
                "package",
                CapabilityStatus::Planned,
                &[ExecutionMode::Deterministic],
            ),
            stage(
                "render",
                CapabilityStatus::Planned,
                &[ExecutionMode::Deterministic],
            ),
        ]
    }
}

fn stage(
    id: &str,
    status: CapabilityStatus,
    execution_modes: &[ExecutionMode],
) -> AgentStageCapability {
    AgentStageCapability {
        id: id.to_owned(),
        status,
        execution_modes: execution_modes.to_vec(),
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
    use canisend_contracts::CapabilityStatus;

    use super::{CapabilityRegistry, StageRegistry};

    #[test]
    fn registry_is_unique_and_truthful() {
        let capabilities = CapabilityRegistry::built_in();
        let mut ids = capabilities.iter().map(|item| &item.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), capabilities.len());
        assert!(capabilities.iter().any(|item| {
            item.id == "workspace.lifecycle" && item.status == CapabilityStatus::Available
        }));
        assert!(
            capabilities.iter().any(|item| {
                item.id == "job.intake" && item.status == CapabilityStatus::Available
            })
        );
        assert!(capabilities.iter().any(|item| {
            item.id == "discovery.refresh" && item.status == CapabilityStatus::Available
        }));
        assert!(capabilities.iter().any(|item| {
            item.id == "task.lifecycle" && item.status == CapabilityStatus::Available
        }));
        let stages = StageRegistry::built_in();
        assert_eq!(
            stages
                .iter()
                .filter(|stage| stage.status == CapabilityStatus::Available)
                .count(),
            2
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
