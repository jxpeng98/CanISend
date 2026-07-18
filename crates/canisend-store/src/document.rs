use canisend_contracts::{
    ArtifactKind, ArtifactReference, CandidateValidationError, DocumentKind, DocumentRecord,
    DocumentSetRecord, EntityId, Revision, Sha256Digest, validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::{BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError};

pub struct DocumentService<'a> {
    database: &'a Database,
    blobs: &'a BlobStore,
}

impl<'a> DocumentService<'a> {
    #[must_use]
    pub fn new(database: &'a Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn list(&self, job_id: &EntityId) -> Result<Vec<DocumentRecord>, StoreError> {
        load_current_references(self.database.connection(), job_id)?
            .into_iter()
            .map(|reference| load_document(self.blobs, &reference))
            .collect()
    }

    pub fn current(
        &self,
        job_id: &EntityId,
        kind: DocumentKind,
    ) -> Result<DocumentRecord, StoreError> {
        self.list(job_id)?
            .into_iter()
            .find(|document| document.kind == kind)
            .ok_or_else(|| {
                StoreError::WorkflowConflict(format!(
                    "{} has no current {} draft",
                    job_id,
                    document_kind_name(kind)
                ))
            })
    }

    pub fn set(&self, job_id: &EntityId) -> Result<DocumentSetRecord, StoreError> {
        type Row = (String, i64, String, String);
        let row: Option<Row> = self
            .database
            .connection()
            .query_row(
                "SELECT artifact.id, execution.output_artifact_revision,
                        artifact.kind, revision.sha256
                 FROM workflow_runs AS run
                 JOIN stage_executions AS execution ON execution.workflow_run_id = run.id
                 JOIN artifacts AS artifact ON artifact.id = execution.output_artifact_id
                 JOIN artifact_revisions AS revision
                   ON revision.artifact_id = execution.output_artifact_id
                  AND revision.revision = execution.output_artifact_revision
                 WHERE run.id = (
                     SELECT id FROM workflow_runs WHERE job_id = ?1
                     ORDER BY created_at DESC, id DESC LIMIT 1
                 )
                   AND execution.stage = 'draft' AND execution.status = 'complete'
                   AND artifact.kind = 'document-set' AND artifact.stale = 0",
                params![job_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let (id, revision, kind, sha256) = row.ok_or_else(|| {
            StoreError::WorkflowConflict(format!(
                "{} does not have a complete current document set",
                job_id
            ))
        })?;
        let reference = ArtifactReference {
            kind: serde_json::from_value(Value::String(kind))?,
            id: EntityId::try_new(id)?,
            revision: Revision::try_new(to_u64(revision)?)?,
            sha256: Sha256Digest::try_new(sha256)?,
        };
        let set = load_document_set(self.blobs, &reference)?;
        if set.job_id != *job_id {
            return Err(StoreError::Invariant(
                "document set belongs to a different job".to_owned(),
            ));
        }
        let current = load_current_references(self.database.connection(), job_id)?;
        if set.documents != current {
            return Err(StoreError::DependencyConflict(
                "document set does not contain every exact current document head".to_owned(),
            ));
        }
        Ok(set)
    }
}

fn load_current_references(
    connection: &Connection,
    job_id: &EntityId,
) -> Result<Vec<ArtifactReference>, StoreError> {
    type Row = (String, i64, String, String);
    let mut statement = connection.prepare(
        "SELECT artifact.id, head.artifact_revision, artifact.kind, revision.sha256
         FROM workflow_runs AS run
         JOIN document_heads AS head ON head.workflow_run_id = run.id
         JOIN application_plan_heads AS plan ON plan.workflow_run_id = run.id
         JOIN artifacts AS artifact ON artifact.id = head.artifact_id
         JOIN artifact_revisions AS revision
           ON revision.artifact_id = head.artifact_id
          AND revision.revision = head.artifact_revision
         WHERE run.id = (
             SELECT id FROM workflow_runs WHERE job_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT 1
         )
           AND head.plan_artifact_id = plan.artifact_id
           AND head.plan_artifact_revision = plan.artifact_revision
           AND artifact.stale = 0 AND artifact.head_revision = head.artifact_revision
         ORDER BY CASE head.kind
             WHEN 'cover-letter' THEN 1
             WHEN 'research-statement' THEN 2
             WHEN 'teaching-statement' THEN 3
             WHEN 'cv' THEN 4
             ELSE 5 END",
    )?;
    let rows = statement
        .query_map(params![job_id.as_str()], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<Vec<Row>, _>>()?;
    rows.into_iter()
        .map(|(id, revision, kind, sha256)| {
            let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
            Ok(ArtifactReference {
                kind,
                id: EntityId::try_new(id)?,
                revision: Revision::try_new(to_u64(revision)?)?,
                sha256: Sha256Digest::try_new(sha256)?,
            })
        })
        .collect()
}

fn load_document(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<DocumentRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
}

fn load_document_set(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<DocumentSetRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
}

fn candidate_error(error: CandidateValidationError) -> StoreError {
    match error {
        CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        CandidateValidationError::Semantic(violations) => StoreError::CandidateSemantic(violations),
    }
}

fn document_kind_name(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::CoverLetter => "cover-letter",
        DocumentKind::ResearchStatement => "research-statement",
        DocumentKind::TeachingStatement => "teaching-statement",
        DocumentKind::Cv => "cv",
    }
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}
