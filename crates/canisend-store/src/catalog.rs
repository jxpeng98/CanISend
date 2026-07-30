use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    ActorKind, ArtifactReference, EntityId, PrivacyClassification, Revision, Sha256Digest,
    UtcTimestamp,
};
use rusqlite::params;

use crate::{Database, StoreError};

pub const MAX_CONTENT_CATALOG_ENTRIES: usize = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSourceScope {
    Job,
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSourceRole {
    Original,
    Normalized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSourceMetadata {
    pub scope: CatalogSourceScope,
    pub role: CatalogSourceRole,
    pub source_id: EntityId,
    pub source_kind: String,
    pub content_type: String,
    pub locator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSubjectJob {
    pub id: EntityId,
    pub title: String,
    pub institution: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogArtifactMetadata {
    pub artifact: ArtifactReference,
    pub size: u64,
    pub stale: bool,
    pub actor: ActorKind,
    pub reason: String,
    pub created_at: UtcTimestamp,
    pub privacy: PrivacyClassification,
    pub source: Option<CatalogSourceMetadata>,
    pub subject_jobs: Vec<CatalogSubjectJob>,
    pub dependencies: Vec<ArtifactReference>,
    pub current_stage_output: bool,
}

pub struct ContentCatalogService<'a> {
    database: &'a Database,
}

impl<'a> ContentCatalogService<'a> {
    #[must_use]
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<CatalogArtifactMetadata>, StoreError> {
        let source_metadata = self.source_metadata()?;
        let subject_jobs = self.subject_jobs()?;
        let dependencies = self.dependencies()?;
        let current_stage_outputs = self.current_stage_outputs()?;
        let mut privacy_cache = BTreeMap::new();
        let mut statement = self.database.connection().prepare(
            "SELECT artifact.id, artifact.kind, artifact.head_revision, artifact.stale,
                    revision.sha256, revision.size, revision.actor, revision.reason,
                    revision.created_at
             FROM artifacts AS artifact
             JOIN artifact_revisions AS revision
               ON revision.artifact_id = artifact.id
              AND revision.revision = artifact.head_revision
             ORDER BY revision.created_at DESC, artifact.id
             LIMIT ?1",
        )?;
        type Row = (
            String,
            String,
            i64,
            i64,
            String,
            i64,
            String,
            String,
            String,
        );
        let rows = statement
            .query_map(
                params![i64::try_from(MAX_CONTENT_CATALOG_ENTRIES + 1).map_err(|_| {
                    StoreError::Invariant("catalog entry bound does not fit SQLite".to_owned())
                })?],
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
                    ))
                },
            )?
            .collect::<Result<Vec<Row>, _>>()?;
        if rows.len() > MAX_CONTENT_CATALOG_ENTRIES {
            return Err(StoreError::InvalidInput(format!(
                "content catalog exceeds the {MAX_CONTENT_CATALOG_ENTRIES}-entry bound"
            )));
        }

        rows.into_iter()
            .map(
                |(id, kind, revision, stale, sha256, size, actor, reason, created_at)| {
                    let source = source_metadata.get(&id).cloned();
                    let privacy = artifact_privacy(
                        &id,
                        &source_metadata,
                        &dependencies,
                        &mut privacy_cache,
                        &mut BTreeSet::new(),
                    )?;
                    let artifact_id = EntityId::try_new(id.clone())?;
                    Ok(CatalogArtifactMetadata {
                        artifact: ArtifactReference {
                            kind: enum_value(&kind)?,
                            id: artifact_id,
                            revision: Revision::try_new(to_u64(revision)?)?,
                            sha256: Sha256Digest::try_new(sha256)?,
                        },
                        size: to_u64(size)?,
                        stale: stale != 0,
                        actor: enum_value(&actor)?,
                        reason,
                        created_at: UtcTimestamp::try_new(created_at)?,
                        privacy,
                        source: source.map(|(metadata, _)| metadata),
                        subject_jobs: subject_jobs.get(&id).cloned().unwrap_or_default(),
                        dependencies: dependencies.get(&id).cloned().unwrap_or_default(),
                        current_stage_output: current_stage_outputs.contains(&id),
                    })
                },
            )
            .collect()
    }

    fn source_metadata(
        &self,
    ) -> Result<BTreeMap<String, (CatalogSourceMetadata, PrivacyClassification)>, StoreError> {
        let mut result = BTreeMap::new();
        {
            let mut statement = self.database.connection().prepare(
                "WITH latest AS (
                     SELECT source_id, MAX(revision) AS revision
                     FROM source_revisions GROUP BY source_id
                 )
                 SELECT source.id, source.kind, revision.privacy, revision.content_type,
                        revision.source_url, revision.final_url,
                        revision.original_artifact_id, revision.normalized_artifact_id
                 FROM sources AS source
                 JOIN latest ON latest.source_id = source.id
                 JOIN source_revisions AS revision
                   ON revision.source_id = latest.source_id
                  AND revision.revision = latest.revision
                 ORDER BY source.id",
            )?;
            type Row = (
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            );
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                })?
                .collect::<Result<Vec<Row>, _>>()?;
            for (
                source_id,
                source_kind,
                privacy,
                content_type,
                source_url,
                final_url,
                original_artifact_id,
                normalized_artifact_id,
            ) in rows
            {
                let source_id = EntityId::try_new(source_id)?;
                let privacy = enum_value(&privacy)?;
                let locator = final_url.or(source_url);
                let content_type =
                    content_type.unwrap_or_else(|| "application/octet-stream".into());
                if let Some(artifact_id) = original_artifact_id {
                    result.insert(
                        artifact_id,
                        (
                            CatalogSourceMetadata {
                                scope: CatalogSourceScope::Job,
                                role: CatalogSourceRole::Original,
                                source_id: source_id.clone(),
                                source_kind: source_kind.clone(),
                                content_type: content_type.clone(),
                                locator: locator.clone(),
                            },
                            privacy,
                        ),
                    );
                }
                if let Some(artifact_id) = normalized_artifact_id {
                    result.insert(
                        artifact_id,
                        (
                            CatalogSourceMetadata {
                                scope: CatalogSourceScope::Job,
                                role: CatalogSourceRole::Normalized,
                                source_id,
                                source_kind,
                                content_type,
                                locator,
                            },
                            privacy,
                        ),
                    );
                }
            }
        }
        {
            let mut statement = self.database.connection().prepare(
                "WITH latest AS (
                     SELECT source_id, MAX(revision) AS revision
                     FROM profile_source_revisions GROUP BY source_id
                 )
                 SELECT source.id, source.kind, revision.sensitivity, revision.content_type,
                        revision.original_artifact_id, revision.normalized_artifact_id
                 FROM profile_sources AS source
                 JOIN latest ON latest.source_id = source.id
                 JOIN profile_source_revisions AS revision
                   ON revision.source_id = latest.source_id
                  AND revision.revision = latest.revision
                 ORDER BY source.id",
            )?;
            type Row = (String, String, String, String, String, String);
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })?
                .collect::<Result<Vec<Row>, _>>()?;
            for (
                source_id,
                source_kind,
                sensitivity,
                content_type,
                original_artifact_id,
                normalized_artifact_id,
            ) in rows
            {
                let source_id = EntityId::try_new(source_id)?;
                let privacy = enum_value(&sensitivity)?;
                result.insert(
                    original_artifact_id,
                    (
                        CatalogSourceMetadata {
                            scope: CatalogSourceScope::Profile,
                            role: CatalogSourceRole::Original,
                            source_id: source_id.clone(),
                            source_kind: source_kind.clone(),
                            content_type: content_type.clone(),
                            locator: None,
                        },
                        privacy,
                    ),
                );
                result.insert(
                    normalized_artifact_id,
                    (
                        CatalogSourceMetadata {
                            scope: CatalogSourceScope::Profile,
                            role: CatalogSourceRole::Normalized,
                            source_id,
                            source_kind,
                            content_type,
                            locator: None,
                        },
                        privacy,
                    ),
                );
            }
        }
        Ok(result)
    }

    fn subject_jobs(&self) -> Result<BTreeMap<String, Vec<CatalogSubjectJob>>, StoreError> {
        let mut statement = self.database.connection().prepare(
            "WITH RECURSIVE direct_jobs(artifact_id, job_id) AS (
                 SELECT revision.original_artifact_id, source.job_id
                 FROM source_revisions AS revision
                 JOIN sources AS source ON source.id = revision.source_id
                 WHERE revision.original_artifact_id IS NOT NULL
                 UNION
                 SELECT revision.normalized_artifact_id, source.job_id
                 FROM source_revisions AS revision
                 JOIN sources AS source ON source.id = revision.source_id
                 WHERE revision.normalized_artifact_id IS NOT NULL
                 UNION
                 SELECT execution.output_artifact_id, run.job_id
                 FROM stage_executions AS execution
                 JOIN workflow_runs AS run ON run.id = execution.workflow_run_id
                 WHERE execution.output_artifact_id IS NOT NULL AND run.job_id IS NOT NULL
                 UNION
                 SELECT result.artifact_id, task.job_id
                 FROM task_results AS result
                 JOIN tasks AS task ON task.id = result.task_id
                 WHERE task.job_id IS NOT NULL
                 UNION
                 SELECT head.artifact_id, run.job_id
                 FROM document_heads AS head
                 JOIN workflow_runs AS run ON run.id = head.workflow_run_id
                 WHERE run.job_id IS NOT NULL
                 UNION
                 SELECT head.artifact_id, run.job_id
                 FROM export_heads AS head
                 JOIN workflow_runs AS run ON run.id = head.workflow_run_id
                 WHERE run.job_id IS NOT NULL
                 UNION
                 SELECT head.artifact_id, run.job_id
                 FROM render_heads AS head
                 JOIN workflow_runs AS run ON run.id = head.workflow_run_id
                 WHERE run.job_id IS NOT NULL
             ),
             artifact_jobs(artifact_id, job_id) AS (
                 SELECT artifact_id, job_id FROM direct_jobs
                 UNION
                 SELECT dependency.depends_on_artifact_id, artifact_jobs.job_id
                 FROM artifact_dependencies AS dependency
                 JOIN artifact_jobs ON artifact_jobs.artifact_id = dependency.artifact_id
             )
             SELECT DISTINCT artifact_jobs.artifact_id, job.id, job.title,
                    job.institution, job.archived
             FROM artifact_jobs
             JOIN jobs AS job ON job.id = artifact_jobs.job_id
             ORDER BY artifact_jobs.artifact_id, job.created_at, job.id",
        )?;
        type Row = (String, String, String, String, i64);
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<Result<Vec<Row>, _>>()?;
        let mut result = BTreeMap::<String, Vec<CatalogSubjectJob>>::new();
        for (artifact_id, job_id, title, institution, archived) in rows {
            result
                .entry(artifact_id)
                .or_default()
                .push(CatalogSubjectJob {
                    id: EntityId::try_new(job_id)?,
                    title,
                    institution,
                    archived: archived != 0,
                });
        }
        Ok(result)
    }

    fn dependencies(&self) -> Result<BTreeMap<String, Vec<ArtifactReference>>, StoreError> {
        let mut statement = self.database.connection().prepare(
            "SELECT dependency.artifact_id, upstream.kind,
                    dependency.depends_on_artifact_id,
                    dependency.depends_on_revision,
                    dependency.depends_on_sha256
             FROM artifact_dependencies AS dependency
             JOIN artifacts AS owner ON owner.id = dependency.artifact_id
             JOIN artifacts AS upstream ON upstream.id = dependency.depends_on_artifact_id
             WHERE dependency.revision = owner.head_revision
             ORDER BY dependency.artifact_id, dependency.depends_on_artifact_id",
        )?;
        type Row = (String, String, String, i64, String);
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<Result<Vec<Row>, _>>()?;
        let mut result = BTreeMap::<String, Vec<ArtifactReference>>::new();
        for (artifact_id, kind, upstream_id, revision, sha256) in rows {
            result
                .entry(artifact_id)
                .or_default()
                .push(ArtifactReference {
                    kind: enum_value(&kind)?,
                    id: EntityId::try_new(upstream_id)?,
                    revision: Revision::try_new(to_u64(revision)?)?,
                    sha256: Sha256Digest::try_new(sha256)?,
                });
        }
        Ok(result)
    }

    fn current_stage_outputs(&self) -> Result<BTreeSet<String>, StoreError> {
        let mut statement = self.database.connection().prepare(
            "SELECT DISTINCT output_artifact_id
             FROM stage_executions
             WHERE output_artifact_id IS NOT NULL AND status = 'complete'
             ORDER BY output_artifact_id",
        )?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(StoreError::from)
    }
}

fn enum_value<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, StoreError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(StoreError::from)
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative catalog value".to_owned()))
}

fn artifact_privacy(
    artifact_id: &str,
    sources: &BTreeMap<String, (CatalogSourceMetadata, PrivacyClassification)>,
    dependencies: &BTreeMap<String, Vec<ArtifactReference>>,
    cache: &mut BTreeMap<String, PrivacyClassification>,
    visiting: &mut BTreeSet<String>,
) -> Result<PrivacyClassification, StoreError> {
    if let Some(privacy) = cache.get(artifact_id) {
        return Ok(*privacy);
    }
    if !visiting.insert(artifact_id.to_owned()) {
        return Err(StoreError::Invariant(format!(
            "artifact dependency cycle reached while classifying {artifact_id}"
        )));
    }
    let privacy = if let Some((_, privacy)) = sources.get(artifact_id) {
        *privacy
    } else if let Some(upstream) = dependencies.get(artifact_id) {
        upstream
            .iter()
            .try_fold(PrivacyClassification::Public, |current, reference| {
                let candidate = artifact_privacy(
                    reference.id.as_str(),
                    sources,
                    dependencies,
                    cache,
                    visiting,
                )?;
                Ok::<_, StoreError>(more_restrictive_privacy(current, candidate))
            })?
    } else {
        PrivacyClassification::PrivateLocal
    };
    visiting.remove(artifact_id);
    cache.insert(artifact_id.to_owned(), privacy);
    Ok(privacy)
}

fn more_restrictive_privacy(
    left: PrivacyClassification,
    right: PrivacyClassification,
) -> PrivacyClassification {
    if privacy_rank(left) >= privacy_rank(right) {
        left
    } else {
        right
    }
}

const fn privacy_rank(value: PrivacyClassification) -> u8 {
    match value {
        PrivacyClassification::Public => 0,
        PrivacyClassification::PrivateLocal => 1,
        PrivacyClassification::ProviderBound => 2,
        PrivacyClassification::Secret => 3,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{ActorKind, ArtifactKind, PrivacyClassification, SourceKind};

    use super::{CatalogSourceRole, CatalogSourceScope, ContentCatalogService};
    use crate::{ArtifactService, JobService, NewSource, Workspace};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-content-catalog-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn catalog_rebuilds_current_source_metadata_and_relationships() {
        let root = temporary_root();
        let mut workspace = Workspace::init(&root).expect("workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Lecturer in Economics", "University X", ActorKind::User)
            .expect("job");
        let source = JobService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                &job.id,
                NewSource {
                    kind: SourceKind::UserUrl,
                    original_bytes: b"Private advert".to_vec(),
                    normalized_text: "Private advert".to_owned(),
                    source_url: Some("https://example.edu/job".to_owned()),
                    final_url: Some("https://example.edu/job".to_owned()),
                    content_type: "text/plain".to_owned(),
                    redirect_chain: Vec::new(),
                    privacy: PrivacyClassification::Secret,
                },
                ActorKind::User,
            )
            .expect("source");
        let normalized = source.normalized_text.clone().expect("normalized source");
        let workspace_root = workspace.paths.root.clone();
        ArtifactService::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .commit(
                None,
                ArtifactKind::ParsedJob,
                br#"{"title":"Lecturer in Economics"}"#,
                &[normalized],
                ActorKind::System,
                "test derived privacy",
            )
            .expect("derived artifact");

        let catalog = ContentCatalogService::new(&workspace.database)
            .list()
            .expect("catalog");
        assert_eq!(catalog.len(), 3);
        assert!(
            catalog
                .iter()
                .filter(|entry| entry.source.is_some())
                .all(|entry| entry.subject_jobs.first().map(|job| &job.id) == Some(&job.id))
        );
        let normalized = catalog
            .iter()
            .find(|entry| {
                entry
                    .source
                    .as_ref()
                    .is_some_and(|source| source.role == CatalogSourceRole::Normalized)
            })
            .expect("normalized source");
        assert_eq!(
            normalized.source.as_ref().map(|source| source.scope),
            Some(CatalogSourceScope::Job)
        );
        assert_eq!(normalized.dependencies.len(), 1);
        assert_eq!(normalized.privacy, PrivacyClassification::Secret);
        assert_eq!(
            catalog
                .iter()
                .find(|entry| entry.artifact.kind == ArtifactKind::ParsedJob)
                .map(|entry| entry.privacy),
            Some(PrivacyClassification::Secret)
        );

        drop(workspace);
        fs::remove_dir_all(root).expect("remove workspace");
    }
}
