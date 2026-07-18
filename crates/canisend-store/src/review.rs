use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, CandidateValidationError, ContractViolation,
    EntityId, FindingAuthority, FindingDisposition, FindingDispositionCandidateRecord,
    FindingStatus, ReviewDispositionCandidate, ReviewFindingsRecord, Revision, Sha256Digest,
    validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Map, Value};

use crate::{BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, now_utc};

pub struct ReviewService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> ReviewService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn current(&self, job_id: &EntityId) -> Result<ReviewFindingsRecord, StoreError> {
        let (_, reference) = load_current_reference(self.database.connection(), job_id)?;
        load_review(self.blobs, &reference)
    }

    pub fn template(&self, job_id: &EntityId) -> Result<ReviewDispositionCandidate, StoreError> {
        let (_, reference) = load_current_reference(self.database.connection(), job_id)?;
        let review = load_review(self.blobs, &reference)?;
        Ok(ReviewDispositionCandidate {
            job_id: job_id.clone(),
            review_artifact: reference,
            decisions: review
                .findings
                .iter()
                .filter(|finding| finding.authority == FindingAuthority::HumanReview)
                .map(|finding| FindingDispositionCandidateRecord {
                    finding_id: finding.id.clone(),
                    expected_revision: finding.revision,
                    disposition: match finding.status {
                        FindingStatus::AcceptedRisk => Some(FindingDisposition::AcceptedRisk),
                        FindingStatus::Dismissed => Some(FindingDisposition::Dismissed),
                        FindingStatus::Open | FindingStatus::Resolved => None,
                    },
                    rationale: finding.disposition_reason.clone(),
                })
                .collect(),
        })
    }

    pub fn confirm(
        &mut self,
        job_id: &EntityId,
        value: &Value,
    ) -> Result<ArtifactReference, StoreError> {
        let candidate: ReviewDispositionCandidate =
            validate_external_candidate(value).map_err(candidate_error)?;
        if candidate.job_id != *job_id {
            return Err(semantic_error(
                "review_disposition.job_mismatch",
                "/job_id",
                "review disposition job ID must match the command subject",
            ));
        }
        let (run_id, current_reference) =
            load_current_reference(self.database.connection(), job_id)?;
        if candidate.review_artifact != current_reference {
            return Err(StoreError::TaskStale(
                "review disposition does not reference the exact current review revision"
                    .to_owned(),
            ));
        }
        let mut review = load_review(self.blobs, &current_reference)?;
        let decided_at = now_utc()?;
        let mut changed = false;
        let mut selected = 0_usize;
        for (index, decision) in candidate.decisions.iter().enumerate() {
            let Some(disposition) = decision.disposition else {
                continue;
            };
            selected += 1;
            let finding = review
                .findings
                .iter_mut()
                .find(|finding| finding.id == decision.finding_id)
                .ok_or_else(|| {
                    semantic_error(
                        "review_disposition.finding_unknown",
                        format!("/decisions/{index}/finding_id"),
                        "disposition references a finding outside the current review",
                    )
                })?;
            if finding.revision != decision.expected_revision {
                return Err(StoreError::TaskStale(format!(
                    "finding {} revision changed before disposition",
                    finding.id
                )));
            }
            if finding.authority != FindingAuthority::HumanReview {
                return Err(semantic_error(
                    "review_disposition.deterministic_forbidden",
                    format!("/decisions/{index}/finding_id"),
                    "deterministic findings can be resolved only by regenerating current documents",
                ));
            }
            let rationale = decision.rationale.clone().ok_or_else(|| {
                StoreError::Invariant("validated disposition has no rationale".to_owned())
            })?;
            let next_status = match disposition {
                FindingDisposition::AcceptedRisk => FindingStatus::AcceptedRisk,
                FindingDisposition::Dismissed => FindingStatus::Dismissed,
            };
            if finding.status != next_status
                || finding.disposition_reason.as_deref() != Some(rationale.as_str())
            {
                let next =
                    finding.revision.get().checked_add(1).ok_or_else(|| {
                        StoreError::Invariant("finding revision overflow".to_owned())
                    })?;
                finding.status = next_status;
                finding.disposition_reason = Some(rationale);
                finding.decided_by = Some(ActorKind::User);
                finding.decided_at = Some(decided_at.clone());
                finding.revision = Revision::try_new(next)?;
                changed = true;
            }
        }
        if selected == 0 {
            return Err(StoreError::InvalidInput(
                "select at least one human-review finding disposition".to_owned(),
            ));
        }
        if !changed {
            return Ok(current_reference);
        }
        let next_revision = review
            .revision
            .get()
            .checked_add(1)
            .ok_or_else(|| StoreError::Invariant("review revision overflow".to_owned()))?;
        review.revision = Revision::try_new(next_revision)?;
        validate_external_candidate::<ReviewFindingsRecord>(&serde_json::to_value(&review)?)
            .map_err(candidate_error)?;
        let bytes = canonical_json_bytes(&serde_json::to_value(&review)?)?;
        let digest = self.blobs.put_bytes(&bytes)?;
        let size = self.blobs.verify(&digest, DEFAULT_MAX_BLOB_BYTES)?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        verify_current_reference(&transaction, &run_id, &current_reference)?;
        let updated = transaction.execute(
            "UPDATE artifacts SET head_revision = ?2, stale = 0
             WHERE id = ?1 AND head_revision = ?3 AND stale = 0",
            params![
                current_reference.id.as_str(),
                to_i64(next_revision)?,
                to_i64(current_reference.revision.get())?
            ],
        )?;
        if updated != 1 {
            return Err(StoreError::TaskStale(
                "review changed while committing dispositions".to_owned(),
            ));
        }
        transaction.execute(
            "INSERT INTO artifact_revisions(
                artifact_id, revision, sha256, size, actor, reason, created_at
             ) VALUES (?1, ?2, ?3, ?4, 'user', 'record review finding dispositions', ?5)",
            params![
                current_reference.id.as_str(),
                to_i64(next_revision)?,
                digest.as_str(),
                to_i64(size)?,
                decided_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
             VALUES (?1, 'artifact', ?2, ?3, ?4)",
            params![
                digest.as_str(),
                current_reference.id.as_str(),
                to_i64(next_revision)?,
                decided_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO artifact_dependencies(
                artifact_id, revision, depends_on_artifact_id, depends_on_revision,
                depends_on_sha256
             )
             SELECT artifact_id, ?3, depends_on_artifact_id, depends_on_revision,
                    depends_on_sha256
             FROM artifact_dependencies WHERE artifact_id = ?1 AND revision = ?2",
            params![
                current_reference.id.as_str(),
                to_i64(current_reference.revision.get())?,
                to_i64(next_revision)?
            ],
        )?;
        let deterministic_blockers = review
            .findings
            .iter()
            .filter(|finding| {
                finding.authority == FindingAuthority::Deterministic
                    && finding.severity == canisend_contracts::FindingSeverity::Blocker
                    && finding.status == FindingStatus::Open
            })
            .count();
        let pending_human = review
            .findings
            .iter()
            .filter(|finding| {
                finding.authority == FindingAuthority::HumanReview
                    && finding.status == FindingStatus::Open
            })
            .count();
        let head_updated = transaction.execute(
            "UPDATE review_heads
             SET artifact_revision = ?2, deterministic_blocker_count = ?3,
                 pending_human_count = ?4, updated_at = ?5
             WHERE workflow_run_id = ?1 AND artifact_id = ?6 AND artifact_revision = ?7",
            params![
                run_id.as_str(),
                to_i64(next_revision)?,
                to_i64(u64::try_from(deterministic_blockers).map_err(|_| {
                    StoreError::Invariant("deterministic blocker count overflow".to_owned())
                })?)?,
                to_i64(u64::try_from(pending_human).map_err(|_| {
                    StoreError::Invariant("pending human count overflow".to_owned())
                })?)?,
                decided_at.as_str(),
                current_reference.id.as_str(),
                to_i64(current_reference.revision.get())?
            ],
        )?;
        if head_updated != 1 {
            return Err(StoreError::TaskStale(
                "review head changed while committing dispositions".to_owned(),
            ));
        }
        let stage_updated = transaction.execute(
            "UPDATE stage_executions
             SET output_artifact_revision = ?2, updated_at = ?3
             WHERE workflow_run_id = ?1 AND stage = 'review' AND status = 'complete'
               AND output_artifact_id = ?4 AND output_artifact_revision = ?5",
            params![
                run_id.as_str(),
                to_i64(next_revision)?,
                decided_at.as_str(),
                current_reference.id.as_str(),
                to_i64(current_reference.revision.get())?
            ],
        )?;
        if stage_updated != 1 {
            return Err(StoreError::TaskStale(
                "review stage changed while committing dispositions".to_owned(),
            ));
        }
        invalidate_package_descendants(&transaction, &run_id, &decided_at)?;
        insert_audit(
            &transaction,
            &event_id,
            &review.id,
            review.revision,
            &decided_at,
        )?;
        transaction.commit()?;
        Ok(ArtifactReference {
            kind: ArtifactKind::ReviewFindings,
            id: current_reference.id,
            revision: review.revision,
            sha256: digest,
        })
    }
}

fn load_current_reference(
    connection: &Connection,
    job_id: &EntityId,
) -> Result<(EntityId, ArtifactReference), StoreError> {
    type Row = (String, String, i64, String, String, i64, i64);
    let row: Option<Row> = connection
        .query_row(
            "SELECT run.id, head.artifact_id, head.artifact_revision,
                    artifact.kind, revision.sha256, artifact.head_revision, artifact.stale
             FROM workflow_runs AS run
             JOIN stage_executions AS review
               ON review.workflow_run_id = run.id AND review.stage = 'review'
             JOIN review_heads AS head ON head.workflow_run_id = run.id
             JOIN artifacts AS artifact ON artifact.id = head.artifact_id
             JOIN artifact_revisions AS revision
               ON revision.artifact_id = head.artifact_id
              AND revision.revision = head.artifact_revision
             WHERE run.id = (
                 SELECT id FROM workflow_runs WHERE job_id = ?1
                 ORDER BY created_at DESC, id DESC LIMIT 1
             ) AND review.status = 'complete'
               AND review.output_artifact_id = head.artifact_id
               AND review.output_artifact_revision = head.artifact_revision",
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
        .optional()?;
    let (run_id, id, revision, kind, sha256, head_revision, stale) =
        row.ok_or_else(|| StoreError::WorkflowConflict("review stage is not complete".to_owned()))?;
    let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
    if kind != ArtifactKind::ReviewFindings || head_revision != revision || stale != 0 {
        return Err(StoreError::TaskStale(
            "current review findings are stale".to_owned(),
        ));
    }
    Ok((
        EntityId::try_new(run_id)?,
        ArtifactReference {
            kind,
            id: EntityId::try_new(id)?,
            revision: Revision::try_new(to_u64(revision)?)?,
            sha256: Sha256Digest::try_new(sha256)?,
        },
    ))
}

fn load_review(
    blobs: &BlobStore,
    reference: &ArtifactReference,
) -> Result<ReviewFindingsRecord, StoreError> {
    let bytes = blobs.read_verified(&reference.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
}

fn verify_current_reference(
    transaction: &Transaction<'_>,
    run_id: &EntityId,
    reference: &ArtifactReference,
) -> Result<(), StoreError> {
    let current: Option<(String, i64)> = transaction
        .query_row(
            "SELECT artifact_id, artifact_revision FROM review_heads
             WHERE workflow_run_id = ?1",
            params![run_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if current.as_ref().is_some_and(|(id, revision)| {
        id == reference.id.as_str()
            && *revision == i64::try_from(reference.revision.get()).unwrap_or(-1)
    }) {
        Ok(())
    } else {
        Err(StoreError::TaskStale(
            "review head changed before commit".to_owned(),
        ))
    }
}

fn invalidate_package_descendants(
    transaction: &Transaction<'_>,
    run_id: &EntityId,
    updated_at: &canisend_contracts::UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT artifact_id FROM export_heads WHERE workflow_run_id = ?1
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "DELETE FROM export_heads WHERE workflow_run_id = ?1",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "DELETE FROM package_heads WHERE workflow_run_id = ?1",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT output_artifact_id FROM stage_executions
             WHERE workflow_run_id = ?1 AND stage IN ('package', 'render')
               AND output_artifact_id IS NOT NULL
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE tasks SET status = 'stale' WHERE status = 'prepared'
         AND stage_execution_id IN (
             SELECT id FROM stage_executions
             WHERE workflow_run_id = ?1 AND stage IN ('package', 'render')
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE stage_executions
         SET status = CASE WHEN stage = 'package' THEN 'ready' ELSE 'stale' END,
             execution_mode = NULL, output_artifact_id = NULL,
             output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
             updated_at = ?2
         WHERE workflow_run_id = ?1 AND stage IN ('package', 'render')",
        params![run_id.as_str(), updated_at.as_str()],
    )?;
    transaction.execute(
        "UPDATE workflow_runs SET status = 'active' WHERE id = ?1",
        params![run_id.as_str()],
    )?;
    Ok(())
}

fn insert_audit(
    transaction: &Transaction<'_>,
    event_id: &EntityId,
    review_id: &EntityId,
    revision: Revision,
    created_at: &canisend_contracts::UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, 'user', 'review.disposition.confirm', ?2, ?3, ?4, ?5)",
        params![
            event_id.as_str(),
            review_id.as_str(),
            to_i64(revision.get())?,
            "record explicit user dispositions without changing deterministic findings",
            created_at.as_str()
        ],
    )?;
    Ok(())
}

fn semantic_error(
    code: impl Into<String>,
    pointer: impl Into<String>,
    message: impl Into<String>,
) -> StoreError {
    StoreError::CandidateSemantic(vec![ContractViolation::new(code, pointer, message)])
}

fn candidate_error(error: CandidateValidationError) -> StoreError {
    match error {
        CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        CandidateValidationError::Semantic(violations) => StoreError::CandidateSemantic(violations),
    }
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

fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value)
        .map_err(|_| StoreError::Invariant("value exceeds SQLite INTEGER range".to_owned()))
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}
