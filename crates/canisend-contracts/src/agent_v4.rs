use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ApplicationId, ConsentScope, ContractViolation, EntityId, OperationId, PrivacyClassification,
    Revision, SemanticValidate, SemanticVersion, Sha256Digest, UtcTimestamp, WORKSPACE_V4_FORMAT,
    WorkflowPackId,
};

pub const AGENT_V4_PROTOCOL: &str = "canisend.agent/v4";
pub const AGENT_V4_TASK_MODEL_FORMAT: &str = "canisend.agent-task-resource-model/v4";
pub const AGENT_V4_MAX_RESOURCES: usize = 256;
pub const AGENT_V4_MAX_SUMMARY_BYTES: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AgentProtocolV4 {
    #[serde(rename = "canisend.agent/v4")]
    V4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AgentWorkspaceFormatV4 {
    #[serde(rename = "canisend.workspace/v4")]
    V4,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum AgentTaskKindV4 {
    Orientation,
    ProfileEvidence,
    Intake,
    ApplicationCreate,
    Requirements,
    FitPlan,
    Drafting,
    Review,
    Export,
    Recovery,
}

impl AgentTaskKindV4 {
    pub const ALL: [Self; 10] = [
        Self::Orientation,
        Self::ProfileEvidence,
        Self::Intake,
        Self::ApplicationCreate,
        Self::Requirements,
        Self::FitPlan,
        Self::Drafting,
        Self::Review,
        Self::Export,
        Self::Recovery,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Orientation => "orientation",
            Self::ProfileEvidence => "profile-evidence",
            Self::Intake => "intake",
            Self::ApplicationCreate => "application-create",
            Self::Requirements => "requirements",
            Self::FitPlan => "fit-plan",
            Self::Drafting => "drafting",
            Self::Review => "review",
            Self::Export => "export",
            Self::Recovery => "recovery",
        }
    }

    #[must_use]
    pub fn allows_workspace_only_context(self) -> bool {
        matches!(
            self,
            Self::Orientation | Self::ApplicationCreate | Self::Recovery | Self::ProfileEvidence
        )
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum AgentTaskPhaseV4 {
    Orient,
    Propose,
    Preview,
    Approve,
    Commit,
    Verify,
}

impl AgentTaskPhaseV4 {
    pub const MUTATION_SEQUENCE: [Self; 6] = [
        Self::Orient,
        Self::Propose,
        Self::Preview,
        Self::Approve,
        Self::Commit,
        Self::Verify,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentPackBindingV4 {
    pub id: WorkflowPackId,
    pub version: SemanticVersion,
    pub content_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentApplicationBindingV4 {
    pub id: ApplicationId,
    pub pack: AgentPackBindingV4,
    pub expected_revision: Revision,
    pub snapshot_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentContextBindingV4 {
    pub workspace_id: EntityId,
    pub workspace_format: AgentWorkspaceFormatV4,
    pub application: Option<AgentApplicationBindingV4>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum AgentResourceKindV4 {
    WorkspaceHealth,
    Profile,
    Evidence,
    Source,
    Requirement,
    Plan,
    Deliverable,
    Review,
    Export,
    Backup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentResourceReferenceV4 {
    pub kind: AgentResourceKindV4,
    pub id: EntityId,
    pub revision: Option<Revision>,
    pub sha256: Sha256Digest,
    pub privacy: PrivacyClassification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentConsentBindingV4 {
    pub scope: ConsentScope,
    pub granted_by_user: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentTaskRequestV4 {
    pub protocol: AgentProtocolV4,
    pub task_id: EntityId,
    pub task: AgentTaskKindV4,
    pub operation: OperationId,
    pub context: AgentContextBindingV4,
    pub resources: Vec<AgentResourceReferenceV4>,
    pub requested_consents: Vec<ConsentScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentProposalV4 {
    pub protocol: AgentProtocolV4,
    pub task_id: EntityId,
    pub operation: OperationId,
    pub context: AgentContextBindingV4,
    pub request_sha256: Sha256Digest,
    pub candidate_schema: String,
    pub candidate_sha256: Sha256Digest,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentMutationPreviewV4 {
    pub protocol: AgentProtocolV4,
    pub task_id: EntityId,
    pub operation: OperationId,
    pub context: AgentContextBindingV4,
    pub proposal_sha256: Sha256Digest,
    pub preview_sha256: Sha256Digest,
    pub required_consents: Vec<ConsentScope>,
    pub expires_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentApprovalV4 {
    pub protocol: AgentProtocolV4,
    pub task_id: EntityId,
    pub preview_sha256: Sha256Digest,
    pub approved: bool,
    pub consents: Vec<AgentConsentBindingV4>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentCommitRequestV4 {
    pub protocol: AgentProtocolV4,
    pub task_id: EntityId,
    pub operation: OperationId,
    pub context: AgentContextBindingV4,
    pub preview_token: String,
    pub preview_sha256: Sha256Digest,
    pub approval: AgentApprovalV4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AgentReceiptStatusV4 {
    Current,
    Previewed,
    Committed,
    Rejected,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentReceiptV4 {
    pub protocol: AgentProtocolV4,
    pub task_id: EntityId,
    pub operation: OperationId,
    pub status: AgentReceiptStatusV4,
    pub context: AgentContextBindingV4,
    pub committed_revision: Option<Revision>,
    pub snapshot_sha256: Option<Sha256Digest>,
    pub artifacts: Vec<AgentResourceReferenceV4>,
    pub audit_event_id: Option<EntityId>,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentTaskDefinitionV4 {
    pub task: AgentTaskKindV4,
    pub description: String,
    pub operation_prefixes: Vec<String>,
    pub mutation: bool,
    pub phases: Vec<AgentTaskPhaseV4>,
    pub workspace_only_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentTaskResourceModelV4 {
    pub format: String,
    pub protocol: String,
    pub workspace_format: String,
    pub tasks: Vec<AgentTaskDefinitionV4>,
    pub authority: String,
    pub transport_preference: Vec<String>,
    pub direct_internal_writes_allowed: bool,
    pub submission_supported: bool,
    pub legacy_compatibility_supported: bool,
}

impl SemanticValidate for AgentTaskRequestV4 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if !self.task.allows_workspace_only_context() && self.context.application.is_none() {
            violations.push(ContractViolation::new(
                "agent_v4.application_context_required",
                "/context/application",
                "this task requires one exact Application, Pack, revision, and snapshot binding",
            ));
        }
        validate_operation(self.operation.as_str(), "/operation", &mut violations);
        if !canonical_operation_prefixes(self.task)
            .iter()
            .any(|prefix| self.operation.as_str().starts_with(prefix))
        {
            violations.push(ContractViolation::new(
                "agent_v4.task_operation_mismatch",
                "/operation",
                "operation does not belong to the requested canonical task",
            ));
        }
        validate_resources(&self.resources, "/resources", &mut violations);
        validate_unique_consents(
            self.requested_consents.iter().copied(),
            "/requested_consents",
            &mut violations,
        );
        violations
    }
}

impl SemanticValidate for AgentProposalV4 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        validate_operation(self.operation.as_str(), "/operation", &mut violations);
        validate_operation_context(
            self.operation.as_str(),
            &self.context,
            "/context/application",
            &mut violations,
        );
        if self.candidate_schema.trim().is_empty() || self.candidate_schema.len() > 256 {
            violations.push(ContractViolation::new(
                "agent_v4.candidate_schema_invalid",
                "/candidate_schema",
                "candidate schema ID must contain between 1 and 256 bytes",
            ));
        }
        if self.summary.trim().is_empty() || self.summary.len() > AGENT_V4_MAX_SUMMARY_BYTES {
            violations.push(ContractViolation::new(
                "agent_v4.summary_invalid",
                "/summary",
                "proposal summary must be non-empty and bounded",
            ));
        }
        violations
    }
}

impl SemanticValidate for AgentMutationPreviewV4 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        validate_operation(self.operation.as_str(), "/operation", &mut violations);
        validate_operation_context(
            self.operation.as_str(),
            &self.context,
            "/context/application",
            &mut violations,
        );
        validate_unique_consents(
            self.required_consents.iter().copied(),
            "/required_consents",
            &mut violations,
        );
        violations
    }
}

impl SemanticValidate for AgentApprovalV4 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if !self.approved {
            violations.push(ContractViolation::new(
                "agent_v4.approval_required",
                "/approved",
                "an approval record must represent explicit user approval",
            ));
        }
        validate_unique_consents(
            self.consents.iter().map(|consent| consent.scope),
            "/consents",
            &mut violations,
        );
        if self.consents.iter().any(|consent| !consent.granted_by_user) {
            violations.push(ContractViolation::new(
                "agent_v4.consent_not_granted",
                "/consents",
                "every approved consent must be explicitly granted by the user",
            ));
        }
        violations
    }
}

impl SemanticValidate for AgentCommitRequestV4 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        validate_operation(self.operation.as_str(), "/operation", &mut violations);
        validate_operation_context(
            self.operation.as_str(),
            &self.context,
            "/context/application",
            &mut violations,
        );
        if self.preview_token.is_empty() || self.preview_token.len() > 128 {
            violations.push(ContractViolation::new(
                "agent_v4.preview_token_invalid",
                "/preview_token",
                "preview token must contain between 1 and 128 bytes",
            ));
        }
        if self.approval.task_id != self.task_id
            || self.approval.preview_sha256 != self.preview_sha256
        {
            violations.push(ContractViolation::new(
                "agent_v4.approval_binding_mismatch",
                "/approval",
                "approval must bind the exact task and preview digest being committed",
            ));
        }
        for violation in self.approval.validate_semantics() {
            violations.push(ContractViolation::new(
                violation.code,
                format!("/approval{}", violation.json_pointer),
                violation.message,
            ));
        }
        violations
    }
}

impl SemanticValidate for AgentReceiptV4 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        validate_operation(self.operation.as_str(), "/operation", &mut violations);
        validate_operation_context(
            self.operation.as_str(),
            &self.context,
            "/context/application",
            &mut violations,
        );
        validate_resources(&self.artifacts, "/artifacts", &mut violations);
        if self.submission_performed {
            violations.push(ContractViolation::new(
                "agent_v4.submission_forbidden",
                "/submission_performed",
                "CanISend never submits an Application",
            ));
        }
        if self.status == AgentReceiptStatusV4::Committed
            && (self.committed_revision.is_none()
                || self.snapshot_sha256.is_none()
                || self.audit_event_id.is_none())
        {
            violations.push(ContractViolation::new(
                "agent_v4.commit_receipt_incomplete",
                "",
                "a committed receipt requires revision, snapshot digest, and audit event identity",
            ));
        }
        violations
    }
}

impl SemanticValidate for AgentTaskResourceModelV4 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if self.format != AGENT_V4_TASK_MODEL_FORMAT
            || self.protocol != AGENT_V4_PROTOCOL
            || self.workspace_format != WORKSPACE_V4_FORMAT
        {
            violations.push(ContractViolation::new(
                "agent_v4.model_version_invalid",
                "",
                "task model must bind Agent v4 and Workspace v4 exactly",
            ));
        }
        let expected = AgentTaskKindV4::ALL.into_iter().collect::<BTreeSet<_>>();
        let actual = self
            .tasks
            .iter()
            .map(|task| task.task)
            .collect::<BTreeSet<_>>();
        if actual != expected || self.tasks.len() != expected.len() {
            violations.push(ContractViolation::new(
                "agent_v4.task_registry_incomplete",
                "/tasks",
                "task model must declare each canonical Agent v4 task exactly once",
            ));
        }
        for (index, task) in self.tasks.iter().enumerate() {
            if task.description.trim().is_empty()
                || task.description.len() > AGENT_V4_MAX_SUMMARY_BYTES
            {
                violations.push(ContractViolation::new(
                    "agent_v4.task_description_invalid",
                    format!("/tasks/{index}/description"),
                    "task descriptions must be non-empty and bounded",
                ));
            }
            let expected_phases = if task.mutation {
                AgentTaskPhaseV4::MUTATION_SEQUENCE.to_vec()
            } else {
                vec![AgentTaskPhaseV4::Orient, AgentTaskPhaseV4::Verify]
            };
            if task.phases != expected_phases {
                violations.push(ContractViolation::new(
                    "agent_v4.phase_sequence_invalid",
                    format!("/tasks/{index}/phases"),
                    "mutations require the complete approval sequence; reads require orient then verify",
                ));
            }
            if task.workspace_only_allowed != task.task.allows_workspace_only_context() {
                violations.push(ContractViolation::new(
                    "agent_v4.context_policy_invalid",
                    format!("/tasks/{index}/workspace_only_allowed"),
                    "task context policy does not match the canonical task kind",
                ));
            }
            if task.operation_prefixes != canonical_operation_prefixes(task.task) {
                violations.push(ContractViolation::new(
                    "agent_v4.operation_family_invalid",
                    format!("/tasks/{index}/operation_prefixes"),
                    "task operation families must exactly match the neutral Agent v4 registry",
                ));
            }
        }
        if self.authority != "canisend-application-facade"
            || self.transport_preference != ["mcp", "native-cli"]
        {
            violations.push(ContractViolation::new(
                "agent_v4.authority_invalid",
                "",
                "the application facade is authoritative; MCP precedes the native CLI equivalent",
            ));
        }
        if self.direct_internal_writes_allowed
            || self.submission_supported
            || self.legacy_compatibility_supported
        {
            violations.push(ContractViolation::new(
                "agent_v4.safety_boundary_invalid",
                "",
                "Agent v4 forbids direct internal writes, submission, and legacy compatibility",
            ));
        }
        violations
    }
}

fn validate_operation(operation: &str, pointer: &str, violations: &mut Vec<ContractViolation>) {
    if operation.contains("job")
        || operation.contains("generic")
        || operation.contains("academic")
        || operation.contains("v2")
        || operation.contains("v3")
    {
        violations.push(ContractViolation::new(
            "agent_v4.legacy_operation_unsupported",
            pointer,
            "Agent v4 accepts only neutral canonical operation IDs",
        ));
    }
}

fn canonical_operation_prefixes(task: AgentTaskKindV4) -> Vec<String> {
    let prefixes: &[&str] = match task {
        AgentTaskKindV4::Orientation => &["workspace.", "application.list", "application.show"],
        AgentTaskKindV4::ProfileEvidence => &["profile.", "evidence."],
        AgentTaskKindV4::Intake => &["source.intake.", "source.association."],
        AgentTaskKindV4::ApplicationCreate => &["application.create"],
        AgentTaskKindV4::Requirements => &["requirement."],
        AgentTaskKindV4::FitPlan => &["plan.", "evidence.match."],
        AgentTaskKindV4::Drafting => &["deliverable."],
        AgentTaskKindV4::Review => &["review."],
        AgentTaskKindV4::Export => &["export.", "render."],
        AgentTaskKindV4::Recovery => &[
            "workspace.check",
            "workspace.backup",
            "workspace.restore",
            "workspace.repair",
        ],
    };
    prefixes.iter().map(|prefix| (*prefix).to_owned()).collect()
}

fn validate_operation_context(
    operation: &str,
    context: &AgentContextBindingV4,
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    let workspace_only = operation.starts_with("workspace.")
        || operation.starts_with("profile.")
        || operation.starts_with("evidence.")
        || operation == "application.create"
        || operation == "application.list"
        || operation == "application.show";
    if !workspace_only && context.application.is_none() {
        violations.push(ContractViolation::new(
            "agent_v4.application_context_required",
            pointer,
            "this operation requires one exact Application, Pack, revision, and snapshot binding",
        ));
    }
}

fn validate_resources(
    resources: &[AgentResourceReferenceV4],
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    if resources.len() > AGENT_V4_MAX_RESOURCES {
        violations.push(ContractViolation::new(
            "agent_v4.resource_count_invalid",
            pointer,
            "Agent v4 resource sets are bounded to 256 references",
        ));
    }
    let mut identities = BTreeSet::new();
    for (index, resource) in resources.iter().enumerate() {
        if !identities.insert((resource.kind, resource.id.clone(), resource.revision)) {
            violations.push(ContractViolation::new(
                "agent_v4.resource_duplicate",
                format!("{pointer}/{index}"),
                "resource identity and revision must be unique within the task",
            ));
        }
        if resource.privacy == PrivacyClassification::Secret {
            violations.push(ContractViolation::new(
                "agent_v4.secret_resource_forbidden",
                format!("{pointer}/{index}/privacy"),
                "secret material is never an Agent task resource",
            ));
        }
    }
}

fn validate_unique_consents(
    consents: impl IntoIterator<Item = ConsentScope>,
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    let mut seen = BTreeSet::new();
    for consent in consents {
        if !seen.insert(format!("{consent:?}")) {
            violations.push(ContractViolation::new(
                "agent_v4.consent_duplicate",
                pointer,
                "consent scopes must be unique",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::validate_external_candidate;

    fn id(suffix: u16) -> EntityId {
        EntityId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012x}")).expect("UUIDv7")
    }

    fn digest(byte: char) -> Sha256Digest {
        Sha256Digest::try_new(byte.to_string().repeat(64)).expect("digest")
    }

    fn context() -> AgentContextBindingV4 {
        AgentContextBindingV4 {
            workspace_id: id(1),
            workspace_format: AgentWorkspaceFormatV4::V4,
            application: Some(AgentApplicationBindingV4 {
                id: ApplicationId::try_new(id(2).to_string()).expect("Application ID"),
                pack: AgentPackBindingV4 {
                    id: WorkflowPackId::try_new("org.canisend.generic-application")
                        .expect("Pack ID"),
                    version: SemanticVersion::try_new("1.0.0").expect("version"),
                    content_digest: digest('a'),
                },
                expected_revision: Revision::try_new(3).expect("revision"),
                snapshot_sha256: digest('b'),
            }),
        }
    }

    #[test]
    fn request_requires_exact_application_context_for_application_tasks() {
        let request = AgentTaskRequestV4 {
            protocol: AgentProtocolV4::V4,
            task_id: id(3),
            task: AgentTaskKindV4::Drafting,
            operation: OperationId::try_new("deliverable.compose").expect("operation"),
            context: AgentContextBindingV4 {
                workspace_id: id(1),
                workspace_format: AgentWorkspaceFormatV4::V4,
                application: None,
            },
            resources: Vec::new(),
            requested_consents: Vec::new(),
        };
        assert_eq!(
            request.validate_semantics()[0].code,
            "agent_v4.application_context_required"
        );
    }

    #[test]
    fn request_rejects_an_operation_from_a_different_task_family() {
        let request = AgentTaskRequestV4 {
            protocol: AgentProtocolV4::V4,
            task_id: id(3),
            task: AgentTaskKindV4::Drafting,
            operation: OperationId::try_new("review.inspect").expect("operation"),
            context: context(),
            resources: Vec::new(),
            requested_consents: Vec::new(),
        };
        assert!(
            request
                .validate_semantics()
                .iter()
                .any(|violation| violation.code == "agent_v4.task_operation_mismatch")
        );
    }

    #[test]
    fn commit_binds_task_preview_approval_and_granted_consents() {
        let commit = AgentCommitRequestV4 {
            protocol: AgentProtocolV4::V4,
            task_id: id(3),
            operation: OperationId::try_new("source.intake.commit").expect("operation"),
            context: context(),
            preview_token: "opaque-single-use-token".to_owned(),
            preview_sha256: digest('c'),
            approval: AgentApprovalV4 {
                protocol: AgentProtocolV4::V4,
                task_id: id(3),
                preview_sha256: digest('c'),
                approved: true,
                consents: vec![AgentConsentBindingV4 {
                    scope: ConsentScope::ReadPrivateInputs,
                    granted_by_user: true,
                }],
            },
        };
        assert!(commit.validate_semantics().is_empty());

        let mut denied = commit.clone();
        denied.approval.consents[0].granted_by_user = false;
        assert!(
            denied
                .validate_semantics()
                .iter()
                .any(|violation| violation.code == "agent_v4.consent_not_granted")
        );

        let mut unbound = commit;
        unbound.context.application = None;
        assert!(
            unbound
                .validate_semantics()
                .iter()
                .any(|violation| violation.code == "agent_v4.application_context_required")
        );
    }

    #[test]
    fn legacy_protocol_and_unknown_fields_fail_structurally_before_semantics() {
        let candidate = json!({
            "protocol": "canisend.agent/v3",
            "task_id": id(3),
            "task": "orientation",
            "operation": "workspace.status",
            "context": {
                "workspace_id": id(1),
                "workspace_format": "canisend.workspace/v4",
                "application": null
            },
            "resources": [],
            "requested_consents": [],
            "compatibility": true
        });
        assert!(validate_external_candidate::<AgentTaskRequestV4>(&candidate).is_err());
    }

    #[test]
    fn committed_receipt_is_audited_revision_bound_and_submission_free() {
        let receipt = AgentReceiptV4 {
            protocol: AgentProtocolV4::V4,
            task_id: id(3),
            operation: OperationId::try_new("deliverable.compose").expect("operation"),
            status: AgentReceiptStatusV4::Committed,
            context: context(),
            committed_revision: Some(Revision::try_new(4).expect("revision")),
            snapshot_sha256: Some(digest('d')),
            artifacts: Vec::new(),
            audit_event_id: Some(id(4)),
            submission_performed: false,
        };
        assert!(receipt.validate_semantics().is_empty());
    }
}
