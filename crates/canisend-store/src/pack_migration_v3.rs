use std::collections::{BTreeMap, BTreeSet, VecDeque};

use canisend_contracts::{
    ActorKind, ApplicationFieldValueV3, ApplicationId, ApplicationModelSnapshotV3,
    ApplicationPackBindingV3, DeliverableId, DeliverableKindId, DeliverableStateV3, PlanId,
    PlanStateV3, Revision, SafeRelativePath, Sha256Digest, WorkflowPackFieldDefinition,
    WorkflowPackFieldType, WorkflowPackIdMapping, WorkflowPackItemId, WorkflowPackManifest,
    WorkflowPackMigrationKind, WorkflowPackStageOutput,
};
use canisend_core::VerifiedWorkflowPackBundle;
use rusqlite::{Connection, params};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::{
    Database, StoreError, StoredApplicationModelV3,
    application_storage::ApplicationStorage,
    application_v3::{
        enum_name, insert_audit, insert_content_blob_references, insert_dependencies,
        insert_revision, load_current, next_revision, serialize_snapshot, to_i64, validate_reason,
        validate_snapshot,
    },
    generate_id, now_utc,
};

pub const APPLICATION_PACK_MIGRATION_FORMAT_V3: &str = "canisend.application-pack-migration/v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPackMigrationImpactV3 {
    pub plan_invalidated: bool,
    pub stale_plan_ids: Vec<PlanId>,
    pub stale_deliverable_ids: Vec<DeliverableId>,
    pub rebound_requirement_count: u64,
    pub rebound_deliverable_count: u64,
    pub superseded_projection_paths: Vec<SafeRelativePath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPackMigrationPreviewV3 {
    pub format: String,
    pub application_id: ApplicationId,
    pub from_application_revision: Revision,
    pub to_application_revision: Revision,
    pub source_pack: ApplicationPackBindingV3,
    pub target_pack: ApplicationPackBindingV3,
    pub source_manifest_sha256: Sha256Digest,
    pub target_manifest_sha256: Sha256Digest,
    pub impact: ApplicationPackMigrationImpactV3,
    pub preview_sha256: Sha256Digest,
    pub inspected_at: canisend_contracts::UtcTimestamp,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationPackMigrationResultV3 {
    pub format: String,
    pub migration_id: canisend_contracts::EntityId,
    pub stored: StoredApplicationModelV3,
    pub source_pack: ApplicationPackBindingV3,
    pub target_pack: ApplicationPackBindingV3,
    pub impact: ApplicationPackMigrationImpactV3,
    pub preview_sha256: Sha256Digest,
    pub migrated_at: canisend_contracts::UtcTimestamp,
    pub submission_performed: bool,
}

pub struct ApplicationPackMigrationService<'a> {
    database: &'a mut Database,
}

impl<'a> ApplicationPackMigrationService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database) -> Self {
        Self { database }
    }

    pub fn preview(
        &mut self,
        application_id: &ApplicationId,
        source: &VerifiedWorkflowPackBundle,
        target: &VerifiedWorkflowPackBundle,
    ) -> Result<ApplicationPackMigrationPreviewV3, StoreError> {
        let current = load_current(self.database.connection(), application_id)?;
        let projection_paths = load_current_projection_paths(
            self.database.connection(),
            application_id,
            current.snapshot.application.revision,
        )?;
        let plan = build_migration_plan(&current, source, target, projection_paths)?;
        plan.preview(now_utc()?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn migrate(
        &mut self,
        application_id: &ApplicationId,
        expected_revision: Revision,
        expected_preview_sha256: &Sha256Digest,
        source: &VerifiedWorkflowPackBundle,
        target: &VerifiedWorkflowPackBundle,
        actor: ActorKind,
        reason: &str,
    ) -> Result<ApplicationPackMigrationResultV3, StoreError> {
        let reason = validate_reason(reason)?.to_owned();
        let migrated_at = now_utc()?;
        let migration_id = generate_id()?;
        let audit_id = generate_id()?;
        let actor_name = enum_name(actor)?;
        let transaction = self.database.immediate_transaction()?;
        let storage = ApplicationStorage::detect(&transaction)?;
        let current = load_current(&transaction, application_id)?;
        if current.snapshot.application.revision != expected_revision {
            return Err(StoreError::TaskStale(format!(
                "Pack migration expected Application revision {}, found {}",
                expected_revision.get(),
                current.snapshot.application.revision.get()
            )));
        }
        let projection_paths = load_current_projection_paths(
            &transaction,
            application_id,
            current.snapshot.application.revision,
        )?;
        let plan = build_migration_plan(&current, source, target, projection_paths)?;
        if &plan.preview_sha256 != expected_preview_sha256 {
            return Err(StoreError::TaskStale(
                "Pack migration preview is stale; inspect the current Application and projections again"
                    .to_owned(),
            ));
        }
        let candidate = plan.migrated_snapshot(&current.snapshot, &migrated_at)?;
        validate_pack_migration_transition(&current.snapshot, &candidate)?;
        validate_snapshot(&candidate)?;
        let (snapshot_json, snapshot_sha256) = serialize_snapshot(&candidate)?;
        insert_revision(
            &transaction,
            &candidate,
            &snapshot_json,
            &snapshot_sha256,
            &actor_name,
            &reason,
            &migrated_at,
        )?;
        insert_content_blob_references(&transaction, &candidate, &migrated_at)?;
        let updated = transaction.execute(
            &format!(
                "UPDATE {}
             SET opportunity_id = ?2, pack_id = ?3, pack_version = ?4, pack_digest = ?5,
                 head_revision = ?6, updated_at = ?7
             WHERE application_id = ?1 AND head_revision = ?8
               AND pack_id = ?9 AND pack_version = ?10 AND pack_digest = ?11",
                storage.heads()
            ),
            params![
                application_id.as_str(),
                candidate.opportunity.id.as_str(),
                candidate.pack.id.as_str(),
                candidate.pack.version.as_str(),
                candidate.pack.content_digest.as_str(),
                to_i64(candidate.application.revision.get())?,
                candidate.application.updated_at.as_str(),
                to_i64(current.snapshot.application.revision.get())?,
                current.snapshot.pack.id.as_str(),
                current.snapshot.pack.version.as_str(),
                current.snapshot.pack.content_digest.as_str(),
            ],
        )?;
        if updated != 1 {
            return Err(StoreError::TaskStale(
                "Application head or Pack binding changed during migration".to_owned(),
            ));
        }
        insert_dependencies(&transaction, &candidate)?;
        insert_audit(
            &transaction,
            audit_id.as_str(),
            &actor_name,
            match storage {
                ApplicationStorage::V3 => "application-v3.pack-migrate",
                ApplicationStorage::V4 => "application-v4.pack-migrate",
            },
            application_id,
            candidate.application.revision,
            &reason,
            &migrated_at,
        )?;
        transaction.execute(
            &format!(
                "INSERT INTO {}(
                id, application_id, from_application_revision, to_application_revision,
                pack_id, from_pack_version, from_pack_digest, to_pack_version, to_pack_digest,
                source_manifest_sha256, target_manifest_sha256, preview_sha256,
                plan_invalidated, stale_deliverable_count, actor, reason, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17)",
                storage.pack_migrations()
            ),
            params![
                migration_id.as_str(),
                application_id.as_str(),
                to_i64(current.snapshot.application.revision.get())?,
                to_i64(candidate.application.revision.get())?,
                candidate.pack.id.as_str(),
                current.snapshot.pack.version.as_str(),
                current.snapshot.pack.content_digest.as_str(),
                candidate.pack.version.as_str(),
                candidate.pack.content_digest.as_str(),
                plan.source_manifest_sha256.as_str(),
                plan.target_manifest_sha256.as_str(),
                plan.preview_sha256.as_str(),
                i64::from(plan.impact.plan_invalidated),
                i64::try_from(plan.impact.stale_deliverable_ids.len()).map_err(|_| {
                    StoreError::Invariant("stale Deliverable count exceeds SQLite i64".to_owned())
                })?,
                actor_name,
                reason,
                migrated_at.as_str(),
            ],
        )?;
        transaction.commit()?;
        Ok(ApplicationPackMigrationResultV3 {
            format: APPLICATION_PACK_MIGRATION_FORMAT_V3.to_owned(),
            migration_id,
            stored: StoredApplicationModelV3 {
                snapshot: candidate,
                snapshot_sha256,
                committed_at: migrated_at.clone(),
            },
            source_pack: plan.source_pack,
            target_pack: plan.target_pack,
            impact: plan.impact,
            preview_sha256: plan.preview_sha256,
            migrated_at,
            submission_performed: false,
        })
    }
}

#[derive(Debug, Clone)]
struct MigrationPlan {
    application_id: ApplicationId,
    from_application_revision: Revision,
    to_application_revision: Revision,
    source_snapshot_sha256: Sha256Digest,
    source_pack: ApplicationPackBindingV3,
    target_pack: ApplicationPackBindingV3,
    source_manifest_sha256: Sha256Digest,
    target_manifest_sha256: Sha256Digest,
    impact: ApplicationPackMigrationImpactV3,
    maps: MigrationMaps,
    preview_sha256: Sha256Digest,
}

impl MigrationPlan {
    fn preview(
        &self,
        inspected_at: canisend_contracts::UtcTimestamp,
    ) -> Result<ApplicationPackMigrationPreviewV3, StoreError> {
        Ok(ApplicationPackMigrationPreviewV3 {
            format: APPLICATION_PACK_MIGRATION_FORMAT_V3.to_owned(),
            application_id: self.application_id.clone(),
            from_application_revision: self.from_application_revision,
            to_application_revision: self.to_application_revision,
            source_pack: self.source_pack.clone(),
            target_pack: self.target_pack.clone(),
            source_manifest_sha256: self.source_manifest_sha256.clone(),
            target_manifest_sha256: self.target_manifest_sha256.clone(),
            impact: self.impact.clone(),
            preview_sha256: self.preview_sha256.clone(),
            inspected_at,
            submission_performed: false,
        })
    }

    fn migrated_snapshot(
        &self,
        current: &ApplicationModelSnapshotV3,
        migrated_at: &canisend_contracts::UtcTimestamp,
    ) -> Result<ApplicationModelSnapshotV3, StoreError> {
        let mut candidate = current.clone();
        candidate.pack = self.target_pack.clone();
        candidate.application.pack = self.target_pack.clone();
        candidate.application.revision = self.to_application_revision;
        candidate.application.updated_at = migrated_at.clone();
        candidate.application.metadata = map_metadata(
            &candidate.application.metadata,
            &self.maps.fields,
            "Application",
        )?;
        candidate.opportunity.pack = self.target_pack.clone();
        candidate.opportunity.revision = next_revision(candidate.opportunity.revision)?;
        candidate.opportunity.metadata = map_metadata(
            &candidate.opportunity.metadata,
            &self.maps.fields,
            "Opportunity",
        )?;

        let mut next_requirement_revisions = BTreeMap::new();
        for requirement in &mut candidate.requirements {
            requirement.pack = self.target_pack.clone();
            requirement.category =
                mapped_item(&self.maps.requirement_categories, &requirement.category);
            requirement.revision = next_revision(requirement.revision)?;
            next_requirement_revisions.insert(requirement.id.clone(), requirement.revision);
        }

        if let Some(plan) = candidate.plan.as_mut() {
            plan.pack = self.target_pack.clone();
            plan.revision = next_revision(plan.revision)?;
            for deliverable in &mut plan.deliverables {
                deliverable.kind = map_deliverable_kind(
                    &deliverable.kind,
                    &self.target_pack,
                    &self.maps.deliverables,
                )?;
            }
            if self.impact.plan_invalidated {
                plan.state = PlanStateV3::Stale;
            } else {
                for reference in &mut plan.requirement_inputs {
                    reference.revision = *next_requirement_revisions
                        .get(&reference.id)
                        .ok_or_else(|| {
                            StoreError::ApplicationModelIntegrity(
                                "Pack migration lost a Plan Requirement reference".to_owned(),
                            )
                        })?;
                }
                for blocker in &mut plan.blockers {
                    if let Some(reference) = blocker.requirement.as_mut() {
                        reference.revision = *next_requirement_revisions
                            .get(&reference.id)
                            .ok_or_else(|| {
                                StoreError::ApplicationModelIntegrity(
                                    "Pack migration lost a blocker Requirement reference"
                                        .to_owned(),
                                )
                            })?;
                    }
                }
            }
        }

        let stale_ids = self
            .impact
            .stale_deliverable_ids
            .iter()
            .collect::<BTreeSet<_>>();
        let next_plan_revision = candidate.plan.as_ref().map(|plan| plan.revision);
        for deliverable in &mut candidate.deliverables {
            let was_stale = deliverable.state == DeliverableStateV3::Stale;
            deliverable.pack = self.target_pack.clone();
            deliverable.kind = map_deliverable_kind(
                &deliverable.kind,
                &self.target_pack,
                &self.maps.deliverables,
            )?;
            deliverable.revision = next_revision(deliverable.revision)?;
            if stale_ids.contains(&deliverable.id) {
                deliverable.state = DeliverableStateV3::Stale;
            } else if !was_stale && let Some(revision) = next_plan_revision {
                deliverable.plan.revision = revision;
            }
        }
        Ok(candidate)
    }
}

#[derive(Debug, Clone, Default)]
struct MigrationMaps {
    stages: BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    requirement_categories: BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    evidence_categories: BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    deliverables: BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    resources: BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    validators: BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    fields: BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
}

fn build_migration_plan(
    current: &StoredApplicationModelV3,
    source: &VerifiedWorkflowPackBundle,
    target: &VerifiedWorkflowPackBundle,
    projection_paths: Vec<SafeRelativePath>,
) -> Result<MigrationPlan, StoreError> {
    let source_pack = binding(source);
    let target_pack = binding(target);
    if source_pack != current.snapshot.pack {
        return Err(StoreError::ApplicationModelConflict(
            "verified source Pack does not match the current Application binding".to_owned(),
        ));
    }
    if target_pack.id != source_pack.id {
        return Err(StoreError::ApplicationModelConflict(
            "Pack migration cannot change the Pack ID; use a separately reviewed import boundary"
                .to_owned(),
        ));
    }
    let source_version = Version::parse(source_pack.version.as_str())
        .map_err(|error| StoreError::Invariant(error.to_string()))?;
    let target_version = Version::parse(target_pack.version.as_str())
        .map_err(|error| StoreError::Invariant(error.to_string()))?;
    if target_version <= source_version || target_pack.content_digest == source_pack.content_digest
    {
        return Err(StoreError::ApplicationModelConflict(
            "target Pack must have a greater version and a different verified content digest"
                .to_owned(),
        ));
    }
    let declared = target
        .manifest()
        .migrations
        .iter()
        .find(|migration| migration.from_version == source_pack.version)
        .ok_or_else(|| {
            StoreError::ApplicationModelConflict(format!(
                "target Pack declares no migration from version {}",
                source_pack.version
            ))
        })?;
    let maps = migration_maps(&declared.mappings)?;
    validate_mapping_catalogs(source.manifest(), target.manifest(), &maps)?;
    validate_snapshot_against_target(&current.snapshot, target.manifest(), &maps)?;

    let workflow_outputs = changed_workflow_outputs(source.manifest(), target.manifest(), &maps)?;
    let requirement_changes = changed_categories(
        &source.manifest().requirements.categories,
        &target.manifest().requirements.categories,
        &maps.requirement_categories,
        &maps.fields,
    );
    let evidence_changed = !changed_categories(
        &source.manifest().evidence.categories,
        &target.manifest().evidence.categories,
        &maps.evidence_categories,
        &maps.fields,
    )
    .is_empty();
    let catalog_plan_changed = deliverable_plan_contract_changed(
        &current.snapshot,
        source.manifest(),
        target.manifest(),
        &maps,
    )?;
    let plan_consumes_changed_requirement = current.snapshot.plan.as_ref().is_some_and(|plan| {
        plan.requirement_inputs.iter().any(|reference| {
            current
                .snapshot
                .requirements
                .iter()
                .find(|requirement| requirement.id == reference.id)
                .is_some_and(|requirement| requirement_changes.contains(&requirement.category))
        })
    });
    let plan_invalidated = current.snapshot.plan.is_some()
        && (plan_consumes_changed_requirement
            || catalog_plan_changed
            || workflow_outputs.contains(&WorkflowPackStageOutput::Requirements)
            || workflow_outputs.contains(&WorkflowPackStageOutput::Plan));
    let global_deliverable_invalidated =
        workflow_outputs.iter().any(|output| {
            matches!(
                output,
                WorkflowPackStageOutput::Deliverables
                    | WorkflowPackStageOutput::Review
                    | WorkflowPackStageOutput::Package
                    | WorkflowPackStageOutput::Render
            )
        }) || readiness_contract_changed(source.manifest(), target.manifest(), &maps)?;
    let changed_deliverable_kinds = changed_deliverable_output_kinds(source, target, &maps)?;
    let mut stale_deliverable_ids = current
        .snapshot
        .deliverables
        .iter()
        .filter(|deliverable| {
            !matches!(
                deliverable.state,
                DeliverableStateV3::Planned | DeliverableStateV3::Stale
            ) && (plan_invalidated
                || global_deliverable_invalidated
                || changed_deliverable_kinds.contains(deliverable.kind.local_id_str())
                || (evidence_changed && !deliverable.evidence_inputs.is_empty()))
        })
        .map(|deliverable| deliverable.id.clone())
        .collect::<Vec<_>>();
    stale_deliverable_ids.sort_unstable();
    let stale_plan_ids = current
        .snapshot
        .plan
        .as_ref()
        .filter(|plan| plan_invalidated && plan.state != PlanStateV3::Stale)
        .map(|plan| vec![plan.id.clone()])
        .unwrap_or_default();
    let mut projection_paths = projection_paths;
    projection_paths.sort_unstable();
    let impact = ApplicationPackMigrationImpactV3 {
        plan_invalidated,
        stale_plan_ids,
        stale_deliverable_ids,
        rebound_requirement_count: u64::try_from(current.snapshot.requirements.len())
            .map_err(|_| StoreError::Invariant("Requirement count exceeds u64".to_owned()))?,
        rebound_deliverable_count: u64::try_from(current.snapshot.deliverables.len())
            .map_err(|_| StoreError::Invariant("Deliverable count exceeds u64".to_owned()))?,
        superseded_projection_paths: projection_paths,
    };
    let mut plan = MigrationPlan {
        application_id: current.snapshot.application.id.clone(),
        from_application_revision: current.snapshot.application.revision,
        to_application_revision: next_revision(current.snapshot.application.revision)?,
        source_snapshot_sha256: current.snapshot_sha256.clone(),
        source_pack,
        target_pack,
        source_manifest_sha256: source.snapshot().manifest_sha256().clone(),
        target_manifest_sha256: target.snapshot().manifest_sha256().clone(),
        impact,
        maps,
        preview_sha256: Sha256Digest::try_new("0".repeat(64))?,
    };
    plan.preview_sha256 = digest_plan(&plan)?;
    Ok(plan)
}

fn binding(bundle: &VerifiedWorkflowPackBundle) -> ApplicationPackBindingV3 {
    ApplicationPackBindingV3 {
        id: bundle.manifest().id.clone(),
        version: bundle.manifest().version.clone(),
        content_digest: bundle.manifest().content_digest.clone(),
    }
}

fn migration_maps(mappings: &[WorkflowPackIdMapping]) -> Result<MigrationMaps, StoreError> {
    let mut result = MigrationMaps::default();
    for mapping in mappings {
        let map = match mapping.kind {
            WorkflowPackMigrationKind::Stage => &mut result.stages,
            WorkflowPackMigrationKind::RequirementCategory => &mut result.requirement_categories,
            WorkflowPackMigrationKind::EvidenceCategory => &mut result.evidence_categories,
            WorkflowPackMigrationKind::Deliverable => &mut result.deliverables,
            WorkflowPackMigrationKind::Resource => &mut result.resources,
            WorkflowPackMigrationKind::Validator => &mut result.validators,
            WorkflowPackMigrationKind::Field => &mut result.fields,
        };
        if map
            .insert(mapping.from.clone(), mapping.to.clone())
            .is_some()
        {
            return Err(StoreError::ApplicationModelConflict(
                "Pack migration repeats a source ID mapping".to_owned(),
            ));
        }
    }
    Ok(result)
}

fn validate_mapping_catalogs(
    source: &WorkflowPackManifest,
    target: &WorkflowPackManifest,
    maps: &MigrationMaps,
) -> Result<(), StoreError> {
    validate_map(
        &maps.stages,
        source.workflow.stages.iter().map(|value| &value.id),
        target.workflow.stages.iter().map(|value| &value.id),
        "Stage",
    )?;
    validate_map(
        &maps.requirement_categories,
        source.requirements.categories.iter().map(|value| &value.id),
        target.requirements.categories.iter().map(|value| &value.id),
        "Requirement category",
    )?;
    validate_map(
        &maps.evidence_categories,
        source.evidence.categories.iter().map(|value| &value.id),
        target.evidence.categories.iter().map(|value| &value.id),
        "Evidence category",
    )?;
    validate_map(
        &maps.deliverables,
        source.deliverables.kinds.iter().map(|value| &value.id),
        target.deliverables.kinds.iter().map(|value| &value.id),
        "Deliverable",
    )?;
    validate_map(
        &maps.resources,
        source.resources.iter().map(|value| &value.id),
        target.resources.iter().map(|value| &value.id),
        "resource",
    )?;
    validate_map(
        &maps.validators,
        source.validation.definitions.iter().map(|value| &value.id),
        target.validation.definitions.iter().map(|value| &value.id),
        "validator",
    )?;
    let source_fields = all_field_ids(source);
    let target_fields = all_field_ids(target);
    validate_map(
        &maps.fields,
        source_fields.iter(),
        target_fields.iter(),
        "field",
    )
}

fn validate_map<'a>(
    mappings: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    source: impl IntoIterator<Item = &'a WorkflowPackItemId>,
    target: impl IntoIterator<Item = &'a WorkflowPackItemId>,
    kind: &str,
) -> Result<(), StoreError> {
    let source = source.into_iter().cloned().collect::<BTreeSet<_>>();
    let target = target.into_iter().cloned().collect::<BTreeSet<_>>();
    for (from, to) in mappings {
        if !source.contains(from) || !target.contains(to) {
            return Err(StoreError::ApplicationModelConflict(format!(
                "Pack migration {kind} mapping {from} -> {to} does not bind declared source and target IDs"
            )));
        }
    }
    let mut resolved_targets = BTreeSet::new();
    for from in source {
        let to = mappings.get(&from).cloned().unwrap_or(from);
        if target.contains(&to) && !resolved_targets.insert(to.clone()) {
            return Err(StoreError::ApplicationModelConflict(format!(
                "Pack migration maps multiple {kind} IDs to the same target ID {to}"
            )));
        }
    }
    Ok(())
}

fn all_field_ids(manifest: &WorkflowPackManifest) -> BTreeSet<WorkflowPackItemId> {
    manifest
        .application
        .opportunity_fields
        .iter()
        .chain(&manifest.application.application_fields)
        .chain(
            manifest
                .requirements
                .categories
                .iter()
                .flat_map(|category| &category.fields),
        )
        .chain(
            manifest
                .evidence
                .categories
                .iter()
                .flat_map(|category| &category.fields),
        )
        .map(|field| field.id.clone())
        .collect()
}

fn validate_snapshot_against_target(
    snapshot: &ApplicationModelSnapshotV3,
    target: &WorkflowPackManifest,
    maps: &MigrationMaps,
) -> Result<(), StoreError> {
    validate_metadata_target(
        &snapshot.opportunity.metadata,
        &target.application.opportunity_fields,
        &maps.fields,
        "Opportunity",
    )?;
    validate_metadata_target(
        &snapshot.application.metadata,
        &target.application.application_fields,
        &maps.fields,
        "Application",
    )?;
    let categories = target
        .requirements
        .categories
        .iter()
        .map(|category| &category.id)
        .collect::<BTreeSet<_>>();
    for requirement in &snapshot.requirements {
        let mapped = mapped_item(&maps.requirement_categories, &requirement.category);
        if !categories.contains(&mapped) {
            return Err(StoreError::ApplicationModelConflict(format!(
                "Requirement {} category {} has no target Pack definition or mapping",
                requirement.id, requirement.category
            )));
        }
    }
    let target_kinds = target
        .deliverables
        .kinds
        .iter()
        .map(|kind| &kind.id)
        .collect::<BTreeSet<_>>();
    let kinds = snapshot
        .plan
        .iter()
        .flat_map(|plan| plan.deliverables.iter().map(|value| &value.kind))
        .chain(snapshot.deliverables.iter().map(|value| &value.kind));
    for kind in kinds {
        let local = WorkflowPackItemId::try_new(kind.local_id_str())
            .map_err(|error| StoreError::ApplicationModelIntegrity(error.to_string()))?;
        let mapped = mapped_item(&maps.deliverables, &local);
        if !target_kinds.contains(&mapped) {
            return Err(StoreError::ApplicationModelConflict(format!(
                "Deliverable kind {kind} has no target Pack definition or mapping"
            )));
        }
    }
    Ok(())
}

fn validate_metadata_target(
    metadata: &BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    target_fields: &[WorkflowPackFieldDefinition],
    mappings: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    entity: &str,
) -> Result<(), StoreError> {
    let mapped = map_metadata(metadata, mappings, entity)?;
    let fields = target_fields
        .iter()
        .map(|field| (&field.id, field))
        .collect::<BTreeMap<_, _>>();
    for (id, value) in &mapped {
        let field = fields.get(id).ok_or_else(|| {
            StoreError::ApplicationModelConflict(format!(
                "{entity} field {id} has no target Pack definition or mapping"
            ))
        })?;
        if !field_value_compatible(value, field) {
            return Err(StoreError::ApplicationModelConflict(format!(
                "{entity} field {id} is incompatible with its target Pack definition"
            )));
        }
    }
    for field in target_fields {
        if field.required && !mapped.contains_key(&field.id) {
            return Err(StoreError::ApplicationModelConflict(format!(
                "target Pack requires missing {entity} field {}",
                field.id
            )));
        }
    }
    Ok(())
}

fn field_value_compatible(
    value: &ApplicationFieldValueV3,
    field: &WorkflowPackFieldDefinition,
) -> bool {
    match (value, field.field_type) {
        (ApplicationFieldValueV3::ShortText(_), WorkflowPackFieldType::ShortText)
        | (ApplicationFieldValueV3::LongText(_), WorkflowPackFieldType::LongText)
        | (ApplicationFieldValueV3::Integer(_), WorkflowPackFieldType::Integer)
        | (ApplicationFieldValueV3::Boolean(_), WorkflowPackFieldType::Boolean)
        | (ApplicationFieldValueV3::Date(_), WorkflowPackFieldType::Date)
        | (ApplicationFieldValueV3::Url(_), WorkflowPackFieldType::Url)
        | (ApplicationFieldValueV3::StringList(_), WorkflowPackFieldType::StringList) => true,
        (ApplicationFieldValueV3::Choice(selected), WorkflowPackFieldType::Choice) => {
            field.options.iter().any(|option| option.id == *selected)
        }
        _ => false,
    }
}

fn map_metadata(
    metadata: &BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    mappings: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    entity: &str,
) -> Result<BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>, StoreError> {
    let mut mapped = BTreeMap::new();
    for (id, value) in metadata {
        let target = mapped_item(mappings, id);
        if mapped.insert(target.clone(), value.clone()).is_some() {
            return Err(StoreError::ApplicationModelConflict(format!(
                "Pack migration maps multiple {entity} fields to {target}"
            )));
        }
    }
    Ok(mapped)
}

fn mapped_item(
    mappings: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    value: &WorkflowPackItemId,
) -> WorkflowPackItemId {
    mappings
        .get(value)
        .cloned()
        .unwrap_or_else(|| value.clone())
}

fn map_deliverable_kind(
    kind: &DeliverableKindId,
    target: &ApplicationPackBindingV3,
    mappings: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
) -> Result<DeliverableKindId, StoreError> {
    let local = WorkflowPackItemId::try_new(kind.local_id_str())
        .map_err(|error| StoreError::ApplicationModelIntegrity(error.to_string()))?;
    Ok(DeliverableKindId::from_parts(
        &target.id,
        &mapped_item(mappings, &local),
    ))
}

fn changed_categories(
    source: &[canisend_contracts::WorkflowPackCategoryDefinition],
    target: &[canisend_contracts::WorkflowPackCategoryDefinition],
    category_maps: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
    field_maps: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
) -> BTreeSet<WorkflowPackItemId> {
    let target = target
        .iter()
        .map(|category| (&category.id, category))
        .collect::<BTreeMap<_, _>>();
    let mut changed = BTreeSet::new();
    for category in source {
        let target_id = mapped_item(category_maps, &category.id);
        let differs = target.get(&target_id).is_none_or(|target| {
            category_signature(category, &target_id, field_maps)
                != category_signature(target, &target.id, &BTreeMap::new())
        });
        if differs {
            changed.insert(category.id.clone());
        }
    }
    changed
}

fn category_signature(
    category: &canisend_contracts::WorkflowPackCategoryDefinition,
    id: &WorkflowPackItemId,
    field_maps: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
) -> Value {
    let mut value = serde_json::to_value(category).expect("typed Pack category serializes");
    strip_labels(&mut value);
    value["id"] = Value::String(id.to_string());
    if let Some(fields) = value.get_mut("fields").and_then(Value::as_array_mut) {
        for field in fields {
            if let Some(source_id) = field.get("id").and_then(Value::as_str)
                && let Ok(source_id) = WorkflowPackItemId::try_new(source_id)
            {
                field["id"] = Value::String(mapped_item(field_maps, &source_id).to_string());
            }
        }
    }
    sort_json(value)
}

fn deliverable_plan_contract_changed(
    snapshot: &ApplicationModelSnapshotV3,
    source: &WorkflowPackManifest,
    target: &WorkflowPackManifest,
    maps: &MigrationMaps,
) -> Result<bool, StoreError> {
    let source_kinds = source
        .deliverables
        .kinds
        .iter()
        .map(|kind| (&kind.id, kind))
        .collect::<BTreeMap<_, _>>();
    let target_kinds = target
        .deliverables
        .kinds
        .iter()
        .map(|kind| (&kind.id, kind))
        .collect::<BTreeMap<_, _>>();
    let source_required = source
        .deliverables
        .kinds
        .iter()
        .filter(|kind| kind.minimum > 0)
        .map(|kind| mapped_item(&maps.deliverables, &kind.id))
        .collect::<BTreeSet<_>>();
    let target_required = target
        .deliverables
        .kinds
        .iter()
        .filter(|kind| kind.minimum > 0)
        .map(|kind| kind.id.clone())
        .collect::<BTreeSet<_>>();
    if source_required != target_required {
        return Ok(true);
    }
    let Some(plan) = &snapshot.plan else {
        return Ok(false);
    };
    for planned in &plan.deliverables {
        let local = WorkflowPackItemId::try_new(planned.kind.local_id_str())
            .map_err(|error| StoreError::ApplicationModelIntegrity(error.to_string()))?;
        let target_id = mapped_item(&maps.deliverables, &local);
        let source = source_kinds.get(&local).ok_or_else(|| {
            StoreError::ApplicationModelConflict(format!(
                "current Plan kind {} is absent from the verified source Pack",
                planned.kind
            ))
        })?;
        let target = target_kinds.get(&target_id).ok_or_else(|| {
            StoreError::ApplicationModelConflict(format!(
                "current Plan kind {} is absent from the target Pack",
                planned.kind
            ))
        })?;
        if source.minimum != target.minimum || source.maximum != target.maximum {
            return Ok(true);
        }
    }
    Ok(false)
}

fn changed_deliverable_output_kinds(
    source: &VerifiedWorkflowPackBundle,
    target: &VerifiedWorkflowPackBundle,
    maps: &MigrationMaps,
) -> Result<BTreeSet<String>, StoreError> {
    let target_kinds = target
        .manifest()
        .deliverables
        .kinds
        .iter()
        .map(|kind| (&kind.id, kind))
        .collect::<BTreeMap<_, _>>();
    let mut changed = BTreeSet::new();
    for source_kind in &source.manifest().deliverables.kinds {
        let target_id = mapped_item(&maps.deliverables, &source_kind.id);
        let differs = target_kinds.get(&target_id).is_none_or(|target_kind| {
            deliverable_output_signature(source, source_kind, &target_id, maps)
                != deliverable_output_signature(
                    target,
                    target_kind,
                    &target_kind.id,
                    &MigrationMaps::default(),
                )
        });
        if differs {
            changed.insert(source_kind.id.to_string());
        }
    }
    Ok(changed)
}

fn deliverable_output_signature(
    bundle: &VerifiedWorkflowPackBundle,
    kind: &canisend_contracts::WorkflowPackDeliverableDefinition,
    target_id: &WorkflowPackItemId,
    maps: &MigrationMaps,
) -> Value {
    let validators = kind
        .validators
        .iter()
        .map(|id| {
            let target = mapped_item(&maps.validators, id);
            let definition = bundle
                .manifest()
                .validation
                .definitions
                .iter()
                .find(|definition| definition.id == *id)
                .map(|definition| {
                    json!({
                        "id": target,
                        "capability": definition.capability,
                        "parameters": definition.parameters,
                    })
                });
            (target, definition)
        })
        .collect::<Vec<_>>();
    let template = kind.template.as_ref().map(|path| {
        let resource = bundle
            .manifest()
            .resources
            .iter()
            .find(|resource| resource.path == *path);
        json!({
            "path": path,
            "resource": resource.map(|resource| json!({
                "kind": resource.kind,
                "version": resource.version,
                "sha256": resource.sha256,
            })),
        })
    });
    sort_json(json!({
        "id": target_id,
        "template": template,
        "renderer": kind.renderer,
        "validators": validators,
    }))
}

fn readiness_contract_changed(
    source: &WorkflowPackManifest,
    target: &WorkflowPackManifest,
    maps: &MigrationMaps,
) -> Result<bool, StoreError> {
    let source_ids = source
        .validation
        .readiness
        .iter()
        .map(|id| mapped_item(&maps.validators, id))
        .collect::<BTreeSet<_>>();
    let target_ids = target
        .validation
        .readiness
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if source_ids != target_ids {
        return Ok(true);
    }
    for source_id in &source.validation.readiness {
        let target_id = mapped_item(&maps.validators, source_id);
        let source_definition = source
            .validation
            .definitions
            .iter()
            .find(|definition| definition.id == *source_id);
        let target_definition = target
            .validation
            .definitions
            .iter()
            .find(|definition| definition.id == target_id);
        let source_signature = source_definition.map(|definition| {
            sort_json(json!({
                "id": target_id,
                "capability": definition.capability,
                "parameters": definition.parameters,
            }))
        });
        let target_signature = target_definition.map(|definition| {
            sort_json(json!({
                "id": definition.id,
                "capability": definition.capability,
                "parameters": definition.parameters,
            }))
        });
        if source_signature != target_signature {
            return Ok(true);
        }
    }
    Ok(false)
}

fn changed_workflow_outputs(
    source: &WorkflowPackManifest,
    target: &WorkflowPackManifest,
    maps: &MigrationMaps,
) -> Result<Vec<WorkflowPackStageOutput>, StoreError> {
    let source_stages = normalized_stages(source, &maps.stages);
    let target_stages = normalized_stages(target, &BTreeMap::new());
    let ids = source_stages
        .keys()
        .chain(target_stages.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let changed = ids
        .into_iter()
        .filter(|id| source_stages.get(id) != target_stages.get(id))
        .collect::<BTreeSet<_>>();
    if changed.is_empty() {
        return Ok(Vec::new());
    }
    let mut affected = changed.clone();
    collect_descendants(&source_stages, &mut affected, &changed);
    collect_descendants(&target_stages, &mut affected, &changed);
    let mut outputs = Vec::new();
    for output in affected.iter().filter_map(|id| {
        target_stages
            .get(id)
            .or_else(|| source_stages.get(id))
            .map(|stage| stage.output)
    }) {
        if !outputs.contains(&output) {
            outputs.push(output);
        }
    }
    Ok(outputs)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedStage {
    dependencies: BTreeSet<WorkflowPackItemId>,
    output: WorkflowPackStageOutput,
    execution_modes: BTreeSet<String>,
}

fn normalized_stages(
    manifest: &WorkflowPackManifest,
    mappings: &BTreeMap<WorkflowPackItemId, WorkflowPackItemId>,
) -> BTreeMap<WorkflowPackItemId, NormalizedStage> {
    manifest
        .workflow
        .stages
        .iter()
        .map(|stage| {
            (
                mapped_item(mappings, &stage.id),
                NormalizedStage {
                    dependencies: stage
                        .depends_on
                        .iter()
                        .map(|dependency| mapped_item(mappings, dependency))
                        .collect(),
                    output: stage.output,
                    execution_modes: stage
                        .execution_modes
                        .iter()
                        .map(|mode| {
                            serde_json::to_value(mode)
                                .expect("ExecutionMode serializes")
                                .as_str()
                                .expect("ExecutionMode is a string")
                                .to_owned()
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

fn collect_descendants(
    stages: &BTreeMap<WorkflowPackItemId, NormalizedStage>,
    affected: &mut BTreeSet<WorkflowPackItemId>,
    roots: &BTreeSet<WorkflowPackItemId>,
) {
    let mut queue = roots.iter().cloned().collect::<VecDeque<_>>();
    while let Some(root) = queue.pop_front() {
        for (id, stage) in stages {
            if stage.dependencies.contains(&root) && affected.insert(id.clone()) {
                queue.push_back(id.clone());
            }
        }
    }
}

fn strip_labels(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.remove("labels");
            for child in object.values_mut() {
                strip_labels(child);
            }
        }
        Value::Array(values) => {
            for child in values {
                strip_labels(child);
            }
        }
        _ => {}
    }
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
        value => value,
    }
}

fn digest_plan(plan: &MigrationPlan) -> Result<Sha256Digest, StoreError> {
    let value = json!({
        "format": APPLICATION_PACK_MIGRATION_FORMAT_V3,
        "application_id": plan.application_id,
        "from_application_revision": plan.from_application_revision,
        "to_application_revision": plan.to_application_revision,
        "source_snapshot_sha256": plan.source_snapshot_sha256,
        "source_pack": plan.source_pack,
        "target_pack": plan.target_pack,
        "source_manifest_sha256": plan.source_manifest_sha256,
        "target_manifest_sha256": plan.target_manifest_sha256,
        "impact": plan.impact,
    });
    let bytes = serde_json::to_vec(&sort_json(value))?;
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes))).map_err(StoreError::from)
}

fn load_current_projection_paths(
    connection: &Connection,
    application_id: &ApplicationId,
    application_revision: Revision,
) -> Result<Vec<SafeRelativePath>, StoreError> {
    let storage = ApplicationStorage::detect(connection)?;
    let mut statement = connection.prepare(&format!(
        "SELECT relative_path FROM {}
         WHERE application_id = ?1 AND application_revision = ?2 ORDER BY relative_path",
        storage.projections()
    ))?;
    statement
        .query_map(
            params![application_id.as_str(), to_i64(application_revision.get())?],
            |row| row.get::<_, String>(0),
        )?
        .map(|row| SafeRelativePath::try_new(row?).map_err(StoreError::from))
        .collect()
}

fn validate_pack_migration_transition(
    current: &ApplicationModelSnapshotV3,
    candidate: &ApplicationModelSnapshotV3,
) -> Result<(), StoreError> {
    if candidate.application.id != current.application.id
        || candidate.opportunity.id != current.opportunity.id
        || candidate.application.opportunity_id != current.application.opportunity_id
        || candidate.application.created_at != current.application.created_at
        || candidate.opportunity.created_at != current.opportunity.created_at
        || candidate.pack.id != current.pack.id
        || candidate.pack == current.pack
    {
        return Err(StoreError::ApplicationModelConflict(
            "Pack migration changed an immutable identity or did not change the Pack binding"
                .to_owned(),
        ));
    }
    if candidate.application.revision != next_revision(current.application.revision)?
        || candidate.opportunity.revision != next_revision(current.opportunity.revision)?
    {
        return Err(StoreError::ApplicationModelConflict(
            "Pack migration must advance Application and Opportunity exactly once".to_owned(),
        ));
    }
    validate_rebound_revisions(
        current
            .requirements
            .iter()
            .map(|value| (&value.id, value.revision)),
        candidate
            .requirements
            .iter()
            .map(|value| (&value.id, value.revision)),
        "Requirement",
    )?;
    validate_rebound_revisions(
        current
            .deliverables
            .iter()
            .map(|value| (&value.id, value.revision)),
        candidate
            .deliverables
            .iter()
            .map(|value| (&value.id, value.revision)),
        "Deliverable",
    )?;
    match (&current.plan, &candidate.plan) {
        (Some(current), Some(candidate))
            if candidate.id == current.id
                && candidate.revision == next_revision(current.revision)? => {}
        (None, None) => {}
        _ => {
            return Err(StoreError::ApplicationModelConflict(
                "Pack migration must preserve the Plan identity and advance it exactly once"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_rebound_revisions<'a, T: Ord + std::fmt::Display + 'a>(
    current: impl IntoIterator<Item = (&'a T, Revision)>,
    candidate: impl IntoIterator<Item = (&'a T, Revision)>,
    kind: &str,
) -> Result<(), StoreError> {
    let current = current.into_iter().collect::<BTreeMap<_, _>>();
    let candidate = candidate.into_iter().collect::<BTreeMap<_, _>>();
    if current.len() != candidate.len() || current.keys().ne(candidate.keys()) {
        return Err(StoreError::ApplicationModelConflict(format!(
            "Pack migration cannot add or remove {kind} identities"
        )));
    }
    for (id, revision) in current {
        if candidate.get(id).copied() != Some(next_revision(revision)?) {
            return Err(StoreError::ApplicationModelConflict(format!(
                "Pack migration must advance {kind} {id} exactly once"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ApplicationLifecycleV3, ApplicationModelFormatV3, ApplicationRecordV3,
        ContentRevisionReferenceV3, ContentSpanV3, DeliverableRecordV3, EntityId,
        EntityRevisionReferenceV3, ExecutionMode, OpportunityId, OpportunityRecordV3, PlanId,
        PlanRecordV3, PlanRevisionReferenceV3, PlannedDeliverableDispositionV3,
        PlannedDeliverableV3, RequirementConfirmationV3, RequirementId, RequirementPriorityV3,
        RequirementRecordV3, RequirementRevisionReferenceV3, SemanticVersion, UtcTimestamp,
        WorkflowPackApplicationDefinition, WorkflowPackCapabilities,
        WorkflowPackCategoryDefinition, WorkflowPackCompatibility, WorkflowPackDeliverableCatalog,
        WorkflowPackDeliverableDefinition, WorkflowPackFieldDefinition, WorkflowPackFormat,
        WorkflowPackId, WorkflowPackLocaleId, WorkflowPackLocalizedText, WorkflowPackManifest,
        WorkflowPackMigration, WorkflowPackPublisher, WorkflowPackPublisherId,
        WorkflowPackResource, WorkflowPackResourceKind, WorkflowPackStageDefinition,
        WorkflowPackTaxonomy, WorkflowPackValidationPolicy, WorkflowPackValidatorDefinition,
        WorkflowPackVocabulary, WorkflowPackWorkflowDefinition,
    };
    use canisend_core::{
        WorkflowPackCapabilityRegistry, WorkflowPackOrigin, WorkflowPackRuntime,
        calculate_workflow_pack_content_digest,
    };
    use canisend_io::EmbeddedTypstCompiler;
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::{
        ApplicationModelRepository, ApplicationProjectionService, ProjectionService, Workspace,
        application_v3::activate_workspace_v3_authority,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "canisend-pack-migration-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entity(suffix: u16) -> EntityId {
        EntityId::try_new(format!("019f2f55-7c00-7000-8001-{suffix:012}")).expect("Entity ID")
    }

    fn application_id(suffix: u16) -> ApplicationId {
        ApplicationId::try_new(entity(suffix).to_string()).expect("Application ID")
    }

    fn opportunity_id(suffix: u16) -> OpportunityId {
        OpportunityId::try_new(entity(suffix).to_string()).expect("Opportunity ID")
    }

    fn requirement_id(suffix: u16) -> RequirementId {
        RequirementId::try_new(entity(suffix).to_string()).expect("Requirement ID")
    }

    fn plan_id(suffix: u16) -> PlanId {
        PlanId::try_new(entity(suffix).to_string()).expect("Plan ID")
    }

    fn deliverable_id(suffix: u16) -> DeliverableId {
        DeliverableId::try_new(entity(suffix).to_string()).expect("Deliverable ID")
    }

    fn revision(value: u64) -> Revision {
        Revision::try_new(value).expect("revision")
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn locale(value: &str) -> WorkflowPackLocaleId {
        WorkflowPackLocaleId::try_new(value).expect("locale")
    }

    fn labels(value: &str) -> WorkflowPackLocalizedText {
        WorkflowPackLocalizedText(BTreeMap::from([(locale("en"), value.to_owned())]))
    }

    fn timestamp() -> UtcTimestamp {
        UtcTimestamp::try_new("2026-08-02T17:00:00Z").expect("timestamp")
    }

    fn digest(bytes: &[u8]) -> Sha256Digest {
        Sha256Digest::try_new(hex::encode(Sha256::digest(bytes))).expect("digest")
    }

    fn safe_path(value: &str) -> SafeRelativePath {
        SafeRelativePath::try_new(value).expect("safe path")
    }

    fn runtime() -> WorkflowPackRuntime {
        WorkflowPackRuntime::parse("1.0.0-alpha.5", "3.0.0-alpha.1", "3.0.0-alpha.1")
            .expect("runtime")
    }

    fn bundle_fixture(
        version: &str,
    ) -> (WorkflowPackManifest, BTreeMap<SafeRelativePath, Vec<u8>>) {
        let statement_path = safe_path("templates/statement.typ");
        let appendix_path = safe_path("templates/appendix.typ");
        let statement = b"statement-v1".to_vec();
        let appendix = b"appendix-v1".to_vec();
        let renderer =
            canisend_contracts::WorkflowPackCapabilityId::try_new("canisend.renderer.typst")
                .expect("renderer");
        let validator_capability = canisend_contracts::WorkflowPackCapabilityId::try_new(
            "canisend.validator.evidence-traceability",
        )
        .expect("validator");
        let mut manifest = WorkflowPackManifest {
            format: WorkflowPackFormat::V1,
            id: WorkflowPackId::try_new("org.canisend.migration-test").expect("Pack ID"),
            version: SemanticVersion::try_new(version).expect("version"),
            schema_version: SemanticVersion::try_new("1.0.0").expect("schema version"),
            publisher: WorkflowPackPublisher {
                id: WorkflowPackPublisherId::try_new("org.canisend").expect("publisher"),
                name: "CanISend".to_owned(),
                homepage: None,
            },
            compatibility: WorkflowPackCompatibility {
                kernel: ">=1.0.0-alpha.5, <2.0.0".to_owned(),
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
                opportunity_fields: vec![WorkflowPackFieldDefinition {
                    id: item("title"),
                    labels: labels("Title"),
                    field_type: WorkflowPackFieldType::ShortText,
                    required: true,
                    options: Vec::new(),
                }],
                application_fields: Vec::new(),
            },
            workflow: WorkflowPackWorkflowDefinition {
                stages: vec![
                    stage("intake", &[], WorkflowPackStageOutput::Sources),
                    stage(
                        "requirements",
                        &["intake"],
                        WorkflowPackStageOutput::Requirements,
                    ),
                    stage("plan", &["requirements"], WorkflowPackStageOutput::Plan),
                    stage("draft", &["plan"], WorkflowPackStageOutput::Deliverables),
                    stage("render", &["draft"], WorkflowPackStageOutput::Render),
                ],
                terminal_stage: item("render"),
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
                kinds: vec![
                    deliverable_kind("statement", statement_path.clone(), &renderer),
                    deliverable_kind("appendix", appendix_path.clone(), &renderer),
                ],
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
            resources: vec![
                resource("statement-template", statement_path.clone(), &statement),
                resource("appendix-template", appendix_path.clone(), &appendix),
            ],
            migrations: Vec::new(),
            content_digest: Sha256Digest::try_new("0".repeat(64)).expect("placeholder"),
        };
        let resources = BTreeMap::from([(statement_path, statement), (appendix_path, appendix)]);
        manifest.content_digest =
            calculate_workflow_pack_content_digest(&manifest, &resources).expect("Pack digest");
        (manifest, resources)
    }

    fn stage(
        id: &str,
        dependencies: &[&str],
        output: WorkflowPackStageOutput,
    ) -> WorkflowPackStageDefinition {
        WorkflowPackStageDefinition {
            id: item(id),
            labels: labels(id),
            depends_on: dependencies.iter().map(|value| item(value)).collect(),
            output,
            execution_modes: vec![ExecutionMode::Deterministic],
        }
    }

    fn deliverable_kind(
        id: &str,
        template: SafeRelativePath,
        renderer: &canisend_contracts::WorkflowPackCapabilityId,
    ) -> WorkflowPackDeliverableDefinition {
        WorkflowPackDeliverableDefinition {
            id: item(id),
            labels: labels(id),
            minimum: 1,
            maximum: 1,
            template: Some(template),
            renderer: Some(renderer.clone()),
            validators: vec![item("traceability")],
        }
    }

    fn resource(id: &str, path: SafeRelativePath, bytes: &[u8]) -> WorkflowPackResource {
        WorkflowPackResource {
            id: item(id),
            kind: WorkflowPackResourceKind::Template,
            path,
            version: SemanticVersion::try_new("1.0.0").expect("resource version"),
            size_bytes: bytes.len() as u64,
            sha256: digest(bytes),
        }
    }

    fn verified(
        manifest: &WorkflowPackManifest,
        resources: BTreeMap<SafeRelativePath, Vec<u8>>,
    ) -> VerifiedWorkflowPackBundle {
        VerifiedWorkflowPackBundle::verify(
            &serde_json::to_value(manifest).expect("manifest value"),
            resources,
            WorkflowPackOrigin::External,
            &runtime(),
            &WorkflowPackCapabilityRegistry::built_in(),
        )
        .expect("verified Pack")
    }

    fn target_bundle(
        mut manifest: WorkflowPackManifest,
        resources: BTreeMap<SafeRelativePath, Vec<u8>>,
        mutate: impl FnOnce(&mut WorkflowPackManifest, &mut BTreeMap<SafeRelativePath, Vec<u8>>),
    ) -> VerifiedWorkflowPackBundle {
        manifest.version = SemanticVersion::try_new("1.1.0").expect("target version");
        manifest.migrations = vec![WorkflowPackMigration {
            from_version: SemanticVersion::try_new("1.0.0").expect("source version"),
            mappings: Vec::new(),
        }];
        let mut resources = resources;
        mutate(&mut manifest, &mut resources);
        for resource in &mut manifest.resources {
            let bytes = resources.get(&resource.path).expect("resource bytes");
            resource.size_bytes = bytes.len() as u64;
            resource.sha256 = digest(bytes);
        }
        manifest.content_digest =
            calculate_workflow_pack_content_digest(&manifest, &resources).expect("target digest");
        verified(&manifest, resources)
    }

    fn recalculated_bundle(
        mut manifest: WorkflowPackManifest,
        resources: BTreeMap<SafeRelativePath, Vec<u8>>,
    ) -> VerifiedWorkflowPackBundle {
        manifest.content_digest =
            calculate_workflow_pack_content_digest(&manifest, &resources).expect("Pack digest");
        verified(&manifest, resources)
    }

    fn snapshot(
        pack: ApplicationPackBindingV3,
        content_sha256: Sha256Digest,
    ) -> ApplicationModelSnapshotV3 {
        let application_id = application_id(101);
        let opportunity_id = opportunity_id(102);
        let requirement_id = requirement_id(103);
        let plan_id = plan_id(104);
        let statement_kind = DeliverableKindId::from_parts(&pack.id, &item("statement"));
        let appendix_kind = DeliverableKindId::from_parts(&pack.id, &item("appendix"));
        ApplicationModelSnapshotV3 {
            format: ApplicationModelFormatV3::V3,
            pack: pack.clone(),
            opportunity: OpportunityRecordV3 {
                id: opportunity_id.clone(),
                pack: pack.clone(),
                title: "Community grant".to_owned(),
                metadata: BTreeMap::from([(
                    item("title"),
                    ApplicationFieldValueV3::ShortText("Community grant".to_owned()),
                )]),
                source_ids: vec![entity(105)],
                created_at: timestamp(),
                revision: revision(1),
                archived: false,
            },
            application: ApplicationRecordV3 {
                id: application_id.clone(),
                opportunity_id,
                pack: pack.clone(),
                metadata: BTreeMap::new(),
                lifecycle: ApplicationLifecycleV3::Active,
                created_at: timestamp(),
                updated_at: timestamp(),
                revision: revision(1),
            },
            requirements: vec![RequirementRecordV3 {
                id: requirement_id.clone(),
                application_id: application_id.clone(),
                pack: pack.clone(),
                category: item("general"),
                statement: "Explain the public benefit.".to_owned(),
                priority: RequirementPriorityV3::Mandatory,
                source_span: ContentSpanV3 {
                    content: ContentRevisionReferenceV3 {
                        id: entity(106),
                        revision: revision(1),
                        sha256: content_sha256.clone(),
                    },
                    start_byte: 0,
                    end_byte: 8,
                },
                confirmation: RequirementConfirmationV3::Confirmed,
                confirmed_by: Some(ActorKind::User),
                confirmed_at: Some(timestamp()),
                revision: revision(1),
            }],
            plan: Some(PlanRecordV3 {
                id: plan_id.clone(),
                application_id: application_id.clone(),
                pack: pack.clone(),
                state: PlanStateV3::Confirmed,
                decision: Some(item("proceed")),
                requirement_inputs: vec![RequirementRevisionReferenceV3 {
                    id: requirement_id,
                    revision: revision(1),
                }],
                deliverables: vec![
                    planned(statement_kind.clone()),
                    planned(appendix_kind.clone()),
                ],
                blockers: Vec::new(),
                decided_by: Some(ActorKind::User),
                decided_at: Some(timestamp()),
                revision: revision(1),
            }),
            deliverables: vec![
                deliverable(
                    107,
                    &application_id,
                    &plan_id,
                    pack.clone(),
                    statement_kind,
                    content_sha256.clone(),
                    true,
                ),
                deliverable(
                    108,
                    &application_id,
                    &plan_id,
                    pack,
                    appendix_kind,
                    content_sha256,
                    false,
                ),
            ],
        }
    }

    fn planned(kind: DeliverableKindId) -> PlannedDeliverableV3 {
        PlannedDeliverableV3 {
            kind,
            disposition: PlannedDeliverableDispositionV3::Required,
            rationale: "Required by the Pack.".to_owned(),
            constraints: Vec::new(),
            execution_mode: Some(ExecutionMode::HostAgent),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn deliverable(
        suffix: u16,
        application_id: &ApplicationId,
        plan_id: &PlanId,
        pack: ApplicationPackBindingV3,
        kind: DeliverableKindId,
        content_sha256: Sha256Digest,
        with_evidence: bool,
    ) -> DeliverableRecordV3 {
        DeliverableRecordV3 {
            id: deliverable_id(suffix),
            application_id: application_id.clone(),
            pack,
            plan: PlanRevisionReferenceV3 {
                id: plan_id.clone(),
                revision: revision(1),
            },
            kind,
            title: format!("Deliverable {suffix}"),
            state: DeliverableStateV3::Draft,
            content: Some(ContentRevisionReferenceV3 {
                id: entity(suffix + 100),
                revision: revision(1),
                sha256: content_sha256,
            }),
            media_type: Some("text/markdown".to_owned()),
            evidence_inputs: with_evidence
                .then(|| EntityRevisionReferenceV3 {
                    id: entity(109),
                    revision: revision(1),
                })
                .into_iter()
                .collect(),
            revision: revision(1),
        }
    }

    fn fixture(
        label: &str,
    ) -> (
        TestDirectory,
        Workspace,
        VerifiedWorkflowPackBundle,
        WorkflowPackManifest,
        BTreeMap<SafeRelativePath, Vec<u8>>,
        ApplicationId,
    ) {
        let root = TestDirectory::new(label);
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        activate_workspace_v3_authority(
            &mut workspace.database,
            ActorKind::User,
            "activate-pack-migration-test",
        )
        .expect("activate v3");
        let (manifest, resources) = bundle_fixture("1.0.0");
        let source = verified(&manifest, resources.clone());
        let bytes = b"authoritative-content";
        let content_sha256 = workspace.blobs.put_bytes(bytes).expect("content Blob");
        let snapshot = snapshot(binding(&source), content_sha256);
        let application_id = snapshot.application.id.clone();
        ApplicationModelRepository::new(&mut workspace.database)
            .create(snapshot, ActorKind::User, "create-pack-migration-test")
            .expect("Application");
        (root, workspace, source, manifest, resources, application_id)
    }

    fn fixture_v4(
        label: &str,
    ) -> (
        TestDirectory,
        Workspace,
        VerifiedWorkflowPackBundle,
        WorkflowPackManifest,
        BTreeMap<SafeRelativePath, Vec<u8>>,
        ApplicationId,
    ) {
        let root = TestDirectory::new(label);
        let mut workspace = Workspace::init_v4(root.path()).expect("Workspace v4");
        let (manifest, resources) = bundle_fixture("1.0.0");
        let source = verified(&manifest, resources.clone());
        let content_sha256 = workspace
            .blobs
            .put_bytes(b"native-v4-authoritative-content")
            .expect("content Blob");
        let snapshot = snapshot(binding(&source), content_sha256);
        let application_id = snapshot.application.id.clone();
        ApplicationModelRepository::new(&mut workspace.database)
            .create(snapshot, ActorKind::User, "create-native-v4-pack-migration")
            .expect("native v4 Application");
        (root, workspace, source, manifest, resources, application_id)
    }

    #[test]
    fn native_v4_pack_migration_uses_only_native_heads_revisions_and_ledger() {
        let (_root, mut workspace, source, manifest, resources, application_id) =
            fixture_v4("native-v4-ledger");
        let target = target_bundle(manifest, resources, |manifest, _| {
            manifest.workflow.stages[0].labels = labels("Native v4 updated label");
        });
        let preview = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &target)
            .expect("native v4 preview");
        ApplicationPackMigrationService::new(&mut workspace.database)
            .migrate(
                &application_id,
                revision(1),
                &preview.preview_sha256,
                &source,
                &target,
                ActorKind::User,
                "upgrade-native-v4-pack",
            )
            .expect("native v4 Pack migration");

        let native_ledger: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_pack_v4_migrations",
                [],
                |row| row.get(0),
            )
            .expect("native v4 ledger count");
        let legacy_ledger: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_pack_v3_migrations",
                [],
                |row| row.get(0),
            )
            .expect("legacy v3 ledger count");
        let native_revisions: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_v4_revisions WHERE application_id = ?1",
                [application_id.as_str()],
                |row| row.get(0),
            )
            .expect("native v4 revision count");
        assert_eq!(native_ledger, 1);
        assert_eq!(legacy_ledger, 0);
        assert_eq!(native_revisions, 2);
    }

    #[test]
    fn label_only_upgrade_rebinds_without_invalidating_outputs_and_supersedes_old_projections() {
        let mut executor = EmbeddedTypstCompiler::new();
        let (_root, mut workspace, source, manifest, resources, application_id) = fixture("labels");
        let target = target_bundle(manifest, resources, |manifest, _| {
            manifest.workflow.stages[0].labels = labels("Updated intake label");
        });
        let workspace_root = workspace.paths.root.clone();
        let old_catalog = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .project(&application_id)
        .expect("old projections");
        let preview = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &target)
            .expect("preview");
        assert!(!preview.impact.plan_invalidated);
        assert!(preview.impact.stale_plan_ids.is_empty());
        assert!(preview.impact.stale_deliverable_ids.is_empty());
        assert_eq!(
            preview.impact.superseded_projection_paths.len(),
            old_catalog.projections.len()
        );
        let result = ApplicationPackMigrationService::new(&mut workspace.database)
            .migrate(
                &application_id,
                revision(1),
                &preview.preview_sha256,
                &source,
                &target,
                ActorKind::User,
                "upgrade-pack-labels",
            )
            .expect("migrate Pack");
        assert_eq!(result.stored.snapshot.application.revision, revision(2));
        assert_eq!(
            result.stored.snapshot.plan.as_ref().expect("Plan").state,
            PlanStateV3::Confirmed
        );
        assert!(
            result
                .stored
                .snapshot
                .deliverables
                .iter()
                .all(|deliverable| deliverable.state == DeliverableStateV3::Draft)
        );
        assert!(
            result
                .stored
                .snapshot
                .requirements
                .iter()
                .all(|requirement| requirement.pack == result.target_pack)
        );
        let superseded = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .catalog(&application_id)
        .expect("superseded catalog");
        assert!(superseded.projections.iter().all(|row| row.superseded));
        let removed = superseded.projections[0].relative_path.clone();
        fs::remove_file(workspace_root.join(removed.as_str())).expect("remove old projection");
        assert_eq!(
            ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
                .repair_all(&mut executor)
                .expect("repair skips superseded rows"),
            0
        );
        assert!(!workspace_root.join(removed.as_str()).exists());
        let current = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .project(&application_id)
        .expect("current projections");
        assert!(current.projections.iter().all(|row| !row.superseded));
    }

    #[test]
    fn template_change_invalidates_only_the_affected_deliverable_kind() {
        let (_root, mut workspace, source, manifest, resources, application_id) =
            fixture("template");
        let target = target_bundle(manifest, resources, |_, resources| {
            resources.insert(
                safe_path("templates/statement.typ"),
                b"statement-v2".to_vec(),
            );
        });
        let preview = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &target)
            .expect("preview");
        assert!(!preview.impact.plan_invalidated);
        assert_eq!(
            preview.impact.stale_deliverable_ids,
            vec![deliverable_id(107)]
        );
        let result = ApplicationPackMigrationService::new(&mut workspace.database)
            .migrate(
                &application_id,
                revision(1),
                &preview.preview_sha256,
                &source,
                &target,
                ActorKind::User,
                "upgrade-statement-template",
            )
            .expect("migrate Pack");
        let plan = result.stored.snapshot.plan.expect("Plan");
        assert_eq!(plan.state, PlanStateV3::Confirmed);
        assert_eq!(plan.revision, revision(2));
        let statement = result
            .stored
            .snapshot
            .deliverables
            .iter()
            .find(|deliverable| deliverable.id == deliverable_id(107))
            .expect("statement");
        let appendix = result
            .stored
            .snapshot
            .deliverables
            .iter()
            .find(|deliverable| deliverable.id == deliverable_id(108))
            .expect("appendix");
        assert_eq!(statement.state, DeliverableStateV3::Stale);
        assert_eq!(statement.plan.revision, revision(1));
        assert_eq!(appendix.state, DeliverableStateV3::Draft);
        assert_eq!(appendix.plan.revision, revision(2));
    }

    #[test]
    fn requirement_contract_change_stales_the_plan_and_its_materialized_outputs() {
        let (_root, mut workspace, source, manifest, resources, application_id) =
            fixture("requirements");
        let target = target_bundle(manifest, resources, |manifest, _| {
            manifest.requirements.categories[0]
                .fields
                .push(WorkflowPackFieldDefinition {
                    id: item("detail"),
                    labels: labels("Detail"),
                    field_type: WorkflowPackFieldType::LongText,
                    required: false,
                    options: Vec::new(),
                });
        });
        let preview = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &target)
            .expect("preview");
        assert!(preview.impact.plan_invalidated);
        assert_eq!(preview.impact.stale_plan_ids, vec![plan_id(104)]);
        assert_eq!(
            preview.impact.stale_deliverable_ids,
            vec![deliverable_id(107), deliverable_id(108)]
        );
        let result = ApplicationPackMigrationService::new(&mut workspace.database)
            .migrate(
                &application_id,
                revision(1),
                &preview.preview_sha256,
                &source,
                &target,
                ActorKind::User,
                "upgrade-requirement-contract",
            )
            .expect("migrate Pack");
        let plan = result.stored.snapshot.plan.expect("Plan");
        assert_eq!(plan.state, PlanStateV3::Stale);
        assert_eq!(plan.requirement_inputs[0].revision, revision(1));
        assert!(
            result
                .stored
                .snapshot
                .deliverables
                .iter()
                .all(|deliverable| {
                    deliverable.state == DeliverableStateV3::Stale
                        && deliverable.plan.revision == revision(1)
                })
        );
    }

    #[test]
    fn stale_preview_and_ledger_failure_are_atomic_and_retryable() {
        let (_root, mut workspace, source, manifest, resources, application_id) = fixture("atomic");
        let target = target_bundle(manifest, resources, |manifest, _| {
            manifest
                .locales
                .get_mut(&locale("en"))
                .expect("locale")
                .application_singular = "Submission".to_owned();
        });
        let preview = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &target)
            .expect("preview");
        let workspace_root = workspace.paths.root.clone();
        ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .project(&application_id)
        .expect("projection changes preview inputs");
        let error = ApplicationPackMigrationService::new(&mut workspace.database)
            .migrate(
                &application_id,
                revision(1),
                &preview.preview_sha256,
                &source,
                &target,
                ActorKind::User,
                "reject-stale-preview",
            )
            .expect_err("projection-set change must stale preview");
        assert!(matches!(error, StoreError::TaskStale(_)));
        let current_preview = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &target)
            .expect("current preview");
        workspace
            .database
            .connection()
            .execute_batch(
                "CREATE TEMP TRIGGER fail_pack_migration_ledger
                 BEFORE INSERT ON application_pack_v3_migrations
                 BEGIN SELECT RAISE(ABORT, 'pack migration ledger fixture'); END;",
            )
            .expect("failure trigger");
        ApplicationPackMigrationService::new(&mut workspace.database)
            .migrate(
                &application_id,
                revision(1),
                &current_preview.preview_sha256,
                &source,
                &target,
                ActorKind::User,
                "inject-ledger-failure",
            )
            .expect_err("ledger failure must roll back");
        let stored = ApplicationModelRepository::new(&mut workspace.database)
            .get(&application_id)
            .expect("rolled-back Application");
        assert_eq!(stored.snapshot.application.revision, revision(1));
        let ledger_count: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_pack_v3_migrations",
                [],
                |row| row.get(0),
            )
            .expect("ledger count");
        assert_eq!(ledger_count, 0);
        workspace
            .database
            .connection()
            .execute_batch("DROP TRIGGER fail_pack_migration_ledger")
            .expect("drop trigger");
        ApplicationPackMigrationService::new(&mut workspace.database)
            .migrate(
                &application_id,
                revision(1),
                &current_preview.preview_sha256,
                &source,
                &target,
                ActorKind::User,
                "retry-pack-migration",
            )
            .expect("retry migration");
        let replay = ApplicationPackMigrationService::new(&mut workspace.database)
            .migrate(
                &application_id,
                revision(1),
                &current_preview.preview_sha256,
                &source,
                &target,
                ActorKind::User,
                "reject-pack-replay",
            )
            .expect_err("old revision cannot migrate twice");
        assert!(matches!(replay, StoreError::TaskStale(_)));
    }

    #[test]
    fn wrong_source_id_digest_and_missing_migration_fail_without_mutation() {
        let (_root, mut workspace, source, manifest, resources, application_id) =
            fixture("wrong-pack");
        let valid_target = target_bundle(manifest.clone(), resources.clone(), |manifest, _| {
            manifest.publisher.name = "CanISend updated".to_owned();
        });

        let mut substitute_manifest = manifest.clone();
        substitute_manifest.publisher.name = "Substituted source".to_owned();
        let substituted_source = recalculated_bundle(substitute_manifest, resources.clone());
        let error = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &substituted_source, &valid_target)
            .expect_err("same-version digest substitution must not match current authority");
        assert!(matches!(error, StoreError::ApplicationModelConflict(_)));

        let wrong_id_target = target_bundle(manifest.clone(), resources.clone(), |manifest, _| {
            manifest.id =
                WorkflowPackId::try_new("org.canisend.other-migration-test").expect("Pack ID");
        });
        let error = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &wrong_id_target)
            .expect_err("Pack ID change must fail closed");
        assert!(matches!(error, StoreError::ApplicationModelConflict(_)));

        let collapsed_mapping_target =
            target_bundle(manifest.clone(), resources.clone(), |manifest, _| {
                manifest.migrations[0].mappings = vec![WorkflowPackIdMapping {
                    kind: WorkflowPackMigrationKind::Stage,
                    from: item("intake"),
                    to: item("plan"),
                }];
            });
        let error = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &collapsed_mapping_target)
            .expect_err("many-to-one Pack mappings must fail closed");
        assert!(matches!(error, StoreError::ApplicationModelConflict(_)));

        let mut undeclared_manifest = manifest;
        undeclared_manifest.version = SemanticVersion::try_new("1.1.0").expect("target version");
        undeclared_manifest.publisher.name = "Undeclared migration".to_owned();
        let undeclared_target = recalculated_bundle(undeclared_manifest, resources);
        let error = ApplicationPackMigrationService::new(&mut workspace.database)
            .preview(&application_id, &source, &undeclared_target)
            .expect_err("target without predecessor migration must fail closed");
        assert!(matches!(error, StoreError::ApplicationModelConflict(_)));

        let stored = ApplicationModelRepository::new(&mut workspace.database)
            .get(&application_id)
            .expect("unchanged Application");
        assert_eq!(stored.snapshot.application.revision, revision(1));
        assert_eq!(stored.snapshot.pack, binding(&source));
        let ledger_count: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_pack_v3_migrations",
                [],
                |row| row.get(0),
            )
            .expect("ledger count");
        assert_eq!(ledger_count, 0);
    }
}
