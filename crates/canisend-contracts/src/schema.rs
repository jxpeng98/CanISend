use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    AgentContextData, AgentResponse, ApplicationModelSnapshotV3, ApplicationPackBindingV3,
    ApplicationPlanCandidate, ApplicationPlanRecord, ApplicationRecordV3, BackupManifestData,
    CapabilitiesData, CriteriaSetRecord, CriterionRecord, DeliverableRecordV3, DiscoveryBatch,
    DiscoveryLeadRecord, DocumentCandidate, DocumentRecord, DocumentSetRecord,
    EvidenceCatalogRecord, EvidenceMatchProposalSet, EvidenceMatchRecord, EvidenceMatchSetRecord,
    EvidenceProposalSet, EvidenceRecord, FindingRecord, JobRecord, OpportunityRecordV3,
    PackageExportManifestRecord, PackageManifestRecord, ParsedJobRecord, PlanRecordV3,
    ProfileSourceRecord, ProjectionReconcileRecord, ProjectionRecord, ReadinessRecord,
    RenderManifestRecord, RenderedDocumentRecord, RequirementRecordV3, ReviewCandidate,
    ReviewDispositionCandidate, ReviewFindingsRecord, SourceRecord, TaskCompletionRequest,
    TaskDescriptor, VersionData, WORKFLOW_PACK_SCHEMA_ID, WORKFLOW_PACK_SCHEMA_URI,
    WORKFLOW_PACK_SCHEMA_VERSION, WorkflowPackManifest, WorkflowStatusData, WorkspaceCheckData,
    WorkspaceStatusData,
};

pub const PUBLIC_SCHEMA_VERSION: &str = "2.0.0";
pub const PUBLIC_SCHEMA_BASE: &str = "https://schemas.canisend.dev/v2";
pub const APPLICATION_MODEL_SCHEMA_VERSION: &str = "3.0.0";
pub const APPLICATION_MODEL_SCHEMA_BASE: &str = "https://schemas.canisend.dev/v3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationModelSchemaId {
    PackBinding,
    Opportunity,
    Application,
    Requirement,
    Plan,
    Deliverable,
    ApplicationModel,
}

impl ApplicationModelSchemaId {
    pub const ALL: [Self; 7] = [
        Self::PackBinding,
        Self::Opportunity,
        Self::Application,
        Self::Requirement,
        Self::Plan,
        Self::Deliverable,
        Self::ApplicationModel,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PackBinding => "canisend.application-pack-binding/v3",
            Self::Opportunity => "canisend.opportunity/v3",
            Self::Application => "canisend.application/v3",
            Self::Requirement => "canisend.requirement/v3",
            Self::Plan => "canisend.plan/v3",
            Self::Deliverable => "canisend.deliverable/v3",
            Self::ApplicationModel => "canisend.application-model/v3",
        }
    }

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::PackBinding => "application-pack-binding",
            Self::Opportunity => "opportunity",
            Self::Application => "application",
            Self::Requirement => "requirement",
            Self::Plan => "plan",
            Self::Deliverable => "deliverable",
            Self::ApplicationModel => "application-model",
        }
    }

    #[must_use]
    pub fn file_name(self) -> String {
        format!("{}.schema.json", self.slug())
    }

    #[must_use]
    pub fn canonical_uri(self) -> String {
        format!("{APPLICATION_MODEL_SCHEMA_BASE}/{}", self.file_name())
    }
}

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
    DocumentSet,
    ReviewCandidate,
    ReviewFindings,
    ReviewDispositionCandidate,
    Finding,
    Readiness,
    PackageManifest,
    Projection,
    PackageExportManifest,
    ProjectionReconcile,
    RenderedDocument,
    RenderManifest,
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
    pub const ALL: [Self; 40] = [
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
        Self::DocumentSet,
        Self::ReviewCandidate,
        Self::ReviewFindings,
        Self::ReviewDispositionCandidate,
        Self::Finding,
        Self::Readiness,
        Self::PackageManifest,
        Self::Projection,
        Self::PackageExportManifest,
        Self::ProjectionReconcile,
        Self::RenderedDocument,
        Self::RenderManifest,
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
            Self::DocumentSet => "canisend.document-set/v2",
            Self::ReviewCandidate => "canisend.review-candidate/v2",
            Self::ReviewFindings => "canisend.review-findings/v2",
            Self::ReviewDispositionCandidate => "canisend.review-disposition-candidate/v2",
            Self::Finding => "canisend.finding/v2",
            Self::Readiness => "canisend.readiness/v2",
            Self::PackageManifest => "canisend.package-manifest/v2",
            Self::Projection => "canisend.projection/v2",
            Self::PackageExportManifest => "canisend.package-export-manifest/v2",
            Self::ProjectionReconcile => "canisend.projection-reconcile/v2",
            Self::RenderedDocument => "canisend.rendered-document/v2",
            Self::RenderManifest => "canisend.render-manifest/v2",
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
            Self::DocumentSet => "document-set",
            Self::ReviewCandidate => "review-candidate",
            Self::ReviewFindings => "review-findings",
            Self::ReviewDispositionCandidate => "review-disposition-candidate",
            Self::Finding => "finding",
            Self::Readiness => "readiness",
            Self::PackageManifest => "package-manifest",
            Self::Projection => "projection",
            Self::PackageExportManifest => "package-export-manifest",
            Self::ProjectionReconcile => "projection-reconcile",
            Self::RenderedDocument => "rendered-document",
            Self::RenderManifest => "render-manifest",
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

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedApplicationModelSchema {
    pub id: ApplicationModelSchemaId,
    pub document: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedWorkflowPackSchema {
    pub document: Value,
}

impl GeneratedWorkflowPackSchema {
    #[must_use]
    pub const fn file_name(&self) -> &'static str {
        "manifest.schema.json"
    }

    #[must_use]
    pub fn canonical_json(&self) -> String {
        let mut output = serde_json::to_string_pretty(&sort_json(self.document.clone()))
            .expect("generated workflow-pack schema serializes");
        output.push('\n');
        output
    }
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

impl GeneratedApplicationModelSchema {
    #[must_use]
    pub fn canonical_json(&self) -> String {
        let mut output = serde_json::to_string_pretty(&sort_json(self.document.clone()))
            .expect("generated application-model schema serializes");
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
        generate::<DocumentSetRecord>(PublicSchemaId::DocumentSet),
        generate::<ReviewCandidate>(PublicSchemaId::ReviewCandidate),
        generate::<ReviewFindingsRecord>(PublicSchemaId::ReviewFindings),
        generate::<ReviewDispositionCandidate>(PublicSchemaId::ReviewDispositionCandidate),
        generate::<FindingRecord>(PublicSchemaId::Finding),
        generate::<ReadinessRecord>(PublicSchemaId::Readiness),
        generate::<PackageManifestRecord>(PublicSchemaId::PackageManifest),
        generate::<ProjectionRecord>(PublicSchemaId::Projection),
        generate::<PackageExportManifestRecord>(PublicSchemaId::PackageExportManifest),
        generate::<ProjectionReconcileRecord>(PublicSchemaId::ProjectionReconcile),
        generate::<RenderedDocumentRecord>(PublicSchemaId::RenderedDocument),
        generate::<RenderManifestRecord>(PublicSchemaId::RenderManifest),
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

#[must_use]
pub fn generate_application_model_schemas() -> Vec<GeneratedApplicationModelSchema> {
    vec![
        generate_application_model::<ApplicationPackBindingV3>(
            ApplicationModelSchemaId::PackBinding,
        ),
        generate_application_model::<OpportunityRecordV3>(ApplicationModelSchemaId::Opportunity),
        generate_application_model::<ApplicationRecordV3>(ApplicationModelSchemaId::Application),
        generate_application_model::<RequirementRecordV3>(ApplicationModelSchemaId::Requirement),
        generate_application_model::<PlanRecordV3>(ApplicationModelSchemaId::Plan),
        generate_application_model::<DeliverableRecordV3>(ApplicationModelSchemaId::Deliverable),
        generate_application_model::<ApplicationModelSnapshotV3>(
            ApplicationModelSchemaId::ApplicationModel,
        ),
    ]
}

pub fn verify_application_model_schemas() -> Result<(), String> {
    let schemas = generate_application_model_schemas();
    if schemas.len() != ApplicationModelSchemaId::ALL.len() {
        return Err(
            "application-model schema registry length does not match its ID registry".to_owned(),
        );
    }
    let mut ids = BTreeSet::new();
    for schema in schemas {
        if !ids.insert(schema.id) {
            return Err(format!(
                "duplicate application-model schema ID: {}",
                schema.id.as_str()
            ));
        }
        if !jsonschema::meta::is_valid(&schema.document) {
            return Err(format!(
                "generated application-model schema does not satisfy its meta-schema: {}",
                schema.id.as_str()
            ));
        }
        if schema.document["$id"] != schema.id.canonical_uri()
            || schema.document["x-canisend-id"] != schema.id.as_str()
            || schema.document["x-canisend-version"] != APPLICATION_MODEL_SCHEMA_VERSION
        {
            return Err(format!(
                "generated application-model schema metadata is incomplete: {}",
                schema.id.as_str()
            ));
        }
    }
    Ok(())
}

#[must_use]
pub fn generate_workflow_pack_schema() -> GeneratedWorkflowPackSchema {
    GeneratedWorkflowPackSchema {
        document: generate_schema_document::<WorkflowPackManifest>(
            WORKFLOW_PACK_SCHEMA_URI,
            WORKFLOW_PACK_SCHEMA_ID,
            WORKFLOW_PACK_SCHEMA_VERSION,
        ),
    }
}

pub fn verify_workflow_pack_schema() -> Result<(), String> {
    let schema = generate_workflow_pack_schema();
    if !jsonschema::meta::is_valid(&schema.document) {
        return Err("generated workflow-pack schema does not satisfy its meta-schema".to_owned());
    }
    if schema.document["$id"] != WORKFLOW_PACK_SCHEMA_URI
        || schema.document["x-canisend-id"] != WORKFLOW_PACK_SCHEMA_ID
        || schema.document["x-canisend-version"] != WORKFLOW_PACK_SCHEMA_VERSION
    {
        return Err("generated workflow-pack schema metadata is incomplete".to_owned());
    }
    Ok(())
}

fn generate<T: JsonSchema>(id: PublicSchemaId) -> GeneratedSchema {
    let document =
        generate_schema_document::<T>(&id.canonical_uri(), id.as_str(), PUBLIC_SCHEMA_VERSION);
    GeneratedSchema { id, document }
}

fn generate_application_model<T: JsonSchema>(
    id: ApplicationModelSchemaId,
) -> GeneratedApplicationModelSchema {
    let document = generate_schema_document::<T>(
        &id.canonical_uri(),
        id.as_str(),
        APPLICATION_MODEL_SCHEMA_VERSION,
    );
    GeneratedApplicationModelSchema { id, document }
}

fn generate_schema_document<T: JsonSchema>(uri: &str, id: &str, version: &str) -> Value {
    let mut document = serde_json::to_value(schemars::schema_for!(T))
        .expect("generated schema serializes to JSON");
    let object = document
        .as_object_mut()
        .expect("generated JSON Schema root is an object");
    object.insert("$id".to_owned(), Value::String(uri.to_owned()));
    object.insert("x-canisend-id".to_owned(), Value::String(id.to_owned()));
    object.insert(
        "x-canisend-version".to_owned(),
        Value::String(version.to_owned()),
    );
    document
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

    use super::{
        ApplicationModelSchemaId, PublicSchemaId, generate_application_model_schemas,
        generate_public_schemas, generate_workflow_pack_schema, verify_application_model_schemas,
        verify_public_schemas, verify_workflow_pack_schema,
    };

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

    #[test]
    fn workflow_pack_schema_is_versioned_and_deterministic() {
        verify_workflow_pack_schema().expect("workflow-pack schema verifies");
        let first = generate_workflow_pack_schema();
        let second = generate_workflow_pack_schema();
        assert_eq!(first, second);
        assert_eq!(first.file_name(), "manifest.schema.json");
        assert_eq!(first.canonical_json(), second.canonical_json());
    }

    #[test]
    fn application_model_schema_registry_is_complete_and_deterministic() {
        verify_application_model_schemas().expect("application-model schemas verify");
        let first = generate_application_model_schemas();
        let second = generate_application_model_schemas();
        assert_eq!(first, second);
        assert_eq!(first.len(), ApplicationModelSchemaId::ALL.len());
        assert_eq!(
            first
                .iter()
                .map(|schema| schema.id.file_name())
                .collect::<BTreeSet<_>>()
                .len(),
            first.len()
        );
    }

    #[test]
    fn application_model_schemas_have_no_academic_only_required_fields_or_fixed_types() {
        const ACADEMIC_ONLY_FIELDS: &[&str] = &[
            "job",
            "job_id",
            "institution",
            "faculty",
            "research",
            "teaching",
            "cv",
            "cover_letter",
            "document_kind",
            "criterion",
            "criterion_id",
        ];
        const FIXED_V2_TYPES: &[&str] = &[
            "ArtifactKind",
            "ApplicationDecision",
            "DocumentKind",
            "cover-letter",
            "research-statement",
            "teaching-statement",
        ];

        for schema in generate_application_model_schemas() {
            let mut required_fields = Vec::new();
            collect_required_fields(&schema.document, &mut required_fields);
            for field in ACADEMIC_ONLY_FIELDS {
                assert!(
                    !required_fields.iter().any(|required| required == field),
                    "{} requires academic-only field {field}",
                    schema.id.as_str()
                );
            }
            let canonical = schema.canonical_json();
            for fixed_type in FIXED_V2_TYPES {
                assert!(
                    !canonical.contains(fixed_type),
                    "{} contains fixed v2 type {fixed_type}",
                    schema.id.as_str()
                );
            }
        }
    }

    fn collect_required_fields(value: &serde_json::Value, fields: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(object) => {
                if let Some(serde_json::Value::Array(required)) = object.get("required") {
                    fields.extend(
                        required
                            .iter()
                            .filter_map(|field| field.as_str().map(str::to_owned)),
                    );
                }
                for nested in object.values() {
                    collect_required_fields(nested, fields);
                }
            }
            serde_json::Value::Array(values) => {
                for nested in values {
                    collect_required_fields(nested, fields);
                }
            }
            _ => {}
        }
    }
}
