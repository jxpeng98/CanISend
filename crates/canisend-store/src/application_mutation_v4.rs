use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    ActorKind, ApplicationId, ApplicationLifecycleV3, ApplicationPackBindingV3,
    ContentRevisionReferenceV3, DeliverableId, DeliverableStateV3, EntityRevisionReferenceV3,
    PlanId, PlanRecordV3, PlanRevisionReferenceV3, PlanStateV3, PlannedDeliverableV3,
    RequirementConfirmationV3, RequirementId, RequirementRevisionReferenceV3, Revision,
    Sha256Digest, WorkflowPackItemId,
};
use canisend_core::{VerifiedWorkflowPackBundle, WorkflowPackDeliverableCatalogRuntime};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ApplicationAssociationServiceV4, ApplicationFlowComposeRequestV3,
    ApplicationFlowPlannedDeliverableV3, ApplicationModelCommitResultV3,
    ApplicationModelRepository, BlobStore, Database, MAX_APPLICATION_FLOW_DELIVERABLE_BYTES_V3,
    StoreError, StoredApplicationModelV3, generate_id, now_utc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementDecisionV4 {
    Confirm,
    Exclude,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationRequirementConfirmRequestV4 {
    pub expected_revision: Revision,
    pub decisions: BTreeMap<RequirementId, RequirementDecisionV4>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPlanProposeRequestV4 {
    pub expected_revision: Revision,
    pub decision: WorkflowPackItemId,
    pub deliverables: Vec<ApplicationFlowPlannedDeliverableV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPlanConfirmRequestV4 {
    pub expected_revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDeliverableReviseRequestV4 {
    pub expected_revision: Revision,
    pub deliverable_id: DeliverableId,
    pub title: String,
    pub media_type: String,
    pub content: String,
}

pub struct ApplicationMutationServiceV4<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> ApplicationMutationServiceV4<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn validate_requirement_confirmation(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: &ApplicationRequirementConfirmRequestV4,
    ) -> Result<StoredApplicationModelV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        if current.snapshot.requirements.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "at least one proposed Requirement is required".to_owned(),
            ));
        }
        if current.snapshot.plan.is_some() || !current.snapshot.deliverables.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "Requirements cannot be decided after Plan or Deliverable creation".to_owned(),
            ));
        }
        let expected_ids = current
            .snapshot
            .requirements
            .iter()
            .map(|requirement| requirement.id.clone())
            .collect::<BTreeSet<_>>();
        let decided_ids = request.decisions.keys().cloned().collect::<BTreeSet<_>>();
        if expected_ids != decided_ids {
            return Err(StoreError::InvalidInput(
                "Requirement confirmation must decide every exact current Requirement once"
                    .to_owned(),
            ));
        }
        if current
            .snapshot
            .requirements
            .iter()
            .any(|requirement| requirement.confirmation != RequirementConfirmationV3::Proposed)
        {
            return Err(StoreError::ApplicationModelConflict(
                "Requirement decisions are already committed".to_owned(),
            ));
        }
        Ok(current)
    }

    pub fn validate_plan_proposal(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: &ApplicationPlanProposeRequestV4,
    ) -> Result<StoredApplicationModelV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        if current.snapshot.plan.is_some() || !current.snapshot.deliverables.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "a Plan already exists for this Application".to_owned(),
            ));
        }
        if current.snapshot.requirements.is_empty()
            || current
                .snapshot
                .requirements
                .iter()
                .any(|requirement| requirement.confirmation == RequirementConfirmationV3::Proposed)
        {
            return Err(StoreError::ApplicationModelConflict(
                "all Requirements require an explicit decision before Plan proposal".to_owned(),
            ));
        }
        if !current
            .snapshot
            .requirements
            .iter()
            .any(|requirement| requirement.confirmation == RequirementConfirmationV3::Confirmed)
        {
            return Err(StoreError::ApplicationModelConflict(
                "a Plan requires at least one confirmed Requirement".to_owned(),
            ));
        }
        let catalog = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(pack)
            .map_err(|error| StoreError::InvalidInput(error.to_string()))?;
        crate::application_flow_v3::validate_plan_selection(&catalog, &request.deliverables)?;
        Ok(current)
    }

    pub fn validate_plan_confirmation(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: &ApplicationPlanConfirmRequestV4,
    ) -> Result<StoredApplicationModelV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        if !current.snapshot.deliverables.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "a Plan cannot be confirmed after Deliverable creation".to_owned(),
            ));
        }
        let plan = current.snapshot.plan.as_ref().ok_or_else(|| {
            StoreError::ApplicationModelConflict(
                "a proposed Plan is required before confirmation".to_owned(),
            )
        })?;
        if plan.state != PlanStateV3::Draft {
            return Err(StoreError::ApplicationModelConflict(
                "the current Plan is not awaiting confirmation".to_owned(),
            ));
        }
        Ok(current)
    }

    pub fn validate_deliverable_revision(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: &ApplicationDeliverableReviseRequestV4,
    ) -> Result<StoredApplicationModelV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        validate_deliverable_text(&request.title, &request.media_type, &request.content)?;
        let plan = current.snapshot.plan.as_ref().ok_or_else(|| {
            StoreError::ApplicationModelConflict(
                "a confirmed Plan is required before Deliverable revision".to_owned(),
            )
        })?;
        if plan.state != PlanStateV3::Confirmed {
            return Err(StoreError::ApplicationModelConflict(
                "the current Plan is not confirmed".to_owned(),
            ));
        }
        if !current
            .snapshot
            .deliverables
            .iter()
            .any(|deliverable| deliverable.id == request.deliverable_id)
        {
            return Err(StoreError::ApplicationModelConflict(
                "Deliverable does not belong to the selected Application".to_owned(),
            ));
        }
        Ok(current)
    }

    pub fn validate_deliverable_draft(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: &ApplicationFlowComposeRequestV3,
    ) -> Result<StoredApplicationModelV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        let plan = current.snapshot.plan.as_ref().ok_or_else(|| {
            StoreError::ApplicationModelConflict(
                "a confirmed Plan is required before drafting Deliverables".to_owned(),
            )
        })?;
        if plan.state != PlanStateV3::Confirmed {
            return Err(StoreError::ApplicationModelConflict(
                "the current Plan is not confirmed".to_owned(),
            ));
        }
        if !current.snapshot.deliverables.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "Deliverables are already materialized; use Deliverable revise".to_owned(),
            ));
        }
        let catalog = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(pack)
            .map_err(|error| StoreError::InvalidInput(error.to_string()))?;
        crate::application_flow_v3::validate_composed_deliverables(
            &catalog,
            &request.deliverables,
        )?;
        for deliverable in &request.deliverables {
            validate_deliverable_text(
                &deliverable.title,
                &deliverable.media_type,
                &deliverable.content,
            )?;
        }
        Ok(current)
    }

    pub fn confirm_requirements(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: ApplicationRequirementConfirmRequestV4,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        if current.snapshot.requirements.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "at least one proposed Requirement is required".to_owned(),
            ));
        }
        if current.snapshot.plan.is_some() || !current.snapshot.deliverables.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "Requirements cannot be decided after Plan or Deliverable creation".to_owned(),
            ));
        }
        let expected_ids = current
            .snapshot
            .requirements
            .iter()
            .map(|requirement| requirement.id.clone())
            .collect::<BTreeSet<_>>();
        let decided_ids = request.decisions.keys().cloned().collect::<BTreeSet<_>>();
        if expected_ids != decided_ids {
            return Err(StoreError::InvalidInput(
                "Requirement confirmation must decide every exact current Requirement once"
                    .to_owned(),
            ));
        }
        if current
            .snapshot
            .requirements
            .iter()
            .any(|requirement| requirement.confirmation != RequirementConfirmationV3::Proposed)
        {
            return Err(StoreError::ApplicationModelConflict(
                "Requirement decisions are already committed".to_owned(),
            ));
        }

        let decided_at = now_utc()?;
        let mut candidate = current.snapshot;
        candidate.application.updated_at = decided_at.clone();
        candidate.application.revision = next_revision(candidate.application.revision)?;
        for requirement in &mut candidate.requirements {
            requirement.confirmation = match request
                .decisions
                .get(&requirement.id)
                .expect("exact decision set validated above")
            {
                RequirementDecisionV4::Confirm => RequirementConfirmationV3::Confirmed,
                RequirementDecisionV4::Exclude => RequirementConfirmationV3::Excluded,
            };
            requirement.confirmed_by = Some(ActorKind::User);
            requirement.confirmed_at = Some(decided_at.clone());
            requirement.revision = next_revision(requirement.revision)?;
        }
        crate::application_v3::validate_snapshot(&candidate)?;
        ApplicationModelRepository::new(self.database).commit(
            application_id,
            request.expected_revision,
            candidate,
            ActorKind::User,
            "application-v4-requirement-confirm",
        )
    }

    pub fn propose_plan(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: ApplicationPlanProposeRequestV4,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        if current.snapshot.plan.is_some() || !current.snapshot.deliverables.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "a Plan already exists for this Application".to_owned(),
            ));
        }
        if current.snapshot.requirements.is_empty()
            || current
                .snapshot
                .requirements
                .iter()
                .any(|requirement| requirement.confirmation == RequirementConfirmationV3::Proposed)
        {
            return Err(StoreError::ApplicationModelConflict(
                "all Requirements require an explicit decision before Plan proposal".to_owned(),
            ));
        }
        if !current
            .snapshot
            .requirements
            .iter()
            .any(|requirement| requirement.confirmation == RequirementConfirmationV3::Confirmed)
        {
            return Err(StoreError::ApplicationModelConflict(
                "a Plan requires at least one confirmed Requirement".to_owned(),
            ));
        }
        let catalog = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(pack)
            .map_err(|error| StoreError::InvalidInput(error.to_string()))?;
        crate::application_flow_v3::validate_plan_selection(&catalog, &request.deliverables)?;

        let updated_at = now_utc()?;
        let mut candidate = current.snapshot;
        candidate.application.lifecycle = ApplicationLifecycleV3::Active;
        candidate.application.updated_at = updated_at;
        candidate.application.revision = next_revision(candidate.application.revision)?;
        let requirement_inputs = candidate
            .requirements
            .iter()
            .filter(|requirement| requirement.confirmation == RequirementConfirmationV3::Confirmed)
            .map(|requirement| RequirementRevisionReferenceV3 {
                id: requirement.id.clone(),
                revision: requirement.revision,
            })
            .collect();
        let deliverables = request
            .deliverables
            .into_iter()
            .map(|planned| PlannedDeliverableV3 {
                kind: catalog.kind_id(&planned.kind),
                disposition: planned.disposition,
                rationale: planned.rationale,
                constraints: planned.constraints,
                execution_mode: planned.execution_mode,
            })
            .collect();
        candidate.plan = Some(PlanRecordV3 {
            id: PlanId::try_new(generate_id()?.to_string())?,
            application_id: application_id.clone(),
            pack: candidate.pack.clone(),
            state: PlanStateV3::Draft,
            decision: Some(request.decision),
            requirement_inputs,
            deliverables,
            blockers: Vec::new(),
            decided_by: None,
            decided_at: None,
            revision: Revision::try_new(1)?,
        });
        crate::application_v3::validate_snapshot(&candidate)?;
        ApplicationModelRepository::new(self.database).commit(
            application_id,
            request.expected_revision,
            candidate,
            ActorKind::HostAgent,
            "application-v4-plan-propose",
        )
    }

    pub fn confirm_plan(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: ApplicationPlanConfirmRequestV4,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        if !current.snapshot.deliverables.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "a Plan cannot be confirmed after Deliverable creation".to_owned(),
            ));
        }
        let mut candidate = current.snapshot;
        let decided_at = now_utc()?;
        let plan = candidate.plan.as_mut().ok_or_else(|| {
            StoreError::ApplicationModelConflict(
                "a proposed Plan is required before confirmation".to_owned(),
            )
        })?;
        if plan.state != PlanStateV3::Draft {
            return Err(StoreError::ApplicationModelConflict(
                "the current Plan is not awaiting confirmation".to_owned(),
            ));
        }
        plan.state = PlanStateV3::Confirmed;
        plan.decided_by = Some(ActorKind::User);
        plan.decided_at = Some(decided_at.clone());
        plan.revision = next_revision(plan.revision)?;
        candidate.application.updated_at = decided_at;
        candidate.application.revision = next_revision(candidate.application.revision)?;
        crate::application_v3::validate_snapshot(&candidate)?;
        ApplicationModelRepository::new(self.database).commit(
            application_id,
            request.expected_revision,
            candidate,
            ActorKind::User,
            "application-v4-plan-confirm",
        )
    }

    pub fn revise_deliverable(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: ApplicationDeliverableReviseRequestV4,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        let current = self.current(pack, application_id, request.expected_revision)?;
        validate_deliverable_text(&request.title, &request.media_type, &request.content)?;
        let digest =
            Sha256Digest::try_new(hex::encode(Sha256::digest(request.content.as_bytes())))?;
        let evidence_inputs = ApplicationAssociationServiceV4::new(self.database, self.blobs)
            .evidence_associations(application_id)?
            .into_iter()
            .filter(|association| !association.stale)
            .map(|association| EntityRevisionReferenceV3 {
                id: association.evidence.id,
                revision: association.evidence.revision,
            })
            .collect::<Vec<_>>();
        let mut candidate = current.snapshot;
        let plan = candidate.plan.as_ref().ok_or_else(|| {
            StoreError::ApplicationModelConflict(
                "a confirmed Plan is required before Deliverable revision".to_owned(),
            )
        })?;
        if plan.state != PlanStateV3::Confirmed {
            return Err(StoreError::ApplicationModelConflict(
                "the current Plan is not confirmed".to_owned(),
            ));
        }
        let deliverable = candidate
            .deliverables
            .iter_mut()
            .find(|deliverable| deliverable.id == request.deliverable_id)
            .ok_or_else(|| {
                StoreError::ApplicationModelConflict(
                    "Deliverable does not belong to the selected Application".to_owned(),
                )
            })?;
        let revision = next_revision(deliverable.revision)?;
        deliverable.title = request.title;
        deliverable.media_type = Some(request.media_type);
        deliverable.content = Some(ContentRevisionReferenceV3 {
            id: deliverable.id.as_entity_id().clone(),
            revision,
            sha256: digest.clone(),
        });
        deliverable.state = DeliverableStateV3::ReviewRequired;
        deliverable.evidence_inputs = evidence_inputs;
        deliverable.revision = revision;
        deliverable.plan = PlanRevisionReferenceV3 {
            id: plan.id.clone(),
            revision: plan.revision,
        };
        candidate.application.updated_at = now_utc()?;
        candidate.application.revision = next_revision(candidate.application.revision)?;
        crate::application_v3::validate_snapshot(&candidate)?;
        let stored_digest = self.blobs.put_bytes(request.content.as_bytes())?;
        if stored_digest != digest {
            return Err(StoreError::ApplicationModelIntegrity(
                "revised Deliverable digest differs from stored Blob".to_owned(),
            ));
        }
        ApplicationModelRepository::new(self.database).commit(
            application_id,
            request.expected_revision,
            candidate,
            ActorKind::HostAgent,
            "application-v4-deliverable-revise",
        )
    }

    fn current(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        expected_revision: Revision,
    ) -> Result<StoredApplicationModelV3, StoreError> {
        let current = ApplicationModelRepository::new(self.database).get(application_id)?;
        if current.snapshot.application.revision != expected_revision {
            return Err(StoreError::ApplicationModelConflict(format!(
                "expected Application revision {}, found {}",
                expected_revision.get(),
                current.snapshot.application.revision.get()
            )));
        }
        if current.snapshot.pack != pack_binding(pack) {
            return Err(StoreError::ApplicationModelConflict(
                "operation Pack does not match the exact Application Pack binding".to_owned(),
            ));
        }
        Ok(current)
    }
}

fn validate_deliverable_text(
    title: &str,
    media_type: &str,
    content: &str,
) -> Result<(), StoreError> {
    if title.trim().is_empty() || title.len() > 512 {
        return Err(StoreError::InvalidInput(
            "Deliverable title must contain 1 to 512 bytes".to_owned(),
        ));
    }
    if media_type != "text/plain" && media_type != "text/markdown" {
        return Err(StoreError::InvalidInput(
            "Deliverable media type must be text/plain or text/markdown".to_owned(),
        ));
    }
    if content.trim().is_empty() || content.len() > MAX_APPLICATION_FLOW_DELIVERABLE_BYTES_V3 {
        return Err(StoreError::InvalidInput(
            "Deliverable content must be nonempty and within the canonical byte limit".to_owned(),
        ));
    }
    Ok(())
}

fn pack_binding(pack: &VerifiedWorkflowPackBundle) -> ApplicationPackBindingV3 {
    ApplicationPackBindingV3 {
        id: pack.manifest().id.clone(),
        version: pack.manifest().version.clone(),
        content_digest: pack.manifest().content_digest.clone(),
    }
}

fn next_revision(revision: Revision) -> Result<Revision, StoreError> {
    Revision::try_new(
        revision
            .get()
            .checked_add(1)
            .ok_or_else(|| StoreError::Invariant("revision overflow".to_owned()))?,
    )
    .map_err(StoreError::from)
}
