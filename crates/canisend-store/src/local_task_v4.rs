use canisend_contracts::{
    AgentApplicationBindingV4, AgentPackBindingV4, ApplicationId, EntityId, LocalTaskStateV4,
    LocalTaskV4, Revision, Sha256Digest, UtcTimestamp,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{BlobStore, Database, StoreError, application_v3::load_current, generate_id};

pub const LOCAL_TASK_OPERATION_V4: &str = "local.deliverable.draft";
pub const LOCAL_TASK_CANDIDATE_PURPOSE_V4: &str = "deliverable.draft.preview";
const MAX_CANDIDATE_BYTES: usize = 256 * 1024;

pub struct LocalTaskServiceV4<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> LocalTaskServiceV4<'a> {
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn prepare(
        &mut self,
        application_id: &ApplicationId,
        expected_revision: Revision,
    ) -> Result<LocalTaskV4, StoreError> {
        let transaction = self.database.immediate_transaction()?;
        let (application, input_sha256) = inputs(&transaction, application_id)?;
        if application.expected_revision != expected_revision {
            return Err(StoreError::TaskStale(
                "Application revision changed".to_owned(),
            ));
        }
        let task = LocalTaskV4 {
            id: generate_id()?,
            application,
            input_sha256,
            generation: 1,
            state: LocalTaskStateV4::Prepared,
            lease_id: None,
            lease_expires_at: None,
            candidate_sha256: None,
            candidate_bytes: None,
        };
        transaction.execute(
            "INSERT INTO tasks(id, status, created_at, operation, descriptor_json, actor, execution_mode)
             VALUES (?1, 'prepared', ?2, ?3, ?4, 'host-agent', 'host-agent')",
            params![task.id.as_str(), timestamp(OffsetDateTime::now_utc())?.as_str(),
                LOCAL_TASK_OPERATION_V4, serde_json::to_string(&task)?],
        )?;
        audit(&transaction, &task, "local.task.prepare")?;
        transaction.commit()?;
        Ok(task)
    }

    pub fn show(&self, id: &EntityId) -> Result<LocalTaskV4, StoreError> {
        load(self.database.connection(), id)
    }

    /// Most recent 100 descriptors for canonical resumption; candidate bodies are excluded.
    pub fn list(&self, application_id: &ApplicationId) -> Result<Vec<LocalTaskV4>, StoreError> {
        load_current(self.database.connection(), application_id)?;
        let mut statement = self.database.connection().prepare(
            "SELECT id FROM tasks WHERE operation=?2 AND job_id IS NULL AND stage_execution_id IS NULL
             AND json_extract(CASE WHEN operation=?2 AND job_id IS NULL AND stage_execution_id IS NULL
                 THEN descriptor_json ELSE '{}' END, '$.application.id')=?1
             ORDER BY created_at DESC,id DESC LIMIT 100",
        )?;
        let ids = statement
            .query_map(
                params![application_id.as_str(), LOCAL_TASK_OPERATION_V4],
                |row| row.get::<_, String>(0),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| self.show(&EntityId::try_new(id)?))
            .collect()
    }

    pub fn claim(&mut self, id: &EntityId, generation: u64) -> Result<LocalTaskV4, StoreError> {
        self.claim_at(id, generation, OffsetDateTime::now_utc())
    }

    fn claim_at(
        &mut self,
        id: &EntityId,
        generation: u64,
        now: OffsetDateTime,
    ) -> Result<LocalTaskV4, StoreError> {
        let transaction = self.database.immediate_transaction()?;
        let mut task = load(&transaction, id)?;
        require_generation(&task, generation)?;
        require_current(&transaction, &task)?;
        if task.state != LocalTaskStateV4::Prepared
            && !(task.state == LocalTaskStateV4::Claimed && !lease_live(&task, now)?)
        {
            return Err(StoreError::TaskConflict(
                "Task is already claimed or terminal".to_owned(),
            ));
        }
        task.generation = next_generation(task.generation)?;
        task.state = LocalTaskStateV4::Claimed;
        task.lease_id = Some(generate_id()?);
        task.lease_expires_at = Some(timestamp(now + Duration::minutes(15))?);
        save(&transaction, &task, "local.task.claim")?;
        transaction.commit()?;
        Ok(task)
    }

    pub fn submit(
        &mut self,
        id: &EntityId,
        generation: u64,
        lease_id: &EntityId,
        candidate: &Value,
    ) -> Result<LocalTaskV4, StoreError> {
        let bytes = candidate_bytes(candidate)?;
        let transaction = self.database.immediate_transaction()?;
        let mut task = load(&transaction, id)?;
        require_generation(&task, generation)?;
        require_current(&transaction, &task)?;
        require_lease(&task, lease_id, OffsetDateTime::now_utc())?;
        if task.state != LocalTaskStateV4::Claimed {
            return Err(StoreError::TaskConflict(
                "Only a claimed task accepts a candidate".to_owned(),
            ));
        }
        task.generation = next_generation(task.generation)?;
        task.state = LocalTaskStateV4::Submitted;
        let digest = self.blobs.put_bytes(&bytes)?;
        task.candidate_sha256 = Some(digest.clone());
        task.candidate_bytes = Some(bytes.len() as u64);
        transaction.execute(
            "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
             VALUES (?1, 'task-candidate', ?2, ?3, ?4)",
            params![
                digest.as_str(),
                task.id.as_str(),
                to_i64(task.generation)?,
                timestamp(OffsetDateTime::now_utc())?.as_str()
            ],
        )?;
        save(&transaction, &task, "local.task.submit")?;
        transaction.commit()?;
        Ok(task)
    }

    pub fn cancel(
        &mut self,
        id: &EntityId,
        generation: u64,
        lease_id: &EntityId,
    ) -> Result<LocalTaskV4, StoreError> {
        let transaction = self.database.immediate_transaction()?;
        let mut task = load(&transaction, id)?;
        require_generation(&task, generation)?;
        require_lease(&task, lease_id, OffsetDateTime::now_utc())?;
        if !matches!(
            task.state,
            LocalTaskStateV4::Claimed | LocalTaskStateV4::Submitted
        ) {
            return Err(StoreError::TaskConflict(
                "Task is not cancellable".to_owned(),
            ));
        }
        task.generation = next_generation(task.generation)?;
        task.state = LocalTaskStateV4::Cancelled;
        // Retain any submitted candidate and its blob reference for inspection/recovery.
        save(&transaction, &task, "local.task.cancel")?;
        transaction.commit()?;
        Ok(task)
    }

    /// Private content access: the application facade must obtain explicit private-read consent.
    pub fn candidate(&self, id: &EntityId) -> Result<Value, StoreError> {
        let task = load(self.database.connection(), id)?;
        let digest = task
            .candidate_sha256
            .ok_or_else(|| StoreError::TaskConflict("Task has no candidate".to_owned()))?;
        let bytes = self
            .blobs
            .read_verified(&digest, MAX_CANDIDATE_BYTES as u64)?;
        if Some(bytes.len() as u64) != task.candidate_bytes {
            return Err(StoreError::TaskConflict(
                "Candidate size differs from its descriptor".to_owned(),
            ));
        }
        let value: Value = serde_json::from_slice(&bytes)?;
        if candidate_bytes(&value)? != bytes {
            return Err(StoreError::TaskConflict(
                "Candidate is not canonical JSON".to_owned(),
            ));
        }
        Ok(value)
    }
}

fn inputs(
    connection: &Connection,
    id: &ApplicationId,
) -> Result<(AgentApplicationBindingV4, Sha256Digest), StoreError> {
    let format: String = connection.query_row(
        "SELECT workspace_format FROM workspace_metadata WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    if format != "canisend.workspace/v4" {
        return Err(StoreError::TaskConflict(
            "Local tasks require Workspace v4".to_owned(),
        ));
    }
    let stored = load_current(connection, id)?;
    if stored.snapshot.application.lifecycle == canisend_contracts::ApplicationLifecycleV3::Archived
    {
        return Err(StoreError::TaskConflict(
            "Archived Applications cannot start or advance local work".to_owned(),
        ));
    }
    let pack = stored.snapshot.pack;
    let application = AgentApplicationBindingV4 {
        id: id.clone(),
        pack: AgentPackBindingV4 {
            id: pack.id,
            version: pack.version,
            content_digest: pack.content_digest,
        },
        expected_revision: stored.snapshot.application.revision,
        snapshot_sha256: stored.snapshot_sha256,
    };
    let mut ancillary = Vec::new();
    // Same scoped association/head inputs as ApplicationAssociationServiceV4, read within
    // the caller's immediate transaction so claims cannot race an association change.
    for query in [
        "SELECT json_array(workspace_id, profile_revision) FROM workspace_metadata WHERE singleton = 1 AND ?1 IS NOT NULL",
        "SELECT json_array(a.source_id,a.source_revision,a.source_sha256,a.consent_scope,a.associated_at,h.head_revision)
         FROM application_source_v4_associations a LEFT JOIN workspace_source_v4_heads h ON h.source_id=a.source_id
         WHERE a.application_id=?1 ORDER BY a.source_id",
        "SELECT json_array(a.profile_source_id,a.profile_source_revision,a.profile_source_sha256,a.consent_scope,a.associated_at,r.revision,r.sha256,r.sensitivity)
         FROM application_profile_v4_associations a LEFT JOIN profile_source_revisions r ON r.source_id=a.profile_source_id
          AND r.revision=(SELECT MAX(revision) FROM profile_source_revisions WHERE source_id=a.profile_source_id)
         WHERE a.application_id=?1 ORDER BY a.profile_source_id",
        "SELECT json_array(a.evidence_id,a.evidence_revision,a.evidence_sha256,a.consent_scope,a.associated_at,r.revision,r.sha256,r.confirmed,r.excluded,r.sensitivity)
         FROM application_evidence_v4_associations a LEFT JOIN evidence_revisions r ON r.evidence_id=a.evidence_id
          AND r.revision=(SELECT MAX(revision) FROM evidence_revisions WHERE evidence_id=a.evidence_id)
         WHERE a.application_id=?1 ORDER BY a.evidence_id",
    ] {
        let mut statement = connection.prepare(query)?;
        let rows = statement.query_map([id.as_str()], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ancillary.push(rows);
    }
    let digest =
        Sha256Digest::try_new(hex::encode(Sha256::digest(serde_json::to_vec(&ancillary)?)))?;
    Ok((application, digest))
}

fn require_current(connection: &Connection, task: &LocalTaskV4) -> Result<(), StoreError> {
    let (application, input) = inputs(connection, &task.application.id)?;
    if task.application != application || task.input_sha256 != input {
        return Err(StoreError::TaskStale(
            "Application or associated inputs changed".to_owned(),
        ));
    }
    Ok(())
}

fn load(connection: &Connection, id: &EntityId) -> Result<LocalTaskV4, StoreError> {
    type TaskRow = (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let row: Option<TaskRow> = connection.query_row(
        "SELECT substr(descriptor_json,1,16385),status,lease_id,lease_expires_at,candidate_sha256
         FROM tasks WHERE id=?1 AND operation=?2 AND job_id IS NULL AND stage_execution_id IS NULL",
        params![id.as_str(), LOCAL_TASK_OPERATION_V4],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
    ).optional()?;
    let (json, status, lease_id, lease_expires_at, candidate_sha256) =
        row.ok_or_else(|| StoreError::TaskNotFound(id.to_string()))?;
    if json.len() > 16 * 1024 {
        return Err(StoreError::TaskConflict(
            "Local task descriptor exceeds its bound".to_owned(),
        ));
    }
    let task: LocalTaskV4 = serde_json::from_str(&json)?;
    if task.id != *id || task.generation == 0 || task.generation > i64::MAX as u64 {
        return Err(StoreError::TaskConflict(
            "Invalid local task descriptor identity".to_owned(),
        ));
    }
    let lease_present = task.lease_id.is_some() && task.lease_expires_at.is_some();
    let candidate_present = task.candidate_sha256.is_some()
        && task
            .candidate_bytes
            .is_some_and(|bytes| (2..=MAX_CANDIDATE_BYTES as u64).contains(&bytes));
    let coherent = match task.state {
        LocalTaskStateV4::Prepared => {
            task.generation == 1
                && !lease_present
                && task.lease_id.is_none()
                && task.lease_expires_at.is_none()
                && !candidate_present
        }
        LocalTaskStateV4::Claimed => task.generation >= 2 && lease_present && !candidate_present,
        LocalTaskStateV4::Submitted => task.generation >= 3 && lease_present && candidate_present,
        LocalTaskStateV4::Cancelled => task.generation >= 3 && lease_present,
    };
    if !coherent
        || task.candidate_sha256.is_some() != task.candidate_bytes.is_some()
        || (task.candidate_sha256.is_some() && !candidate_present)
        || matches!(
            task.state,
            LocalTaskStateV4::Prepared | LocalTaskStateV4::Claimed
        ) && task.candidate_sha256.is_some()
        || serde_json::to_value(task.state)?.as_str() != Some(status.as_str())
        || task.lease_id.as_ref().map(EntityId::as_str) != lease_id.as_deref()
        || task.lease_expires_at.as_ref().map(UtcTimestamp::as_str) != lease_expires_at.as_deref()
        || task.candidate_sha256.as_ref().map(Sha256Digest::as_str) != candidate_sha256.as_deref()
    {
        return Err(StoreError::TaskConflict(
            "Local task row and descriptor are inconsistent".to_owned(),
        ));
    }
    Ok(task)
}

fn save(transaction: &Transaction<'_>, task: &LocalTaskV4, action: &str) -> Result<(), StoreError> {
    transaction.execute(
        "UPDATE tasks SET descriptor_json=?2,status=?3,lease_id=?4,lease_expires_at=?5,candidate_sha256=?6 WHERE id=?1 AND operation=?7",
        params![task.id.as_str(), serde_json::to_string(task)?, serde_json::to_value(task.state)?.as_str(),
            task.lease_id.as_ref().map(EntityId::as_str), task.lease_expires_at.as_ref().map(UtcTimestamp::as_str),
            task.candidate_sha256.as_ref().map(Sha256Digest::as_str), LOCAL_TASK_OPERATION_V4],
    )?;
    audit(transaction, task, action)
}

fn audit(
    transaction: &Transaction<'_>,
    task: &LocalTaskV4,
    action: &str,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(id,actor,action,subject_id,subject_revision,reason,created_at)
         VALUES (?1,'host-agent',?2,?3,?4,'local coordination only; no business approval',?5)",
        params![
            generate_id()?.as_str(),
            action,
            task.id.as_str(),
            to_i64(task.generation)?,
            timestamp(OffsetDateTime::now_utc())?.as_str()
        ],
    )?;
    Ok(())
}

fn require_generation(task: &LocalTaskV4, generation: u64) -> Result<(), StoreError> {
    if task.generation != generation {
        return Err(StoreError::TaskConflict(
            "Task generation changed".to_owned(),
        ));
    }
    Ok(())
}
fn next_generation(value: u64) -> Result<u64, StoreError> {
    value
        .checked_add(1)
        .filter(|value| *value <= i64::MAX as u64)
        .ok_or_else(|| StoreError::TaskConflict("Task generation overflow".to_owned()))
}
fn require_lease(
    task: &LocalTaskV4,
    lease: &EntityId,
    now: OffsetDateTime,
) -> Result<(), StoreError> {
    if task.lease_id.as_ref() != Some(lease) || !lease_live(task, now)? {
        return Err(StoreError::TaskConflict(
            "Task lease is missing, expired or belongs to another worker".to_owned(),
        ));
    }
    Ok(())
}
fn lease_live(task: &LocalTaskV4, now: OffsetDateTime) -> Result<bool, StoreError> {
    task.lease_expires_at
        .as_ref()
        .map(|value| {
            OffsetDateTime::parse(value.as_str(), &Rfc3339)
                .map(|expiry| now < expiry)
                .map_err(|error| StoreError::Invariant(error.to_string()))
        })
        .transpose()
        .map(|live| live.unwrap_or(false))
}
fn timestamp(now: OffsetDateTime) -> Result<UtcTimestamp, StoreError> {
    UtcTimestamp::try_new(
        now.format(&Rfc3339)
            .map_err(|error| StoreError::Invariant(error.to_string()))?,
    )
    .map_err(StoreError::from)
}
fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value)
        .map_err(|_| StoreError::TaskConflict("Task generation overflow".to_owned()))
}
fn candidate_bytes(candidate: &Value) -> Result<Vec<u8>, StoreError> {
    if !candidate.is_object() {
        return Err(StoreError::InvalidInput(
            "Local candidate must be a JSON object".to_owned(),
        ));
    }
    let mut canonical = candidate.clone();
    canonical.sort_all_objects();
    let bytes = serde_json::to_vec(&canonical)?;
    if bytes.len() > MAX_CANDIDATE_BYTES {
        return Err(StoreError::InvalidInput(
            "Local candidate exceeds 256 KiB".to_owned(),
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ApplicationFlowCreateRequestV3, ApplicationFlowRequirementDraftV3,
        ApplicationFlowServiceV3, ApplicationModelRepository, NewProfileSource, ProfileService,
        Workspace,
    };
    use canisend_contracts::{
        ActorKind, PrivacyClassification, ProfileSourceKind, RequirementPriorityV3,
        WorkflowPackItemId,
    };
    use canisend_core::{
        WorkflowPackByteLoader, WorkflowPackCapabilityRegistry, WorkflowPackOrigin,
        WorkflowPackRuntime,
    };
    use canisend_resources::generic_application_workflow_pack;
    use serde_json::json;

    #[test]
    fn leases_serialize_workers_reject_stale_inputs_and_retain_candidates_without_application_writes()
     {
        let root =
            std::env::temp_dir().join(format!("canisend-local-task-{}", generate_id().unwrap()));
        let mut first = Workspace::init_v4(&root).unwrap();
        let embedded = generic_application_workflow_pack();
        let pack = WorkflowPackByteLoader::verify(
            embedded.manifest_bytes(),
            embedded.into_resources(),
            WorkflowPackOrigin::BuiltIn,
            &WorkflowPackRuntime::parse(
                env!("CARGO_PKG_VERSION"),
                "3.0.0-alpha.1",
                "3.0.0-alpha.1",
            )
            .unwrap(),
            &WorkflowPackCapabilityRegistry::built_in(),
        )
        .unwrap()
        .into_bundle();
        let app = ApplicationFlowServiceV3::new(&mut first.database, &first.blobs, &root)
            .create(
                &pack,
                ApplicationFlowCreateRequestV3 {
                    title: "Synthetic local task".to_owned(),
                    opportunity_metadata: Default::default(),
                    application_metadata: Default::default(),
                    source_text: "Provide a narrative.".to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: WorkflowPackItemId::try_new("format").unwrap(),
                        statement: "Provide a narrative.".to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: 20,
                    }],
                },
            )
            .unwrap()
            .stored;
        let app_id = app.snapshot.application.id.clone();
        drop(first);
        let mut first = Workspace::open_v4(Some(&root)).unwrap();
        let mut second = Workspace::open_v4(Some(&root)).unwrap();
        let task = LocalTaskServiceV4::new(&mut first.database, &first.blobs)
            .prepare(&app_id, Revision::try_new(1).unwrap())
            .unwrap();
        let stale_context_task = LocalTaskServiceV4::new(&mut first.database, &first.blobs)
            .prepare(&app_id, Revision::try_new(1).unwrap())
            .unwrap();
        let claim = LocalTaskServiceV4::new(&mut first.database, &first.blobs)
            .claim(&task.id, task.generation)
            .unwrap();
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .claim(&task.id, task.generation)
                .is_err()
        );
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .claim(&task.id, claim.generation)
                .is_err()
        );
        let candidate = json!({"deliverables": [{"kind": "primary-document", "content": "Synthetic candidate only."}]});
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .submit(
                    &task.id,
                    claim.generation,
                    &generate_id().unwrap(),
                    &candidate
                )
                .is_err()
        );
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .submit(
                    &task.id,
                    task.generation,
                    claim.lease_id.as_ref().unwrap(),
                    &candidate
                )
                .is_err()
        );
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .submit(
                    &task.id,
                    claim.generation,
                    claim.lease_id.as_ref().unwrap(),
                    &json!({"content": "x".repeat(MAX_CANDIDATE_BYTES)})
                )
                .is_err()
        );
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .submit(
                    &task.id,
                    claim.generation,
                    claim.lease_id.as_ref().unwrap(),
                    &json!([])
                )
                .is_err()
        );
        let submitted = LocalTaskServiceV4::new(&mut first.database, &first.blobs)
            .submit(
                &task.id,
                claim.generation,
                claim.lease_id.as_ref().unwrap(),
                &candidate,
            )
            .unwrap();
        assert_eq!(submitted.state, LocalTaskStateV4::Submitted);
        assert_eq!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .candidate(&task.id)
                .unwrap(),
            candidate
        );
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .submit(
                    &task.id,
                    submitted.generation,
                    claim.lease_id.as_ref().unwrap(),
                    &candidate
                )
                .is_err()
        );
        let cancelled = LocalTaskServiceV4::new(&mut second.database, &second.blobs)
            .cancel(
                &task.id,
                submitted.generation,
                claim.lease_id.as_ref().unwrap(),
            )
            .unwrap();
        assert_eq!(cancelled.state, LocalTaskStateV4::Cancelled);
        assert_eq!(
            LocalTaskServiceV4::new(&mut first.database, &first.blobs)
                .candidate(&task.id)
                .unwrap(),
            candidate
        );
        assert_eq!(cancelled.candidate_sha256, submitted.candidate_sha256);
        assert_eq!(
            ApplicationModelRepository::new(&mut first.database)
                .get(&app_id)
                .unwrap(),
            app
        );

        let expired = LocalTaskServiceV4::new(&mut first.database, &first.blobs)
            .prepare(&app_id, Revision::try_new(1).unwrap())
            .unwrap();
        let old = LocalTaskServiceV4::new(&mut first.database, &first.blobs)
            .claim_at(
                &expired.id,
                expired.generation,
                OffsetDateTime::now_utc() - Duration::minutes(16),
            )
            .unwrap();
        assert!(
            LocalTaskServiceV4::new(&mut first.database, &first.blobs)
                .submit(
                    &old.id,
                    old.generation,
                    old.lease_id.as_ref().unwrap(),
                    &candidate
                )
                .is_err()
        );
        let reclaimed = LocalTaskServiceV4::new(&mut second.database, &second.blobs)
            .claim(&old.id, old.generation)
            .unwrap();
        assert_ne!(reclaimed.lease_id, old.lease_id);
        assert!(
            LocalTaskServiceV4::new(&mut first.database, &first.blobs)
                .submit(
                    &old.id,
                    reclaimed.generation,
                    old.lease_id.as_ref().unwrap(),
                    &candidate
                )
                .is_err()
        );
        ProfileService::new(&mut first.database, &first.blobs)
            .import_source(
                NewProfileSource {
                    kind: ProfileSourceKind::PlainText,
                    original_bytes: b"Synthetic input change".to_vec(),
                    normalized_text: "Synthetic input change".to_owned(),
                    content_type: "text/plain".to_owned(),
                    sensitivity: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .unwrap();
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .claim(&stale_context_task.id, stale_context_task.generation)
                .is_err()
        );
        assert!(
            LocalTaskServiceV4::new(&mut second.database, &second.blobs)
                .submit(
                    &reclaimed.id,
                    reclaimed.generation,
                    reclaimed.lease_id.as_ref().unwrap(),
                    &candidate
                )
                .is_err()
        );
        // Cancellation remains possible for stale work and never changes the Application.
        LocalTaskServiceV4::new(&mut second.database, &second.blobs)
            .cancel(
                &reclaimed.id,
                reclaimed.generation,
                reclaimed.lease_id.as_ref().unwrap(),
            )
            .unwrap();
        assert_eq!(
            ApplicationModelRepository::new(&mut first.database)
                .get(&app_id)
                .unwrap(),
            app
        );
        assert!(first.check().unwrap().ok);
        drop(first);
        drop(second);
        let mut reopened = Workspace::open_v4(Some(&root)).unwrap();
        assert_eq!(
            LocalTaskServiceV4::new(&mut reopened.database, &reopened.blobs)
                .candidate(&task.id)
                .unwrap(),
            candidate
        );
        let resumed = LocalTaskServiceV4::new(&mut reopened.database, &reopened.blobs)
            .list(&app_id)
            .unwrap();
        assert!(
            resumed
                .iter()
                .any(|entry| entry.id == task.id && entry.state == LocalTaskStateV4::Cancelled)
        );
        let archive_task = LocalTaskServiceV4::new(&mut reopened.database, &reopened.blobs)
            .prepare(&app_id, Revision::try_new(1).unwrap())
            .unwrap();
        ApplicationModelRepository::new(&mut reopened.database)
            .archive(
                &app_id,
                Revision::try_new(1).unwrap(),
                ActorKind::User,
                "synthetic-archive",
            )
            .unwrap();
        assert!(
            LocalTaskServiceV4::new(&mut reopened.database, &reopened.blobs)
                .claim(&archive_task.id, archive_task.generation)
                .is_err()
        );
        assert!(
            LocalTaskServiceV4::new(&mut reopened.database, &reopened.blobs)
                .prepare(&app_id, Revision::try_new(2).unwrap())
                .is_err()
        );
        // Verified reads reject row/descriptor disagreement as well as a changed digest.
        reopened
            .database
            .connection()
            .execute(
                "UPDATE tasks SET status='claimed' WHERE id=?1",
                [task.id.as_str()],
            )
            .unwrap();
        assert!(
            LocalTaskServiceV4::new(&mut reopened.database, &reopened.blobs)
                .show(&task.id)
                .is_err()
        );
        reopened
            .database
            .connection()
            .execute(
                "UPDATE tasks SET status='cancelled' WHERE id=?1",
                [task.id.as_str()],
            )
            .unwrap();
        // Verified blob reads reject a descriptor binding to a different digest.
        let mut corrupt = cancelled;
        corrupt.candidate_sha256 = Some(Sha256Digest::try_new("f".repeat(64)).unwrap());
        reopened
            .database
            .connection()
            .execute(
                "UPDATE tasks SET descriptor_json=?2 WHERE id=?1",
                params![task.id.as_str(), serde_json::to_string(&corrupt).unwrap()],
            )
            .unwrap();
        assert!(
            LocalTaskServiceV4::new(&mut reopened.database, &reopened.blobs)
                .candidate(&task.id)
                .is_err()
        );
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }
}
