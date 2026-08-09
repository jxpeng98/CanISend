use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use canisend_contracts::{
    APPLICATION_MODEL_V3_MAX_REQUIREMENTS, ActorKind, ApplicationFieldValueV3, ApplicationId,
    ApplicationLifecycleV3, ApplicationModelFormatV3, ApplicationModelSnapshotV3,
    ApplicationPackBindingV3, ApplicationRecordV3, ConsentScope, ContentRevisionReferenceV3,
    ContentSpanV3, DeliverableId, DeliverableKindId, DeliverableRecordV3, DeliverableStateV3,
    EntityRevisionReferenceV3, ExecutionMode, OpportunityId, OpportunityRecordV3, PlanId,
    PlanRecordV3, PlanRevisionReferenceV3, PlanStateV3, PlannedDeliverableDispositionV3,
    PlannedDeliverableV3, PrivacyClassification, RequirementConfirmationV3, RequirementId,
    RequirementPriorityV3, RequirementRecordV3, RequirementRevisionReferenceV3, Revision,
    SafeRelativePath, Sha256Digest, StageId, WorkflowPackFieldDefinition, WorkflowPackFieldType,
    WorkflowPackItemId, WorkflowPackStageOutput, WorkspaceSourceKindV4,
};
use canisend_core::{VerifiedWorkflowPackBundle, WorkflowPackDeliverableCatalogRuntime};
use canisend_io::{EmbeddedTypstCompiler, project_deliverable_typst_v3};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ApplicationAssociationServiceV4, ApplicationModelCommitResultV3, ApplicationModelRepository,
    ApplicationProjectionCatalogV3, ApplicationProjectionService, BlobStore,
    DEFAULT_MAX_BLOB_BYTES, Database, NewWorkspaceSourceV4, StoreError, StoredApplicationModelV3,
    association_v4::{prepare_source, validate_new_source_consent},
    generate_id, now_utc,
    render::{create_empty_export_directory, join_path, write_new_file},
};

pub const APPLICATION_FLOW_EXPORT_FORMAT_V3: &str = "canisend.application-flow-export/v3";
pub const MAX_APPLICATION_FLOW_SOURCE_BYTES_V3: usize = 4 * 1024 * 1024;
pub const MAX_APPLICATION_FLOW_DELIVERABLE_BYTES_V3: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowRequirementDraftV3 {
    pub category: WorkflowPackItemId,
    pub statement: String,
    pub priority: RequirementPriorityV3,
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowCreateRequestV3 {
    pub title: String,
    pub opportunity_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub application_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub source_text: String,
    pub requirements: Vec<ApplicationFlowRequirementDraftV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowPlannedDeliverableV3 {
    pub kind: WorkflowPackItemId,
    pub disposition: PlannedDeliverableDispositionV3,
    pub rationale: String,
    pub constraints: Vec<String>,
    pub execution_mode: Option<ExecutionMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowPlanRequestV3 {
    pub expected_revision: Revision,
    pub decision: WorkflowPackItemId,
    pub deliverables: Vec<ApplicationFlowPlannedDeliverableV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowDeliverableDraftV3 {
    pub kind: WorkflowPackItemId,
    pub title: String,
    pub media_type: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowComposeRequestV3 {
    pub expected_revision: Revision,
    pub deliverables: Vec<ApplicationFlowDeliverableDraftV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowApproveRequestV3 {
    pub expected_revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationFlowStageStateV3 {
    Pending,
    Ready,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowStageReadModelV3 {
    pub id: StageId,
    pub state: ApplicationFlowStageStateV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowReadModelV3 {
    pub stored: StoredApplicationModelV3,
    pub stages: Vec<ApplicationFlowStageReadModelV3>,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowCommitReadModelV3 {
    pub commit: ApplicationModelCommitResultV3,
    pub stages: Vec<ApplicationFlowStageReadModelV3>,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowReviewDeliverableV3 {
    pub deliverable: DeliverableRecordV3,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowReviewReadModelV3 {
    pub stored: StoredApplicationModelV3,
    pub deliverables: Vec<ApplicationFlowReviewDeliverableV3>,
    pub stages: Vec<ApplicationFlowStageReadModelV3>,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowRenderedDeliverableV3 {
    pub deliverable_id: DeliverableId,
    pub deliverable_revision: Revision,
    pub kind: DeliverableKindId,
    pub source_sha256: Sha256Digest,
    pub pdf_sha256: Sha256Digest,
    pub relative_path: SafeRelativePath,
    pub page_count: u32,
    pub byte_count: u64,
    pub warning_count: u32,
    pub elapsed_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowExportManifestV3 {
    pub format: String,
    pub application_id: ApplicationId,
    pub application_revision: Revision,
    pub snapshot_sha256: Sha256Digest,
    pub pack: ApplicationPackBindingV3,
    pub destination: SafeRelativePath,
    pub documents: Vec<ApplicationFlowRenderedDeliverableV3>,
    pub exported_at: canisend_contracts::UtcTimestamp,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFlowExportReadModelV3 {
    pub package: ApplicationProjectionCatalogV3,
    pub render: ApplicationFlowExportManifestV3,
    pub stages: Vec<ApplicationFlowStageReadModelV3>,
}

pub struct ApplicationFlowServiceV3<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
    workspace_root: &'a Path,
}

pub fn validate_application_flow_create_request(
    pack: &VerifiedWorkflowPackBundle,
    request: &ApplicationFlowCreateRequestV3,
) -> Result<(), StoreError> {
    if request.title.trim().is_empty()
        || request.title.len() > 512
        || request.title.chars().any(char::is_control)
    {
        return Err(StoreError::InvalidInput(
            "Application title must contain 1 to 512 non-control bytes".to_owned(),
        ));
    }
    validate_metadata(
        pack.manifest().application.opportunity_fields.as_slice(),
        &request.opportunity_metadata,
    )?;
    validate_metadata(
        pack.manifest().application.application_fields.as_slice(),
        &request.application_metadata,
    )?;
    validate_source_and_requirements(pack, &request.source_text, &request.requirements)
}

impl<'a> ApplicationFlowServiceV3<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore, workspace_root: &'a Path) -> Self {
        Self {
            database,
            blobs,
            workspace_root,
        }
    }

    pub fn create(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        request: ApplicationFlowCreateRequestV3,
    ) -> Result<ApplicationFlowReadModelV3, StoreError> {
        self.create_with_actor(pack, request, ActorKind::User)
    }

    pub fn create_with_actor(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        request: ApplicationFlowCreateRequestV3,
        actor: ActorKind,
    ) -> Result<ApplicationFlowReadModelV3, StoreError> {
        let normalized_text = request.source_text.clone();
        self.create_with_source_and_actor(
            pack,
            request,
            NewWorkspaceSourceV4 {
                kind: WorkspaceSourceKindV4::PastedText,
                locator: "pasted-text".to_owned(),
                final_locator: None,
                redirect_chain: Vec::new(),
                content_type: "text/plain; charset=utf-8".to_owned(),
                original_bytes: normalized_text.as_bytes().to_vec(),
                normalized_text,
                privacy: PrivacyClassification::PrivateLocal,
            },
            None,
            actor,
        )
    }

    pub fn create_with_source(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        request: ApplicationFlowCreateRequestV3,
        source: NewWorkspaceSourceV4,
        consent: Option<ConsentScope>,
    ) -> Result<ApplicationFlowReadModelV3, StoreError> {
        self.create_with_source_and_actor(pack, request, source, consent, ActorKind::User)
    }

    pub fn create_with_source_and_actor(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        request: ApplicationFlowCreateRequestV3,
        source: NewWorkspaceSourceV4,
        consent: Option<ConsentScope>,
        actor: ActorKind,
    ) -> Result<ApplicationFlowReadModelV3, StoreError> {
        validate_application_flow_create_request(pack, &request)?;
        if source.normalized_text != request.source_text {
            return Err(StoreError::ApplicationModelIntegrity(
                "Source normalized text differs from the validated Requirement span authority"
                    .to_owned(),
            ));
        }
        validate_new_source_consent(&source, consent)?;
        ApplicationModelRepository::new(self.database).authority()?;

        let binding = pack_binding(pack);
        let created_at = now_utc()?;
        let source_id = generate_id()?;
        let source_digest =
            Sha256Digest::try_new(hex::encode(Sha256::digest(request.source_text.as_bytes())))?;
        let opportunity_id = OpportunityId::try_new(generate_id()?.to_string())?;
        let application_id = ApplicationId::try_new(generate_id()?.to_string())?;
        let requirements = request
            .requirements
            .into_iter()
            .map(|draft| {
                Ok(RequirementRecordV3 {
                    id: RequirementId::try_new(generate_id()?.to_string())?,
                    application_id: application_id.clone(),
                    pack: binding.clone(),
                    category: draft.category,
                    statement: draft.statement,
                    priority: draft.priority,
                    source_span: ContentSpanV3 {
                        content: ContentRevisionReferenceV3 {
                            id: source_id.clone(),
                            revision: Revision::try_new(1)?,
                            sha256: source_digest.clone(),
                        },
                        start_byte: draft.start_byte,
                        end_byte: draft.end_byte,
                    },
                    confirmation: RequirementConfirmationV3::Proposed,
                    confirmed_by: None,
                    confirmed_at: None,
                    revision: Revision::try_new(1)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let snapshot = ApplicationModelSnapshotV3 {
            format: ApplicationModelFormatV3::V3,
            pack: binding.clone(),
            opportunity: OpportunityRecordV3 {
                id: opportunity_id.clone(),
                pack: binding.clone(),
                title: request.title,
                metadata: request.opportunity_metadata,
                source_ids: vec![source_id.clone()],
                created_at: created_at.clone(),
                revision: Revision::try_new(1)?,
                archived: false,
            },
            application: ApplicationRecordV3 {
                id: application_id,
                opportunity_id,
                pack: binding,
                metadata: request.application_metadata,
                lifecycle: ApplicationLifecycleV3::Draft,
                created_at: created_at.clone(),
                updated_at: created_at,
                revision: Revision::try_new(1)?,
            },
            requirements,
            plan: None,
            deliverables: Vec::new(),
        };
        crate::application_v3::validate_snapshot(&snapshot)?;
        let source = prepare_source(self.blobs, source, source_id, Revision::try_new(1)?)?;
        if source.record.normalized_sha256 != source_digest {
            return Err(StoreError::ApplicationModelIntegrity(
                "prepared Source digest differs from validated Source reference".to_owned(),
            ));
        }
        let commit = ApplicationModelRepository::new(self.database)
            .create_with_source_and_consent(
                snapshot,
                source,
                consent,
                actor,
                "application-flow-intake",
            )?;
        let stages = derive_stages(pack, &commit.stored.snapshot, false, false, false)?;
        Ok(ApplicationFlowReadModelV3 {
            stored: commit.stored,
            stages,
            submission_performed: false,
        })
    }

    pub fn status(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
    ) -> Result<ApplicationFlowReadModelV3, StoreError> {
        let stored = ApplicationModelRepository::new(self.database).get(application_id)?;
        if stored.snapshot.pack != pack_binding(pack) {
            return Err(StoreError::ApplicationModelConflict(
                "operation Pack does not match the exact Application Pack binding".to_owned(),
            ));
        }
        let evidence_current = self.has_current_evidence_association(application_id)?;
        let stages = derive_stages(pack, &stored.snapshot, evidence_current, false, false)?;
        Ok(ApplicationFlowReadModelV3 {
            stored,
            stages,
            submission_performed: false,
        })
    }

    pub fn confirm_requirements_and_plan(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: ApplicationFlowPlanRequestV3,
    ) -> Result<ApplicationFlowCommitReadModelV3, StoreError> {
        let current = self.current_for_pack(pack, application_id, request.expected_revision)?;
        if current.snapshot.requirements.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "at least one proposed Requirement is required before planning".to_owned(),
            ));
        }
        let catalog = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(pack)
            .map_err(pack_catalog_error)?;
        validate_plan_selection(&catalog, &request.deliverables)?;
        let decided_at = now_utc()?;
        let mut candidate = current.snapshot;
        candidate.application.lifecycle = ApplicationLifecycleV3::Active;
        candidate.application.updated_at = decided_at.clone();
        candidate.application.revision = next_revision(candidate.application.revision)?;
        for requirement in &mut candidate.requirements {
            requirement.confirmation = RequirementConfirmationV3::Confirmed;
            requirement.confirmed_by = Some(ActorKind::User);
            requirement.confirmed_at = Some(decided_at.clone());
            requirement.revision = next_revision(requirement.revision)?;
        }
        let plan_id = PlanId::try_new(generate_id()?.to_string())?;
        let requirement_inputs = candidate
            .requirements
            .iter()
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
            id: plan_id,
            application_id: application_id.clone(),
            pack: candidate.pack.clone(),
            state: PlanStateV3::Confirmed,
            decision: Some(request.decision),
            requirement_inputs,
            deliverables,
            blockers: Vec::new(),
            decided_by: Some(ActorKind::User),
            decided_at: Some(decided_at),
            revision: Revision::try_new(1)?,
        });
        let commit = ApplicationModelRepository::new(self.database).commit(
            application_id,
            request.expected_revision,
            candidate,
            ActorKind::User,
            "application-flow-plan",
        )?;
        self.commit_read_model(pack, commit)
    }

    pub fn compose(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: ApplicationFlowComposeRequestV3,
    ) -> Result<ApplicationFlowCommitReadModelV3, StoreError> {
        self.compose_with_actor(pack, application_id, request, ActorKind::User)
    }

    pub fn compose_with_actor(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: ApplicationFlowComposeRequestV3,
        actor: ActorKind,
    ) -> Result<ApplicationFlowCommitReadModelV3, StoreError> {
        let current = self.current_for_pack(pack, application_id, request.expected_revision)?;
        let plan = current.snapshot.plan.as_ref().ok_or_else(|| {
            StoreError::ApplicationModelConflict(
                "a confirmed Plan is required before composing Deliverables".to_owned(),
            )
        })?;
        if plan.state != PlanStateV3::Confirmed {
            return Err(StoreError::ApplicationModelConflict(
                "the current Plan is not confirmed".to_owned(),
            ));
        }
        if !current.snapshot.deliverables.is_empty() {
            return Err(StoreError::ApplicationModelConflict(
                "Deliverables are already materialized; revise them through a later operation"
                    .to_owned(),
            ));
        }
        let catalog = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(pack)
            .map_err(pack_catalog_error)?;
        validate_composed_deliverables(&catalog, &request.deliverables)?;

        let mut prepared = Vec::with_capacity(request.deliverables.len());
        for draft in request.deliverables {
            if draft.media_type != "text/plain" && draft.media_type != "text/markdown" {
                return Err(StoreError::InvalidInput(
                    "canonical v3 rendering accepts text/plain or text/markdown Deliverables"
                        .to_owned(),
                ));
            }
            if draft.content.trim().is_empty()
                || draft.content.len() > MAX_APPLICATION_FLOW_DELIVERABLE_BYTES_V3
            {
                return Err(StoreError::InvalidInput(
                    "Deliverable content must be nonempty and within the canonical v3 byte limit"
                        .to_owned(),
                ));
            }
            let digest =
                Sha256Digest::try_new(hex::encode(Sha256::digest(draft.content.as_bytes())))?;
            let id = DeliverableId::try_new(generate_id()?.to_string())?;
            prepared.push((draft, id, digest));
        }

        let updated_at = now_utc()?;
        let mut candidate = current.snapshot;
        candidate.application.updated_at = updated_at;
        candidate.application.revision = next_revision(candidate.application.revision)?;
        let plan = candidate
            .plan
            .as_ref()
            .expect("confirmed Plan checked before candidate creation");
        let evidence_inputs = ApplicationAssociationServiceV4::new(self.database, self.blobs)
            .evidence_associations(application_id)?
            .into_iter()
            .filter(|association| !association.stale)
            .map(|association| EntityRevisionReferenceV3 {
                id: association.evidence.id,
                revision: association.evidence.revision,
            })
            .collect::<Vec<_>>();
        candidate.deliverables = prepared
            .iter()
            .map(|(draft, id, digest)| {
                Ok(DeliverableRecordV3 {
                    id: id.clone(),
                    application_id: application_id.clone(),
                    pack: candidate.pack.clone(),
                    plan: PlanRevisionReferenceV3 {
                        id: plan.id.clone(),
                        revision: plan.revision,
                    },
                    kind: catalog.kind_id(&draft.kind),
                    title: draft.title.clone(),
                    state: DeliverableStateV3::ReviewRequired,
                    content: Some(ContentRevisionReferenceV3 {
                        id: id.as_entity_id().clone(),
                        revision: Revision::try_new(1)?,
                        sha256: digest.clone(),
                    }),
                    media_type: Some(draft.media_type.clone()),
                    evidence_inputs: evidence_inputs.clone(),
                    revision: Revision::try_new(1)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        crate::application_v3::validate_snapshot(&candidate)?;
        for (draft, _, expected_digest) in &prepared {
            let actual_digest = self.blobs.put_bytes(draft.content.as_bytes())?;
            if actual_digest != *expected_digest {
                return Err(StoreError::ApplicationModelIntegrity(
                    "prepared Deliverable digest differs from stored Blob".to_owned(),
                ));
            }
        }
        let commit = ApplicationModelRepository::new(self.database).commit(
            application_id,
            request.expected_revision,
            candidate,
            actor,
            "application-flow-compose",
        )?;
        self.commit_read_model(pack, commit)
    }

    pub fn approve(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        request: ApplicationFlowApproveRequestV3,
    ) -> Result<ApplicationFlowCommitReadModelV3, StoreError> {
        let current = self.current_for_pack(pack, application_id, request.expected_revision)?;
        if current.snapshot.deliverables.is_empty()
            || current
                .snapshot
                .deliverables
                .iter()
                .any(|deliverable| deliverable.state != DeliverableStateV3::ReviewRequired)
        {
            return Err(StoreError::ApplicationModelConflict(
                "all current Deliverables must require review before approval".to_owned(),
            ));
        }
        verify_content_blobs(self.blobs, &current.snapshot.deliverables)?;
        let mut candidate = current.snapshot;
        candidate.application.updated_at = now_utc()?;
        candidate.application.revision = next_revision(candidate.application.revision)?;
        for deliverable in &mut candidate.deliverables {
            deliverable.state = DeliverableStateV3::Approved;
            deliverable.revision = next_revision(deliverable.revision)?;
            deliverable
                .content
                .as_mut()
                .expect("materialized Deliverable content validated by v3")
                .revision = deliverable.revision;
        }
        let commit = ApplicationModelRepository::new(self.database).commit(
            application_id,
            request.expected_revision,
            candidate,
            ActorKind::User,
            "application-flow-approve",
        )?;
        self.commit_read_model(pack, commit)
    }

    pub fn review(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
    ) -> Result<ApplicationFlowReviewReadModelV3, StoreError> {
        let stored = ApplicationModelRepository::new(self.database).get(application_id)?;
        if stored.snapshot.pack != pack_binding(pack) {
            return Err(StoreError::ApplicationModelConflict(
                "operation Pack does not match the exact Application Pack binding".to_owned(),
            ));
        }
        let mut deliverables = Vec::with_capacity(stored.snapshot.deliverables.len());
        for deliverable in &stored.snapshot.deliverables {
            let content = deliverable.content.as_ref().ok_or_else(|| {
                StoreError::ApplicationModelIntegrity(
                    "materialized Deliverable has no content reference".to_owned(),
                )
            })?;
            let bytes = self.blobs.read_verified(
                &content.sha256,
                MAX_APPLICATION_FLOW_DELIVERABLE_BYTES_V3 as u64,
            )?;
            let content = String::from_utf8(bytes).map_err(|_| {
                StoreError::ApplicationModelIntegrity(
                    "Deliverable Blob is not valid UTF-8 text".to_owned(),
                )
            })?;
            deliverables.push(ApplicationFlowReviewDeliverableV3 {
                deliverable: deliverable.clone(),
                content,
            });
        }
        let evidence_current = self.has_current_evidence_association(application_id)?;
        let stages = derive_stages(pack, &stored.snapshot, evidence_current, false, false)?;
        Ok(ApplicationFlowReviewReadModelV3 {
            stored,
            deliverables,
            stages,
            submission_performed: false,
        })
    }

    pub fn export(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        application_id: &ApplicationId,
        expected_revision: Revision,
        destination: &SafeRelativePath,
    ) -> Result<ApplicationFlowExportReadModelV3, StoreError> {
        ensure_export_destination(application_id, destination)?;
        let current = self.current_for_pack(pack, application_id, expected_revision)?;
        if current.snapshot.deliverables.is_empty()
            || current
                .snapshot
                .deliverables
                .iter()
                .any(|deliverable| deliverable.state != DeliverableStateV3::Approved)
        {
            return Err(StoreError::ApplicationModelConflict(
                "all Deliverables must be approved before package and render".to_owned(),
            ));
        }
        let catalog = WorkflowPackDeliverableCatalogRuntime::from_verified_bundle(pack)
            .map_err(pack_catalog_error)?;
        validate_snapshot_deliverable_counts(&catalog, &current.snapshot.deliverables)?;
        verify_content_blobs(self.blobs, &current.snapshot.deliverables)?;

        let package =
            ApplicationProjectionService::new(self.database, self.blobs, self.workspace_root)
                .project(application_id)?;
        let compiler = EmbeddedTypstCompiler::new();
        let mut counts = BTreeMap::<String, u16>::new();
        let mut rendered = Vec::with_capacity(current.snapshot.deliverables.len());
        let mut files = Vec::with_capacity(current.snapshot.deliverables.len());
        for deliverable in &current.snapshot.deliverables {
            let descriptor = catalog.descriptor(&deliverable.kind).ok_or_else(|| {
                StoreError::ApplicationModelIntegrity(
                    "Deliverable kind is absent from the verified Pack".to_owned(),
                )
            })?;
            let template_path = descriptor.template().ok_or_else(|| {
                StoreError::ApplicationModelConflict(
                    "Deliverable kind has no verified render template".to_owned(),
                )
            })?;
            let template = pack.resources().get(template_path.path()).ok_or_else(|| {
                StoreError::ApplicationModelIntegrity(
                    "verified Pack template bytes are unavailable".to_owned(),
                )
            })?;
            let content_reference = deliverable
                .content
                .as_ref()
                .expect("approved Deliverable content validated above");
            let content = self
                .blobs
                .read_verified(&content_reference.sha256, DEFAULT_MAX_BLOB_BYTES)?;
            let source = project_deliverable_typst_v3(
                template,
                &current.snapshot.pack,
                current.snapshot.application.revision,
                deliverable,
                &content,
            )
            .map_err(|error| StoreError::InvalidInput(error.to_string()))?;
            let pdf = compiler.compile_pdf(&source)?;
            let local_id = deliverable.kind.local_id_str().to_owned();
            let index = counts.entry(local_id.clone()).or_insert(0);
            *index = index.checked_add(1).ok_or_else(|| {
                StoreError::Invariant("Deliverable export index overflow".to_owned())
            })?;
            let suffix = if *index == 1 {
                String::new()
            } else {
                format!("-{}", *index)
            };
            let relative_path = join_path(destination, &format!("{local_id}{suffix}.pdf"))?;
            let pdf_sha256 = Sha256Digest::try_new(hex::encode(Sha256::digest(pdf.bytes())))?;
            let byte_count = u64::try_from(pdf.bytes().len())
                .map_err(|_| StoreError::Invariant("render byte count overflow".to_owned()))?;
            let warning_count = u32::try_from(pdf.warning_count())
                .map_err(|_| StoreError::Invariant("render warning count overflow".to_owned()))?;
            let elapsed_millis = u64::try_from(pdf.elapsed().as_millis())
                .map_err(|_| StoreError::Invariant("render duration overflow".to_owned()))?;
            rendered.push(ApplicationFlowRenderedDeliverableV3 {
                deliverable_id: deliverable.id.clone(),
                deliverable_revision: deliverable.revision,
                kind: deliverable.kind.clone(),
                source_sha256: content_reference.sha256.clone(),
                pdf_sha256,
                relative_path: relative_path.clone(),
                page_count: pdf.page_count(),
                byte_count,
                warning_count,
                elapsed_millis,
            });
            files.push((relative_path, pdf.into_bytes()));
        }
        let exported_at = now_utc()?;
        let manifest = ApplicationFlowExportManifestV3 {
            format: APPLICATION_FLOW_EXPORT_FORMAT_V3.to_owned(),
            application_id: application_id.clone(),
            application_revision: current.snapshot.application.revision,
            snapshot_sha256: current.snapshot_sha256.clone(),
            pack: current.snapshot.pack.clone(),
            destination: destination.clone(),
            documents: rendered,
            exported_at: exported_at.clone(),
            submission_performed: false,
        };
        let manifest_path = join_path(destination, "render-manifest.json")?;
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
        create_empty_export_directory(self.workspace_root, destination)?;
        for (path, bytes) in &files {
            write_new_file(self.workspace_root, path, bytes)?;
        }
        write_new_file(self.workspace_root, &manifest_path, &manifest_bytes)?;
        self.database.connection().execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, 'user', 'application-flow.export', ?2, ?3,
                       'export-pack-bound-approved-deliverables', ?4)",
            params![
                generate_id()?.as_str(),
                application_id.as_str(),
                i64::try_from(current.snapshot.application.revision.get())
                    .map_err(|_| StoreError::Invariant("revision overflow".to_owned()))?,
                exported_at.as_str(),
            ],
        )?;
        let evidence_current = self.has_current_evidence_association(application_id)?;
        let stages = derive_stages(pack, &current.snapshot, evidence_current, true, true)?;
        Ok(ApplicationFlowExportReadModelV3 {
            package,
            render: manifest,
            stages,
        })
    }

    fn current_for_pack(
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

    fn commit_read_model(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        commit: ApplicationModelCommitResultV3,
    ) -> Result<ApplicationFlowCommitReadModelV3, StoreError> {
        let evidence_current =
            self.has_current_evidence_association(&commit.stored.snapshot.application.id)?;
        let stages = derive_stages(
            pack,
            &commit.stored.snapshot,
            evidence_current,
            false,
            false,
        )?;
        Ok(ApplicationFlowCommitReadModelV3 {
            commit,
            stages,
            submission_performed: false,
        })
    }

    fn has_current_evidence_association(
        &mut self,
        application_id: &ApplicationId,
    ) -> Result<bool, StoreError> {
        Ok(
            ApplicationAssociationServiceV4::new(self.database, self.blobs)
                .evidence_associations(application_id)?
                .into_iter()
                .any(|association| !association.stale),
        )
    }
}

fn pack_binding(pack: &VerifiedWorkflowPackBundle) -> ApplicationPackBindingV3 {
    ApplicationPackBindingV3 {
        id: pack.manifest().id.clone(),
        version: pack.manifest().version.clone(),
        content_digest: pack.manifest().content_digest.clone(),
    }
}

fn validate_source_and_requirements(
    pack: &VerifiedWorkflowPackBundle,
    source: &str,
    requirements: &[ApplicationFlowRequirementDraftV3],
) -> Result<(), StoreError> {
    if source.trim().is_empty() || source.len() > MAX_APPLICATION_FLOW_SOURCE_BYTES_V3 {
        return Err(StoreError::InvalidInput(
            "source text must be nonempty and within the canonical v3 byte limit".to_owned(),
        ));
    }
    if requirements.is_empty() {
        return Err(StoreError::InvalidInput(
            "at least one Requirement draft is required".to_owned(),
        ));
    }
    if requirements.len() > APPLICATION_MODEL_V3_MAX_REQUIREMENTS {
        return Err(StoreError::InvalidInput(format!(
            "an Application may propose at most {APPLICATION_MODEL_V3_MAX_REQUIREMENTS} Requirements"
        )));
    }
    let categories = pack
        .manifest()
        .requirements
        .categories
        .iter()
        .map(|category| &category.id)
        .collect::<BTreeSet<_>>();
    for requirement in requirements {
        if requirement.statement.trim().is_empty()
            || requirement.statement.len() > 16_384
            || requirement.statement.chars().any(char::is_control)
        {
            return Err(StoreError::InvalidInput(
                "Requirement statement must contain 1 to 16384 non-control bytes".to_owned(),
            ));
        }
        if !categories.contains(&requirement.category) {
            return Err(StoreError::InvalidInput(
                "Requirement category is not declared by the verified Pack".to_owned(),
            ));
        }
        let start = usize::try_from(requirement.start_byte)
            .map_err(|_| StoreError::InvalidInput("Requirement span is invalid".to_owned()))?;
        let end = usize::try_from(requirement.end_byte)
            .map_err(|_| StoreError::InvalidInput("Requirement span is invalid".to_owned()))?;
        if start >= end
            || end > source.len()
            || !source.is_char_boundary(start)
            || !source.is_char_boundary(end)
        {
            return Err(StoreError::InvalidInput(
                "Requirement span must select exact UTF-8 source bytes".to_owned(),
            ));
        }
        if source.get(start..end) != Some(requirement.statement.as_str()) {
            return Err(StoreError::InvalidInput(
                "Requirement statement must equal the exact selected source bytes".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_metadata(
    definitions: &[WorkflowPackFieldDefinition],
    values: &BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
) -> Result<(), StoreError> {
    let definitions = definitions
        .iter()
        .map(|definition| (&definition.id, definition))
        .collect::<BTreeMap<_, _>>();
    for definition in definitions.values() {
        if definition.required && !values.contains_key(&definition.id) {
            return Err(StoreError::InvalidInput(
                "required Pack metadata field is missing".to_owned(),
            ));
        }
    }
    for (id, value) in values {
        let definition = definitions.get(id).ok_or_else(|| {
            StoreError::InvalidInput(
                "metadata field is not declared by the verified Pack".to_owned(),
            )
        })?;
        let type_matches = matches!(
            (&definition.field_type, value),
            (
                WorkflowPackFieldType::ShortText,
                ApplicationFieldValueV3::ShortText(_)
            ) | (
                WorkflowPackFieldType::LongText,
                ApplicationFieldValueV3::LongText(_)
            ) | (
                WorkflowPackFieldType::Integer,
                ApplicationFieldValueV3::Integer(_)
            ) | (
                WorkflowPackFieldType::Boolean,
                ApplicationFieldValueV3::Boolean(_)
            ) | (
                WorkflowPackFieldType::Date,
                ApplicationFieldValueV3::Date(_)
            ) | (WorkflowPackFieldType::Url, ApplicationFieldValueV3::Url(_))
                | (
                    WorkflowPackFieldType::StringList,
                    ApplicationFieldValueV3::StringList(_)
                )
                | (
                    WorkflowPackFieldType::Choice,
                    ApplicationFieldValueV3::Choice(_)
                )
        );
        if !type_matches {
            return Err(StoreError::InvalidInput(
                "metadata value type does not match the verified Pack field".to_owned(),
            ));
        }
        if let ApplicationFieldValueV3::Choice(choice) = value
            && !definition.options.iter().any(|option| option.id == *choice)
        {
            return Err(StoreError::InvalidInput(
                "metadata choice is not declared by the verified Pack field".to_owned(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_plan_selection(
    catalog: &WorkflowPackDeliverableCatalogRuntime,
    planned: &[ApplicationFlowPlannedDeliverableV3],
) -> Result<(), StoreError> {
    let mut seen = BTreeSet::new();
    let mut counts = BTreeMap::new();
    for item in planned {
        if !seen.insert(&item.kind) {
            return Err(StoreError::InvalidInput(
                "Plan contains a duplicate Deliverable kind".to_owned(),
            ));
        }
        let kind = catalog.kind_id(&item.kind);
        if catalog.descriptor(&kind).is_none() {
            return Err(StoreError::InvalidInput(
                "Plan contains a Deliverable kind absent from the verified Pack".to_owned(),
            ));
        }
        if item.disposition != PlannedDeliverableDispositionV3::Omitted {
            counts.insert(kind, 1_u16);
        }
    }
    catalog.validate_counts(&counts).map_err(pack_catalog_error)
}

pub(crate) fn validate_composed_deliverables(
    catalog: &WorkflowPackDeliverableCatalogRuntime,
    deliverables: &[ApplicationFlowDeliverableDraftV3],
) -> Result<(), StoreError> {
    let mut counts = BTreeMap::new();
    for deliverable in deliverables {
        let kind = catalog.kind_id(&deliverable.kind);
        let count = counts.entry(kind).or_insert(0_u16);
        *count = count
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidInput("Deliverable count overflow".to_owned()))?;
    }
    catalog.validate_counts(&counts).map_err(pack_catalog_error)
}

fn validate_snapshot_deliverable_counts(
    catalog: &WorkflowPackDeliverableCatalogRuntime,
    deliverables: &[DeliverableRecordV3],
) -> Result<(), StoreError> {
    let mut counts = BTreeMap::new();
    for deliverable in deliverables {
        let count = counts.entry(deliverable.kind.clone()).or_insert(0_u16);
        *count = count
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidInput("Deliverable count overflow".to_owned()))?;
    }
    catalog.validate_counts(&counts).map_err(pack_catalog_error)
}

fn verify_content_blobs(
    blobs: &BlobStore,
    deliverables: &[DeliverableRecordV3],
) -> Result<(), StoreError> {
    for deliverable in deliverables {
        let content = deliverable.content.as_ref().ok_or_else(|| {
            StoreError::ApplicationModelIntegrity(
                "materialized Deliverable has no content reference".to_owned(),
            )
        })?;
        blobs.verify(&content.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    }
    Ok(())
}

fn derive_stages(
    pack: &VerifiedWorkflowPackBundle,
    snapshot: &ApplicationModelSnapshotV3,
    evidence_confirmed: bool,
    packaged: bool,
    rendered: bool,
) -> Result<Vec<ApplicationFlowStageReadModelV3>, StoreError> {
    let plan_confirmed = snapshot
        .plan
        .as_ref()
        .is_some_and(|plan| plan.state == PlanStateV3::Confirmed);
    let requirements_confirmed = !snapshot.requirements.is_empty()
        && snapshot
            .requirements
            .iter()
            .all(|requirement| requirement.confirmation != RequirementConfirmationV3::Proposed);
    let deliverables_materialized = !snapshot.deliverables.is_empty()
        && snapshot.deliverables.iter().all(|deliverable| {
            matches!(
                deliverable.state,
                DeliverableStateV3::ReviewRequired | DeliverableStateV3::Approved
            )
        });
    let review_complete = !snapshot.deliverables.is_empty()
        && snapshot
            .deliverables
            .iter()
            .all(|deliverable| deliverable.state == DeliverableStateV3::Approved);
    let mut complete = BTreeMap::<WorkflowPackItemId, bool>::new();
    for stage in &pack.manifest().workflow.stages {
        complete.insert(
            stage.id.clone(),
            match stage.output {
                WorkflowPackStageOutput::None => true,
                WorkflowPackStageOutput::Sources => !snapshot.opportunity.source_ids.is_empty(),
                WorkflowPackStageOutput::Requirements => requirements_confirmed,
                WorkflowPackStageOutput::Evidence => evidence_confirmed,
                WorkflowPackStageOutput::Matches => plan_confirmed,
                WorkflowPackStageOutput::Plan => plan_confirmed,
                WorkflowPackStageOutput::Deliverables => deliverables_materialized,
                WorkflowPackStageOutput::Review => review_complete,
                WorkflowPackStageOutput::Package => packaged,
                WorkflowPackStageOutput::Render => rendered,
            },
        );
    }
    let mut result = Vec::with_capacity(pack.manifest().workflow.stages.len());
    for stage in &pack.manifest().workflow.stages {
        let is_complete = complete.get(&stage.id).copied().unwrap_or(false);
        let dependencies_complete = stage
            .depends_on
            .iter()
            .all(|dependency| complete.get(dependency).copied().unwrap_or(false));
        let state = if is_complete {
            ApplicationFlowStageStateV3::Complete
        } else if dependencies_complete {
            ApplicationFlowStageStateV3::Ready
        } else {
            ApplicationFlowStageStateV3::Pending
        };
        result.push(ApplicationFlowStageReadModelV3 {
            id: StageId::from_parts(&snapshot.pack.id, &stage.id),
            state,
        });
    }
    Ok(result)
}

fn ensure_export_destination(
    application_id: &ApplicationId,
    destination: &SafeRelativePath,
) -> Result<(), StoreError> {
    let prefix = format!("applications/{application_id}/exports/");
    if !destination.as_str().starts_with(&prefix)
        || destination.as_str().trim_end_matches('/') == prefix.trim_end_matches('/')
    {
        return Err(StoreError::ProjectionPathRejected);
    }
    Ok(())
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

fn pack_catalog_error(error: impl std::fmt::Display) -> StoreError {
    StoreError::InvalidInput(format!(
        "verified Pack Deliverable catalog rejected input: {error}"
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_core::{
        WorkflowPackByteLoader, WorkflowPackCapabilityRegistry, WorkflowPackOrigin,
        WorkflowPackRuntime,
    };
    use canisend_resources::{
        EmbeddedWorkflowPack, academic_job_workflow_pack, generic_application_workflow_pack,
    };

    use super::*;
    use crate::Workspace;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn bundle(embedded: EmbeddedWorkflowPack) -> VerifiedWorkflowPackBundle {
        WorkflowPackByteLoader::verify(
            embedded.manifest_bytes(),
            embedded.into_resources(),
            WorkflowPackOrigin::BuiltIn,
            &WorkflowPackRuntime::parse(
                env!("CARGO_PKG_VERSION"),
                "3.0.0-alpha.1",
                "3.0.0-alpha.1",
            )
            .expect("runtime"),
            &WorkflowPackCapabilityRegistry::built_in(),
        )
        .expect("verified Pack")
        .into_bundle()
    }

    fn root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-application-flow-v3-{}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    #[test]
    fn current_evidence_associations_drive_stage_and_deliverable_inputs() {
        let root = root();
        let mut workspace = Workspace::init_v4(&root).expect("Workspace v4");
        let generic = bundle(generic_application_workflow_pack());
        let source = "Provide one project narrative.";
        let created =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create(
                    &generic,
                    ApplicationFlowCreateRequestV3 {
                        title: "Evidence association fixture".to_owned(),
                        opportunity_metadata: BTreeMap::new(),
                        application_metadata: BTreeMap::new(),
                        source_text: source.to_owned(),
                        requirements: vec![ApplicationFlowRequirementDraftV3 {
                            category: item("format"),
                            statement: source.to_owned(),
                            priority: RequirementPriorityV3::Mandatory,
                            start_byte: 0,
                            end_byte: u64::try_from(source.len()).expect("source length"),
                        }],
                    },
                )
                .expect("Application");
        let application_id = created.stored.snapshot.application.id;
        let evidence_id = generate_id().expect("Evidence ID");
        let evidence_digest = Sha256Digest::try_new(hex::encode(Sha256::digest(b"evidence-v1")))
            .expect("Evidence digest");
        let created_at = now_utc().expect("timestamp");
        workspace
            .database
            .connection()
            .execute(
                "INSERT INTO evidence_items(id, kind, created_at)
                 VALUES (?1, 'other', ?2)",
                params![evidence_id.as_str(), created_at.as_str()],
            )
            .expect("Evidence item");
        workspace
            .database
            .connection()
            .execute(
                "INSERT INTO evidence_revisions(
                    evidence_id, revision, sha256, confirmed, created_at, excluded, sensitivity
                 ) VALUES (?1, 1, ?2, 1, ?3, 0, 'public')",
                params![
                    evidence_id.as_str(),
                    evidence_digest.as_str(),
                    created_at.as_str()
                ],
            )
            .expect("Evidence revision");
        ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs)
            .associate_evidence(
                &application_id,
                &ContentRevisionReferenceV3 {
                    id: evidence_id.clone(),
                    revision: Revision::try_new(1).expect("revision"),
                    sha256: evidence_digest,
                },
                None,
                ActorKind::User,
            )
            .expect("Evidence association");

        let status =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .status(&generic, &application_id)
                .expect("status");
        assert!(status.stages.iter().any(|stage| {
            stage.id.local_id_str() == "evidence"
                && stage.state == ApplicationFlowStageStateV3::Complete
        }));
        ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
            .confirm_requirements_and_plan(
                &generic,
                &application_id,
                ApplicationFlowPlanRequestV3 {
                    expected_revision: Revision::try_new(1).expect("revision"),
                    decision: item("proceed"),
                    deliverables: vec![ApplicationFlowPlannedDeliverableV3 {
                        kind: item("primary-document"),
                        disposition: PlannedDeliverableDispositionV3::Required,
                        rationale: "Required by Pack".to_owned(),
                        constraints: Vec::new(),
                        execution_mode: Some(ExecutionMode::ManualImport),
                    }],
                },
            )
            .expect("Plan");
        let composed =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .compose(
                    &generic,
                    &application_id,
                    ApplicationFlowComposeRequestV3 {
                        expected_revision: Revision::try_new(2).expect("revision"),
                        deliverables: vec![ApplicationFlowDeliverableDraftV3 {
                            kind: item("primary-document"),
                            title: "Narrative".to_owned(),
                            media_type: "text/plain".to_owned(),
                            content: "Grounded in the explicitly associated Evidence.".to_owned(),
                        }],
                    },
                )
                .expect("Deliverable");
        assert_eq!(
            composed.commit.stored.snapshot.deliverables[0].evidence_inputs,
            vec![EntityRevisionReferenceV3 {
                id: evidence_id,
                revision: Revision::try_new(1).expect("revision"),
            }]
        );

        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn stale_and_wrong_pack_flow_requests_do_not_write_content_or_revisions() {
        let root = root();
        let mut workspace = Workspace::init(&root).expect("Workspace");
        ApplicationModelRepository::new(&mut workspace.database)
            .activate_empty_workspace(ActorKind::User, "new-workspace-v3")
            .expect("v3 authority");
        let generic = bundle(generic_application_workflow_pack());
        let academic = bundle(academic_job_workflow_pack());
        let source = "Provide a narrative and an appendix.";
        let mismatched =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create(
                    &generic,
                    ApplicationFlowCreateRequestV3 {
                        title: "Mismatched source span".to_owned(),
                        opportunity_metadata: BTreeMap::new(),
                        application_metadata: BTreeMap::new(),
                        source_text: source.to_owned(),
                        requirements: vec![ApplicationFlowRequirementDraftV3 {
                            category: item("format"),
                            statement: "Different statement".to_owned(),
                            priority: RequirementPriorityV3::Mandatory,
                            start_byte: 0,
                            end_byte: u64::try_from(source.len()).expect("source length"),
                        }],
                    },
                )
                .expect_err("mismatched source statement must fail");
        assert!(matches!(mismatched, StoreError::InvalidInput(_)));
        assert!(
            ApplicationModelRepository::new(&mut workspace.database)
                .list()
                .expect("list after mismatch")
                .is_empty()
        );
        let created =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create(
                    &generic,
                    ApplicationFlowCreateRequestV3 {
                        title: "Synthetic application".to_owned(),
                        opportunity_metadata: BTreeMap::new(),
                        application_metadata: BTreeMap::new(),
                        source_text: source.to_owned(),
                        requirements: vec![ApplicationFlowRequirementDraftV3 {
                            category: item("format"),
                            statement: source.to_owned(),
                            priority: RequirementPriorityV3::Mandatory,
                            start_byte: 0,
                            end_byte: u64::try_from(source.len()).expect("source length"),
                        }],
                    },
                )
                .expect("create");
        let application_id = created.stored.snapshot.application.id;
        let references_after_create = workspace.database.referenced_digests().expect("references");
        let blobs_after_create = workspace
            .blobs
            .audit(&references_after_create)
            .expect("blob audit")
            .present;
        let malformed =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create(
                    &generic,
                    ApplicationFlowCreateRequestV3 {
                        title: String::new(),
                        opportunity_metadata: BTreeMap::new(),
                        application_metadata: BTreeMap::new(),
                        source_text: "MALFORMED-SOURCE-MUST-NOT-BE-WRITTEN".to_owned(),
                        requirements: vec![ApplicationFlowRequirementDraftV3 {
                            category: item("format"),
                            statement: "MALFORMED-".to_owned(),
                            priority: RequirementPriorityV3::Mandatory,
                            start_byte: 0,
                            end_byte: 10,
                        }],
                    },
                )
                .expect_err("malformed create");
        assert!(matches!(malformed, StoreError::InvalidInput(_)));
        assert_eq!(
            workspace
                .blobs
                .audit(&workspace.database.referenced_digests().expect("references"))
                .expect("blob audit")
                .present,
            blobs_after_create
        );
        ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
            .confirm_requirements_and_plan(
                &generic,
                &application_id,
                ApplicationFlowPlanRequestV3 {
                    expected_revision: Revision::try_new(1).expect("revision"),
                    decision: item("proceed"),
                    deliverables: vec![
                        ApplicationFlowPlannedDeliverableV3 {
                            kind: item("primary-document"),
                            disposition: PlannedDeliverableDispositionV3::Required,
                            rationale: "Required by Pack".to_owned(),
                            constraints: Vec::new(),
                            execution_mode: Some(ExecutionMode::ManualImport),
                        },
                        ApplicationFlowPlannedDeliverableV3 {
                            kind: item("supporting-document"),
                            disposition: PlannedDeliverableDispositionV3::Optional,
                            rationale: "Requested appendix".to_owned(),
                            constraints: Vec::new(),
                            execution_mode: Some(ExecutionMode::ManualImport),
                        },
                    ],
                },
            )
            .expect("plan");

        let referenced_before = workspace.database.referenced_digests().expect("references");
        let blobs_before = workspace
            .blobs
            .audit(&referenced_before)
            .expect("blob audit")
            .present;
        let stale = ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
            .compose(
                &generic,
                &application_id,
                ApplicationFlowComposeRequestV3 {
                    expected_revision: Revision::try_new(1).expect("stale revision"),
                    deliverables: vec![ApplicationFlowDeliverableDraftV3 {
                        kind: item("primary-document"),
                        title: "Must not persist".to_owned(),
                        media_type: "text/plain".to_owned(),
                        content: "STALE-CONTENT-MUST-NOT-BE-WRITTEN".to_owned(),
                    }],
                },
            )
            .expect_err("stale compose");
        assert!(matches!(stale, StoreError::ApplicationModelConflict(_)));

        let wrong_pack =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .confirm_requirements_and_plan(
                    &academic,
                    &application_id,
                    ApplicationFlowPlanRequestV3 {
                        expected_revision: Revision::try_new(2).expect("revision"),
                        decision: item("proceed"),
                        deliverables: Vec::new(),
                    },
                )
                .expect_err("wrong Pack");
        assert!(matches!(
            wrong_pack,
            StoreError::ApplicationModelConflict(_)
        ));

        assert_eq!(
            ApplicationModelRepository::new(&mut workspace.database)
                .history(&application_id)
                .expect("history")
                .len(),
            2
        );
        let referenced_after = workspace.database.referenced_digests().expect("references");
        let audit_after = workspace
            .blobs
            .audit(&referenced_after)
            .expect("blob audit");
        assert_eq!(audit_after.present, blobs_before);
        assert!(audit_after.unreferenced.is_empty());

        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn local_and_new_url_sources_require_consent_before_blob_or_authority_mutation() {
        let root = root();
        let mut workspace = Workspace::init(&root).expect("Workspace");
        ApplicationModelRepository::new(&mut workspace.database)
            .activate_empty_workspace(ActorKind::User, "new-workspace-v4")
            .expect("v4 authority");
        let generic = bundle(generic_application_workflow_pack());
        let text = "One exact requirement.";
        let request = ApplicationFlowCreateRequestV3 {
            title: "Private local Source".to_owned(),
            opportunity_metadata: BTreeMap::new(),
            application_metadata: BTreeMap::new(),
            source_text: text.to_owned(),
            requirements: vec![ApplicationFlowRequirementDraftV3 {
                category: item("format"),
                statement: text.to_owned(),
                priority: RequirementPriorityV3::Mandatory,
                start_byte: 0,
                end_byte: u64::try_from(text.len()).expect("text length"),
            }],
        };
        let url_request = request.clone();
        let before = workspace.blobs.audit(&BTreeSet::new()).expect("blob audit");
        let error = ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
            .create_with_source(
                &generic,
                request,
                NewWorkspaceSourceV4 {
                    kind: WorkspaceSourceKindV4::LocalFile,
                    locator: "requirements.txt".to_owned(),
                    final_locator: None,
                    redirect_chain: Vec::new(),
                    content_type: "text/plain; charset=utf-8".to_owned(),
                    original_bytes: text.as_bytes().to_vec(),
                    normalized_text: text.to_owned(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                None,
            )
            .expect_err("private local Source consent");
        assert!(matches!(
            error,
            StoreError::ApplicationAssociationConsentRequired(_)
        ));
        let url_error =
            ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
                .create_with_source(
                    &generic,
                    url_request,
                    NewWorkspaceSourceV4 {
                        kind: WorkspaceSourceKindV4::Url,
                        locator: "https://example.invalid/start".to_owned(),
                        final_locator: Some("https://example.invalid/final".to_owned()),
                        redirect_chain: vec!["https://example.invalid/final".to_owned()],
                        content_type: "text/plain; charset=utf-8".to_owned(),
                        original_bytes: text.as_bytes().to_vec(),
                        normalized_text: text.to_owned(),
                        privacy: PrivacyClassification::Public,
                    },
                    None,
                )
                .expect_err("new URL Source consent");
        assert!(matches!(
            url_error,
            StoreError::ApplicationAssociationConsentRequired(_)
        ));
        assert!(
            ApplicationModelRepository::new(&mut workspace.database)
                .list()
                .expect("Applications")
                .is_empty()
        );
        assert_eq!(
            workspace.blobs.audit(&BTreeSet::new()).expect("blob audit"),
            before
        );
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
