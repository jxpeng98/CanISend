use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    DeliverableKindId, SafeRelativePath, SemanticVersion, Sha256Digest,
    WORKFLOW_PACK_MAX_DELIVERABLE_CARDINALITY, WORKFLOW_PACK_MAX_DELIVERABLE_KINDS,
    WorkflowPackCapabilityId, WorkflowPackDeliverableCatalog, WorkflowPackDeliverableDefinition,
    WorkflowPackId, WorkflowPackItemId, WorkflowPackLocalizedText, WorkflowPackResource,
    WorkflowPackResourceKind, WorkflowPackValidatorDefinition,
};
use serde_json::Value;
use thiserror::Error;

use crate::VerifiedWorkflowPackBundle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPackTemplateBinding {
    id: WorkflowPackItemId,
    path: SafeRelativePath,
    version: SemanticVersion,
    size_bytes: u64,
    sha256: Sha256Digest,
}

impl WorkflowPackTemplateBinding {
    #[must_use]
    pub const fn id(&self) -> &WorkflowPackItemId {
        &self.id
    }

    #[must_use]
    pub const fn path(&self) -> &SafeRelativePath {
        &self.path
    }

    #[must_use]
    pub const fn version(&self) -> &SemanticVersion {
        &self.version
    }

    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    #[must_use]
    pub const fn sha256(&self) -> &Sha256Digest {
        &self.sha256
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowPackValidatorBinding {
    id: WorkflowPackItemId,
    capability: WorkflowPackCapabilityId,
    parameters: BTreeMap<String, Value>,
}

impl WorkflowPackValidatorBinding {
    #[must_use]
    pub const fn id(&self) -> &WorkflowPackItemId {
        &self.id
    }

    #[must_use]
    pub const fn capability(&self) -> &WorkflowPackCapabilityId {
        &self.capability
    }

    #[must_use]
    pub const fn parameters(&self) -> &BTreeMap<String, Value> {
        &self.parameters
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowPackDeliverableDescriptor {
    kind: DeliverableKindId,
    local_id: WorkflowPackItemId,
    labels: WorkflowPackLocalizedText,
    minimum: u16,
    maximum: u16,
    template: Option<WorkflowPackTemplateBinding>,
    renderer: Option<WorkflowPackCapabilityId>,
    validators: Vec<WorkflowPackValidatorBinding>,
}

impl WorkflowPackDeliverableDescriptor {
    #[must_use]
    pub const fn kind(&self) -> &DeliverableKindId {
        &self.kind
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
    pub const fn minimum(&self) -> u16 {
        self.minimum
    }

    #[must_use]
    pub const fn maximum(&self) -> u16 {
        self.maximum
    }

    #[must_use]
    pub const fn template(&self) -> Option<&WorkflowPackTemplateBinding> {
        self.template.as_ref()
    }

    #[must_use]
    pub const fn renderer(&self) -> Option<&WorkflowPackCapabilityId> {
        self.renderer.as_ref()
    }

    #[must_use]
    pub fn validators(&self) -> &[WorkflowPackValidatorBinding] {
        &self.validators
    }

    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.minimum > 0
    }

    #[must_use]
    pub const fn allows_count(&self, count: u16) -> bool {
        count >= self.minimum && count <= self.maximum
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowPackDeliverableCatalogRuntime {
    pack_id: WorkflowPackId,
    descriptors: BTreeMap<DeliverableKindId, WorkflowPackDeliverableDescriptor>,
    order: Vec<DeliverableKindId>,
}

impl WorkflowPackDeliverableCatalogRuntime {
    pub fn try_new(
        pack_id: WorkflowPackId,
        catalog: &WorkflowPackDeliverableCatalog,
        resources: &[WorkflowPackResource],
        selected_renderers: &[WorkflowPackCapabilityId],
        selected_validators: &[WorkflowPackCapabilityId],
        validator_definitions: &[WorkflowPackValidatorDefinition],
    ) -> Result<Self, WorkflowPackDeliverableCatalogError> {
        validate_deliverable_count(catalog.kinds.len())?;
        let resources = index_resources(resources)?;
        let renderers = selected_renderers.iter().collect::<BTreeSet<_>>();
        let validators = index_validators(selected_validators, validator_definitions)?;
        let mut descriptors = BTreeMap::new();
        let mut order = Vec::with_capacity(catalog.kinds.len());
        for definition in &catalog.kinds {
            let descriptor =
                compile_descriptor(&pack_id, definition, &resources, &renderers, &validators)?;
            let kind = descriptor.kind.clone();
            if descriptors.insert(kind.clone(), descriptor).is_some() {
                return Err(WorkflowPackDeliverableCatalogError::DuplicateKind { kind });
            }
            order.push(kind);
        }
        Ok(Self {
            pack_id,
            descriptors,
            order,
        })
    }

    pub fn from_verified_bundle(
        bundle: &VerifiedWorkflowPackBundle,
    ) -> Result<Self, WorkflowPackDeliverableCatalogError> {
        let manifest = bundle.manifest();
        Self::try_new(
            manifest.id.clone(),
            &manifest.deliverables,
            &manifest.resources,
            &manifest.capabilities.renderers,
            &manifest.capabilities.validators,
            &manifest.validation.definitions,
        )
    }

    #[must_use]
    pub const fn pack_id(&self) -> &WorkflowPackId {
        &self.pack_id
    }

    #[must_use]
    pub fn kind_id(&self, local_id: &WorkflowPackItemId) -> DeliverableKindId {
        DeliverableKindId::from_parts(&self.pack_id, local_id)
    }

    #[must_use]
    pub fn descriptor(
        &self,
        kind: &DeliverableKindId,
    ) -> Option<&WorkflowPackDeliverableDescriptor> {
        self.descriptors.get(kind)
    }

    #[must_use]
    pub fn descriptor_for_local_id(
        &self,
        local_id: &WorkflowPackItemId,
    ) -> Option<&WorkflowPackDeliverableDescriptor> {
        self.descriptor(&self.kind_id(local_id))
    }

    #[must_use]
    pub fn descriptors(&self) -> Vec<&WorkflowPackDeliverableDescriptor> {
        self.order
            .iter()
            .map(|kind| {
                self.descriptors
                    .get(kind)
                    .expect("catalog order contains only compiled Deliverable kinds")
            })
            .collect()
    }

    #[must_use]
    pub fn required_kinds(&self) -> Vec<&DeliverableKindId> {
        self.order
            .iter()
            .filter(|kind| {
                self.descriptors
                    .get(*kind)
                    .expect("catalog order contains only compiled Deliverable kinds")
                    .is_required()
            })
            .collect()
    }

    pub fn validate_counts(
        &self,
        counts: &BTreeMap<DeliverableKindId, u16>,
    ) -> Result<(), WorkflowPackDeliverableCatalogError> {
        for kind in counts.keys() {
            if !self.descriptors.contains_key(kind) {
                return Err(WorkflowPackDeliverableCatalogError::UnknownSelectedKind {
                    kind: kind.clone(),
                });
            }
        }
        for kind in &self.order {
            let descriptor = self
                .descriptors
                .get(kind)
                .expect("catalog order contains only compiled Deliverable kinds");
            let actual = counts.get(kind).copied().unwrap_or(0);
            if actual < descriptor.minimum {
                return Err(
                    WorkflowPackDeliverableCatalogError::DeliverableCountBelowMinimum {
                        kind: kind.clone(),
                        minimum: descriptor.minimum,
                        actual,
                    },
                );
            }
            if actual > descriptor.maximum {
                return Err(
                    WorkflowPackDeliverableCatalogError::DeliverableCountAboveMaximum {
                        kind: kind.clone(),
                        maximum: descriptor.maximum,
                        actual,
                    },
                );
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn minimum_total(&self) -> u32 {
        self.descriptors
            .values()
            .map(|descriptor| u32::from(descriptor.minimum))
            .sum()
    }

    #[must_use]
    pub fn maximum_total(&self) -> u32 {
        self.descriptors
            .values()
            .map(|descriptor| u32::from(descriptor.maximum))
            .sum()
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

fn validate_deliverable_count(actual: usize) -> Result<(), WorkflowPackDeliverableCatalogError> {
    if actual == 0 || actual > WORKFLOW_PACK_MAX_DELIVERABLE_KINDS {
        return Err(
            WorkflowPackDeliverableCatalogError::DeliverableKindCountInvalid {
                minimum: 1,
                maximum: WORKFLOW_PACK_MAX_DELIVERABLE_KINDS,
                actual,
            },
        );
    }
    Ok(())
}

fn index_resources(
    resources: &[WorkflowPackResource],
) -> Result<BTreeMap<&SafeRelativePath, &WorkflowPackResource>, WorkflowPackDeliverableCatalogError>
{
    let mut indexed = BTreeMap::new();
    for resource in resources {
        if indexed.insert(&resource.path, resource).is_some() {
            return Err(WorkflowPackDeliverableCatalogError::ResourcePathDuplicate {
                path: resource.path.clone(),
            });
        }
    }
    Ok(indexed)
}

fn index_validators<'a>(
    selected_capabilities: &[WorkflowPackCapabilityId],
    definitions: &'a [WorkflowPackValidatorDefinition],
) -> Result<
    BTreeMap<&'a WorkflowPackItemId, &'a WorkflowPackValidatorDefinition>,
    WorkflowPackDeliverableCatalogError,
> {
    let selected = selected_capabilities.iter().collect::<BTreeSet<_>>();
    let mut indexed = BTreeMap::new();
    for validator in definitions {
        if indexed.insert(&validator.id, validator).is_some() {
            return Err(
                WorkflowPackDeliverableCatalogError::ValidatorDefinitionDuplicate {
                    validator: validator.id.clone(),
                },
            );
        }
        if !selected.contains(&validator.capability) {
            return Err(
                WorkflowPackDeliverableCatalogError::ValidatorCapabilityNotSelected {
                    validator: validator.id.clone(),
                    capability: validator.capability.clone(),
                },
            );
        }
    }
    Ok(indexed)
}

fn compile_descriptor(
    pack_id: &WorkflowPackId,
    definition: &WorkflowPackDeliverableDefinition,
    resources: &BTreeMap<&SafeRelativePath, &WorkflowPackResource>,
    selected_renderers: &BTreeSet<&WorkflowPackCapabilityId>,
    validators: &BTreeMap<&WorkflowPackItemId, &WorkflowPackValidatorDefinition>,
) -> Result<WorkflowPackDeliverableDescriptor, WorkflowPackDeliverableCatalogError> {
    let kind = DeliverableKindId::from_parts(pack_id, &definition.id);
    if definition.maximum == 0
        || definition.minimum > definition.maximum
        || definition.maximum > WORKFLOW_PACK_MAX_DELIVERABLE_CARDINALITY
    {
        return Err(WorkflowPackDeliverableCatalogError::CardinalityInvalid {
            kind,
            minimum: definition.minimum,
            maximum: definition.maximum,
            allowed_maximum: WORKFLOW_PACK_MAX_DELIVERABLE_CARDINALITY,
        });
    }
    let template = definition
        .template
        .as_ref()
        .map(|path| compile_template_binding(&kind, path, resources))
        .transpose()?;
    if let Some(renderer) = &definition.renderer
        && !selected_renderers.contains(renderer)
    {
        return Err(WorkflowPackDeliverableCatalogError::RendererNotSelected {
            kind,
            renderer: renderer.clone(),
        });
    }
    let mut compiled_validators = Vec::with_capacity(definition.validators.len());
    let mut unique_validators = BTreeSet::new();
    for validator_id in &definition.validators {
        if !unique_validators.insert(validator_id) {
            return Err(
                WorkflowPackDeliverableCatalogError::ValidatorReferenceDuplicate {
                    kind,
                    validator: validator_id.clone(),
                },
            );
        }
        let validator = validators.get(validator_id).ok_or_else(|| {
            WorkflowPackDeliverableCatalogError::ValidatorReferenceUnknown {
                kind: kind.clone(),
                validator: validator_id.clone(),
            }
        })?;
        compiled_validators.push(WorkflowPackValidatorBinding {
            id: validator.id.clone(),
            capability: validator.capability.clone(),
            parameters: validator.parameters.clone(),
        });
    }
    Ok(WorkflowPackDeliverableDescriptor {
        kind,
        local_id: definition.id.clone(),
        labels: definition.labels.clone(),
        minimum: definition.minimum,
        maximum: definition.maximum,
        template,
        renderer: definition.renderer.clone(),
        validators: compiled_validators,
    })
}

fn compile_template_binding(
    kind: &DeliverableKindId,
    path: &SafeRelativePath,
    resources: &BTreeMap<&SafeRelativePath, &WorkflowPackResource>,
) -> Result<WorkflowPackTemplateBinding, WorkflowPackDeliverableCatalogError> {
    let resource = resources.get(path).ok_or_else(|| {
        WorkflowPackDeliverableCatalogError::TemplateResourceUnknown {
            kind: kind.clone(),
            path: path.clone(),
        }
    })?;
    if resource.kind != WorkflowPackResourceKind::Template {
        return Err(
            WorkflowPackDeliverableCatalogError::TemplateResourceKindInvalid {
                kind: kind.clone(),
                path: path.clone(),
                actual: resource.kind,
            },
        );
    }
    Ok(WorkflowPackTemplateBinding {
        id: resource.id.clone(),
        path: resource.path.clone(),
        version: resource.version.clone(),
        size_bytes: resource.size_bytes,
        sha256: resource.sha256.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowPackDeliverableCatalogError {
    #[error(
        "workflow-pack Deliverable kind count must be between {minimum} and {maximum}; found {actual}"
    )]
    DeliverableKindCountInvalid {
        minimum: usize,
        maximum: usize,
        actual: usize,
    },
    #[error("workflow-pack Deliverable kind is declared more than once: {kind}")]
    DuplicateKind { kind: DeliverableKindId },
    #[error(
        "workflow-pack Deliverable {kind} cardinality {minimum}..{maximum} is invalid; maximum allowed is {allowed_maximum}"
    )]
    CardinalityInvalid {
        kind: DeliverableKindId,
        minimum: u16,
        maximum: u16,
        allowed_maximum: u16,
    },
    #[error("workflow-pack resource path is declared more than once: {path}")]
    ResourcePathDuplicate { path: SafeRelativePath },
    #[error("workflow-pack Deliverable {kind} references unknown template resource {path}")]
    TemplateResourceUnknown {
        kind: DeliverableKindId,
        path: SafeRelativePath,
    },
    #[error("workflow-pack Deliverable {kind} resource {path} is {actual:?}, not a template")]
    TemplateResourceKindInvalid {
        kind: DeliverableKindId,
        path: SafeRelativePath,
        actual: WorkflowPackResourceKind,
    },
    #[error("workflow-pack Deliverable {kind} references unselected renderer {renderer}")]
    RendererNotSelected {
        kind: DeliverableKindId,
        renderer: WorkflowPackCapabilityId,
    },
    #[error("workflow-pack validator definition is declared more than once: {validator}")]
    ValidatorDefinitionDuplicate { validator: WorkflowPackItemId },
    #[error("workflow-pack validator {validator} references unselected capability {capability}")]
    ValidatorCapabilityNotSelected {
        validator: WorkflowPackItemId,
        capability: WorkflowPackCapabilityId,
    },
    #[error("workflow-pack Deliverable {kind} references unknown validator {validator}")]
    ValidatorReferenceUnknown {
        kind: DeliverableKindId,
        validator: WorkflowPackItemId,
    },
    #[error("workflow-pack Deliverable {kind} repeats validator {validator}")]
    ValidatorReferenceDuplicate {
        kind: DeliverableKindId,
        validator: WorkflowPackItemId,
    },
    #[error("Deliverable selection contains a kind outside this Pack catalog: {kind}")]
    UnknownSelectedKind { kind: DeliverableKindId },
    #[error("Deliverable selection for {kind} requires at least {minimum} item(s); found {actual}")]
    DeliverableCountBelowMinimum {
        kind: DeliverableKindId,
        minimum: u16,
        actual: u16,
    },
    #[error("Deliverable selection for {kind} permits at most {maximum} item(s); found {actual}")]
    DeliverableCountAboveMaximum {
        kind: DeliverableKindId,
        maximum: u16,
        actual: u16,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use canisend_contracts::{
        DeliverableKindId, SafeRelativePath, SemanticVersion, Sha256Digest,
        WORKFLOW_PACK_MAX_DELIVERABLE_CARDINALITY, WORKFLOW_PACK_MAX_DELIVERABLE_KINDS,
        WorkflowPackCapabilityId, WorkflowPackDeliverableCatalog,
        WorkflowPackDeliverableDefinition, WorkflowPackId, WorkflowPackItemId,
        WorkflowPackLocaleId, WorkflowPackLocalizedText, WorkflowPackResource,
        WorkflowPackResourceKind, WorkflowPackValidatorDefinition,
    };
    use serde_json::json;

    use super::{WorkflowPackDeliverableCatalogError, WorkflowPackDeliverableCatalogRuntime};

    fn pack(value: &str) -> WorkflowPackId {
        WorkflowPackId::try_new(value).expect("pack ID")
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("item ID")
    }

    fn capability(value: &str) -> WorkflowPackCapabilityId {
        WorkflowPackCapabilityId::try_new(value).expect("capability ID")
    }

    fn path(value: &str) -> SafeRelativePath {
        SafeRelativePath::try_new(value).expect("safe path")
    }

    fn labels(value: &str) -> WorkflowPackLocalizedText {
        WorkflowPackLocalizedText(BTreeMap::from([(
            WorkflowPackLocaleId::try_new("en").expect("locale"),
            value.to_owned(),
        )]))
    }

    fn resource(resource_path: &str, kind: WorkflowPackResourceKind) -> WorkflowPackResource {
        WorkflowPackResource {
            id: item("statement-template"),
            kind,
            path: path(resource_path),
            version: SemanticVersion::try_new("1.0.0").expect("version"),
            size_bytes: 16,
            sha256: Sha256Digest::try_new("a".repeat(64)).expect("digest"),
        }
    }

    fn validator() -> WorkflowPackValidatorDefinition {
        WorkflowPackValidatorDefinition {
            id: item("traceability"),
            capability: capability("canisend.validator.evidence-traceability"),
            parameters: BTreeMap::from([("strict".to_owned(), json!(true))]),
        }
    }

    fn definition(
        id: &str,
        minimum: u16,
        maximum: u16,
        template: Option<&str>,
        renderer: Option<WorkflowPackCapabilityId>,
        validators: &[&str],
    ) -> WorkflowPackDeliverableDefinition {
        WorkflowPackDeliverableDefinition {
            id: item(id),
            labels: labels(id),
            minimum,
            maximum,
            template: template.map(path),
            renderer,
            validators: validators.iter().map(|value| item(value)).collect(),
        }
    }

    fn valid_catalog() -> WorkflowPackDeliverableCatalog {
        WorkflowPackDeliverableCatalog {
            kinds: vec![
                definition("appendix", 0, 2, None, None, &[]),
                definition(
                    "statement",
                    1,
                    1,
                    Some("templates/statement.typ"),
                    Some(capability("canisend.renderer.typst")),
                    &["traceability"],
                ),
            ],
        }
    }

    fn compile(
        pack_id: WorkflowPackId,
        catalog: &WorkflowPackDeliverableCatalog,
    ) -> Result<WorkflowPackDeliverableCatalogRuntime, WorkflowPackDeliverableCatalogError> {
        WorkflowPackDeliverableCatalogRuntime::try_new(
            pack_id,
            catalog,
            &[resource(
                "templates/statement.typ",
                WorkflowPackResourceKind::Template,
            )],
            &[capability("canisend.renderer.typst")],
            &[capability("canisend.validator.evidence-traceability")],
            &[validator()],
        )
    }

    fn kind(pack_id: &WorkflowPackId, local_id: &str) -> DeliverableKindId {
        DeliverableKindId::from_parts(pack_id, &item(local_id))
    }

    #[test]
    fn compiles_ordered_resolved_catalog_and_validates_counts() {
        let pack_id = pack("org.canisend.deliverable-test");
        let catalog = compile(pack_id.clone(), &valid_catalog()).expect("valid catalog");
        assert_eq!(catalog.pack_id(), &pack_id);
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.minimum_total(), 1);
        assert_eq!(catalog.maximum_total(), 3);
        assert_eq!(
            catalog
                .descriptors()
                .iter()
                .map(|descriptor| descriptor.kind().clone())
                .collect::<Vec<_>>(),
            vec![kind(&pack_id, "appendix"), kind(&pack_id, "statement")]
        );
        let statement = catalog
            .descriptor_for_local_id(&item("statement"))
            .expect("statement descriptor");
        assert!(statement.is_required());
        assert!(statement.allows_count(1));
        let template = statement.template().expect("template binding");
        assert_eq!(template.id().as_str(), "statement-template");
        assert_eq!(template.path().as_str(), "templates/statement.typ");
        assert_eq!(template.version().as_str(), "1.0.0");
        assert_eq!(template.size_bytes(), 16);
        assert_eq!(template.sha256().as_str(), "a".repeat(64));
        assert_eq!(
            statement.renderer().expect("renderer").as_str(),
            "canisend.renderer.typst"
        );
        assert_eq!(
            statement.validators()[0].parameters()["strict"],
            json!(true)
        );
        assert_eq!(
            catalog
                .required_kinds()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![kind(&pack_id, "statement")]
        );

        let valid = BTreeMap::from([
            (kind(&pack_id, "statement"), 1),
            (kind(&pack_id, "appendix"), 2),
        ]);
        catalog.validate_counts(&valid).expect("valid counts");
        assert_eq!(
            catalog.validate_counts(&BTreeMap::new()),
            Err(
                WorkflowPackDeliverableCatalogError::DeliverableCountBelowMinimum {
                    kind: kind(&pack_id, "statement"),
                    minimum: 1,
                    actual: 0,
                }
            )
        );
        assert_eq!(
            catalog.validate_counts(&BTreeMap::from([(kind(&pack_id, "statement"), 2)])),
            Err(
                WorkflowPackDeliverableCatalogError::DeliverableCountAboveMaximum {
                    kind: kind(&pack_id, "statement"),
                    maximum: 1,
                    actual: 2,
                }
            )
        );
    }

    #[test]
    fn equal_local_kinds_are_isolated_between_pack_catalogs() {
        let first_pack = pack("org.canisend.first-pack");
        let second_pack = pack("org.canisend.second-pack");
        let first = compile(first_pack.clone(), &valid_catalog()).expect("first catalog");
        let second = compile(second_pack.clone(), &valid_catalog()).expect("second catalog");
        let first_statement = kind(&first_pack, "statement");
        let second_statement = kind(&second_pack, "statement");
        assert!(first.descriptor(&first_statement).is_some());
        assert!(first.descriptor(&second_statement).is_none());
        assert!(second.descriptor(&second_statement).is_some());
        assert_eq!(
            first.validate_counts(&BTreeMap::from([(second_statement.clone(), 1)])),
            Err(WorkflowPackDeliverableCatalogError::UnknownSelectedKind {
                kind: second_statement,
            })
        );
    }

    #[test]
    fn count_duplicate_and_cardinality_errors_fail_closed() {
        let pack_id = pack("org.canisend.deliverable-test");
        let empty = WorkflowPackDeliverableCatalog { kinds: Vec::new() };
        assert!(matches!(
            compile(pack_id.clone(), &empty),
            Err(WorkflowPackDeliverableCatalogError::DeliverableKindCountInvalid { actual: 0, .. })
        ));

        let oversized = WorkflowPackDeliverableCatalog {
            kinds: (0..=WORKFLOW_PACK_MAX_DELIVERABLE_KINDS)
                .map(|index| definition(&format!("kind-{index}"), 0, 1, None, None, &[]))
                .collect(),
        };
        assert!(matches!(
            compile(pack_id.clone(), &oversized),
            Err(WorkflowPackDeliverableCatalogError::DeliverableKindCountInvalid {
                actual,
                ..
            }) if actual == WORKFLOW_PACK_MAX_DELIVERABLE_KINDS + 1
        ));

        let duplicate = WorkflowPackDeliverableCatalog {
            kinds: vec![
                definition("statement", 0, 1, None, None, &[]),
                definition("statement", 0, 1, None, None, &[]),
            ],
        };
        assert!(matches!(
            compile(pack_id.clone(), &duplicate),
            Err(WorkflowPackDeliverableCatalogError::DuplicateKind { .. })
        ));

        for (minimum, maximum) in [
            (1, 0),
            (2, 1),
            (0, WORKFLOW_PACK_MAX_DELIVERABLE_CARDINALITY + 1),
        ] {
            let invalid = WorkflowPackDeliverableCatalog {
                kinds: vec![definition("statement", minimum, maximum, None, None, &[])],
            };
            assert!(matches!(
                compile(pack_id.clone(), &invalid),
                Err(WorkflowPackDeliverableCatalogError::CardinalityInvalid { .. })
            ));
        }
    }

    #[test]
    fn template_renderer_and_validator_references_fail_closed() {
        let pack_id = pack("org.canisend.deliverable-test");
        let missing_template = WorkflowPackDeliverableCatalog {
            kinds: vec![definition(
                "statement",
                1,
                1,
                Some("templates/missing.typ"),
                None,
                &[],
            )],
        };
        assert!(matches!(
            compile(pack_id.clone(), &missing_template),
            Err(WorkflowPackDeliverableCatalogError::TemplateResourceUnknown { .. })
        ));

        let wrong_kind = WorkflowPackDeliverableCatalogRuntime::try_new(
            pack_id.clone(),
            &valid_catalog(),
            &[resource(
                "templates/statement.typ",
                WorkflowPackResourceKind::Example,
            )],
            &[capability("canisend.renderer.typst")],
            &[capability("canisend.validator.evidence-traceability")],
            &[validator()],
        );
        assert!(matches!(
            wrong_kind,
            Err(WorkflowPackDeliverableCatalogError::TemplateResourceKindInvalid { .. })
        ));

        let duplicate_resources = [
            resource(
                "templates/statement.typ",
                WorkflowPackResourceKind::Template,
            ),
            resource(
                "templates/statement.typ",
                WorkflowPackResourceKind::Template,
            ),
        ];
        let duplicate_resource = WorkflowPackDeliverableCatalogRuntime::try_new(
            pack_id.clone(),
            &valid_catalog(),
            &duplicate_resources,
            &[capability("canisend.renderer.typst")],
            &[capability("canisend.validator.evidence-traceability")],
            &[validator()],
        );
        assert!(matches!(
            duplicate_resource,
            Err(WorkflowPackDeliverableCatalogError::ResourcePathDuplicate { .. })
        ));

        let unselected_renderer = WorkflowPackDeliverableCatalogRuntime::try_new(
            pack_id.clone(),
            &valid_catalog(),
            &[resource(
                "templates/statement.typ",
                WorkflowPackResourceKind::Template,
            )],
            &[],
            &[capability("canisend.validator.evidence-traceability")],
            &[validator()],
        );
        assert!(matches!(
            unselected_renderer,
            Err(WorkflowPackDeliverableCatalogError::RendererNotSelected { .. })
        ));

        let unknown_validator = WorkflowPackDeliverableCatalog {
            kinds: vec![definition("statement", 1, 1, None, None, &["missing"])],
        };
        assert!(matches!(
            compile(pack_id.clone(), &unknown_validator),
            Err(WorkflowPackDeliverableCatalogError::ValidatorReferenceUnknown { .. })
        ));

        let duplicate_validator = WorkflowPackDeliverableCatalog {
            kinds: vec![definition(
                "statement",
                1,
                1,
                None,
                None,
                &["traceability", "traceability"],
            )],
        };
        assert!(matches!(
            compile(pack_id.clone(), &duplicate_validator),
            Err(WorkflowPackDeliverableCatalogError::ValidatorReferenceDuplicate { .. })
        ));

        let definitions = [validator(), validator()];
        let duplicate_definition = WorkflowPackDeliverableCatalogRuntime::try_new(
            pack_id,
            &valid_catalog(),
            &[resource(
                "templates/statement.typ",
                WorkflowPackResourceKind::Template,
            )],
            &[capability("canisend.renderer.typst")],
            &[capability("canisend.validator.evidence-traceability")],
            &definitions,
        );
        assert!(matches!(
            duplicate_definition,
            Err(WorkflowPackDeliverableCatalogError::ValidatorDefinitionDuplicate { .. })
        ));

        let unselected_validator_capability = WorkflowPackDeliverableCatalogRuntime::try_new(
            pack("org.canisend.deliverable-test"),
            &valid_catalog(),
            &[resource(
                "templates/statement.typ",
                WorkflowPackResourceKind::Template,
            )],
            &[capability("canisend.renderer.typst")],
            &[],
            &[validator()],
        );
        assert!(matches!(
            unselected_validator_capability,
            Err(WorkflowPackDeliverableCatalogError::ValidatorCapabilityNotSelected { .. })
        ));
    }
}
