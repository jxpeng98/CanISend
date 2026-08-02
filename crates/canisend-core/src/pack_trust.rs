use std::collections::BTreeMap;

use canisend_contracts::{
    SafeRelativePath, SemanticVersion, Sha256Digest, WORKFLOW_PACK_MAX_MANIFEST_BYTES,
    WORKFLOW_PACK_MAX_RESOURCE_BYTES, WORKFLOW_PACK_MAX_RESOURCES,
    WORKFLOW_PACK_MAX_TOTAL_RESOURCE_BYTES, WorkflowPackCapabilityId, WorkflowPackId,
    WorkflowPackPublisherId,
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    VerifiedWorkflowPackBundle, WorkflowPackBundleError, WorkflowPackCapabilityKind,
    WorkflowPackCapabilityRegistry, WorkflowPackOrigin, WorkflowPackRuntime,
};

pub const WORKFLOW_PACK_TRUST_REPORT_FORMAT: &str = "canisend.workflow-pack-trust-report/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackTrustStatus {
    VerifiedDataOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackPublisherAuthentication {
    DeclaredOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackSignatureStatus {
    NotSpecifiedByV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackInstallationStatus {
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackExecutablePolicy {
    DataOnlyNoExecutionAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackTrustCheckStatus {
    Passed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackTrustCapabilityReference {
    kind: WorkflowPackCapabilityKind,
    id: WorkflowPackCapabilityId,
}

impl WorkflowPackTrustCapabilityReference {
    #[must_use]
    pub const fn kind(&self) -> WorkflowPackCapabilityKind {
        self.kind
    }

    #[must_use]
    pub const fn id(&self) -> &WorkflowPackCapabilityId {
        &self.id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackTrustCheck {
    code: String,
    status: WorkflowPackTrustCheckStatus,
    description: String,
}

impl WorkflowPackTrustCheck {
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    #[must_use]
    pub const fn status(&self) -> WorkflowPackTrustCheckStatus {
        self.status
    }

    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackTrustReport {
    format: String,
    status: WorkflowPackTrustStatus,
    pack_id: WorkflowPackId,
    pack_version: SemanticVersion,
    content_digest: Sha256Digest,
    origin: WorkflowPackOrigin,
    publisher_id: WorkflowPackPublisherId,
    publisher_authentication: WorkflowPackPublisherAuthentication,
    signature_status: WorkflowPackSignatureStatus,
    installation_status: WorkflowPackInstallationStatus,
    executable_policy: WorkflowPackExecutablePolicy,
    manifest_bytes: u64,
    resource_count: usize,
    resource_bytes: u64,
    capability_references: Vec<WorkflowPackTrustCapabilityReference>,
    checks: Vec<WorkflowPackTrustCheck>,
    contains_resource_bodies: bool,
}

impl WorkflowPackTrustReport {
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    #[must_use]
    pub const fn status(&self) -> WorkflowPackTrustStatus {
        self.status
    }

    #[must_use]
    pub const fn pack_id(&self) -> &WorkflowPackId {
        &self.pack_id
    }

    #[must_use]
    pub const fn pack_version(&self) -> &SemanticVersion {
        &self.pack_version
    }

    #[must_use]
    pub const fn content_digest(&self) -> &Sha256Digest {
        &self.content_digest
    }

    #[must_use]
    pub const fn origin(&self) -> &WorkflowPackOrigin {
        &self.origin
    }

    #[must_use]
    pub const fn publisher_id(&self) -> &WorkflowPackPublisherId {
        &self.publisher_id
    }

    #[must_use]
    pub const fn publisher_authentication(&self) -> WorkflowPackPublisherAuthentication {
        self.publisher_authentication
    }

    #[must_use]
    pub const fn signature_status(&self) -> WorkflowPackSignatureStatus {
        self.signature_status
    }

    #[must_use]
    pub const fn installation_status(&self) -> WorkflowPackInstallationStatus {
        self.installation_status
    }

    #[must_use]
    pub const fn executable_policy(&self) -> WorkflowPackExecutablePolicy {
        self.executable_policy
    }

    #[must_use]
    pub const fn manifest_bytes(&self) -> u64 {
        self.manifest_bytes
    }

    #[must_use]
    pub const fn resource_count(&self) -> usize {
        self.resource_count
    }

    #[must_use]
    pub const fn resource_bytes(&self) -> u64 {
        self.resource_bytes
    }

    #[must_use]
    pub fn capability_references(&self) -> &[WorkflowPackTrustCapabilityReference] {
        &self.capability_references
    }

    #[must_use]
    pub fn checks(&self) -> &[WorkflowPackTrustCheck] {
        &self.checks
    }

    #[must_use]
    pub const fn contains_resource_bodies(&self) -> bool {
        self.contains_resource_bodies
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedWorkflowPackCandidate {
    bundle: VerifiedWorkflowPackBundle,
    trust_report: WorkflowPackTrustReport,
}

impl VerifiedWorkflowPackCandidate {
    #[must_use]
    pub const fn bundle(&self) -> &VerifiedWorkflowPackBundle {
        &self.bundle
    }

    #[must_use]
    pub const fn trust_report(&self) -> &WorkflowPackTrustReport {
        &self.trust_report
    }

    #[must_use]
    pub fn into_bundle(self) -> VerifiedWorkflowPackBundle {
        self.bundle
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WorkflowPackByteLoader;

impl WorkflowPackByteLoader {
    pub fn verify(
        manifest_bytes: &[u8],
        resources: BTreeMap<SafeRelativePath, Vec<u8>>,
        origin: WorkflowPackOrigin,
        runtime: &WorkflowPackRuntime,
        capabilities: &WorkflowPackCapabilityRegistry,
    ) -> Result<VerifiedWorkflowPackCandidate, WorkflowPackByteLoaderError> {
        validate_manifest_bound(manifest_bytes)?;
        let resource_bytes = validate_resource_bounds_and_content(&resources)?;
        let manifest_value: Value = serde_json::from_slice(manifest_bytes).map_err(|error| {
            WorkflowPackByteLoaderError::ManifestJsonInvalid {
                message: error.to_string(),
            }
        })?;
        let manifest_size = u64::try_from(manifest_bytes.len())
            .expect("supported manifest byte length fits in u64");
        let resource_count = resources.len();
        let bundle = VerifiedWorkflowPackBundle::verify(
            &manifest_value,
            resources,
            origin,
            runtime,
            capabilities,
        )?;
        let trust_report =
            build_trust_report(&bundle, manifest_size, resource_count, resource_bytes);
        Ok(VerifiedWorkflowPackCandidate {
            bundle,
            trust_report,
        })
    }
}

fn validate_manifest_bound(manifest_bytes: &[u8]) -> Result<(), WorkflowPackByteLoaderError> {
    if manifest_bytes.is_empty() {
        return Err(WorkflowPackByteLoaderError::ManifestEmpty);
    }
    if manifest_bytes.len() > WORKFLOW_PACK_MAX_MANIFEST_BYTES {
        return Err(WorkflowPackByteLoaderError::ManifestTooLarge {
            maximum: WORKFLOW_PACK_MAX_MANIFEST_BYTES,
            actual: manifest_bytes.len(),
        });
    }
    Ok(())
}

fn validate_resource_bounds_and_content(
    resources: &BTreeMap<SafeRelativePath, Vec<u8>>,
) -> Result<u64, WorkflowPackByteLoaderError> {
    if resources.len() > WORKFLOW_PACK_MAX_RESOURCES {
        return Err(WorkflowPackByteLoaderError::ResourceCountTooLarge {
            maximum: WORKFLOW_PACK_MAX_RESOURCES,
            actual: resources.len(),
        });
    }
    let mut total = 0_u64;
    for (path, bytes) in resources {
        let actual = u64::try_from(bytes.len()).expect("resource byte length fits in u64");
        if actual > WORKFLOW_PACK_MAX_RESOURCE_BYTES {
            return Err(WorkflowPackByteLoaderError::ResourceTooLarge {
                path: path.clone(),
                maximum: WORKFLOW_PACK_MAX_RESOURCE_BYTES,
                actual,
            });
        }
        total = total.checked_add(actual).ok_or(
            WorkflowPackByteLoaderError::ResourceTotalTooLarge {
                maximum: WORKFLOW_PACK_MAX_TOTAL_RESOURCE_BYTES,
                actual: u64::MAX,
            },
        )?;
        if total > WORKFLOW_PACK_MAX_TOTAL_RESOURCE_BYTES {
            return Err(WorkflowPackByteLoaderError::ResourceTotalTooLarge {
                maximum: WORKFLOW_PACK_MAX_TOTAL_RESOURCE_BYTES,
                actual: total,
            });
        }
        validate_data_resource(path, bytes)?;
    }
    Ok(total)
}

fn validate_data_resource(
    path: &SafeRelativePath,
    bytes: &[u8],
) -> Result<(), WorkflowPackByteLoaderError> {
    if let Some(signature) = binary_signature(bytes) {
        return Err(
            WorkflowPackByteLoaderError::ResourceBinarySignatureRejected {
                path: path.clone(),
                signature,
            },
        );
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| WorkflowPackByteLoaderError::ResourceNotUtf8 { path: path.clone() })?;
    if let Some((offset, byte)) = bytes
        .iter()
        .copied()
        .enumerate()
        .find(|(_, byte)| *byte < 0x20 && !matches!(*byte, b'\t' | b'\n' | b'\r'))
    {
        return Err(WorkflowPackByteLoaderError::ResourceControlByteRejected {
            path: path.clone(),
            offset,
            byte,
        });
    }
    if text
        .trim_start_matches(|character: char| {
            character == '\u{feff}' || character.is_ascii_whitespace()
        })
        .starts_with("#!")
    {
        return Err(WorkflowPackByteLoaderError::ResourceShebangRejected { path: path.clone() });
    }
    Ok(())
}

fn binary_signature(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x7fELF") {
        return Some("elf");
    }
    if bytes.starts_with(b"MZ") {
        return Some("portable-executable");
    }
    if bytes.starts_with(b"\0asm") {
        return Some("webassembly");
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return Some("zip-archive");
    }
    if [
        [0xfe, 0xed, 0xfa, 0xce],
        [0xce, 0xfa, 0xed, 0xfe],
        [0xfe, 0xed, 0xfa, 0xcf],
        [0xcf, 0xfa, 0xed, 0xfe],
        [0xca, 0xfe, 0xba, 0xbe],
    ]
    .iter()
    .any(|signature| bytes.starts_with(signature))
    {
        return Some("mach-o");
    }
    None
}

fn build_trust_report(
    bundle: &VerifiedWorkflowPackBundle,
    manifest_bytes: u64,
    resource_count: usize,
    resource_bytes: u64,
) -> WorkflowPackTrustReport {
    let manifest = bundle.manifest();
    let mut capability_references = Vec::new();
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
        capability_references.extend(
            selected
                .iter()
                .cloned()
                .map(|id| WorkflowPackTrustCapabilityReference { kind, id }),
        );
    }
    capability_references
        .sort_unstable_by(|left, right| (left.kind, &left.id).cmp(&(right.kind, &right.id)));
    WorkflowPackTrustReport {
        format: WORKFLOW_PACK_TRUST_REPORT_FORMAT.to_owned(),
        status: WorkflowPackTrustStatus::VerifiedDataOnly,
        pack_id: manifest.id.clone(),
        pack_version: manifest.version.clone(),
        content_digest: manifest.content_digest.clone(),
        origin: bundle.snapshot().origin().clone(),
        publisher_id: manifest.publisher.id.clone(),
        publisher_authentication: WorkflowPackPublisherAuthentication::DeclaredOnly,
        signature_status: WorkflowPackSignatureStatus::NotSpecifiedByV1,
        installation_status: WorkflowPackInstallationStatus::Disabled,
        executable_policy: WorkflowPackExecutablePolicy::DataOnlyNoExecutionAuthority,
        manifest_bytes,
        resource_count,
        resource_bytes,
        capability_references,
        checks: passed_checks(),
        contains_resource_bodies: false,
    }
}

fn passed_checks() -> Vec<WorkflowPackTrustCheck> {
    [
        (
            "manifest-bounds",
            "Manifest bytes were limited before JSON parsing.",
        ),
        (
            "typed-manifest",
            "Schema, primitive, structural, and semantic validation passed.",
        ),
        (
            "runtime-compatibility",
            "Kernel, Agent, and Workspace compatibility requirements matched.",
        ),
        (
            "safe-resource-paths",
            "All declared and supplied resource paths use portable safe relative paths.",
        ),
        (
            "bounded-data-resources",
            "Resource counts and bytes were bounded and accepted as UTF-8 data-only content.",
        ),
        (
            "resource-digests",
            "Every exact resource byte sequence matched its declared size and SHA-256.",
        ),
        (
            "registered-capabilities",
            "Every selected adapter, Renderer, and Validator is kernel-registered.",
        ),
        (
            "bundle-content-digest",
            "The domain-separated canonical bundle digest matched the Manifest.",
        ),
    ]
    .into_iter()
    .map(|(code, description)| WorkflowPackTrustCheck {
        code: code.to_owned(),
        status: WorkflowPackTrustCheckStatus::Passed,
        description: description.to_owned(),
    })
    .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowPackByteLoaderError {
    #[error("workflow-pack Manifest bytes are empty")]
    ManifestEmpty,
    #[error(
        "workflow-pack Manifest exceeds the {maximum}-byte pre-parse limit; found {actual} bytes"
    )]
    ManifestTooLarge { maximum: usize, actual: usize },
    #[error("workflow-pack Manifest is not valid JSON: {message}")]
    ManifestJsonInvalid { message: String },
    #[error(
        "workflow-pack resource count exceeds the {maximum}-resource pre-parse limit; found {actual}"
    )]
    ResourceCountTooLarge { maximum: usize, actual: usize },
    #[error("workflow-pack resource {path} exceeds the {maximum}-byte limit; found {actual} bytes")]
    ResourceTooLarge {
        path: SafeRelativePath,
        maximum: u64,
        actual: u64,
    },
    #[error(
        "workflow-pack resource bytes exceed the {maximum}-byte total limit; found {actual} bytes"
    )]
    ResourceTotalTooLarge { maximum: u64, actual: u64 },
    #[error("workflow-pack resource {path} has rejected binary signature {signature}")]
    ResourceBinarySignatureRejected {
        path: SafeRelativePath,
        signature: &'static str,
    },
    #[error("workflow-pack resource {path} is not UTF-8 data")]
    ResourceNotUtf8 { path: SafeRelativePath },
    #[error(
        "workflow-pack resource {path} contains rejected control byte 0x{byte:02x} at offset {offset}"
    )]
    ResourceControlByteRejected {
        path: SafeRelativePath,
        offset: usize,
        byte: u8,
    },
    #[error("workflow-pack resource {path} begins with a rejected executable shebang")]
    ResourceShebangRejected { path: SafeRelativePath },
    #[error(transparent)]
    Bundle(#[from] WorkflowPackBundleError),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use canisend_contracts::{
        SafeRelativePath, WORKFLOW_PACK_MAX_MANIFEST_BYTES, WORKFLOW_PACK_MAX_RESOURCE_BYTES,
        WORKFLOW_PACK_MAX_RESOURCES,
    };

    use super::{
        WorkflowPackByteLoader, WorkflowPackByteLoaderError, WorkflowPackCapabilityRegistry,
        WorkflowPackOrigin, WorkflowPackRuntime, validate_data_resource,
    };

    fn path(value: &str) -> SafeRelativePath {
        SafeRelativePath::try_new(value).expect("safe path")
    }

    #[test]
    fn data_policy_accepts_text_and_rejects_executable_binary_and_control_content() {
        let resource_path = path("templates/example.typ");
        validate_data_resource(&resource_path, "Hello, 世界\n#let x = 1".as_bytes())
            .expect("UTF-8 template data");
        for (bytes, expected) in [
            (b"\x7fELFbinary".as_slice(), "binary signature"),
            (b"MZbinary".as_slice(), "binary signature"),
            (b"PK\x03\x04binary".as_slice(), "binary signature"),
            (b"\xffinvalid".as_slice(), "not UTF-8"),
            (b"text\x01control".as_slice(), "control byte"),
            (b" \n#!/bin/sh".as_slice(), "executable shebang"),
        ] {
            let error = validate_data_resource(&resource_path, bytes).expect_err("must reject");
            assert!(
                error.to_string().contains(expected),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn preparse_limits_fail_before_manifest_decoding() {
        let runtime = WorkflowPackRuntime::parse("1.0.0-alpha.5", "3.0.0-alpha.1", "3.0.0-alpha.1")
            .expect("runtime");
        let capabilities = WorkflowPackCapabilityRegistry::built_in();
        let oversized_manifest = vec![b' '; WORKFLOW_PACK_MAX_MANIFEST_BYTES + 1];
        assert!(matches!(
            WorkflowPackByteLoader::verify(
                &oversized_manifest,
                BTreeMap::new(),
                WorkflowPackOrigin::External,
                &runtime,
                &capabilities,
            ),
            Err(WorkflowPackByteLoaderError::ManifestTooLarge { .. })
        ));

        let oversized_resource = BTreeMap::from([(
            path("examples/large.txt"),
            vec![b'x'; usize::try_from(WORKFLOW_PACK_MAX_RESOURCE_BYTES + 1).expect("test size")],
        )]);
        assert!(matches!(
            WorkflowPackByteLoader::verify(
                b"{}",
                oversized_resource,
                WorkflowPackOrigin::External,
                &runtime,
                &capabilities,
            ),
            Err(WorkflowPackByteLoaderError::ResourceTooLarge { .. })
        ));

        let too_many_resources = (0..=WORKFLOW_PACK_MAX_RESOURCES)
            .map(|index| (path(&format!("examples/item-{index}.txt")), Vec::new()))
            .collect();
        assert!(matches!(
            WorkflowPackByteLoader::verify(
                b"{}",
                too_many_resources,
                WorkflowPackOrigin::External,
                &runtime,
                &capabilities,
            ),
            Err(WorkflowPackByteLoaderError::ResourceCountTooLarge { .. })
        ));

        assert!(matches!(
            WorkflowPackByteLoader::verify(
                b"not-json",
                BTreeMap::new(),
                WorkflowPackOrigin::External,
                &runtime,
                &capabilities,
            ),
            Err(WorkflowPackByteLoaderError::ManifestJsonInvalid { .. })
        ));
    }
}
