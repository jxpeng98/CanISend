use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use canisend_contracts::{
    CandidateValidationError, SafeRelativePath, SemanticVersion, Sha256Digest,
    WorkflowPackCapabilityId, WorkflowPackId, WorkflowPackItemId, WorkflowPackManifest,
    WorkflowPackResource, WorkflowPackResourceKind, validate_workflow_pack_manifest,
};
use semver::{Version, VersionReq};
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

const WORKFLOW_PACK_BUNDLE_DIGEST_DOMAIN: &[u8] = b"canisend.workflow-pack-bundle/v1\0";
const WORKFLOW_PACK_SNAPSHOT_FORMAT: &str = "canisend.workflow-pack-snapshot/v1";
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPackRuntime {
    kernel: Version,
    agent: Version,
    workspace: Version,
}

impl WorkflowPackRuntime {
    #[must_use]
    pub const fn new(kernel: Version, agent: Version, workspace: Version) -> Self {
        Self {
            kernel,
            agent,
            workspace,
        }
    }

    pub fn parse(
        kernel: &str,
        agent: &str,
        workspace: &str,
    ) -> Result<Self, WorkflowPackBundleError> {
        Ok(Self::new(
            parse_runtime_version("kernel", kernel)?,
            parse_runtime_version("agent", agent)?,
            parse_runtime_version("workspace", workspace)?,
        ))
    }

    #[must_use]
    pub const fn kernel(&self) -> &Version {
        &self.kernel
    }

    #[must_use]
    pub const fn agent(&self) -> &Version {
        &self.agent
    }

    #[must_use]
    pub const fn workspace(&self) -> &Version {
        &self.workspace
    }
}

fn parse_runtime_version(
    surface: &'static str,
    value: &str,
) -> Result<Version, WorkflowPackBundleError> {
    Version::parse(value).map_err(|error| WorkflowPackBundleError::RuntimeVersionInvalid {
        surface,
        value: value.to_owned(),
        message: error.to_string(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackCapabilityKind {
    IntakeAdapter,
    Renderer,
    Validator,
}

impl WorkflowPackCapabilityKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IntakeAdapter => "intake-adapter",
            Self::Renderer => "renderer",
            Self::Validator => "validator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowPackCapabilityRegistry {
    intake_adapters: BTreeSet<WorkflowPackCapabilityId>,
    renderers: BTreeSet<WorkflowPackCapabilityId>,
    validators: BTreeSet<WorkflowPackCapabilityId>,
}

impl WorkflowPackCapabilityRegistry {
    #[must_use]
    pub fn built_in() -> Self {
        Self {
            intake_adapters: capability_set(&[
                "canisend.intake.local-file",
                "canisend.intake.user-url",
                "canisend.intake.text-pdf",
                "canisend.discovery.rss-atom",
                "canisend.discovery.jobs-ac-uk",
                "canisend.discovery.greenhouse",
                "canisend.discovery.lever",
            ]),
            renderers: capability_set(&["canisend.renderer.typst"]),
            validators: capability_set(&[
                "canisend.validator.evidence-traceability",
                "canisend.validator.unsupported-claims",
                "canisend.validator.placeholder-free",
                "canisend.validator.citation-integrity",
                "canisend.validator.review-complete",
            ]),
        }
    }

    #[must_use]
    pub fn from_sets(
        intake_adapters: impl IntoIterator<Item = WorkflowPackCapabilityId>,
        renderers: impl IntoIterator<Item = WorkflowPackCapabilityId>,
        validators: impl IntoIterator<Item = WorkflowPackCapabilityId>,
    ) -> Self {
        Self {
            intake_adapters: intake_adapters.into_iter().collect(),
            renderers: renderers.into_iter().collect(),
            validators: validators.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn supports(
        &self,
        kind: WorkflowPackCapabilityKind,
        id: &WorkflowPackCapabilityId,
    ) -> bool {
        match kind {
            WorkflowPackCapabilityKind::IntakeAdapter => self.intake_adapters.contains(id),
            WorkflowPackCapabilityKind::Renderer => self.renderers.contains(id),
            WorkflowPackCapabilityKind::Validator => self.validators.contains(id),
        }
    }

    #[must_use]
    pub fn intake_adapters(&self) -> &BTreeSet<WorkflowPackCapabilityId> {
        &self.intake_adapters
    }

    #[must_use]
    pub fn renderers(&self) -> &BTreeSet<WorkflowPackCapabilityId> {
        &self.renderers
    }

    #[must_use]
    pub fn validators(&self) -> &BTreeSet<WorkflowPackCapabilityId> {
        &self.validators
    }
}

fn capability_set(values: &[&str]) -> BTreeSet<WorkflowPackCapabilityId> {
    values
        .iter()
        .map(|value| {
            WorkflowPackCapabilityId::try_new(*value).expect("built-in capability ID is valid")
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackOrigin {
    BuiltIn,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackSnapshotResource {
    id: WorkflowPackItemId,
    kind: WorkflowPackResourceKind,
    path: SafeRelativePath,
    version: SemanticVersion,
    size_bytes: u64,
    sha256: Sha256Digest,
}

impl WorkflowPackSnapshotResource {
    #[must_use]
    pub const fn id(&self) -> &WorkflowPackItemId {
        &self.id
    }

    #[must_use]
    pub const fn kind(&self) -> WorkflowPackResourceKind {
        self.kind
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackSnapshot {
    format: String,
    id: WorkflowPackId,
    version: SemanticVersion,
    origin: WorkflowPackOrigin,
    content_digest: Sha256Digest,
    manifest_sha256: Sha256Digest,
    resources: Vec<WorkflowPackSnapshotResource>,
}

impl WorkflowPackSnapshot {
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    #[must_use]
    pub const fn id(&self) -> &WorkflowPackId {
        &self.id
    }

    #[must_use]
    pub const fn version(&self) -> &SemanticVersion {
        &self.version
    }

    #[must_use]
    pub const fn origin(&self) -> &WorkflowPackOrigin {
        &self.origin
    }

    #[must_use]
    pub const fn content_digest(&self) -> &Sha256Digest {
        &self.content_digest
    }

    #[must_use]
    pub const fn manifest_sha256(&self) -> &Sha256Digest {
        &self.manifest_sha256
    }

    #[must_use]
    pub fn resources(&self) -> &[WorkflowPackSnapshotResource] {
        &self.resources
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedWorkflowPackBundle {
    manifest: WorkflowPackManifest,
    resources: BTreeMap<SafeRelativePath, Vec<u8>>,
    snapshot: WorkflowPackSnapshot,
}

impl VerifiedWorkflowPackBundle {
    pub fn verify(
        manifest_value: &Value,
        resources: BTreeMap<SafeRelativePath, Vec<u8>>,
        origin: WorkflowPackOrigin,
        runtime: &WorkflowPackRuntime,
        capabilities: &WorkflowPackCapabilityRegistry,
    ) -> Result<Self, WorkflowPackBundleError> {
        let manifest = validate_workflow_pack_manifest(manifest_value)?;
        validate_runtime_compatibility(&manifest, runtime)?;
        validate_registered_capabilities(&manifest, capabilities)?;
        validate_resource_bytes(&manifest, &resources)?;
        let actual_content_digest = calculate_validated_content_digest(&manifest, &resources);
        if actual_content_digest != manifest.content_digest {
            return Err(WorkflowPackBundleError::ContentDigestMismatch {
                expected: manifest.content_digest.clone(),
                actual: actual_content_digest,
            });
        }
        let manifest_bytes = canonical_manifest_bytes(&manifest, false);
        let manifest_sha256 = digest_bytes(&manifest_bytes);
        let snapshot = WorkflowPackSnapshot {
            format: WORKFLOW_PACK_SNAPSHOT_FORMAT.to_owned(),
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            origin,
            content_digest: manifest.content_digest.clone(),
            manifest_sha256,
            resources: snapshot_resources(&manifest),
        };
        Ok(Self {
            manifest,
            resources,
            snapshot,
        })
    }

    #[must_use]
    pub const fn manifest(&self) -> &WorkflowPackManifest {
        &self.manifest
    }

    #[must_use]
    pub const fn resources(&self) -> &BTreeMap<SafeRelativePath, Vec<u8>> {
        &self.resources
    }

    #[must_use]
    pub const fn snapshot(&self) -> &WorkflowPackSnapshot {
        &self.snapshot
    }
}

fn snapshot_resources(manifest: &WorkflowPackManifest) -> Vec<WorkflowPackSnapshotResource> {
    let mut resources = manifest
        .resources
        .iter()
        .map(|resource| WorkflowPackSnapshotResource {
            id: resource.id.clone(),
            kind: resource.kind,
            path: resource.path.clone(),
            version: resource.version.clone(),
            size_bytes: resource.size_bytes,
            sha256: resource.sha256.clone(),
        })
        .collect::<Vec<_>>();
    resources.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    resources
}

pub fn calculate_workflow_pack_content_digest(
    manifest: &WorkflowPackManifest,
    resources: &BTreeMap<SafeRelativePath, Vec<u8>>,
) -> Result<Sha256Digest, WorkflowPackBundleError> {
    let value = serde_json::to_value(manifest).expect("typed workflow-pack manifest serializes");
    let validated = validate_workflow_pack_manifest(&value)?;
    validate_resource_bytes(&validated, resources)?;
    Ok(calculate_validated_content_digest(&validated, resources))
}

fn calculate_validated_content_digest(
    manifest: &WorkflowPackManifest,
    resources: &BTreeMap<SafeRelativePath, Vec<u8>>,
) -> Sha256Digest {
    let normalized_manifest = canonical_manifest_bytes(manifest, true);
    let mut hasher = Sha256::new();
    hasher.update(WORKFLOW_PACK_BUNDLE_DIGEST_DOMAIN);
    hash_segment(&mut hasher, b"manifest");
    hash_segment(&mut hasher, &normalized_manifest);
    for resource in sorted_resources(&manifest.resources) {
        let bytes = resources
            .get(&resource.path)
            .expect("validated resource set contains every declared path");
        hash_segment(&mut hasher, b"resource-path");
        hash_segment(&mut hasher, resource.path.as_str().as_bytes());
        hash_segment(&mut hasher, b"resource-bytes");
        hash_segment(&mut hasher, bytes);
    }
    Sha256Digest::try_new(hex::encode(hasher.finalize())).expect("SHA-256 digest is canonical")
}

fn canonical_manifest_bytes(manifest: &WorkflowPackManifest, zero_content_digest: bool) -> Vec<u8> {
    let mut value =
        serde_json::to_value(manifest).expect("typed workflow-pack manifest serializes");
    if zero_content_digest {
        value["content_digest"] = Value::String(ZERO_SHA256.to_owned());
    }
    serde_json::to_vec(&sort_json(value)).expect("canonical workflow-pack manifest serializes")
}

fn sort_json(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries = object.into_iter().collect::<Vec<_>>();
            entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, sort_json(value)))
                    .collect::<Map<_, _>>(),
            )
        }
        Value::Array(values) => Value::Array(values.into_iter().map(sort_json).collect()),
        scalar => scalar,
    }
}

fn hash_segment(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn digest_bytes(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes))).expect("SHA-256 digest is canonical")
}

fn sorted_resources(resources: &[WorkflowPackResource]) -> Vec<&WorkflowPackResource> {
    let mut sorted = resources.iter().collect::<Vec<_>>();
    sorted.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    sorted
}

fn validate_resource_bytes(
    manifest: &WorkflowPackManifest,
    resources: &BTreeMap<SafeRelativePath, Vec<u8>>,
) -> Result<(), WorkflowPackBundleError> {
    let expected = manifest
        .resources
        .iter()
        .map(|resource| resource.path.clone())
        .collect::<BTreeSet<_>>();
    let actual = resources.keys().cloned().collect::<BTreeSet<_>>();
    if expected != actual {
        return Err(WorkflowPackBundleError::ResourceSetMismatch {
            missing: expected.difference(&actual).cloned().collect(),
            undeclared: actual.difference(&expected).cloned().collect(),
        });
    }
    for resource in &manifest.resources {
        let bytes = resources
            .get(&resource.path)
            .expect("resource set equality was established");
        let actual_size = bytes.len() as u64;
        if actual_size != resource.size_bytes {
            return Err(WorkflowPackBundleError::ResourceSizeMismatch {
                path: resource.path.clone(),
                expected: resource.size_bytes,
                actual: actual_size,
            });
        }
        let actual_digest = digest_bytes(bytes);
        if actual_digest != resource.sha256 {
            return Err(WorkflowPackBundleError::ResourceDigestMismatch {
                path: resource.path.clone(),
                expected: resource.sha256.clone(),
                actual: actual_digest,
            });
        }
    }
    Ok(())
}

fn validate_runtime_compatibility(
    manifest: &WorkflowPackManifest,
    runtime: &WorkflowPackRuntime,
) -> Result<(), WorkflowPackBundleError> {
    for (surface, requirement, actual) in [
        ("kernel", &manifest.compatibility.kernel, runtime.kernel()),
        ("agent", &manifest.compatibility.agent, runtime.agent()),
        (
            "workspace",
            &manifest.compatibility.workspace,
            runtime.workspace(),
        ),
    ] {
        let parsed = VersionReq::parse(requirement)
            .expect("semantic manifest validation accepted compatibility requirement");
        if !parsed.matches(actual) {
            return Err(WorkflowPackBundleError::IncompatibleRuntime {
                surface,
                required: requirement.clone(),
                actual: actual.clone(),
            });
        }
    }
    Ok(())
}

fn validate_registered_capabilities(
    manifest: &WorkflowPackManifest,
    registry: &WorkflowPackCapabilityRegistry,
) -> Result<(), WorkflowPackBundleError> {
    for (kind, selected) in [
        (
            WorkflowPackCapabilityKind::IntakeAdapter,
            &manifest.capabilities.intake_adapters,
        ),
        (
            WorkflowPackCapabilityKind::Renderer,
            &manifest.capabilities.renderers,
        ),
        (
            WorkflowPackCapabilityKind::Validator,
            &manifest.capabilities.validators,
        ),
    ] {
        for capability in selected {
            if !registry.supports(kind, capability) {
                return Err(WorkflowPackBundleError::CapabilityUnavailable {
                    kind,
                    id: capability.clone(),
                });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowPackRegistryInsert {
    Inserted,
    AlreadyPresent,
}

#[derive(Debug, Default)]
pub struct WorkflowPackRegistry {
    bundles: BTreeMap<(WorkflowPackId, SemanticVersion), VerifiedWorkflowPackBundle>,
}

impl WorkflowPackRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bundles: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        bundle: VerifiedWorkflowPackBundle,
    ) -> Result<WorkflowPackRegistryInsert, WorkflowPackBundleError> {
        let key = (bundle.snapshot.id.clone(), bundle.snapshot.version.clone());
        if let Some(existing) = self.bundles.get(&key) {
            if existing.snapshot.content_digest != bundle.snapshot.content_digest {
                return Err(WorkflowPackBundleError::VersionSubstitution {
                    id: key.0,
                    version: key.1,
                    existing: existing.snapshot.content_digest.clone(),
                    requested: bundle.snapshot.content_digest,
                });
            }
            // Origin describes how the already-verified bytes entered this process; it is not
            // part of the bundle identity. Re-registering identical bytes through another origin
            // is idempotent and preserves the first snapshot's provenance.
            if existing.manifest != bundle.manifest || existing.resources != bundle.resources {
                return Err(WorkflowPackBundleError::DigestCollision {
                    id: key.0,
                    version: key.1,
                    digest: existing.snapshot.content_digest.clone(),
                });
            }
            return Ok(WorkflowPackRegistryInsert::AlreadyPresent);
        }
        self.bundles.insert(key, bundle);
        Ok(WorkflowPackRegistryInsert::Inserted)
    }

    pub fn resolve_exact(
        &self,
        id: &WorkflowPackId,
        version: &SemanticVersion,
        content_digest: &Sha256Digest,
    ) -> Result<&VerifiedWorkflowPackBundle, WorkflowPackBundleError> {
        let key = (id.clone(), version.clone());
        let bundle =
            self.bundles
                .get(&key)
                .ok_or_else(|| WorkflowPackBundleError::PackVersionNotFound {
                    id: id.clone(),
                    version: version.clone(),
                })?;
        if &bundle.snapshot.content_digest != content_digest {
            return Err(WorkflowPackBundleError::SnapshotBindingMismatch {
                id: id.clone(),
                version: version.clone(),
                expected: content_digest.clone(),
                actual: bundle.snapshot.content_digest.clone(),
            });
        }
        Ok(bundle)
    }

    #[must_use]
    pub fn contains_exact(
        &self,
        id: &WorkflowPackId,
        version: &SemanticVersion,
        content_digest: &Sha256Digest,
    ) -> bool {
        self.resolve_exact(id, version, content_digest).is_ok()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bundles.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bundles.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowPackBundleError {
    #[error("workflow-pack manifest is invalid: {0}")]
    Manifest(#[from] CandidateValidationError),
    #[error("workflow-pack runtime {surface} version `{value}` is invalid: {message}")]
    RuntimeVersionInvalid {
        surface: &'static str,
        value: String,
        message: String,
    },
    #[error("workflow-pack requires {surface} `{required}` but the current runtime is `{actual}`")]
    IncompatibleRuntime {
        surface: &'static str,
        required: String,
        actual: Version,
    },
    #[error("workflow-pack {kind} capability is not registered: {id}")]
    CapabilityUnavailable {
        kind: WorkflowPackCapabilityKind,
        id: WorkflowPackCapabilityId,
    },
    #[error("workflow-pack resource set differs; missing={missing:?}, undeclared={undeclared:?}")]
    ResourceSetMismatch {
        missing: Vec<SafeRelativePath>,
        undeclared: Vec<SafeRelativePath>,
    },
    #[error("workflow-pack resource size mismatch at {path}: expected {expected}, found {actual}")]
    ResourceSizeMismatch {
        path: SafeRelativePath,
        expected: u64,
        actual: u64,
    },
    #[error(
        "workflow-pack resource digest mismatch at {path}: expected {expected}, found {actual}"
    )]
    ResourceDigestMismatch {
        path: SafeRelativePath,
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("workflow-pack content digest mismatch: expected {expected}, found {actual}")]
    ContentDigestMismatch {
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error(
        "workflow-pack version substitution rejected for {id} {version}: existing {existing}, requested {requested}"
    )]
    VersionSubstitution {
        id: WorkflowPackId,
        version: SemanticVersion,
        existing: Sha256Digest,
        requested: Sha256Digest,
    },
    #[error(
        "workflow-pack digest collision rejected for {id} {version} at content digest {digest}"
    )]
    DigestCollision {
        id: WorkflowPackId,
        version: SemanticVersion,
        digest: Sha256Digest,
    },
    #[error("workflow-pack version is not registered: {id} {version}")]
    PackVersionNotFound {
        id: WorkflowPackId,
        version: SemanticVersion,
    },
    #[error(
        "workflow-pack snapshot binding mismatch for {id} {version}: expected {expected}, found {actual}"
    )]
    SnapshotBindingMismatch {
        id: WorkflowPackId,
        version: SemanticVersion,
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
}

impl fmt::Display for WorkflowPackCapabilityKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use canisend_contracts::{
        ExecutionMode, SafeRelativePath, SemanticVersion, Sha256Digest,
        WorkflowPackApplicationDefinition, WorkflowPackCapabilities,
        WorkflowPackCategoryDefinition, WorkflowPackCompatibility, WorkflowPackDeliverableCatalog,
        WorkflowPackDeliverableDefinition, WorkflowPackFieldDefinition, WorkflowPackFieldType,
        WorkflowPackFormat, WorkflowPackId, WorkflowPackItemId, WorkflowPackLocaleId,
        WorkflowPackLocalizedText, WorkflowPackManifest, WorkflowPackPublisher,
        WorkflowPackPublisherId, WorkflowPackResource, WorkflowPackResourceKind,
        WorkflowPackStageDefinition, WorkflowPackStageOutput, WorkflowPackTaxonomy,
        WorkflowPackValidationPolicy, WorkflowPackValidatorDefinition, WorkflowPackVocabulary,
        WorkflowPackWorkflowDefinition,
    };
    use serde_json::Value;
    use sha2::{Digest, Sha256};

    use crate::{
        WorkflowPackByteLoader, WorkflowPackByteLoaderError, WorkflowPackDeliverableCatalogRuntime,
        WorkflowPackExecutablePolicy, WorkflowPackHostLocale, WorkflowPackInstallationStatus,
        WorkflowPackLocaleMatch, WorkflowPackLocalizationError, WorkflowPackLocalizationRuntime,
        WorkflowPackPublisherAuthentication, WorkflowPackSelectionBindingMismatch,
        WorkflowPackSignatureStatus, WorkflowPackStageGraph, WorkflowPackTextMatch,
        WorkflowPackTrustCheckStatus, WorkflowPackTrustStatus,
    };

    use super::{
        VerifiedWorkflowPackBundle, WorkflowPackBundleError, WorkflowPackCapabilityRegistry,
        WorkflowPackOrigin, WorkflowPackRegistry, WorkflowPackRegistryInsert, WorkflowPackRuntime,
        calculate_workflow_pack_content_digest,
    };

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("item ID")
    }

    fn locale(value: &str) -> WorkflowPackLocaleId {
        WorkflowPackLocaleId::try_new(value).expect("locale ID")
    }

    fn labels(value: &str) -> WorkflowPackLocalizedText {
        WorkflowPackLocalizedText(BTreeMap::from([(locale("en"), value.to_owned())]))
    }

    fn field(id: &str) -> WorkflowPackFieldDefinition {
        WorkflowPackFieldDefinition {
            id: item(id),
            labels: labels(id),
            field_type: WorkflowPackFieldType::ShortText,
            required: true,
            options: Vec::new(),
        }
    }

    fn path(value: &str) -> SafeRelativePath {
        SafeRelativePath::try_new(value).expect("safe resource path")
    }

    fn digest(bytes: &[u8]) -> Sha256Digest {
        Sha256Digest::try_new(hex::encode(Sha256::digest(bytes))).expect("digest")
    }

    fn manifest_and_resources(
        version: &str,
        template_bytes: &[u8],
    ) -> (WorkflowPackManifest, BTreeMap<SafeRelativePath, Vec<u8>>) {
        let template_path = path("templates/statement.typ");
        let validator_capability = canisend_contracts::WorkflowPackCapabilityId::try_new(
            "canisend.validator.evidence-traceability",
        )
        .expect("validator capability");
        let renderer =
            canisend_contracts::WorkflowPackCapabilityId::try_new("canisend.renderer.typst")
                .expect("renderer");
        let resources = BTreeMap::from([(template_path.clone(), template_bytes.to_vec())]);
        let mut manifest = WorkflowPackManifest {
            format: WorkflowPackFormat::V1,
            id: WorkflowPackId::try_new("org.canisend.registry-test").expect("pack ID"),
            version: SemanticVersion::try_new(version).expect("version"),
            schema_version: SemanticVersion::try_new("1.0.0").expect("schema version"),
            publisher: WorkflowPackPublisher {
                id: WorkflowPackPublisherId::try_new("org.canisend").expect("publisher ID"),
                name: "CanISend".to_owned(),
                homepage: None,
            },
            compatibility: WorkflowPackCompatibility {
                kernel: ">=1.0.0-alpha.5, <2.0.0".to_owned(),
                agent: ">=3.0.0-alpha.1, <4.0.0".to_owned(),
                workspace: ">=3.0.0-alpha.1, <4.0.0".to_owned(),
            },
            default_locale: locale("en"),
            locales: BTreeMap::from([(
                locale("en"),
                WorkflowPackVocabulary {
                    application_singular: "Application".to_owned(),
                    application_plural: "Applications".to_owned(),
                    opportunity_singular: "Opportunity".to_owned(),
                    opportunity_plural: "Opportunities".to_owned(),
                    requirement_plural: "Requirements".to_owned(),
                    evidence_plural: "Evidence".to_owned(),
                    deliverable_plural: "Deliverables".to_owned(),
                },
            )]),
            application: WorkflowPackApplicationDefinition {
                opportunity_fields: vec![field("title")],
                application_fields: Vec::new(),
            },
            workflow: WorkflowPackWorkflowDefinition {
                stages: vec![
                    WorkflowPackStageDefinition {
                        id: item("intake"),
                        labels: labels("Intake"),
                        depends_on: Vec::new(),
                        output: WorkflowPackStageOutput::Sources,
                        execution_modes: vec![ExecutionMode::ManualImport],
                    },
                    WorkflowPackStageDefinition {
                        id: item("export"),
                        labels: labels("Export"),
                        depends_on: vec![item("intake")],
                        output: WorkflowPackStageOutput::Render,
                        execution_modes: vec![ExecutionMode::Deterministic],
                    },
                ],
                terminal_stage: item("export"),
            },
            requirements: WorkflowPackTaxonomy {
                categories: vec![WorkflowPackCategoryDefinition {
                    id: item("general"),
                    labels: labels("General"),
                    fields: Vec::new(),
                }],
            },
            evidence: WorkflowPackTaxonomy {
                categories: vec![WorkflowPackCategoryDefinition {
                    id: item("experience"),
                    labels: labels("Experience"),
                    fields: Vec::new(),
                }],
            },
            deliverables: WorkflowPackDeliverableCatalog {
                kinds: vec![WorkflowPackDeliverableDefinition {
                    id: item("statement"),
                    labels: labels("Statement"),
                    minimum: 1,
                    maximum: 1,
                    template: Some(template_path.clone()),
                    renderer: Some(renderer.clone()),
                    validators: vec![item("traceability")],
                }],
            },
            capabilities: WorkflowPackCapabilities {
                intake_adapters: Vec::new(),
                renderers: vec![renderer],
                validators: vec![validator_capability.clone()],
            },
            validation: WorkflowPackValidationPolicy {
                definitions: vec![WorkflowPackValidatorDefinition {
                    id: item("traceability"),
                    capability: validator_capability,
                    parameters: BTreeMap::new(),
                }],
                readiness: vec![item("traceability")],
            },
            resources: vec![WorkflowPackResource {
                id: item("statement-template"),
                kind: WorkflowPackResourceKind::Template,
                path: template_path,
                version: SemanticVersion::try_new("1.0.0").expect("resource version"),
                size_bytes: template_bytes.len() as u64,
                sha256: digest(template_bytes),
            }],
            migrations: Vec::new(),
            content_digest: Sha256Digest::try_new("0".repeat(64)).expect("placeholder digest"),
        };
        manifest.content_digest =
            calculate_workflow_pack_content_digest(&manifest, &resources).expect("content digest");
        (manifest, resources)
    }

    fn runtime() -> WorkflowPackRuntime {
        WorkflowPackRuntime::parse("1.0.0-alpha.5", "3.0.0-alpha.1", "3.0.0-alpha.1")
            .expect("runtime")
    }

    fn verify(
        manifest: &WorkflowPackManifest,
        resources: BTreeMap<SafeRelativePath, Vec<u8>>,
    ) -> Result<VerifiedWorkflowPackBundle, WorkflowPackBundleError> {
        let value = serde_json::to_value(manifest).expect("manifest value");
        VerifiedWorkflowPackBundle::verify(
            &value,
            resources,
            WorkflowPackOrigin::External,
            &runtime(),
            &WorkflowPackCapabilityRegistry::built_in(),
        )
    }

    #[test]
    fn verified_bundle_binds_manifest_resources_runtime_and_snapshot() {
        let (manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        let manifest_bytes = serde_json::to_vec(&manifest).expect("manifest bytes");
        let candidate = WorkflowPackByteLoader::verify(
            &manifest_bytes,
            resources.clone(),
            WorkflowPackOrigin::External,
            &runtime(),
            &WorkflowPackCapabilityRegistry::built_in(),
        )
        .expect("verified byte candidate");
        let report = candidate.trust_report();
        assert_eq!(report.status(), WorkflowPackTrustStatus::VerifiedDataOnly);
        assert_eq!(
            report.publisher_authentication(),
            WorkflowPackPublisherAuthentication::DeclaredOnly
        );
        assert_eq!(
            report.signature_status(),
            WorkflowPackSignatureStatus::NotSpecifiedByV1
        );
        assert_eq!(
            report.installation_status(),
            WorkflowPackInstallationStatus::Disabled
        );
        assert_eq!(
            report.executable_policy(),
            WorkflowPackExecutablePolicy::DataOnlyNoExecutionAuthority
        );
        assert_eq!(report.resource_count(), 1);
        assert_eq!(report.resource_bytes(), 11);
        assert_eq!(report.capability_references().len(), 2);
        assert!(
            report
                .checks()
                .iter()
                .all(|check| check.status() == WorkflowPackTrustCheckStatus::Passed)
        );
        assert!(!report.contains_resource_bodies());
        let report_json = serde_json::to_string(report).expect("trust report JSON");
        assert!(!report_json.contains("template-v1"));

        let first = verify(&manifest, resources.clone()).expect("verified bundle");
        let second = verify(&manifest, resources).expect("repeat verified bundle");
        assert_eq!(first, second);
        assert_eq!(
            first.snapshot().format(),
            "canisend.workflow-pack-snapshot/v1"
        );
        assert_eq!(first.snapshot().id(), &manifest.id);
        assert_eq!(first.snapshot().version(), &manifest.version);
        assert_eq!(first.snapshot().content_digest(), &manifest.content_digest);
        assert_eq!(first.snapshot().resources().len(), 1);
        let graph = WorkflowPackStageGraph::from_verified_bundle(&first)
            .expect("verified manifest compiles into a stage graph");
        assert_eq!(
            graph.terminal_stage().as_str(),
            "org.canisend.registry-test:export"
        );
        assert_eq!(graph.topological_order().len(), 2);
        let deliverables = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(&first)
            .expect("verified manifest compiles into a Deliverable catalog");
        assert_eq!(deliverables.len(), 1);
        assert_eq!(
            deliverables.descriptors()[0].kind().as_str(),
            "org.canisend.registry-test:statement"
        );
        let snapshot_json = serde_json::to_value(first.snapshot()).expect("snapshot JSON");
        assert_eq!(snapshot_json["origin"], "external");
        assert_eq!(
            snapshot_json["resources"][0]["path"],
            "templates/statement.typ"
        );
    }

    #[test]
    fn localization_resolves_host_locale_fallback_and_restart_deterministically() {
        let (mut manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        let simplified_chinese = locale("zh-Hans");
        manifest.locales.insert(
            simplified_chinese.clone(),
            WorkflowPackVocabulary {
                application_singular: "申请".to_owned(),
                application_plural: "申请".to_owned(),
                opportunity_singular: "机会".to_owned(),
                opportunity_plural: "机会".to_owned(),
                requirement_plural: "要求".to_owned(),
                evidence_plural: "证据".to_owned(),
                deliverable_plural: "交付材料".to_owned(),
            },
        );
        manifest.workflow.stages[0]
            .labels
            .0
            .insert(simplified_chinese, "导入".to_owned());
        manifest.content_digest = calculate_workflow_pack_content_digest(&manifest, &resources)
            .expect("localized digest");
        let bundle = verify(&manifest, resources).expect("localized verified bundle");
        let localization = WorkflowPackLocalizationRuntime::from_verified_bundle(&bundle)
            .expect("localization runtime");

        let english = localization.select_host_locale(WorkflowPackHostLocale::English);
        assert_eq!(english.selected_locale().as_str(), "en");
        assert_eq!(english.match_kind(), WorkflowPackLocaleMatch::Exact);

        let chinese = localization.select_host_locale(WorkflowPackHostLocale::SimplifiedChinese);
        assert_eq!(chinese.requested_locale().as_str(), "zh-CN");
        assert_eq!(chinese.selected_locale().as_str(), "zh-Hans");
        assert_eq!(chinese.match_kind(), WorkflowPackLocaleMatch::Compatible);
        assert_eq!(
            localization
                .vocabulary(&chinese)
                .expect("Chinese vocabulary")
                .application_singular,
            "申请"
        );

        let localized_stage = localization
            .resolve_text(&chinese, &bundle.manifest().workflow.stages[0].labels)
            .expect("Chinese stage label");
        assert_eq!(localized_stage.locale().as_str(), "zh-Hans");
        assert_eq!(localized_stage.value(), "导入");
        assert_eq!(
            localized_stage.match_kind(),
            WorkflowPackTextMatch::SelectedLocale
        );
        let fallback_stage = localization
            .resolve_text(&chinese, &bundle.manifest().workflow.stages[1].labels)
            .expect("default stage fallback");
        assert_eq!(fallback_stage.locale().as_str(), "en");
        assert_eq!(fallback_stage.value(), "Export");
        assert_eq!(
            fallback_stage.match_kind(),
            WorkflowPackTextMatch::PackDefault
        );

        let persisted = serde_json::to_string(&WorkflowPackHostLocale::SimplifiedChinese)
            .expect("persist host locale");
        assert_eq!(persisted, "\"zh-CN\"");
        let restored: WorkflowPackHostLocale =
            serde_json::from_str(&persisted).expect("restore host locale");
        let reopened = WorkflowPackLocalizationRuntime::from_verified_bundle(&bundle)
            .expect("reopened localization runtime");
        assert_eq!(chinese, reopened.select_host_locale(restored));
        let selection_json = serde_json::to_string(&chinese).expect("selection JSON");
        assert!(!selection_json.contains("申请"));
        assert!(!selection_json.contains("导入"));
    }

    #[test]
    fn localization_missing_default_and_cross_pack_selection_fail_closed() {
        let (manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        let empty = WorkflowPackLocalizationRuntime::try_new(
            WorkflowPackId::try_new("org.canisend.empty-locales").expect("pack ID"),
            manifest.version.clone(),
            manifest.content_digest.clone(),
            manifest.default_locale.clone(),
            BTreeMap::new(),
        )
        .expect_err("locale count must be bounded");
        assert!(matches!(
            empty,
            WorkflowPackLocalizationError::LocaleCountInvalid { .. }
        ));
        let missing = WorkflowPackLocalizationRuntime::try_new(
            WorkflowPackId::try_new("org.canisend.missing-default").expect("pack ID"),
            manifest.version.clone(),
            manifest.content_digest.clone(),
            locale("zh-Hans"),
            manifest.locales.clone(),
        )
        .expect_err("default locale must exist");
        assert!(matches!(
            missing,
            WorkflowPackLocalizationError::DefaultLocaleMissing { .. }
        ));

        let bundle = verify(&manifest, resources).expect("verified bundle");
        let first = WorkflowPackLocalizationRuntime::from_verified_bundle(&bundle)
            .expect("first localization runtime");
        let second = WorkflowPackLocalizationRuntime::try_new(
            WorkflowPackId::try_new("org.canisend.other-pack").expect("other pack ID"),
            manifest.version.clone(),
            manifest.content_digest.clone(),
            manifest.default_locale.clone(),
            manifest.locales.clone(),
        )
        .expect("other localization runtime");
        let foreign = second.select_host_locale(WorkflowPackHostLocale::English);
        assert!(matches!(
            first.vocabulary(&foreign),
            Err(WorkflowPackLocalizationError::SelectionBindingMismatch {
                mismatch: WorkflowPackSelectionBindingMismatch::PackId,
                ..
            })
        ));

        let stale = WorkflowPackLocalizationRuntime::try_new(
            manifest.id.clone(),
            manifest.version.clone(),
            Sha256Digest::try_new("f".repeat(64)).expect("stale digest"),
            manifest.default_locale.clone(),
            manifest.locales.clone(),
        )
        .expect("stale localization runtime");
        let stale_selection = stale.select_host_locale(WorkflowPackHostLocale::English);
        assert!(matches!(
            first.vocabulary(&stale_selection),
            Err(WorkflowPackLocalizationError::SelectionBindingMismatch {
                mismatch: WorkflowPackSelectionBindingMismatch::ContentDigest,
                ..
            })
        ));

        let english = first.select_host_locale(WorkflowPackHostLocale::English);
        let missing_label_default =
            WorkflowPackLocalizedText(BTreeMap::from([(locale("zh-Hans"), "仅中文".to_owned())]));
        assert!(matches!(
            first.resolve_text(&english, &missing_label_default),
            Err(WorkflowPackLocalizationError::LocalizedTextDefaultMissing { .. })
        ));

        let unavailable = locale("zh-Hant");
        let fallback = first.select_locale(&unavailable);
        assert_eq!(fallback.selected_locale().as_str(), "en");
        assert_eq!(fallback.match_kind(), WorkflowPackLocaleMatch::PackDefault);
    }

    #[test]
    fn resource_bytes_and_content_digest_fail_closed() {
        let (manifest, mut resources) = manifest_and_resources("1.0.0", b"template-v1");
        resources.insert(path("templates/statement.typ"), b"tampered!!!".to_vec());
        assert!(matches!(
            verify(&manifest, resources),
            Err(WorkflowPackBundleError::ResourceDigestMismatch { .. })
                | Err(WorkflowPackBundleError::ResourceSizeMismatch { .. })
        ));

        let (mut manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        manifest.content_digest = Sha256Digest::try_new("f".repeat(64)).expect("wrong digest");
        assert!(matches!(
            verify(&manifest, resources),
            Err(WorkflowPackBundleError::ContentDigestMismatch { .. })
        ));
    }

    #[test]
    fn byte_loader_preserves_data_digest_and_capability_failures() {
        let (manifest, mut resources) = manifest_and_resources("1.0.0", b"template-v1");
        resources.insert(
            path("templates/statement.typ"),
            b"#!/bin/sh\necho no".to_vec(),
        );
        let manifest_bytes = serde_json::to_vec(&manifest).expect("manifest bytes");
        assert!(matches!(
            WorkflowPackByteLoader::verify(
                &manifest_bytes,
                resources,
                WorkflowPackOrigin::External,
                &runtime(),
                &WorkflowPackCapabilityRegistry::built_in(),
            ),
            Err(WorkflowPackByteLoaderError::ResourceShebangRejected { .. })
        ));

        let (mut manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        manifest.content_digest = Sha256Digest::try_new("f".repeat(64)).expect("wrong digest");
        let manifest_bytes = serde_json::to_vec(&manifest).expect("manifest bytes");
        assert!(matches!(
            WorkflowPackByteLoader::verify(
                &manifest_bytes,
                resources,
                WorkflowPackOrigin::External,
                &runtime(),
                &WorkflowPackCapabilityRegistry::built_in(),
            ),
            Err(WorkflowPackByteLoaderError::Bundle(
                WorkflowPackBundleError::ContentDigestMismatch { .. }
            ))
        ));

        let (manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        let manifest_bytes = serde_json::to_vec(&manifest).expect("manifest bytes");
        assert!(matches!(
            WorkflowPackByteLoader::verify(
                &manifest_bytes,
                resources,
                WorkflowPackOrigin::External,
                &runtime(),
                &WorkflowPackCapabilityRegistry::default(),
            ),
            Err(WorkflowPackByteLoaderError::Bundle(
                WorkflowPackBundleError::CapabilityUnavailable { .. }
            ))
        ));
    }

    #[test]
    fn resource_set_rejects_missing_and_undeclared_paths() {
        let (manifest, _) = manifest_and_resources("1.0.0", b"template-v1");
        let resources = BTreeMap::from([(path("extra.txt"), b"extra".to_vec())]);
        match verify(&manifest, resources).expect_err("resource set mismatch") {
            WorkflowPackBundleError::ResourceSetMismatch {
                missing,
                undeclared,
            } => {
                assert_eq!(missing, vec![path("templates/statement.typ")]);
                assert_eq!(undeclared, vec![path("extra.txt")]);
            }
            error => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn incompatible_runtime_and_unknown_capability_are_rejected() {
        let (manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        let value = serde_json::to_value(&manifest).expect("manifest value");
        let old_runtime = WorkflowPackRuntime::parse("0.9.0", "3.0.0-alpha.1", "3.0.0-alpha.1")
            .expect("old runtime");
        assert!(matches!(
            VerifiedWorkflowPackBundle::verify(
                &value,
                resources.clone(),
                WorkflowPackOrigin::External,
                &old_runtime,
                &WorkflowPackCapabilityRegistry::built_in(),
            ),
            Err(WorkflowPackBundleError::IncompatibleRuntime {
                surface: "kernel",
                ..
            })
        ));

        assert!(matches!(
            VerifiedWorkflowPackBundle::verify(
                &value,
                resources,
                WorkflowPackOrigin::External,
                &runtime(),
                &WorkflowPackCapabilityRegistry::default(),
            ),
            Err(WorkflowPackBundleError::CapabilityUnavailable { .. })
        ));
    }

    #[test]
    fn registry_is_idempotent_exact_bound_and_never_selects_latest() {
        let (manifest_v1, resources_v1) = manifest_and_resources("1.0.0", b"template-v1");
        let bundle_v1 = verify(&manifest_v1, resources_v1).expect("v1 bundle");
        let value_v1 = serde_json::to_value(&manifest_v1).expect("v1 manifest value");
        let built_in_v1 = VerifiedWorkflowPackBundle::verify(
            &value_v1,
            bundle_v1.resources().clone(),
            WorkflowPackOrigin::BuiltIn,
            &runtime(),
            &WorkflowPackCapabilityRegistry::built_in(),
        )
        .expect("built-in v1 bundle");
        let (manifest_v2, resources_v2) = manifest_and_resources("1.1.0", b"template-v2");
        let bundle_v2 = verify(&manifest_v2, resources_v2).expect("v2 bundle");
        let mut registry = WorkflowPackRegistry::new();
        assert_eq!(
            registry.insert(bundle_v1.clone()).expect("insert v1"),
            WorkflowPackRegistryInsert::Inserted
        );
        assert_eq!(
            registry.insert(built_in_v1).expect("repeat v1"),
            WorkflowPackRegistryInsert::AlreadyPresent
        );
        assert_eq!(
            registry
                .resolve_exact(
                    &manifest_v1.id,
                    &manifest_v1.version,
                    &manifest_v1.content_digest,
                )
                .expect("registered v1")
                .snapshot()
                .origin(),
            &WorkflowPackOrigin::External,
            "the first verified origin remains authoritative",
        );
        assert_eq!(
            registry.insert(bundle_v2).expect("insert v2"),
            WorkflowPackRegistryInsert::Inserted
        );
        assert_eq!(registry.len(), 2);
        assert!(registry.contains_exact(
            &manifest_v1.id,
            &manifest_v1.version,
            &manifest_v1.content_digest
        ));
        assert!(matches!(
            registry.resolve_exact(
                &manifest_v1.id,
                &manifest_v1.version,
                &manifest_v2.content_digest,
            ),
            Err(WorkflowPackBundleError::SnapshotBindingMismatch { .. })
        ));
    }

    #[test]
    fn registry_rejects_same_version_substitution() {
        let (manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        let original = verify(&manifest, resources).expect("original bundle");
        let (substitute_manifest, substitute_resources) =
            manifest_and_resources("1.0.0", b"different-template");
        let substitute =
            verify(&substitute_manifest, substitute_resources).expect("substitute bundle");
        let mut registry = WorkflowPackRegistry::new();
        registry.insert(original).expect("insert original");
        assert!(matches!(
            registry.insert(substitute),
            Err(WorkflowPackBundleError::VersionSubstitution { .. })
        ));
    }

    #[test]
    fn canonical_digest_is_stable_across_manifest_object_key_order() {
        let (manifest, resources) = manifest_and_resources("1.0.0", b"template-v1");
        let canonical = calculate_workflow_pack_content_digest(&manifest, &resources)
            .expect("canonical digest");
        let encoded = serde_json::to_string(&manifest).expect("manifest JSON");
        let reparsed: Value = serde_json::from_str(&encoded).expect("reparsed value");
        let reparsed_manifest: WorkflowPackManifest =
            serde_json::from_value(reparsed).expect("reparsed manifest");
        let repeated = calculate_workflow_pack_content_digest(&reparsed_manifest, &resources)
            .expect("repeated digest");
        assert_eq!(canonical, repeated);
    }
}
