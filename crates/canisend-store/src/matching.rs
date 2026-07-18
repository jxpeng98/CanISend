use canisend_contracts::{
    ArtifactKind, ArtifactReference, CandidateValidationError, EntityId, EvidenceMatchSetRecord,
    Revision, Sha256Digest, validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::{BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError};

pub struct MatchService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> MatchService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn current(&self, job_id: &EntityId) -> Result<EvidenceMatchSetRecord, StoreError> {
        let artifact = load_match_output(self.database.connection(), job_id)?;
        let bytes = self
            .blobs
            .read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
        let value: Value = serde_json::from_slice(&bytes)?;
        validate_external_candidate(&value).map_err(candidate_error)
    }
}

fn load_match_output(
    connection: &Connection,
    job_id: &EntityId,
) -> Result<ArtifactReference, StoreError> {
    type Row = (String, i64, String, String, i64, i64);
    let row: Row = connection
        .query_row(
            "SELECT matching.output_artifact_id, matching.output_artifact_revision,
                    artifact.kind, revision.sha256, artifact.head_revision, artifact.stale
             FROM workflow_runs AS run
             JOIN jobs ON jobs.id = run.job_id
             JOIN stage_executions AS matching
               ON matching.workflow_run_id = run.id AND matching.stage = 'match'
             JOIN artifacts AS artifact ON artifact.id = matching.output_artifact_id
             JOIN artifact_revisions AS revision
               ON revision.artifact_id = artifact.id
              AND revision.revision = matching.output_artifact_revision
             WHERE run.job_id = ?1 AND run.job_revision = jobs.revision
               AND matching.status = 'complete'
             ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
            params![job_id.as_str()],
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
            StoreError::WorkflowConflict("match stage has no current completed output".to_owned())
        })?;
    let (id, revision, kind, sha256, head_revision, stale) = row;
    let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
    if kind != ArtifactKind::EvidenceMatches || revision != head_revision || stale != 0 {
        return Err(StoreError::WorkflowConflict(
            "match output is stale or has the wrong artifact kind".to_owned(),
        ));
    }
    Ok(ArtifactReference {
        kind,
        id: EntityId::try_new(id)?,
        revision: Revision::try_new(to_u64(revision)?)?,
        sha256: Sha256Digest::try_new(sha256)?,
    })
}

fn candidate_error(error: CandidateValidationError) -> StoreError {
    match error {
        CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        CandidateValidationError::Semantic(violations) => StoreError::CandidateSemantic(violations),
    }
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}
