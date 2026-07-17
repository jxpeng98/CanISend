use std::collections::BTreeMap;

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, CandidateValidationError, ConsentRequest,
    ConsentScope, ContractViolation, CriterionRecord, EntityId, ExecutionMode,
    PUBLIC_SCHEMA_VERSION, PublicSchemaId, Revision, SchemaReference, SemanticVersion,
    Sha256Digest, TaskCommitData, TaskCompletionRequest, TaskDescriptor, TaskLease, TaskStateData,
    TaskStatus, UtcTimestamp, validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::{Map, Value};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, now_utc};

pub const JOB_CRITERION_OPERATION: &str = "job.criterion.extract";
const TASK_LEASE_MINUTES: i64 = 15;

pub struct TaskService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> TaskService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn prepare_job_criterion(
        &mut self,
        job_id: &EntityId,
    ) -> Result<TaskDescriptor, StoreError> {
        let task_id = generate_id()?;
        let lease_id = generate_id()?;
        let event_id = generate_id()?;
        let created_at = now_utc()?;
        let expires_at = timestamp_after_minutes(TASK_LEASE_MINUTES)?;
        let transaction = self.database.immediate_transaction()?;
        let (archived, job_revision): (i64, i64) = transaction
            .query_row(
                "SELECT archived, revision FROM jobs WHERE id = ?1",
                params![job_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::JobNotFound(job_id.to_string()))?;
        if archived != 0 {
            return Err(StoreError::JobArchived(job_id.to_string()));
        }
        let inputs = load_normalized_inputs(&transaction, job_id)?;
        if inputs.is_empty() {
            return Err(StoreError::InvalidInput(
                "job must have at least one normalized source before preparing a task".to_owned(),
            ));
        }
        let descriptor = TaskDescriptor {
            id: task_id.clone(),
            operation: JOB_CRITERION_OPERATION.to_owned(),
            job_id: job_id.clone(),
            job_revision: Revision::try_new(to_u64(job_revision)?)?,
            actor: ActorKind::HostAgent,
            execution_mode: ExecutionMode::HostAgent,
            input_artifacts: inputs.clone(),
            allowed_output_kind: ArtifactKind::Criteria,
            candidate_schema: SchemaReference {
                id: PublicSchemaId::Criterion.as_str().to_owned(),
                version: SemanticVersion::try_new(PUBLIC_SCHEMA_VERSION)?,
            },
            required_consents: vec![ConsentRequest {
                scope: ConsentScope::ReadPrivateInputs,
                description: "Read only the normalized advert artifacts declared by this task"
                    .to_owned(),
                artifacts: inputs.clone(),
            }],
            private_read_scope: inputs.clone(),
            lease: TaskLease {
                id: lease_id.clone(),
                expires_at: expires_at.clone(),
            },
        };
        let descriptor_json = serde_json::to_string(&descriptor)?;
        transaction.execute(
            "INSERT INTO tasks(
                id, stage_execution_id, status, lease_expires_at, created_at,
                job_id, job_revision, operation, actor, execution_mode,
                allowed_output_kind, candidate_schema_id, candidate_schema_version,
                lease_id, descriptor_json
             ) VALUES (?1, NULL, 'prepared', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                task_id.as_str(),
                expires_at.as_str(),
                created_at.as_str(),
                job_id.as_str(),
                job_revision,
                descriptor.operation,
                enum_name(descriptor.actor)?,
                enum_name(descriptor.execution_mode)?,
                enum_name(descriptor.allowed_output_kind)?,
                descriptor.candidate_schema.id,
                descriptor.candidate_schema.version.as_str(),
                lease_id.as_str(),
                descriptor_json
            ],
        )?;
        for input in &inputs {
            transaction.execute(
                "INSERT INTO task_inputs(task_id, artifact_id, revision, sha256)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    task_id.as_str(),
                    input.id.as_str(),
                    to_i64(input.revision.get())?,
                    input.sha256.as_str()
                ],
            )?;
        }
        insert_audit(
            &transaction,
            &event_id,
            "task.prepare",
            &task_id,
            None,
            "prepare host-agent task with immutable input revisions",
            &created_at,
        )?;
        transaction.commit()?;
        Ok(descriptor)
    }

    pub fn get(&self, task_id: &EntityId) -> Result<TaskStateData, StoreError> {
        load_task_state(self.database.connection(), task_id)
    }

    pub fn cancel(&mut self, task_id: &EntityId) -> Result<TaskStateData, StoreError> {
        let event_id = generate_id()?;
        let cancelled_at = now_utc()?;
        let transaction = self.database.immediate_transaction()?;
        let status: String = transaction
            .query_row(
                "SELECT status FROM tasks WHERE id = ?1",
                params![task_id.as_str()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| StoreError::TaskNotFound(task_id.to_string()))?;
        match status.as_str() {
            "prepared" => {
                transaction.execute(
                    "UPDATE tasks SET status = 'cancelled', cancelled_at = ?2 WHERE id = ?1",
                    params![task_id.as_str(), cancelled_at.as_str()],
                )?;
                insert_audit(
                    &transaction,
                    &event_id,
                    "task.cancel",
                    task_id,
                    None,
                    "cancel prepared task",
                    &cancelled_at,
                )?;
            }
            "cancelled" => {}
            other => {
                return Err(StoreError::TaskConflict(format!(
                    "cannot cancel task {task_id} in {other} state"
                )));
            }
        }
        transaction.commit()?;
        self.get(task_id)
    }

    pub fn complete(
        &mut self,
        request: &TaskCompletionRequest,
    ) -> Result<TaskCommitData, StoreError> {
        let initial = self.get(&request.task_id)?;
        validate_request_shape(&initial.descriptor, request)?;
        validate_candidate(&initial.descriptor, &request.candidate)?;
        let bytes = canonical_json_bytes(&request.candidate)?;
        let digest = self.blobs.put_bytes(&bytes)?;
        let size = self.blobs.verify(&digest, DEFAULT_MAX_BLOB_BYTES)?;
        let transaction = self.database.immediate_transaction()?;
        let row: (String, String, String, i64) = transaction
            .query_row(
                "SELECT status, lease_id, lease_expires_at, job_revision
                 FROM tasks WHERE id = ?1",
                params![request.task_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::TaskNotFound(request.task_id.to_string()))?;
        let (status, lease_id, lease_expires_at, prepared_job_revision) = row;
        if status == "committed" {
            let (artifact, committed_at) = load_task_result(&transaction, &request.task_id)?
                .ok_or_else(|| {
                    StoreError::Invariant("committed task has no result artifact".to_owned())
                })?;
            if artifact.sha256 != digest {
                return Err(StoreError::TaskConflict(format!(
                    "task {} was already completed with a different candidate",
                    request.task_id
                )));
            }
            transaction.commit()?;
            return Ok(TaskCommitData {
                task_id: request.task_id.clone(),
                status: TaskStatus::Committed,
                artifact,
                committed_at,
                idempotent: true,
            });
        }
        if status != "prepared" {
            return Err(StoreError::TaskConflict(format!(
                "task {} is in {status} state",
                request.task_id
            )));
        }
        if lease_id != request.lease_id.as_str() {
            return Err(StoreError::TaskConflict(format!(
                "lease does not belong to task {}",
                request.task_id
            )));
        }
        let current_job_revision: i64 = transaction
            .query_row(
                "SELECT revision FROM jobs WHERE id = ?1",
                params![initial.descriptor.job_id.as_str()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| StoreError::TaskStale("job no longer exists".to_owned()))?;
        let expired = timestamp_is_past(&lease_expires_at)?;
        let stale = expired
            || current_job_revision != prepared_job_revision
            || current_job_revision != to_i64(request.expected_job_revision.get())?
            || !inputs_are_current(&transaction, request)?;
        if stale {
            let stale_at = now_utc()?;
            let event_id = generate_id()?;
            transaction.execute(
                "UPDATE tasks SET status = 'stale' WHERE id = ?1 AND status = 'prepared'",
                params![request.task_id.as_str()],
            )?;
            insert_audit(
                &transaction,
                &event_id,
                "task.stale",
                &request.task_id,
                None,
                if expired {
                    "task lease expired before completion"
                } else {
                    "task input or job revision changed before completion"
                },
                &stale_at,
            )?;
            transaction.commit()?;
            return Err(StoreError::TaskStale(format!(
                "task {} must be prepared again",
                request.task_id
            )));
        }

        let artifact_id = generate_id()?;
        let event_id = generate_id()?;
        let committed_at = now_utc()?;
        transaction.execute(
            "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
             VALUES (?1, ?2, 1, 0, ?3)",
            params![
                artifact_id.as_str(),
                enum_name(initial.descriptor.allowed_output_kind)?,
                committed_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO artifact_revisions(
                artifact_id, revision, sha256, size, actor, reason, created_at
             ) VALUES (?1, 1, ?2, ?3, 'host-agent', 'complete declared task', ?4)",
            params![
                artifact_id.as_str(),
                digest.as_str(),
                to_i64(size)?,
                committed_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
             VALUES (?1, 'artifact', ?2, 1, ?3)",
            params![digest.as_str(), artifact_id.as_str(), committed_at.as_str()],
        )?;
        for input in &initial.descriptor.input_artifacts {
            transaction.execute(
                "INSERT INTO artifact_dependencies(
                    artifact_id, revision, depends_on_artifact_id, depends_on_revision,
                    depends_on_sha256
                 ) VALUES (?1, 1, ?2, ?3, ?4)",
                params![
                    artifact_id.as_str(),
                    input.id.as_str(),
                    to_i64(input.revision.get())?,
                    input.sha256.as_str()
                ],
            )?;
        }
        transaction.execute(
            "INSERT INTO task_results(task_id, artifact_id, revision, committed_at)
             VALUES (?1, ?2, 1, ?3)",
            params![
                request.task_id.as_str(),
                artifact_id.as_str(),
                committed_at.as_str()
            ],
        )?;
        transaction.execute(
            "UPDATE tasks SET status = 'committed', completed_at = ?2 WHERE id = ?1",
            params![request.task_id.as_str(), committed_at.as_str()],
        )?;
        insert_audit(
            &transaction,
            &event_id,
            "task.complete",
            &request.task_id,
            Some(1),
            "atomically commit validated host-agent candidate",
            &committed_at,
        )?;
        transaction.commit()?;
        Ok(TaskCommitData {
            task_id: request.task_id.clone(),
            status: TaskStatus::Committed,
            artifact: ArtifactReference {
                kind: initial.descriptor.allowed_output_kind,
                id: artifact_id,
                revision: Revision::try_new(1)?,
                sha256: digest,
            },
            committed_at,
            idempotent: false,
        })
    }
}

fn load_normalized_inputs(
    transaction: &Transaction<'_>,
    job_id: &EntityId,
) -> Result<Vec<ArtifactReference>, StoreError> {
    let mut statement = transaction.prepare(
        "SELECT sr.normalized_artifact_id, sr.normalized_artifact_revision, ar.sha256
         FROM sources AS s
         JOIN source_revisions AS sr ON sr.source_id = s.id
         JOIN artifact_revisions AS ar
           ON ar.artifact_id = sr.normalized_artifact_id
          AND ar.revision = sr.normalized_artifact_revision
         WHERE s.job_id = ?1
           AND sr.revision = (SELECT MAX(latest.revision) FROM source_revisions AS latest
                              WHERE latest.source_id = s.id)
         ORDER BY sr.normalized_artifact_id",
    )?;
    statement
        .query_map(params![job_id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .map(|row| {
            let (id, revision, sha256) = row?;
            Ok(ArtifactReference {
                kind: ArtifactKind::SourceNormalizedText,
                id: EntityId::try_new(id)?,
                revision: Revision::try_new(to_u64(revision)?)?,
                sha256: Sha256Digest::try_new(sha256)?,
            })
        })
        .collect()
}

fn load_task_state(
    connection: &Connection,
    task_id: &EntityId,
) -> Result<TaskStateData, StoreError> {
    let (status, descriptor_json): (String, String) = connection
        .query_row(
            "SELECT status, descriptor_json FROM tasks WHERE id = ?1",
            params![task_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::TaskNotFound(task_id.to_string()))?;
    let descriptor = serde_json::from_str(&descriptor_json)?;
    let result = load_task_result(connection, task_id)?.map(|(artifact, _)| artifact);
    Ok(TaskStateData {
        descriptor,
        status: parse_status(&status)?,
        result,
    })
}

fn load_task_result(
    connection: &Connection,
    task_id: &EntityId,
) -> Result<Option<(ArtifactReference, UtcTimestamp)>, StoreError> {
    type ResultRow = (String, i64, String, String, String);
    let row: Option<ResultRow> = connection
        .query_row(
            "SELECT tr.artifact_id, tr.revision, a.kind, ar.sha256, tr.committed_at
             FROM task_results AS tr
             JOIN artifacts AS a ON a.id = tr.artifact_id
             JOIN artifact_revisions AS ar
               ON ar.artifact_id = tr.artifact_id AND ar.revision = tr.revision
             WHERE tr.task_id = ?1",
            params![task_id.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()?;
    row.map(|(id, revision, kind, sha256, committed_at)| {
        Ok((
            ArtifactReference {
                kind: serde_json::from_value(Value::String(kind))?,
                id: EntityId::try_new(id)?,
                revision: Revision::try_new(to_u64(revision)?)?,
                sha256: Sha256Digest::try_new(sha256)?,
            },
            UtcTimestamp::try_new(committed_at)?,
        ))
    })
    .transpose()
}

fn validate_request_shape(
    descriptor: &TaskDescriptor,
    request: &TaskCompletionRequest,
) -> Result<(), StoreError> {
    if descriptor.id != request.task_id || descriptor.lease.id != request.lease_id {
        return Err(StoreError::TaskConflict(
            "task or lease ID does not match the prepared descriptor".to_owned(),
        ));
    }
    if descriptor.job_revision != request.expected_job_revision {
        return Err(StoreError::TaskStale(
            "expected job revision does not match the prepared task".to_owned(),
        ));
    }
    let expected = descriptor
        .input_artifacts
        .iter()
        .map(|input| {
            (
                input.id.as_str(),
                (input.revision.get(), input.sha256.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let provided = request
        .expected_inputs
        .iter()
        .map(|input| {
            (
                input.artifact_id.as_str(),
                (input.revision.get(), input.sha256.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if expected != provided || request.expected_inputs.len() != descriptor.input_artifacts.len() {
        return Err(StoreError::TaskStale(
            "completion request does not repeat every exact prepared input".to_owned(),
        ));
    }
    Ok(())
}

fn validate_candidate(descriptor: &TaskDescriptor, value: &Value) -> Result<(), StoreError> {
    if descriptor.operation != JOB_CRITERION_OPERATION
        || descriptor.candidate_schema.id != PublicSchemaId::Criterion.as_str()
    {
        return Err(StoreError::Invariant(format!(
            "unsupported task contract: {}",
            descriptor.operation
        )));
    }
    let candidate =
        validate_external_candidate::<CriterionRecord>(value).map_err(|error| match error {
            CandidateValidationError::Structural(violations) => {
                StoreError::CandidateStructural(violations)
            }
            CandidateValidationError::Semantic(violations) => {
                StoreError::CandidateSemantic(violations)
            }
        })?;
    if candidate.job_id != descriptor.job_id {
        return Err(StoreError::CandidateSemantic(vec![ContractViolation::new(
            "candidate.job_mismatch",
            "/job_id",
            "candidate job ID does not match the task subject",
        )]));
    }
    Ok(())
}

fn inputs_are_current(
    transaction: &Transaction<'_>,
    request: &TaskCompletionRequest,
) -> Result<bool, StoreError> {
    for input in &request.expected_inputs {
        let actual: Option<(i64, String)> = transaction
            .query_row(
                "SELECT a.head_revision, ar.sha256
                 FROM artifacts AS a
                 JOIN artifact_revisions AS ar
                   ON ar.artifact_id = a.id AND ar.revision = a.head_revision
                 JOIN task_inputs AS ti ON ti.artifact_id = a.id
                 WHERE ti.task_id = ?1 AND a.id = ?2",
                params![request.task_id.as_str(), input.artifact_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if actual.as_ref().is_none_or(|(revision, sha256)| {
            *revision != i64::try_from(input.revision.get()).unwrap_or(-1)
                || sha256 != input.sha256.as_str()
        }) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, StoreError> {
    let sorted = sort_json(value.clone());
    let mut bytes = serde_json::to_vec_pretty(&sorted)?;
    bytes.push(b'\n');
    Ok(bytes)
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
        other => other,
    }
}

fn timestamp_after_minutes(minutes: i64) -> Result<UtcTimestamp, StoreError> {
    let value = (OffsetDateTime::now_utc() + Duration::minutes(minutes))
        .format(&Rfc3339)
        .map_err(|error| StoreError::Invariant(error.to_string()))?;
    Ok(UtcTimestamp::try_new(value)?)
}

fn timestamp_is_past(value: &str) -> Result<bool, StoreError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|error| StoreError::Invariant(error.to_string()))?;
    Ok(parsed <= OffsetDateTime::now_utc())
}

fn insert_audit(
    transaction: &Transaction<'_>,
    event_id: &EntityId,
    action: &str,
    subject_id: &EntityId,
    subject_revision: Option<i64>,
    reason: &str,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, 'host-agent', ?2, ?3, ?4, ?5, ?6)",
        params![
            event_id.as_str(),
            action,
            subject_id.as_str(),
            subject_revision,
            reason,
            created_at.as_str()
        ],
    )?;
    Ok(())
}

fn parse_status(value: &str) -> Result<TaskStatus, StoreError> {
    match value {
        "prepared" => Ok(TaskStatus::Prepared),
        "committed" => Ok(TaskStatus::Committed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        "stale" => Ok(TaskStatus::Stale),
        other => Err(StoreError::Invariant(format!(
            "unknown task status: {other}"
        ))),
    }
}

fn enum_name<T: Serialize>(value: T) -> Result<String, StoreError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| StoreError::Invariant("enum did not serialize as a string".to_owned()))
}

fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value)
        .map_err(|_| StoreError::Invariant("value exceeds SQLite INTEGER range".to_owned()))
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use canisend_contracts::{
        ActorKind, ExpectedInputRevision, PrivacyClassification, SourceKind, TaskCompletionRequest,
        TaskStatus,
    };
    use rusqlite::params;
    use serde_json::json;

    use super::TaskService;
    use crate::{JobService, NewSource, StoreError, Workspace};

    #[test]
    fn expired_lease_is_durably_marked_stale() {
        let root =
            std::env::temp_dir().join(format!("canisend-task-expiry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut workspace = Workspace::init(&root).expect("workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Research Fellow", "University X", ActorKind::User)
            .expect("job");
        JobService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                &job.id,
                NewSource {
                    kind: SourceKind::LocalFile,
                    original_bytes: b"Conduct research".to_vec(),
                    normalized_text: "Conduct research\n".to_owned(),
                    source_url: None,
                    final_url: None,
                    content_type: "text/plain; charset=utf-8".to_owned(),
                    redirect_chain: Vec::new(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("source");
        let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_job_criterion(&job.id)
            .expect("task");
        workspace
            .database
            .connection()
            .execute(
                "UPDATE tasks SET lease_expires_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
                params![descriptor.id.as_str()],
            )
            .expect("expire lease");
        let request = TaskCompletionRequest {
            task_id: descriptor.id.clone(),
            lease_id: descriptor.lease.id,
            expected_job_revision: descriptor.job_revision,
            expected_inputs: descriptor
                .input_artifacts
                .iter()
                .map(|input| ExpectedInputRevision {
                    artifact_id: input.id.clone(),
                    revision: input.revision,
                    sha256: input.sha256.clone(),
                })
                .collect(),
            candidate: json!({
                "id": "019f2f55-7c00-7000-8000-000000000201",
                "job_id": job.id,
                "kind": "research",
                "requirement": "Conduct research",
                "importance": "essential",
                "source_quote": "Conduct research",
                "revision": 1
            }),
        };
        assert!(matches!(
            TaskService::new(&mut workspace.database, &workspace.blobs).complete(&request),
            Err(StoreError::TaskStale(_))
        ));
        assert_eq!(
            TaskService::new(&mut workspace.database, &workspace.blobs)
                .get(&descriptor.id)
                .expect("state")
                .status,
            TaskStatus::Stale
        );
        drop(workspace);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
