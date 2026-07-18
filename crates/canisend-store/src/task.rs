use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, CandidateValidationError, ConsentRequest,
    ConsentScope, ContractViolation, CriteriaSetRecord, EntityId, EvidenceCatalogRecord,
    EvidenceMatchProposalSet, EvidenceMatchRecord, EvidenceMatchSetRecord, EvidenceProposalSet,
    EvidenceRecord, ExecutionMode, PUBLIC_SCHEMA_VERSION, ParsedJobRecord, PublicSchemaId,
    Revision, SafeRelativePath, SchemaReference, SemanticVersion, Sha256Digest, TaskCommitData,
    TaskCompletionRequest, TaskDescriptor, TaskInputExportData, TaskInputExportFile, TaskLease,
    TaskStateData, TaskStatus, UtcTimestamp, validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, now_utc};

pub const JOB_PARSE_OPERATION: &str = "job.parse";
pub const EVIDENCE_NORMALIZE_OPERATION: &str = "profile.evidence.normalize";
pub const EVIDENCE_MATCH_OPERATION: &str = "evidence.match";
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

    pub fn prepare_job_parse(
        &mut self,
        job_id: &EntityId,
        mode: ExecutionMode,
    ) -> Result<TaskDescriptor, StoreError> {
        if !matches!(
            mode,
            ExecutionMode::HostAgent | ExecutionMode::ConfiguredProvider
        ) {
            return Err(StoreError::TaskConflict(
                "job.parse supports only host-agent or configured-provider mode".to_owned(),
            ));
        }
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
        let (stage_execution_id, stage_status): (String, String) = transaction
            .query_row(
                "SELECT se.id, se.status
                 FROM workflow_runs AS wr
                 JOIN stage_executions AS se ON se.workflow_run_id = wr.id
                 WHERE wr.job_id = ?1 AND wr.job_revision = ?2 AND se.stage = 'parse'
                 ORDER BY wr.created_at DESC, wr.id DESC LIMIT 1",
                params![job_id.as_str(), job_revision],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| {
                StoreError::WorkflowNotFound(format!(
                    "{}; start or reconcile the workflow first",
                    job_id
                ))
            })?;
        if stage_status != "ready" {
            return Err(StoreError::WorkflowConflict(format!(
                "parse stage is {stage_status}, not ready"
            )));
        }
        let actor = if mode == ExecutionMode::ConfiguredProvider {
            ActorKind::ConfiguredProvider
        } else {
            ActorKind::HostAgent
        };
        let mut required_consents = vec![ConsentRequest {
            scope: ConsentScope::ReadPrivateInputs,
            description: "Read only the normalized advert artifacts declared by this task"
                .to_owned(),
            artifacts: inputs.clone(),
        }];
        if mode == ExecutionMode::ConfiguredProvider {
            required_consents.push(ConsentRequest {
                scope: ConsentScope::SendToConfiguredProvider,
                description: "Send only these exact source revisions to the configured provider"
                    .to_owned(),
                artifacts: inputs.clone(),
            });
        }
        let descriptor = TaskDescriptor {
            id: task_id.clone(),
            operation: JOB_PARSE_OPERATION.to_owned(),
            job_id: job_id.clone(),
            job_revision: Revision::try_new(to_u64(job_revision)?)?,
            profile_revision: None,
            actor,
            execution_mode: mode,
            input_artifacts: inputs.clone(),
            allowed_output_kind: ArtifactKind::ParsedJob,
            candidate_schema: SchemaReference {
                id: PublicSchemaId::ParsedJob.as_str().to_owned(),
                version: SemanticVersion::try_new(PUBLIC_SCHEMA_VERSION)?,
            },
            required_consents,
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
        let linked = transaction.execute(
            "UPDATE tasks SET stage_execution_id = ?2 WHERE id = ?1",
            params![task_id.as_str(), &stage_execution_id],
        )?;
        let started = transaction.execute(
            "UPDATE stage_executions
             SET status = 'running', execution_mode = ?2, started_at = ?3, updated_at = ?3
             WHERE id = ?1 AND status = 'ready'",
            params![&stage_execution_id, enum_name(mode)?, created_at.as_str()],
        )?;
        if linked != 1 || started != 1 {
            return Err(StoreError::Invariant(
                "parse task could not claim exactly one ready stage".to_owned(),
            ));
        }
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
            "prepare parse task with immutable input revisions",
            &created_at,
        )?;
        transaction.commit()?;
        Ok(descriptor)
    }

    pub fn prepare_evidence_normalization(
        &mut self,
        job_id: &EntityId,
        mode: ExecutionMode,
    ) -> Result<TaskDescriptor, StoreError> {
        if !matches!(
            mode,
            ExecutionMode::HostAgent | ExecutionMode::ConfiguredProvider
        ) {
            return Err(StoreError::TaskConflict(
                "profile.evidence.normalize supports only host-agent or configured-provider mode"
                    .to_owned(),
            ));
        }
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
        let profile_revision: i64 = transaction.query_row(
            "SELECT profile_revision FROM workspace_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        if profile_revision <= 0 {
            return Err(StoreError::InvalidInput(
                "import at least one profile source before normalizing evidence".to_owned(),
            ));
        }
        let inputs = load_profile_inputs(&transaction)?;
        if inputs.is_empty() {
            return Err(StoreError::Invariant(
                "positive profile revision has no normalized sources".to_owned(),
            ));
        }
        let (stage_execution_id, stage_status): (String, String) = transaction
            .query_row(
                "SELECT se.id, se.status
                 FROM workflow_runs AS wr
                 JOIN stage_executions AS se ON se.workflow_run_id = wr.id
                 WHERE wr.job_id = ?1 AND wr.job_revision = ?2 AND se.stage = 'evidence'
                 ORDER BY wr.created_at DESC, wr.id DESC LIMIT 1",
                params![job_id.as_str(), job_revision],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
        if stage_status != "ready" {
            return Err(StoreError::WorkflowConflict(format!(
                "evidence stage is {stage_status}, not ready"
            )));
        }
        let actor = actor_for_mode(mode);
        let required_consents = task_consents(mode, &inputs);
        let descriptor = TaskDescriptor {
            id: task_id.clone(),
            operation: EVIDENCE_NORMALIZE_OPERATION.to_owned(),
            job_id: job_id.clone(),
            job_revision: Revision::try_new(to_u64(job_revision)?)?,
            profile_revision: Some(Revision::try_new(to_u64(profile_revision)?)?),
            actor,
            execution_mode: mode,
            input_artifacts: inputs.clone(),
            allowed_output_kind: ArtifactKind::EvidenceCatalog,
            candidate_schema: SchemaReference {
                id: PublicSchemaId::EvidenceProposals.as_str().to_owned(),
                version: SemanticVersion::try_new(PUBLIC_SCHEMA_VERSION)?,
            },
            required_consents,
            private_read_scope: inputs.clone(),
            lease: TaskLease {
                id: lease_id.clone(),
                expires_at: expires_at.clone(),
            },
        };
        transaction.execute(
            "INSERT INTO tasks(
                id, stage_execution_id, status, lease_expires_at, created_at,
                job_id, job_revision, operation, actor, execution_mode,
                allowed_output_kind, candidate_schema_id, candidate_schema_version,
                lease_id, descriptor_json, profile_revision
             ) VALUES (?1, ?2, 'prepared', ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                       ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                task_id.as_str(),
                &stage_execution_id,
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
                serde_json::to_string(&descriptor)?,
                profile_revision
            ],
        )?;
        let started = transaction.execute(
            "UPDATE stage_executions
             SET status = 'running', execution_mode = ?2, input_profile_revision = ?3,
                 started_at = ?4, updated_at = ?4
             WHERE id = ?1 AND status = 'ready'",
            params![
                &stage_execution_id,
                enum_name(mode)?,
                profile_revision,
                created_at.as_str()
            ],
        )?;
        if started != 1 {
            return Err(StoreError::Invariant(
                "evidence task could not claim exactly one ready stage".to_owned(),
            ));
        }
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
        insert_audit_as(
            &transaction,
            &event_id,
            (actor, "task.prepare"),
            &task_id,
            None,
            "prepare evidence normalization with immutable profile revisions",
            &created_at,
        )?;
        transaction.commit()?;
        Ok(descriptor)
    }

    pub fn prepare_evidence_match(
        &mut self,
        job_id: &EntityId,
        mode: ExecutionMode,
    ) -> Result<TaskDescriptor, StoreError> {
        if !matches!(
            mode,
            ExecutionMode::HostAgent | ExecutionMode::ConfiguredProvider
        ) {
            return Err(StoreError::TaskConflict(
                "evidence.match supports only host-agent or configured-provider mode".to_owned(),
            ));
        }
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
        type StageRow = (String, String, Option<i64>);
        let (stage_execution_id, stage_status, evidence_profile_revision): StageRow = transaction
            .query_row(
                "SELECT matching.id, matching.status, evidence.input_profile_revision
                 FROM workflow_runs AS run
                 JOIN stage_executions AS matching
                   ON matching.workflow_run_id = run.id AND matching.stage = 'match'
                 JOIN stage_executions AS evidence
                   ON evidence.workflow_run_id = run.id AND evidence.stage = 'evidence'
                 WHERE run.job_id = ?1 AND run.job_revision = ?2
                 ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
                params![job_id.as_str(), job_revision],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
        if stage_status != "ready" {
            return Err(StoreError::WorkflowConflict(format!(
                "match stage is {stage_status}, not ready"
            )));
        }
        let profile_revision = evidence_profile_revision.ok_or_else(|| {
            StoreError::Invariant("confirmed evidence has no profile revision".to_owned())
        })?;
        let current_profile_revision: i64 = transaction.query_row(
            "SELECT profile_revision FROM workspace_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        if profile_revision != current_profile_revision {
            return Err(StoreError::WorkflowConflict(
                "confirmed evidence does not match the current profile revision".to_owned(),
            ));
        }
        let criteria = load_completed_stage_output(
            &transaction,
            job_id,
            job_revision,
            "criteria",
            ArtifactKind::Criteria,
        )?;
        let evidence = load_completed_stage_output(
            &transaction,
            job_id,
            job_revision,
            "evidence",
            ArtifactKind::EvidenceCatalog,
        )?;
        let inputs = vec![criteria, evidence];
        let actor = actor_for_mode(mode);
        let required_consents = task_consents(mode, &inputs);
        let descriptor = TaskDescriptor {
            id: task_id.clone(),
            operation: EVIDENCE_MATCH_OPERATION.to_owned(),
            job_id: job_id.clone(),
            job_revision: Revision::try_new(to_u64(job_revision)?)?,
            profile_revision: Some(Revision::try_new(to_u64(profile_revision)?)?),
            actor,
            execution_mode: mode,
            input_artifacts: inputs.clone(),
            allowed_output_kind: ArtifactKind::EvidenceMatches,
            candidate_schema: SchemaReference {
                id: PublicSchemaId::EvidenceMatchProposals.as_str().to_owned(),
                version: SemanticVersion::try_new(PUBLIC_SCHEMA_VERSION)?,
            },
            required_consents,
            private_read_scope: inputs.clone(),
            lease: TaskLease {
                id: lease_id.clone(),
                expires_at: expires_at.clone(),
            },
        };
        transaction.execute(
            "INSERT INTO tasks(
                id, stage_execution_id, status, lease_expires_at, created_at,
                job_id, job_revision, operation, actor, execution_mode,
                allowed_output_kind, candidate_schema_id, candidate_schema_version,
                lease_id, descriptor_json, profile_revision
             ) VALUES (?1, ?2, 'prepared', ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                       ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                task_id.as_str(),
                &stage_execution_id,
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
                serde_json::to_string(&descriptor)?,
                profile_revision
            ],
        )?;
        let started = transaction.execute(
            "UPDATE stage_executions
             SET status = 'running', execution_mode = ?2, input_profile_revision = ?3,
                 started_at = ?4, updated_at = ?4
             WHERE id = ?1 AND status = 'ready'",
            params![
                &stage_execution_id,
                enum_name(mode)?,
                profile_revision,
                created_at.as_str()
            ],
        )?;
        if started != 1 {
            return Err(StoreError::Invariant(
                "match task could not claim exactly one ready stage".to_owned(),
            ));
        }
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
        insert_audit_as(
            &transaction,
            &event_id,
            (actor, "task.prepare"),
            &task_id,
            None,
            "prepare evidence match task with exact criteria and evidence revisions",
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
        let (status, stage_execution_id): (String, Option<String>) = transaction
            .query_row(
                "SELECT status, stage_execution_id FROM tasks WHERE id = ?1",
                params![task_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::TaskNotFound(task_id.to_string()))?;
        match status.as_str() {
            "prepared" => {
                transaction.execute(
                    "UPDATE tasks SET status = 'cancelled', cancelled_at = ?2 WHERE id = ?1",
                    params![task_id.as_str(), cancelled_at.as_str()],
                )?;
                if let Some(stage_execution_id) = &stage_execution_id {
                    transaction.execute(
                        "UPDATE stage_executions
                         SET status = 'ready', execution_mode = NULL, started_at = NULL,
                             updated_at = ?2
                         WHERE id = ?1 AND status = 'running'",
                        params![stage_execution_id, cancelled_at.as_str()],
                    )?;
                }
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

    pub fn export_inputs(
        &mut self,
        task_id: &EntityId,
        destination: &Path,
        provider_send_allowed: bool,
    ) -> Result<TaskInputExportData, StoreError> {
        let state = self.get(task_id)?;
        if state.status != TaskStatus::Prepared {
            return Err(StoreError::TaskConflict(format!(
                "cannot export inputs for task {task_id} in {:?} state",
                state.status
            )));
        }
        if state.descriptor.execution_mode == ExecutionMode::ConfiguredProvider
            && !provider_send_allowed
        {
            return Err(StoreError::TaskConflict(
                "send-to-configured-provider consent must be explicitly confirmed".to_owned(),
            ));
        }
        ensure_empty_external_directory(destination)?;
        let inputs_directory = destination.join("inputs");
        create_private_directory(&inputs_directory)?;
        let mut files = Vec::with_capacity(state.descriptor.private_read_scope.len());
        for (index, artifact) in state.descriptor.private_read_scope.iter().enumerate() {
            verify_scoped_input(self.database.connection(), task_id, artifact)?;
            let bytes = self
                .blobs
                .read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
            let relative_path =
                SafeRelativePath::try_new(format!("inputs/{:03}-{}.txt", index + 1, artifact.id))?;
            write_private_new_file(&destination.join(relative_path.as_str()), &bytes)?;
            files.push(TaskInputExportFile {
                artifact: artifact.clone(),
                relative_path,
            });
        }
        let manifest_body = TaskInputManifestBody {
            format: "canisend.task-inputs/v2".to_owned(),
            task_id: task_id.clone(),
            job_id: state.descriptor.job_id.clone(),
            job_revision: state.descriptor.job_revision,
            files: files.clone(),
        };
        let mut manifest_bytes = serde_json::to_vec_pretty(&manifest_body)?;
        manifest_bytes.push(b'\n');
        let manifest_sha256 = Sha256Digest::try_new(hex::encode(Sha256::digest(&manifest_bytes)))?;
        write_private_new_file(
            &destination.join("canisend-task-inputs.json"),
            &manifest_bytes,
        )?;
        let exported_at = now_utc()?;
        let consent_id = generate_id()?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        let status: String = transaction
            .query_row(
                "SELECT status FROM tasks WHERE id = ?1",
                params![task_id.as_str()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| StoreError::TaskNotFound(task_id.to_string()))?;
        if status != "prepared" {
            return Err(StoreError::TaskConflict(format!(
                "task {task_id} changed state while exporting inputs"
            )));
        }
        transaction.execute(
            "INSERT INTO consents(id, scope, actor, manifest_sha256, granted_at)
             VALUES (?1, 'read-private-inputs', 'user', ?2, ?3)",
            params![
                consent_id.as_str(),
                manifest_sha256.as_str(),
                exported_at.as_str()
            ],
        )?;
        if state.descriptor.execution_mode == ExecutionMode::ConfiguredProvider {
            let provider_consent_id = generate_id()?;
            transaction.execute(
                "INSERT INTO consents(id, scope, actor, manifest_sha256, granted_at)
                 VALUES (?1, 'send-to-configured-provider', 'user', ?2, ?3)",
                params![
                    provider_consent_id.as_str(),
                    manifest_sha256.as_str(),
                    exported_at.as_str()
                ],
            )?;
        }
        insert_audit(
            &transaction,
            &event_id,
            "task.inputs.export",
            task_id,
            None,
            "export declared private inputs after explicit consent",
            &exported_at,
        )?;
        transaction.commit()?;
        Ok(TaskInputExportData {
            format: manifest_body.format,
            task_id: task_id.clone(),
            job_id: state.descriptor.job_id,
            job_revision: state.descriptor.job_revision,
            files,
            manifest_sha256,
        })
    }

    pub fn complete(
        &mut self,
        request: &TaskCompletionRequest,
    ) -> Result<TaskCommitData, StoreError> {
        let initial = self.get(&request.task_id)?;
        if initial.status == TaskStatus::Stale {
            return Err(StoreError::TaskStale(format!(
                "task {} must be prepared again",
                request.task_id
            )));
        }
        validate_request_shape(&initial.descriptor, request)?;
        let validated = validate_candidate(&initial.descriptor, &request.candidate)?;
        validate_task_source_spans(
            self.database.connection(),
            self.blobs,
            &initial.descriptor,
            &validated,
        )?;
        let candidate_bytes = canonical_json_bytes(&request.candidate)?;
        let candidate_sha256 =
            Sha256Digest::try_new(hex::encode(Sha256::digest(&candidate_bytes)))?;
        if initial.status == TaskStatus::Committed {
            return replay_task(
                self.database.connection(),
                &request.task_id,
                &candidate_sha256,
            );
        }
        let output_value = match validated {
            ValidatedTaskCandidate::ParsedJob(_) => request.candidate.clone(),
            ValidatedTaskCandidate::EvidenceProposals(proposals) => {
                serde_json::to_value(build_evidence_catalog(proposals)?)?
            }
            ValidatedTaskCandidate::MatchProposals(proposals) => {
                serde_json::to_value(build_match_set(proposals)?)?
            }
        };
        let bytes = canonical_json_bytes(&output_value)?;
        let digest = self.blobs.put_bytes(&bytes)?;
        let size = self.blobs.verify(&digest, DEFAULT_MAX_BLOB_BYTES)?;
        let transaction = self.database.immediate_transaction()?;
        type TaskRow = (
            String,
            String,
            String,
            i64,
            Option<String>,
            Option<i64>,
            Option<String>,
        );
        let row: TaskRow = transaction
            .query_row(
                "SELECT status, lease_id, lease_expires_at, job_revision, stage_execution_id,
                        profile_revision, candidate_sha256
                 FROM tasks WHERE id = ?1",
                params![request.task_id.as_str()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| StoreError::TaskNotFound(request.task_id.to_string()))?;
        let (
            status,
            lease_id,
            lease_expires_at,
            prepared_job_revision,
            stage_execution_id,
            prepared_profile_revision,
            stored_candidate_sha256,
        ) = row;
        if status == "committed" {
            let (artifact, committed_at) = load_task_result(&transaction, &request.task_id)?
                .ok_or_else(|| {
                    StoreError::Invariant("committed task has no result artifact".to_owned())
                })?;
            if stored_candidate_sha256.as_deref() != Some(candidate_sha256.as_str()) {
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
        if status == "stale" {
            return Err(StoreError::TaskStale(format!(
                "task {} must be prepared again",
                request.task_id
            )));
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
        let current_profile_revision: i64 = transaction.query_row(
            "SELECT profile_revision FROM workspace_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        let expired = timestamp_is_past(&lease_expires_at)?;
        let profile_stale =
            prepared_profile_revision.is_some_and(|revision| revision != current_profile_revision);
        let stale = expired
            || current_job_revision != prepared_job_revision
            || current_job_revision != to_i64(request.expected_job_revision.get())?
            || profile_stale
            || !inputs_are_current(&transaction, request)?;
        if stale {
            let stale_at = now_utc()?;
            let event_id = generate_id()?;
            transaction.execute(
                "UPDATE tasks SET status = 'stale' WHERE id = ?1 AND status = 'prepared'",
                params![request.task_id.as_str()],
            )?;
            if let Some(stage_execution_id) = &stage_execution_id {
                transaction.execute(
                    "UPDATE stage_executions
                     SET status = ?2, execution_mode = NULL, started_at = NULL, updated_at = ?3
                     WHERE id = ?1 AND status = 'running'",
                    params![
                        stage_execution_id,
                        if expired { "ready" } else { "stale" },
                        stale_at.as_str()
                    ],
                )?;
            }
            insert_audit(
                &transaction,
                &event_id,
                "task.stale",
                &request.task_id,
                None,
                if expired {
                    "task lease expired before completion"
                } else if profile_stale {
                    "profile revision changed before completion"
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
             ) VALUES (?1, 1, ?2, ?3, ?4, 'complete declared task', ?5)",
            params![
                artifact_id.as_str(),
                digest.as_str(),
                to_i64(size)?,
                enum_name(initial.descriptor.actor)?,
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
            "UPDATE tasks
             SET status = 'committed', completed_at = ?2, candidate_sha256 = ?3
             WHERE id = ?1",
            params![
                request.task_id.as_str(),
                committed_at.as_str(),
                candidate_sha256.as_str()
            ],
        )?;
        if let Some(stage_execution_id) = &stage_execution_id {
            if initial.descriptor.operation == JOB_PARSE_OPERATION {
                let completed = transaction.execute(
                    "UPDATE stage_executions
                     SET status = 'complete', output_artifact_id = ?2,
                         output_artifact_revision = 1, completed_at = ?3, updated_at = ?3
                     WHERE id = ?1 AND status = 'running'",
                    params![
                        stage_execution_id,
                        artifact_id.as_str(),
                        committed_at.as_str()
                    ],
                )?;
                if completed != 1 {
                    return Err(StoreError::Invariant(
                        "parse task could not complete its claimed stage".to_owned(),
                    ));
                }
                transaction.execute(
                    "UPDATE stage_executions SET status = 'ready', updated_at = ?2
                     WHERE workflow_run_id = (
                         SELECT workflow_run_id FROM stage_executions WHERE id = ?1
                     ) AND stage = 'criteria' AND status IN ('blocked', 'stale')",
                    params![stage_execution_id, committed_at.as_str()],
                )?;
            } else if initial.descriptor.operation == EVIDENCE_NORMALIZE_OPERATION {
                let awaiting = transaction.execute(
                    "UPDATE stage_executions
                     SET status = 'awaiting-user', updated_at = ?2
                     WHERE id = ?1 AND status = 'running'",
                    params![stage_execution_id, committed_at.as_str()],
                )?;
                if awaiting != 1 {
                    return Err(StoreError::Invariant(
                        "evidence task could not enter user confirmation".to_owned(),
                    ));
                }
            } else if initial.descriptor.operation == EVIDENCE_MATCH_OPERATION {
                let completed = transaction.execute(
                    "UPDATE stage_executions
                     SET status = 'complete', output_artifact_id = ?2,
                         output_artifact_revision = 1, completed_at = ?3, updated_at = ?3
                     WHERE id = ?1 AND status = 'running'",
                    params![
                        stage_execution_id,
                        artifact_id.as_str(),
                        committed_at.as_str()
                    ],
                )?;
                if completed != 1 {
                    return Err(StoreError::Invariant(
                        "match task could not complete its claimed stage".to_owned(),
                    ));
                }
                transaction.execute(
                    "UPDATE stage_executions SET status = 'ready', updated_at = ?2
                     WHERE workflow_run_id = (
                         SELECT workflow_run_id FROM stage_executions WHERE id = ?1
                     ) AND stage = 'plan' AND status IN ('blocked', 'stale')",
                    params![stage_execution_id, committed_at.as_str()],
                )?;
            } else {
                return Err(StoreError::Invariant(format!(
                    "task operation has no stage completion rule: {}",
                    initial.descriptor.operation
                )));
            }
        }
        insert_audit_as(
            &transaction,
            &event_id,
            (initial.descriptor.actor, "task.complete"),
            &request.task_id,
            Some(1),
            "atomically commit validated task candidate",
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

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct TaskInputManifestBody {
    format: String,
    task_id: EntityId,
    job_id: EntityId,
    job_revision: Revision,
    files: Vec<TaskInputExportFile>,
}

fn verify_scoped_input(
    connection: &Connection,
    task_id: &EntityId,
    artifact: &ArtifactReference,
) -> Result<(), StoreError> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM task_inputs
             WHERE task_id = ?1 AND artifact_id = ?2 AND revision = ?3 AND sha256 = ?4",
            params![
                task_id.as_str(),
                artifact.id.as_str(),
                to_i64(artifact.revision.get())?,
                artifact.sha256.as_str()
            ],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(StoreError::TaskStale(format!(
            "declared input {} is no longer in task scope",
            artifact.id
        )));
    }
    Ok(())
}

fn ensure_empty_external_directory(path: &Path) -> Result<(), StoreError> {
    if path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(".canisend")
    }) {
        return Err(StoreError::UnsafePath(path.to_path_buf()));
    }
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(StoreError::UnsafePath(path.to_path_buf()));
        }
        if fs::read_dir(path)
            .map_err(|source| crate::io_error(path, source))?
            .next()
            .is_some()
        {
            return Err(StoreError::InvalidInput(
                "task input destination must be empty".to_owned(),
            ));
        }
        set_private_directory_permissions(path)?;
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::UnsafePath(path.to_path_buf()))?;
    let parent_metadata =
        fs::symlink_metadata(parent).map_err(|source| crate::io_error(parent, source))?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(StoreError::UnsafePath(parent.to_path_buf()));
    }
    create_private_directory(path)
}

fn create_private_directory(path: &Path) -> Result<(), StoreError> {
    fs::create_dir(path).map_err(|source| crate::io_error(path, source))?;
    set_private_directory_permissions(path)
}

fn write_private_new_file(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| crate::io_error(path, source))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| crate::io_error(path, source))?;
    set_private_file_permissions(path)
}

fn load_completed_stage_output(
    transaction: &Transaction<'_>,
    job_id: &EntityId,
    job_revision: i64,
    stage: &str,
    expected_kind: ArtifactKind,
) -> Result<ArtifactReference, StoreError> {
    type Row = (String, i64, String, String, i64, i64);
    let row: Row = transaction
        .query_row(
            "SELECT execution.output_artifact_id, execution.output_artifact_revision,
                    artifact.kind, revision.sha256, artifact.head_revision, artifact.stale
             FROM workflow_runs AS run
             JOIN stage_executions AS execution ON execution.workflow_run_id = run.id
             JOIN artifacts AS artifact ON artifact.id = execution.output_artifact_id
             JOIN artifact_revisions AS revision
               ON revision.artifact_id = artifact.id
              AND revision.revision = execution.output_artifact_revision
             WHERE run.job_id = ?1 AND run.job_revision = ?2
               AND execution.stage = ?3 AND execution.status = 'complete'
             ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
            params![job_id.as_str(), job_revision, stage],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| {
            StoreError::WorkflowConflict(format!("{stage} stage has no current completed output"))
        })?;
    let (id, revision, kind, sha256, head_revision, stale) = row;
    let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
    if kind != expected_kind || revision != head_revision || stale != 0 {
        return Err(StoreError::WorkflowConflict(format!(
            "{stage} output is stale or has the wrong artifact kind"
        )));
    }
    Ok(ArtifactReference {
        kind,
        id: EntityId::try_new(id)?,
        revision: Revision::try_new(to_u64(revision)?)?,
        sha256: Sha256Digest::try_new(sha256)?,
    })
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|source| crate::io_error(path, source))
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|source| crate::io_error(path, source))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<(), StoreError> {
    Ok(())
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

fn load_profile_inputs(
    transaction: &Transaction<'_>,
) -> Result<Vec<ArtifactReference>, StoreError> {
    let mut statement = transaction.prepare(
        "SELECT revision.normalized_artifact_id, revision.normalized_artifact_revision,
                artifact.sha256
         FROM profile_sources AS source
         JOIN profile_source_revisions AS revision ON revision.source_id = source.id
         JOIN artifact_revisions AS artifact
           ON artifact.artifact_id = revision.normalized_artifact_id
          AND artifact.revision = revision.normalized_artifact_revision
         WHERE revision.revision = (
             SELECT MAX(head.revision) FROM profile_source_revisions AS head
             WHERE head.source_id = source.id
         )
         ORDER BY source.created_at, source.id",
    )?;
    statement
        .query_map([], |row| {
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

fn actor_for_mode(mode: ExecutionMode) -> ActorKind {
    if mode == ExecutionMode::ConfiguredProvider {
        ActorKind::ConfiguredProvider
    } else {
        ActorKind::HostAgent
    }
}

fn task_consents(mode: ExecutionMode, inputs: &[ArtifactReference]) -> Vec<ConsentRequest> {
    let mut consents = vec![ConsentRequest {
        scope: ConsentScope::ReadPrivateInputs,
        description: "Read only the exact private artifacts declared by this task".to_owned(),
        artifacts: inputs.to_vec(),
    }];
    if mode == ExecutionMode::ConfiguredProvider {
        consents.push(ConsentRequest {
            scope: ConsentScope::SendToConfiguredProvider,
            description: "Send only these exact artifact revisions to the configured provider"
                .to_owned(),
            artifacts: inputs.to_vec(),
        });
    }
    consents
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

enum ValidatedTaskCandidate {
    ParsedJob(ParsedJobRecord),
    EvidenceProposals(EvidenceProposalSet),
    MatchProposals(EvidenceMatchProposalSet),
}

fn validate_candidate(
    descriptor: &TaskDescriptor,
    value: &Value,
) -> Result<ValidatedTaskCandidate, StoreError> {
    match (
        descriptor.operation.as_str(),
        descriptor.candidate_schema.id.as_str(),
    ) {
        (JOB_PARSE_OPERATION, schema) if schema == PublicSchemaId::ParsedJob.as_str() => {
            let candidate = validate_external_candidate::<ParsedJobRecord>(value)
                .map_err(map_candidate_error)?;
            if candidate.job_id != descriptor.job_id {
                return Err(StoreError::CandidateSemantic(vec![ContractViolation::new(
                    "candidate.job_mismatch",
                    "/job_id",
                    "candidate job ID does not match the task subject",
                )]));
            }
            Ok(ValidatedTaskCandidate::ParsedJob(candidate))
        }
        (EVIDENCE_NORMALIZE_OPERATION, schema)
            if schema == PublicSchemaId::EvidenceProposals.as_str() =>
        {
            let candidate = validate_external_candidate::<EvidenceProposalSet>(value)
                .map_err(map_candidate_error)?;
            let expected_revision = descriptor.profile_revision.ok_or_else(|| {
                StoreError::Invariant("evidence task is missing its profile revision".to_owned())
            })?;
            if candidate.profile_revision != expected_revision {
                return Err(StoreError::CandidateSemantic(vec![ContractViolation::new(
                    "candidate.profile_revision_mismatch",
                    "/profile_revision",
                    "candidate profile revision does not match the prepared task",
                )]));
            }
            Ok(ValidatedTaskCandidate::EvidenceProposals(candidate))
        }
        (EVIDENCE_MATCH_OPERATION, schema)
            if schema == PublicSchemaId::EvidenceMatchProposals.as_str() =>
        {
            let candidate = validate_external_candidate::<EvidenceMatchProposalSet>(value)
                .map_err(map_candidate_error)?;
            if candidate.job_id != descriptor.job_id {
                return Err(StoreError::CandidateSemantic(vec![ContractViolation::new(
                    "candidate.job_mismatch",
                    "/job_id",
                    "candidate job ID does not match the task subject",
                )]));
            }
            Ok(ValidatedTaskCandidate::MatchProposals(candidate))
        }
        _ => Err(StoreError::Invariant(format!(
            "unsupported task contract: {} -> {}",
            descriptor.operation, descriptor.candidate_schema.id
        ))),
    }
}

fn map_candidate_error(error: CandidateValidationError) -> StoreError {
    match error {
        CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        CandidateValidationError::Semantic(violations) => StoreError::CandidateSemantic(violations),
    }
}

fn build_evidence_catalog(
    proposals: EvidenceProposalSet,
) -> Result<EvidenceCatalogRecord, StoreError> {
    let items = proposals
        .proposals
        .into_iter()
        .map(|proposal| {
            Ok(EvidenceRecord {
                id: generate_id()?,
                kind: proposal.kind,
                summary: proposal.summary,
                source_quote: proposal.source_quote,
                source_span: proposal.source_span,
                confirmed: false,
                excluded: false,
                sensitivity: proposal.sensitivity,
                revision: Revision::try_new(1)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(EvidenceCatalogRecord {
        id: generate_id()?,
        profile_revision: proposals.profile_revision,
        items,
        revision: Revision::try_new(1)?,
    })
}

fn build_match_set(
    proposals: EvidenceMatchProposalSet,
) -> Result<EvidenceMatchSetRecord, StoreError> {
    let matches = proposals
        .proposals
        .into_iter()
        .map(|proposal| {
            Ok(EvidenceMatchRecord {
                id: generate_id()?,
                criterion: proposal.criterion,
                evidence: proposal.evidence,
                strength: proposal.strength,
                rationale: proposal.rationale,
                gap: proposal.gap,
                prohibited_claims: proposal.prohibited_claims,
                revision: Revision::try_new(1)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(EvidenceMatchSetRecord {
        id: generate_id()?,
        job_id: proposals.job_id,
        criteria_artifact: proposals.criteria_artifact,
        evidence_artifact: proposals.evidence_artifact,
        matches,
        revision: Revision::try_new(1)?,
    })
}

fn validate_task_source_spans(
    connection: &Connection,
    blobs: &BlobStore,
    descriptor: &TaskDescriptor,
    candidate: &ValidatedTaskCandidate,
) -> Result<(), StoreError> {
    if let ValidatedTaskCandidate::MatchProposals(proposals) = candidate {
        return validate_match_scope(connection, blobs, descriptor, proposals);
    }
    let allowed = descriptor
        .input_artifacts
        .iter()
        .map(|artifact| (artifact.id.as_str(), artifact))
        .collect::<BTreeMap<_, _>>();
    let mut cached = BTreeMap::new();
    let mut violations = Vec::new();
    let spans = match candidate {
        ValidatedTaskCandidate::ParsedJob(parsed_job) => parsed_job
            .criteria
            .iter()
            .enumerate()
            .map(|(index, criterion)| {
                (
                    &criterion.source_span,
                    criterion.source_quote.as_str(),
                    format!("/criteria/{index}/source_span"),
                )
            })
            .collect::<Vec<_>>(),
        ValidatedTaskCandidate::EvidenceProposals(proposals) => proposals
            .proposals
            .iter()
            .enumerate()
            .map(|(index, proposal)| {
                (
                    &proposal.source_span,
                    proposal.source_quote.as_str(),
                    format!("/proposals/{index}/source_span"),
                )
            })
            .collect::<Vec<_>>(),
        ValidatedTaskCandidate::MatchProposals(_) => unreachable!("match candidates return above"),
    };
    for (span, source_quote, pointer) in spans {
        let source = &span.source;
        let Some(expected) = allowed.get(source.id.as_str()) else {
            violations.push(ContractViolation::new(
                "candidate.source_out_of_scope",
                format!("{pointer}/source"),
                "source is outside the task input scope",
            ));
            continue;
        };
        if *expected != source {
            violations.push(ContractViolation::new(
                "candidate.source_revision_mismatch",
                format!("{pointer}/source"),
                "source revision/hash does not exactly match the task input",
            ));
            continue;
        }
        verify_artifact_revision(connection, source)?;
        let bytes = if let Some(bytes) = cached.get(source.id.as_str()) {
            bytes
        } else {
            let bytes = blobs.read_verified(&source.sha256, DEFAULT_MAX_BLOB_BYTES)?;
            cached.insert(source.id.as_str().to_owned(), bytes);
            cached
                .get(source.id.as_str())
                .expect("inserted source bytes are present")
        };
        let start = usize::try_from(span.start_byte).unwrap_or(usize::MAX);
        let end = usize::try_from(span.end_byte).unwrap_or(usize::MAX);
        let exact = bytes
            .get(start..end)
            .is_some_and(|bytes| bytes == source_quote.as_bytes());
        if !exact {
            violations.push(ContractViolation::new(
                "candidate.source_span_mismatch",
                pointer,
                "source span must select bytes exactly equal to source_quote",
            ));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(StoreError::CandidateSemantic(violations))
    }
}

fn validate_match_scope(
    connection: &Connection,
    blobs: &BlobStore,
    descriptor: &TaskDescriptor,
    candidate: &EvidenceMatchProposalSet,
) -> Result<(), StoreError> {
    let criteria_input = unique_input_kind(descriptor, ArtifactKind::Criteria)?;
    let evidence_input = unique_input_kind(descriptor, ArtifactKind::EvidenceCatalog)?;
    let mut violations = Vec::new();
    if candidate.criteria_artifact != *criteria_input {
        violations.push(ContractViolation::new(
            "match.criteria_artifact_mismatch",
            "/criteria_artifact",
            "candidate must repeat the exact declared criteria artifact revision and hash",
        ));
    }
    if candidate.evidence_artifact != *evidence_input {
        violations.push(ContractViolation::new(
            "match.evidence_artifact_mismatch",
            "/evidence_artifact",
            "candidate must repeat the exact declared evidence artifact revision and hash",
        ));
    }
    verify_artifact_revision(connection, criteria_input)?;
    verify_artifact_revision(connection, evidence_input)?;
    let criteria = load_criteria_artifact(blobs, criteria_input)?;
    let evidence = load_evidence_artifact(blobs, evidence_input)?;
    if criteria.job_id != descriptor.job_id {
        return Err(StoreError::Invariant(
            "criteria artifact belongs to a different job".to_owned(),
        ));
    }
    if evidence.items.iter().any(|item| !item.confirmed) {
        return Err(StoreError::Invariant(
            "match task received an unconfirmed evidence catalog".to_owned(),
        ));
    }
    let expected_criteria = criteria
        .criteria
        .iter()
        .map(|criterion| (&criterion.id, criterion.revision))
        .collect::<BTreeMap<_, _>>();
    let available_evidence = evidence
        .items
        .iter()
        .map(|item| (&item.id, item))
        .collect::<BTreeMap<_, _>>();
    let mut proposed_criteria = BTreeMap::new();
    for (index, proposal) in candidate.proposals.iter().enumerate() {
        proposed_criteria.insert(&proposal.criterion.id, proposal.criterion.revision);
        match expected_criteria.get(&proposal.criterion.id) {
            None => violations.push(ContractViolation::new(
                "match.criterion_unknown",
                format!("/proposals/{index}/criterion/id"),
                "proposal references a criterion outside the declared criteria artifact",
            )),
            Some(revision) if *revision != proposal.criterion.revision => {
                violations.push(ContractViolation::new(
                    "match.criterion_revision_mismatch",
                    format!("/proposals/{index}/criterion/revision"),
                    "proposal must cite the exact confirmed criterion revision",
                ));
            }
            Some(_) => {}
        }
        for (evidence_index, reference) in proposal.evidence.iter().enumerate() {
            let pointer = format!("/proposals/{index}/evidence/{evidence_index}");
            match available_evidence.get(&reference.id) {
                None => violations.push(ContractViolation::new(
                    "match.evidence_unknown",
                    format!("{pointer}/id"),
                    "proposal references evidence outside the declared catalog",
                )),
                Some(item) if item.revision != reference.revision => {
                    violations.push(ContractViolation::new(
                        "match.evidence_revision_mismatch",
                        format!("{pointer}/revision"),
                        "proposal must cite the exact confirmed evidence revision",
                    ));
                }
                Some(item) if item.excluded => violations.push(ContractViolation::new(
                    "match.evidence_excluded",
                    format!("{pointer}/id"),
                    "excluded evidence cannot support a criterion",
                )),
                Some(_) => {}
            }
        }
    }
    let expected_references = expected_criteria
        .iter()
        .map(|(id, revision)| (*id, *revision))
        .collect::<BTreeMap<_, _>>();
    if proposed_criteria != expected_references {
        violations.push(ContractViolation::new(
            "match.criteria_coverage_invalid",
            "/proposals",
            "candidate must contain exactly one exact-revision proposal for every criterion",
        ));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(StoreError::CandidateSemantic(violations))
    }
}

fn unique_input_kind(
    descriptor: &TaskDescriptor,
    kind: ArtifactKind,
) -> Result<&ArtifactReference, StoreError> {
    let matching = descriptor
        .input_artifacts
        .iter()
        .filter(|artifact| artifact.kind == kind)
        .collect::<Vec<_>>();
    if matching.len() == 1 {
        Ok(matching[0])
    } else {
        Err(StoreError::Invariant(format!(
            "task {} requires exactly one {kind:?} input",
            descriptor.id
        )))
    }
}

fn load_criteria_artifact(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<CriteriaSetRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(map_candidate_error)
}

fn load_evidence_artifact(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<EvidenceCatalogRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(map_candidate_error)
}

fn replay_task(
    connection: &Connection,
    task_id: &EntityId,
    candidate_sha256: &Sha256Digest,
) -> Result<TaskCommitData, StoreError> {
    let stored_candidate_sha256: Option<String> = connection
        .query_row(
            "SELECT candidate_sha256 FROM tasks WHERE id = ?1 AND status = 'committed'",
            params![task_id.as_str()],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    if stored_candidate_sha256.as_deref() != Some(candidate_sha256.as_str()) {
        return Err(StoreError::TaskConflict(format!(
            "task {task_id} was already completed with a different candidate"
        )));
    }
    let (artifact, committed_at) = load_task_result(connection, task_id)?
        .ok_or_else(|| StoreError::Invariant("committed task has no result artifact".to_owned()))?;
    Ok(TaskCommitData {
        task_id: task_id.clone(),
        status: TaskStatus::Committed,
        artifact,
        committed_at,
        idempotent: true,
    })
}

fn verify_artifact_revision(
    connection: &Connection,
    artifact: &ArtifactReference,
) -> Result<(), StoreError> {
    let actual: Option<String> = connection
        .query_row(
            "SELECT sha256 FROM artifact_revisions WHERE artifact_id = ?1 AND revision = ?2",
            params![artifact.id.as_str(), to_i64(artifact.revision.get())?],
            |row| row.get(0),
        )
        .optional()?;
    if actual.as_deref() == Some(artifact.sha256.as_str()) {
        Ok(())
    } else {
        Err(StoreError::TaskStale(format!(
            "source artifact {} is no longer available",
            artifact.id
        )))
    }
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
    insert_audit_as(
        transaction,
        event_id,
        (ActorKind::HostAgent, action),
        subject_id,
        subject_revision,
        reason,
        created_at,
    )
}

fn insert_audit_as(
    transaction: &Transaction<'_>,
    event_id: &EntityId,
    actor_action: (ActorKind, &str),
    subject_id: &EntityId,
    subject_revision: Option<i64>,
    reason: &str,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    let (actor, action) = actor_action;
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event_id.as_str(),
            enum_name(actor)?,
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
        ActorKind, ExecutionMode, ExpectedInputRevision, PrivacyClassification, SourceKind,
        TaskCompletionRequest, TaskStatus,
    };
    use rusqlite::params;
    use serde_json::json;

    use super::TaskService;
    use crate::{JobService, NewSource, StoreError, WorkflowService, Workspace};

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
        WorkflowService::new(&mut workspace.database)
            .start(&job.id)
            .expect("workflow");
        let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_job_parse(&job.id, ExecutionMode::HostAgent)
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
                "title": "Research Fellow",
                "institution": "University X",
                "summary": "Conduct research",
                "responsibilities": ["Conduct research"],
                "criteria": [{
                    "id": "019f2f55-7c00-7000-8000-000000000202",
                    "job_id": job.id,
                    "kind": "research",
                    "requirement": "Conduct research",
                    "importance": "essential",
                    "source_quote": "Conduct research",
                    "source_span": {
                        "source": descriptor.input_artifacts[0],
                        "start_byte": 0,
                        "end_byte": 16
                    },
                    "confidence_milli": 900,
                    "confirmed": false,
                    "revision": 1
                }],
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
