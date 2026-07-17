use std::{fs, io::Write, path::Path};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, ArtifactRevisionData, EntityId, Revision,
    SafeRelativePath, Sha256Digest,
};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::{
    BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, io_error, now_utc,
};

pub struct ArtifactService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
    workspace_root: &'a Path,
}

impl<'a> ArtifactService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore, workspace_root: &'a Path) -> Self {
        Self {
            database,
            blobs,
            workspace_root,
        }
    }

    pub fn commit(
        &mut self,
        artifact_id: Option<EntityId>,
        kind: ArtifactKind,
        bytes: &[u8],
        dependencies: &[ArtifactReference],
        actor: ActorKind,
        reason: &str,
    ) -> Result<ArtifactRevisionData, StoreError> {
        if reason.trim().is_empty() {
            return Err(StoreError::Invariant(
                "artifact commit reason cannot be empty".to_owned(),
            ));
        }
        let digest = self.blobs.put_bytes(bytes)?;
        let size = self.blobs.verify(&digest, DEFAULT_MAX_BLOB_BYTES)?;
        let artifact_id = artifact_id.unwrap_or(generate_id()?);
        let created_at = now_utc()?;
        let kind_name = enum_name(kind)?;
        let actor_name = enum_name(actor)?;
        let event_id = generate_id()?;

        let transaction = self.database.immediate_transaction()?;
        let existing: Option<(String, i64)> = transaction
            .query_row(
                "SELECT kind, head_revision FROM artifacts WHERE id = ?1",
                params![artifact_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let revision_number = if let Some((existing_kind, head_revision)) = existing {
            if existing_kind != kind_name {
                return Err(StoreError::Invariant(format!(
                    "artifact kind changed from {existing_kind} to {kind_name}"
                )));
            }
            head_revision
                .checked_add(1)
                .ok_or_else(|| StoreError::Invariant("artifact revision overflow".to_owned()))?
        } else {
            transaction.execute(
                "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
                 VALUES (?1, ?2, 0, 0, ?3)",
                params![artifact_id.as_str(), kind_name, created_at.as_str()],
            )?;
            1
        };

        for dependency in dependencies {
            let actual: Option<String> = transaction
                .query_row(
                    "SELECT sha256 FROM artifact_revisions
                     WHERE artifact_id = ?1 AND revision = ?2",
                    params![dependency.id.as_str(), to_i64(dependency.revision.get())?],
                    |row| row.get(0),
                )
                .optional()?;
            if actual.as_deref() != Some(dependency.sha256.as_str()) {
                return Err(StoreError::DependencyConflict(dependency.id.to_string()));
            }
        }

        transaction.execute(
            "INSERT INTO artifact_revisions(
                artifact_id, revision, sha256, size, actor, reason, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                artifact_id.as_str(),
                revision_number,
                digest.as_str(),
                to_i64(size)?,
                actor_name,
                reason,
                created_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
             VALUES (?1, 'artifact', ?2, ?3, ?4)",
            params![
                digest.as_str(),
                artifact_id.as_str(),
                revision_number,
                created_at.as_str()
            ],
        )?;
        for dependency in dependencies {
            transaction.execute(
                "INSERT INTO artifact_dependencies(
                    artifact_id, revision, depends_on_artifact_id, depends_on_revision,
                    depends_on_sha256
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    artifact_id.as_str(),
                    revision_number,
                    dependency.id.as_str(),
                    to_i64(dependency.revision.get())?,
                    dependency.sha256.as_str()
                ],
            )?;
        }
        transaction.execute(
            "WITH RECURSIVE descendants(id) AS (
                 SELECT DISTINCT artifact_id FROM artifact_dependencies
                 WHERE depends_on_artifact_id = ?1
                 UNION
                 SELECT DISTINCT dependency.artifact_id
                 FROM artifact_dependencies AS dependency
                 JOIN descendants ON dependency.depends_on_artifact_id = descendants.id
             )
             UPDATE artifacts SET stale = 1 WHERE id IN (SELECT id FROM descendants)",
            params![artifact_id.as_str()],
        )?;
        transaction.execute(
            "UPDATE artifacts SET head_revision = ?2, stale = 0 WHERE id = ?1",
            params![artifact_id.as_str(), revision_number],
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'artifact.commit', ?3, ?4, ?5, ?6)",
            params![
                event_id.as_str(),
                actor_name,
                artifact_id.as_str(),
                revision_number,
                reason,
                created_at.as_str()
            ],
        )?;
        transaction.commit()?;
        Ok(ArtifactRevisionData {
            artifact_id,
            revision: Revision::try_new(to_u64(revision_number)?)?,
            sha256: digest,
            created_at,
            stale: false,
        })
    }

    pub fn reference(&self, artifact_id: &EntityId) -> Result<ArtifactReference, StoreError> {
        let row: Option<(String, i64, String)> = self
            .database
            .connection()
            .query_row(
                "SELECT kind, head_revision,
                        (SELECT sha256 FROM artifact_revisions
                         WHERE artifact_id = artifacts.id AND revision = artifacts.head_revision)
                 FROM artifacts WHERE id = ?1",
                params![artifact_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (kind, revision, digest) =
            row.ok_or_else(|| StoreError::ArtifactNotFound(artifact_id.to_string()))?;
        Ok(ArtifactReference {
            kind: serde_json::from_value(serde_json::Value::String(kind))?,
            id: artifact_id.clone(),
            revision: Revision::try_new(to_u64(revision)?)?,
            sha256: Sha256Digest::try_new(digest)?,
        })
    }

    pub fn read(&self, artifact_id: &EntityId, revision: Revision) -> Result<Vec<u8>, StoreError> {
        let digest: Option<String> = self
            .database
            .connection()
            .query_row(
                "SELECT sha256 FROM artifact_revisions WHERE artifact_id = ?1 AND revision = ?2",
                params![artifact_id.as_str(), to_i64(revision.get())?],
                |row| row.get(0),
            )
            .optional()?;
        let digest = digest.ok_or_else(|| StoreError::ArtifactNotFound(artifact_id.to_string()))?;
        self.blobs
            .read_verified(&Sha256Digest::try_new(digest)?, DEFAULT_MAX_BLOB_BYTES)
    }

    pub fn project(
        &mut self,
        artifact_id: &EntityId,
        revision: Revision,
        relative_path: &SafeRelativePath,
    ) -> Result<(), StoreError> {
        if !relative_path
            .as_str()
            .split('/')
            .next()
            .is_some_and(|component| matches!(component, "jobs" | "profile"))
        {
            return Err(StoreError::ProjectionPathRejected);
        }
        let digest: String = self.database.connection().query_row(
            "SELECT sha256 FROM artifact_revisions WHERE artifact_id = ?1 AND revision = ?2",
            params![artifact_id.as_str(), to_i64(revision.get())?],
            |row| row.get(0),
        )?;
        let parsed_digest = Sha256Digest::try_new(&digest)?;
        let bytes = self
            .blobs
            .read_verified(&parsed_digest, DEFAULT_MAX_BLOB_BYTES)?;
        let result = write_projection(self.workspace_root, relative_path, &bytes);
        let (status, last_error) = match &result {
            Ok(()) => ("current", None),
            Err(error) => ("repair-required", Some(error.to_string())),
        };
        let updated_at = now_utc()?;
        let transaction = self.database.immediate_transaction()?;
        transaction.execute(
            "INSERT INTO projection_manifests(
                artifact_id, revision, relative_path, sha256, status, last_error, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(artifact_id, revision, relative_path) DO UPDATE SET
                sha256 = excluded.sha256,
                status = excluded.status,
                last_error = excluded.last_error,
                updated_at = excluded.updated_at",
            params![
                artifact_id.as_str(),
                to_i64(revision.get())?,
                relative_path.as_str(),
                digest,
                status,
                last_error,
                updated_at.as_str()
            ],
        )?;
        transaction.commit()?;
        result
    }

    pub fn repair_projections(&mut self) -> Result<usize, StoreError> {
        let repairs = {
            let mut statement = self.database.connection().prepare(
                "SELECT artifact_id, revision, relative_path FROM projection_manifests
                 WHERE status = 'repair-required' ORDER BY artifact_id, revision, relative_path",
            )?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        for (artifact_id, revision, relative_path) in &repairs {
            self.project(
                &EntityId::try_new(artifact_id)?,
                Revision::try_new(to_u64(*revision)?)?,
                &SafeRelativePath::try_new(relative_path)?,
            )?;
        }
        Ok(repairs.len())
    }
}

fn enum_name<T: Serialize>(value: T) -> Result<String, StoreError> {
    let value = serde_json::to_value(value)?;
    value
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

fn write_projection(
    root: &Path,
    relative_path: &SafeRelativePath,
    bytes: &[u8],
) -> Result<(), StoreError> {
    let destination = root.join(relative_path.as_str());
    let parent = destination
        .parent()
        .ok_or(StoreError::ProjectionPathRejected)?;
    ensure_projection_parent(root, parent)?;
    if let Ok(metadata) = fs::symlink_metadata(&destination)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(StoreError::UnsafePath(destination));
    }
    let temporary = parent.join(format!(".canisend-projection-{}.tmp", generate_id()?));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|source| io_error(&temporary, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error(&temporary, source))?;
    file.sync_all()
        .map_err(|source| io_error(&temporary, source))?;
    if destination.exists() {
        fs::remove_file(&destination).map_err(|source| io_error(&destination, source))?;
    }
    fs::rename(&temporary, &destination).map_err(|source| io_error(&destination, source))
}

fn ensure_projection_parent(root: &Path, parent: &Path) -> Result<(), StoreError> {
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| StoreError::ProjectionPathRejected)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&current) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(StoreError::UnsafePath(current));
            }
        } else {
            fs::create_dir(&current).map_err(|source| io_error(&current, source))?;
        }
    }
    Ok(())
}
