use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    ActorKind, ApplicationId, ApplicationModelSnapshotV3, ConsentScope, DeliverableId,
    DeliverableRecordV3, DeliverableStateV3, OpportunityRecordV3, PlanId, PlanRecordV3,
    PlanStateV3, RequirementRecordV3, Revision, SemanticValidate, Sha256Digest, UtcTimestamp,
    WORKSPACE_V4_FORMAT, WorkflowPackItemId, validate_application_model_snapshot_v3,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{Database, StoreError, generate_id, now_utc};
use crate::{PreparedWorkspaceSourceV4, association_v4::insert_prepared_source_association};

pub use canisend_contracts::WORKSPACE_V3_FORMAT;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceV3AuthorityState {
    pub workspace_format: String,
    pub activated_at: UtcTimestamp,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredApplicationModelV3 {
    pub snapshot: ApplicationModelSnapshotV3,
    pub snapshot_sha256: Sha256Digest,
    pub committed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationModelRevisionV3 {
    pub application_id: ApplicationId,
    pub revision: Revision,
    pub snapshot_sha256: Sha256Digest,
    pub actor: ActorKind,
    pub reason: String,
    pub committed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationModelCommitResultV3 {
    pub stored: StoredApplicationModelV3,
    pub stale_plan_ids: Vec<PlanId>,
    pub stale_deliverable_ids: Vec<DeliverableId>,
}

pub struct ApplicationModelRepository<'a> {
    database: &'a mut Database,
}

impl<'a> ApplicationModelRepository<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database) -> Self {
        Self { database }
    }

    pub fn authority(&self) -> Result<WorkspaceV3AuthorityState, StoreError> {
        load_authority(self.database.connection())
    }

    /// Activates canonical v3 authority only for a freshly initialized Workspace.
    ///
    /// Legacy Workspaces must continue to use the verified, backup-backed migration boundary.
    pub fn activate_empty_workspace(
        &mut self,
        actor: ActorKind,
        reason: &str,
    ) -> Result<WorkspaceV3AuthorityState, StoreError> {
        match load_authority(self.database.connection()) {
            Ok(authority) => {
                return Err(StoreError::ApplicationModelConflict(format!(
                    "Application authority is already active for {}",
                    authority.workspace_format
                )));
            }
            Err(StoreError::ApplicationModelUnavailable) => {}
            Err(error) => return Err(error),
        }
        ensure_workspace_has_no_product_data(self.database.connection())?;
        activate_workspace_v3_authority(self.database, actor, reason)
    }

    pub fn create(
        &mut self,
        snapshot: ApplicationModelSnapshotV3,
        actor: ActorKind,
        reason: &str,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        self.create_internal(snapshot, None, actor, reason)
    }

    pub fn create_with_source(
        &mut self,
        snapshot: ApplicationModelSnapshotV3,
        source: PreparedWorkspaceSourceV4,
        actor: ActorKind,
        reason: &str,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        self.create_with_source_and_consent(snapshot, source, None, actor, reason)
    }

    pub fn create_with_source_and_consent(
        &mut self,
        snapshot: ApplicationModelSnapshotV3,
        source: PreparedWorkspaceSourceV4,
        consent: Option<ConsentScope>,
        actor: ActorKind,
        reason: &str,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        validate_prepared_source_binding(&snapshot, &source)?;
        self.create_internal(snapshot, Some((source, consent)), actor, reason)
    }

    fn create_internal(
        &mut self,
        snapshot: ApplicationModelSnapshotV3,
        source: Option<(PreparedWorkspaceSourceV4, Option<ConsentScope>)>,
        actor: ActorKind,
        reason: &str,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        validate_initial_revisions(&snapshot)?;
        validate_snapshot(&snapshot)?;
        let reason = validate_reason(reason)?.to_owned();
        let actor_name = enum_name(actor)?;
        let (snapshot_json, snapshot_sha256) = serialize_snapshot(&snapshot)?;
        let committed_at = now_utc()?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        ensure_authority(&transaction)?;
        let exists = transaction
            .query_row(
                "SELECT 1 FROM application_model_v3_heads WHERE application_id = ?1",
                [snapshot.application.id.as_str()],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if exists {
            return Err(StoreError::ApplicationModelConflict(format!(
                "Application {} already exists",
                snapshot.application.id
            )));
        }
        transaction.execute(
            "INSERT INTO application_model_v3_heads(
                application_id, opportunity_id, pack_id, pack_version, pack_digest,
                head_revision, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
            params![
                snapshot.application.id.as_str(),
                snapshot.opportunity.id.as_str(),
                snapshot.pack.id.as_str(),
                snapshot.pack.version.as_str(),
                snapshot.pack.content_digest.as_str(),
                snapshot.application.created_at.as_str(),
                snapshot.application.updated_at.as_str(),
            ],
        )?;
        insert_revision(
            &transaction,
            &snapshot,
            &snapshot_json,
            &snapshot_sha256,
            &actor_name,
            &reason,
            &committed_at,
        )?;
        if let Some((source, consent)) = &source {
            insert_prepared_source_association(
                &transaction,
                &snapshot.application.id,
                source,
                *consent,
            )?;
        }
        insert_content_blob_references(&transaction, &snapshot, &committed_at)?;
        insert_dependencies(&transaction, &snapshot)?;
        insert_audit(
            &transaction,
            event_id.as_str(),
            &actor_name,
            "application-v3.create",
            &snapshot.application.id,
            snapshot.application.revision,
            &reason,
            &committed_at,
        )?;
        transaction.commit()?;
        Ok(ApplicationModelCommitResultV3 {
            stored: StoredApplicationModelV3 {
                snapshot,
                snapshot_sha256,
                committed_at,
            },
            stale_plan_ids: Vec::new(),
            stale_deliverable_ids: Vec::new(),
        })
    }

    pub fn get(
        &self,
        application_id: &ApplicationId,
    ) -> Result<StoredApplicationModelV3, StoreError> {
        ensure_authority(self.database.connection())?;
        load_current(self.database.connection(), application_id)
    }

    pub fn list(&self) -> Result<Vec<StoredApplicationModelV3>, StoreError> {
        ensure_authority(self.database.connection())?;
        let mut statement = self.database.connection().prepare(
            "SELECT application_id FROM application_model_v3_heads
             ORDER BY created_at, application_id",
        )?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| {
                let id = ApplicationId::try_new(id)?;
                load_current(self.database.connection(), &id)
            })
            .collect()
    }

    pub fn history(
        &self,
        application_id: &ApplicationId,
    ) -> Result<Vec<ApplicationModelRevisionV3>, StoreError> {
        ensure_authority(self.database.connection())?;
        let exists = self
            .database
            .connection()
            .query_row(
                "SELECT 1 FROM application_model_v3_heads WHERE application_id = ?1",
                [application_id.as_str()],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(StoreError::ApplicationModelNotFound(
                application_id.to_string(),
            ));
        }
        let mut statement = self.database.connection().prepare(
            "SELECT revision, snapshot_sha256, actor, reason, created_at
             FROM application_model_v3_revisions
             WHERE application_id = ?1 ORDER BY revision",
        )?;
        statement
            .query_map([application_id.as_str()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .map(|row| {
                let (revision, digest, actor, reason, committed_at) = row?;
                if WorkflowPackItemId::try_new(&reason).is_err() {
                    return Err(StoreError::ApplicationModelIntegrity(
                        "stored revision reason is not a body-free code".to_owned(),
                    ));
                }
                Ok(ApplicationModelRevisionV3 {
                    application_id: application_id.clone(),
                    revision: Revision::try_new(to_u64(revision)?)?,
                    snapshot_sha256: Sha256Digest::try_new(digest)?,
                    actor: enum_value(&actor)?,
                    reason,
                    committed_at: UtcTimestamp::try_new(committed_at)?,
                })
            })
            .collect()
    }

    pub fn commit(
        &mut self,
        application_id: &ApplicationId,
        expected_revision: Revision,
        candidate: ApplicationModelSnapshotV3,
        actor: ActorKind,
        reason: &str,
    ) -> Result<ApplicationModelCommitResultV3, StoreError> {
        let reason = validate_reason(reason)?.to_owned();
        let actor_name = enum_name(actor)?;
        let committed_at = now_utc()?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        ensure_authority(&transaction)?;
        let current = load_current(&transaction, application_id)?;
        if current.snapshot.application.revision != expected_revision {
            return Err(StoreError::ApplicationModelConflict(format!(
                "expected Application revision {}, found {}",
                expected_revision.get(),
                current.snapshot.application.revision.get()
            )));
        }
        let (snapshot, stale_plan_ids, stale_deliverable_ids) =
            prepare_update(&current.snapshot, candidate)?;
        validate_snapshot(&snapshot)?;
        let (snapshot_json, snapshot_sha256) = serialize_snapshot(&snapshot)?;
        insert_revision(
            &transaction,
            &snapshot,
            &snapshot_json,
            &snapshot_sha256,
            &actor_name,
            &reason,
            &committed_at,
        )?;
        insert_content_blob_references(&transaction, &snapshot, &committed_at)?;
        let updated = transaction.execute(
            "UPDATE application_model_v3_heads
             SET opportunity_id = ?2, head_revision = ?3, updated_at = ?4
             WHERE application_id = ?1 AND head_revision = ?5
               AND pack_id = ?6 AND pack_version = ?7 AND pack_digest = ?8",
            params![
                application_id.as_str(),
                snapshot.opportunity.id.as_str(),
                to_i64(snapshot.application.revision.get())?,
                snapshot.application.updated_at.as_str(),
                to_i64(expected_revision.get())?,
                snapshot.pack.id.as_str(),
                snapshot.pack.version.as_str(),
                snapshot.pack.content_digest.as_str(),
            ],
        )?;
        if updated != 1 {
            return Err(StoreError::ApplicationModelConflict(
                "Application head or Pack binding changed during commit".to_owned(),
            ));
        }
        insert_dependencies(&transaction, &snapshot)?;
        insert_audit(
            &transaction,
            event_id.as_str(),
            &actor_name,
            "application-v3.commit",
            application_id,
            snapshot.application.revision,
            &reason,
            &committed_at,
        )?;
        transaction.commit()?;
        Ok(ApplicationModelCommitResultV3 {
            stored: StoredApplicationModelV3 {
                snapshot,
                snapshot_sha256,
                committed_at,
            },
            stale_plan_ids,
            stale_deliverable_ids,
        })
    }
}

fn validate_prepared_source_binding(
    snapshot: &ApplicationModelSnapshotV3,
    prepared: &PreparedWorkspaceSourceV4,
) -> Result<(), StoreError> {
    let source = &prepared.record;
    if source.revision.get() != 1 {
        return Err(StoreError::ApplicationAssociationConflict(
            "an intake Source must begin at revision one".to_owned(),
        ));
    }
    if !snapshot.opportunity.source_ids.contains(&source.id) {
        return Err(StoreError::ApplicationAssociationConflict(
            "Opportunity does not reference the prepared Source".to_owned(),
        ));
    }
    if snapshot.requirements.iter().any(|requirement| {
        let reference = &requirement.source_span.content;
        reference.id != source.id
            || reference.revision != source.revision
            || reference.sha256 != source.normalized_sha256
    }) {
        return Err(StoreError::ApplicationAssociationConflict(
            "Requirement span does not bind the exact prepared Source revision".to_owned(),
        ));
    }
    Ok(())
}

fn ensure_workspace_has_no_product_data(connection: &Connection) -> Result<(), StoreError> {
    const PRODUCT_TABLES: &[&str] = &[
        "jobs",
        "sources",
        "evidence_items",
        "artifacts",
        "workflow_runs",
        "tasks",
        "profile_sources",
        "discovery_sources",
        "job_leads",
        "application_model_v3_heads",
        "workspace_v3_migrations",
        "workspace_v3_application_links",
        "application_projection_v3_manifests",
        "application_pack_v3_migrations",
    ];
    for table in PRODUCT_TABLES {
        let count: i64 =
            connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })?;
        if count != 0 {
            return Err(StoreError::ApplicationModelConflict(format!(
                "Workspace contains product data in {table}; use the verified v2-to-v3 migration"
            )));
        }
    }
    Ok(())
}

// Shared by the failure-atomic GF2 migration boundary and the empty-Workspace activation above.
#[allow(dead_code)]
pub(crate) fn activate_workspace_v3_authority(
    database: &mut Database,
    actor: ActorKind,
    reason: &str,
) -> Result<WorkspaceV3AuthorityState, StoreError> {
    let reason = validate_reason(reason)?.to_owned();
    let activated_at = now_utc()?;
    let event_id = generate_id()?;
    let actor_name = enum_name(actor)?;
    let (workspace_id, _) = database.workspace_identity()?;
    let transaction = database.immediate_transaction()?;
    insert_workspace_v3_authority(
        &transaction,
        &workspace_id,
        event_id.as_str(),
        &actor_name,
        &reason,
        &activated_at,
    )?;
    transaction.commit()?;
    Ok(WorkspaceV3AuthorityState {
        workspace_format: WORKSPACE_V3_FORMAT.to_owned(),
        activated_at,
        reason,
    })
}

pub(crate) fn insert_workspace_v3_authority(
    transaction: &Transaction<'_>,
    workspace_id: &canisend_contracts::EntityId,
    event_id: &str,
    actor: &str,
    reason: &str,
    activated_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    if transaction
        .query_row(
            "SELECT 1 FROM workspace_v3_authority WHERE singleton = 1",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some()
    {
        return Err(StoreError::ApplicationModelConflict(
            "Workspace v3 authority is already active".to_owned(),
        ));
    }
    transaction.execute(
        "INSERT INTO workspace_v3_authority(singleton, workspace_format, activated_at, reason)
         VALUES (1, ?1, ?2, ?3)",
        params![WORKSPACE_V3_FORMAT, activated_at.as_str(), reason],
    )?;
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, ?2, 'workspace-v3.activate', ?3, NULL, ?4, ?5)",
        params![
            event_id,
            actor,
            workspace_id.as_str(),
            reason,
            activated_at.as_str()
        ],
    )?;
    Ok(())
}

pub(crate) fn insert_migrated_application_model(
    transaction: &Transaction<'_>,
    snapshot: &ApplicationModelSnapshotV3,
    actor: ActorKind,
    reason: &str,
    committed_at: &UtcTimestamp,
) -> Result<StoredApplicationModelV3, StoreError> {
    if snapshot.application.revision.get() != 1 {
        return Err(StoreError::ApplicationModelConflict(
            "a migrated Application must begin at aggregate revision one".to_owned(),
        ));
    }
    validate_snapshot(snapshot)?;
    let reason = validate_reason(reason)?;
    let actor = enum_name(actor)?;
    let (snapshot_json, snapshot_sha256) = serialize_snapshot(snapshot)?;
    transaction.execute(
        "INSERT INTO application_model_v3_heads(
            application_id, opportunity_id, pack_id, pack_version, pack_digest,
            head_revision, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
        params![
            snapshot.application.id.as_str(),
            snapshot.opportunity.id.as_str(),
            snapshot.pack.id.as_str(),
            snapshot.pack.version.as_str(),
            snapshot.pack.content_digest.as_str(),
            snapshot.application.created_at.as_str(),
            snapshot.application.updated_at.as_str(),
        ],
    )?;
    insert_revision(
        transaction,
        snapshot,
        &snapshot_json,
        &snapshot_sha256,
        &actor,
        reason,
        committed_at,
    )?;
    insert_dependencies(transaction, snapshot)?;
    insert_audit(
        transaction,
        generate_id()?.as_str(),
        &actor,
        "application-v3.create",
        &snapshot.application.id,
        snapshot.application.revision,
        reason,
        committed_at,
    )?;
    Ok(StoredApplicationModelV3 {
        snapshot: snapshot.clone(),
        snapshot_sha256,
        committed_at: committed_at.clone(),
    })
}

fn load_authority(connection: &Connection) -> Result<WorkspaceV3AuthorityState, StoreError> {
    let (workspace_format, created_at): (String, String) = connection.query_row(
        "SELECT workspace_format, created_at FROM workspace_metadata WHERE singleton = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if workspace_format == WORKSPACE_V4_FORMAT {
        return Ok(WorkspaceV3AuthorityState {
            workspace_format,
            activated_at: UtcTimestamp::try_new(created_at)?,
            reason: "clean-workspace-v4".to_owned(),
        });
    }

    let row = connection
        .query_row(
            "SELECT workspace_format, activated_at, reason
             FROM workspace_v3_authority WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let (workspace_format, activated_at, reason) =
        row.ok_or(StoreError::ApplicationModelUnavailable)?;
    if workspace_format != WORKSPACE_V3_FORMAT {
        return Err(StoreError::ApplicationModelIntegrity(format!(
            "unexpected authority format {workspace_format}"
        )));
    }
    if WorkflowPackItemId::try_new(&reason).is_err() {
        return Err(StoreError::ApplicationModelIntegrity(
            "Workspace Application-authority reason is not a body-free code".to_owned(),
        ));
    }
    Ok(WorkspaceV3AuthorityState {
        workspace_format,
        activated_at: UtcTimestamp::try_new(activated_at)?,
        reason,
    })
}

fn ensure_authority(connection: &Connection) -> Result<(), StoreError> {
    load_authority(connection).map(|_| ())
}

fn validate_initial_revisions(snapshot: &ApplicationModelSnapshotV3) -> Result<(), StoreError> {
    let one = Revision::try_new(1)?;
    let all_initial = snapshot.application.revision == one
        && snapshot.opportunity.revision == one
        && snapshot
            .requirements
            .iter()
            .all(|record| record.revision == one)
        && snapshot
            .plan
            .as_ref()
            .is_none_or(|record| record.revision == one)
        && snapshot
            .deliverables
            .iter()
            .all(|record| record.revision == one);
    if !all_initial {
        return Err(StoreError::ApplicationModelConflict(
            "new application-model entities must begin at revision one".to_owned(),
        ));
    }
    Ok(())
}

fn prepare_update(
    current: &ApplicationModelSnapshotV3,
    mut candidate: ApplicationModelSnapshotV3,
) -> Result<(ApplicationModelSnapshotV3, Vec<PlanId>, Vec<DeliverableId>), StoreError> {
    if candidate.application.id != current.application.id {
        return Err(StoreError::ApplicationModelConflict(
            "Application identity cannot change".to_owned(),
        ));
    }
    if candidate.opportunity.id != current.opportunity.id
        || candidate.application.opportunity_id != current.application.opportunity_id
    {
        return Err(StoreError::ApplicationModelConflict(
            "Opportunity identity cannot change in an Application revision".to_owned(),
        ));
    }
    if candidate.pack != current.pack {
        return Err(StoreError::ApplicationModelConflict(
            "Pack migration requires the dedicated migration boundary".to_owned(),
        ));
    }
    if candidate.application.created_at != current.application.created_at
        || candidate.opportunity.created_at != current.opportunity.created_at
    {
        return Err(StoreError::ApplicationModelConflict(
            "entity creation timestamps are immutable".to_owned(),
        ));
    }
    if !timestamp_after(
        &candidate.application.updated_at,
        &current.application.updated_at,
    )? {
        return Err(StoreError::ApplicationModelConflict(
            "Application updated_at must advance on commit".to_owned(),
        ));
    }
    let next_application_revision = next_revision(current.application.revision)?;
    if candidate.application.revision != next_application_revision {
        return Err(StoreError::ApplicationModelConflict(format!(
            "next Application revision must be {}",
            next_application_revision.get()
        )));
    }

    check_opportunity_transition(&current.opportunity, &candidate.opportunity)?;
    let current_requirements = current
        .requirements
        .iter()
        .map(|record| (&record.id, record))
        .collect::<BTreeMap<_, _>>();
    let candidate_requirements = candidate
        .requirements
        .iter()
        .map(|record| (&record.id, record))
        .collect::<BTreeMap<_, _>>();
    ensure_ids_preserved(
        current_requirements.keys().copied(),
        &candidate_requirements,
        "Requirement",
    )?;
    for requirement in &candidate.requirements {
        if let Some(current) = current_requirements.get(&requirement.id) {
            check_requirement_transition(current, requirement)?;
        } else if requirement.revision.get() != 1 {
            return Err(StoreError::ApplicationModelConflict(format!(
                "new Requirement {} must begin at revision one",
                requirement.id
            )));
        }
    }

    match (&current.plan, &candidate.plan) {
        (Some(current), Some(candidate)) => check_plan_transition(current, candidate)?,
        (Some(_), None) => {
            return Err(StoreError::ApplicationModelConflict(
                "a persisted Plan cannot be deleted; mark or revise it instead".to_owned(),
            ));
        }
        (None, Some(candidate)) if candidate.revision.get() != 1 => {
            return Err(StoreError::ApplicationModelConflict(
                "a new Plan must begin at revision one".to_owned(),
            ));
        }
        (None, Some(_)) | (None, None) => {}
    }

    let current_deliverables = current
        .deliverables
        .iter()
        .map(|record| (&record.id, record))
        .collect::<BTreeMap<_, _>>();
    let candidate_deliverables = candidate
        .deliverables
        .iter()
        .map(|record| (&record.id, record))
        .collect::<BTreeMap<_, _>>();
    ensure_ids_preserved(
        current_deliverables.keys().copied(),
        &candidate_deliverables,
        "Deliverable",
    )?;
    for deliverable in &candidate.deliverables {
        if let Some(current) = current_deliverables.get(&deliverable.id) {
            check_deliverable_transition(current, deliverable)?;
        } else if deliverable.revision.get() != 1 {
            return Err(StoreError::ApplicationModelConflict(format!(
                "new Deliverable {} must begin at revision one",
                deliverable.id
            )));
        }
    }

    let changed_requirement_ids = candidate
        .requirements
        .iter()
        .filter_map(|requirement| {
            current_requirements
                .get(&requirement.id)
                .filter(|current| current.revision != requirement.revision)
                .map(|_| requirement.id.clone())
        })
        .collect::<BTreeSet<_>>();

    let mut stale_plan_ids = Vec::new();
    if let (Some(current_plan), Some(candidate_plan)) = (&current.plan, candidate.plan.as_mut()) {
        let consumes_changed_requirement = current_plan
            .requirement_inputs
            .iter()
            .any(|reference| changed_requirement_ids.contains(&reference.id));
        if consumes_changed_requirement
            && candidate_plan.revision == current_plan.revision
            && current_plan.state != PlanStateV3::Stale
        {
            candidate_plan.state = PlanStateV3::Stale;
            candidate_plan.revision = next_revision(current_plan.revision)?;
            stale_plan_ids.push(candidate_plan.id.clone());
        }
    }

    let plan_revision_changed = match (&current.plan, &candidate.plan) {
        (Some(current), Some(candidate)) => current.revision != candidate.revision,
        (None, Some(_)) => true,
        _ => false,
    };
    let mut stale_deliverable_ids = Vec::new();
    for deliverable in &mut candidate.deliverables {
        let Some(current_deliverable) = current_deliverables.get(&deliverable.id) else {
            continue;
        };
        let consumes_changed_requirement =
            current_deliverable.evidence_inputs.iter().any(|input| {
                changed_requirement_ids
                    .iter()
                    .any(|id| id.as_entity_id() == &input.id)
            });
        if deliverable.revision == current_deliverable.revision
            && current_deliverable.state != DeliverableStateV3::Planned
            && current_deliverable.state != DeliverableStateV3::Stale
            && (plan_revision_changed || consumes_changed_requirement)
        {
            deliverable.state = DeliverableStateV3::Stale;
            deliverable.revision = next_revision(current_deliverable.revision)?;
            stale_deliverable_ids.push(deliverable.id.clone());
        }
    }

    Ok((candidate, stale_plan_ids, stale_deliverable_ids))
}

fn check_opportunity_transition(
    current: &OpportunityRecordV3,
    candidate: &OpportunityRecordV3,
) -> Result<(), StoreError> {
    let mut normalized = candidate.clone();
    normalized.revision = current.revision;
    check_revision_transition(
        "Opportunity",
        current.revision,
        candidate.revision,
        &normalized != current,
    )
}

fn check_requirement_transition(
    current: &RequirementRecordV3,
    candidate: &RequirementRecordV3,
) -> Result<(), StoreError> {
    let mut normalized = candidate.clone();
    normalized.revision = current.revision;
    check_revision_transition(
        "Requirement",
        current.revision,
        candidate.revision,
        &normalized != current,
    )
}

fn check_plan_transition(
    current: &PlanRecordV3,
    candidate: &PlanRecordV3,
) -> Result<(), StoreError> {
    if candidate.id != current.id {
        return Err(StoreError::ApplicationModelConflict(
            "Plan identity cannot change".to_owned(),
        ));
    }
    if candidate.state == PlanStateV3::Stale && candidate != current {
        return Err(StoreError::ApplicationModelConflict(
            "Plan stale transitions are repository-owned; replan against current inputs instead"
                .to_owned(),
        ));
    }
    let mut normalized = candidate.clone();
    normalized.revision = current.revision;
    check_revision_transition(
        "Plan",
        current.revision,
        candidate.revision,
        &normalized != current,
    )
}

fn check_deliverable_transition(
    current: &DeliverableRecordV3,
    candidate: &DeliverableRecordV3,
) -> Result<(), StoreError> {
    if candidate.state == DeliverableStateV3::Stale && candidate != current {
        return Err(StoreError::ApplicationModelConflict(
            "Deliverable stale transitions are repository-owned; regenerate against the current Plan instead"
                .to_owned(),
        ));
    }
    let mut normalized = candidate.clone();
    normalized.revision = current.revision;
    check_revision_transition(
        "Deliverable",
        current.revision,
        candidate.revision,
        &normalized != current,
    )
}

fn check_revision_transition(
    kind: &str,
    current: Revision,
    candidate: Revision,
    changed: bool,
) -> Result<(), StoreError> {
    let expected = if changed {
        next_revision(current)?
    } else {
        current
    };
    if candidate != expected {
        return Err(StoreError::ApplicationModelConflict(format!(
            "{kind} revision must be {} when content {}",
            expected.get(),
            if changed { "changes" } else { "is unchanged" }
        )));
    }
    Ok(())
}

fn ensure_ids_preserved<'a, T: Ord + std::fmt::Display + 'a>(
    current: impl IntoIterator<Item = &'a T>,
    candidate: &BTreeMap<&T, impl Sized>,
    kind: &str,
) -> Result<(), StoreError> {
    for id in current {
        if !candidate.contains_key(id) {
            return Err(StoreError::ApplicationModelConflict(format!(
                "persisted {kind} {id} cannot be deleted"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_snapshot(snapshot: &ApplicationModelSnapshotV3) -> Result<(), StoreError> {
    let violations = snapshot.validate_semantics();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(StoreError::CandidateSemantic(violations))
    }
}

pub(crate) fn serialize_snapshot(
    snapshot: &ApplicationModelSnapshotV3,
) -> Result<(String, Sha256Digest), StoreError> {
    let snapshot_json = serde_json::to_string(snapshot)?;
    let digest = Sha256::digest(snapshot_json.as_bytes());
    let snapshot_sha256 = Sha256Digest::try_new(hex::encode(digest))?;
    Ok((snapshot_json, snapshot_sha256))
}

pub(crate) fn load_current(
    connection: &Connection,
    application_id: &ApplicationId,
) -> Result<StoredApplicationModelV3, StoreError> {
    type CurrentRow = (i64, String, String, String, String, String, String, String);
    let row: Option<CurrentRow> = connection
        .query_row(
            "SELECT head.head_revision, revision.snapshot_json, revision.snapshot_sha256,
                    revision.created_at, head.opportunity_id, head.pack_id,
                    head.pack_version, head.pack_digest
             FROM application_model_v3_heads AS head
             JOIN application_model_v3_revisions AS revision
               ON revision.application_id = head.application_id
              AND revision.revision = head.head_revision
             WHERE head.application_id = ?1",
            [application_id.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()?;
    let (
        head_revision,
        snapshot_json,
        stored_digest,
        committed_at,
        opportunity_id,
        pack_id,
        pack_version,
        pack_digest,
    ) = row.ok_or_else(|| StoreError::ApplicationModelNotFound(application_id.to_string()))?;
    let calculated_digest = hex::encode(Sha256::digest(snapshot_json.as_bytes()));
    if calculated_digest != stored_digest {
        return Err(StoreError::ApplicationModelIntegrity(format!(
            "snapshot digest mismatch for Application {application_id}"
        )));
    }
    let value = serde_json::from_str(&snapshot_json)?;
    let snapshot = match validate_application_model_snapshot_v3(&value) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return Err(StoreError::ApplicationModelIntegrity(format!(
                "stored snapshot contract validation failed: {error}"
            )));
        }
    };
    if snapshot.application.id != *application_id
        || snapshot.application.revision.get() != to_u64(head_revision)?
        || snapshot.opportunity.id.as_str() != opportunity_id
        || snapshot.pack.id.as_str() != pack_id
        || snapshot.pack.version.as_str() != pack_version
        || snapshot.pack.content_digest.as_str() != pack_digest
    {
        return Err(StoreError::ApplicationModelIntegrity(format!(
            "head metadata differs from snapshot for Application {application_id}"
        )));
    }
    Ok(StoredApplicationModelV3 {
        snapshot,
        snapshot_sha256: Sha256Digest::try_new(stored_digest)?,
        committed_at: UtcTimestamp::try_new(committed_at)?,
    })
}

pub(crate) fn load_application_model_revision(
    connection: &Connection,
    application_id: &ApplicationId,
    revision: Revision,
) -> Result<StoredApplicationModelV3, StoreError> {
    let row: Option<(String, String, String)> = connection
        .query_row(
            "SELECT snapshot_json, snapshot_sha256, created_at
             FROM application_model_v3_revisions
             WHERE application_id = ?1 AND revision = ?2",
            params![application_id.as_str(), to_i64(revision.get())?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (snapshot_json, stored_digest, committed_at) = row.ok_or_else(|| {
        StoreError::ApplicationModelNotFound(format!("{application_id}@{}", revision.get()))
    })?;
    let calculated_digest = hex::encode(Sha256::digest(snapshot_json.as_bytes()));
    if calculated_digest != stored_digest {
        return Err(StoreError::ApplicationModelIntegrity(format!(
            "snapshot digest mismatch for Application {application_id}@{}",
            revision.get()
        )));
    }
    let value = serde_json::from_str(&snapshot_json)?;
    let snapshot = validate_application_model_snapshot_v3(&value).map_err(|error| {
        StoreError::ApplicationModelIntegrity(format!(
            "stored snapshot contract validation failed: {error}"
        ))
    })?;
    if snapshot.application.id != *application_id || snapshot.application.revision != revision {
        return Err(StoreError::ApplicationModelIntegrity(format!(
            "stored snapshot identity differs for Application {application_id}@{}",
            revision.get()
        )));
    }
    Ok(StoredApplicationModelV3 {
        snapshot,
        snapshot_sha256: Sha256Digest::try_new(stored_digest)?,
        committed_at: UtcTimestamp::try_new(committed_at)?,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn insert_revision(
    transaction: &Transaction<'_>,
    snapshot: &ApplicationModelSnapshotV3,
    snapshot_json: &str,
    snapshot_sha256: &Sha256Digest,
    actor: &str,
    reason: &str,
    committed_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO application_model_v3_revisions(
            application_id, revision, snapshot_json, snapshot_sha256, actor, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            snapshot.application.id.as_str(),
            to_i64(snapshot.application.revision.get())?,
            snapshot_json,
            snapshot_sha256.as_str(),
            actor,
            reason,
            committed_at.as_str(),
        ],
    )?;
    Ok(())
}

pub(crate) fn insert_content_blob_references(
    transaction: &Transaction<'_>,
    snapshot: &ApplicationModelSnapshotV3,
    committed_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    for requirement in &snapshot.requirements {
        let content = &requirement.source_span.content;
        transaction.execute(
            "INSERT OR IGNORE INTO blob_references(
                sha256, owner_type, owner_id, owner_revision, created_at
             ) VALUES (?1, 'application-v3-source', ?2, ?3, ?4)",
            params![
                content.sha256.as_str(),
                content.id.as_str(),
                to_i64(content.revision.get())?,
                committed_at.as_str(),
            ],
        )?;
    }
    for deliverable in &snapshot.deliverables {
        let Some(content) = &deliverable.content else {
            continue;
        };
        transaction.execute(
            "INSERT OR IGNORE INTO blob_references(
                sha256, owner_type, owner_id, owner_revision, created_at
             ) VALUES (?1, 'application-v3-content', ?2, ?3, ?4)",
            params![
                content.sha256.as_str(),
                content.id.as_str(),
                to_i64(content.revision.get())?,
                committed_at.as_str(),
            ],
        )?;
    }
    Ok(())
}

pub(crate) fn insert_dependencies(
    transaction: &Transaction<'_>,
    snapshot: &ApplicationModelSnapshotV3,
) -> Result<(), StoreError> {
    let application_revision = to_i64(snapshot.application.revision.get())?;
    if let Some(plan) = &snapshot.plan {
        for requirement in &plan.requirement_inputs {
            insert_dependency(
                transaction,
                snapshot.application.id.as_str(),
                application_revision,
                "plan",
                plan.id.as_str(),
                plan.revision,
                "requirement",
                requirement.id.as_str(),
                requirement.revision,
            )?;
        }
        for blocker in &plan.blockers {
            if let Some(requirement) = &blocker.requirement {
                insert_dependency(
                    transaction,
                    snapshot.application.id.as_str(),
                    application_revision,
                    "plan",
                    plan.id.as_str(),
                    plan.revision,
                    "requirement",
                    requirement.id.as_str(),
                    requirement.revision,
                )?;
            }
        }
    }
    for deliverable in &snapshot.deliverables {
        insert_dependency(
            transaction,
            snapshot.application.id.as_str(),
            application_revision,
            "deliverable",
            deliverable.id.as_str(),
            deliverable.revision,
            "plan",
            deliverable.plan.id.as_str(),
            deliverable.plan.revision,
        )?;
        for evidence in &deliverable.evidence_inputs {
            insert_dependency(
                transaction,
                snapshot.application.id.as_str(),
                application_revision,
                "deliverable",
                deliverable.id.as_str(),
                deliverable.revision,
                "evidence",
                evidence.id.as_str(),
                evidence.revision,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_dependency(
    transaction: &Transaction<'_>,
    application_id: &str,
    application_revision: i64,
    dependent_kind: &str,
    dependent_id: &str,
    dependent_revision: Revision,
    upstream_kind: &str,
    upstream_id: &str,
    upstream_revision: Revision,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT OR IGNORE INTO application_model_v3_dependencies(
            application_id, application_revision, dependent_kind, dependent_id,
            dependent_revision, upstream_kind, upstream_id, upstream_revision
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            application_id,
            application_revision,
            dependent_kind,
            dependent_id,
            to_i64(dependent_revision.get())?,
            upstream_kind,
            upstream_id,
            to_i64(upstream_revision.get())?,
        ],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn insert_audit(
    transaction: &Transaction<'_>,
    event_id: &str,
    actor: &str,
    action: &str,
    application_id: &ApplicationId,
    revision: Revision,
    reason: &str,
    committed_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event_id,
            actor,
            action,
            application_id.as_str(),
            to_i64(revision.get())?,
            reason,
            committed_at.as_str(),
        ],
    )?;
    Ok(())
}

pub(crate) fn validate_reason(value: &str) -> Result<&str, StoreError> {
    WorkflowPackItemId::try_new(value).map_err(|_| {
        StoreError::InvalidInput(
            "application-model reason must be a bounded lowercase kebab-case code".to_owned(),
        )
    })?;
    Ok(value)
}

pub(crate) fn next_revision(revision: Revision) -> Result<Revision, StoreError> {
    Revision::try_new(
        revision
            .get()
            .checked_add(1)
            .ok_or_else(|| StoreError::Invariant("revision overflow".to_owned()))?,
    )
    .map_err(StoreError::from)
}

fn timestamp_after(left: &UtcTimestamp, right: &UtcTimestamp) -> Result<bool, StoreError> {
    let left = OffsetDateTime::parse(left.as_str(), &Rfc3339)
        .map_err(|error| StoreError::Invariant(error.to_string()))?;
    let right = OffsetDateTime::parse(right.as_str(), &Rfc3339)
        .map_err(|error| StoreError::Invariant(error.to_string()))?;
    Ok(left > right)
}

pub(crate) fn enum_name<T: Serialize>(value: T) -> Result<String, StoreError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| StoreError::Invariant("enum did not serialize as a string".to_owned()))
}

fn enum_value<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, StoreError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(StoreError::from)
}

pub(crate) fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value)
        .map_err(|_| StoreError::Invariant("revision exceeds SQLite i64".to_owned()))
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
        sync::{
            Arc, Barrier,
            atomic::{AtomicU64, Ordering},
        },
        thread,
    };

    use canisend_contracts::{
        ApplicationFieldValueV3, ApplicationLifecycleV3, ApplicationModelFormatV3,
        ApplicationPackBindingV3, ApplicationRecordV3, ContentRevisionReferenceV3, ContentSpanV3,
        DeliverableKindId, EntityId, EntityRevisionReferenceV3, ExecutionMode, OpportunityId,
        PlanRevisionReferenceV3, PlannedDeliverableDispositionV3, PlannedDeliverableV3,
        RequirementConfirmationV3, RequirementPriorityV3, RequirementRevisionReferenceV3,
        SemanticVersion, WorkflowPackId, WorkflowPackItemId,
    };

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct TestDatabase {
        path: PathBuf,
        database: Option<Database>,
    }

    impl TestDatabase {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "canisend-application-v3-{label}-{}-{}.sqlite3",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut database = Database::open(&path).expect("database");
            database
                .initialize_workspace(&entity_id(900), &timestamp("2026-08-02T10:00:00Z"))
                .expect("workspace identity");
            Self {
                path,
                database: Some(database),
            }
        }

        fn new_v4(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "canisend-application-v4-{label}-{}-{}.sqlite3",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut database = Database::open(&path).expect("database");
            database
                .initialize_workspace_with_format(
                    &entity_id(901),
                    &timestamp("2026-08-08T10:00:00Z"),
                    WORKSPACE_V4_FORMAT,
                )
                .expect("Workspace v4 identity");
            database
                .initialize_workspace_v4_application_storage(&timestamp("2026-08-08T10:00:00Z"))
                .expect("Workspace v4 Application storage");
            Self {
                path,
                database: Some(database),
            }
        }

        fn database(&mut self) -> &mut Database {
            self.database.as_mut().expect("database remains open")
        }

        fn close(&mut self) {
            self.database.take();
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            self.database.take();
            remove_database_files(&self.path);
        }
    }

    fn remove_database_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{}-wal", path.display()));
        let _ = fs::remove_file(format!("{}-shm", path.display()));
    }

    fn activate(database: &mut Database) {
        activate_workspace_v3_authority(database, ActorKind::User, "test-activation")
            .expect("activate v3 authority");
    }

    #[test]
    fn empty_workspace_activation_rejects_legacy_product_data() {
        let mut fixture = TestDatabase::new("activation-legacy-data");
        let path = fixture.path.clone();
        let blobs = crate::BlobStore::new(
            path.with_extension("blobs"),
            path.with_extension("temporary"),
        );
        crate::JobService::new(fixture.database(), &blobs)
            .create(
                "Legacy opportunity",
                "Example organization",
                ActorKind::User,
            )
            .expect("legacy data");

        let error = ApplicationModelRepository::new(fixture.database())
            .activate_empty_workspace(ActorKind::User, "new-workspace-v3")
            .expect_err("legacy data must use migration");
        assert!(matches!(error, StoreError::ApplicationModelConflict(_)));
        assert!(matches!(
            ApplicationModelRepository::new(fixture.database()).authority(),
            Err(StoreError::ApplicationModelUnavailable)
        ));
        let _ = fs::remove_dir_all(path.with_extension("blobs"));
        let _ = fs::remove_dir_all(path.with_extension("temporary"));
    }

    fn pack() -> ApplicationPackBindingV3 {
        ApplicationPackBindingV3 {
            id: WorkflowPackId::try_new("org.canisend.generic-application").expect("pack ID"),
            version: SemanticVersion::try_new("1.0.0").expect("version"),
            content_digest: Sha256Digest::try_new("a".repeat(64)).expect("digest"),
        }
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("item ID")
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

    fn requirement_id(suffix: u16) -> canisend_contracts::RequirementId {
        canisend_contracts::RequirementId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}"))
            .expect("Requirement ID")
    }

    fn plan_id(suffix: u16) -> PlanId {
        PlanId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}")).expect("Plan ID")
    }

    fn deliverable_id(suffix: u16) -> DeliverableId {
        DeliverableId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}"))
            .expect("Deliverable ID")
    }

    fn revision(value: u64) -> Revision {
        Revision::try_new(value).expect("revision")
    }

    fn timestamp(value: &str) -> UtcTimestamp {
        UtcTimestamp::try_new(value).expect("timestamp")
    }

    fn draft_snapshot(suffix: u16) -> ApplicationModelSnapshotV3 {
        let pack = pack();
        let opportunity_id = opportunity_id(suffix);
        let application_id = application_id(suffix + 1);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            item("programme-name"),
            ApplicationFieldValueV3::ShortText("Community funding".to_owned()),
        );
        ApplicationModelSnapshotV3 {
            format: ApplicationModelFormatV3::V3,
            pack: pack.clone(),
            opportunity: OpportunityRecordV3 {
                id: opportunity_id.clone(),
                pack: pack.clone(),
                title: "Local initiative funding".to_owned(),
                metadata,
                source_ids: vec![entity_id(suffix + 7)],
                created_at: timestamp("2026-08-02T12:00:00Z"),
                revision: revision(1),
                archived: false,
            },
            application: ApplicationRecordV3 {
                id: application_id,
                opportunity_id,
                pack,
                metadata: BTreeMap::new(),
                lifecycle: ApplicationLifecycleV3::Draft,
                created_at: timestamp("2026-08-02T12:00:00Z"),
                updated_at: timestamp("2026-08-02T12:00:00Z"),
                revision: revision(1),
            },
            requirements: Vec::new(),
            plan: None,
            deliverables: Vec::new(),
        }
    }

    #[test]
    fn clean_workspace_v4_holds_independent_pack_bound_applications() {
        let mut fixture = TestDatabase::new_v4("mixed-pack");
        let generic = draft_snapshot(701);
        let generic_id = generic.application.id.clone();
        let mut academic = draft_snapshot(801);
        let academic_id = academic.application.id.clone();
        let academic_pack = ApplicationPackBindingV3 {
            id: WorkflowPackId::try_new("org.canisend.academic-job").expect("pack ID"),
            version: SemanticVersion::try_new("1.0.0").expect("version"),
            content_digest: Sha256Digest::try_new("b".repeat(64)).expect("digest"),
        };
        academic.pack = academic_pack.clone();
        academic.opportunity.pack = academic_pack.clone();
        academic.application.pack = academic_pack;

        {
            let mut repository = ApplicationModelRepository::new(fixture.database());
            assert_eq!(
                repository
                    .authority()
                    .expect("v4 authority")
                    .workspace_format,
                WORKSPACE_V4_FORMAT
            );
            repository
                .create(generic.clone(), ActorKind::User, "create-generic")
                .expect("generic Application");
            repository
                .create(academic.clone(), ActorKind::User, "create-academic")
                .expect("academic Application");

            let applications = repository.list().expect("mixed Application collection");
            assert_eq!(applications.len(), 2);
            assert!(applications.iter().any(|stored| {
                stored.snapshot.application.id == generic_id
                    && stored.snapshot.pack.id.as_str() == "org.canisend.generic-application"
            }));
            assert!(applications.iter().any(|stored| {
                stored.snapshot.application.id == academic_id
                    && stored.snapshot.pack.id.as_str() == "org.canisend.academic-job"
            }));

            let mut revised_generic = generic;
            revised_generic.application.revision = revision(2);
            revised_generic.application.updated_at = timestamp("2026-08-08T10:01:00Z");
            revised_generic.application.metadata.insert(
                item("tracking-note"),
                ApplicationFieldValueV3::ShortText("independent".to_owned()),
            );
            repository
                .commit(
                    &generic_id,
                    revision(1),
                    revised_generic,
                    ActorKind::User,
                    "revise-generic",
                )
                .expect("revise generic Application");
            assert_eq!(
                repository
                    .get(&academic_id)
                    .expect("academic remains")
                    .snapshot
                    .application
                    .revision,
                revision(1)
            );
        }
        assert_eq!(
            fixture
                .database()
                .status()
                .expect("v4 status")
                .application_count,
            2
        );
    }

    fn confirmed_snapshot() -> ApplicationModelSnapshotV3 {
        let mut snapshot = draft_snapshot(601);
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

    #[test]
    fn v2_workspace_fails_closed_until_v3_authority_is_activated() {
        let mut fixture = TestDatabase::new("authority");
        let snapshot = draft_snapshot(701);
        let application_id = snapshot.application.id.clone();
        let error = ApplicationModelRepository::new(fixture.database())
            .create(snapshot, ActorKind::User, "create-generic-application")
            .expect_err("v2 authority rejects v3 write");
        assert!(matches!(error, StoreError::ApplicationModelUnavailable));
        let count: i64 = fixture
            .database()
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_model_v3_heads",
                [],
                |row| row.get(0),
            )
            .expect("head count");
        assert_eq!(count, 0);
        assert!(matches!(
            ApplicationModelRepository::new(fixture.database()).get(&application_id),
            Err(StoreError::ApplicationModelUnavailable)
        ));
    }

    #[test]
    fn exact_snapshot_history_dependencies_and_audit_are_transactional() {
        let mut fixture = TestDatabase::new("round-trip");
        activate(fixture.database());
        let snapshot = confirmed_snapshot();
        let application_id = snapshot.application.id.clone();
        assert!(matches!(
            ApplicationModelRepository::new(fixture.database()).create(
                snapshot.clone(),
                ActorKind::User,
                "private application body"
            ),
            Err(StoreError::InvalidInput(_))
        ));
        let created = ApplicationModelRepository::new(fixture.database())
            .create(
                snapshot.clone(),
                ActorKind::User,
                "create-generic-application",
            )
            .expect("create model");
        assert_eq!(created.stored.snapshot, snapshot);
        assert_eq!(
            ApplicationModelRepository::new(fixture.database())
                .get(&application_id)
                .expect("load model"),
            created.stored
        );
        let history = ApplicationModelRepository::new(fixture.database())
            .history(&application_id)
            .expect("history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].snapshot_sha256, created.stored.snapshot_sha256);
        let dependency_count: i64 = fixture
            .database()
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_model_v3_dependencies
                 WHERE application_id = ?1 AND application_revision = 1",
                [application_id.as_str()],
                |row| row.get(0),
            )
            .expect("dependency count");
        assert_eq!(dependency_count, 3);
        let audit_count: i64 = fixture
            .database()
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM audit_events
                 WHERE action IN ('workspace-v3.activate', 'application-v3.create')",
                [],
                |row| row.get(0),
            )
            .expect("audit count");
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn requirement_change_stales_exact_downstream_revisions() {
        let mut fixture = TestDatabase::new("stale");
        activate(fixture.database());
        let snapshot = confirmed_snapshot();
        let application_id = snapshot.application.id.clone();
        ApplicationModelRepository::new(fixture.database())
            .create(
                snapshot.clone(),
                ActorKind::User,
                "create-generic-application",
            )
            .expect("create model");

        let mut candidate = snapshot;
        candidate.application.revision = revision(2);
        candidate.application.updated_at = timestamp("2026-08-02T12:10:00Z");
        candidate.requirements[0].statement = "Explain the revised public benefit.".to_owned();
        candidate.requirements[0].revision = revision(2);
        let committed = ApplicationModelRepository::new(fixture.database())
            .commit(
                &application_id,
                revision(1),
                candidate,
                ActorKind::User,
                "confirm-revised-requirement",
            )
            .expect("commit revision");
        let plan = committed.stored.snapshot.plan.as_ref().expect("plan");
        assert_eq!(plan.state, PlanStateV3::Stale);
        assert_eq!(plan.revision, revision(2));
        assert_eq!(plan.requirement_inputs[0].revision, revision(1));
        assert_eq!(committed.stale_plan_ids, vec![plan.id.clone()]);
        let deliverable = &committed.stored.snapshot.deliverables[0];
        assert_eq!(deliverable.state, DeliverableStateV3::Stale);
        assert_eq!(deliverable.revision, revision(2));
        assert_eq!(deliverable.plan.revision, revision(1));
        assert_eq!(
            committed.stale_deliverable_ids,
            vec![deliverable.id.clone()]
        );
        assert_eq!(
            ApplicationModelRepository::new(fixture.database())
                .history(&application_id)
                .expect("history")
                .len(),
            2
        );
    }

    #[test]
    fn failed_update_rolls_back_revision_dependencies_and_audit() {
        let mut fixture = TestDatabase::new("rollback");
        activate(fixture.database());
        let snapshot = confirmed_snapshot();
        let application_id = snapshot.application.id.clone();
        ApplicationModelRepository::new(fixture.database())
            .create(
                snapshot.clone(),
                ActorKind::User,
                "create-generic-application",
            )
            .expect("create model");
        let mut invalid = snapshot;
        invalid.application.revision = revision(2);
        invalid.application.updated_at = timestamp("2026-08-02T12:10:00Z");
        invalid.requirements.clear();
        assert!(matches!(
            ApplicationModelRepository::new(fixture.database()).commit(
                &application_id,
                revision(1),
                invalid,
                ActorKind::User,
                "invalid-deletion"
            ),
            Err(StoreError::ApplicationModelConflict(_))
        ));
        let current = ApplicationModelRepository::new(fixture.database())
            .get(&application_id)
            .expect("current model")
            .snapshot;
        let mut forged_stale = current;
        forged_stale.application.revision = revision(2);
        forged_stale.application.updated_at = timestamp("2026-08-02T12:11:00Z");
        forged_stale.plan.as_mut().expect("plan").state = PlanStateV3::Stale;
        forged_stale.plan.as_mut().expect("plan").revision = revision(2);
        assert!(matches!(
            ApplicationModelRepository::new(fixture.database()).commit(
                &application_id,
                revision(1),
                forged_stale,
                ActorKind::User,
                "forge-stale-state"
            ),
            Err(StoreError::ApplicationModelConflict(_))
        ));
        assert_eq!(
            ApplicationModelRepository::new(fixture.database())
                .history(&application_id)
                .expect("history")
                .len(),
            1
        );
        let commit_audits: i64 = fixture
            .database()
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE action = 'application-v3.commit'",
                [],
                |row| row.get(0),
            )
            .expect("commit audit count");
        assert_eq!(commit_audits, 0);
    }

    #[test]
    fn concurrent_writers_cannot_commit_the_same_expected_revision_twice() {
        let mut fixture = TestDatabase::new("concurrent");
        activate(fixture.database());
        let snapshot = draft_snapshot(801);
        let application_id = snapshot.application.id.clone();
        ApplicationModelRepository::new(fixture.database())
            .create(
                snapshot.clone(),
                ActorKind::User,
                "create-generic-application",
            )
            .expect("create model");
        let path = fixture.path.clone();
        fixture.close();
        let barrier = Arc::new(Barrier::new(2));
        let handles = ["Writer A", "Writer B"].map(|title| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            let application_id = application_id.clone();
            let mut candidate = snapshot.clone();
            candidate.application.revision = revision(2);
            candidate.application.updated_at = if title == "Writer A" {
                timestamp("2026-08-02T12:10:00Z")
            } else {
                timestamp("2026-08-02T12:11:00Z")
            };
            candidate.opportunity.title = title.to_owned();
            candidate.opportunity.revision = revision(2);
            thread::spawn(move || {
                let mut database = Database::open(&path).expect("thread database");
                barrier.wait();
                ApplicationModelRepository::new(&mut database).commit(
                    &application_id,
                    revision(1),
                    candidate,
                    ActorKind::User,
                    "concurrent-update",
                )
            })
        });
        let results = handles.map(|handle| handle.join().expect("writer thread"));
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(StoreError::ApplicationModelConflict(_))))
                .count(),
            1
        );
        let mut database = Database::open(&path).expect("final database");
        assert_eq!(
            ApplicationModelRepository::new(&mut database)
                .history(&application_id)
                .expect("history")
                .len(),
            2
        );
    }
}
