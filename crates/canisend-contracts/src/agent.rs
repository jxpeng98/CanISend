use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ActorKind, ArtifactKind, ArtifactReference, ConsentScope, EntityId, ExecutionMode,
    PrivacyClassification, Revision, SemanticVersion, Sha256Digest, UtcTimestamp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AgentProtocolVersion {
    #[serde(rename = "canisend.agent/v2")]
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityStatus {
    Available,
    Planned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub version: SemanticVersion,
    pub status: CapabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsentRequest {
    pub scope: ConsentScope,
    pub description: String,
    pub artifacts: Vec<ArtifactReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NextAction {
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ErrorCode {
    #[serde(rename = "input.invalid")]
    InputInvalid,
    #[serde(rename = "input.path_rejected")]
    InputPathRejected,
    #[serde(rename = "workspace.not_found")]
    WorkspaceNotFound,
    #[serde(rename = "workspace.conflict")]
    WorkspaceConflict,
    #[serde(rename = "job.not_found")]
    JobNotFound,
    #[serde(rename = "job.archived")]
    JobArchived,
    #[serde(rename = "discovery.source_not_found")]
    DiscoverySourceNotFound,
    #[serde(rename = "discovery.lead_not_found")]
    DiscoveryLeadNotFound,
    #[serde(rename = "discovery.conflict")]
    DiscoveryConflict,
    #[serde(rename = "pdf.encrypted")]
    PdfEncrypted,
    #[serde(rename = "pdf.malformed")]
    PdfMalformed,
    #[serde(rename = "pdf_text_unavailable")]
    PdfTextUnavailable,
    #[serde(rename = "resource.not_found")]
    ResourceNotFound,
    #[serde(rename = "resources.integrity_failed")]
    ResourcesIntegrityFailed,
    #[serde(rename = "schema.not_found")]
    SchemaNotFound,
    #[serde(rename = "candidate.schema_invalid")]
    CandidateSchemaInvalid,
    #[serde(rename = "candidate.semantic_invalid")]
    CandidateSemanticInvalid,
    #[serde(rename = "candidate.unknown_evidence")]
    CandidateUnknownEvidence,
    #[serde(rename = "task.not_found")]
    TaskNotFound,
    #[serde(rename = "task.stale")]
    TaskStale,
    #[serde(rename = "task.conflict")]
    TaskConflict,
    #[serde(rename = "consent.required")]
    ConsentRequired,
    #[serde(rename = "external.io_failed")]
    ExternalIoFailed,
    #[serde(rename = "provider.failed")]
    ProviderFailed,
    #[serde(rename = "internal.invariant_failed")]
    InternalInvariantFailed,
}

impl ErrorCode {
    pub const ALL: [Self; 25] = [
        Self::InputInvalid,
        Self::InputPathRejected,
        Self::WorkspaceNotFound,
        Self::WorkspaceConflict,
        Self::JobNotFound,
        Self::JobArchived,
        Self::DiscoverySourceNotFound,
        Self::DiscoveryLeadNotFound,
        Self::DiscoveryConflict,
        Self::PdfEncrypted,
        Self::PdfMalformed,
        Self::PdfTextUnavailable,
        Self::ResourceNotFound,
        Self::ResourcesIntegrityFailed,
        Self::SchemaNotFound,
        Self::CandidateSchemaInvalid,
        Self::CandidateSemanticInvalid,
        Self::CandidateUnknownEvidence,
        Self::TaskNotFound,
        Self::TaskStale,
        Self::TaskConflict,
        Self::ConsentRequired,
        Self::ExternalIoFailed,
        Self::ProviderFailed,
        Self::InternalInvariantFailed,
    ];

    #[must_use]
    pub const fn exit_class(self) -> ExitClass {
        match self {
            Self::WorkspaceNotFound
            | Self::WorkspaceConflict
            | Self::JobNotFound
            | Self::JobArchived
            | Self::DiscoverySourceNotFound
            | Self::DiscoveryLeadNotFound
            | Self::DiscoveryConflict
            | Self::TaskNotFound
            | Self::TaskStale
            | Self::TaskConflict => ExitClass::Conflict,
            Self::ExternalIoFailed | Self::ProviderFailed => ExitClass::ExternalIo,
            Self::ResourcesIntegrityFailed | Self::InternalInvariantFailed => ExitClass::Internal,
            Self::InputInvalid
            | Self::InputPathRejected
            | Self::PdfEncrypted
            | Self::PdfMalformed
            | Self::PdfTextUnavailable
            | Self::ResourceNotFound
            | Self::SchemaNotFound
            | Self::CandidateSchemaInvalid
            | Self::CandidateSemanticInvalid
            | Self::CandidateUnknownEvidence
            | Self::ConsentRequired => ExitClass::Validation,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputInvalid => "input.invalid",
            Self::InputPathRejected => "input.path_rejected",
            Self::WorkspaceNotFound => "workspace.not_found",
            Self::WorkspaceConflict => "workspace.conflict",
            Self::JobNotFound => "job.not_found",
            Self::JobArchived => "job.archived",
            Self::DiscoverySourceNotFound => "discovery.source_not_found",
            Self::DiscoveryLeadNotFound => "discovery.lead_not_found",
            Self::DiscoveryConflict => "discovery.conflict",
            Self::PdfEncrypted => "pdf.encrypted",
            Self::PdfMalformed => "pdf.malformed",
            Self::PdfTextUnavailable => "pdf_text_unavailable",
            Self::ResourceNotFound => "resource.not_found",
            Self::ResourcesIntegrityFailed => "resources.integrity_failed",
            Self::SchemaNotFound => "schema.not_found",
            Self::CandidateSchemaInvalid => "candidate.schema_invalid",
            Self::CandidateSemanticInvalid => "candidate.semantic_invalid",
            Self::CandidateUnknownEvidence => "candidate.unknown_evidence",
            Self::TaskNotFound => "task.not_found",
            Self::TaskStale => "task.stale",
            Self::TaskConflict => "task.conflict",
            Self::ConsentRequired => "consent.required",
            Self::ExternalIoFailed => "external.io_failed",
            Self::ProviderFailed => "provider.failed",
            Self::InternalInvariantFailed => "internal.invariant_failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitClass {
    Success = 0,
    CliUsage = 2,
    Validation = 3,
    Conflict = 4,
    ExternalIo = 5,
    Internal = 6,
}

impl ExitClass {
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<NextAction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentResponse {
    pub protocol: AgentProtocolVersion,
    pub operation: String,
    pub ok: bool,
    pub status: String,
    pub data: Option<Value>,
    pub artifacts: Vec<ArtifactReference>,
    pub required_consents: Vec<ConsentRequest>,
    pub warnings: Vec<String>,
    pub next_actions: Vec<NextAction>,
    pub error: Option<AgentError>,
}

impl AgentResponse {
    #[must_use]
    pub fn success(operation: impl Into<String>, status: impl Into<String>, data: Value) -> Self {
        Self {
            protocol: AgentProtocolVersion::V2,
            operation: operation.into(),
            ok: true,
            status: status.into(),
            data: Some(data),
            artifacts: Vec::new(),
            required_consents: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
            error: None,
        }
    }

    #[must_use]
    pub fn failure(
        operation: impl Into<String>,
        status: impl Into<String>,
        error: AgentError,
    ) -> Self {
        Self {
            protocol: AgentProtocolVersion::V2,
            operation: operation.into(),
            ok: false,
            status: status.into(),
            data: None,
            artifacts: Vec::new(),
            required_consents: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
            error: Some(error),
        }
    }

    #[must_use]
    pub fn exit_class(&self) -> ExitClass {
        self.error
            .as_ref()
            .map_or(ExitClass::Success, |error| error.code.exit_class())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilitiesData {
    pub product_version: SemanticVersion,
    pub protocol: String,
    pub workspace_format: String,
    pub resource_format: String,
    pub capabilities: Vec<Capability>,
    pub error_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentContextData {
    pub product_version: SemanticVersion,
    pub actor: ActorKind,
    pub execution_mode: ExecutionMode,
    pub workspace_id: Option<EntityId>,
    pub active_job_id: Option<EntityId>,
    pub privacy: PrivacyClassification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaCatalogEntry {
    pub id: String,
    pub version: SemanticVersion,
    pub uri: String,
    pub resource_id: String,
    pub size: usize,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaCatalogData {
    pub schemas: Vec<SchemaCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceCatalogEntry {
    pub id: String,
    pub kind: String,
    pub version: SemanticVersion,
    pub size: usize,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceCatalogData {
    pub resources: Vec<ResourceCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionData {
    pub product: String,
    pub version: SemanticVersion,
    pub protocol: String,
    pub workspace_format: String,
    pub resource_format: String,
    pub rustc: String,
    pub target: String,
    pub git_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaReference {
    pub id: String,
    pub version: SemanticVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskLease {
    pub id: EntityId,
    pub expires_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskDescriptor {
    pub id: EntityId,
    pub operation: String,
    pub actor: ActorKind,
    pub execution_mode: ExecutionMode,
    pub input_artifacts: Vec<ArtifactReference>,
    pub allowed_output_kind: ArtifactKind,
    pub candidate_schema: SchemaReference,
    pub required_consents: Vec<ConsentRequest>,
    pub private_read_scope: Vec<ArtifactReference>,
    pub lease: TaskLease,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpectedInputRevision {
    pub artifact_id: EntityId,
    pub revision: Revision,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskCompletionRequest {
    pub task_id: EntityId,
    pub lease_id: EntityId,
    pub expected_inputs: Vec<ExpectedInputRevision>,
    pub candidate: Value,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{AgentError, AgentResponse, ErrorCode, ExitClass};

    #[test]
    fn error_registry_is_unique_and_has_exit_mapping() {
        let codes = ErrorCode::ALL
            .into_iter()
            .map(ErrorCode::as_str)
            .collect::<HashSet<_>>();
        assert_eq!(codes.len(), ErrorCode::ALL.len());
        assert_eq!(
            ErrorCode::CandidateSchemaInvalid.exit_class(),
            ExitClass::Validation
        );
        assert_eq!(ErrorCode::TaskStale.exit_class(), ExitClass::Conflict);
        assert_eq!(
            ErrorCode::ProviderFailed.exit_class(),
            ExitClass::ExternalIo
        );
    }

    #[test]
    fn failure_exit_class_comes_from_stable_error_code() {
        let response = AgentResponse::failure(
            "task.complete",
            "validation-failed",
            AgentError {
                code: ErrorCode::CandidateSemanticInvalid,
                message: "candidate failed semantic validation".to_owned(),
                retryable: true,
                details: None,
                remediation: None,
            },
        );
        assert_eq!(response.exit_class().code(), 3);
    }
}
