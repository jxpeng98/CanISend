use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, EntityId, PrivacyClassification, ProfileSourceKind,
    ProfileSourceRecord, Revision, Sha256Digest, UtcTimestamp,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;

use crate::{BlobStore, Database, StoreError, generate_id, now_utc};

#[derive(Debug, Clone)]
pub struct NewProfileSource {
    pub kind: ProfileSourceKind,
    pub original_bytes: Vec<u8>,
    pub normalized_text: String,
    pub content_type: String,
    pub sensitivity: PrivacyClassification,
}

pub struct ProfileService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> ProfileService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn import_source(
        &mut self,
        source: NewProfileSource,
        actor: ActorKind,
    ) -> Result<ProfileSourceRecord, StoreError> {
        validate_source(&source)?;
        let original_digest = self.blobs.put_bytes(&source.original_bytes)?;
        let normalized_digest = self.blobs.put_bytes(source.normalized_text.as_bytes())?;
        let source_id = generate_id()?;
        let original_artifact_id = generate_id()?;
        let normalized_artifact_id = generate_id()?;
        let event_id = generate_id()?;
        let created_at = now_utc()?;
        let actor_name = enum_name(actor)?;
        let original_size = to_i64(source.original_bytes.len())?;
        let normalized_size = to_i64(source.normalized_text.len())?;
        let transaction = self.database.immediate_transaction()?;
        transaction.execute(
            "INSERT INTO profile_sources(id, kind, created_at) VALUES (?1, ?2, ?3)",
            params![
                source_id.as_str(),
                enum_name(source.kind)?,
                created_at.as_str()
            ],
        )?;
        insert_artifact(
            &transaction,
            &original_artifact_id,
            ArtifactKind::SourceOriginal,
            &original_digest,
            original_size,
            &actor_name,
            "import original profile source",
            &created_at,
        )?;
        insert_artifact(
            &transaction,
            &normalized_artifact_id,
            ArtifactKind::SourceNormalizedText,
            &normalized_digest,
            normalized_size,
            &actor_name,
            "normalize imported profile source",
            &created_at,
        )?;
        transaction.execute(
            "INSERT INTO artifact_dependencies(
                artifact_id, revision, depends_on_artifact_id, depends_on_revision,
                depends_on_sha256
             ) VALUES (?1, 1, ?2, 1, ?3)",
            params![
                normalized_artifact_id.as_str(),
                original_artifact_id.as_str(),
                original_digest.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO profile_source_revisions(
                source_id, revision, sha256, original_artifact_id,
                original_artifact_revision, normalized_artifact_id,
                normalized_artifact_revision, content_type, sensitivity, created_at
             ) VALUES (?1, 1, ?2, ?3, 1, ?4, 1, ?5, ?6, ?7)",
            params![
                source_id.as_str(),
                original_digest.as_str(),
                original_artifact_id.as_str(),
                normalized_artifact_id.as_str(),
                source.content_type,
                enum_name(source.sensitivity)?,
                created_at.as_str()
            ],
        )?;
        transaction.execute(
            "UPDATE workspace_metadata SET profile_revision = profile_revision + 1
             WHERE singleton = 1",
            [],
        )?;
        invalidate_evidence_workflows(&transaction, &created_at)?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'profile.source.import', ?3, 1,
                       'import profile evidence source', ?4)",
            params![
                event_id.as_str(),
                actor_name,
                source_id.as_str(),
                created_at.as_str()
            ],
        )?;
        transaction.commit()?;
        load_source(self.database.connection(), &source_id)
    }

    pub fn list_sources(&self) -> Result<Vec<ProfileSourceRecord>, StoreError> {
        let mut statement = self
            .database
            .connection()
            .prepare("SELECT id FROM profile_sources ORDER BY created_at, id")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| load_source(self.database.connection(), &EntityId::try_new(id)?))
            .collect()
    }

    pub fn get_source(&self, source_id: &EntityId) -> Result<ProfileSourceRecord, StoreError> {
        load_source(self.database.connection(), source_id)
    }

    pub fn revision(&self) -> Result<u64, StoreError> {
        let revision: i64 = self.database.connection().query_row(
            "SELECT profile_revision FROM workspace_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        to_u64(revision)
    }
}

fn invalidate_evidence_workflows(
    transaction: &Transaction<'_>,
    updated_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (SELECT artifact_id FROM render_heads)",
        [],
    )?;
    transaction.execute("DELETE FROM render_heads", [])?;
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (SELECT artifact_id FROM export_heads)",
        [],
    )?;
    transaction.execute("DELETE FROM export_heads", [])?;
    transaction.execute("DELETE FROM package_heads", [])?;
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT artifact_id FROM review_heads
         )",
        [],
    )?;
    transaction.execute("DELETE FROM review_heads", [])?;
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT artifact_id FROM document_heads
         )",
        [],
    )?;
    transaction.execute("DELETE FROM document_heads", [])?;
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT output_artifact_id FROM stage_executions
             WHERE stage IN ('evidence', 'match', 'plan', 'draft', 'review', 'package', 'render')
               AND output_artifact_id IS NOT NULL
         )",
        [],
    )?;
    transaction.execute(
        "UPDATE tasks SET status = 'stale' WHERE status = 'prepared' AND stage_execution_id IN (
             SELECT id FROM stage_executions
             WHERE stage IN ('evidence', 'match', 'plan', 'draft', 'review', 'package', 'render')
         )",
        [],
    )?;
    transaction.execute(
        "UPDATE stage_executions
         SET status = 'ready', execution_mode = NULL, output_artifact_id = NULL,
             output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
             updated_at = ?1
         WHERE stage = 'evidence'",
        params![updated_at.as_str()],
    )?;
    transaction.execute(
        "UPDATE stage_executions
         SET status = CASE WHEN status = 'blocked' THEN 'blocked' ELSE 'stale' END,
             execution_mode = NULL, output_artifact_id = NULL,
             output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
             updated_at = ?1
         WHERE stage IN ('match', 'plan', 'draft', 'review', 'package', 'render')",
        params![updated_at.as_str()],
    )?;
    transaction.execute("UPDATE workflow_runs SET status = 'active'", [])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_artifact(
    transaction: &Transaction<'_>,
    id: &EntityId,
    kind: ArtifactKind,
    digest: &Sha256Digest,
    size: i64,
    actor: &str,
    reason: &str,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
         VALUES (?1, ?2, 1, 0, ?3)",
        params![id.as_str(), enum_name(kind)?, created_at.as_str()],
    )?;
    transaction.execute(
        "INSERT INTO artifact_revisions(
            artifact_id, revision, sha256, size, actor, reason, created_at
         ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id.as_str(),
            digest.as_str(),
            size,
            actor,
            reason,
            created_at.as_str()
        ],
    )?;
    transaction.execute(
        "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
         VALUES (?1, 'artifact', ?2, 1, ?3)",
        params![digest.as_str(), id.as_str(), created_at.as_str()],
    )?;
    Ok(())
}

fn load_source(
    connection: &Connection,
    source_id: &EntityId,
) -> Result<ProfileSourceRecord, StoreError> {
    type Row = (
        String,
        i64,
        String,
        String,
        i64,
        String,
        i64,
        String,
        String,
        String,
        String,
    );
    let row: Row = connection
        .query_row(
            "SELECT source.kind, revision.revision, revision.created_at,
                    revision.original_artifact_id, revision.original_artifact_revision,
                    revision.normalized_artifact_id, revision.normalized_artifact_revision,
                    revision.content_type, revision.sensitivity,
                    original.sha256, normalized.sha256
             FROM profile_sources AS source
             JOIN profile_source_revisions AS revision ON revision.source_id = source.id
             JOIN artifact_revisions AS original
               ON original.artifact_id = revision.original_artifact_id
              AND original.revision = revision.original_artifact_revision
             JOIN artifact_revisions AS normalized
               ON normalized.artifact_id = revision.normalized_artifact_id
              AND normalized.revision = revision.normalized_artifact_revision
             WHERE source.id = ?1 ORDER BY revision.revision DESC LIMIT 1",
            params![source_id.as_str()],
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
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| StoreError::ProfileSourceNotFound(source_id.to_string()))?;
    let (
        kind,
        revision,
        created_at,
        original_id,
        original_revision,
        normalized_id,
        normalized_revision,
        content_type,
        sensitivity,
        original_sha256,
        normalized_sha256,
    ) = row;
    Ok(ProfileSourceRecord {
        id: source_id.clone(),
        kind: serde_json::from_value(serde_json::Value::String(kind))?,
        original: ArtifactReference {
            kind: ArtifactKind::SourceOriginal,
            id: EntityId::try_new(original_id)?,
            revision: Revision::try_new(to_u64(original_revision)?)?,
            sha256: Sha256Digest::try_new(original_sha256)?,
        },
        normalized_text: ArtifactReference {
            kind: ArtifactKind::SourceNormalizedText,
            id: EntityId::try_new(normalized_id)?,
            revision: Revision::try_new(to_u64(normalized_revision)?)?,
            sha256: Sha256Digest::try_new(normalized_sha256)?,
        },
        content_type,
        sensitivity: serde_json::from_value(serde_json::Value::String(sensitivity))?,
        created_at: UtcTimestamp::try_new(created_at)?,
        revision: Revision::try_new(to_u64(revision)?)?,
    })
}

fn validate_source(source: &NewProfileSource) -> Result<(), StoreError> {
    if source.original_bytes.is_empty() || source.normalized_text.trim().is_empty() {
        return Err(StoreError::InvalidInput(
            "profile source bodies cannot be empty".to_owned(),
        ));
    }
    if source.content_type.trim().is_empty() || source.content_type.len() > 200 {
        return Err(StoreError::InvalidInput(
            "content type must contain between 1 and 200 bytes".to_owned(),
        ));
    }
    Ok(())
}

fn enum_name<T: Serialize>(value: T) -> Result<String, StoreError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| StoreError::Invariant("enum did not serialize as a string".to_owned()))
}

fn to_i64(value: usize) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::InvalidInput("profile source is too large".into()))
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}
