use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    AGENT_V4_PROTOCOL, AgentTaskKindV4, ContractViolation, OperationId, SemanticValidate,
    WORKSPACE_V4_FORMAT,
};

pub const OPERATION_REGISTRY_V4_FORMAT: &str = "canisend.operation-registry/v4";
pub const OPERATION_REGISTRY_V4_VERSION: u16 = 4;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum OperationPhaseV4 {
    Read,
    Preview,
    Commit,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum OperationContextV4 {
    Host,
    Workspace,
    Application,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationAdaptersV4 {
    pub cli: String,
    pub mcp: String,
    pub tauri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationDefinitionV4 {
    pub id: OperationId,
    pub phase: OperationPhaseV4,
    pub context: OperationContextV4,
    pub agent_task: Option<AgentTaskKindV4>,
    pub adapters: OperationAdaptersV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationRegistryV4 {
    pub format: String,
    pub version: u16,
    pub workspace_format: String,
    pub agent_protocol: String,
    pub operations: Vec<OperationDefinitionV4>,
    pub compatibility_aliases_supported: bool,
}

impl OperationRegistryV4 {
    #[must_use]
    pub fn operation(&self, id: &str) -> Option<&OperationDefinitionV4> {
        self.operations
            .iter()
            .find(|operation| operation.id.as_str() == id)
    }
}

impl SemanticValidate for OperationRegistryV4 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if self.format != OPERATION_REGISTRY_V4_FORMAT
            || self.version != OPERATION_REGISTRY_V4_VERSION
            || self.workspace_format != WORKSPACE_V4_FORMAT
            || self.agent_protocol != AGENT_V4_PROTOCOL
        {
            violations.push(ContractViolation::new(
                "operation_v4.version_invalid",
                "",
                "operation registry must bind clean Workspace v4 and Agent v4 exactly",
            ));
        }
        if self.compatibility_aliases_supported {
            violations.push(ContractViolation::new(
                "operation_v4.compatibility_alias_forbidden",
                "/compatibility_aliases_supported",
                "the clean-v4 operation registry cannot advertise compatibility aliases",
            ));
        }

        let required_namespaces = [
            "workspace",
            "application",
            "profile",
            "source",
            "evidence",
            "requirement",
            "plan",
            "deliverable",
            "review",
            "export",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let actual_namespaces = self
            .operations
            .iter()
            .filter_map(|operation| operation.id.as_str().split('.').next())
            .collect::<BTreeSet<_>>();
        if actual_namespaces != required_namespaces {
            violations.push(ContractViolation::new(
                "operation_v4.namespace_registry_incomplete",
                "/operations",
                "registry must contain only and all neutral v4 operation namespaces",
            ));
        }

        let mut operation_ids = BTreeSet::new();
        let mut cli_adapters = BTreeSet::new();
        let mut mcp_adapters = BTreeSet::new();
        let mut tauri_adapters = BTreeSet::new();
        let mut phase_index = BTreeMap::new();
        for (index, operation) in self.operations.iter().enumerate() {
            let id = operation.id.as_str();
            if !operation_ids.insert(id) {
                violations.push(ContractViolation::new(
                    "operation_v4.duplicate_id",
                    format!("/operations/{index}/id"),
                    "operation IDs must be unique",
                ));
            }
            if contains_legacy_vocabulary(id) {
                violations.push(ContractViolation::new(
                    "operation_v4.legacy_vocabulary_forbidden",
                    format!("/operations/{index}/id"),
                    "clean-v4 operation IDs cannot contain domain or legacy version aliases",
                ));
            }
            validate_adapter(
                &operation.adapters.cli,
                &expected_cli_adapter(id),
                &mut cli_adapters,
                index,
                "cli",
                &mut violations,
            );
            validate_adapter(
                &operation.adapters.mcp,
                &expected_mcp_adapter(id),
                &mut mcp_adapters,
                index,
                "mcp",
                &mut violations,
            );
            validate_adapter(
                &operation.adapters.tauri,
                &expected_tauri_adapter(id),
                &mut tauri_adapters,
                index,
                "tauri",
                &mut violations,
            );
            validate_phase_suffix(operation, index, &mut violations);
            if operation.context == OperationContextV4::Host
                && !id.starts_with("workspace.initialize.")
            {
                violations.push(ContractViolation::new(
                    "operation_v4.host_context_invalid",
                    format!("/operations/{index}/context"),
                    "only clean Workspace initialization may run before Workspace authority exists",
                ));
            }
            if operation.context != expected_context(id) {
                violations.push(ContractViolation::new(
                    "operation_v4.context_invalid",
                    format!("/operations/{index}/context"),
                    "operation context does not match the canonical authority boundary",
                ));
            }
            let accepting_tasks = AgentTaskKindV4::ALL
                .into_iter()
                .filter(|task| task.accepts_operation(id))
                .collect::<Vec<_>>();
            let expected_tasks = operation.agent_task.into_iter().collect::<Vec<_>>();
            if accepting_tasks != expected_tasks {
                violations.push(ContractViolation::new(
                    "operation_v4.agent_task_mismatch",
                    format!("/operations/{index}/agent_task"),
                    "operation must belong to exactly one declared Agent task or to no Agent task",
                ));
            }
            phase_index.insert(
                id.to_owned(),
                (operation.phase, operation.context, operation.agent_task),
            );
        }

        for (index, operation) in self.operations.iter().enumerate() {
            if !matches!(
                operation.phase,
                OperationPhaseV4::Preview | OperationPhaseV4::Commit
            ) {
                continue;
            }
            let id = operation.id.as_str();
            let base = id
                .strip_suffix(".preview")
                .or_else(|| id.strip_suffix(".commit"))
                .expect("phase suffix was validated");
            let counterpart_phase = match operation.phase {
                OperationPhaseV4::Preview => OperationPhaseV4::Commit,
                OperationPhaseV4::Commit => OperationPhaseV4::Preview,
                OperationPhaseV4::Read => unreachable!(),
            };
            let counterpart_id = format!(
                "{base}.{}",
                match counterpart_phase {
                    OperationPhaseV4::Preview => "preview",
                    OperationPhaseV4::Commit => "commit",
                    OperationPhaseV4::Read => unreachable!(),
                }
            );
            if phase_index.get(&counterpart_id)
                != Some(&(counterpart_phase, operation.context, operation.agent_task))
            {
                violations.push(ContractViolation::new(
                    "operation_v4.preview_commit_pair_incomplete",
                    format!("/operations/{index}"),
                    "every mutation requires a preview/commit pair with identical context and task",
                ));
            }
        }
        violations
    }
}

fn validate_phase_suffix(
    operation: &OperationDefinitionV4,
    index: usize,
    violations: &mut Vec<ContractViolation>,
) {
    let id = operation.id.as_str();
    let valid = match operation.phase {
        OperationPhaseV4::Read => !id.ends_with(".preview") && !id.ends_with(".commit"),
        OperationPhaseV4::Preview => id.ends_with(".preview"),
        OperationPhaseV4::Commit => id.ends_with(".commit"),
    };
    if !valid {
        violations.push(ContractViolation::new(
            "operation_v4.phase_suffix_mismatch",
            format!("/operations/{index}/phase"),
            "operation phase and canonical ID suffix must agree",
        ));
    }
}

fn validate_adapter(
    actual: &str,
    expected: &str,
    seen: &mut BTreeSet<String>,
    index: usize,
    surface: &str,
    violations: &mut Vec<ContractViolation>,
) {
    if actual != expected || !seen.insert(actual.to_owned()) {
        violations.push(ContractViolation::new(
            "operation_v4.adapter_invalid",
            format!("/operations/{index}/adapters/{surface}"),
            "surface adapter must be uniquely and mechanically derived from the operation ID",
        ));
    }
}

fn contains_legacy_vocabulary(value: &str) -> bool {
    ["job", "academic", "generic", "v2", "v3"]
        .iter()
        .any(|needle| value.contains(needle))
}

fn expected_context(id: &str) -> OperationContextV4 {
    if id.starts_with("workspace.initialize.") {
        return OperationContextV4::Host;
    }
    if id.starts_with("workspace.")
        || id.starts_with("profile.")
        || (id.starts_with("evidence.") && !id.starts_with("evidence.association."))
        || id == "application.list"
        || id.starts_with("application.create.")
    {
        return OperationContextV4::Workspace;
    }
    OperationContextV4::Application
}

fn expected_cli_adapter(id: &str) -> String {
    id.replace('.', " ")
}

fn expected_mcp_adapter(id: &str) -> String {
    format!("canisend_{}", id.replace('.', "_"))
}

fn expected_tauri_adapter(id: &str) -> String {
    id.replace('.', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(id: &str, phase: OperationPhaseV4) -> OperationDefinitionV4 {
        OperationDefinitionV4 {
            id: OperationId::try_new(id).expect("operation ID"),
            phase,
            context: OperationContextV4::Application,
            agent_task: Some(AgentTaskKindV4::Drafting),
            adapters: OperationAdaptersV4 {
                cli: expected_cli_adapter(id),
                mcp: expected_mcp_adapter(id),
                tauri: expected_tauri_adapter(id),
            },
        }
    }

    #[test]
    fn adapter_names_are_exact_mechanical_projections() {
        let operation = operation("deliverable.draft.preview", OperationPhaseV4::Preview);
        assert_eq!(operation.adapters.cli, "deliverable draft preview");
        assert_eq!(operation.adapters.mcp, "canisend_deliverable_draft_preview");
        assert_eq!(operation.adapters.tauri, "deliverable_draft_preview");
    }

    #[test]
    fn mutation_requires_an_exact_counterpart() {
        let registry = OperationRegistryV4 {
            format: OPERATION_REGISTRY_V4_FORMAT.to_owned(),
            version: OPERATION_REGISTRY_V4_VERSION,
            workspace_format: WORKSPACE_V4_FORMAT.to_owned(),
            agent_protocol: AGENT_V4_PROTOCOL.to_owned(),
            operations: vec![operation(
                "deliverable.draft.preview",
                OperationPhaseV4::Preview,
            )],
            compatibility_aliases_supported: false,
        };
        assert!(
            registry
                .validate_semantics()
                .iter()
                .any(|violation| violation.code == "operation_v4.preview_commit_pair_incomplete")
        );
    }
}
