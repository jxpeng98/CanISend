use std::collections::BTreeMap;

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, CandidateValidationError, ContractViolation,
    CriteriaSetRecord, EntityId, ParsedJobRecord, Revision, Sha256Digest, UtcTimestamp,
    validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::Value;

use crate::{BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, now_utc};

pub struct CriteriaService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> CriteriaService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn proposed(&self, job_id: &EntityId) -> Result<ParsedJobRecord, StoreError> {
        let context = load_parse_context(self.database.connection(), job_id)?;
        load_parsed_job(self.blobs, &context.parse_output)
    }

    pub fn template(&self, job_id: &EntityId) -> Result<CriteriaSetRecord, StoreError> {
        let parsed = self.proposed(job_id)?;
        let mut criteria = parsed.criteria;
        for criterion in &mut criteria {
            criterion.confirmed = true;
        }
        Ok(CriteriaSetRecord {
            id: generate_id()?,
            job_id: job_id.clone(),
            criteria,
            revision: Revision::try_new(1)?,
        })
    }

    pub fn confirmed(&self, job_id: &EntityId) -> Result<CriteriaSetRecord, StoreError> {
        let output = load_stage_output(
            self.database.connection(),
            job_id,
            "criteria",
            ArtifactKind::Criteria,
        )?;
        load_criteria_set(self.blobs, &output)
    }

    pub fn confirm(
        &mut self,
        job_id: &EntityId,
        value: &Value,
    ) -> Result<ArtifactReference, StoreError> {
        let candidate = validate_candidate(value)?;
        if candidate.job_id != *job_id {
            return Err(StoreError::CandidateSemantic(vec![ContractViolation::new(
                "candidate.job_mismatch",
                "/job_id",
                "criteria job ID must match the command subject",
            )]));
        }
        let initial = load_parse_context(self.database.connection(), job_id)?;
        validate_source_spans(
            self.database.connection(),
            self.blobs,
            &initial.allowed_sources,
            &candidate,
        )?;
        let bytes = canonical_json_bytes(&serde_json::to_value(&candidate)?)?;
        let digest = self.blobs.put_bytes(&bytes)?;
        let size = self.blobs.verify(&digest, DEFAULT_MAX_BLOB_BYTES)?;
        let artifact_id = generate_id()?;
        let event_id = generate_id()?;
        let committed_at = now_utc()?;

        let transaction = self.database.immediate_transaction()?;
        let current = load_parse_context(&transaction, job_id)?;
        if current.run_id != initial.run_id
            || current.parse_output != initial.parse_output
            || current.allowed_sources != initial.allowed_sources
        {
            return Err(StoreError::WorkflowConflict(
                "parse output changed while confirming criteria".to_owned(),
            ));
        }
        if !matches!(current.criteria_status.as_str(), "ready" | "awaiting-user") {
            return Err(StoreError::WorkflowConflict(format!(
                "criteria stage is {}, not ready for confirmation",
                current.criteria_status
            )));
        }
        validate_source_spans(
            &transaction,
            self.blobs,
            &current.allowed_sources,
            &candidate,
        )?;
        transaction.execute(
            "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
             VALUES (?1, 'criteria', 1, 0, ?2)",
            params![artifact_id.as_str(), committed_at.as_str()],
        )?;
        transaction.execute(
            "INSERT INTO artifact_revisions(
                artifact_id, revision, sha256, size, actor, reason, created_at
             ) VALUES (?1, 1, ?2, ?3, 'user', 'confirm corrected job criteria', ?4)",
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
        let dependencies = std::iter::once(&current.parse_output)
            .chain(current.allowed_sources.iter())
            .collect::<Vec<_>>();
        for dependency in dependencies {
            verify_artifact_revision(&transaction, dependency)?;
            transaction.execute(
                "INSERT INTO artifact_dependencies(
                    artifact_id, revision, depends_on_artifact_id, depends_on_revision,
                    depends_on_sha256
                 ) VALUES (?1, 1, ?2, ?3, ?4)",
                params![
                    artifact_id.as_str(),
                    dependency.id.as_str(),
                    to_i64(dependency.revision.get())?,
                    dependency.sha256.as_str()
                ],
            )?;
        }
        let completed = transaction.execute(
            "UPDATE stage_executions
             SET status = 'complete', execution_mode = 'user-decision',
                 output_artifact_id = ?3, output_artifact_revision = 1,
                 started_at = COALESCE(started_at, ?4), completed_at = ?4, updated_at = ?4
             WHERE workflow_run_id = ?1 AND stage = 'criteria'
               AND status IN ('ready', 'awaiting-user') AND id = ?2",
            params![
                current.run_id.as_str(),
                current.criteria_execution_id.as_str(),
                artifact_id.as_str(),
                committed_at.as_str()
            ],
        )?;
        if completed != 1 {
            return Err(StoreError::WorkflowConflict(
                "criteria stage changed while committing confirmation".to_owned(),
            ));
        }
        transaction.execute(
            "UPDATE stage_executions SET status = 'ready', updated_at = ?2
             WHERE workflow_run_id = ?1 AND stage = 'match' AND status IN ('blocked', 'stale')
               AND EXISTS (
                   SELECT 1 FROM stage_executions
                   WHERE workflow_run_id = ?1 AND stage = 'evidence' AND status = 'complete'
               )",
            params![current.run_id.as_str(), committed_at.as_str()],
        )?;
        insert_audit(
            &transaction,
            &event_id,
            "criteria.confirm",
            &artifact_id,
            "validate source-bound corrections and confirm criteria",
            &committed_at,
        )?;
        transaction.commit()?;
        Ok(ArtifactReference {
            kind: ArtifactKind::Criteria,
            id: artifact_id,
            revision: Revision::try_new(1)?,
            sha256: digest,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ParseContext {
    run_id: EntityId,
    criteria_execution_id: EntityId,
    criteria_status: String,
    parse_output: ArtifactReference,
    allowed_sources: Vec<ArtifactReference>,
}

fn load_parse_context(
    connection: &Connection,
    job_id: &EntityId,
) -> Result<ParseContext, StoreError> {
    type Row = (
        String,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<i64>,
    );
    let row: Row = connection
        .query_row(
            "SELECT wr.id, criteria.id, criteria.status, wr.job_revision, jobs.revision,
                    parse.output_artifact_id, parse.output_artifact_revision
             FROM workflow_runs AS wr
             JOIN jobs ON jobs.id = wr.job_id
             JOIN stage_executions AS parse
               ON parse.workflow_run_id = wr.id AND parse.stage = 'parse'
             JOIN stage_executions AS criteria
               ON criteria.workflow_run_id = wr.id AND criteria.stage = 'criteria'
             WHERE wr.job_id = ?1
             ORDER BY wr.created_at DESC, wr.id DESC LIMIT 1",
            params![job_id.as_str()],
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
        .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
    let (run_id, criteria_id, criteria_status, run_revision, job_revision, output_id, output_rev) =
        row;
    if run_revision != job_revision {
        return Err(StoreError::WorkflowConflict(
            "job changed; reconcile the workflow before confirming criteria".to_owned(),
        ));
    }
    let (output_id, output_rev) = output_id.zip(output_rev).ok_or_else(|| {
        StoreError::WorkflowConflict("parse stage has no completed output".to_owned())
    })?;
    let parse_output = load_artifact_reference(
        connection,
        &EntityId::try_new(output_id)?,
        Revision::try_new(to_u64(output_rev)?)?,
        ArtifactKind::ParsedJob,
    )?;
    let mut statement = connection.prepare(
        "SELECT dependency.depends_on_artifact_id, dependency.depends_on_revision,
                dependency.depends_on_sha256
         FROM artifact_dependencies AS dependency
         JOIN artifacts ON artifacts.id = dependency.depends_on_artifact_id
         WHERE dependency.artifact_id = ?1 AND dependency.revision = ?2
           AND artifacts.kind = 'source-normalized-text'
         ORDER BY dependency.depends_on_artifact_id",
    )?;
    let allowed_sources = statement
        .query_map(
            params![
                parse_output.id.as_str(),
                to_i64(parse_output.revision.get())?
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?
        .map(|row| {
            let (id, revision, sha256) = row?;
            Ok(ArtifactReference {
                kind: ArtifactKind::SourceNormalizedText,
                id: EntityId::try_new(id)?,
                revision: Revision::try_new(to_u64(revision)?)?,
                sha256: Sha256Digest::try_new(sha256)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    if allowed_sources.is_empty() {
        return Err(StoreError::Invariant(
            "parsed job has no normalized source dependencies".to_owned(),
        ));
    }
    Ok(ParseContext {
        run_id: EntityId::try_new(run_id)?,
        criteria_execution_id: EntityId::try_new(criteria_id)?,
        criteria_status,
        parse_output,
        allowed_sources,
    })
}

fn load_stage_output(
    connection: &Connection,
    job_id: &EntityId,
    stage: &str,
    expected_kind: ArtifactKind,
) -> Result<ArtifactReference, StoreError> {
    let output: Option<(String, i64)> = connection
        .query_row(
            "SELECT stage.output_artifact_id, stage.output_artifact_revision
             FROM workflow_runs AS run
             JOIN stage_executions AS stage ON stage.workflow_run_id = run.id
             WHERE run.job_id = ?1 AND stage.stage = ?2 AND stage.status = 'complete'
             ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
            params![job_id.as_str(), stage],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (id, revision) = output.ok_or_else(|| {
        StoreError::WorkflowConflict(format!("{stage} stage has no completed output"))
    })?;
    load_artifact_reference(
        connection,
        &EntityId::try_new(id)?,
        Revision::try_new(to_u64(revision)?)?,
        expected_kind,
    )
}

fn load_artifact_reference(
    connection: &Connection,
    id: &EntityId,
    revision: Revision,
    expected_kind: ArtifactKind,
) -> Result<ArtifactReference, StoreError> {
    let row: Option<(String, String, i64)> = connection
        .query_row(
            "SELECT artifacts.kind, revisions.sha256, artifacts.stale
             FROM artifacts
             JOIN artifact_revisions AS revisions
               ON revisions.artifact_id = artifacts.id AND revisions.revision = ?2
             WHERE artifacts.id = ?1 AND artifacts.head_revision = ?2",
            params![id.as_str(), to_i64(revision.get())?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (kind, sha256, stale) = row.ok_or_else(|| StoreError::ArtifactNotFound(id.to_string()))?;
    let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
    if kind != expected_kind || stale != 0 {
        return Err(StoreError::WorkflowConflict(format!(
            "artifact {id} is stale or has the wrong kind"
        )));
    }
    Ok(ArtifactReference {
        kind,
        id: id.clone(),
        revision,
        sha256: Sha256Digest::try_new(sha256)?,
    })
}

fn load_parsed_job(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<ParsedJobRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
}

fn load_criteria_set(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<CriteriaSetRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
}

fn validate_candidate(value: &Value) -> Result<CriteriaSetRecord, StoreError> {
    validate_external_candidate(value).map_err(candidate_error)
}

fn candidate_error(error: CandidateValidationError) -> StoreError {
    match error {
        CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        CandidateValidationError::Semantic(violations) => StoreError::CandidateSemantic(violations),
    }
}

fn validate_source_spans(
    connection: &Connection,
    blobs: &BlobStore,
    allowed_sources: &[ArtifactReference],
    criteria: &CriteriaSetRecord,
) -> Result<(), StoreError> {
    let allowed = allowed_sources
        .iter()
        .map(|artifact| (artifact.id.as_str(), artifact))
        .collect::<BTreeMap<_, _>>();
    let mut cached = BTreeMap::new();
    let mut violations = Vec::new();
    for (index, criterion) in criteria.criteria.iter().enumerate() {
        let source = &criterion.source_span.source;
        let Some(expected) = allowed.get(source.id.as_str()) else {
            violations.push(ContractViolation::new(
                "candidate.source_out_of_scope",
                format!("/criteria/{index}/source_span/source"),
                "criterion source is outside the parsed job input scope",
            ));
            continue;
        };
        if *expected != source {
            violations.push(ContractViolation::new(
                "candidate.source_revision_mismatch",
                format!("/criteria/{index}/source_span/source"),
                "criterion source revision/hash does not match the parsed job input",
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
                .expect("inserted source bytes")
        };
        let start = usize::try_from(criterion.source_span.start_byte).unwrap_or(usize::MAX);
        let end = usize::try_from(criterion.source_span.end_byte).unwrap_or(usize::MAX);
        if !bytes
            .get(start..end)
            .is_some_and(|span| span == criterion.source_quote.as_bytes())
        {
            violations.push(ContractViolation::new(
                "candidate.source_span_mismatch",
                format!("/criteria/{index}/source_span"),
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
        Err(StoreError::DependencyConflict(artifact.id.to_string()))
    }
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, StoreError> {
    fn canonicalize(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let sorted = map
                    .iter()
                    .map(|(key, value)| (key.clone(), canonicalize(value)))
                    .collect::<BTreeMap<_, _>>();
                Value::Object(sorted.into_iter().collect())
            }
            Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
            other => other.clone(),
        }
    }
    let mut bytes = serde_json::to_vec(&canonicalize(value))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn insert_audit(
    transaction: &Transaction<'_>,
    event_id: &EntityId,
    action: &str,
    subject_id: &EntityId,
    reason: &str,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6)",
        params![
            event_id.as_str(),
            enum_name(ActorKind::User)?,
            action,
            subject_id.as_str(),
            reason,
            created_at.as_str()
        ],
    )?;
    Ok(())
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
