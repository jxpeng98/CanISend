use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, EntityId, JobRecord, PrivacyClassification,
    Revision, Sha256Digest, SourceKind, SourceRecord, UtcTimestamp,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::{BlobStore, Database, StoreError, generate_id, now_utc};

#[derive(Debug, Clone)]
pub struct NewSource {
    pub kind: SourceKind,
    pub original_bytes: Vec<u8>,
    pub normalized_text: String,
    pub source_url: Option<String>,
    pub final_url: Option<String>,
    pub content_type: String,
    pub redirect_chain: Vec<String>,
    pub privacy: PrivacyClassification,
}

pub struct JobService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> JobService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn create(
        &mut self,
        title: &str,
        institution: &str,
        actor: ActorKind,
    ) -> Result<JobRecord, StoreError> {
        let title = validate_label("title", title)?;
        let institution = validate_label("institution", institution)?;
        let job_id = generate_id()?;
        let event_id = generate_id()?;
        let created_at = now_utc()?;
        let actor = enum_name(actor)?;
        let transaction = self.database.immediate_transaction()?;
        transaction.execute(
            "INSERT INTO jobs(id, title, institution, archived, created_at, revision)
             VALUES (?1, ?2, ?3, 0, ?4, 1)",
            params![job_id.as_str(), title, institution, created_at.as_str()],
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'job.create', ?3, 1, 'create job', ?4)",
            params![
                event_id.as_str(),
                actor,
                job_id.as_str(),
                created_at.as_str()
            ],
        )?;
        transaction.commit()?;
        load_job(self.database.connection(), &job_id)
    }

    pub fn list(&self, include_archived: bool) -> Result<Vec<JobRecord>, StoreError> {
        let mut statement = self.database.connection().prepare(if include_archived {
            "SELECT id FROM jobs ORDER BY created_at, id"
        } else {
            "SELECT id FROM jobs WHERE archived = 0 ORDER BY created_at, id"
        })?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| load_job(self.database.connection(), &EntityId::try_new(id)?))
            .collect()
    }

    pub fn get(&self, job_id: &EntityId) -> Result<JobRecord, StoreError> {
        load_job(self.database.connection(), job_id)
    }

    pub fn sources(&self, job_id: &EntityId) -> Result<Vec<SourceRecord>, StoreError> {
        let _ = load_job(self.database.connection(), job_id)?;
        let mut statement = self
            .database
            .connection()
            .prepare("SELECT id FROM sources WHERE job_id = ?1 ORDER BY created_at, id")?;
        let ids = statement
            .query_map(params![job_id.as_str()], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| load_source(self.database.connection(), &EntityId::try_new(id)?))
            .collect()
    }

    pub fn archive(
        &mut self,
        job_id: &EntityId,
        actor: ActorKind,
    ) -> Result<JobRecord, StoreError> {
        let created_at = now_utc()?;
        let event_id = generate_id()?;
        let actor = enum_name(actor)?;
        let transaction = self.database.immediate_transaction()?;
        let changed = transaction.execute(
            "UPDATE jobs SET archived = 1, revision = revision + 1
             WHERE id = ?1 AND archived = 0",
            params![job_id.as_str()],
        )?;
        if changed == 0 {
            let archived: Option<i64> = transaction
                .query_row(
                    "SELECT archived FROM jobs WHERE id = ?1",
                    params![job_id.as_str()],
                    |row| row.get(0),
                )
                .optional()?;
            return match archived {
                Some(_) => Err(StoreError::JobArchived(job_id.to_string())),
                None => Err(StoreError::JobNotFound(job_id.to_string())),
            };
        }
        let revision: i64 = transaction.query_row(
            "SELECT revision FROM jobs WHERE id = ?1",
            params![job_id.as_str()],
            |row| row.get(0),
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'job.archive', ?3, ?4, 'archive job', ?5)",
            params![
                event_id.as_str(),
                actor,
                job_id.as_str(),
                revision,
                created_at.as_str()
            ],
        )?;
        transaction.commit()?;
        load_job(self.database.connection(), job_id)
    }

    pub fn import_source(
        &mut self,
        job_id: &EntityId,
        source: NewSource,
        actor: ActorKind,
    ) -> Result<SourceRecord, StoreError> {
        validate_source(&source)?;
        let original_digest = self.blobs.put_bytes(&source.original_bytes)?;
        let normalized_digest = self.blobs.put_bytes(source.normalized_text.as_bytes())?;
        let source_id = generate_id()?;
        let original_artifact_id = generate_id()?;
        let normalized_artifact_id = generate_id()?;
        let event_id = generate_id()?;
        let created_at = now_utc()?;
        let source_kind = enum_name(source.kind)?;
        let privacy = enum_name(source.privacy)?;
        let actor = enum_name(actor)?;
        let redirect_chain_json = serde_json::to_string(&source.redirect_chain)?;
        let original_size = i64::try_from(source.original_bytes.len())
            .map_err(|_| StoreError::InvalidInput("original source is too large".to_owned()))?;
        let normalized_size = i64::try_from(source.normalized_text.len())
            .map_err(|_| StoreError::InvalidInput("normalized source is too large".to_owned()))?;

        let transaction = self.database.immediate_transaction()?;
        let archived: Option<i64> = transaction
            .query_row(
                "SELECT archived FROM jobs WHERE id = ?1",
                params![job_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        match archived {
            None => return Err(StoreError::JobNotFound(job_id.to_string())),
            Some(1) => return Err(StoreError::JobArchived(job_id.to_string())),
            Some(0) => {}
            Some(_) => {
                return Err(StoreError::Invariant(
                    "job archived flag is not boolean".to_owned(),
                ));
            }
        }

        transaction.execute(
            "INSERT INTO sources(id, job_id, kind, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                source_id.as_str(),
                job_id.as_str(),
                source_kind,
                created_at.as_str()
            ],
        )?;
        insert_artifact(
            &transaction,
            ArtifactInsert {
                id: &original_artifact_id,
                kind: ArtifactKind::SourceOriginal,
                digest: &original_digest,
                size: original_size,
                actor: &actor,
                reason: "import original source",
                created_at: &created_at,
            },
        )?;
        insert_artifact(
            &transaction,
            ArtifactInsert {
                id: &normalized_artifact_id,
                kind: ArtifactKind::SourceNormalizedText,
                digest: &normalized_digest,
                size: normalized_size,
                actor: &actor,
                reason: "normalize imported source",
                created_at: &created_at,
            },
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
            "INSERT INTO source_revisions(
                source_id, revision, sha256, created_at,
                original_artifact_id, original_artifact_revision,
                normalized_artifact_id, normalized_artifact_revision,
                source_url, final_url, content_type, redirect_chain_json, privacy
             ) VALUES (?1, 1, ?2, ?3, ?4, 1, ?5, 1, ?6, ?7, ?8, ?9, ?10)",
            params![
                source_id.as_str(),
                original_digest.as_str(),
                created_at.as_str(),
                original_artifact_id.as_str(),
                normalized_artifact_id.as_str(),
                source.source_url,
                source.final_url,
                source.content_type,
                redirect_chain_json,
                privacy
            ],
        )?;
        transaction.execute(
            "UPDATE jobs SET revision = revision + 1 WHERE id = ?1",
            params![job_id.as_str()],
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'source.import', ?3, 1, 'import source', ?4)",
            params![
                event_id.as_str(),
                actor,
                source_id.as_str(),
                created_at.as_str()
            ],
        )?;
        transaction.commit()?;
        load_source(self.database.connection(), &source_id)
    }
}

struct ArtifactInsert<'a> {
    id: &'a EntityId,
    kind: ArtifactKind,
    digest: &'a Sha256Digest,
    size: i64,
    actor: &'a str,
    reason: &'a str,
    created_at: &'a UtcTimestamp,
}

fn insert_artifact(
    transaction: &rusqlite::Transaction<'_>,
    artifact: ArtifactInsert<'_>,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
         VALUES (?1, ?2, 1, 0, ?3)",
        params![
            artifact.id.as_str(),
            enum_name(artifact.kind)?,
            artifact.created_at.as_str()
        ],
    )?;
    transaction.execute(
        "INSERT INTO artifact_revisions(
            artifact_id, revision, sha256, size, actor, reason, created_at
         ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6)",
        params![
            artifact.id.as_str(),
            artifact.digest.as_str(),
            artifact.size,
            artifact.actor,
            artifact.reason,
            artifact.created_at.as_str()
        ],
    )?;
    transaction.execute(
        "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
         VALUES (?1, 'artifact', ?2, 1, ?3)",
        params![
            artifact.digest.as_str(),
            artifact.id.as_str(),
            artifact.created_at.as_str()
        ],
    )?;
    Ok(())
}

fn load_job(connection: &Connection, job_id: &EntityId) -> Result<JobRecord, StoreError> {
    let row: Option<(String, String, i64, String, i64)> = connection
        .query_row(
            "SELECT title, institution, archived, created_at, revision FROM jobs WHERE id = ?1",
            params![job_id.as_str()],
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
    let (title, institution, archived, created_at, revision) =
        row.ok_or_else(|| StoreError::JobNotFound(job_id.to_string()))?;
    let mut statement =
        connection.prepare("SELECT id FROM sources WHERE job_id = ?1 ORDER BY created_at, id")?;
    let source_ids = statement
        .query_map(params![job_id.as_str()], |row| row.get::<_, String>(0))?
        .map(|value| EntityId::try_new(value?).map_err(StoreError::from))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(JobRecord {
        id: job_id.clone(),
        title,
        institution,
        source_ids,
        created_at: UtcTimestamp::try_new(created_at)?,
        revision: Revision::try_new(to_u64(revision)?)?,
        archived: archived != 0,
    })
}

fn load_source(connection: &Connection, source_id: &EntityId) -> Result<SourceRecord, StoreError> {
    type SourceRow = (
        String,
        String,
        i64,
        String,
        String,
        i64,
        String,
        i64,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
        String,
        String,
    );
    let row: Option<SourceRow> = connection
        .query_row(
            "SELECT s.job_id, s.kind, sr.revision, sr.created_at,
                    sr.original_artifact_id, sr.original_artifact_revision,
                    sr.normalized_artifact_id, sr.normalized_artifact_revision,
                    sr.source_url, sr.final_url, sr.content_type, sr.redirect_chain_json,
                    sr.privacy, original.sha256, normalized.sha256
             FROM sources AS s
             JOIN source_revisions AS sr ON sr.source_id = s.id
             JOIN artifact_revisions AS original
               ON original.artifact_id = sr.original_artifact_id
              AND original.revision = sr.original_artifact_revision
             JOIN artifact_revisions AS normalized
               ON normalized.artifact_id = sr.normalized_artifact_id
              AND normalized.revision = sr.normalized_artifact_revision
             WHERE s.id = ?1 ORDER BY sr.revision DESC LIMIT 1",
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
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                ))
            },
        )
        .optional()?;
    let (
        job_id,
        kind,
        _source_revision,
        created_at,
        original_id,
        original_revision,
        normalized_id,
        normalized_revision,
        source_url,
        final_url,
        content_type,
        redirect_chain_json,
        privacy,
        original_digest,
        normalized_digest,
    ) = row.ok_or_else(|| StoreError::Invariant(format!("source has no revision: {source_id}")))?;
    Ok(SourceRecord {
        id: source_id.clone(),
        job_id: EntityId::try_new(job_id)?,
        kind: serde_json::from_value(serde_json::Value::String(kind))?,
        original: ArtifactReference {
            kind: ArtifactKind::SourceOriginal,
            id: EntityId::try_new(original_id)?,
            revision: Revision::try_new(to_u64(original_revision)?)?,
            sha256: Sha256Digest::try_new(original_digest)?,
        },
        normalized_text: Some(ArtifactReference {
            kind: ArtifactKind::SourceNormalizedText,
            id: EntityId::try_new(normalized_id)?,
            revision: Revision::try_new(to_u64(normalized_revision)?)?,
            sha256: Sha256Digest::try_new(normalized_digest)?,
        }),
        source_url,
        final_url,
        content_type,
        redirect_chain: serde_json::from_str(&redirect_chain_json)?,
        retrieved_at: UtcTimestamp::try_new(created_at)?,
        privacy: serde_json::from_value(serde_json::Value::String(privacy))?,
    })
}

fn validate_label<'a>(name: &str, value: &'a str) -> Result<&'a str, StoreError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 300 || value.contains('\0') {
        return Err(StoreError::InvalidInput(format!(
            "{name} must contain between 1 and 300 UTF-8 bytes"
        )));
    }
    Ok(value)
}

fn validate_source(source: &NewSource) -> Result<(), StoreError> {
    if source.original_bytes.is_empty() || source.normalized_text.trim().is_empty() {
        return Err(StoreError::InvalidInput(
            "source bodies cannot be empty".to_owned(),
        ));
    }
    if source.content_type.trim().is_empty() || source.content_type.len() > 200 {
        return Err(StoreError::InvalidInput(
            "content type must contain between 1 and 200 bytes".to_owned(),
        ));
    }
    if source.redirect_chain.len() > 10 {
        return Err(StoreError::InvalidInput(
            "redirect chain exceeds ten entries".to_owned(),
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

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}
