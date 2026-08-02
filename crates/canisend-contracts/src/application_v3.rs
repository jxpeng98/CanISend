use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, Month, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    ActorKind, CandidateValidationError, ContractViolation, DeliverableKindId, EntityId,
    ExecutionMode, Revision, SemanticValidate, SemanticVersion, Sha256Digest, UtcTimestamp,
    WorkflowPackId, WorkflowPackItemId, validate_external_candidate,
};

pub const APPLICATION_MODEL_V3_FORMAT: &str = "canisend.application-model/v3";
pub const WORKSPACE_V3_FORMAT: &str = "canisend.workspace/v3";
pub const APPLICATION_MODEL_V3_MAX_METADATA_FIELDS: usize = 128;
pub const APPLICATION_MODEL_V3_MAX_SOURCES: usize = 128;
pub const APPLICATION_MODEL_V3_MAX_REQUIREMENTS: usize = 1_000;
pub const APPLICATION_MODEL_V3_MAX_DELIVERABLES: usize = 256;
pub const APPLICATION_MODEL_V3_MAX_BLOCKERS: usize = 256;

macro_rules! application_entity_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(EntityId);

        impl $name {
            pub fn try_new(value: impl Into<String>) -> Result<Self, crate::PrimitiveError> {
                EntityId::try_new(value).map(Self)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            #[must_use]
            pub const fn as_entity_id(&self) -> &EntityId {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

application_entity_id!(OpportunityId);
application_entity_id!(ApplicationId);
application_entity_id!(RequirementId);
application_entity_id!(PlanId);
application_entity_id!(DeliverableId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ApplicationModelFormatV3 {
    #[serde(rename = "canisend.application-model/v3")]
    V3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPackBindingV3 {
    pub id: WorkflowPackId,
    pub version: SemanticVersion,
    pub content_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum ApplicationFieldValueV3 {
    ShortText(String),
    LongText(String),
    Integer(i64),
    Boolean(bool),
    Date(String),
    Url(String),
    StringList(Vec<String>),
    Choice(WorkflowPackItemId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentRevisionReferenceV3 {
    pub id: EntityId,
    pub revision: Revision,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentSpanV3 {
    pub content: ContentRevisionReferenceV3,
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntityRevisionReferenceV3 {
    pub id: EntityId,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementRevisionReferenceV3 {
    pub id: RequirementId,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanRevisionReferenceV3 {
    pub id: PlanId,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpportunityRecordV3 {
    pub id: OpportunityId,
    pub pack: ApplicationPackBindingV3,
    pub title: String,
    pub metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub source_ids: Vec<EntityId>,
    pub created_at: UtcTimestamp,
    pub revision: Revision,
    pub archived: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationLifecycleV3 {
    Draft,
    Active,
    Paused,
    Completed,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationRecordV3 {
    pub id: ApplicationId,
    pub opportunity_id: OpportunityId,
    pub pack: ApplicationPackBindingV3,
    pub metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub lifecycle: ApplicationLifecycleV3,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementPriorityV3 {
    Mandatory,
    Recommended,
    Informational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementConfirmationV3 {
    Proposed,
    Confirmed,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequirementRecordV3 {
    pub id: RequirementId,
    pub application_id: ApplicationId,
    pub pack: ApplicationPackBindingV3,
    pub category: WorkflowPackItemId,
    pub statement: String,
    pub priority: RequirementPriorityV3,
    pub source_span: ContentSpanV3,
    pub confirmation: RequirementConfirmationV3,
    pub confirmed_by: Option<ActorKind>,
    pub confirmed_at: Option<UtcTimestamp>,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStateV3 {
    Draft,
    Confirmed,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlannedDeliverableDispositionV3 {
    Required,
    Optional,
    Omitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedDeliverableV3 {
    pub kind: DeliverableKindId,
    pub disposition: PlannedDeliverableDispositionV3,
    pub rationale: String,
    pub constraints: Vec<String>,
    pub execution_mode: Option<ExecutionMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlanBlockerSeverityV3 {
    Blocking,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanBlockerV3 {
    pub code: String,
    pub requirement: Option<RequirementRevisionReferenceV3>,
    pub severity: PlanBlockerSeverityV3,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanRecordV3 {
    pub id: PlanId,
    pub application_id: ApplicationId,
    pub pack: ApplicationPackBindingV3,
    pub state: PlanStateV3,
    pub decision: Option<WorkflowPackItemId>,
    pub requirement_inputs: Vec<RequirementRevisionReferenceV3>,
    pub deliverables: Vec<PlannedDeliverableV3>,
    pub blockers: Vec<PlanBlockerV3>,
    pub decided_by: Option<ActorKind>,
    pub decided_at: Option<UtcTimestamp>,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DeliverableStateV3 {
    Planned,
    Draft,
    ReviewRequired,
    Approved,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliverableRecordV3 {
    pub id: DeliverableId,
    pub application_id: ApplicationId,
    pub pack: ApplicationPackBindingV3,
    pub plan: PlanRevisionReferenceV3,
    pub kind: DeliverableKindId,
    pub title: String,
    pub state: DeliverableStateV3,
    pub content: Option<ContentRevisionReferenceV3>,
    pub media_type: Option<String>,
    pub evidence_inputs: Vec<EntityRevisionReferenceV3>,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplicationModelSnapshotV3 {
    pub format: ApplicationModelFormatV3,
    pub pack: ApplicationPackBindingV3,
    pub opportunity: OpportunityRecordV3,
    pub application: ApplicationRecordV3,
    pub requirements: Vec<RequirementRecordV3>,
    pub plan: Option<PlanRecordV3>,
    pub deliverables: Vec<DeliverableRecordV3>,
}

pub fn validate_application_model_snapshot_v3(
    value: &Value,
) -> Result<ApplicationModelSnapshotV3, CandidateValidationError> {
    validate_external_candidate(value)
}

impl SemanticValidate for ApplicationPackBindingV3 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        Vec::new()
    }
}

impl SemanticValidate for OpportunityRecordV3 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.title, "/title", 512, &mut violations);
        validate_metadata(&self.metadata, "/metadata", &mut violations);
        if self.source_ids.len() > APPLICATION_MODEL_V3_MAX_SOURCES {
            violations.push(ContractViolation::new(
                "application_v3.opportunity_source_count_invalid",
                "/source_ids",
                format!("an Opportunity may reference at most {APPLICATION_MODEL_V3_MAX_SOURCES} sources"),
            ));
        }
        if self.source_ids.iter().collect::<BTreeSet<_>>().len() != self.source_ids.len() {
            violations.push(ContractViolation::new(
                "application_v3.opportunity_source_duplicate",
                "/source_ids",
                "Opportunity source IDs must be unique",
            ));
        }
        violations
    }
}

impl SemanticValidate for ApplicationRecordV3 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        validate_metadata(&self.metadata, "/metadata", &mut violations);
        if timestamp_precedes(&self.updated_at, &self.created_at) {
            violations.push(ContractViolation::new(
                "application_v3.timestamp_order_invalid",
                "/updated_at",
                "Application updated_at cannot precede created_at",
            ));
        }
        violations
    }
}

impl SemanticValidate for RequirementRecordV3 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.statement, "/statement", 16_384, &mut violations);
        validate_content_span(&self.source_span, "/source_span", &mut violations);
        let decision_complete =
            self.confirmed_by == Some(ActorKind::User) && self.confirmed_at.is_some();
        match self.confirmation {
            RequirementConfirmationV3::Proposed => {
                if self.confirmed_by.is_some() || self.confirmed_at.is_some() {
                    violations.push(ContractViolation::new(
                        "application_v3.requirement_confirmation_invalid",
                        "/confirmation",
                        "a proposed Requirement cannot carry confirmation authority or time",
                    ));
                }
            }
            RequirementConfirmationV3::Confirmed | RequirementConfirmationV3::Excluded => {
                if !decision_complete {
                    violations.push(ContractViolation::new(
                        "application_v3.requirement_confirmation_invalid",
                        "/confirmation",
                        "a confirmed or excluded Requirement requires an explicit user decision and time",
                    ));
                }
            }
        }
        violations
    }
}

impl SemanticValidate for PlanRecordV3 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        if self.requirement_inputs.len() > APPLICATION_MODEL_V3_MAX_REQUIREMENTS {
            violations.push(ContractViolation::new(
                "application_v3.plan_requirement_count_invalid",
                "/requirement_inputs",
                format!("a Plan may reference at most {APPLICATION_MODEL_V3_MAX_REQUIREMENTS} Requirements"),
            ));
        }
        if self.deliverables.len() > APPLICATION_MODEL_V3_MAX_DELIVERABLES {
            violations.push(ContractViolation::new(
                "application_v3.plan_deliverable_count_invalid",
                "/deliverables",
                format!("a Plan may declare at most {APPLICATION_MODEL_V3_MAX_DELIVERABLES} Deliverables"),
            ));
        }
        if self.blockers.len() > APPLICATION_MODEL_V3_MAX_BLOCKERS {
            violations.push(ContractViolation::new(
                "application_v3.plan_blocker_count_invalid",
                "/blockers",
                format!("a Plan may declare at most {APPLICATION_MODEL_V3_MAX_BLOCKERS} blockers"),
            ));
        }
        unique_requirement_references(
            &self.requirement_inputs,
            "/requirement_inputs",
            &mut violations,
        );
        let mut kinds = BTreeSet::new();
        for (index, deliverable) in self.deliverables.iter().enumerate() {
            let pointer = format!("/deliverables/{index}");
            if !kinds.insert(&deliverable.kind) {
                violations.push(ContractViolation::new(
                    "application_v3.plan_deliverable_kind_duplicate",
                    format!("{pointer}/kind"),
                    "a Plan may declare each Deliverable kind only once",
                ));
            }
            validate_pack_owned_deliverable_kind(
                &deliverable.kind,
                &self.pack,
                &format!("{pointer}/kind"),
                &mut violations,
            );
            required_text(
                &deliverable.rationale,
                &format!("{pointer}/rationale"),
                4096,
                &mut violations,
            );
            validate_text_list(
                &deliverable.constraints,
                &format!("{pointer}/constraints"),
                128,
                4096,
                &mut violations,
            );
            if deliverable.disposition == PlannedDeliverableDispositionV3::Omitted
                && deliverable.execution_mode.is_some()
            {
                violations.push(ContractViolation::new(
                    "application_v3.omitted_deliverable_executor_invalid",
                    format!("{pointer}/execution_mode"),
                    "an omitted Deliverable cannot select an execution mode",
                ));
            }
        }
        for (index, blocker) in self.blockers.iter().enumerate() {
            required_text(
                &blocker.code,
                &format!("/blockers/{index}/code"),
                128,
                &mut violations,
            );
            required_text(
                &blocker.description,
                &format!("/blockers/{index}/description"),
                4096,
                &mut violations,
            );
        }
        match self.state {
            PlanStateV3::Draft => {
                if self.decided_by.is_some() || self.decided_at.is_some() {
                    violations.push(ContractViolation::new(
                        "application_v3.plan_decision_invalid",
                        "/state",
                        "a draft Plan cannot carry final decision authority or time",
                    ));
                }
            }
            PlanStateV3::Confirmed => {
                if self.decision.is_none()
                    || self.decided_by != Some(ActorKind::User)
                    || self.decided_at.is_none()
                {
                    violations.push(ContractViolation::new(
                        "application_v3.plan_decision_invalid",
                        "/state",
                        "a confirmed Plan requires a Pack-owned decision and explicit user authority/time",
                    ));
                }
            }
            PlanStateV3::Stale => {
                let preserves_confirmed_decision = self.decision.is_some()
                    && self.decided_by == Some(ActorKind::User)
                    && self.decided_at.is_some();
                let preserves_draft_decision =
                    self.decided_by.is_none() && self.decided_at.is_none();
                if !preserves_confirmed_decision && !preserves_draft_decision {
                    violations.push(ContractViolation::new(
                        "application_v3.plan_decision_invalid",
                        "/state",
                        "a stale Plan must preserve either a complete user decision or an undecided draft state",
                    ));
                }
            }
        }
        violations
    }
}

impl SemanticValidate for DeliverableRecordV3 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        required_text(&self.title, "/title", 512, &mut violations);
        validate_pack_owned_deliverable_kind(&self.kind, &self.pack, "/kind", &mut violations);
        if self.evidence_inputs.len() > APPLICATION_MODEL_V3_MAX_REQUIREMENTS {
            violations.push(ContractViolation::new(
                "application_v3.deliverable_evidence_count_invalid",
                "/evidence_inputs",
                format!("a Deliverable may reference at most {APPLICATION_MODEL_V3_MAX_REQUIREMENTS} Evidence items"),
            ));
        }
        if self
            .evidence_inputs
            .iter()
            .map(|reference| &reference.id)
            .collect::<BTreeSet<_>>()
            .len()
            != self.evidence_inputs.len()
        {
            violations.push(ContractViolation::new(
                "application_v3.deliverable_evidence_duplicate",
                "/evidence_inputs",
                "Deliverable Evidence inputs must be unique",
            ));
        }
        let has_content = self.content.is_some();
        let has_media_type = self.media_type.is_some();
        if self.state == DeliverableStateV3::Planned {
            if has_content || has_media_type {
                violations.push(ContractViolation::new(
                    "application_v3.deliverable_content_state_invalid",
                    "/state",
                    "a planned Deliverable cannot already bind content",
                ));
            }
        } else if !has_content || !has_media_type {
            violations.push(ContractViolation::new(
                "application_v3.deliverable_content_state_invalid",
                "/state",
                "a materialized Deliverable state requires exact content and media type",
            ));
        }
        if let Some(media_type) = &self.media_type {
            required_text(media_type, "/media_type", 255, &mut violations);
            if !valid_media_type(media_type) {
                violations.push(ContractViolation::new(
                    "application_v3.media_type_invalid",
                    "/media_type",
                    "Deliverable media type must be a bounded type/subtype token",
                ));
            }
        }
        violations
    }
}

impl SemanticValidate for ApplicationModelSnapshotV3 {
    fn validate_semantics(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        extend_prefixed(
            &mut violations,
            "/opportunity",
            self.opportunity.validate_semantics(),
        );
        extend_prefixed(
            &mut violations,
            "/application",
            self.application.validate_semantics(),
        );
        if self.opportunity.pack != self.pack || self.application.pack != self.pack {
            violations.push(ContractViolation::new(
                "application_v3.pack_binding_mismatch",
                "/pack",
                "Opportunity and Application must use the exact snapshot Pack binding",
            ));
        }
        if self.application.opportunity_id != self.opportunity.id {
            violations.push(ContractViolation::new(
                "application_v3.opportunity_binding_mismatch",
                "/application/opportunity_id",
                "Application must reference the snapshot Opportunity",
            ));
        }
        if self.requirements.len() > APPLICATION_MODEL_V3_MAX_REQUIREMENTS {
            violations.push(ContractViolation::new(
                "application_v3.requirement_count_invalid",
                "/requirements",
                format!("an Application may contain at most {APPLICATION_MODEL_V3_MAX_REQUIREMENTS} Requirements"),
            ));
        }
        let mut requirements = BTreeMap::new();
        for (index, requirement) in self.requirements.iter().enumerate() {
            extend_prefixed(
                &mut violations,
                &format!("/requirements/{index}"),
                requirement.validate_semantics(),
            );
            if requirement.application_id != self.application.id || requirement.pack != self.pack {
                violations.push(ContractViolation::new(
                    "application_v3.requirement_binding_mismatch",
                    format!("/requirements/{index}"),
                    "Requirement must use the snapshot Application and Pack binding",
                ));
            }
            if requirements.insert(&requirement.id, requirement).is_some() {
                violations.push(ContractViolation::new(
                    "application_v3.requirement_id_duplicate",
                    format!("/requirements/{index}/id"),
                    "Requirement IDs must be unique within an Application",
                ));
            }
        }
        if self.deliverables.len() > APPLICATION_MODEL_V3_MAX_DELIVERABLES {
            violations.push(ContractViolation::new(
                "application_v3.deliverable_count_invalid",
                "/deliverables",
                format!("an Application may contain at most {APPLICATION_MODEL_V3_MAX_DELIVERABLES} Deliverables"),
            ));
        }
        let plan = self.plan.as_ref();
        if let Some(plan) = plan {
            extend_prefixed(&mut violations, "/plan", plan.validate_semantics());
            if plan.application_id != self.application.id || plan.pack != self.pack {
                violations.push(ContractViolation::new(
                    "application_v3.plan_binding_mismatch",
                    "/plan",
                    "Plan must use the snapshot Application and Pack binding",
                ));
            }
            for (index, reference) in plan.requirement_inputs.iter().enumerate() {
                let reference_is_valid =
                    requirements.get(&reference.id).is_some_and(|requirement| {
                        if plan.state == PlanStateV3::Stale {
                            reference.revision <= requirement.revision
                        } else {
                            reference.revision == requirement.revision
                        }
                    });
                if !reference_is_valid {
                    violations.push(ContractViolation::new(
                        "application_v3.plan_requirement_revision_mismatch",
                        format!("/plan/requirement_inputs/{index}"),
                        "current Plans require exact Requirement revisions; stale Plans may preserve earlier revisions",
                    ));
                }
            }
            for (index, blocker) in plan.blockers.iter().enumerate() {
                if let Some(reference) = &blocker.requirement
                    && !requirements.get(&reference.id).is_some_and(|requirement| {
                        if plan.state == PlanStateV3::Stale {
                            reference.revision <= requirement.revision
                        } else {
                            reference.revision == requirement.revision
                        }
                    })
                {
                    violations.push(ContractViolation::new(
                        "application_v3.blocker_requirement_revision_mismatch",
                        format!("/plan/blockers/{index}/requirement"),
                        "Plan blocker must reference an exact snapshot Requirement revision",
                    ));
                }
            }
        } else if !self.deliverables.is_empty() {
            violations.push(ContractViolation::new(
                "application_v3.deliverable_plan_missing",
                "/deliverables",
                "materialized Deliverables require a snapshot Plan",
            ));
        }
        let mut deliverable_ids = BTreeSet::new();
        for (index, deliverable) in self.deliverables.iter().enumerate() {
            extend_prefixed(
                &mut violations,
                &format!("/deliverables/{index}"),
                deliverable.validate_semantics(),
            );
            if !deliverable_ids.insert(&deliverable.id) {
                violations.push(ContractViolation::new(
                    "application_v3.deliverable_id_duplicate",
                    format!("/deliverables/{index}/id"),
                    "Deliverable IDs must be unique within an Application",
                ));
            }
            if deliverable.application_id != self.application.id || deliverable.pack != self.pack {
                violations.push(ContractViolation::new(
                    "application_v3.deliverable_binding_mismatch",
                    format!("/deliverables/{index}"),
                    "Deliverable must use the snapshot Application and Pack binding",
                ));
            }
            if let Some(plan) = plan {
                let plan_reference_is_valid = deliverable.plan.id == plan.id
                    && if deliverable.state == DeliverableStateV3::Stale {
                        deliverable.plan.revision <= plan.revision
                    } else {
                        deliverable.plan.revision == plan.revision
                    };
                if !plan_reference_is_valid {
                    violations.push(ContractViolation::new(
                        "application_v3.deliverable_plan_revision_mismatch",
                        format!("/deliverables/{index}/plan"),
                        "current Deliverables require the exact Plan revision; stale Deliverables may preserve an earlier revision",
                    ));
                }
                if deliverable.state != DeliverableStateV3::Stale
                    && !plan.deliverables.iter().any(|planned| {
                        planned.kind == deliverable.kind
                            && planned.disposition != PlannedDeliverableDispositionV3::Omitted
                    })
                {
                    violations.push(ContractViolation::new(
                        "application_v3.deliverable_kind_unplanned",
                        format!("/deliverables/{index}/kind"),
                        "Deliverable kind must be included by the snapshot Plan",
                    ));
                }
            }
        }
        violations
    }
}

fn required_text(
    value: &str,
    pointer: &str,
    maximum: usize,
    violations: &mut Vec<ContractViolation>,
) {
    if value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        violations.push(ContractViolation::new(
            "application_v3.text_invalid",
            pointer,
            format!("value must contain 1-{maximum} non-control bytes"),
        ));
    }
}

fn validate_metadata(
    metadata: &BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    if metadata.len() > APPLICATION_MODEL_V3_MAX_METADATA_FIELDS {
        violations.push(ContractViolation::new(
            "application_v3.metadata_count_invalid",
            pointer,
            format!(
                "metadata may contain at most {APPLICATION_MODEL_V3_MAX_METADATA_FIELDS} fields"
            ),
        ));
    }
    for (id, value) in metadata {
        let field_pointer = format!("{pointer}/{id}");
        match value {
            ApplicationFieldValueV3::ShortText(value) => {
                required_text(value, &field_pointer, 512, violations);
            }
            ApplicationFieldValueV3::LongText(value) => {
                required_text(value, &field_pointer, 16_384, violations);
            }
            ApplicationFieldValueV3::Date(value) => {
                if !valid_date(value) {
                    violations.push(ContractViolation::new(
                        "application_v3.date_invalid",
                        field_pointer,
                        "date metadata must use a valid YYYY-MM-DD calendar date",
                    ));
                }
            }
            ApplicationFieldValueV3::Url(value) => {
                if value.len() > 2048
                    || value.chars().any(char::is_control)
                    || value.chars().any(char::is_whitespace)
                    || !value
                        .strip_prefix("https://")
                        .or_else(|| value.strip_prefix("http://"))
                        .is_some_and(|destination| !destination.is_empty())
                {
                    violations.push(ContractViolation::new(
                        "application_v3.url_invalid",
                        field_pointer,
                        "URL metadata must be a bounded HTTP or HTTPS URL",
                    ));
                }
            }
            ApplicationFieldValueV3::StringList(values) => {
                validate_text_list(values, &field_pointer, 128, 512, violations);
            }
            ApplicationFieldValueV3::Integer(_)
            | ApplicationFieldValueV3::Boolean(_)
            | ApplicationFieldValueV3::Choice(_) => {}
        }
    }
}

fn valid_date(value: &str) -> bool {
    let mut parts = value.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) =
        (year.parse::<i32>(), month.parse::<u8>(), day.parse::<u8>())
    else {
        return false;
    };
    Month::try_from(month)
        .ok()
        .and_then(|month| Date::from_calendar_date(year, month, day).ok())
        .is_some()
}

fn timestamp_precedes(left: &UtcTimestamp, right: &UtcTimestamp) -> bool {
    let left = OffsetDateTime::parse(left.as_str(), &Rfc3339)
        .expect("UtcTimestamp guarantees a valid RFC 3339 value");
    let right = OffsetDateTime::parse(right.as_str(), &Rfc3339)
        .expect("UtcTimestamp guarantees a valid RFC 3339 value");
    left < right
}

fn valid_media_type(value: &str) -> bool {
    let Some((kind, subtype)) = value.split_once('/') else {
        return false;
    };
    !kind.is_empty()
        && !subtype.is_empty()
        && !subtype.contains('/')
        && kind.bytes().all(media_type_token_byte)
        && subtype.bytes().all(media_type_token_byte)
}

fn media_type_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
        )
}

fn validate_content_span(
    span: &ContentSpanV3,
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    if span.start_byte >= span.end_byte {
        violations.push(ContractViolation::new(
            "application_v3.source_span_invalid",
            pointer,
            "source span must be nonempty and ordered",
        ));
    }
}

fn validate_text_list(
    values: &[String],
    pointer: &str,
    maximum_items: usize,
    maximum_bytes: usize,
    violations: &mut Vec<ContractViolation>,
) {
    if values.len() > maximum_items {
        violations.push(ContractViolation::new(
            "application_v3.text_list_count_invalid",
            pointer,
            format!("list may contain at most {maximum_items} items"),
        ));
    }
    for (index, value) in values.iter().enumerate() {
        required_text(
            value,
            &format!("{pointer}/{index}"),
            maximum_bytes,
            violations,
        );
    }
}

fn unique_requirement_references(
    references: &[RequirementRevisionReferenceV3],
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    if references
        .iter()
        .map(|reference| &reference.id)
        .collect::<BTreeSet<_>>()
        .len()
        != references.len()
    {
        violations.push(ContractViolation::new(
            "application_v3.requirement_reference_duplicate",
            pointer,
            "Requirement references must be unique",
        ));
    }
}

fn validate_pack_owned_deliverable_kind(
    kind: &DeliverableKindId,
    pack: &ApplicationPackBindingV3,
    pointer: &str,
    violations: &mut Vec<ContractViolation>,
) {
    if kind.pack_id_str() != pack.id.as_str() {
        violations.push(ContractViolation::new(
            "application_v3.deliverable_kind_pack_mismatch",
            pointer,
            "Deliverable kind must belong to the exact bound Pack",
        ));
    }
}

fn extend_prefixed(
    target: &mut Vec<ContractViolation>,
    prefix: &str,
    nested: Vec<ContractViolation>,
) {
    target.extend(nested.into_iter().map(|mut violation| {
        violation.json_pointer = format!("{prefix}{}", violation.json_pointer);
        violation
    }));
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::to_value;

    use super::*;

    fn pack(id: &str, digest: char) -> ApplicationPackBindingV3 {
        ApplicationPackBindingV3 {
            id: WorkflowPackId::try_new(id).expect("pack ID"),
            version: SemanticVersion::try_new("1.0.0").expect("version"),
            content_digest: Sha256Digest::try_new(digest.to_string().repeat(64)).expect("digest"),
        }
    }

    fn item(id: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(id).expect("item ID")
    }

    fn revision(value: u64) -> Revision {
        Revision::try_new(value).expect("revision")
    }

    fn timestamp(value: &str) -> UtcTimestamp {
        UtcTimestamp::try_new(value).expect("timestamp")
    }

    fn entity_id(suffix: u16) -> EntityId {
        EntityId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}")).expect("entity ID")
    }

    fn opportunity_id(suffix: u16) -> OpportunityId {
        OpportunityId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}"))
            .expect("Opportunity ID")
    }

    fn application_id(suffix: u16) -> ApplicationId {
        ApplicationId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}"))
            .expect("Application ID")
    }

    fn requirement_id(suffix: u16) -> RequirementId {
        RequirementId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}"))
            .expect("Requirement ID")
    }

    fn plan_id(suffix: u16) -> PlanId {
        PlanId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}")).expect("Plan ID")
    }

    fn deliverable_id(suffix: u16) -> DeliverableId {
        DeliverableId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}"))
            .expect("Deliverable ID")
    }

    fn draft_snapshot() -> ApplicationModelSnapshotV3 {
        let pack = pack("org.canisend.generic-application", 'a');
        let opportunity_id = opportunity_id(601);
        let application_id = application_id(602);
        let mut opportunity_metadata = BTreeMap::new();
        opportunity_metadata.insert(
            item("programme-name"),
            ApplicationFieldValueV3::ShortText("Community funding round".to_owned()),
        );
        opportunity_metadata.insert(
            item("deadline"),
            ApplicationFieldValueV3::Date("2026-09-30".to_owned()),
        );
        ApplicationModelSnapshotV3 {
            format: ApplicationModelFormatV3::V3,
            pack: pack.clone(),
            opportunity: OpportunityRecordV3 {
                id: opportunity_id.clone(),
                pack: pack.clone(),
                title: "Local initiative funding".to_owned(),
                metadata: opportunity_metadata,
                source_ids: vec![entity_id(608)],
                created_at: timestamp("2026-08-02T12:00:00Z"),
                revision: revision(1),
                archived: false,
            },
            application: ApplicationRecordV3 {
                id: application_id,
                opportunity_id,
                pack: pack.clone(),
                metadata: BTreeMap::new(),
                lifecycle: ApplicationLifecycleV3::Draft,
                created_at: timestamp("2026-08-02T12:00:00.9Z"),
                updated_at: timestamp("2026-08-02T12:00:01Z"),
                revision: revision(1),
            },
            requirements: Vec::new(),
            plan: None,
            deliverables: Vec::new(),
        }
    }

    fn confirmed_snapshot() -> ApplicationModelSnapshotV3 {
        let mut snapshot = draft_snapshot();
        let requirement_id = requirement_id(603);
        let plan_id = plan_id(605);
        let deliverable_kind = DeliverableKindId::from_parts(&snapshot.pack.id, &item("proposal"));
        snapshot.application.lifecycle = ApplicationLifecycleV3::Active;
        snapshot.requirements.push(RequirementRecordV3 {
            id: requirement_id.clone(),
            application_id: snapshot.application.id.clone(),
            pack: snapshot.pack.clone(),
            category: item("eligibility"),
            statement: "Explain the intended public benefit.".to_owned(),
            priority: RequirementPriorityV3::Mandatory,
            source_span: ContentSpanV3 {
                content: ContentRevisionReferenceV3 {
                    id: entity_id(604),
                    revision: revision(1),
                    sha256: Sha256Digest::try_new("b".repeat(64)).expect("digest"),
                },
                start_byte: 24,
                end_byte: 59,
            },
            confirmation: RequirementConfirmationV3::Confirmed,
            confirmed_by: Some(ActorKind::User),
            confirmed_at: Some(timestamp("2026-08-02T12:02:00Z")),
            revision: revision(1),
        });
        snapshot.plan = Some(PlanRecordV3 {
            id: plan_id.clone(),
            application_id: snapshot.application.id.clone(),
            pack: snapshot.pack.clone(),
            state: PlanStateV3::Confirmed,
            decision: Some(item("proceed")),
            requirement_inputs: vec![RequirementRevisionReferenceV3 {
                id: requirement_id.clone(),
                revision: revision(1),
            }],
            deliverables: vec![PlannedDeliverableV3 {
                kind: deliverable_kind.clone(),
                disposition: PlannedDeliverableDispositionV3::Required,
                rationale: "The bound Pack requires one response.".to_owned(),
                constraints: vec!["Use plain language".to_owned()],
                execution_mode: Some(ExecutionMode::HostAgent),
            }],
            blockers: Vec::new(),
            decided_by: Some(ActorKind::User),
            decided_at: Some(timestamp("2026-08-02T12:03:00Z")),
            revision: revision(1),
        });
        snapshot.deliverables.push(DeliverableRecordV3 {
            id: deliverable_id(606),
            application_id: snapshot.application.id.clone(),
            pack: snapshot.pack.clone(),
            plan: PlanRevisionReferenceV3 {
                id: plan_id,
                revision: revision(1),
            },
            kind: deliverable_kind,
            title: "Public-benefit proposal".to_owned(),
            state: DeliverableStateV3::Draft,
            content: Some(ContentRevisionReferenceV3 {
                id: entity_id(607),
                revision: revision(1),
                sha256: Sha256Digest::try_new("c".repeat(64)).expect("digest"),
            }),
            media_type: Some("text/markdown".to_owned()),
            evidence_inputs: vec![EntityRevisionReferenceV3 {
                id: requirement_id.as_entity_id().clone(),
                revision: revision(1),
            }],
            revision: revision(1),
        });
        snapshot
    }

    fn violation_codes(snapshot: &ApplicationModelSnapshotV3) -> Vec<String> {
        snapshot
            .validate_semantics()
            .into_iter()
            .map(|violation| violation.code)
            .collect()
    }

    #[test]
    fn draft_without_requirements_or_evidence_is_valid() {
        let snapshot = draft_snapshot();
        assert!(snapshot.validate_semantics().is_empty());
        assert_eq!(
            validate_application_model_snapshot_v3(&to_value(&snapshot).expect("snapshot JSON"))
                .expect("valid external candidate"),
            snapshot
        );
    }

    #[test]
    fn confirmed_pack_bound_application_snapshot_is_valid() {
        let snapshot = confirmed_snapshot();
        assert!(snapshot.validate_semantics().is_empty());
        validate_application_model_snapshot_v3(&to_value(snapshot).expect("snapshot JSON"))
            .expect("valid external candidate");
    }

    #[test]
    fn exact_pack_binding_and_pack_owned_deliverables_are_enforced() {
        let mut snapshot = confirmed_snapshot();
        snapshot.requirements[0].pack = pack("org.canisend.generic-application", 'd');
        assert!(
            violation_codes(&snapshot)
                .iter()
                .any(|code| code == "application_v3.requirement_binding_mismatch")
        );

        let mut snapshot = confirmed_snapshot();
        let foreign_pack = WorkflowPackId::try_new("org.example.community-grant").expect("pack ID");
        snapshot.deliverables[0].kind =
            DeliverableKindId::from_parts(&foreign_pack, &item("proposal"));
        assert!(
            violation_codes(&snapshot)
                .iter()
                .any(|code| code == "application_v3.deliverable_kind_pack_mismatch")
        );
    }

    #[test]
    fn stale_requirement_and_plan_revisions_are_rejected() {
        let mut snapshot = confirmed_snapshot();
        snapshot.plan.as_mut().expect("plan").requirement_inputs[0].revision = revision(2);
        assert!(
            violation_codes(&snapshot)
                .iter()
                .any(|code| code == "application_v3.plan_requirement_revision_mismatch")
        );

        let mut snapshot = confirmed_snapshot();
        snapshot.deliverables[0].plan.revision = revision(2);
        assert!(
            violation_codes(&snapshot)
                .iter()
                .any(|code| code == "application_v3.deliverable_plan_revision_mismatch")
        );
    }

    #[test]
    fn stale_outputs_preserve_the_historical_revisions_that_produced_them() {
        let mut snapshot = confirmed_snapshot();
        snapshot.requirements[0].statement = "Explain the revised public benefit.".to_owned();
        snapshot.requirements[0].revision = revision(2);
        snapshot.plan.as_mut().expect("plan").state = PlanStateV3::Stale;
        snapshot.plan.as_mut().expect("plan").revision = revision(2);
        snapshot.deliverables[0].state = DeliverableStateV3::Stale;
        snapshot.deliverables[0].revision = revision(2);

        assert!(snapshot.validate_semantics().is_empty());
        assert_eq!(
            snapshot.plan.as_ref().expect("plan").requirement_inputs[0].revision,
            revision(1)
        );
        assert_eq!(snapshot.deliverables[0].plan.revision, revision(1));
    }

    #[test]
    fn confirmations_require_explicit_user_authority() {
        let mut snapshot = confirmed_snapshot();
        snapshot.requirements[0].confirmed_by = Some(ActorKind::HostAgent);
        assert!(
            violation_codes(&snapshot)
                .iter()
                .any(|code| code == "application_v3.requirement_confirmation_invalid")
        );

        let mut snapshot = confirmed_snapshot();
        snapshot.plan.as_mut().expect("plan").decided_by = Some(ActorKind::HostAgent);
        assert!(
            violation_codes(&snapshot)
                .iter()
                .any(|code| code == "application_v3.plan_decision_invalid")
        );
    }

    #[test]
    fn generic_metadata_and_materialized_content_are_bounded() {
        let mut snapshot = confirmed_snapshot();
        snapshot.opportunity.metadata.insert(
            item("website"),
            ApplicationFieldValueV3::Url("https://bad destination.example".to_owned()),
        );
        snapshot.application.metadata = (0..=APPLICATION_MODEL_V3_MAX_METADATA_FIELDS)
            .map(|index| {
                (
                    item(&format!("field-{index}")),
                    ApplicationFieldValueV3::Integer(index as i64),
                )
            })
            .collect();
        snapshot.deliverables[0].media_type = Some("text/markdown/extra".to_owned());
        let codes = violation_codes(&snapshot);
        assert!(
            codes
                .iter()
                .any(|code| code == "application_v3.url_invalid")
        );
        assert!(
            codes
                .iter()
                .any(|code| code == "application_v3.metadata_count_invalid")
        );
        assert!(
            codes
                .iter()
                .any(|code| code == "application_v3.media_type_invalid")
        );
    }

    #[test]
    fn entity_types_reject_non_uuidv7_values_at_deserialization() {
        assert!(serde_json::from_str::<ApplicationId>(r#""not-an-id""#).is_err());
        assert!(ApplicationId::try_new("019f2f55-7c00-6000-8000-000000000602").is_err());
    }
}
