use std::{fs, io::Write, path::Path};

use canisend_contracts::{
    ApplicationId, ApplicationPackBindingV3, DeliverableId, DeliverableRecordV3,
    ProjectionEditStatus, ProjectionReconcileAction, Revision, SafeRelativePath, Sha256Digest,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    ApplicationModelRepository, BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError,
    StoredApplicationModelV3,
    application_v3::load_application_model_revision,
    artifact::{digest_file, write_projection},
    io_error, now_utc,
};

pub const APPLICATION_PROJECTION_FORMAT_V3: &str = "canisend.application-projections/v3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationProjectionKindV3 {
    ApplicationModelJson,
    DeliverableMetadataJson,
    DeliverableContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationProjectionRecordV3 {
    pub application_id: ApplicationId,
    pub application_revision: Revision,
    pub snapshot_sha256: Sha256Digest,
    pub pack: ApplicationPackBindingV3,
    pub deliverable_id: Option<DeliverableId>,
    pub deliverable_revision: Option<Revision>,
    pub relative_path: SafeRelativePath,
    pub kind: ApplicationProjectionKindV3,
    pub source_sha256: Sha256Digest,
    pub generated_sha256: Sha256Digest,
    pub observed_sha256: Option<Sha256Digest>,
    pub edit_status: ProjectionEditStatus,
    pub superseded: bool,
    pub updated_at: canisend_contracts::UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationLegacyProjectionV3 {
    pub application_id: ApplicationId,
    pub legacy_job_id: canisend_contracts::EntityId,
    pub relative_path: SafeRelativePath,
    pub legacy_projection_kind: String,
    pub generated_sha256: Sha256Digest,
    pub observed_sha256: Option<Sha256Digest>,
    pub edit_status: ProjectionEditStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationProjectionCatalogV3 {
    pub format: String,
    pub application_id: ApplicationId,
    pub current_application_revision: Revision,
    pub current_snapshot_sha256: Sha256Digest,
    pub pack: ApplicationPackBindingV3,
    pub projections: Vec<ApplicationProjectionRecordV3>,
    pub legacy_projections: Vec<ApplicationLegacyProjectionV3>,
    pub inspected_at: canisend_contracts::UtcTimestamp,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationProjectionReconcileV3 {
    pub application_id: ApplicationId,
    pub projection: ApplicationProjectionRecordV3,
    pub action: ProjectionReconcileAction,
    pub preserved_copy_path: Option<SafeRelativePath>,
    pub preserved_copy_sha256: Option<Sha256Digest>,
    pub authoritative_changed: bool,
    pub reconciled_at: canisend_contracts::UtcTimestamp,
}

pub struct ApplicationProjectionService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
    workspace_root: &'a Path,
}

impl<'a> ApplicationProjectionService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore, workspace_root: &'a Path) -> Self {
        Self {
            database,
            blobs,
            workspace_root,
        }
    }

    pub fn project(
        &mut self,
        application_id: &ApplicationId,
    ) -> Result<ApplicationProjectionCatalogV3, StoreError> {
        let stored = ApplicationModelRepository::new(self.database).get(application_id)?;
        let generated = generate_projections(self.blobs, &stored)?;
        for projection in &generated {
            preflight_projection(
                self.database.connection(),
                self.workspace_root,
                &projection.record.relative_path,
            )?;
        }
        let projected_at = now_utc()?;
        let transaction = self.database.immediate_transaction()?;
        verify_current_application(&transaction, &stored)?;
        for projection in &generated {
            record_pending_projection(&transaction, &projection.record, &projected_at)?;
        }
        transaction.commit()?;

        for projection in &generated {
            if let Err(error) = write_projection(
                self.workspace_root,
                &projection.record.relative_path,
                &projection.bytes,
            ) {
                update_observation(
                    self.database.connection(),
                    &projection.record.relative_path,
                    ProjectionEditStatus::RepairRequired,
                    None,
                    Some(&error.to_string()),
                    &projected_at,
                )?;
                return Err(error);
            }
            update_observation(
                self.database.connection(),
                &projection.record.relative_path,
                ProjectionEditStatus::Current,
                Some(&projection.record.generated_sha256),
                None,
                &projected_at,
            )?;
        }
        verify_current_application(self.database.connection(), &stored)?;
        self.catalog(application_id)
    }

    pub fn catalog(
        &mut self,
        application_id: &ApplicationId,
    ) -> Result<ApplicationProjectionCatalogV3, StoreError> {
        let current = ApplicationModelRepository::new(self.database).get(application_id)?;
        let inspected_at = now_utc()?;
        let mut projections = load_projection_rows(self.database.connection(), application_id)?;
        for projection in &mut projections {
            let (status, observed, last_error) = observe_projection(
                self.workspace_root,
                &projection.relative_path,
                &projection.generated_sha256,
            );
            update_observation(
                self.database.connection(),
                &projection.relative_path,
                status,
                observed.as_ref(),
                last_error.as_deref(),
                &inspected_at,
            )?;
            projection.edit_status = status;
            projection.observed_sha256 = observed;
            projection.superseded = projection.application_revision
                != current.snapshot.application.revision
                || projection.snapshot_sha256 != current.snapshot_sha256
                || projection.pack != current.snapshot.pack;
            projection.updated_at = inspected_at.clone();
        }
        let legacy_projections = recognize_legacy_projections(
            self.database.connection(),
            self.workspace_root,
            application_id,
        )?;
        Ok(ApplicationProjectionCatalogV3 {
            format: APPLICATION_PROJECTION_FORMAT_V3.to_owned(),
            application_id: application_id.clone(),
            current_application_revision: current.snapshot.application.revision,
            current_snapshot_sha256: current.snapshot_sha256,
            pack: current.snapshot.pack,
            projections,
            legacy_projections,
            inspected_at,
            submission_performed: false,
        })
    }

    pub fn replace(
        &mut self,
        application_id: &ApplicationId,
        relative_path: &SafeRelativePath,
    ) -> Result<ApplicationProjectionReconcileV3, StoreError> {
        self.restore_projection(application_id, relative_path, None)
    }

    pub fn copy_as_new(
        &mut self,
        application_id: &ApplicationId,
        relative_path: &SafeRelativePath,
        destination: &SafeRelativePath,
    ) -> Result<ApplicationProjectionReconcileV3, StoreError> {
        ensure_application_destination(application_id, destination)?;
        if relative_path == destination {
            return Err(StoreError::InvalidInput(
                "copy destination must differ from the managed projection".to_owned(),
            ));
        }
        preflight_new_user_copy(self.database.connection(), self.workspace_root, destination)?;
        self.restore_projection(application_id, relative_path, Some(destination))
    }

    pub fn repair_all(&mut self) -> Result<usize, StoreError> {
        let rows = load_all_projection_rows(self.database.connection())?;
        let mut repaired = 0;
        for row in rows {
            repaired += usize::from(repair_projection(
                self.database.connection(),
                self.blobs,
                self.workspace_root,
                &row,
            )?);
        }
        Ok(repaired)
    }

    fn restore_projection(
        &mut self,
        application_id: &ApplicationId,
        relative_path: &SafeRelativePath,
        copy_destination: Option<&SafeRelativePath>,
    ) -> Result<ApplicationProjectionReconcileV3, StoreError> {
        ensure_application_destination(application_id, relative_path)?;
        let current = ApplicationModelRepository::new(self.database).get(application_id)?;
        let mut row = load_projection_row(self.database.connection(), relative_path)?
            .ok_or_else(|| StoreError::ProjectionNotFound(relative_path.to_string()))?;
        if row.application_id != *application_id {
            return Err(StoreError::ProjectionNotFound(relative_path.to_string()));
        }
        let superseded = row.application_revision != current.snapshot.application.revision
            || row.snapshot_sha256 != current.snapshot_sha256
            || row.pack != current.snapshot.pack;
        if superseded && copy_destination.is_none() {
            return Err(StoreError::TaskStale(
                "Application projection does not bind the current Application revision".to_owned(),
            ));
        }
        let (status, observed, _) =
            observe_projection(self.workspace_root, relative_path, &row.generated_sha256);
        if status == ProjectionEditStatus::RepairRequired {
            return Err(StoreError::UnsafePath(
                self.workspace_root.join(relative_path.as_str()),
            ));
        }
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
        let bytes =
            generate_projection_bytes_from_database(self.database.connection(), self.blobs, &row)?;
        if digest_bytes(&bytes)? != row.generated_sha256 {
            return Err(StoreError::DependencyConflict(format!(
                "projection recipe changed for {relative_path}"
            )));
        }
        write_projection(self.workspace_root, relative_path, &bytes)?;
        let reconciled_at = now_utc()?;
        update_observation(
            self.database.connection(),
            relative_path,
            ProjectionEditStatus::Current,
            Some(&row.generated_sha256),
            None,
            &reconciled_at,
        )?;
        row.edit_status = ProjectionEditStatus::Current;
        row.observed_sha256 = Some(row.generated_sha256.clone());
        row.superseded = superseded;
        row.updated_at = reconciled_at.clone();
        Ok(ApplicationProjectionReconcileV3 {
            application_id: application_id.clone(),
            projection: row,
            action: if copy_destination.is_some() {
                ProjectionReconcileAction::CopyAsNew
            } else {
                ProjectionReconcileAction::Replace
            },
            preserved_copy_path: copy_destination.cloned(),
            preserved_copy_sha256,
            authoritative_changed: false,
            reconciled_at,
        })
    }
}

struct GeneratedProjectionV3 {
    record: ApplicationProjectionRecordV3,
    bytes: Vec<u8>,
}

fn generate_projections(
    blobs: &BlobStore,
    stored: &StoredApplicationModelV3,
) -> Result<Vec<GeneratedProjectionV3>, StoreError> {
    let snapshot = &stored.snapshot;
    let application_id = &snapshot.application.id;
    let application_revision = snapshot.application.revision;
    let updated_at = now_utc()?;
    let model_bytes = pretty_json_bytes(&serde_json::to_value(snapshot)?)?;
    let mut generated = vec![generated_projection(
        stored,
        None,
        SafeRelativePath::try_new(format!("applications/{application_id}/application.json"))?,
        ApplicationProjectionKindV3::ApplicationModelJson,
        stored.snapshot_sha256.clone(),
        model_bytes,
        &updated_at,
    )?];
    let mut deliverables = snapshot.deliverables.iter().collect::<Vec<_>>();
    deliverables.sort_by(|left, right| left.id.cmp(&right.id));
    for deliverable in deliverables {
        let base = format!(
            "applications/{application_id}/deliverables/{}",
            deliverable.id
        );
        let metadata_bytes = pretty_json_bytes(&serde_json::to_value(deliverable)?)?;
        generated.push(generated_projection(
            stored,
            Some(deliverable),
            SafeRelativePath::try_new(format!("{base}/deliverable.json"))?,
            ApplicationProjectionKindV3::DeliverableMetadataJson,
            stored.snapshot_sha256.clone(),
            metadata_bytes,
            &updated_at,
        )?);
        if let Some(content) = &deliverable.content {
            let bytes = blobs.read_verified(&content.sha256, DEFAULT_MAX_BLOB_BYTES)?;
            generated.push(generated_projection(
                stored,
                Some(deliverable),
                SafeRelativePath::try_new(format!(
                    "{base}/content{}",
                    media_type_extension(deliverable.media_type.as_deref())
                ))?,
                ApplicationProjectionKindV3::DeliverableContent,
                content.sha256.clone(),
                bytes,
                &updated_at,
            )?);
        }
    }
    debug_assert!(generated.iter().all(|projection| {
        projection.record.application_id == *application_id
            && projection.record.application_revision == application_revision
    }));
    Ok(generated)
}

#[allow(clippy::too_many_arguments)]
fn generated_projection(
    stored: &StoredApplicationModelV3,
    deliverable: Option<&DeliverableRecordV3>,
    relative_path: SafeRelativePath,
    kind: ApplicationProjectionKindV3,
    source_sha256: Sha256Digest,
    bytes: Vec<u8>,
    updated_at: &canisend_contracts::UtcTimestamp,
) -> Result<GeneratedProjectionV3, StoreError> {
    let generated_sha256 = digest_bytes(&bytes)?;
    Ok(GeneratedProjectionV3 {
        record: ApplicationProjectionRecordV3 {
            application_id: stored.snapshot.application.id.clone(),
            application_revision: stored.snapshot.application.revision,
            snapshot_sha256: stored.snapshot_sha256.clone(),
            pack: stored.snapshot.pack.clone(),
            deliverable_id: deliverable.map(|value| value.id.clone()),
            deliverable_revision: deliverable.map(|value| value.revision),
            relative_path,
            kind,
            source_sha256,
            generated_sha256,
            observed_sha256: None,
            edit_status: ProjectionEditStatus::Missing,
            superseded: false,
            updated_at: updated_at.clone(),
        },
        bytes,
    })
}

fn generate_projection_bytes_from_database(
    connection: &Connection,
    blobs: &BlobStore,
    row: &ApplicationProjectionRecordV3,
) -> Result<Vec<u8>, StoreError> {
    let stored =
        load_application_model_revision(connection, &row.application_id, row.application_revision)?;
    if stored.snapshot_sha256 != row.snapshot_sha256 || stored.snapshot.pack != row.pack {
        return Err(StoreError::ApplicationModelIntegrity(
            "Application projection binding differs from its immutable snapshot".to_owned(),
        ));
    }
    match row.kind {
        ApplicationProjectionKindV3::ApplicationModelJson => {
            pretty_json_bytes(&serde_json::to_value(stored.snapshot)?)
        }
        ApplicationProjectionKindV3::DeliverableMetadataJson => {
            let deliverable = find_deliverable(&stored, row)?;
            pretty_json_bytes(&serde_json::to_value(deliverable)?)
        }
        ApplicationProjectionKindV3::DeliverableContent => {
            let deliverable = find_deliverable(&stored, row)?;
            let content = deliverable.content.as_ref().ok_or_else(|| {
                StoreError::ApplicationModelIntegrity(
                    "Deliverable content projection has no content binding".to_owned(),
                )
            })?;
            if content.sha256 != row.source_sha256 {
                return Err(StoreError::ApplicationModelIntegrity(
                    "Deliverable projection source digest differs from its snapshot".to_owned(),
                ));
            }
            blobs.read_verified(&content.sha256, DEFAULT_MAX_BLOB_BYTES)
        }
    }
}

fn find_deliverable<'a>(
    stored: &'a StoredApplicationModelV3,
    row: &ApplicationProjectionRecordV3,
) -> Result<&'a DeliverableRecordV3, StoreError> {
    let deliverable_id = row.deliverable_id.as_ref().ok_or_else(|| {
        StoreError::ApplicationModelIntegrity(
            "Deliverable projection has no Deliverable identity".to_owned(),
        )
    })?;
    let deliverable = stored
        .snapshot
        .deliverables
        .iter()
        .find(|value| value.id == *deliverable_id)
        .ok_or_else(|| {
            StoreError::ApplicationModelIntegrity(format!(
                "Deliverable {deliverable_id} is absent from its projection snapshot"
            ))
        })?;
    if Some(deliverable.revision) != row.deliverable_revision {
        return Err(StoreError::ApplicationModelIntegrity(
            "Deliverable projection revision differs from its snapshot".to_owned(),
        ));
    }
    Ok(deliverable)
}

fn repair_projection(
    connection: &Connection,
    blobs: &BlobStore,
    workspace_root: &Path,
    row: &ApplicationProjectionRecordV3,
) -> Result<bool, StoreError> {
    let repaired_at = now_utc()?;
    let (status, observed, last_error) =
        observe_projection(workspace_root, &row.relative_path, &row.generated_sha256);
    update_observation(
        connection,
        &row.relative_path,
        status,
        observed.as_ref(),
        last_error.as_deref(),
        &repaired_at,
    )?;
    if matches!(
        status,
        ProjectionEditStatus::Current | ProjectionEditStatus::Edited
    ) {
        return Ok(false);
    }
    let bytes = match generate_projection_bytes_from_database(connection, blobs, row) {
        Ok(bytes) => bytes,
        Err(error) => {
            update_observation(
                connection,
                &row.relative_path,
                ProjectionEditStatus::RepairRequired,
                None,
                Some(&error.to_string()),
                &repaired_at,
            )?;
            return Err(error);
        }
    };
    if digest_bytes(&bytes)? != row.generated_sha256 {
        let error = StoreError::DependencyConflict(format!(
            "projection recipe changed for {}",
            row.relative_path
        ));
        update_observation(
            connection,
            &row.relative_path,
            ProjectionEditStatus::RepairRequired,
            None,
            Some(&error.to_string()),
            &repaired_at,
        )?;
        return Err(error);
    }
    if let Err(error) = write_projection(workspace_root, &row.relative_path, &bytes) {
        update_observation(
            connection,
            &row.relative_path,
            ProjectionEditStatus::RepairRequired,
            None,
            Some(&error.to_string()),
            &repaired_at,
        )?;
        return Err(error);
    }
    update_observation(
        connection,
        &row.relative_path,
        ProjectionEditStatus::Current,
        Some(&row.generated_sha256),
        None,
        &repaired_at,
    )?;
    Ok(true)
}

fn verify_current_application(
    connection: &Connection,
    stored: &StoredApplicationModelV3,
) -> Result<(), StoreError> {
    let current: Option<(i64, String, String, String, String)> = connection
        .query_row(
            "SELECT head.head_revision, revision.snapshot_sha256,
                    head.pack_id, head.pack_version, head.pack_digest
             FROM application_model_v3_heads AS head
             JOIN application_model_v3_revisions AS revision
               ON revision.application_id = head.application_id
              AND revision.revision = head.head_revision
             WHERE head.application_id = ?1",
            [stored.snapshot.application.id.as_str()],
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
    let expected = (
        to_i64(stored.snapshot.application.revision.get())?,
        stored.snapshot_sha256.to_string(),
        stored.snapshot.pack.id.to_string(),
        stored.snapshot.pack.version.to_string(),
        stored.snapshot.pack.content_digest.to_string(),
    );
    if current == Some(expected) {
        Ok(())
    } else {
        Err(StoreError::TaskStale(
            "Application changed while publishing projections".to_owned(),
        ))
    }
}

fn record_pending_projection(
    connection: &Connection,
    record: &ApplicationProjectionRecordV3,
    updated_at: &canisend_contracts::UtcTimestamp,
) -> Result<(), StoreError> {
    connection.execute(
        "INSERT INTO application_projection_v3_manifests(
            application_id, application_revision, snapshot_sha256,
            pack_id, pack_version, pack_digest, relative_path, projection_kind,
            deliverable_id, deliverable_revision, source_sha256, generated_sha256,
            observed_sha256, status, last_error, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                   NULL, 'missing', NULL, ?13)
         ON CONFLICT(relative_path) DO UPDATE SET
            application_id = excluded.application_id,
            application_revision = excluded.application_revision,
            snapshot_sha256 = excluded.snapshot_sha256,
            pack_id = excluded.pack_id,
            pack_version = excluded.pack_version,
            pack_digest = excluded.pack_digest,
            projection_kind = excluded.projection_kind,
            deliverable_id = excluded.deliverable_id,
            deliverable_revision = excluded.deliverable_revision,
            source_sha256 = excluded.source_sha256,
            generated_sha256 = excluded.generated_sha256,
            observed_sha256 = NULL,
            status = 'missing',
            last_error = NULL,
            updated_at = excluded.updated_at",
        params![
            record.application_id.as_str(),
            to_i64(record.application_revision.get())?,
            record.snapshot_sha256.as_str(),
            record.pack.id.as_str(),
            record.pack.version.as_str(),
            record.pack.content_digest.as_str(),
            record.relative_path.as_str(),
            enum_name(record.kind)?,
            record.deliverable_id.as_ref().map(DeliverableId::as_str),
            record
                .deliverable_revision
                .map(Revision::get)
                .map(to_i64)
                .transpose()?,
            record.source_sha256.as_str(),
            record.generated_sha256.as_str(),
            updated_at.as_str(),
        ],
    )?;
    Ok(())
}

fn load_all_projection_rows(
    connection: &Connection,
) -> Result<Vec<ApplicationProjectionRecordV3>, StoreError> {
    query_projection_rows(
        connection,
        "SELECT manifest.application_id, manifest.application_revision, manifest.snapshot_sha256,
                manifest.pack_id, manifest.pack_version, manifest.pack_digest,
                manifest.relative_path, manifest.projection_kind,
                manifest.deliverable_id, manifest.deliverable_revision,
                manifest.source_sha256, manifest.generated_sha256,
                manifest.observed_sha256, manifest.status, manifest.updated_at
         FROM application_projection_v3_manifests AS manifest
         JOIN application_model_v3_heads AS head
           ON head.application_id = manifest.application_id
          AND head.head_revision = manifest.application_revision
          AND head.pack_id = manifest.pack_id
          AND head.pack_version = manifest.pack_version
          AND head.pack_digest = manifest.pack_digest
         ORDER BY relative_path",
        [],
    )
}

fn load_projection_rows(
    connection: &Connection,
    application_id: &ApplicationId,
) -> Result<Vec<ApplicationProjectionRecordV3>, StoreError> {
    query_projection_rows(
        connection,
        "SELECT application_id, application_revision, snapshot_sha256,
                pack_id, pack_version, pack_digest, relative_path, projection_kind,
                deliverable_id, deliverable_revision, source_sha256, generated_sha256,
                observed_sha256, status, updated_at
         FROM application_projection_v3_manifests
         WHERE application_id = ?1 ORDER BY relative_path",
        [application_id.as_str()],
    )
}

fn query_projection_rows<P>(
    connection: &Connection,
    sql: &str,
    parameters: P,
) -> Result<Vec<ApplicationProjectionRecordV3>, StoreError>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map(parameters, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter().map(parse_projection_row).collect()
}

#[allow(clippy::type_complexity)]
fn parse_projection_row(
    row: (
        String,
        i64,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<i64>,
        String,
        String,
        Option<String>,
        String,
        String,
    ),
) -> Result<ApplicationProjectionRecordV3, StoreError> {
    let (
        application_id,
        application_revision,
        snapshot_sha256,
        pack_id,
        pack_version,
        pack_digest,
        relative_path,
        kind,
        deliverable_id,
        deliverable_revision,
        source_sha256,
        generated_sha256,
        observed_sha256,
        status,
        updated_at,
    ) = row;
    Ok(ApplicationProjectionRecordV3 {
        application_id: ApplicationId::try_new(application_id)?,
        application_revision: Revision::try_new(to_u64(application_revision)?)?,
        snapshot_sha256: Sha256Digest::try_new(snapshot_sha256)?,
        pack: ApplicationPackBindingV3 {
            id: canisend_contracts::WorkflowPackId::try_new(pack_id).map_err(|error| {
                StoreError::ApplicationModelIntegrity(format!(
                    "stored projection Pack identity is invalid: {error}"
                ))
            })?,
            version: canisend_contracts::SemanticVersion::try_new(pack_version)?,
            content_digest: Sha256Digest::try_new(pack_digest)?,
        },
        deliverable_id: deliverable_id.map(DeliverableId::try_new).transpose()?,
        deliverable_revision: deliverable_revision
            .map(to_u64)
            .transpose()?
            .map(Revision::try_new)
            .transpose()?,
        relative_path: SafeRelativePath::try_new(relative_path)?,
        kind: enum_value(&kind)?,
        source_sha256: Sha256Digest::try_new(source_sha256)?,
        generated_sha256: Sha256Digest::try_new(generated_sha256)?,
        observed_sha256: observed_sha256.map(Sha256Digest::try_new).transpose()?,
        edit_status: enum_value(&status)?,
        superseded: false,
        updated_at: canisend_contracts::UtcTimestamp::try_new(updated_at)?,
    })
}

fn load_projection_row(
    connection: &Connection,
    relative_path: &SafeRelativePath,
) -> Result<Option<ApplicationProjectionRecordV3>, StoreError> {
    let rows = query_projection_rows(
        connection,
        "SELECT application_id, application_revision, snapshot_sha256,
                pack_id, pack_version, pack_digest, relative_path, projection_kind,
                deliverable_id, deliverable_revision, source_sha256, generated_sha256,
                observed_sha256, status, updated_at
         FROM application_projection_v3_manifests WHERE relative_path = ?1",
        [relative_path.as_str()],
    )?;
    Ok(rows.into_iter().next())
}

fn recognize_legacy_projections(
    connection: &Connection,
    workspace_root: &Path,
    application_id: &ApplicationId,
) -> Result<Vec<ApplicationLegacyProjectionV3>, StoreError> {
    let legacy_job_id: Option<String> = connection
        .query_row(
            "SELECT legacy_job_id FROM workspace_v3_application_links
             WHERE application_id = ?1 ORDER BY migration_id DESC LIMIT 1",
            [application_id.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    let Some(legacy_job_id) = legacy_job_id else {
        return Ok(Vec::new());
    };
    let prefix = format!("jobs/{legacy_job_id}/");
    let mut statement = connection.prepare(
        "SELECT relative_path, projection_kind, generated_sha256
         FROM projection_manifests
         WHERE relative_path LIKE ?1 ORDER BY relative_path",
    )?;
    let rows = statement
        .query_map([format!("{prefix}%")], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let legacy_job_id = canisend_contracts::EntityId::try_new(legacy_job_id)?;
    rows.into_iter()
        .map(|(path, kind, generated)| {
            let relative_path = SafeRelativePath::try_new(path)?;
            if !relative_path.as_str().starts_with(&prefix) {
                return Err(StoreError::ApplicationModelIntegrity(
                    "legacy projection escaped its exact Job path".to_owned(),
                ));
            }
            let generated_sha256 = Sha256Digest::try_new(generated)?;
            let (edit_status, observed_sha256, _) =
                observe_projection(workspace_root, &relative_path, &generated_sha256);
            Ok(ApplicationLegacyProjectionV3 {
                application_id: application_id.clone(),
                legacy_job_id: legacy_job_id.clone(),
                relative_path,
                legacy_projection_kind: kind,
                generated_sha256,
                observed_sha256,
                edit_status,
            })
        })
        .collect()
}

fn preflight_projection(
    connection: &Connection,
    workspace_root: &Path,
    relative_path: &SafeRelativePath,
) -> Result<(), StoreError> {
    preflight_parent_chain(workspace_root, relative_path)?;
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

fn preflight_parent_chain(
    workspace_root: &Path,
    relative_path: &SafeRelativePath,
) -> Result<(), StoreError> {
    let destination = workspace_root.join(relative_path.as_str());
    let parent = destination
        .parent()
        .ok_or(StoreError::ProjectionPathRejected)?;
    let relative_parent = parent
        .strip_prefix(workspace_root)
        .map_err(|_| StoreError::ProjectionPathRejected)?;
    let mut current = workspace_root.to_path_buf();
    for component in relative_parent.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(StoreError::UnsafePath(current));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(source) => return Err(io_error(current, source)),
        }
    }
    Ok(())
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
        "UPDATE application_projection_v3_manifests
         SET observed_sha256 = ?2, status = ?3, last_error = ?4, updated_at = ?5
         WHERE relative_path = ?1",
        params![
            relative_path.as_str(),
            observed.map(Sha256Digest::as_str),
            enum_name(status)?,
            last_error,
            updated_at.as_str(),
        ],
    )?;
    if updated != 1 {
        return Err(StoreError::ProjectionNotFound(relative_path.to_string()));
    }
    Ok(())
}

fn ensure_application_destination(
    application_id: &ApplicationId,
    destination: &SafeRelativePath,
) -> Result<(), StoreError> {
    let expected = format!("applications/{application_id}/");
    if !destination.as_str().starts_with(&expected) {
        return Err(StoreError::ProjectionPathRejected);
    }
    Ok(())
}

fn preflight_new_user_copy(
    connection: &Connection,
    workspace_root: &Path,
    destination: &SafeRelativePath,
) -> Result<(), StoreError> {
    preflight_parent_chain(workspace_root, destination)?;
    if load_projection_row(connection, destination)?.is_some() {
        return Err(StoreError::ProjectionUnmanagedConflict(
            destination.to_string(),
        ));
    }
    match fs::symlink_metadata(workspace_root.join(destination.as_str())) {
        Ok(_) => Err(StoreError::ProjectionUnmanagedConflict(
            destination.to_string(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(workspace_root.join(destination.as_str()), source)),
    }
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

fn media_type_extension(media_type: Option<&str>) -> &'static str {
    match media_type {
        Some("text/markdown") => ".md",
        Some("text/plain") => ".txt",
        Some("text/html") => ".html",
        Some("application/json") => ".json",
        Some("application/pdf") => ".pdf",
        _ => ".bin",
    }
}

fn pretty_json_bytes(value: &Value) -> Result<Vec<u8>, StoreError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn digest_bytes(bytes: &[u8]) -> Result<Sha256Digest, StoreError> {
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes))).map_err(StoreError::from)
}

fn enum_name<T: Serialize>(value: T) -> Result<String, StoreError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| StoreError::Invariant("enum did not serialize as a string".to_owned()))
}

fn enum_value<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, StoreError> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(StoreError::from)
}

fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::Invariant("value exceeds SQLite i64".to_owned()))
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite value".to_owned()))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ActorKind, ApplicationLifecycleV3, ApplicationModelFormatV3, ApplicationRecordV3,
        ContentRevisionReferenceV3, DeliverableKindId, DeliverableRecordV3, DeliverableStateV3,
        EntityId, ExecutionMode, OpportunityId, OpportunityRecordV3, PlanId, PlanRecordV3,
        PlanRevisionReferenceV3, PlanStateV3, PlannedDeliverableDispositionV3,
        PlannedDeliverableV3, PrivacyClassification, SemanticVersion, SourceKind, UtcTimestamp,
        WorkflowPackId, WorkflowPackItemId,
    };

    use super::*;
    use crate::{
        ApplicationModelRepository, ArtifactService, JobService, NewSource, ProjectionService,
        Workspace, application_v3::activate_workspace_v3_authority,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            Self(std::env::temp_dir().join(format!(
                "canisend-application-projection-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entity(suffix: u16) -> EntityId {
        EntityId::try_new(format!("019f2f55-7c00-7000-8000-{suffix:012}")).expect("Entity ID")
    }

    fn application_id(suffix: u16) -> ApplicationId {
        ApplicationId::try_new(entity(suffix).to_string()).expect("Application ID")
    }

    fn opportunity_id(suffix: u16) -> OpportunityId {
        OpportunityId::try_new(entity(suffix).to_string()).expect("Opportunity ID")
    }

    fn deliverable_id(suffix: u16) -> DeliverableId {
        DeliverableId::try_new(entity(suffix).to_string()).expect("Deliverable ID")
    }

    fn plan_id(suffix: u16) -> PlanId {
        PlanId::try_new(entity(suffix).to_string()).expect("Plan ID")
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn revision(value: u64) -> Revision {
        Revision::try_new(value).expect("revision")
    }

    fn timestamp() -> UtcTimestamp {
        UtcTimestamp::try_new("2026-08-02T16:00:00Z").expect("timestamp")
    }

    fn pack(id: &str, digest: char) -> ApplicationPackBindingV3 {
        ApplicationPackBindingV3 {
            id: WorkflowPackId::try_new(id).expect("Pack ID"),
            version: SemanticVersion::try_new("1.0.0").expect("Pack version"),
            content_digest: Sha256Digest::try_new(digest.to_string().repeat(64)).expect("digest"),
        }
    }

    fn snapshot(
        suffix: u16,
        pack: ApplicationPackBindingV3,
        opportunity: Option<OpportunityId>,
        content_sha256: Sha256Digest,
    ) -> canisend_contracts::ApplicationModelSnapshotV3 {
        let application_id = application_id(suffix);
        let opportunity_id = opportunity.unwrap_or_else(|| opportunity_id(suffix + 1));
        let plan_id = plan_id(suffix + 2);
        let kind = DeliverableKindId::from_parts(&pack.id, &item("proposal"));
        canisend_contracts::ApplicationModelSnapshotV3 {
            format: ApplicationModelFormatV3::V3,
            pack: pack.clone(),
            opportunity: OpportunityRecordV3 {
                id: opportunity_id.clone(),
                pack: pack.clone(),
                title: "Community funding opportunity".to_owned(),
                metadata: BTreeMap::new(),
                source_ids: Vec::new(),
                created_at: timestamp(),
                revision: revision(1),
                archived: false,
            },
            application: ApplicationRecordV3 {
                id: application_id.clone(),
                opportunity_id,
                pack: pack.clone(),
                metadata: BTreeMap::new(),
                lifecycle: ApplicationLifecycleV3::Active,
                created_at: timestamp(),
                updated_at: timestamp(),
                revision: revision(1),
            },
            requirements: Vec::new(),
            plan: Some(PlanRecordV3 {
                id: plan_id.clone(),
                application_id: application_id.clone(),
                pack: pack.clone(),
                state: PlanStateV3::Confirmed,
                decision: Some(item("proceed")),
                requirement_inputs: Vec::new(),
                deliverables: vec![PlannedDeliverableV3 {
                    kind: kind.clone(),
                    disposition: PlannedDeliverableDispositionV3::Required,
                    rationale: "The Pack requires one proposal.".to_owned(),
                    constraints: vec!["Use plain language".to_owned()],
                    execution_mode: Some(ExecutionMode::HostAgent),
                }],
                blockers: Vec::new(),
                decided_by: Some(ActorKind::User),
                decided_at: Some(timestamp()),
                revision: revision(1),
            }),
            deliverables: vec![DeliverableRecordV3 {
                id: deliverable_id(suffix + 3),
                application_id,
                pack,
                plan: PlanRevisionReferenceV3 {
                    id: plan_id,
                    revision: revision(1),
                },
                kind,
                title: "Public-benefit proposal".to_owned(),
                state: DeliverableStateV3::Draft,
                content: Some(ContentRevisionReferenceV3 {
                    id: entity(suffix + 4),
                    revision: revision(1),
                    sha256: content_sha256,
                }),
                media_type: Some("text/markdown".to_owned()),
                evidence_inputs: Vec::new(),
                revision: revision(1),
            }],
        }
    }

    fn create_v3_workspace(
        label: &str,
        pack: ApplicationPackBindingV3,
        content: Option<&[u8]>,
    ) -> (
        TestDirectory,
        Workspace,
        canisend_contracts::ApplicationModelSnapshotV3,
    ) {
        let root = TestDirectory::new(label);
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        activate_workspace_v3_authority(
            &mut workspace.database,
            ActorKind::User,
            "activate-projection-test",
        )
        .expect("activate v3");
        let content_sha256 = match content {
            Some(bytes) => workspace.blobs.put_bytes(bytes).expect("content Blob"),
            None => Sha256Digest::try_new("f".repeat(64)).expect("missing digest"),
        };
        let snapshot = snapshot(801, pack, None, content_sha256);
        ApplicationModelRepository::new(&mut workspace.database)
            .create(
                snapshot.clone(),
                ActorKind::User,
                "create-projection-fixture",
            )
            .expect("Application");
        (root, workspace, snapshot)
    }

    #[test]
    fn generic_projections_preserve_edits_copy_replace_and_repair() {
        let content = b"# Public benefit\n\nA locally reviewed proposal.\n";
        let (_root, mut workspace, stored_snapshot) = create_v3_workspace(
            "lifecycle",
            pack("org.canisend.generic-starter", 'a'),
            Some(content),
        );
        let workspace_root = workspace.paths.root.clone();
        let application_id = stored_snapshot.application.id.clone();
        let catalog = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .project(&application_id)
        .expect("project Application");
        assert_eq!(catalog.format, APPLICATION_PROJECTION_FORMAT_V3);
        assert_eq!(catalog.projections.len(), 3);
        assert!(catalog.legacy_projections.is_empty());
        assert!(!catalog.submission_performed);
        assert!(catalog.projections.iter().all(|projection| {
            projection
                .relative_path
                .as_str()
                .starts_with(&format!("applications/{application_id}/"))
                && projection.pack == stored_snapshot.pack
        }));

        let metadata = catalog
            .projections
            .iter()
            .find(|projection| {
                projection.kind == ApplicationProjectionKindV3::DeliverableMetadataJson
            })
            .expect("metadata projection")
            .relative_path
            .clone();
        let content_path = catalog
            .projections
            .iter()
            .find(|projection| projection.kind == ApplicationProjectionKindV3::DeliverableContent)
            .expect("content projection")
            .relative_path
            .clone();
        fs::write(
            workspace_root.join(metadata.as_str()),
            b"USER-EDITED-METADATA\n",
        )
        .expect("edit metadata");
        fs::remove_file(workspace_root.join(content_path.as_str())).expect("remove content");

        assert_eq!(
            ProjectionService::new(&mut workspace.database, &workspace.blobs, &workspace_root,)
                .repair_all()
                .expect("repair projections"),
            1
        );
        assert_eq!(
            fs::read(workspace_root.join(content_path.as_str())).expect("repaired content"),
            content
        );
        assert_eq!(
            fs::read(workspace_root.join(metadata.as_str())).expect("preserved edit"),
            b"USER-EDITED-METADATA\n"
        );

        ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .replace(&application_id, &metadata)
        .expect("replace edited metadata");
        assert_ne!(
            fs::read(workspace_root.join(metadata.as_str())).expect("restored metadata"),
            b"USER-EDITED-METADATA\n"
        );

        let user_content = b"USER-EDITED-CONTENT\n";
        fs::write(workspace_root.join(content_path.as_str()), user_content).expect("edit content");
        fs::remove_file(workspace_root.join(metadata.as_str())).expect("remove managed target");
        let error = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .copy_as_new(&application_id, &content_path, &metadata)
        .expect_err("managed destination must not become a user copy");
        assert!(matches!(error, StoreError::ProjectionUnmanagedConflict(_)));
        assert!(!workspace_root.join(metadata.as_str()).exists());
        ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .replace(&application_id, &metadata)
        .expect("restore missing managed destination");
        let copy_path = SafeRelativePath::try_new(format!(
            "applications/{application_id}/user-edits/proposal.md"
        ))
        .expect("copy path");
        let reconciled = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .copy_as_new(&application_id, &content_path, &copy_path)
        .expect("copy edit and restore");
        assert_eq!(reconciled.action, ProjectionReconcileAction::CopyAsNew);
        assert!(!reconciled.authoritative_changed);
        assert_eq!(
            fs::read(workspace_root.join(copy_path.as_str())).expect("user copy"),
            user_content
        );
        assert_eq!(
            fs::read(workspace_root.join(content_path.as_str())).expect("restored content"),
            content
        );
        let stored = ApplicationModelRepository::new(&mut workspace.database)
            .get(&application_id)
            .expect("Application remains current");
        assert_eq!(stored.snapshot.application.revision, revision(1));
    }

    #[test]
    fn backup_restore_rebuilds_generic_projections_from_authoritative_content() {
        let content = b"# Restored proposal\n\nThe immutable content survives backup.\n";
        let (_root, mut workspace, stored_snapshot) = create_v3_workspace(
            "backup-source",
            pack("org.canisend.generic-starter", 'd'),
            Some(content),
        );
        let application_id = stored_snapshot.application.id.clone();
        let content_sha256 = stored_snapshot.deliverables[0]
            .content
            .as_ref()
            .expect("content reference")
            .sha256
            .clone();
        let reference_count: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM blob_references
                 WHERE sha256 = ?1 AND owner_type = 'application-v3-content'",
                [content_sha256.as_str()],
                |row| row.get(0),
            )
            .expect("v3 Blob reference before projection");
        assert_eq!(reference_count, 1);
        let workspace_root = workspace.paths.root.clone();
        let catalog = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .project(&application_id)
        .expect("project Application");
        let expected = catalog
            .projections
            .iter()
            .map(|projection| {
                (
                    projection.relative_path.clone(),
                    fs::read(workspace_root.join(projection.relative_path.as_str()))
                        .expect("source projection"),
                )
            })
            .collect::<Vec<_>>();

        let backup_parent = TestDirectory::new("backup-container");
        let restored_parent = TestDirectory::new("backup-restore");
        let backup = backup_parent.path().join("snapshot");
        let destination = restored_parent.path().join("workspace");
        let backup_result = workspace.backup(&backup).expect("backup");
        assert!(
            backup_result
                .manifest
                .blobs
                .iter()
                .any(|blob| blob.sha256 == content_sha256)
        );

        let mut restored = Workspace::restore(&backup, &destination).expect("restore");
        for (relative_path, bytes) in expected {
            assert_eq!(
                fs::read(destination.join(relative_path.as_str())).expect("restored projection"),
                bytes
            );
        }
        let restored_root = restored.paths.root.clone();
        let restored_catalog = ApplicationProjectionService::new(
            &mut restored.database,
            &restored.blobs,
            &restored_root,
        )
        .catalog(&application_id)
        .expect("restored catalog");
        assert!(
            restored_catalog
                .projections
                .iter()
                .all(|projection| { projection.edit_status == ProjectionEditStatus::Current })
        );
        assert_eq!(
            ProjectionService::new(&mut restored.database, &restored.blobs, &restored_root,)
                .repair_all()
                .expect("idempotent repair"),
            0
        );
    }

    #[test]
    fn unmanaged_missing_blob_and_symlink_paths_fail_before_projection_ownership() {
        let (_root, mut workspace, stored_snapshot) = create_v3_workspace(
            "unmanaged",
            pack("org.canisend.generic-starter", 'b'),
            Some(b"content"),
        );
        let application_id = stored_snapshot.application.id.clone();
        let workspace_root = workspace.paths.root.clone();
        let application_root = workspace_root.join(format!("applications/{application_id}"));
        fs::create_dir_all(&application_root).expect("Application directory");
        let unmanaged = application_root.join("application.json");
        fs::write(&unmanaged, b"user-owned").expect("unmanaged file");
        let error = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .project(&application_id)
        .expect_err("unmanaged file must block projection");
        assert!(matches!(error, StoreError::ProjectionUnmanagedConflict(_)));
        let count: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_projection_v3_manifests",
                [],
                |row| row.get(0),
            )
            .expect("manifest count");
        assert_eq!(count, 0);
        assert_eq!(
            fs::read(&unmanaged).expect("unmanaged remains"),
            b"user-owned"
        );

        fs::remove_file(&unmanaged).expect("remove unmanaged fixture");
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let outside = TestDirectory::new("symlink-outside");
            fs::create_dir_all(outside.path()).expect("outside directory");
            let sentinel = outside.path().join("sentinel.txt");
            fs::write(&sentinel, b"outside").expect("outside sentinel");
            symlink(outside.path(), application_root.join("deliverables"))
                .expect("symlinked projection parent");
            let error = ApplicationProjectionService::new(
                &mut workspace.database,
                &workspace.blobs,
                &workspace_root,
            )
            .project(&application_id)
            .expect_err("symlinked parent must fail closed");
            assert!(matches!(error, StoreError::UnsafePath(_)));
            assert!(!unmanaged.exists());
            assert_eq!(fs::read(&sentinel).expect("outside remains"), b"outside");
            let count: i64 = workspace
                .database
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM application_projection_v3_manifests",
                    [],
                    |row| row.get(0),
                )
                .expect("manifest count");
            assert_eq!(count, 0);
        }

        let missing_root = TestDirectory::new("missing-blob");
        let mut missing_workspace = Workspace::init(missing_root.path()).expect("Workspace");
        activate_workspace_v3_authority(
            &mut missing_workspace.database,
            ActorKind::User,
            "activate-projection-test",
        )
        .expect("activate v3");
        let missing_snapshot = snapshot(
            901,
            pack("org.canisend.generic-starter", 'c'),
            None,
            Sha256Digest::try_new("f".repeat(64)).expect("missing digest"),
        );
        let missing_id = missing_snapshot.application.id.clone();
        ApplicationModelRepository::new(&mut missing_workspace.database)
            .create(
                missing_snapshot,
                ActorKind::User,
                "create-missing-content-fixture",
            )
            .expect("Application");
        let missing_workspace_root = missing_workspace.paths.root.clone();
        let error = ApplicationProjectionService::new(
            &mut missing_workspace.database,
            &missing_workspace.blobs,
            &missing_workspace_root,
        )
        .project(&missing_id)
        .expect_err("missing content Blob must fail before projection");
        assert!(matches!(error, StoreError::BlobMissing(_)));
        assert!(
            !missing_workspace_root
                .join(format!("applications/{missing_id}/application.json"))
                .exists()
        );
    }

    #[test]
    fn migrated_academic_legacy_projection_is_recognized_but_never_reowned() {
        let root = TestDirectory::new("legacy");
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Lecturer", "University", ActorKind::User)
            .expect("Job");
        let source = JobService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                &job.id,
                NewSource {
                    kind: SourceKind::LocalFile,
                    original_bytes: b"legacy source".to_vec(),
                    normalized_text: "legacy source\n".to_owned(),
                    source_url: None,
                    final_url: None,
                    content_type: "text/plain; charset=utf-8".to_owned(),
                    redirect_chain: Vec::new(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("source");
        let legacy_path =
            SafeRelativePath::try_new(format!("jobs/{}/source/legacy-source.txt", job.id))
                .expect("legacy path");
        ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        )
        .project(&source.original.id, source.original.revision, &legacy_path)
        .expect("legacy projection");
        let legacy_bytes = fs::read(workspace.paths.root.join(legacy_path.as_str()))
            .expect("legacy projection bytes");
        let academic_pack = pack("org.canisend.academic-job", 'd');
        activate_workspace_v3_authority(
            &mut workspace.database,
            ActorKind::User,
            "activate-projection-test",
        )
        .expect("activate v3");
        let content_sha256 = workspace
            .blobs
            .put_bytes(b"academic projection content")
            .expect("content Blob");
        let snapshot = snapshot(
            951,
            academic_pack.clone(),
            Some(OpportunityId::try_new(job.id.to_string()).expect("Opportunity ID")),
            content_sha256,
        );
        let application_id = snapshot.application.id.clone();
        ApplicationModelRepository::new(&mut workspace.database)
            .create(
                snapshot.clone(),
                ActorKind::User,
                "create-academic-projection-fixture",
            )
            .expect("Application");
        let migration_id = entity(999);
        let recorded_at = timestamp();
        workspace
            .database
            .connection()
            .execute(
                "INSERT INTO workspace_v3_migrations(
                    id, source_workspace_format, target_workspace_format,
                    source_schema_version, target_schema_version,
                    pack_id, pack_version, pack_digest, preview_sha256,
                    source_inventory_sha256, source_inventory_count,
                    referenced_blob_count, referenced_blob_bytes,
                    backup_manifest_sha256, started_at, completed_at
                 ) VALUES (?1, 'canisend.workspace/v2', 'canisend.workspace/v3',
                           13, 16, ?2, ?3, ?4, ?5, ?6, 0, 0, 0, ?7, ?8, ?8)",
                params![
                    migration_id.as_str(),
                    academic_pack.id.as_str(),
                    academic_pack.version.as_str(),
                    academic_pack.content_digest.as_str(),
                    "a".repeat(64),
                    "b".repeat(64),
                    "c".repeat(64),
                    recorded_at.as_str(),
                ],
            )
            .expect("migration ledger fixture");
        workspace
            .database
            .connection()
            .execute(
                "INSERT INTO workspace_v3_application_links(
                    migration_id, legacy_job_id, opportunity_id, application_id
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    migration_id.as_str(),
                    job.id.as_str(),
                    snapshot.opportunity.id.as_str(),
                    application_id.as_str(),
                ],
            )
            .expect("legacy link fixture");
        let unmanaged_legacy = workspace
            .paths
            .root
            .join(format!("jobs/{}/unmanaged.txt", job.id));
        fs::write(&unmanaged_legacy, b"unmanaged legacy file").expect("unmanaged legacy file");
        let workspace_root = workspace.paths.root.clone();

        let catalog = ApplicationProjectionService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .project(&application_id)
        .expect("project migrated Application");
        assert_eq!(catalog.legacy_projections.len(), 1);
        assert_eq!(catalog.legacy_projections[0].relative_path, legacy_path);
        assert_eq!(catalog.legacy_projections[0].legacy_job_id, job.id);
        assert_eq!(
            fs::read(workspace_root.join(legacy_path.as_str())).expect("legacy projection remains"),
            legacy_bytes
        );
        assert_eq!(
            fs::read(&unmanaged_legacy).expect("unmanaged legacy remains"),
            b"unmanaged legacy file"
        );
        assert!(catalog.projections.iter().all(|projection| {
            projection
                .relative_path
                .as_str()
                .starts_with("applications/")
        }));
    }
}
