#![forbid(unsafe_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const AGENT_PROTOCOL: &str = "canisend.agent/v2";
pub const WORKSPACE_FORMAT: &str = "canisend.workspace/v2";
pub const RESOURCE_FORMAT: &str = "canisend.resources/v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AgentProtocolVersion {
    #[serde(rename = "canisend.agent/v2")]
    V2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityStatus {
    Available,
    Planned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub version: String,
    pub status: CapabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReference {
    pub kind: String,
    pub id: String,
    pub revision: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsentRequest {
    pub scope: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NextAction {
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilitiesData {
    pub product_version: String,
    pub protocol: String,
    pub workspace_format: String,
    pub resource_format: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionData {
    pub product: String,
    pub version: String,
    pub protocol: String,
    pub workspace_format: String,
    pub resource_format: String,
    pub rustc: String,
    pub target: String,
    pub git_revision: String,
}
