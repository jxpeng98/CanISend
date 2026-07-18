use std::{collections::BTreeSet, fs, io::Write, path::Path};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, CandidateValidationError, CitationTarget,
    DocumentKind, DocumentRecord, EntityId, PackageExportManifestRecord, PackageManifestRecord,
    ProjectionEditStatus, ProjectionKind, ProjectionReconcileAction, ProjectionReconcileRecord,
    ProjectionRecord, ReadinessState, Revision, SafeRelativePath, Sha256Digest,
    validate_external_candidate,
};
use canisend_io::{TypstProjectionError, project_document_typst};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    ArtifactService, BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, PackageService, StoreError,
    artifact::{digest_file, write_projection},
    generate_id, io_error, now_utc,
};

pub struct ProjectionService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
    workspace_root: &'a Path,
}

impl<'a> ProjectionService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore, workspace_root: &'a Path) -> Self {
        Self {
            database,
            blobs,
            workspace_root,
        }
    }

    pub fn export(
        &mut self,
        job_id: &EntityId,
        destination: &SafeRelativePath,
    ) -> Result<(ArtifactReference, PackageExportManifestRecord), StoreError> {
        ensure_job_destination(job_id, destination)?;
        let (package_artifact, package) =
            PackageService::new(self.database, self.blobs).current_with_reference(job_id)?;
        if !matches!(
            package.readiness.state,
            ReadinessState::ReadyToExport | ReadinessState::Exported
        ) {
            return Err(StoreError::WorkflowConflict(format!(
                "package readiness is {:?}, not ready to export",
                package.readiness.state
            )));
        }
        let documents = load_documents(self.blobs, &package)?;
        let mut generated = Vec::with_capacity(documents.len() * 3 + 1);
        for (reference, document) in &documents {
            let slug = document_kind_slug(document.kind);
            generated.push(GeneratedProjection::new(
                reference.clone(),
                join_projection_path(destination, &format!("{slug}.md"))?,
                ProjectionKind::Markdown,
                markdown_projection(reference, document)?.into_bytes(),
            )?);
            generated.push(GeneratedProjection::new(
                reference.clone(),
                join_projection_path(destination, &format!("{slug}.json"))?,
                ProjectionKind::StructuredJson,
                pretty_json_bytes(&serde_json::to_value(document)?)?,
            )?);
            generated.push(GeneratedProjection::new(
                reference.clone(),
                join_projection_path(destination, &format!("{slug}.typ"))?,
                ProjectionKind::TypstSource,
                project_document_typst(reference, document)
                    .map_err(typst_projection_error)?
                    .into_bytes(),
            )?);
        }
        generated.push(GeneratedProjection::new(
            package_artifact.clone(),
            join_projection_path(destination, "package-manifest.json")?,
            ProjectionKind::PackageManifestJson,
            pretty_json_bytes(&serde_json::to_value(&package)?)?,
        )?);

        verify_package_current(self.database.connection(), job_id, &package_artifact)?;
        for projection in &generated {
            preflight_projection(
                self.database.connection(),
                self.workspace_root,
                &projection.relative_path,
            )?;
        }
        let exported_at = now_utc()?;
        let mut projections = Vec::with_capacity(generated.len());
        for projection in &generated {
            let result = write_projection(
                self.workspace_root,
                &projection.relative_path,
                &projection.bytes,
            );
            let (status, observed, last_error) = match &result {
                Ok(()) => (
                    ProjectionEditStatus::Current,
                    Some(projection.generated_sha256.clone()),
                    None,
                ),
                Err(error) => (
                    ProjectionEditStatus::RepairRequired,
                    None,
                    Some(error.to_string()),
                ),
            };
            record_projection(
                self.database.connection(),
                projection,
                status,
                observed.as_ref(),
                last_error.as_deref(),
                &exported_at,
            )?;
            result?;
            projections.push(ProjectionRecord {
                source_artifact: projection.source_artifact.clone(),
                relative_path: projection.relative_path.clone(),
                kind: projection.kind,
                generated_sha256: projection.generated_sha256.clone(),
                observed_sha256: observed,
                edit_status: status,
                updated_at: exported_at.clone(),
            });
        }
        verify_package_current(self.database.connection(), job_id, &package_artifact)?;
        let receipt = PackageExportManifestRecord {
            id: generate_id()?,
            job_id: job_id.clone(),
            package_artifact: package_artifact.clone(),
            projections,
            exported_at: exported_at.clone(),
            submission_performed: false,
            revision: Revision::try_new(1)?,
        };
        validate_external_candidate::<PackageExportManifestRecord>(&serde_json::to_value(
            &receipt,
        )?)
        .map_err(candidate_error)?;
        let bytes = canonical_json_bytes(&serde_json::to_value(&receipt)?)?;
        let dependencies = std::iter::once(package_artifact.clone())
            .chain(package.documents.iter().cloned())
            .collect::<Vec<_>>();
        let artifact = ArtifactService::new(self.database, self.blobs, self.workspace_root)
            .commit(
                None,
                ArtifactKind::ExportManifest,
                &bytes,
                &dependencies,
                ActorKind::System,
                "record structured application material projections",
            )?;
        let artifact_reference = ArtifactReference {
            kind: ArtifactKind::ExportManifest,
            id: artifact.artifact_id,
            revision: artifact.revision,
            sha256: artifact.sha256,
        };
        let transaction = self.database.immediate_transaction()?;
        verify_package_current(&transaction, job_id, &package_artifact)?;
        let run_id: String = transaction.query_row(
            "SELECT workflow_run_id FROM package_heads
             WHERE artifact_id = ?1 AND artifact_revision = ?2",
            params![
                package_artifact.id.as_str(),
                to_i64(package_artifact.revision.get())?
            ],
            |row| row.get(0),
        )?;
        transaction.execute(
            "UPDATE artifacts SET stale = 1 WHERE id IN (
                 SELECT artifact_id FROM export_heads WHERE workflow_run_id = ?1
             )",
            params![&run_id],
        )?;
        transaction.execute(
            "INSERT INTO export_heads(
                workflow_run_id, package_artifact_id, package_artifact_revision,
                artifact_id, artifact_revision, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(workflow_run_id) DO UPDATE SET
                package_artifact_id = excluded.package_artifact_id,
                package_artifact_revision = excluded.package_artifact_revision,
                artifact_id = excluded.artifact_id,
                artifact_revision = excluded.artifact_revision,
                updated_at = excluded.updated_at",
            params![
                &run_id,
                package_artifact.id.as_str(),
                to_i64(package_artifact.revision.get())?,
                artifact_reference.id.as_str(),
                to_i64(artifact_reference.revision.get())?,
                exported_at.as_str()
            ],
        )?;
        transaction.commit()?;
        Ok((artifact_reference, receipt))
    }

    pub fn current(
        &mut self,
        job_id: &EntityId,
    ) -> Result<(ArtifactReference, PackageExportManifestRecord), StoreError> {
        let (package_artifact, _) =
            PackageService::new(self.database, self.blobs).current_with_reference(job_id)?;
        type Row = (String, i64, String);
        let row: Option<Row> = self
            .database
            .connection()
            .query_row(
                "SELECT head.artifact_id, head.artifact_revision, revision.sha256
                 FROM export_heads AS head
                 JOIN package_heads AS package ON package.workflow_run_id = head.workflow_run_id
                  AND package.artifact_id = head.package_artifact_id
                  AND package.artifact_revision = head.package_artifact_revision
                 JOIN stage_executions AS stage ON stage.workflow_run_id = head.workflow_run_id
                  AND stage.stage = 'package' AND stage.status = 'complete'
                  AND stage.output_artifact_id = package.artifact_id
                  AND stage.output_artifact_revision = package.artifact_revision
                 JOIN artifacts AS artifact ON artifact.id = head.artifact_id
                  AND artifact.head_revision = head.artifact_revision AND artifact.stale = 0
                 JOIN artifact_revisions AS revision ON revision.artifact_id = head.artifact_id
                  AND revision.revision = head.artifact_revision
                 WHERE package.artifact_id = ?1 AND package.artifact_revision = ?2",
                params![
                    package_artifact.id.as_str(),
                    to_i64(package_artifact.revision.get())?
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (id, revision, sha256) = row.ok_or_else(|| {
            StoreError::WorkflowConflict("no current package export receipt".to_owned())
        })?;
        let reference = ArtifactReference {
            kind: ArtifactKind::ExportManifest,
            id: EntityId::try_new(id)?,
            revision: Revision::try_new(to_u64(revision)?)?,
            sha256: Sha256Digest::try_new(sha256)?,
        };
        let receipt = load_record::<PackageExportManifestRecord>(self.blobs, &reference)?;
        if receipt.package_artifact != package_artifact || receipt.job_id != *job_id {
            return Err(StoreError::DependencyConflict(
                "export receipt does not bind the current package".to_owned(),
            ));
        }
        Ok((reference, receipt))
    }

    pub fn reconcile(
        &mut self,
        job_id: &EntityId,
    ) -> Result<Vec<ProjectionReconcileRecord>, StoreError> {
        let (package_artifact, package) =
            PackageService::new(self.database, self.blobs).current_with_reference(job_id)?;
        let allowed = allowed_artifacts(&package_artifact, &package);
        let rows = load_projection_rows(self.database.connection(), &allowed)?;
        let reconciled_at = now_utc()?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let (status, observed, last_error) = observe_projection(
                self.workspace_root,
                &row.relative_path,
                &row.generated_sha256,
            );
            update_observation(
                self.database.connection(),
                &row.relative_path,
                status,
                observed.as_ref(),
                last_error.as_deref(),
                &reconciled_at,
            )?;
            let projection = row.into_record(status, observed, reconciled_at.clone());
            let record = ProjectionReconcileRecord {
                job_id: job_id.clone(),
                package_artifact: package_artifact.clone(),
                projection,
                action: ProjectionReconcileAction::Inspect,
                preserved_copy_path: None,
                preserved_copy_sha256: None,
                authoritative_changed: false,
                reconciled_at: reconciled_at.clone(),
            };
            validate_external_candidate::<ProjectionReconcileRecord>(&serde_json::to_value(
                &record,
            )?)
            .map_err(candidate_error)?;
            records.push(record);
        }
        Ok(records)
    }

    pub fn replace(
        &mut self,
        job_id: &EntityId,
        relative_path: &SafeRelativePath,
    ) -> Result<ProjectionReconcileRecord, StoreError> {
        self.restore_projection(job_id, relative_path, None)
    }

    pub fn copy_as_new(
        &mut self,
        job_id: &EntityId,
        relative_path: &SafeRelativePath,
        destination: &SafeRelativePath,
    ) -> Result<ProjectionReconcileRecord, StoreError> {
        ensure_job_destination(job_id, destination)?;
        if relative_path == destination {
            return Err(StoreError::InvalidInput(
                "copy destination must differ from the managed projection".to_owned(),
            ));
        }
        self.restore_projection(job_id, relative_path, Some(destination))
    }

    fn restore_projection(
        &mut self,
        job_id: &EntityId,
        relative_path: &SafeRelativePath,
        copy_destination: Option<&SafeRelativePath>,
    ) -> Result<ProjectionReconcileRecord, StoreError> {
        let (package_artifact, package) =
            PackageService::new(self.database, self.blobs).current_with_reference(job_id)?;
        let allowed = allowed_artifacts(&package_artifact, &package);
        let row = load_projection_row(self.database.connection(), relative_path)?
            .ok_or_else(|| StoreError::ProjectionNotFound(relative_path.to_string()))?;
        if !allowed.contains(&(row.source_artifact.id.clone(), row.source_artifact.revision)) {
            return Err(StoreError::DependencyConflict(
                "projection is not part of the current package".to_owned(),
            ));
        }
        let (status, observed, _) =
            observe_projection(self.workspace_root, relative_path, &row.generated_sha256);
        let mut preserved_copy_sha256 = None;
        if let Some(destination) = copy_destination {
            if status != ProjectionEditStatus::Edited {
                return Err(StoreError::WorkflowConflict(
                    "copy-as-new requires an edited managed projection".to_owned(),
                ));
            }
            let bytes = read_safe_projection(self.workspace_root, relative_path)?;
            let digest = observed.ok_or_else(|| {
                StoreError::Invariant("edited projection has no observed digest".to_owned())
            })?;
            write_new_user_copy(self.workspace_root, destination, &bytes)?;
            preserved_copy_sha256 = Some(digest);
        }
        let bytes = generate_from_row(self.blobs, &row)?;
        write_projection(self.workspace_root, relative_path, &bytes)?;
        let generated = digest_bytes(&bytes)?;
        if generated != row.generated_sha256 {
            return Err(StoreError::DependencyConflict(
                "projection recipe no longer matches its recorded generated digest".to_owned(),
            ));
        }
        let reconciled_at = now_utc()?;
        update_observation(
            self.database.connection(),
            relative_path,
            ProjectionEditStatus::Current,
            Some(&generated),
            None,
            &reconciled_at,
        )?;
        let projection = row.into_record(
            ProjectionEditStatus::Current,
            Some(generated),
            reconciled_at.clone(),
        );
        let record = ProjectionReconcileRecord {
            job_id: job_id.clone(),
            package_artifact,
            projection,
            action: if copy_destination.is_some() {
                ProjectionReconcileAction::CopyAsNew
            } else {
                ProjectionReconcileAction::Replace
            },
            preserved_copy_path: copy_destination.cloned(),
            preserved_copy_sha256,
            authoritative_changed: false,
            reconciled_at,
        };
        validate_external_candidate::<ProjectionReconcileRecord>(&serde_json::to_value(&record)?)
            .map_err(candidate_error)?;
        Ok(record)
    }
}

struct GeneratedProjection {
    source_artifact: ArtifactReference,
    relative_path: SafeRelativePath,
    kind: ProjectionKind,
    generated_sha256: Sha256Digest,
    bytes: Vec<u8>,
}

impl GeneratedProjection {
    fn new(
        source_artifact: ArtifactReference,
        relative_path: SafeRelativePath,
        kind: ProjectionKind,
        bytes: Vec<u8>,
    ) -> Result<Self, StoreError> {
        let generated_sha256 = digest_bytes(&bytes)?;
        Ok(Self {
            source_artifact,
            relative_path,
            kind,
            generated_sha256,
            bytes,
        })
    }
}

struct ProjectionRow {
    source_artifact: ArtifactReference,
    relative_path: SafeRelativePath,
    kind: ProjectionKind,
    generated_sha256: Sha256Digest,
}

impl ProjectionRow {
    fn into_record(
        self,
        edit_status: ProjectionEditStatus,
        observed_sha256: Option<Sha256Digest>,
        updated_at: canisend_contracts::UtcTimestamp,
    ) -> ProjectionRecord {
        ProjectionRecord {
            source_artifact: self.source_artifact,
            relative_path: self.relative_path,
            kind: self.kind,
            generated_sha256: self.generated_sha256,
            observed_sha256,
            edit_status,
            updated_at,
        }
    }
}

fn load_documents(
    blobs: &BlobStore,
    package: &PackageManifestRecord,
) -> Result<Vec<(ArtifactReference, DocumentRecord)>, StoreError> {
    package
        .documents
        .iter()
        .map(|reference| {
            Ok((
                reference.clone(),
                load_record::<DocumentRecord>(blobs, reference)?,
            ))
        })
        .collect()
}

fn markdown_projection(
    source: &ArtifactReference,
    document: &DocumentRecord,
) -> Result<String, StoreError> {
    let mut output = format!(
        "---\ncanisend-document-id: {}\ncanisend-document-revision: {}\ncanisend-source-artifact: {}@{}\ncanisend-source-sha256: {}\n---\n\n# {}\n",
        document.id,
        document.revision.get(),
        source.id,
        source.revision.get(),
        source.sha256,
        one_line(&document.title)
    );
    for section in &document.sections {
        if let Some(heading) = &section.heading {
            output.push_str("\n## ");
            output.push_str(&one_line(heading));
            output.push('\n');
        }
        output.push('\n');
        output.push_str(section.body.trim());
        output.push('\n');
        for claim in &section.claims {
            let citations = claim
                .citations
                .iter()
                .map(|citation| match &citation.target {
                    CitationTarget::Evidence { evidence } => {
                        format!("evidence:{}@{}", evidence.id, evidence.revision.get())
                    }
                    CitationTarget::Criterion { criterion } => {
                        format!("criterion:{}@{}", criterion.id, criterion.revision.get())
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            output.push_str(&format!(
                "<!-- canisend-claim id={} revision={} classification={} citations={} -->\n",
                claim.id,
                claim.revision.get(),
                enum_name(claim.classification)?,
                citations
            ));
        }
    }
    for placeholder in &document.placeholders {
        output.push_str(&format!(
            "<!-- canisend-placeholder id={} revision={} required={} resolved={} -->\n",
            placeholder.id,
            placeholder.revision.get(),
            placeholder.required,
            placeholder.resolution.is_some()
        ));
    }
    Ok(output)
}

fn generate_from_row(blobs: &BlobStore, row: &ProjectionRow) -> Result<Vec<u8>, StoreError> {
    match row.kind {
        ProjectionKind::Markdown => {
            let document = load_record::<DocumentRecord>(blobs, &row.source_artifact)?;
            Ok(markdown_projection(&row.source_artifact, &document)?.into_bytes())
        }
        ProjectionKind::StructuredJson => {
            let document = load_record::<DocumentRecord>(blobs, &row.source_artifact)?;
            pretty_json_bytes(&serde_json::to_value(document)?)
        }
        ProjectionKind::TypstSource => {
            let document = load_record::<DocumentRecord>(blobs, &row.source_artifact)?;
            Ok(project_document_typst(&row.source_artifact, &document)
                .map_err(typst_projection_error)?
                .into_bytes())
        }
        ProjectionKind::PackageManifestJson => {
            let package = load_record::<PackageManifestRecord>(blobs, &row.source_artifact)?;
            pretty_json_bytes(&serde_json::to_value(package)?)
        }
    }
}

fn record_projection(
    connection: &Connection,
    projection: &GeneratedProjection,
    status: ProjectionEditStatus,
    observed: Option<&Sha256Digest>,
    last_error: Option<&str>,
    updated_at: &canisend_contracts::UtcTimestamp,
) -> Result<(), StoreError> {
    connection.execute(
        "INSERT INTO projection_manifests(
            artifact_id, revision, relative_path, sha256, projection_kind,
            generated_sha256, observed_sha256, status, last_error, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(relative_path) DO UPDATE SET
            artifact_id = excluded.artifact_id,
            revision = excluded.revision,
            sha256 = excluded.sha256,
            projection_kind = excluded.projection_kind,
            generated_sha256 = excluded.generated_sha256,
            observed_sha256 = excluded.observed_sha256,
            status = excluded.status,
            last_error = excluded.last_error,
            updated_at = excluded.updated_at",
        params![
            projection.source_artifact.id.as_str(),
            to_i64(projection.source_artifact.revision.get())?,
            projection.relative_path.as_str(),
            projection.source_artifact.sha256.as_str(),
            enum_name(projection.kind)?,
            projection.generated_sha256.as_str(),
            observed.map(Sha256Digest::as_str),
            enum_name(status)?,
            last_error,
            updated_at.as_str()
        ],
    )?;
    Ok(())
}

fn preflight_projection(
    connection: &Connection,
    workspace_root: &Path,
    relative_path: &SafeRelativePath,
) -> Result<(), StoreError> {
    let managed = load_projection_row(connection, relative_path)?;
    let destination = workspace_root.join(relative_path.as_str());
    match (managed, fs::symlink_metadata(&destination)) {
        (None, Ok(_)) => Err(StoreError::ProjectionUnmanagedConflict(
            relative_path.to_string(),
        )),
        (Some(row), Ok(metadata)) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(StoreError::UnsafePath(destination));
            }
            let observed = digest_file(&destination)?;
            if observed != row.generated_sha256 {
                let updated_at = now_utc()?;
                update_observation(
                    connection,
                    relative_path,
                    ProjectionEditStatus::Edited,
                    Some(&observed),
                    None,
                    &updated_at,
                )?;
                return Err(StoreError::ProjectionEdited(relative_path.to_string()));
            }
            Ok(())
        }
        (_, Err(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        (_, Err(source)) => Err(io_error(destination, source)),
    }
}

fn load_projection_rows(
    connection: &Connection,
    allowed: &BTreeSet<(EntityId, Revision)>,
) -> Result<Vec<ProjectionRow>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT manifest.artifact_id, manifest.revision, artifact.kind, manifest.sha256,
                manifest.relative_path, manifest.projection_kind, manifest.generated_sha256
         FROM projection_manifests AS manifest
         JOIN artifacts AS artifact ON artifact.id = manifest.artifact_id
         WHERE manifest.projection_kind != 'raw'
         ORDER BY manifest.relative_path",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(parse_projection_row)
        .filter_map(|result| match result {
            Ok(row)
                if allowed
                    .contains(&(row.source_artifact.id.clone(), row.source_artifact.revision)) =>
            {
                Some(Ok(row))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn load_projection_row(
    connection: &Connection,
    relative_path: &SafeRelativePath,
) -> Result<Option<ProjectionRow>, StoreError> {
    connection
        .query_row(
            "SELECT manifest.artifact_id, manifest.revision, artifact.kind, manifest.sha256,
                    manifest.relative_path, manifest.projection_kind, manifest.generated_sha256
             FROM projection_manifests AS manifest
             JOIN artifacts AS artifact ON artifact.id = manifest.artifact_id
             WHERE manifest.relative_path = ?1 AND manifest.projection_kind != 'raw'",
            params![relative_path.as_str()],
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
        .map(parse_projection_row)
        .transpose()
}

fn parse_projection_row(
    row: (String, i64, String, String, String, String, String),
) -> Result<ProjectionRow, StoreError> {
    let (id, revision, artifact_kind, source_sha, path, projection_kind, generated_sha) = row;
    Ok(ProjectionRow {
        source_artifact: ArtifactReference {
            kind: serde_json::from_value(Value::String(artifact_kind))?,
            id: EntityId::try_new(id)?,
            revision: Revision::try_new(to_u64(revision)?)?,
            sha256: Sha256Digest::try_new(source_sha)?,
        },
        relative_path: SafeRelativePath::try_new(path)?,
        kind: serde_json::from_value(Value::String(projection_kind))?,
        generated_sha256: Sha256Digest::try_new(generated_sha)?,
    })
}

fn observe_projection(
    workspace_root: &Path,
    relative_path: &SafeRelativePath,
    generated: &Sha256Digest,
) -> (ProjectionEditStatus, Option<Sha256Digest>, Option<String>) {
    let destination = workspace_root.join(relative_path.as_str());
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => (
            ProjectionEditStatus::RepairRequired,
            None,
            Some("projection destination is not a regular file".to_owned()),
        ),
        Ok(_) => match digest_file(&destination) {
            Ok(observed) if observed == *generated => {
                (ProjectionEditStatus::Current, Some(observed), None)
            }
            Ok(observed) => (ProjectionEditStatus::Edited, Some(observed), None),
            Err(error) => (
                ProjectionEditStatus::RepairRequired,
                None,
                Some(error.to_string()),
            ),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (ProjectionEditStatus::Missing, None, None)
        }
        Err(error) => (
            ProjectionEditStatus::RepairRequired,
            None,
            Some(error.to_string()),
        ),
    }
}

fn update_observation(
    connection: &Connection,
    relative_path: &SafeRelativePath,
    status: ProjectionEditStatus,
    observed: Option<&Sha256Digest>,
    last_error: Option<&str>,
    updated_at: &canisend_contracts::UtcTimestamp,
) -> Result<(), StoreError> {
    let updated = connection.execute(
        "UPDATE projection_manifests
         SET observed_sha256 = ?2, status = ?3, last_error = ?4, updated_at = ?5
         WHERE relative_path = ?1",
        params![
            relative_path.as_str(),
            observed.map(Sha256Digest::as_str),
            enum_name(status)?,
            last_error,
            updated_at.as_str()
        ],
    )?;
    if updated != 1 {
        return Err(StoreError::ProjectionNotFound(relative_path.to_string()));
    }
    Ok(())
}

fn allowed_artifacts(
    package_artifact: &ArtifactReference,
    package: &PackageManifestRecord,
) -> BTreeSet<(EntityId, Revision)> {
    std::iter::once((package_artifact.id.clone(), package_artifact.revision))
        .chain(
            package
                .documents
                .iter()
                .map(|reference| (reference.id.clone(), reference.revision)),
        )
        .collect()
}

fn verify_package_current(
    connection: &Connection,
    job_id: &EntityId,
    package: &ArtifactReference,
) -> Result<(), StoreError> {
    let current: Option<(String, i64, String)> = connection
        .query_row(
            "SELECT head.artifact_id, head.artifact_revision, revision.sha256
             FROM package_heads AS head
             JOIN workflow_runs AS run ON run.id = head.workflow_run_id
             JOIN stage_executions AS stage ON stage.workflow_run_id = run.id
              AND stage.stage = 'package' AND stage.status = 'complete'
              AND stage.output_artifact_id = head.artifact_id
              AND stage.output_artifact_revision = head.artifact_revision
             JOIN artifacts AS artifact ON artifact.id = head.artifact_id
              AND artifact.head_revision = head.artifact_revision AND artifact.stale = 0
             JOIN artifact_revisions AS revision ON revision.artifact_id = head.artifact_id
              AND revision.revision = head.artifact_revision
             WHERE run.job_id = ?1
             ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
            params![job_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    if current
        == Some((
            package.id.to_string(),
            to_i64(package.revision.get())?,
            package.sha256.to_string(),
        ))
    {
        Ok(())
    } else {
        Err(StoreError::TaskStale(
            "package changed while exporting projections".to_owned(),
        ))
    }
}

fn ensure_job_destination(
    job_id: &EntityId,
    destination: &SafeRelativePath,
) -> Result<(), StoreError> {
    let expected = format!("jobs/{job_id}/");
    if !destination.as_str().starts_with(&expected) {
        return Err(StoreError::ProjectionPathRejected);
    }
    Ok(())
}

fn join_projection_path(
    base: &SafeRelativePath,
    file_name: &str,
) -> Result<SafeRelativePath, StoreError> {
    SafeRelativePath::try_new(format!("{}/{file_name}", base.as_str())).map_err(StoreError::from)
}

fn read_safe_projection(
    workspace_root: &Path,
    relative_path: &SafeRelativePath,
) -> Result<Vec<u8>, StoreError> {
    let path = workspace_root.join(relative_path.as_str());
    let metadata = fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > DEFAULT_MAX_BLOB_BYTES
    {
        return Err(StoreError::UnsafePath(path));
    }
    fs::read(&path).map_err(|source| io_error(path, source))
}

fn write_new_user_copy(
    workspace_root: &Path,
    destination: &SafeRelativePath,
    bytes: &[u8],
) -> Result<(), StoreError> {
    let path = workspace_root.join(destination.as_str());
    let parent = path.parent().ok_or(StoreError::ProjectionPathRejected)?;
    ensure_safe_parent(workspace_root, parent)?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|source| io_error(&path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error(&path, source))?;
    file.sync_all().map_err(|source| io_error(path, source))
}

fn ensure_safe_parent(root: &Path, parent: &Path) -> Result<(), StoreError> {
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| StoreError::ProjectionPathRejected)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(StoreError::UnsafePath(current));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|source| io_error(&current, source))?;
            }
            Err(source) => return Err(io_error(current, source)),
        }
    }
    Ok(())
}

fn pretty_json_bytes(value: &Value) -> Result<Vec<u8>, StoreError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, StoreError> {
    fn sort(value: Value) -> Value {
        match value {
            Value::Object(object) => {
                let mut entries = object.into_iter().collect::<Vec<_>>();
                entries.sort_by(|left, right| left.0.cmp(&right.0));
                Value::Object(
                    entries
                        .into_iter()
                        .map(|(key, value)| (key, sort(value)))
                        .collect(),
                )
            }
            Value::Array(values) => Value::Array(values.into_iter().map(sort).collect()),
            other => other,
        }
    }
    let mut bytes = serde_json::to_vec(&sort(value.clone()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn digest_bytes(bytes: &[u8]) -> Result<Sha256Digest, StoreError> {
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes))).map_err(StoreError::from)
}

fn load_record<T>(blobs: &BlobStore, reference: &ArtifactReference) -> Result<T, StoreError>
where
    T: serde::de::DeserializeOwned + schemars::JsonSchema + canisend_contracts::SemanticValidate,
{
    let bytes = blobs.read_verified(&reference.sha256, DEFAULT_MAX_BLOB_BYTES)?;
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

fn typst_projection_error(error: TypstProjectionError) -> StoreError {
    match error {
        TypstProjectionError::UnresolvedTemplateFields { count } => {
            StoreError::TemplateFieldsUnresolved { count }
        }
        TypstProjectionError::SourceTooLarge { max_bytes } => StoreError::InvalidInput(format!(
            "generated Typst source exceeds the {max_bytes}-byte render limit"
        )),
        TypstProjectionError::TemplateEncoding => StoreError::TypstProjectionInvariant,
    }
}

fn document_kind_slug(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::CoverLetter => "cover-letter",
        DocumentKind::ResearchStatement => "research-statement",
        DocumentKind::TeachingStatement => "teaching-statement",
        DocumentKind::Cv => "cv",
    }
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
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
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite value".to_owned()))
}
