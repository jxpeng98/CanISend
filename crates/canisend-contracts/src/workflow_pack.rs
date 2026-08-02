use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

use schemars::JsonSchema;
use semver::VersionReq;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;
use thiserror::Error;

use crate::{
    CandidateValidationError, ContractViolation, ExecutionMode, SafeRelativePath, SemanticValidate,
    SemanticVersion, Sha256Digest, validate_external_candidate,
};

pub const WORKFLOW_PACK_FORMAT: &str = "canisend.workflow-pack/v1";
pub const WORKFLOW_PACK_SCHEMA_VERSION: &str = "1.0.0";
pub const WORKFLOW_PACK_SCHEMA_ID: &str = "canisend.workflow-pack-manifest/v1";
pub const WORKFLOW_PACK_SCHEMA_URI: &str =
    "https://schemas.canisend.dev/workflow-pack/v1/manifest.schema.json";
pub const WORKFLOW_PACK_MAX_MANIFEST_BYTES: usize = 1024 * 1024;
pub const WORKFLOW_PACK_MAX_JSON_DEPTH: usize = 32;
pub const WORKFLOW_PACK_MAX_JSON_NODES: usize = 20_000;
pub const WORKFLOW_PACK_MAX_RESOURCE_BYTES: u64 = 8 * 1024 * 1024;
pub const WORKFLOW_PACK_MAX_TOTAL_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;
pub const WORKFLOW_PACK_MAX_RESOURCES: usize = 512;
pub const WORKFLOW_PACK_MAX_LOCALES: usize = 16;
pub const WORKFLOW_PACK_MAX_STAGES: usize = 64;
pub const WORKFLOW_PACK_MAX_DELIVERABLE_KINDS: usize = 64;
pub const WORKFLOW_PACK_MAX_DELIVERABLE_CARDINALITY: u16 = 32;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code}: {message}")]
pub struct WorkflowPackIdentityError {
    pub code: &'static str,
    pub message: String,
}

impl WorkflowPackIdentityError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

macro_rules! workflow_pack_string {
    ($name:ident, $validator:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn try_new(value: impl Into<String>) -> Result<Self, WorkflowPackIdentityError> {
                let value = value.into();
                $validator(&value)?;
                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = WorkflowPackIdentityError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_new(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::try_new(value).map_err(D::Error::custom)
            }
        }
    };
}

workflow_pack_string!(WorkflowPackId, validate_pack_id);
workflow_pack_string!(WorkflowPackPublisherId, validate_publisher_id);
workflow_pack_string!(WorkflowPackItemId, validate_item_id);
workflow_pack_string!(WorkflowPackLocaleId, validate_locale_id);
workflow_pack_string!(WorkflowPackCapabilityId, validate_capability_id);
workflow_pack_string!(StageId, validate_stage_id);
workflow_pack_string!(DeliverableKindId, validate_deliverable_kind_id);

impl StageId {
    const SEPARATOR: char = ':';

    #[must_use]
    pub fn from_parts(pack_id: &WorkflowPackId, local_id: &WorkflowPackItemId) -> Self {
        Self(format!(
            "{}{separator}{}",
            pack_id.as_str(),
            local_id.as_str(),
            separator = Self::SEPARATOR,
        ))
    }

    #[must_use]
    pub fn pack_id_str(&self) -> &str {
        self.0
            .split_once(Self::SEPARATOR)
            .expect("validated StageId contains the separator")
            .0
    }

    #[must_use]
    pub fn local_id_str(&self) -> &str {
        self.0
            .split_once(Self::SEPARATOR)
            .expect("validated StageId contains the separator")
            .1
    }
}

impl DeliverableKindId {
    const SEPARATOR: char = ':';

    #[must_use]
    pub fn from_parts(pack_id: &WorkflowPackId, local_id: &WorkflowPackItemId) -> Self {
        Self(format!(
            "{}{separator}{}",
            pack_id.as_str(),
            local_id.as_str(),
            separator = Self::SEPARATOR,
        ))
    }

    #[must_use]
    pub fn pack_id_str(&self) -> &str {
        self.0
            .split_once(Self::SEPARATOR)
            .expect("validated DeliverableKindId contains the separator")
            .0
    }

    #[must_use]
    pub fn local_id_str(&self) -> &str {
        self.0
            .split_once(Self::SEPARATOR)
            .expect("validated DeliverableKindId contains the separator")
            .1
    }
}

fn validate_pack_id(value: &str) -> Result<(), WorkflowPackIdentityError> {
    validate_qualified_id(value, 3, "workflow_pack.id_invalid")
}

fn validate_publisher_id(value: &str) -> Result<(), WorkflowPackIdentityError> {
    validate_qualified_id(value, 2, "workflow_pack.publisher_id_invalid")
}

fn validate_capability_id(value: &str) -> Result<(), WorkflowPackIdentityError> {
    validate_qualified_id(value, 2, "workflow_pack.capability_id_invalid")
}

fn validate_stage_id(value: &str) -> Result<(), WorkflowPackIdentityError> {
    validate_pack_owned_id(
        value,
        "workflow_pack.stage_id_invalid",
        "expected `<workflow-pack-id>:<local-stage-id>`",
    )
}

fn validate_deliverable_kind_id(value: &str) -> Result<(), WorkflowPackIdentityError> {
    validate_pack_owned_id(
        value,
        "workflow_pack.deliverable_kind_id_invalid",
        "expected `<workflow-pack-id>:<local-deliverable-kind-id>`",
    )
}

fn validate_pack_owned_id(
    value: &str,
    code: &'static str,
    message: &'static str,
) -> Result<(), WorkflowPackIdentityError> {
    let invalid = || WorkflowPackIdentityError::new(code, message);
    let Some((pack_id, local_id)) = value.split_once(':') else {
        return Err(invalid());
    };
    if local_id.contains(':')
        || validate_pack_id(pack_id).is_err()
        || validate_item_id(local_id).is_err()
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_qualified_id(
    value: &str,
    minimum_segments: usize,
    code: &'static str,
) -> Result<(), WorkflowPackIdentityError> {
    let segments = value.split('.').collect::<Vec<_>>();
    if value.len() > 128
        || segments.len() < minimum_segments
        || segments.iter().any(|segment| {
            segment.is_empty()
                || segment.len() > 63
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                || !segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                || !segment
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
    {
        return Err(WorkflowPackIdentityError::new(
            code,
            "expected a lowercase reverse-domain identifier with portable segments",
        ));
    }
    Ok(())
}

fn validate_item_id(value: &str) -> Result<(), WorkflowPackIdentityError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !bytes[0].is_ascii_lowercase()
        || !bytes
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(WorkflowPackIdentityError::new(
            "workflow_pack.item_id_invalid",
            "expected a 1-64 byte lowercase kebab-case identifier beginning with a letter",
        ));
    }
    Ok(())
}

fn validate_locale_id(value: &str) -> Result<(), WorkflowPackIdentityError> {
    let segments = value.split('-').collect::<Vec<_>>();
    if value.len() > 35
        || !(2..=3).contains(&segments[0].len())
        || !segments[0].bytes().all(|byte| byte.is_ascii_lowercase())
        || segments.iter().skip(1).any(|segment| {
            segment.is_empty()
                || segment.len() > 8
                || !segment.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
    {
        return Err(WorkflowPackIdentityError::new(
            "workflow_pack.locale_id_invalid",
            "expected a bounded BCP-47-like locale such as en or zh-Hans",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum WorkflowPackFormat {
    #[serde(rename = "canisend.workflow-pack/v1")]
    V1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct WorkflowPackLocalizedText(pub BTreeMap<WorkflowPackLocaleId, String>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackPublisher {
    pub id: WorkflowPackPublisherId,
    pub name: String,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackCompatibility {
    pub kernel: String,
    pub agent: String,
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackVocabulary {
    pub application_singular: String,
    pub application_plural: String,
    pub opportunity_singular: String,
    pub opportunity_plural: String,
    pub requirement_plural: String,
    pub evidence_plural: String,
    pub deliverable_plural: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackFieldType {
    ShortText,
    LongText,
    Integer,
    Boolean,
    Date,
    Url,
    StringList,
    Choice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackFieldOption {
    pub id: WorkflowPackItemId,
    pub labels: WorkflowPackLocalizedText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackFieldDefinition {
    pub id: WorkflowPackItemId,
    pub labels: WorkflowPackLocalizedText,
    pub field_type: WorkflowPackFieldType,
    pub required: bool,
    pub options: Vec<WorkflowPackFieldOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackApplicationDefinition {
    pub opportunity_fields: Vec<WorkflowPackFieldDefinition>,
    pub application_fields: Vec<WorkflowPackFieldDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackCategoryDefinition {
    pub id: WorkflowPackItemId,
    pub labels: WorkflowPackLocalizedText,
    pub fields: Vec<WorkflowPackFieldDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackTaxonomy {
    pub categories: Vec<WorkflowPackCategoryDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackStageOutput {
    None,
    Sources,
    Requirements,
    Evidence,
    Matches,
    Plan,
    Deliverables,
    Review,
    Package,
    Render,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackStageDefinition {
    pub id: WorkflowPackItemId,
    pub labels: WorkflowPackLocalizedText,
    pub depends_on: Vec<WorkflowPackItemId>,
    pub output: WorkflowPackStageOutput,
    pub execution_modes: Vec<ExecutionMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackWorkflowDefinition {
    pub stages: Vec<WorkflowPackStageDefinition>,
    pub terminal_stage: WorkflowPackItemId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackDeliverableDefinition {
    pub id: WorkflowPackItemId,
    pub labels: WorkflowPackLocalizedText,
    pub minimum: u16,
    pub maximum: u16,
    pub template: Option<SafeRelativePath>,
    pub renderer: Option<WorkflowPackCapabilityId>,
    pub validators: Vec<WorkflowPackItemId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackDeliverableCatalog {
    pub kinds: Vec<WorkflowPackDeliverableDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackCapabilities {
    pub intake_adapters: Vec<WorkflowPackCapabilityId>,
    pub renderers: Vec<WorkflowPackCapabilityId>,
    pub validators: Vec<WorkflowPackCapabilityId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackValidatorDefinition {
    pub id: WorkflowPackItemId,
    pub capability: WorkflowPackCapabilityId,
    pub parameters: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackValidationPolicy {
    pub definitions: Vec<WorkflowPackValidatorDefinition>,
    pub readiness: Vec<WorkflowPackItemId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackResourceKind {
    Prompt,
    Template,
    Example,
    Translation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackResource {
    pub id: WorkflowPackItemId,
    pub kind: WorkflowPackResourceKind,
    pub path: SafeRelativePath,
    pub version: SemanticVersion,
    pub size_bytes: u64,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowPackMigrationKind {
    Stage,
    RequirementCategory,
    EvidenceCategory,
    Deliverable,
    Resource,
    Validator,
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackIdMapping {
    pub kind: WorkflowPackMigrationKind,
    pub from: WorkflowPackItemId,
    pub to: WorkflowPackItemId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackMigration {
    pub from_version: SemanticVersion,
    pub mappings: Vec<WorkflowPackIdMapping>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackManifest {
    pub format: WorkflowPackFormat,
    pub id: WorkflowPackId,
    pub version: SemanticVersion,
    pub schema_version: SemanticVersion,
    pub publisher: WorkflowPackPublisher,
    pub compatibility: WorkflowPackCompatibility,
    pub default_locale: WorkflowPackLocaleId,
    pub locales: BTreeMap<WorkflowPackLocaleId, WorkflowPackVocabulary>,
    pub application: WorkflowPackApplicationDefinition,
    pub workflow: WorkflowPackWorkflowDefinition,
    pub requirements: WorkflowPackTaxonomy,
    pub evidence: WorkflowPackTaxonomy,
    pub deliverables: WorkflowPackDeliverableCatalog,
    pub capabilities: WorkflowPackCapabilities,
    pub validation: WorkflowPackValidationPolicy,
    pub resources: Vec<WorkflowPackResource>,
    pub migrations: Vec<WorkflowPackMigration>,
    pub content_digest: Sha256Digest,
}

pub fn validate_workflow_pack_manifest(
    value: &Value,
) -> Result<WorkflowPackManifest, CandidateValidationError> {
    let encoded = serde_json::to_vec(value).expect("JSON value always serializes");
    if encoded.len() > WORKFLOW_PACK_MAX_MANIFEST_BYTES {
        return Err(CandidateValidationError::Structural(vec![
            ContractViolation::new(
                "workflow_pack.manifest_too_large",
                "",
                format!(
                    "workflow-pack manifest exceeds the {}-byte limit",
                    WORKFLOW_PACK_MAX_MANIFEST_BYTES
                ),
            ),
        ]));
    }
    let mut nodes = 0;
    if !validate_json_shape(value, 0, &mut nodes) {
        return Err(CandidateValidationError::Structural(vec![
            ContractViolation::new(
                "workflow_pack.manifest_shape_invalid",
                "",
                format!(
                    "workflow-pack manifest exceeds depth {} or node count {}",
                    WORKFLOW_PACK_MAX_JSON_DEPTH, WORKFLOW_PACK_MAX_JSON_NODES
                ),
            ),
        ]));
    }
    validate_external_candidate(value)
}

fn validate_json_shape(value: &Value, depth: usize, nodes: &mut usize) -> bool {
    *nodes = nodes.saturating_add(1);
    if depth > WORKFLOW_PACK_MAX_JSON_DEPTH || *nodes > WORKFLOW_PACK_MAX_JSON_NODES {
        return false;
    }
    match value {
        Value::Array(values) => values
            .iter()
            .all(|value| validate_json_shape(value, depth + 1, nodes)),
        Value::Object(values) => values
            .values()
            .all(|value| validate_json_shape(value, depth + 1, nodes)),
        _ => true,
    }
}

impl SemanticValidate for WorkflowPackManifest {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if self.schema_version.as_str() != WORKFLOW_PACK_SCHEMA_VERSION {
            violations.push(ContractViolation::new(
                "workflow_pack.schema_version_unsupported",
                "/schema_version",
                format!("schema_version must be {WORKFLOW_PACK_SCHEMA_VERSION}"),
            ));
        }
        let publisher_prefix = format!("{}.", self.publisher.id.as_str());
        if !self.id.as_str().starts_with(&publisher_prefix) {
            violations.push(ContractViolation::new(
                "workflow_pack.publisher_mismatch",
                "/id",
                "pack ID must be namespaced under the publisher ID",
            ));
        }
        required_bounded_text(
            &self.publisher.name,
            "/publisher/name",
            256,
            &mut violations,
        );
        if let Some(homepage) = &self.publisher.homepage
            && (homepage.len() > 2048 || !homepage.starts_with("https://"))
        {
            violations.push(ContractViolation::new(
                "workflow_pack.publisher_homepage_invalid",
                "/publisher/homepage",
                "publisher homepage must be a bounded HTTPS URL",
            ));
        }
        validate_compatibility(&self.compatibility, &mut violations);
        validate_locales(self, &mut violations);
        validate_application_fields(self, &mut violations);
        validate_taxonomy(
            &self.requirements,
            "/requirements",
            &self.default_locale,
            &self.locales,
            &mut violations,
        );
        validate_taxonomy(
            &self.evidence,
            "/evidence",
            &self.default_locale,
            &self.locales,
            &mut violations,
        );
        validate_capabilities(&self.capabilities, &mut violations);
        validate_validators(self, &mut violations);
        validate_resources(self, &mut violations);
        validate_deliverables(self, &mut violations);
        validate_workflow(self, &mut violations);
        validate_migrations(self, &mut violations);
        violations
    }
}

fn required_bounded_text(
    value: &str,
    pointer: &str,
    maximum: usize,
    violations: &mut Vec<ContractViolation>,
) {
    if value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        violations.push(ContractViolation::new(
            "workflow_pack.text_invalid",
            pointer,
            format!("value must contain 1-{maximum} non-control bytes"),
        ));
    }
    if value.chars().any(is_disallowed_bidi_control) {
        violations.push(ContractViolation::new(
            "workflow_pack.text_bidi_control_invalid",
            pointer,
            "value cannot contain bidirectional formatting or override controls",
        ));
    }
}

fn is_disallowed_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{206f}'
    )
}

fn placeholder_signature(value: &str) -> Option<BTreeMap<String, usize>> {
    let bytes = value.as_bytes();
    let mut placeholders = BTreeMap::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'{' if bytes.get(index + 1) == Some(&b'{') => index += 2,
            b'}' if bytes.get(index + 1) == Some(&b'}') => index += 2,
            b'{' => {
                let relative_end = bytes
                    .get(index + 1..)?
                    .iter()
                    .position(|byte| *byte == b'}')?;
                let end = index + relative_end + 1;
                let key = std::str::from_utf8(bytes.get(index + 1..end)?).ok()?;
                if !valid_placeholder_key(key) {
                    return None;
                }
                *placeholders.entry(key.to_owned()).or_insert(0) += 1;
                index = end + 1;
            }
            b'}' => return None,
            _ => index += 1,
        }
    }
    Some(placeholders)
}

fn valid_placeholder_key(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn validate_placeholder_text(
    value: &str,
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) -> Option<BTreeMap<String, usize>> {
    let signature = placeholder_signature(value);
    if signature.is_none() {
        violations.push(ContractViolation::new(
            "workflow_pack.placeholder_syntax_invalid",
            pointer,
            "placeholders must use balanced {lowercase-kebab-case} tokens; double braces escape literal braces",
        ));
    }
    signature
}

fn validate_compatibility(
    compatibility: &WorkflowPackCompatibility,
    violations: &mut Vec<ContractViolation>,
) {
    for (name, value) in [
        ("kernel", &compatibility.kernel),
        ("agent", &compatibility.agent),
        ("workspace", &compatibility.workspace),
    ] {
        if value.len() > 128 || VersionReq::parse(value).is_err() {
            violations.push(ContractViolation::new(
                "workflow_pack.compatibility_invalid",
                format!("/compatibility/{name}"),
                "compatibility value must be a bounded semantic-version requirement",
            ));
        }
    }
}

fn validate_locales(manifest: &WorkflowPackManifest, violations: &mut Vec<ContractViolation>) {
    if manifest.locales.is_empty() || manifest.locales.len() > WORKFLOW_PACK_MAX_LOCALES {
        violations.push(ContractViolation::new(
            "workflow_pack.locale_count_invalid",
            "/locales",
            format!(
                "a workflow pack must define between 1 and {WORKFLOW_PACK_MAX_LOCALES} locales"
            ),
        ));
    }
    if !manifest.locales.contains_key(&manifest.default_locale) {
        violations.push(ContractViolation::new(
            "workflow_pack.default_locale_missing",
            "/default_locale",
            "default locale must exist in the locales map",
        ));
    }
    for (locale, vocabulary) in &manifest.locales {
        for (field, value) in vocabulary_fields(vocabulary) {
            required_bounded_text(
                value,
                &format!("/locales/{locale}/{field}"),
                256,
                violations,
            );
        }
    }
    validate_vocabulary_placeholders(manifest, violations);
}

fn vocabulary_fields(vocabulary: &WorkflowPackVocabulary) -> [(&'static str, &str); 7] {
    [
        ("application_singular", &vocabulary.application_singular),
        ("application_plural", &vocabulary.application_plural),
        ("opportunity_singular", &vocabulary.opportunity_singular),
        ("opportunity_plural", &vocabulary.opportunity_plural),
        ("requirement_plural", &vocabulary.requirement_plural),
        ("evidence_plural", &vocabulary.evidence_plural),
        ("deliverable_plural", &vocabulary.deliverable_plural),
    ]
}

fn validate_vocabulary_placeholders(
    manifest: &WorkflowPackManifest,
    violations: &mut Vec<ContractViolation>,
) {
    let Some(default_vocabulary) = manifest.locales.get(&manifest.default_locale) else {
        return;
    };
    let default_fields = vocabulary_fields(default_vocabulary);
    let default_signatures = default_fields
        .iter()
        .map(|(field, value)| {
            validate_placeholder_text(
                value,
                &format!("/locales/{}/{field}", manifest.default_locale),
                violations,
            )
        })
        .collect::<Vec<_>>();
    for (locale, vocabulary) in &manifest.locales {
        if locale == &manifest.default_locale {
            continue;
        }
        for (index, (field, value)) in vocabulary_fields(vocabulary).into_iter().enumerate() {
            let pointer = format!("/locales/{locale}/{field}");
            let signature = validate_placeholder_text(value, &pointer, violations);
            if let (Some(expected), Some(actual)) = (&default_signatures[index], signature)
                && expected != &actual
            {
                violations.push(ContractViolation::new(
                    "workflow_pack.placeholder_mismatch",
                    pointer,
                    "localized variants must preserve the default locale placeholder names and counts",
                ));
            }
        }
    }
}

fn validate_localized_text(
    text: &WorkflowPackLocalizedText,
    pointer: &str,
    default_locale: &WorkflowPackLocaleId,
    locales: &BTreeMap<WorkflowPackLocaleId, WorkflowPackVocabulary>,
    violations: &mut Vec<ContractViolation>,
) {
    if text.0.is_empty() || text.0.len() > locales.len() || !text.0.contains_key(default_locale) {
        violations.push(ContractViolation::new(
            "workflow_pack.localized_text_incomplete",
            pointer,
            "localized text must include the default locale and only declared locales",
        ));
    }
    for (locale, value) in &text.0 {
        if !locales.contains_key(locale) {
            violations.push(ContractViolation::new(
                "workflow_pack.localized_text_locale_unknown",
                format!("{pointer}/{locale}"),
                "localized text references an undeclared locale",
            ));
        }
        required_bounded_text(value, &format!("{pointer}/{locale}"), 512, violations);
    }
    let signatures = text
        .0
        .iter()
        .filter_map(|(locale, value)| {
            validate_placeholder_text(value, &format!("{pointer}/{locale}"), violations)
                .map(|signature| (locale, signature))
        })
        .collect::<BTreeMap<_, _>>();
    if let Some(expected) = signatures.get(default_locale) {
        for (locale, actual) in &signatures {
            if *locale != default_locale && actual != expected {
                violations.push(ContractViolation::new(
                    "workflow_pack.placeholder_mismatch",
                    format!("{pointer}/{locale}"),
                    "localized variants must preserve the default locale placeholder names and counts",
                ));
            }
        }
    }
}

fn validate_application_fields(
    manifest: &WorkflowPackManifest,
    violations: &mut Vec<ContractViolation>,
) {
    validate_fields(
        &manifest.application.opportunity_fields,
        "/application/opportunity_fields",
        &manifest.default_locale,
        &manifest.locales,
        violations,
    );
    validate_fields(
        &manifest.application.application_fields,
        "/application/application_fields",
        &manifest.default_locale,
        &manifest.locales,
        violations,
    );
    let total = manifest.application.opportunity_fields.len()
        + manifest.application.application_fields.len();
    if total == 0 || total > 128 {
        violations.push(ContractViolation::new(
            "workflow_pack.application_field_count_invalid",
            "/application",
            "application metadata must define between 1 and 128 fields",
        ));
    }
}

fn validate_fields(
    fields: &[WorkflowPackFieldDefinition],
    pointer: &str,
    default_locale: &WorkflowPackLocaleId,
    locales: &BTreeMap<WorkflowPackLocaleId, WorkflowPackVocabulary>,
    violations: &mut Vec<ContractViolation>,
) {
    if fields.len() > 128 {
        violations.push(ContractViolation::new(
            "workflow_pack.field_count_invalid",
            pointer,
            "a field collection cannot exceed 128 items",
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, field) in fields.iter().enumerate() {
        let field_pointer = format!("{pointer}/{index}");
        if !ids.insert(&field.id) {
            violations.push(ContractViolation::new(
                "workflow_pack.field_id_duplicate",
                format!("{field_pointer}/id"),
                "field IDs must be unique within their collection",
            ));
        }
        validate_localized_text(
            &field.labels,
            &format!("{field_pointer}/labels"),
            default_locale,
            locales,
            violations,
        );
        let choice = field.field_type == WorkflowPackFieldType::Choice;
        if (choice && (field.options.is_empty() || field.options.len() > 64))
            || (!choice && !field.options.is_empty())
        {
            violations.push(ContractViolation::new(
                "workflow_pack.field_options_invalid",
                format!("{field_pointer}/options"),
                "choice fields require 1-64 options and other field types require none",
            ));
        }
        let mut option_ids = BTreeSet::new();
        for (option_index, option) in field.options.iter().enumerate() {
            if !option_ids.insert(&option.id) {
                violations.push(ContractViolation::new(
                    "workflow_pack.field_option_id_duplicate",
                    format!("{field_pointer}/options/{option_index}/id"),
                    "field option IDs must be unique",
                ));
            }
            validate_localized_text(
                &option.labels,
                &format!("{field_pointer}/options/{option_index}/labels"),
                default_locale,
                locales,
                violations,
            );
        }
    }
}

fn validate_taxonomy(
    taxonomy: &WorkflowPackTaxonomy,
    pointer: &str,
    default_locale: &WorkflowPackLocaleId,
    locales: &BTreeMap<WorkflowPackLocaleId, WorkflowPackVocabulary>,
    violations: &mut Vec<ContractViolation>,
) {
    if taxonomy.categories.is_empty() || taxonomy.categories.len() > 64 {
        violations.push(ContractViolation::new(
            "workflow_pack.category_count_invalid",
            format!("{pointer}/categories"),
            "a taxonomy must define between 1 and 64 categories",
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, category) in taxonomy.categories.iter().enumerate() {
        let category_pointer = format!("{pointer}/categories/{index}");
        if !ids.insert(&category.id) {
            violations.push(ContractViolation::new(
                "workflow_pack.category_id_duplicate",
                format!("{category_pointer}/id"),
                "category IDs must be unique within their taxonomy",
            ));
        }
        validate_localized_text(
            &category.labels,
            &format!("{category_pointer}/labels"),
            default_locale,
            locales,
            violations,
        );
        validate_fields(
            &category.fields,
            &format!("{category_pointer}/fields"),
            default_locale,
            locales,
            violations,
        );
    }
}

fn validate_capabilities(
    capabilities: &WorkflowPackCapabilities,
    violations: &mut Vec<ContractViolation>,
) {
    for (name, values) in [
        ("intake_adapters", &capabilities.intake_adapters),
        ("renderers", &capabilities.renderers),
        ("validators", &capabilities.validators),
    ] {
        if values.len() > 64 {
            violations.push(ContractViolation::new(
                "workflow_pack.capability_count_invalid",
                format!("/capabilities/{name}"),
                "a capability collection cannot exceed 64 items",
            ));
        }
        let mut unique = BTreeSet::new();
        for (index, value) in values.iter().enumerate() {
            if !unique.insert(value) {
                violations.push(ContractViolation::new(
                    "workflow_pack.capability_duplicate",
                    format!("/capabilities/{name}/{index}"),
                    "capability references must be unique",
                ));
            }
        }
    }
}

fn validate_validators(manifest: &WorkflowPackManifest, violations: &mut Vec<ContractViolation>) {
    if manifest.validation.definitions.is_empty() || manifest.validation.definitions.len() > 128 {
        violations.push(ContractViolation::new(
            "workflow_pack.validator_count_invalid",
            "/validation/definitions",
            "a workflow pack must define between 1 and 128 validator instances",
        ));
    }
    let selected = manifest
        .capabilities
        .validators
        .iter()
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for (index, validator) in manifest.validation.definitions.iter().enumerate() {
        if !ids.insert(&validator.id) {
            violations.push(ContractViolation::new(
                "workflow_pack.validator_id_duplicate",
                format!("/validation/definitions/{index}/id"),
                "validator instance IDs must be unique",
            ));
        }
        if !selected.contains(&validator.capability) {
            violations.push(ContractViolation::new(
                "workflow_pack.validator_capability_unselected",
                format!("/validation/definitions/{index}/capability"),
                "validator instance must reference a selected validator capability",
            ));
        }
        if validator.parameters.len() > 64 {
            violations.push(ContractViolation::new(
                "workflow_pack.validator_parameters_too_many",
                format!("/validation/definitions/{index}/parameters"),
                "validator parameters cannot exceed 64 entries",
            ));
        }
    }
    if manifest.validation.readiness.is_empty() || manifest.validation.readiness.len() > 64 {
        violations.push(ContractViolation::new(
            "workflow_pack.readiness_count_invalid",
            "/validation/readiness",
            "readiness must reference between 1 and 64 validator instances",
        ));
    }
    let mut readiness = BTreeSet::new();
    for (index, validator_id) in manifest.validation.readiness.iter().enumerate() {
        if !ids.contains(validator_id) {
            violations.push(ContractViolation::new(
                "workflow_pack.readiness_validator_unknown",
                format!("/validation/readiness/{index}"),
                "readiness references an unknown validator instance",
            ));
        }
        if !readiness.insert(validator_id) {
            violations.push(ContractViolation::new(
                "workflow_pack.readiness_validator_duplicate",
                format!("/validation/readiness/{index}"),
                "readiness validator references must be unique",
            ));
        }
    }
}

fn validate_resources(manifest: &WorkflowPackManifest, violations: &mut Vec<ContractViolation>) {
    if manifest.resources.len() > WORKFLOW_PACK_MAX_RESOURCES {
        violations.push(ContractViolation::new(
            "workflow_pack.resource_count_invalid",
            "/resources",
            format!(
                "a workflow pack cannot declare more than {WORKFLOW_PACK_MAX_RESOURCES} resources"
            ),
        ));
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut total = 0_u64;
    for (index, resource) in manifest.resources.iter().enumerate() {
        if !ids.insert(&resource.id) {
            violations.push(ContractViolation::new(
                "workflow_pack.resource_id_duplicate",
                format!("/resources/{index}/id"),
                "resource IDs must be unique",
            ));
        }
        if !paths.insert(resource.path.as_str()) {
            violations.push(ContractViolation::new(
                "workflow_pack.resource_path_duplicate",
                format!("/resources/{index}/path"),
                "resource paths must be unique",
            ));
        }
        if resource.size_bytes == 0 || resource.size_bytes > WORKFLOW_PACK_MAX_RESOURCE_BYTES {
            violations.push(ContractViolation::new(
                "workflow_pack.resource_size_invalid",
                format!("/resources/{index}/size_bytes"),
                format!(
                    "resource size must be between 1 and {WORKFLOW_PACK_MAX_RESOURCE_BYTES} bytes"
                ),
            ));
        }
        total = total.saturating_add(resource.size_bytes);
        let lower = resource.path.as_str().to_ascii_lowercase();
        if [
            ".sh", ".bash", ".zsh", ".ps1", ".bat", ".cmd", ".exe", ".dll", ".dylib", ".so",
            ".wasm", ".js", ".mjs", ".cjs",
        ]
        .iter()
        .any(|extension| lower.ends_with(extension))
        {
            violations.push(ContractViolation::new(
                "workflow_pack.resource_executable_rejected",
                format!("/resources/{index}/path"),
                "workflow packs cannot declare executable or script resources",
            ));
        }
    }
    if total > WORKFLOW_PACK_MAX_TOTAL_RESOURCE_BYTES {
        violations.push(ContractViolation::new(
            "workflow_pack.resource_total_too_large",
            "/resources",
            format!(
                "declared resources exceed the {WORKFLOW_PACK_MAX_TOTAL_RESOURCE_BYTES}-byte pack limit"
            ),
        ));
    }
}

fn validate_deliverables(manifest: &WorkflowPackManifest, violations: &mut Vec<ContractViolation>) {
    if manifest.deliverables.kinds.is_empty()
        || manifest.deliverables.kinds.len() > WORKFLOW_PACK_MAX_DELIVERABLE_KINDS
    {
        violations.push(ContractViolation::new(
            "workflow_pack.deliverable_count_invalid",
            "/deliverables/kinds",
            format!(
                "a workflow pack must define between 1 and {WORKFLOW_PACK_MAX_DELIVERABLE_KINDS} Deliverable kinds"
            ),
        ));
    }
    let resource_templates = manifest
        .resources
        .iter()
        .filter(|resource| resource.kind == WorkflowPackResourceKind::Template)
        .map(|resource| resource.path.as_str())
        .collect::<BTreeSet<_>>();
    let renderers = manifest
        .capabilities
        .renderers
        .iter()
        .collect::<BTreeSet<_>>();
    let validator_ids = manifest
        .validation
        .definitions
        .iter()
        .map(|validator| &validator.id)
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for (index, deliverable) in manifest.deliverables.kinds.iter().enumerate() {
        if !ids.insert(&deliverable.id) {
            violations.push(ContractViolation::new(
                "workflow_pack.deliverable_id_duplicate",
                format!("/deliverables/kinds/{index}/id"),
                "Deliverable kind IDs must be unique",
            ));
        }
        validate_localized_text(
            &deliverable.labels,
            &format!("/deliverables/kinds/{index}/labels"),
            &manifest.default_locale,
            &manifest.locales,
            violations,
        );
        if deliverable.maximum == 0
            || deliverable.minimum > deliverable.maximum
            || deliverable.maximum > WORKFLOW_PACK_MAX_DELIVERABLE_CARDINALITY
        {
            violations.push(ContractViolation::new(
                "workflow_pack.deliverable_cardinality_invalid",
                format!("/deliverables/kinds/{index}"),
                format!(
                    "Deliverable cardinality must satisfy 0 <= minimum <= maximum <= {WORKFLOW_PACK_MAX_DELIVERABLE_CARDINALITY} and maximum > 0"
                ),
            ));
        }
        if let Some(template) = &deliverable.template
            && !resource_templates.contains(template.as_str())
        {
            violations.push(ContractViolation::new(
                "workflow_pack.deliverable_template_unknown",
                format!("/deliverables/kinds/{index}/template"),
                "Deliverable template must reference a declared template resource",
            ));
        }
        if let Some(renderer) = &deliverable.renderer
            && !renderers.contains(renderer)
        {
            violations.push(ContractViolation::new(
                "workflow_pack.deliverable_renderer_unselected",
                format!("/deliverables/kinds/{index}/renderer"),
                "Deliverable renderer must reference a selected renderer capability",
            ));
        }
        let mut selected_validators = BTreeSet::new();
        for (validator_index, validator) in deliverable.validators.iter().enumerate() {
            if !validator_ids.contains(validator) {
                violations.push(ContractViolation::new(
                    "workflow_pack.deliverable_validator_unknown",
                    format!("/deliverables/kinds/{index}/validators/{validator_index}"),
                    "Deliverable references an unknown validator instance",
                ));
            }
            if !selected_validators.insert(validator) {
                violations.push(ContractViolation::new(
                    "workflow_pack.deliverable_validator_duplicate",
                    format!("/deliverables/kinds/{index}/validators/{validator_index}"),
                    "Deliverable validator references must be unique",
                ));
            }
        }
    }
}

fn validate_workflow(manifest: &WorkflowPackManifest, violations: &mut Vec<ContractViolation>) {
    let stages = &manifest.workflow.stages;
    if stages.is_empty() || stages.len() > WORKFLOW_PACK_MAX_STAGES {
        violations.push(ContractViolation::new(
            "workflow_pack.stage_count_invalid",
            "/workflow/stages",
            format!("a workflow pack must define between 1 and {WORKFLOW_PACK_MAX_STAGES} stages"),
        ));
        return;
    }
    let mut by_id = BTreeMap::new();
    for (index, stage) in stages.iter().enumerate() {
        if by_id.insert(&stage.id, stage).is_some() {
            violations.push(ContractViolation::new(
                "workflow_pack.stage_id_duplicate",
                format!("/workflow/stages/{index}/id"),
                "stage IDs must be unique",
            ));
        }
        validate_localized_text(
            &stage.labels,
            &format!("/workflow/stages/{index}/labels"),
            &manifest.default_locale,
            &manifest.locales,
            violations,
        );
        if stage.execution_modes.is_empty() || stage.execution_modes.len() > 5 {
            violations.push(ContractViolation::new(
                "workflow_pack.stage_execution_modes_invalid",
                format!("/workflow/stages/{index}/execution_modes"),
                "a stage must define between 1 and 5 execution modes",
            ));
        }
        let mut modes = Vec::new();
        for (mode_index, mode) in stage.execution_modes.iter().enumerate() {
            if modes.contains(mode) {
                violations.push(ContractViolation::new(
                    "workflow_pack.stage_execution_mode_duplicate",
                    format!("/workflow/stages/{index}/execution_modes/{mode_index}"),
                    "stage execution modes must be unique",
                ));
            } else {
                modes.push(*mode);
            }
        }
    }
    for (index, stage) in stages.iter().enumerate() {
        let mut dependencies = BTreeSet::new();
        for (dependency_index, dependency) in stage.depends_on.iter().enumerate() {
            if dependency == &stage.id || !by_id.contains_key(dependency) {
                violations.push(ContractViolation::new(
                    "workflow_pack.stage_dependency_invalid",
                    format!("/workflow/stages/{index}/depends_on/{dependency_index}"),
                    "stage dependency must reference a different declared stage",
                ));
            }
            if !dependencies.insert(dependency) {
                violations.push(ContractViolation::new(
                    "workflow_pack.stage_dependency_duplicate",
                    format!("/workflow/stages/{index}/depends_on/{dependency_index}"),
                    "stage dependencies must be unique",
                ));
            }
        }
    }
    if !by_id.contains_key(&manifest.workflow.terminal_stage) {
        violations.push(ContractViolation::new(
            "workflow_pack.terminal_stage_unknown",
            "/workflow/terminal_stage",
            "terminal stage must reference a declared stage",
        ));
        return;
    }
    let mut states = BTreeMap::new();
    if has_stage_cycle(&manifest.workflow.terminal_stage, &by_id, &mut states)
        || stages
            .iter()
            .any(|stage| has_stage_cycle(&stage.id, &by_id, &mut states))
    {
        violations.push(ContractViolation::new(
            "workflow_pack.stage_cycle",
            "/workflow/stages",
            "workflow stage dependencies must be acyclic",
        ));
        return;
    }
    let mut ancestors = BTreeSet::new();
    collect_stage_ancestors(&manifest.workflow.terminal_stage, &by_id, &mut ancestors);
    if ancestors.len() != by_id.len() {
        violations.push(ContractViolation::new(
            "workflow_pack.stage_not_terminal_reachable",
            "/workflow/terminal_stage",
            "every stage must contribute to the declared terminal stage",
        ));
    }
}

fn has_stage_cycle<'a>(
    id: &'a WorkflowPackItemId,
    stages: &BTreeMap<&'a WorkflowPackItemId, &'a WorkflowPackStageDefinition>,
    states: &mut BTreeMap<&'a WorkflowPackItemId, u8>,
) -> bool {
    match states.get(id) {
        Some(1) => return true,
        Some(2) => return false,
        _ => {}
    }
    states.insert(id, 1);
    if let Some(stage) = stages.get(id)
        && stage
            .depends_on
            .iter()
            .any(|dependency| has_stage_cycle(dependency, stages, states))
    {
        return true;
    }
    states.insert(id, 2);
    false
}

fn collect_stage_ancestors<'a>(
    id: &'a WorkflowPackItemId,
    stages: &BTreeMap<&'a WorkflowPackItemId, &'a WorkflowPackStageDefinition>,
    ancestors: &mut BTreeSet<&'a WorkflowPackItemId>,
) {
    if !ancestors.insert(id) {
        return;
    }
    if let Some(stage) = stages.get(id) {
        for dependency in &stage.depends_on {
            collect_stage_ancestors(dependency, stages, ancestors);
        }
    }
}

fn validate_migrations(manifest: &WorkflowPackManifest, violations: &mut Vec<ContractViolation>) {
    if manifest.migrations.len() > 64 {
        violations.push(ContractViolation::new(
            "workflow_pack.migration_count_invalid",
            "/migrations",
            "a workflow pack cannot declare more than 64 predecessor migrations",
        ));
    }
    let mut versions = BTreeSet::new();
    for (index, migration) in manifest.migrations.iter().enumerate() {
        if migration.from_version == manifest.version {
            violations.push(ContractViolation::new(
                "workflow_pack.migration_same_version",
                format!("/migrations/{index}/from_version"),
                "migration predecessor must differ from the current pack version",
            ));
        }
        if !versions.insert(&migration.from_version) {
            violations.push(ContractViolation::new(
                "workflow_pack.migration_version_duplicate",
                format!("/migrations/{index}/from_version"),
                "only one migration may exist for a predecessor version",
            ));
        }
        if migration.mappings.len() > 512 {
            violations.push(ContractViolation::new(
                "workflow_pack.migration_mapping_count_invalid",
                format!("/migrations/{index}/mappings"),
                "a migration cannot contain more than 512 ID mappings",
            ));
        }
        let mut mappings = BTreeSet::new();
        for (mapping_index, mapping) in migration.mappings.iter().enumerate() {
            if mapping.from == mapping.to {
                violations.push(ContractViolation::new(
                    "workflow_pack.migration_mapping_identity",
                    format!("/migrations/{index}/mappings/{mapping_index}"),
                    "migration mappings must change the stable ID",
                ));
            }
            let key = (migration_kind_name(mapping.kind), &mapping.from);
            if !mappings.insert(key) {
                violations.push(ContractViolation::new(
                    "workflow_pack.migration_mapping_duplicate",
                    format!("/migrations/{index}/mappings/{mapping_index}"),
                    "migration source IDs must be unique per mapping kind",
                ));
            }
        }
    }
}

const fn migration_kind_name(kind: WorkflowPackMigrationKind) -> &'static str {
    match kind {
        WorkflowPackMigrationKind::Stage => "stage",
        WorkflowPackMigrationKind::RequirementCategory => "requirement-category",
        WorkflowPackMigrationKind::EvidenceCategory => "evidence-category",
        WorkflowPackMigrationKind::Deliverable => "deliverable",
        WorkflowPackMigrationKind::Resource => "resource",
        WorkflowPackMigrationKind::Validator => "validator",
        WorkflowPackMigrationKind::Field => "field",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use super::{
        CandidateValidationError, DeliverableKindId, ExecutionMode, SafeRelativePath,
        SemanticVersion, Sha256Digest, StageId, WorkflowPackApplicationDefinition,
        WorkflowPackCapabilities, WorkflowPackCategoryDefinition, WorkflowPackCompatibility,
        WorkflowPackDeliverableCatalog, WorkflowPackDeliverableDefinition,
        WorkflowPackFieldDefinition, WorkflowPackFieldType, WorkflowPackFormat, WorkflowPackId,
        WorkflowPackItemId, WorkflowPackLocaleId, WorkflowPackLocalizedText, WorkflowPackManifest,
        WorkflowPackPublisher, WorkflowPackPublisherId, WorkflowPackResource,
        WorkflowPackResourceKind, WorkflowPackStageDefinition, WorkflowPackStageOutput,
        WorkflowPackTaxonomy, WorkflowPackValidationPolicy, WorkflowPackValidatorDefinition,
        WorkflowPackVocabulary, WorkflowPackWorkflowDefinition, validate_workflow_pack_manifest,
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

    fn valid_manifest() -> WorkflowPackManifest {
        let validator_capability =
            super::WorkflowPackCapabilityId::try_new("canisend.validator.evidence-traceability")
                .expect("validator capability");
        let renderer =
            super::WorkflowPackCapabilityId::try_new("canisend.renderer.typst").expect("renderer");
        WorkflowPackManifest {
            format: WorkflowPackFormat::V1,
            id: WorkflowPackId::try_new("org.canisend.test-pack").expect("pack ID"),
            version: SemanticVersion::try_new("1.0.0").expect("version"),
            schema_version: SemanticVersion::try_new("1.0.0").expect("schema version"),
            publisher: WorkflowPackPublisher {
                id: WorkflowPackPublisherId::try_new("org.canisend").expect("publisher ID"),
                name: "CanISend".to_owned(),
                homepage: Some("https://canisend.dev".to_owned()),
            },
            compatibility: WorkflowPackCompatibility {
                kernel: ">=1.0.0-alpha.6, <2.0.0".to_owned(),
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
                        id: item("prepare"),
                        labels: labels("Prepare"),
                        depends_on: vec![item("intake")],
                        output: WorkflowPackStageOutput::Deliverables,
                        execution_modes: vec![ExecutionMode::HostAgent],
                    },
                    WorkflowPackStageDefinition {
                        id: item("export"),
                        labels: labels("Export"),
                        depends_on: vec![item("prepare")],
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
                    template: Some(
                        SafeRelativePath::try_new("templates/statement.typ")
                            .expect("template path"),
                    ),
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
                path: SafeRelativePath::try_new("templates/statement.typ").expect("resource path"),
                version: SemanticVersion::try_new("1.0.0").expect("resource version"),
                size_bytes: 128,
                sha256: Sha256Digest::try_new("a".repeat(64)).expect("resource digest"),
            }],
            migrations: Vec::new(),
            content_digest: Sha256Digest::try_new("b".repeat(64)).expect("content digest"),
        }
    }

    fn valid_value() -> Value {
        serde_json::to_value(valid_manifest()).expect("manifest JSON")
    }

    fn semantic_codes(value: &Value) -> Vec<String> {
        match validate_workflow_pack_manifest(value).expect_err("manifest must fail") {
            CandidateValidationError::Structural(violations)
            | CandidateValidationError::Semantic(violations) => violations
                .into_iter()
                .map(|violation| violation.code)
                .collect(),
        }
    }

    #[test]
    fn valid_manifest_round_trips_and_validates() {
        let value = valid_value();
        let validated = validate_workflow_pack_manifest(&value).expect("valid manifest");
        assert_eq!(validated, valid_manifest());
        assert_eq!(validated.id.as_str(), "org.canisend.test-pack");
    }

    #[test]
    fn stage_id_is_pack_qualified_canonical_and_strongly_validated() {
        let pack_id = WorkflowPackId::try_new("org.canisend.test-pack").expect("pack ID");
        let local_id = item("review");
        let stage_id = StageId::from_parts(&pack_id, &local_id);
        assert_eq!(stage_id.as_str(), "org.canisend.test-pack:review");
        assert_eq!(stage_id.pack_id_str(), pack_id.as_str());
        assert_eq!(stage_id.local_id_str(), local_id.as_str());
        assert_eq!(
            serde_json::to_value(&stage_id).expect("stage ID JSON"),
            json!("org.canisend.test-pack:review")
        );
        assert_eq!(
            serde_json::from_value::<StageId>(json!("org.canisend.test-pack:review"))
                .expect("stage ID round trip"),
            stage_id
        );
        for invalid in [
            "review",
            "org.canisend.test-pack:",
            "org.canisend.test-pack:Review",
            "org.canisend.test-pack:review:extra",
            "org.canisend:review",
        ] {
            assert!(StageId::try_new(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn deliverable_kind_id_is_pack_qualified_and_cross_pack_distinct() {
        let first_pack = WorkflowPackId::try_new("org.canisend.first-pack").expect("first pack");
        let second_pack = WorkflowPackId::try_new("org.canisend.second-pack").expect("second pack");
        let local_id = item("statement");
        let first = DeliverableKindId::from_parts(&first_pack, &local_id);
        let second = DeliverableKindId::from_parts(&second_pack, &local_id);
        assert_eq!(first.as_str(), "org.canisend.first-pack:statement");
        assert_eq!(first.pack_id_str(), first_pack.as_str());
        assert_eq!(first.local_id_str(), local_id.as_str());
        assert_ne!(first, second);
        assert_eq!(
            serde_json::from_value::<DeliverableKindId>(
                serde_json::to_value(&first).expect("Deliverable kind JSON")
            )
            .expect("Deliverable kind round trip"),
            first
        );
        for invalid in [
            "statement",
            "org.canisend.first-pack:",
            "org.canisend.first-pack:Statement",
            "org.canisend.first-pack:statement:extra",
        ] {
            assert!(
                DeliverableKindId::try_new(invalid).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn structural_validation_rejects_unknown_fields_and_resource_kinds() {
        let mut value = valid_value();
        value["unexpected"] = json!(true);
        assert!(matches!(
            validate_workflow_pack_manifest(&value),
            Err(CandidateValidationError::Structural(_))
        ));

        let mut value = valid_value();
        value["resources"][0]["kind"] = json!("executable");
        assert!(matches!(
            validate_workflow_pack_manifest(&value),
            Err(CandidateValidationError::Structural(_))
        ));

        for (pointer, invalid) in [
            ("/workflow/stages/0/output", json!("database-write")),
            (
                "/workflow/stages/0/execution_modes/0",
                json!("arbitrary-process"),
            ),
        ] {
            let mut value = valid_value();
            *value.pointer_mut(pointer).expect("stage fixture pointer") = invalid;
            assert!(
                matches!(
                    validate_workflow_pack_manifest(&value),
                    Err(CandidateValidationError::Structural(_))
                ),
                "accepted invalid stage field at {pointer}",
            );
        }
    }

    #[test]
    fn primitive_validation_rejects_invalid_identity_path_and_digest() {
        for (pointer, invalid) in [
            ("/id", json!("Academic Pack")),
            ("/resources/0/path", json!("../escape.typ")),
            ("/content_digest", json!("ABC")),
        ] {
            let mut value = valid_value();
            let target = value.pointer_mut(pointer).expect("fixture pointer");
            *target = invalid;
            assert!(
                semantic_codes(&value)
                    .iter()
                    .any(|code| code == "candidate.primitive_invalid"),
                "missing primitive failure for {pointer}"
            );
        }
    }

    #[test]
    fn semantic_validation_rejects_compatibility_locales_and_capabilities() {
        let mut value = valid_value();
        value["compatibility"]["kernel"] = json!("not a requirement");
        value["default_locale"] = json!("zh-Hans");
        value["deliverables"]["kinds"][0]["renderer"] = json!("canisend.renderer.missing");
        let codes = semantic_codes(&value);
        for expected in [
            "workflow_pack.compatibility_invalid",
            "workflow_pack.default_locale_missing",
            "workflow_pack.deliverable_renderer_unselected",
        ] {
            assert!(
                codes.iter().any(|code| code == expected),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn localization_validation_preserves_placeholders_and_safe_unicode() {
        let mut valid = valid_value();
        valid["locales"]["zh-Hans"] = json!({
            "application_singular": "申请",
            "application_plural": "申请",
            "opportunity_singular": "机会",
            "opportunity_plural": "机会",
            "requirement_plural": "要求",
            "evidence_plural": "证据",
            "deliverable_plural": "交付材料"
        });
        valid["locales"]["ar"] = json!({
            "application_singular": "طلب",
            "application_plural": "طلبات",
            "opportunity_singular": "فرصة",
            "opportunity_plural": "فرص",
            "requirement_plural": "متطلبات",
            "evidence_plural": "أدلة",
            "deliverable_plural": "مخرجات"
        });
        valid["application"]["opportunity_fields"][0]["labels"] = json!({
            "en": "Role for {organization} — A\u{0301}",
            "zh-Hans": "{organization} 的职位"
        });
        validate_workflow_pack_manifest(&valid).expect("safe Unicode and matching placeholders");

        let mut mismatch = valid.clone();
        mismatch["application"]["opportunity_fields"][0]["labels"]["zh-Hans"] =
            json!("{institution} 的职位");
        assert!(
            semantic_codes(&mismatch)
                .iter()
                .any(|code| code == "workflow_pack.placeholder_mismatch")
        );

        let mut count_mismatch = valid.clone();
        count_mismatch["application"]["opportunity_fields"][0]["labels"]["zh-Hans"] =
            json!("{organization} 的 {organization} 职位");
        assert!(
            semantic_codes(&count_mismatch)
                .iter()
                .any(|code| code == "workflow_pack.placeholder_mismatch")
        );

        let mut malformed = valid.clone();
        malformed["application"]["opportunity_fields"][0]["labels"]["en"] =
            json!("Role for {Organization}");
        assert!(
            semantic_codes(&malformed)
                .iter()
                .any(|code| code == "workflow_pack.placeholder_syntax_invalid")
        );

        let mut escaped = valid.clone();
        escaped["application"]["opportunity_fields"][0]["labels"] = json!({
            "en": "Use {{literal}} for {organization}",
            "zh-Hans": "将 {{literal}} 用于 {organization}"
        });
        validate_workflow_pack_manifest(&escaped).expect("escaped braces");

        let mut bidi_override = valid;
        bidi_override["application"]["opportunity_fields"][0]["labels"]["en"] =
            json!("Role \u{202e}hidden");
        assert!(
            semantic_codes(&bidi_override)
                .iter()
                .any(|code| code == "workflow_pack.text_bidi_control_invalid")
        );
    }

    #[test]
    fn semantic_validation_rejects_cycles_disconnected_stages_and_unknown_templates() {
        let mut value = valid_value();
        value["workflow"]["stages"][0]["depends_on"] = json!(["export"]);
        value["deliverables"]["kinds"][0]["template"] = json!("templates/missing.typ");
        let codes = semantic_codes(&value);
        assert!(codes.iter().any(|code| code == "workflow_pack.stage_cycle"));
        assert!(
            codes
                .iter()
                .any(|code| code == "workflow_pack.deliverable_template_unknown")
        );

        let mut value = valid_value();
        value["workflow"]["stages"]
            .as_array_mut()
            .expect("stages")
            .push(json!({
                "id": "orphan",
                "labels": {"en": "Orphan"},
                "depends_on": [],
                "output": "none",
                "execution_modes": ["deterministic"]
            }));
        assert!(
            semantic_codes(&value)
                .iter()
                .any(|code| code == "workflow_pack.stage_not_terminal_reachable")
        );
    }

    #[test]
    fn semantic_validation_rejects_script_resources_and_oversized_declarations() {
        let mut value = valid_value();
        value["resources"][0]["path"] = json!("templates/run.js");
        value["resources"][0]["size_bytes"] = json!(9 * 1024 * 1024_u64);
        let codes = semantic_codes(&value);
        assert!(
            codes
                .iter()
                .any(|code| code == "workflow_pack.resource_executable_rejected")
        );
        assert!(
            codes
                .iter()
                .any(|code| code == "workflow_pack.resource_size_invalid")
        );
    }

    #[test]
    fn shape_validation_rejects_excessive_parameter_depth() {
        let mut nested = json!(true);
        for _ in 0..40 {
            nested = json!({"next": nested});
        }
        let mut value = valid_value();
        value["validation"]["definitions"][0]["parameters"]["nested"] = nested;
        assert!(
            semantic_codes(&value)
                .iter()
                .any(|code| code == "workflow_pack.manifest_shape_invalid")
        );
    }
}
