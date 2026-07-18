use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    AgentContextData, AgentResponse, ApplicationPlanCandidate, ApplicationPlanRecord,
    BackupManifestData, CapabilitiesData, CriteriaSetRecord, CriterionRecord, DiscoveryBatch,
    DiscoveryLeadRecord, DocumentCandidate, DocumentRecord, EvidenceCatalogRecord,
    EvidenceMatchProposalSet, EvidenceMatchRecord, EvidenceMatchSetRecord, EvidenceProposalSet,
    EvidenceRecord, FindingRecord, JobRecord, ParsedJobRecord, ProfileSourceRecord,
    ReadinessRecord, SourceRecord, TaskCompletionRequest, TaskDescriptor, VersionData,
    WorkflowStatusData, WorkspaceCheckData, WorkspaceStatusData,
};

pub const PUBLIC_SCHEMA_VERSION: &str = "2.0.0";
pub const PUBLIC_SCHEMA_BASE: &str = "https://schemas.canisend.dev/v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PublicSchemaId {
    AgentResponse,
    Capabilities,
    AgentContext,
    Version,
    TaskDescriptor,
    TaskCompletion,
    Job,
    Source,
    Evidence,
    Criterion,
    EvidenceMatch,
    ApplicationPlan,
    Document,
    DocumentCandidate,
    Finding,
    Readiness,
    WorkspaceStatus,
    WorkspaceCheck,
    BackupManifest,
    DiscoveryBatch,
    DiscoveryLead,
    WorkflowStatus,
    ParsedJob,
    CriteriaSet,
    ProfileSource,
    EvidenceProposals,
    EvidenceCatalog,
    EvidenceMatchProposals,
    EvidenceMatches,
    ApplicationPlanCandidate,
}

impl PublicSchemaId {
    pub const ALL: [Self; 30] = [
        Self::AgentResponse,
        Self::Capabilities,
        Self::AgentContext,
        Self::Version,
        Self::TaskDescriptor,
        Self::TaskCompletion,
        Self::Job,
        Self::Source,
        Self::Evidence,
        Self::Criterion,
        Self::EvidenceMatch,
        Self::ApplicationPlan,
        Self::Document,
        Self::DocumentCandidate,
        Self::Finding,
        Self::Readiness,
        Self::WorkspaceStatus,
        Self::WorkspaceCheck,
        Self::BackupManifest,
        Self::DiscoveryBatch,
        Self::DiscoveryLead,
        Self::WorkflowStatus,
        Self::ParsedJob,
        Self::CriteriaSet,
        Self::ProfileSource,
        Self::EvidenceProposals,
        Self::EvidenceCatalog,
        Self::EvidenceMatchProposals,
        Self::EvidenceMatches,
        Self::ApplicationPlanCandidate,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AgentResponse => "canisend.agent-response/v2",
            Self::Capabilities => "canisend.capabilities/v2",
            Self::AgentContext => "canisend.agent-context/v2",
            Self::Version => "canisend.version/v2",
            Self::TaskDescriptor => "canisend.task-descriptor/v2",
            Self::TaskCompletion => "canisend.task-completion/v2",
            Self::Job => "canisend.job/v2",
            Self::Source => "canisend.source/v2",
            Self::Evidence => "canisend.evidence/v2",
            Self::Criterion => "canisend.criterion/v2",
            Self::EvidenceMatch => "canisend.evidence-match/v2",
            Self::ApplicationPlan => "canisend.application-plan/v2",
            Self::Document => "canisend.document/v2",
            Self::DocumentCandidate => "canisend.document-candidate/v2",
            Self::Finding => "canisend.finding/v2",
            Self::Readiness => "canisend.readiness/v2",
            Self::WorkspaceStatus => "canisend.workspace-status/v2",
            Self::WorkspaceCheck => "canisend.workspace-check/v2",
            Self::BackupManifest => "canisend.backup-manifest/v2",
            Self::DiscoveryBatch => "canisend.discovery-batch/v2",
            Self::DiscoveryLead => "canisend.discovery-lead/v2",
            Self::WorkflowStatus => "canisend.workflow-status/v2",
            Self::ParsedJob => "canisend.parsed-job/v2",
            Self::CriteriaSet => "canisend.criteria/v2",
            Self::ProfileSource => "canisend.profile-source/v2",
            Self::EvidenceProposals => "canisend.evidence-proposals/v2",
            Self::EvidenceCatalog => "canisend.evidence-catalog/v2",
            Self::EvidenceMatchProposals => "canisend.evidence-match-proposals/v2",
            Self::EvidenceMatches => "canisend.evidence-matches/v2",
            Self::ApplicationPlanCandidate => "canisend.application-plan-candidate/v2",
        }
    }

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::AgentResponse => "agent-response",
            Self::Capabilities => "capabilities",
            Self::AgentContext => "agent-context",
            Self::Version => "version",
            Self::TaskDescriptor => "task-descriptor",
            Self::TaskCompletion => "task-completion",
            Self::Job => "job",
            Self::Source => "source",
            Self::Evidence => "evidence",
            Self::Criterion => "criterion",
            Self::EvidenceMatch => "evidence-match",
            Self::ApplicationPlan => "application-plan",
            Self::Document => "document",
            Self::DocumentCandidate => "document-candidate",
            Self::Finding => "finding",
            Self::Readiness => "readiness",
            Self::WorkspaceStatus => "workspace-status",
            Self::WorkspaceCheck => "workspace-check",
            Self::BackupManifest => "backup-manifest",
            Self::DiscoveryBatch => "discovery-batch",
            Self::DiscoveryLead => "discovery-lead",
            Self::WorkflowStatus => "workflow-status",
            Self::ParsedJob => "parsed-job",
            Self::CriteriaSet => "criteria",
            Self::ProfileSource => "profile-source",
            Self::EvidenceProposals => "evidence-proposals",
            Self::EvidenceCatalog => "evidence-catalog",
            Self::EvidenceMatchProposals => "evidence-match-proposals",
            Self::EvidenceMatches => "evidence-matches",
            Self::ApplicationPlanCandidate => "application-plan-candidate",
        }
    }

    #[must_use]
    pub fn file_name(self) -> String {
        format!("{}.schema.json", self.slug())
    }

    #[must_use]
    pub fn canonical_uri(self) -> String {
        format!("{PUBLIC_SCHEMA_BASE}/{}", self.file_name())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedSchema {
    pub id: PublicSchemaId,
    pub document: Value,
}

impl GeneratedSchema {
    #[must_use]
    pub fn canonical_json(&self) -> String {
        let mut output = serde_json::to_string_pretty(&sort_json(self.document.clone()))
            .expect("generated schema serializes");
        output.push('\n');
        output
    }
}

#[must_use]
pub fn generate_public_schemas() -> Vec<GeneratedSchema> {
    vec![
        generate::<AgentResponse>(PublicSchemaId::AgentResponse),
        generate::<CapabilitiesData>(PublicSchemaId::Capabilities),
        generate::<AgentContextData>(PublicSchemaId::AgentContext),
        generate::<VersionData>(PublicSchemaId::Version),
        generate::<TaskDescriptor>(PublicSchemaId::TaskDescriptor),
        generate::<TaskCompletionRequest>(PublicSchemaId::TaskCompletion),
        generate::<JobRecord>(PublicSchemaId::Job),
        generate::<SourceRecord>(PublicSchemaId::Source),
        generate::<EvidenceRecord>(PublicSchemaId::Evidence),
        generate::<CriterionRecord>(PublicSchemaId::Criterion),
        generate::<EvidenceMatchRecord>(PublicSchemaId::EvidenceMatch),
        generate::<ApplicationPlanRecord>(PublicSchemaId::ApplicationPlan),
        generate::<DocumentRecord>(PublicSchemaId::Document),
        generate::<DocumentCandidate>(PublicSchemaId::DocumentCandidate),
        generate::<FindingRecord>(PublicSchemaId::Finding),
        generate::<ReadinessRecord>(PublicSchemaId::Readiness),
        generate::<WorkspaceStatusData>(PublicSchemaId::WorkspaceStatus),
        generate::<WorkspaceCheckData>(PublicSchemaId::WorkspaceCheck),
        generate::<BackupManifestData>(PublicSchemaId::BackupManifest),
        generate::<DiscoveryBatch>(PublicSchemaId::DiscoveryBatch),
        generate::<DiscoveryLeadRecord>(PublicSchemaId::DiscoveryLead),
        generate::<WorkflowStatusData>(PublicSchemaId::WorkflowStatus),
        generate::<ParsedJobRecord>(PublicSchemaId::ParsedJob),
        generate::<CriteriaSetRecord>(PublicSchemaId::CriteriaSet),
        generate::<ProfileSourceRecord>(PublicSchemaId::ProfileSource),
        generate::<EvidenceProposalSet>(PublicSchemaId::EvidenceProposals),
        generate::<EvidenceCatalogRecord>(PublicSchemaId::EvidenceCatalog),
        generate::<EvidenceMatchProposalSet>(PublicSchemaId::EvidenceMatchProposals),
        generate::<EvidenceMatchSetRecord>(PublicSchemaId::EvidenceMatches),
        generate::<ApplicationPlanCandidate>(PublicSchemaId::ApplicationPlanCandidate),
    ]
}

pub fn verify_public_schemas() -> Result<(), String> {
    let schemas = generate_public_schemas();
    if schemas.len() != PublicSchemaId::ALL.len() {
        return Err("public schema registry length does not match its ID registry".to_owned());
    }
    let mut ids = BTreeSet::new();
    for schema in schemas {
        if !ids.insert(schema.id) {
            return Err(format!(
                "duplicate public schema ID: {}",
                schema.id.as_str()
            ));
        }
        if !jsonschema::meta::is_valid(&schema.document) {
            return Err(format!(
                "generated schema does not satisfy its meta-schema: {}",
                schema.id.as_str()
            ));
        }
        if schema.document["$id"] != schema.id.canonical_uri()
            || schema.document["x-canisend-id"] != schema.id.as_str()
            || schema.document["x-canisend-version"] != PUBLIC_SCHEMA_VERSION
        {
            return Err(format!(
                "generated schema metadata is incomplete: {}",
                schema.id.as_str()
            ));
        }
    }
    Ok(())
}

fn generate<T: JsonSchema>(id: PublicSchemaId) -> GeneratedSchema {
    let mut document = serde_json::to_value(schemars::schema_for!(T))
        .expect("generated schema serializes to JSON");
    let object = document
        .as_object_mut()
        .expect("generated JSON Schema root is an object");
    object.insert("$id".to_owned(), Value::String(id.canonical_uri()));
    object.insert(
        "x-canisend-id".to_owned(),
        Value::String(id.as_str().to_owned()),
    );
    object.insert(
        "x-canisend-version".to_owned(),
        Value::String(PUBLIC_SCHEMA_VERSION.to_owned()),
    );
    GeneratedSchema { id, document }
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{PublicSchemaId, generate_public_schemas, verify_public_schemas};

    #[test]
    fn public_schema_registry_is_complete_and_deterministic() {
        verify_public_schemas().expect("public schemas verify");
        let first = generate_public_schemas();
        let second = generate_public_schemas();
        assert_eq!(first, second);
        assert_eq!(first.len(), PublicSchemaId::ALL.len());
        assert_eq!(
            first
                .iter()
                .map(|schema| schema.id.file_name())
                .collect::<BTreeSet<_>>()
                .len(),
            first.len()
        );
    }
}
