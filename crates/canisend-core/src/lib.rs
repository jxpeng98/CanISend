#![forbid(unsafe_code)]

use canisend_contracts::{Capability, CapabilityStatus, SemanticVersion};

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
            planned("job.intake"),
            planned("discovery.refresh"),
            planned("workflow.execute"),
            planned("render.pdf"),
        ]
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

    use super::CapabilityRegistry;

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
                item.id == "job.intake" && item.status == CapabilityStatus::Planned
            })
        );
    }
}
