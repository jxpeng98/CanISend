use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path},
};

use canisend_contracts::{
    ArtifactKind, ArtifactReference, CandidateValidationError, DocumentKind, DocumentRecord,
    EntityId, ReadinessState, RenderManifestRecord, RenderedDocumentRecord, Revision,
    SafeRelativePath, Sha256Digest, validate_external_candidate,
};
use canisend_io::{EmbeddedTypstCompiler, project_document_typst, validate_rendered_pdf};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;

use crate::{
    BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, PackageService, StoreError, generate_id, io_error,
    now_utc,
};

pub struct RenderService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
    workspace_root: &'a Path,
}

impl<'a> RenderService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore, workspace_root: &'a Path) -> Self {
        Self {
            database,
            blobs,
            workspace_root,
        }
    }

    pub fn build(
        &mut self,
        job_id: &EntityId,
    ) -> Result<(ArtifactReference, RenderManifestRecord), StoreError> {
        let (package_artifact, package) =
            PackageService::new(self.database, self.blobs).current_with_reference(job_id)?;
        match self.current_for_package(job_id, &package_artifact) {
            Ok(current) => return Ok(current),
            Err(StoreError::WorkflowConflict(message))
                if message == "no current render manifest" => {}
            Err(error) => return Err(error),
        }
        if !matches!(
            package.readiness.state,
            ReadinessState::ReadyToExport | ReadinessState::Exported
        ) {
            return Err(StoreError::WorkflowConflict(format!(
                "package readiness is {:?}, not ready to render",
                package.readiness.state
            )));
        }

        let documents = package
            .documents
            .iter()
            .map(|reference| {
                Ok((
                    reference.clone(),
                    load_record::<DocumentRecord>(self.blobs, reference)?,
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let compiler = EmbeddedTypstCompiler::new();
        let mut prepared = Vec::with_capacity(documents.len() * 2);
        let mut rendered_documents = Vec::with_capacity(documents.len());
        for (document_artifact, document) in documents {
            if document.job_id != *job_id {
                return Err(StoreError::DependencyConflict(
                    "package document belongs to another job".to_owned(),
                ));
            }
            let source = project_document_typst(&document_artifact, &document)
                .map_err(typst_projection_error)?;
            let rendered = compiler.compile_pdf(&source)?;
            let page_count = rendered.page_count();
            let warning_count = u32::try_from(rendered.warning_count())
                .map_err(|_| StoreError::Invariant("render warning count overflow".to_owned()))?;
            let elapsed_millis = u64::try_from(rendered.elapsed().as_millis())
                .map_err(|_| StoreError::Invariant("render duration overflow".to_owned()))?;
            let pdf_bytes = rendered.into_bytes();
            let byte_count = u64::try_from(pdf_bytes.len())
                .map_err(|_| StoreError::Invariant("render byte count overflow".to_owned()))?;

            let typst = prepare_artifact(
                self.blobs,
                ArtifactKind::TypstSource,
                source.into_bytes(),
                vec![document_artifact.clone()],
                "generate trusted Typst from the authoritative structured document",
            )?;
            let pdf = prepare_artifact(
                self.blobs,
                ArtifactKind::Pdf,
                pdf_bytes,
                vec![typst.reference.clone()],
                "compile and validate PDF entirely inside the Rust process",
            )?;
            rendered_documents.push(RenderedDocumentRecord {
                kind: document.kind,
                document_artifact,
                typst_artifact: typst.reference.clone(),
                pdf_artifact: pdf.reference.clone(),
                page_count,
                byte_count,
                warning_count,
                elapsed_millis,
            });
            prepared.push(typst);
            prepared.push(pdf);
        }

        let rendered_at = now_utc()?;
        let manifest = RenderManifestRecord {
            id: generate_id()?,
            job_id: job_id.clone(),
            package_artifact: package_artifact.clone(),
            documents: rendered_documents,
            rendered_at: rendered_at.clone(),
            submission_performed: false,
            revision: Revision::try_new(1)?,
        };
        validate_external_candidate::<RenderManifestRecord>(&serde_json::to_value(&manifest)?)
            .map_err(candidate_error)?;
        let manifest_bytes = canonical_json_bytes(&serde_json::to_value(&manifest)?)?;
        let mut manifest_dependencies = vec![package_artifact.clone()];
        for document in &manifest.documents {
            manifest_dependencies.push(document.document_artifact.clone());
            manifest_dependencies.push(document.typst_artifact.clone());
            manifest_dependencies.push(document.pdf_artifact.clone());
        }
        let manifest_artifact = prepare_artifact(
            self.blobs,
            ArtifactKind::RenderManifest,
            manifest_bytes,
            manifest_dependencies,
            "freeze revision-bound in-process render outputs",
        )?;

        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        let context = load_commit_context(&transaction, job_id, &package_artifact)?;
        if context.render_status != "ready" {
            return Err(StoreError::TaskStale(format!(
                "render stage changed to {} while compiling",
                context.render_status
            )));
        }
        verify_reference_current(&transaction, &package_artifact)?;
        for document in &manifest.documents {
            verify_reference_current(&transaction, &document.document_artifact)?;
        }
        transaction.execute(
            "UPDATE artifacts SET stale = 1 WHERE id IN (
                 SELECT artifact_id FROM render_heads WHERE workflow_run_id = ?1
             )",
            params![context.run_id.as_str()],
        )?;
        transaction.execute(
            "DELETE FROM render_heads WHERE workflow_run_id = ?1",
            params![context.run_id.as_str()],
        )?;
        for artifact in &prepared {
            insert_prepared_artifact(&transaction, artifact, &rendered_at)?;
        }
        insert_prepared_artifact(&transaction, &manifest_artifact, &rendered_at)?;
        let completed = transaction.execute(
            "UPDATE stage_executions
             SET status = 'complete', execution_mode = 'deterministic',
                 output_artifact_id = ?3, output_artifact_revision = 1,
                 started_at = COALESCE(started_at, ?4), completed_at = ?4, updated_at = ?4
             WHERE workflow_run_id = ?1 AND id = ?2 AND stage = 'render' AND status = 'ready'",
            params![
                context.run_id.as_str(),
                context.render_execution_id.as_str(),
                manifest_artifact.reference.id.as_str(),
                rendered_at.as_str()
            ],
        )?;
        if completed != 1 {
            return Err(StoreError::TaskStale(
                "render stage changed before artifact commit".to_owned(),
            ));
        }
        transaction.execute(
            "INSERT INTO render_heads(
                workflow_run_id, package_artifact_id, package_artifact_revision,
                artifact_id, artifact_revision, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            params![
                context.run_id.as_str(),
                package_artifact.id.as_str(),
                to_i64(package_artifact.revision.get())?,
                manifest_artifact.reference.id.as_str(),
                rendered_at.as_str()
            ],
        )?;
        transaction.execute(
            "UPDATE workflow_runs SET status = 'complete' WHERE id = ?1",
            params![context.run_id.as_str()],
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, 'system', 'render.build', ?2, 1,
                       'compile trusted sources and freeze validated PDF artifacts', ?3)",
            params![
                event_id.as_str(),
                manifest.id.as_str(),
                rendered_at.as_str()
            ],
        )?;
        transaction.commit()?;
        Ok((manifest_artifact.reference, manifest))
    }

    pub fn current(
        &mut self,
        job_id: &EntityId,
    ) -> Result<(ArtifactReference, RenderManifestRecord), StoreError> {
        let (package_artifact, _) =
            PackageService::new(self.database, self.blobs).current_with_reference(job_id)?;
        self.current_for_package(job_id, &package_artifact)
    }

    pub fn export(
        &mut self,
        job_id: &EntityId,
        destination: &SafeRelativePath,
    ) -> Result<
        (
            ArtifactReference,
            RenderManifestRecord,
            Vec<SafeRelativePath>,
        ),
        StoreError,
    > {
        ensure_job_destination(job_id, destination)?;
        let (manifest_artifact, manifest) = self.current(job_id)?;
        let mut files = Vec::with_capacity(manifest.documents.len() + 1);
        for document in &manifest.documents {
            let bytes = self
                .blobs
                .read_verified(&document.pdf_artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
            let page_count = validate_rendered_pdf(&bytes)?;
            if page_count != document.page_count
                || u64::try_from(bytes.len()).ok() != Some(document.byte_count)
            {
                return Err(StoreError::DependencyConflict(
                    "rendered PDF metadata does not match its validated blob".to_owned(),
                ));
            }
            files.push((
                join_path(
                    destination,
                    &format!("{}.pdf", document_kind_slug(document.kind)),
                )?,
                bytes,
            ));
        }
        files.push((
            join_path(destination, "render-manifest.json")?,
            self.blobs
                .read_verified(&manifest_artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?,
        ));

        create_empty_export_directory(self.workspace_root, destination)?;
        for (path, bytes) in &files {
            write_new_file(self.workspace_root, path, bytes)?;
        }
        let exported_at = now_utc()?;
        let event_id = generate_id()?;
        self.database.connection().execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, 'user', 'render.export', ?2, ?3,
                       'export validated private PDFs after explicit consent', ?4)",
            params![
                event_id.as_str(),
                manifest_artifact.id.as_str(),
                to_i64(manifest_artifact.revision.get())?,
                exported_at.as_str()
            ],
        )?;
        Ok((
            manifest_artifact,
            manifest,
            files.into_iter().map(|(path, _)| path).collect(),
        ))
    }

    fn current_for_package(
        &self,
        job_id: &EntityId,
        package_artifact: &ArtifactReference,
    ) -> Result<(ArtifactReference, RenderManifestRecord), StoreError> {
        type Row = (String, i64, String);
        let row: Option<Row> = self
            .database
            .connection()
            .query_row(
                "SELECT head.artifact_id, head.artifact_revision, revision.sha256
                 FROM render_heads AS head
                 JOIN workflow_runs AS run ON run.id = head.workflow_run_id AND run.job_id = ?1
                 JOIN package_heads AS package ON package.workflow_run_id = head.workflow_run_id
                  AND package.artifact_id = head.package_artifact_id
                  AND package.artifact_revision = head.package_artifact_revision
                 JOIN stage_executions AS stage ON stage.workflow_run_id = head.workflow_run_id
                  AND stage.stage = 'render' AND stage.status = 'complete'
                  AND stage.output_artifact_id = head.artifact_id
                  AND stage.output_artifact_revision = head.artifact_revision
                 JOIN artifacts AS artifact ON artifact.id = head.artifact_id
                  AND artifact.head_revision = head.artifact_revision AND artifact.stale = 0
                 JOIN artifact_revisions AS revision ON revision.artifact_id = head.artifact_id
                  AND revision.revision = head.artifact_revision
                 WHERE head.package_artifact_id = ?2 AND head.package_artifact_revision = ?3",
                params![
                    job_id.as_str(),
                    package_artifact.id.as_str(),
                    to_i64(package_artifact.revision.get())?
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (id, revision, sha256) = row
            .ok_or_else(|| StoreError::WorkflowConflict("no current render manifest".to_owned()))?;
        let reference = ArtifactReference {
            kind: ArtifactKind::RenderManifest,
            id: EntityId::try_new(id)?,
            revision: Revision::try_new(to_u64(revision)?)?,
            sha256: Sha256Digest::try_new(sha256)?,
        };
        let manifest = load_record::<RenderManifestRecord>(self.blobs, &reference)?;
        if manifest.job_id != *job_id || manifest.package_artifact != *package_artifact {
            return Err(StoreError::DependencyConflict(
                "render manifest does not bind the current package".to_owned(),
            ));
        }
        Ok((reference, manifest))
    }
}

#[derive(Debug)]
struct CommitContext {
    run_id: EntityId,
    render_execution_id: EntityId,
    render_status: String,
}

#[derive(Debug)]
struct PreparedArtifact {
    reference: ArtifactReference,
    size: u64,
    dependencies: Vec<ArtifactReference>,
    reason: &'static str,
}

fn prepare_artifact(
    blobs: &BlobStore,
    kind: ArtifactKind,
    bytes: Vec<u8>,
    dependencies: Vec<ArtifactReference>,
    reason: &'static str,
) -> Result<PreparedArtifact, StoreError> {
    let sha256 = blobs.put_bytes(&bytes)?;
    let size = blobs.verify(&sha256, DEFAULT_MAX_BLOB_BYTES)?;
    Ok(PreparedArtifact {
        reference: ArtifactReference {
            kind,
            id: generate_id()?,
            revision: Revision::try_new(1)?,
            sha256,
        },
        size,
        dependencies,
        reason,
    })
}

fn insert_prepared_artifact(
    transaction: &Transaction<'_>,
    artifact: &PreparedArtifact,
    created_at: &canisend_contracts::UtcTimestamp,
) -> Result<(), StoreError> {
    let kind = enum_name(artifact.reference.kind)?;
    transaction.execute(
        "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
         VALUES (?1, ?2, 1, 0, ?3)",
        params![artifact.reference.id.as_str(), kind, created_at.as_str()],
    )?;
    transaction.execute(
        "INSERT INTO artifact_revisions(
            artifact_id, revision, sha256, size, actor, reason, created_at
         ) VALUES (?1, 1, ?2, ?3, 'system', ?4, ?5)",
        params![
            artifact.reference.id.as_str(),
            artifact.reference.sha256.as_str(),
            to_i64(artifact.size)?,
            artifact.reason,
            created_at.as_str()
        ],
    )?;
    transaction.execute(
        "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
         VALUES (?1, 'artifact', ?2, 1, ?3)",
        params![
            artifact.reference.sha256.as_str(),
            artifact.reference.id.as_str(),
            created_at.as_str()
        ],
    )?;
    let mut seen = BTreeSet::new();
    for dependency in &artifact.dependencies {
        if !seen.insert((&dependency.id, dependency.revision)) {
            continue;
        }
        transaction.execute(
            "INSERT INTO artifact_dependencies(
                artifact_id, revision, depends_on_artifact_id, depends_on_revision,
                depends_on_sha256
             ) VALUES (?1, 1, ?2, ?3, ?4)",
            params![
                artifact.reference.id.as_str(),
                dependency.id.as_str(),
                to_i64(dependency.revision.get())?,
                dependency.sha256.as_str()
            ],
        )?;
    }
    Ok(())
}

fn load_commit_context(
    connection: &Connection,
    job_id: &EntityId,
    package: &ArtifactReference,
) -> Result<CommitContext, StoreError> {
    let row: Option<(String, String, String)> = connection
        .query_row(
            "SELECT run.id, render.id, render.status
             FROM workflow_runs AS run
             JOIN package_heads AS package ON package.workflow_run_id = run.id
              AND package.artifact_id = ?2 AND package.artifact_revision = ?3
             JOIN stage_executions AS package_stage ON package_stage.workflow_run_id = run.id
              AND package_stage.stage = 'package' AND package_stage.status = 'complete'
              AND package_stage.output_artifact_id = package.artifact_id
              AND package_stage.output_artifact_revision = package.artifact_revision
             JOIN stage_executions AS render ON render.workflow_run_id = run.id
              AND render.stage = 'render'
             WHERE run.job_id = ?1
             ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
            params![
                job_id.as_str(),
                package.id.as_str(),
                to_i64(package.revision.get())?
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (run_id, render_execution_id, render_status) = row.ok_or_else(|| {
        StoreError::TaskStale("current package no longer allows rendering".to_owned())
    })?;
    Ok(CommitContext {
        run_id: EntityId::try_new(run_id)?,
        render_execution_id: EntityId::try_new(render_execution_id)?,
        render_status,
    })
}

fn verify_reference_current(
    connection: &Connection,
    reference: &ArtifactReference,
) -> Result<(), StoreError> {
    let current: Option<(String, i64, i64, String)> = connection
        .query_row(
            "SELECT kind, head_revision, stale, revision.sha256
             FROM artifacts
             JOIN artifact_revisions AS revision
               ON revision.artifact_id = artifacts.id AND revision.revision = ?2
             WHERE artifacts.id = ?1",
            params![reference.id.as_str(), to_i64(reference.revision.get())?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let expected_kind = enum_name(reference.kind)?;
    if !current.is_some_and(|(kind, head, stale, sha256)| {
        kind == expected_kind
            && head == i64::try_from(reference.revision.get()).unwrap_or(-1)
            && stale == 0
            && sha256 == reference.sha256.as_str()
    }) {
        return Err(StoreError::TaskStale(format!(
            "render input {} is no longer current",
            reference.id
        )));
    }
    Ok(())
}

fn ensure_job_destination(
    job_id: &EntityId,
    destination: &SafeRelativePath,
) -> Result<(), StoreError> {
    let prefix = format!("jobs/{job_id}/");
    if !destination.as_str().starts_with(&prefix)
        || destination.as_str().trim_end_matches('/') == format!("jobs/{job_id}")
    {
        return Err(StoreError::ProjectionPathRejected);
    }
    Ok(())
}

fn create_empty_export_directory(
    root: &Path,
    destination: &SafeRelativePath,
) -> Result<(), StoreError> {
    let root_metadata = fs::symlink_metadata(root).map_err(|source| io_error(root, source))?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(StoreError::UnsafePath(root.to_path_buf()));
    }
    let destination_path = root.join(destination.as_str());
    match fs::symlink_metadata(&destination_path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(StoreError::UnsafePath(destination_path));
            }
            if fs::read_dir(&destination_path)
                .map_err(|source| io_error(&destination_path, source))?
                .next()
                .is_some()
            {
                return Err(StoreError::ProjectionUnmanagedConflict(
                    destination.to_string(),
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error(destination_path, source)),
    }

    let mut current = root.to_path_buf();
    for component in Path::new(destination.as_str()).components() {
        let Component::Normal(name) = component else {
            return Err(StoreError::ProjectionPathRejected);
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(StoreError::UnsafePath(current));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|source| io_error(&current, source))?;
            }
            Err(source) => return Err(io_error(current, source)),
        }
    }
    Ok(())
}

fn write_new_file(
    root: &Path,
    relative_path: &SafeRelativePath,
    bytes: &[u8],
) -> Result<(), StoreError> {
    let path = root.join(relative_path.as_str());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| io_error(&path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error(&path, source))?;
    file.sync_all().map_err(|source| io_error(&path, source))
}

fn join_path(
    destination: &SafeRelativePath,
    file_name: &str,
) -> Result<SafeRelativePath, StoreError> {
    SafeRelativePath::try_new(format!("{}/{file_name}", destination.as_str()))
        .map_err(StoreError::from)
}

const fn document_kind_slug(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::CoverLetter => "cover-letter",
        DocumentKind::ResearchStatement => "research-statement",
        DocumentKind::TeachingStatement => "teaching-statement",
        DocumentKind::Cv => "cv",
    }
}

fn load_record<T>(blobs: &BlobStore, reference: &ArtifactReference) -> Result<T, StoreError>
where
    T: serde::de::DeserializeOwned + schemars::JsonSchema + canisend_contracts::SemanticValidate,
{
    let bytes = blobs.read_verified(&reference.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
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

fn candidate_error(error: CandidateValidationError) -> StoreError {
    match error {
        CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        CandidateValidationError::Semantic(violations) => StoreError::CandidateSemantic(violations),
    }
}

fn typst_projection_error(error: canisend_io::TypstProjectionError) -> StoreError {
    match error {
        canisend_io::TypstProjectionError::UnresolvedTemplateFields { count } => {
            StoreError::TemplateFieldsUnresolved { count }
        }
        canisend_io::TypstProjectionError::TemplateEncoding
        | canisend_io::TypstProjectionError::SourceTooLarge { .. } => {
            StoreError::TypstProjectionInvariant
        }
    }
}

fn enum_name<T: serde::Serialize>(value: T) -> Result<String, StoreError> {
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
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite INTEGER".to_owned()))
}
