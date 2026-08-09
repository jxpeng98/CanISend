use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

pub const OPERATION_REGISTRY_FORMAT: &str = "canisend.operation-registry/v1";

const BUILT_IN_OPERATION_REGISTRY: &str = include_str!("../operation-registry-v1.json");

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct OperationId(String);

impl OperationId {
    pub fn try_new(value: impl Into<String>) -> Result<Self, OperationRegistryError> {
        let value = value.into();
        validate_operation_id(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for OperationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for OperationId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationStatus {
    Implemented,
    Deprecated,
    DeferredBeta,
}

impl OperationStatus {
    pub const ALL: [Self; 3] = [Self::Implemented, Self::Deprecated, Self::DeferredBeta];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Deprecated => "deprecated",
            Self::DeferredBeta => "deferred-beta",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationClass {
    CanonicalLeaf,
    SharedLeaf,
    CompatibilityAlias,
    Composite,
    WildcardAlias,
    AdapterOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationPackScope {
    Any,
    GenericApplication,
    AcademicJob,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationSurface {
    Cli,
    Tauri,
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDefinition {
    pub id: OperationId,
    pub class: OperationClass,
    pub status: OperationStatus,
    pub pack_scope: OperationPackScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationStatusDefinition {
    pub status: OperationStatus,
    pub allowed_classes: Vec<OperationClass>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityOperationDefinition {
    pub id: OperationId,
    pub canonical_operation: OperationId,
    pub status: OperationStatus,
    pub pack_scope: OperationPackScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationOperationDefinition {
    pub id: OperationId,
    pub class: OperationClass,
    pub status: OperationStatus,
    pub members: Vec<OperationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceOperationBinding {
    pub leaf: String,
    pub operation: OperationId,
    pub pack_scope: OperationPackScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSurfaceRegistry {
    pub surface: OperationSurface,
    pub adapter_prefix: String,
    pub default_status: OperationStatus,
    pub default_pack_scope: OperationPackScope,
    pub leaves: Vec<String>,
    pub bindings: Vec<SurfaceOperationBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRegistry {
    pub format: String,
    pub version: u32,
    pub statuses: Vec<OperationStatusDefinition>,
    pub operations: Vec<OperationDefinition>,
    pub compatibility_aliases: Vec<CompatibilityOperationDefinition>,
    pub presentation_aliases: Vec<PresentationOperationDefinition>,
    pub surfaces: Vec<OperationSurfaceRegistry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOperationBinding {
    pub surface: OperationSurface,
    pub leaf: String,
    pub operation: OperationId,
    pub class: OperationClass,
    pub status: OperationStatus,
    pub pack_scope: OperationPackScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OperationRegistryError {
    #[error("operation registry JSON is invalid: {0}")]
    InvalidJson(String),
    #[error("operation registry is invalid: {0}")]
    Invalid(String),
}

impl OperationRegistry {
    pub fn parse(json: &str) -> Result<Self, OperationRegistryError> {
        let registry: Self = serde_json::from_str(json)
            .map_err(|error| OperationRegistryError::InvalidJson(error.to_string()))?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn built_in() -> Result<Self, OperationRegistryError> {
        Self::parse(BUILT_IN_OPERATION_REGISTRY)
    }

    pub fn validate(&self) -> Result<(), OperationRegistryError> {
        if self.format != OPERATION_REGISTRY_FORMAT {
            return invalid(format!(
                "format must be {OPERATION_REGISTRY_FORMAT}, found {}",
                self.format
            ));
        }
        if self.version != 1 {
            return invalid(format!("version must be 1, found {}", self.version));
        }

        let mut status_classes = BTreeMap::new();
        for definition in &self.statuses {
            let allowed = definition
                .allowed_classes
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            if allowed.is_empty() || allowed.len() != definition.allowed_classes.len() {
                return invalid(format!(
                    "status {} must have a non-empty duplicate-free class registry",
                    definition.status.as_str()
                ));
            }
            if status_classes.insert(definition.status, allowed).is_some() {
                return invalid(format!(
                    "duplicate status registry for {}",
                    definition.status.as_str()
                ));
            }
        }
        let expected_statuses = OperationStatus::ALL.into_iter().collect::<BTreeSet<_>>();
        let actual_statuses = status_classes.keys().copied().collect::<BTreeSet<_>>();
        if actual_statuses != expected_statuses {
            return invalid(format!(
                "status registry drifted: expected {expected_statuses:?}, found {actual_statuses:?}"
            ));
        }

        let validate_status_class = |status: OperationStatus,
                                     class: OperationClass,
                                     id: &OperationId|
         -> Result<(), OperationRegistryError> {
            if status_classes
                .get(&status)
                .is_some_and(|allowed| allowed.contains(&class))
            {
                Ok(())
            } else {
                invalid(format!(
                    "operation {id} class {class:?} is not allowed for status {}",
                    status.as_str()
                ))
            }
        };

        let mut declared = BTreeMap::new();
        for operation in &self.operations {
            if operation.id.as_str().contains('*') {
                return invalid(format!(
                    "callable operation {} cannot contain a wildcard",
                    operation.id
                ));
            }
            if !matches!(
                operation.class,
                OperationClass::CanonicalLeaf | OperationClass::SharedLeaf
            ) {
                return invalid(format!(
                    "declared operation {} must be canonical-leaf or shared-leaf",
                    operation.id
                ));
            }
            if operation.status == OperationStatus::Deprecated {
                return invalid(format!(
                    "canonical operation {} cannot be deprecated",
                    operation.id
                ));
            }
            validate_status_class(operation.status, operation.class, &operation.id)?;
            insert_unique(&mut declared, &operation.id, operation.class)?;
        }

        let canonical_ids = self
            .operations
            .iter()
            .map(|operation| operation.id.clone())
            .collect::<BTreeSet<_>>();
        let mut compatibility = BTreeMap::new();
        for alias in &self.compatibility_aliases {
            if alias.id.as_str().contains('*') {
                return invalid(format!(
                    "compatibility alias {} cannot contain a wildcard",
                    alias.id
                ));
            }
            if alias.status != OperationStatus::Deprecated {
                return invalid(format!(
                    "compatibility alias {} must be deprecated",
                    alias.id
                ));
            }
            if !canonical_ids.contains(&alias.canonical_operation) {
                return invalid(format!(
                    "compatibility alias {} targets missing canonical operation {}",
                    alias.id, alias.canonical_operation
                ));
            }
            if alias.pack_scope != OperationPackScope::AcademicJob {
                return invalid(format!(
                    "compatibility alias {} must be bounded to academic-job",
                    alias.id
                ));
            }
            validate_status_class(alias.status, OperationClass::CompatibilityAlias, &alias.id)?;
            if declared.contains_key(&alias.id)
                || compatibility.insert(alias.id.clone(), alias).is_some()
            {
                return invalid(format!("duplicate operation id {}", alias.id));
            }
        }

        let expected_surfaces = BTreeSet::from([
            OperationSurface::Cli,
            OperationSurface::Tauri,
            OperationSurface::Mcp,
        ]);
        let actual_surfaces = self
            .surfaces
            .iter()
            .map(|surface| surface.surface)
            .collect::<BTreeSet<_>>();
        if self.surfaces.len() != expected_surfaces.len() || actual_surfaces != expected_surfaces {
            return invalid("registry must contain exactly one CLI, Tauri, and MCP surface");
        }

        let mut resolved = Vec::new();
        let mut all_ids = declared.keys().cloned().collect::<BTreeSet<_>>();
        all_ids.extend(compatibility.keys().cloned());
        for surface in &self.surfaces {
            validate_adapter_prefix(&surface.adapter_prefix)?;
            let adapter_status_id =
                OperationId::try_new(format!("{}.adapter-status", surface.adapter_prefix))?;
            validate_status_class(
                surface.default_status,
                OperationClass::AdapterOnly,
                &adapter_status_id,
            )?;
            let mut leaves = BTreeSet::new();
            for leaf in &surface.leaves {
                validate_surface_leaf(surface.surface, leaf)?;
                if !leaves.insert(leaf.as_str()) {
                    return invalid(format!("duplicate {:?} leaf {leaf}", surface.surface));
                }
            }
            let mut overrides = BTreeMap::new();
            for binding in &surface.bindings {
                if !leaves.contains(binding.leaf.as_str()) {
                    return invalid(format!(
                        "{:?} binding references missing leaf {}",
                        surface.surface, binding.leaf
                    ));
                }
                if overrides.insert(binding.leaf.as_str(), binding).is_some() {
                    return invalid(format!(
                        "duplicate {:?} binding for {}",
                        surface.surface, binding.leaf
                    ));
                }
                let target_scope = self.operation_scope(&binding.operation).ok_or_else(|| {
                    OperationRegistryError::Invalid(format!(
                        "{:?} leaf {} targets undeclared operation {}",
                        surface.surface, binding.leaf, binding.operation
                    ))
                })?;
                if binding.pack_scope != target_scope {
                    return invalid(format!(
                        "{:?} leaf {} has {:?} Pack scope but operation {} requires {:?}",
                        surface.surface,
                        binding.leaf,
                        binding.pack_scope,
                        binding.operation,
                        target_scope
                    ));
                }
            }

            let mut surface_operations = BTreeSet::new();
            for leaf in &surface.leaves {
                let binding = if let Some(binding) = overrides.get(leaf.as_str()) {
                    let class = if compatibility.contains_key(&binding.operation) {
                        OperationClass::CompatibilityAlias
                    } else {
                        self.operations
                            .iter()
                            .find(|operation| operation.id == binding.operation)
                            .expect("validated operation target")
                            .class
                    };
                    let status = if compatibility.contains_key(&binding.operation) {
                        OperationStatus::Deprecated
                    } else {
                        self.operation_status(&binding.operation)
                            .expect("validated operation status")
                    };
                    ResolvedOperationBinding {
                        surface: surface.surface,
                        leaf: leaf.clone(),
                        operation: binding.operation.clone(),
                        class,
                        status,
                        pack_scope: binding.pack_scope,
                    }
                } else {
                    let operation = adapter_operation_id(&surface.adapter_prefix, leaf)?;
                    if all_ids.contains(&operation) {
                        return invalid(format!(
                            "adapter-only {:?} leaf {leaf} collides with declared operation {operation}",
                            surface.surface
                        ));
                    }
                    all_ids.insert(operation.clone());
                    ResolvedOperationBinding {
                        surface: surface.surface,
                        leaf: leaf.clone(),
                        operation,
                        class: OperationClass::AdapterOnly,
                        status: surface.default_status,
                        pack_scope: surface.default_pack_scope,
                    }
                };
                if !surface_operations.insert(binding.operation.clone()) {
                    return invalid(format!(
                        "{:?} falsely shares operation {} across more than one leaf",
                        surface.surface, binding.operation
                    ));
                }
                resolved.push(binding);
            }
        }

        let binding_surfaces = resolved.iter().fold(
            BTreeMap::<OperationId, BTreeSet<OperationSurface>>::new(),
            |mut map, binding| {
                map.entry(binding.operation.clone())
                    .or_default()
                    .insert(binding.surface);
                map
            },
        );
        for operation in &self.operations {
            let count = binding_surfaces.get(&operation.id).map_or(0, BTreeSet::len);
            match operation.class {
                OperationClass::SharedLeaf if count < 2 => {
                    return invalid(format!(
                        "shared operation {} is bound on only {count} surface(s)",
                        operation.id
                    ));
                }
                OperationClass::CanonicalLeaf if count > 1 => {
                    return invalid(format!(
                        "canonical operation {} is shared by {count} surfaces without shared-leaf classification",
                        operation.id
                    ));
                }
                _ => {}
            }
        }
        for alias in &self.compatibility_aliases {
            if !binding_surfaces.contains_key(&alias.id) {
                return invalid(format!(
                    "compatibility alias {} has no surface binding",
                    alias.id
                ));
            }
        }

        let mut presentation_ids = BTreeSet::new();
        for alias in &self.presentation_aliases {
            if !matches!(
                alias.class,
                OperationClass::Composite | OperationClass::WildcardAlias
            ) {
                return invalid(format!(
                    "presentation alias {} must be composite or wildcard-alias",
                    alias.id
                ));
            }
            if alias.class == OperationClass::WildcardAlias && !alias.id.as_str().ends_with(".*") {
                return invalid(format!("wildcard alias {} must end in .*", alias.id));
            }
            if alias.class == OperationClass::Composite && alias.id.as_str().ends_with(".*") {
                return invalid(format!("composite alias {} cannot end in .*", alias.id));
            }
            validate_status_class(alias.status, alias.class, &alias.id)?;
            if all_ids.contains(&alias.id) || !presentation_ids.insert(alias.id.clone()) {
                return invalid(format!("duplicate operation id {}", alias.id));
            }
            if alias.members.is_empty() {
                return invalid(format!("presentation alias {} has no members", alias.id));
            }
            let mut members = BTreeSet::new();
            for member in &alias.members {
                if !all_ids.contains(member) {
                    return invalid(format!(
                        "presentation alias {} references missing member {member}",
                        alias.id
                    ));
                }
                if !members.insert(member) {
                    return invalid(format!(
                        "presentation alias {} repeats member {member}",
                        alias.id
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn resolved_bindings(
        &self,
    ) -> Result<Vec<ResolvedOperationBinding>, OperationRegistryError> {
        self.validate()?;
        let declared = self
            .operations
            .iter()
            .map(|operation| (operation.id.clone(), operation))
            .collect::<BTreeMap<_, _>>();
        let compatibility = self
            .compatibility_aliases
            .iter()
            .map(|operation| (operation.id.clone(), operation))
            .collect::<BTreeMap<_, _>>();
        let mut bindings = Vec::new();
        for surface in &self.surfaces {
            let overrides = surface
                .bindings
                .iter()
                .map(|binding| (binding.leaf.as_str(), binding))
                .collect::<BTreeMap<_, _>>();
            for leaf in &surface.leaves {
                if let Some(binding) = overrides.get(leaf.as_str()) {
                    let (class, status) = if let Some(operation) = declared.get(&binding.operation)
                    {
                        (operation.class, operation.status)
                    } else {
                        (
                            OperationClass::CompatibilityAlias,
                            compatibility
                                .get(&binding.operation)
                                .expect("validated compatibility target")
                                .status,
                        )
                    };
                    bindings.push(ResolvedOperationBinding {
                        surface: surface.surface,
                        leaf: leaf.clone(),
                        operation: binding.operation.clone(),
                        class,
                        status,
                        pack_scope: binding.pack_scope,
                    });
                } else {
                    bindings.push(ResolvedOperationBinding {
                        surface: surface.surface,
                        leaf: leaf.clone(),
                        operation: adapter_operation_id(&surface.adapter_prefix, leaf)?,
                        class: OperationClass::AdapterOnly,
                        status: surface.default_status,
                        pack_scope: surface.default_pack_scope,
                    });
                }
            }
        }
        Ok(bindings)
    }

    pub fn surface_leaves(
        &self,
        surface: OperationSurface,
    ) -> Result<BTreeSet<String>, OperationRegistryError> {
        self.validate()?;
        Ok(self
            .surfaces
            .iter()
            .find(|entry| entry.surface == surface)
            .expect("validated surface")
            .leaves
            .iter()
            .cloned()
            .collect())
    }

    pub fn binding(
        &self,
        surface: OperationSurface,
        leaf: &str,
    ) -> Result<Option<ResolvedOperationBinding>, OperationRegistryError> {
        Ok(self
            .resolved_bindings()?
            .into_iter()
            .find(|binding| binding.surface == surface && binding.leaf == leaf))
    }

    pub fn compatibility_alias(&self, id: &str) -> Option<&CompatibilityOperationDefinition> {
        self.compatibility_aliases
            .iter()
            .find(|alias| alias.id.as_str() == id)
    }

    fn operation_scope(&self, id: &OperationId) -> Option<OperationPackScope> {
        self.operations
            .iter()
            .find(|operation| &operation.id == id)
            .map(|operation| operation.pack_scope)
            .or_else(|| {
                self.compatibility_aliases
                    .iter()
                    .find(|operation| &operation.id == id)
                    .map(|operation| operation.pack_scope)
            })
    }

    fn operation_status(&self, id: &OperationId) -> Option<OperationStatus> {
        self.operations
            .iter()
            .find(|operation| &operation.id == id)
            .map(|operation| operation.status)
    }
}

fn validate_operation_id(value: &str) -> Result<(), OperationRegistryError> {
    if value.is_empty()
        || value.starts_with('.')
        || value.ends_with('.')
        || value.contains("..")
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'-' | b'_' | b'*')
        })
    {
        return invalid(format!("invalid operation id `{value}`"));
    }
    if value.contains('*') && !value.ends_with(".*") {
        return invalid(format!(
            "wildcard is only allowed as a final .* in `{value}`"
        ));
    }
    Ok(())
}

fn validate_adapter_prefix(value: &str) -> Result<(), OperationRegistryError> {
    validate_operation_id(value)?;
    if value.contains('*') {
        return invalid(format!("adapter prefix cannot contain wildcard: {value}"));
    }
    Ok(())
}

fn validate_surface_leaf(
    surface: OperationSurface,
    value: &str,
) -> Result<(), OperationRegistryError> {
    if value.is_empty()
        || value.trim() != value
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'-'
                || (surface == OperationSurface::Cli && byte == b' ')
                || (surface != OperationSurface::Cli && byte == b'_')
        })
    {
        return invalid(format!("invalid {:?} leaf `{value}`", surface));
    }
    Ok(())
}

fn adapter_operation_id(prefix: &str, leaf: &str) -> Result<OperationId, OperationRegistryError> {
    let normalized = leaf.replace([' ', '_'], ".");
    OperationId::try_new(format!("{prefix}.{normalized}"))
}

fn insert_unique(
    values: &mut BTreeMap<OperationId, OperationClass>,
    id: &OperationId,
    class: OperationClass,
) -> Result<(), OperationRegistryError> {
    if values.insert(id.clone(), class).is_some() {
        return invalid(format!("duplicate operation id {id}"));
    }
    Ok(())
}

fn invalid<T>(message: impl Into<String>) -> Result<T, OperationRegistryError> {
    Err(OperationRegistryError::Invalid(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_registry_has_exact_clean_v4_cli_and_mcp_surfaces() {
        let registry = OperationRegistry::built_in().expect("built-in operation registry");
        let bindings = registry.resolved_bindings().expect("resolved bindings");
        assert_eq!(
            bindings
                .iter()
                .filter(|binding| binding.surface == OperationSurface::Cli)
                .count(),
            31
        );
        assert_eq!(
            bindings
                .iter()
                .filter(|binding| binding.surface == OperationSurface::Tauri)
                .count(),
            129
        );
        assert_eq!(
            bindings
                .iter()
                .filter(|binding| binding.surface == OperationSurface::Mcp)
                .count(),
            36
        );
        assert!(bindings.iter().all(|binding| {
            !matches!(
                binding.leaf.as_str(),
                "plan_generic_application" | "compose_generic_application"
            ) && !matches!(
                binding.operation.as_str(),
                "application.plan" | "application.compose"
            )
        }));
        assert!(registry.compatibility_aliases.is_empty());
        assert!(registry.presentation_aliases.iter().all(|alias| {
            matches!(alias.id.as_str(), "schema.*" | "resource.*")
                && alias.class == OperationClass::WildcardAlias
        }));
        assert!(
            registry
                .presentation_aliases
                .iter()
                .all(|alias| alias.id.as_str() != "application.dossier")
        );
    }

    #[test]
    fn registry_rejects_missing_duplicate_false_sharing_and_pack_mismatch() {
        let registry = OperationRegistry::built_in().expect("built-in operation registry");

        let mut missing = registry.clone();
        let mapped_leaf = missing.surfaces[0].bindings[0].leaf.clone();
        missing.surfaces[0]
            .leaves
            .retain(|leaf| leaf != &mapped_leaf);
        assert!(missing.validate().is_err());

        let mut duplicate = registry.clone();
        let repeated_leaf = duplicate.surfaces[0].leaves[0].clone();
        duplicate.surfaces[0].leaves.push(repeated_leaf);
        assert!(duplicate.validate().is_err());

        let mut falsely_shared = registry.clone();
        let operation = falsely_shared.surfaces[0].bindings[0].operation.clone();
        falsely_shared.surfaces[0].bindings[1].operation = operation;
        assert!(falsely_shared.validate().is_err());

        let mut pack_mismatch = registry;
        pack_mismatch.surfaces[0].bindings[0].pack_scope = OperationPackScope::AcademicJob;
        assert!(pack_mismatch.validate().is_err());

        let mut missing_status = OperationRegistry::built_in().expect("built-in registry");
        missing_status.statuses.pop();
        assert!(missing_status.validate().is_err());
    }
}
