use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use canisend_contracts::{
    ActorKind, ApplicationFieldValueV3, ApplicationId, ApplicationLifecycleV3,
    ApplicationModelFormatV3, ApplicationModelSnapshotV3, ApplicationPackBindingV3,
    ApplicationPlanRecord, ApplicationRecordV3, ArtifactKind, ArtifactReference, CitationTarget,
    ContentRevisionReferenceV3, ContentSpanV3, CriteriaSetRecord, CriterionImportance,
    DeliverableId, DeliverableKindId, DeliverableRecordV3, DeliverableStateV3, DocumentKind,
    DocumentRecord, DocumentRequirement, EntityId, EntityRevisionReferenceV3,
    EvidenceMatchSetRecord, OpportunityId, OpportunityRecordV3, ParsedJobRecord,
    PlanBlockerSeverityV3, PlanBlockerV3, PlanId, PlanRecordV3, PlanRevisionReferenceV3,
    PlanStateV3, PlannedDeliverableDispositionV3, PlannedDeliverableV3, RequirementConfirmationV3,
    RequirementId, RequirementPriorityV3, RequirementRecordV3, RequirementRevisionReferenceV3,
    Revision, SemanticValidate, Sha256Digest, UtcTimestamp, WORKSPACE_FORMAT, WorkflowPackItemId,
    validate_external_candidate,
};
use canisend_core::{VerifiedWorkflowPackBundle, WorkflowPackOrigin};
use rusqlite::{Connection, OptionalExtension, Transaction, params, types::ValueRef};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    BACKUP_FORMAT, BlobStore, DATABASE_SCHEMA_VERSION, DEFAULT_MAX_BLOB_BYTES, StoreError,
    WORKSPACE_V3_FORMAT, Workspace, WorkspaceV3AuthorityState, generate_id, now_utc,
};

use crate::application_v3::{insert_migrated_application_model, insert_workspace_v3_authority};

pub const WORKSPACE_V3_MIGRATION_PREVIEW_FORMAT: &str = "canisend.workspace-migration-preview/v3";
pub const WORKSPACE_V3_MIGRATION_RESULT_FORMAT: &str = "canisend.workspace-migration-result/v3";
pub const ACADEMIC_JOB_PACK_ID: &str = "org.canisend.academic-job";
pub const LEGACY_WORKSPACE_SCHEMA_VERSION: u32 = 13;
const BACKUP_METADATA_ALLOWANCE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
enum MigrationWriteBoundary {
    AuthorityActivated,
    ApplicationModelInserted { ordinal: usize },
    MigrationRecordInserted,
    ApplicationLinkInserted { ordinal: usize },
    LegacyBindingInserted { ordinal: usize },
    MigrationAuditInserted,
    PreCommitVerified,
}

trait MigrationWriteObserver {
    fn after_write(&mut self, boundary: MigrationWriteBoundary) -> Result<(), StoreError>;
}

struct NoopMigrationWriteObserver;

impl MigrationWriteObserver for NoopMigrationWriteObserver {
    fn after_write(&mut self, _boundary: MigrationWriteBoundary) -> Result<(), StoreError> {
        Ok(())
    }
}

const LEGACY_TABLES: [&str; 32] = [
    "schema_migrations",
    "workspace_metadata",
    "jobs",
    "sources",
    "source_revisions",
    "evidence_items",
    "evidence_revisions",
    "artifacts",
    "artifact_revisions",
    "artifact_dependencies",
    "blob_references",
    "workflow_runs",
    "stage_executions",
    "tasks",
    "task_inputs",
    "task_results",
    "consents",
    "audit_events",
    "projection_manifests",
    "discovery_sources",
    "job_leads",
    "provider_invocations",
    "discovery_refresh_receipts",
    "profile_sources",
    "profile_source_revisions",
    "application_plan_heads",
    "application_plan_documents",
    "document_heads",
    "review_heads",
    "package_heads",
    "export_heads",
    "render_heads",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceV3MigrationPreview {
    pub format: String,
    pub workspace_id: EntityId,
    pub source_workspace_format: String,
    pub target_workspace_format: String,
    pub database_schema_version: u32,
    pub legacy_schema_version: u32,
    pub pack: ApplicationPackBindingV3,
    pub application_count: u64,
    pub legacy_inventory_counts: BTreeMap<String, u64>,
    pub legacy_inventory_count: u64,
    pub legacy_inventory_sha256: Sha256Digest,
    pub referenced_blob_count: u64,
    pub referenced_blob_bytes: u64,
    pub required_backup_bytes: u64,
    pub projection_conflict_count: u64,
    pub rollback_boundary: String,
    pub migration_plan_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceV3MigrationResult {
    pub format: String,
    pub migration_id: EntityId,
    pub authority: WorkspaceV3AuthorityState,
    pub pack: ApplicationPackBindingV3,
    pub migration_plan_sha256: Sha256Digest,
    pub backup_format: String,
    pub backup_manifest_sha256: Sha256Digest,
    pub application_ids: Vec<ApplicationId>,
    pub legacy_binding_count: u64,
    pub source_inventory_sha256: Sha256Digest,
    pub post_migration_inventory_sha256: Sha256Digest,
    pub referenced_blob_count: u64,
    pub post_migration_referenced_blob_count: u64,
}

pub struct WorkspaceV3MigrationService<'a> {
    workspace: &'a mut Workspace,
}

impl<'a> WorkspaceV3MigrationService<'a> {
    #[must_use]
    pub fn new(workspace: &'a mut Workspace) -> Self {
        Self { workspace }
    }

    pub fn preview(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
    ) -> Result<WorkspaceV3MigrationPreview, StoreError> {
        Ok(self.build_plan(pack)?.preview)
    }

    pub fn migrate(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        expected_plan_sha256: &Sha256Digest,
        backup_destination: &Path,
    ) -> Result<WorkspaceV3MigrationResult, StoreError> {
        self.migrate_observed(
            pack,
            expected_plan_sha256,
            backup_destination,
            &mut NoopMigrationWriteObserver,
        )
    }

    fn migrate_observed(
        &mut self,
        pack: &VerifiedWorkflowPackBundle,
        expected_plan_sha256: &Sha256Digest,
        backup_destination: &Path,
        observer: &mut impl MigrationWriteObserver,
    ) -> Result<WorkspaceV3MigrationResult, StoreError> {
        let plan = self.build_plan(pack)?;
        if &plan.preview.migration_plan_sha256 != expected_plan_sha256 {
            return Err(StoreError::WorkspaceMigrationConflict(
                "migration preview is stale; run a new dry-run before migrating".to_owned(),
            ));
        }

        let backup = self.workspace.backup(backup_destination)?;
        let backup_manifest_path = backup.directory.join("backup-manifest.json");
        let backup_manifest_bytes = fs::read(&backup_manifest_path)
            .map_err(|source| crate::io_error(&backup_manifest_path, source))?;
        let backup_manifest_sha256 = digest_bytes(&backup_manifest_bytes)?;
        let migration_id = generate_id()?;
        let authority_event_id = generate_id()?;
        let migration_event_id = generate_id()?;
        let started_at = now_utc()?;
        let completed_at = started_at.clone();
        let actor = enum_name(ActorKind::User)?;
        let workspace_id = self.workspace.config.workspace_id.clone();
        let transaction = self.workspace.database.immediate_transaction()?;

        let current_inventory = legacy_inventory(&transaction, &plan.associations)?;
        if current_inventory.digest != plan.inventory.digest
            || current_inventory.counts != plan.inventory.counts
            || current_inventory.rows.len() != plan.inventory.rows.len()
        {
            return Err(StoreError::WorkspaceMigrationConflict(
                "legacy inventory changed after the dry-run".to_owned(),
            ));
        }
        let current_references = referenced_digests(&transaction)?;
        if current_references != plan.referenced_digests {
            return Err(StoreError::WorkspaceMigrationConflict(
                "referenced Blob inventory changed after the dry-run".to_owned(),
            ));
        }
        if projection_state_digest(&transaction)? != plan.projection_state_sha256 {
            return Err(StoreError::WorkspaceMigrationConflict(
                "projection state changed after the dry-run".to_owned(),
            ));
        }

        insert_workspace_v3_authority(
            &transaction,
            &workspace_id,
            authority_event_id.as_str(),
            &actor,
            "migrate-workspace-v2-to-v3",
            &completed_at,
        )?;
        observer.after_write(MigrationWriteBoundary::AuthorityActivated)?;
        let mut application_ids = Vec::with_capacity(plan.snapshots.len());
        for (ordinal, snapshot) in plan.snapshots.iter().enumerate() {
            insert_migrated_application_model(
                &transaction,
                snapshot,
                ActorKind::User,
                "migrate-legacy-application",
                &completed_at,
            )?;
            application_ids.push(snapshot.application.id.clone());
            observer.after_write(MigrationWriteBoundary::ApplicationModelInserted { ordinal })?;
        }

        insert_migration_record(
            &transaction,
            &migration_id,
            &plan.preview,
            &backup_manifest_sha256,
            &started_at,
            &completed_at,
        )?;
        observer.after_write(MigrationWriteBoundary::MigrationRecordInserted)?;
        for (ordinal, snapshot) in plan.snapshots.iter().enumerate() {
            transaction.execute(
                "INSERT INTO workspace_v3_application_links(
                    migration_id, legacy_job_id, opportunity_id, application_id
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    migration_id.as_str(),
                    snapshot.opportunity.id.as_str(),
                    snapshot.opportunity.id.as_str(),
                    snapshot.application.id.as_str(),
                ],
            )?;
            observer.after_write(MigrationWriteBoundary::ApplicationLinkInserted { ordinal })?;
        }
        for (ordinal, row) in plan.inventory.rows.iter().enumerate() {
            transaction.execute(
                "INSERT INTO workspace_v3_legacy_bindings(
                    migration_id, source_table, source_key_sha256, row_sha256,
                    application_id, pack_id, pack_version, pack_digest
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    migration_id.as_str(),
                    row.table,
                    row.key_sha256.as_str(),
                    row.row_sha256.as_str(),
                    row.application_id.as_ref().map(ApplicationId::as_str),
                    plan.preview.pack.id.as_str(),
                    plan.preview.pack.version.as_str(),
                    plan.preview.pack.content_digest.as_str(),
                ],
            )?;
            observer.after_write(MigrationWriteBoundary::LegacyBindingInserted { ordinal })?;
        }
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, ?2, 'workspace-v3.migrate', ?3, NULL, ?4, ?5)",
            params![
                migration_event_id.as_str(),
                actor,
                workspace_id.as_str(),
                "migrate-workspace-v2-to-v3",
                completed_at.as_str(),
            ],
        )?;
        observer.after_write(MigrationWriteBoundary::MigrationAuditInserted)?;

        verify_migrated_transaction(&transaction, &plan)?;
        observer.after_write(MigrationWriteBoundary::PreCommitVerified)?;
        transaction.commit()?;

        let authority = WorkspaceV3AuthorityState {
            workspace_format: WORKSPACE_V3_FORMAT.to_owned(),
            activated_at: completed_at,
            reason: "migrate-workspace-v2-to-v3".to_owned(),
        };
        Ok(WorkspaceV3MigrationResult {
            format: WORKSPACE_V3_MIGRATION_RESULT_FORMAT.to_owned(),
            migration_id,
            authority,
            pack: plan.preview.pack,
            migration_plan_sha256: plan.preview.migration_plan_sha256,
            backup_format: BACKUP_FORMAT.to_owned(),
            backup_manifest_sha256,
            application_ids,
            legacy_binding_count: u64::try_from(plan.inventory.rows.len())
                .expect("legacy inventory length fits u64"),
            source_inventory_sha256: plan.inventory.digest.clone(),
            post_migration_inventory_sha256: plan.inventory.digest,
            referenced_blob_count: u64::try_from(plan.referenced_digests.len())
                .expect("Blob inventory length fits u64"),
            post_migration_referenced_blob_count: u64::try_from(plan.referenced_digests.len())
                .expect("Blob inventory length fits u64"),
        })
    }

    fn build_plan(&self, pack: &VerifiedWorkflowPackBundle) -> Result<MigrationPlan, StoreError> {
        ensure_v2_authority(self.workspace.database.connection())?;
        let pack = academic_pack_binding(pack)?;
        let check = self.workspace.check()?;
        if !check.ok {
            return Err(StoreError::WorkspaceMigrationIntegrity(
                "Workspace integrity or referenced-Blob verification failed".to_owned(),
            ));
        }
        let associations = LegacyAssociations::load(
            self.workspace.database.connection(),
            &self.workspace.config.workspace_id,
        )?;
        let inventory = legacy_inventory(self.workspace.database.connection(), &associations)?;
        let snapshots = build_application_snapshots(
            self.workspace.database.connection(),
            &self.workspace.blobs,
            &pack,
            &associations.job_applications,
        )?;
        let referenced_digests = self.workspace.database.referenced_digests()?;
        let mut referenced_blob_bytes = 0_u64;
        for digest in &referenced_digests {
            let digest = Sha256Digest::try_new(digest.clone())?;
            referenced_blob_bytes = referenced_blob_bytes
                .checked_add(
                    self.workspace
                        .blobs
                        .verify(&digest, DEFAULT_MAX_BLOB_BYTES)?,
                )
                .ok_or_else(|| {
                    StoreError::WorkspaceMigrationIntegrity(
                        "referenced Blob byte total overflowed".to_owned(),
                    )
                })?;
        }
        let projection_state_sha256 =
            projection_state_digest(self.workspace.database.connection())?;
        let required_backup_bytes = required_backup_bytes(self.workspace, referenced_blob_bytes)?;
        let projection_conflict_count =
            count_projection_conflicts(self.workspace.database.connection())?;
        let plan_sha256 = migration_plan_digest(
            &pack,
            &inventory,
            &snapshots,
            &referenced_digests,
            &projection_state_sha256,
        )?;
        let preview = WorkspaceV3MigrationPreview {
            format: WORKSPACE_V3_MIGRATION_PREVIEW_FORMAT.to_owned(),
            workspace_id: self.workspace.config.workspace_id.clone(),
            source_workspace_format: WORKSPACE_FORMAT.to_owned(),
            target_workspace_format: WORKSPACE_V3_FORMAT.to_owned(),
            database_schema_version: DATABASE_SCHEMA_VERSION,
            legacy_schema_version: LEGACY_WORKSPACE_SCHEMA_VERSION,
            pack,
            application_count: u64::try_from(snapshots.len()).expect("Application count fits u64"),
            legacy_inventory_counts: inventory.counts.clone(),
            legacy_inventory_count: u64::try_from(inventory.rows.len())
                .expect("legacy inventory length fits u64"),
            legacy_inventory_sha256: inventory.digest.clone(),
            referenced_blob_count: u64::try_from(referenced_digests.len())
                .expect("Blob count fits u64"),
            referenced_blob_bytes,
            required_backup_bytes,
            projection_conflict_count,
            rollback_boundary: "restore-verified-pre-migration-backup-to-new-path".to_owned(),
            migration_plan_sha256: plan_sha256,
        };
        Ok(MigrationPlan {
            preview,
            snapshots,
            inventory,
            associations,
            referenced_digests,
            projection_state_sha256,
        })
    }
}

struct MigrationPlan {
    preview: WorkspaceV3MigrationPreview,
    snapshots: Vec<ApplicationModelSnapshotV3>,
    inventory: LegacyInventory,
    associations: LegacyAssociations,
    referenced_digests: BTreeSet<String>,
    projection_state_sha256: Sha256Digest,
}

#[derive(Debug, Serialize)]
struct MigrationPlanDigest<'a> {
    pack: &'a ApplicationPackBindingV3,
    inventory_sha256: &'a Sha256Digest,
    inventory_counts: &'a BTreeMap<String, u64>,
    snapshots: &'a [ApplicationModelSnapshotV3],
    referenced_digests: &'a BTreeSet<String>,
    projection_state_sha256: &'a Sha256Digest,
}

fn migration_plan_digest(
    pack: &ApplicationPackBindingV3,
    inventory: &LegacyInventory,
    snapshots: &[ApplicationModelSnapshotV3],
    referenced_digests: &BTreeSet<String>,
    projection_state_sha256: &Sha256Digest,
) -> Result<Sha256Digest, StoreError> {
    digest_bytes(&serde_json::to_vec(&MigrationPlanDigest {
        pack,
        inventory_sha256: &inventory.digest,
        inventory_counts: &inventory.counts,
        snapshots,
        referenced_digests,
        projection_state_sha256,
    })?)
}

fn academic_pack_binding(
    pack: &VerifiedWorkflowPackBundle,
) -> Result<ApplicationPackBindingV3, StoreError> {
    if pack.snapshot().id().as_str() != ACADEMIC_JOB_PACK_ID
        || pack.snapshot().origin() != &WorkflowPackOrigin::BuiltIn
    {
        return Err(StoreError::WorkspaceMigrationConflict(format!(
            "Workspace v2 can migrate only to the verified built-in {ACADEMIC_JOB_PACK_ID} Pack"
        )));
    }
    let declared_deliverables = pack
        .manifest()
        .deliverables
        .kinds
        .iter()
        .map(|kind| kind.id.as_str())
        .collect::<BTreeSet<_>>();
    for required in [
        "cover-letter",
        "research-statement",
        "teaching-statement",
        "cv",
    ] {
        if !declared_deliverables.contains(required) {
            return Err(StoreError::WorkspaceMigrationConflict(format!(
                "verified academic Pack does not declare legacy Deliverable kind {required}"
            )));
        }
    }
    let declared_requirements = pack
        .manifest()
        .requirements
        .categories
        .iter()
        .map(|category| category.id.as_str())
        .collect::<BTreeSet<_>>();
    let declared_evidence = pack
        .manifest()
        .evidence
        .categories
        .iter()
        .map(|category| category.id.as_str())
        .collect::<BTreeSet<_>>();
    for required in [
        "qualification",
        "teaching",
        "research",
        "communication",
        "leadership",
        "service",
        "employment",
        "other",
    ] {
        if !declared_requirements.contains(required) {
            return Err(StoreError::WorkspaceMigrationConflict(format!(
                "verified academic Pack does not declare legacy Requirement category {required}"
            )));
        }
        if !declared_evidence.contains(required) {
            return Err(StoreError::WorkspaceMigrationConflict(format!(
                "verified academic Pack does not declare legacy Evidence category {required}"
            )));
        }
    }
    if !pack
        .manifest()
        .application
        .opportunity_fields
        .iter()
        .any(|field| field.id.as_str() == "institution")
    {
        return Err(StoreError::WorkspaceMigrationConflict(
            "verified academic Pack does not declare the legacy institution Opportunity field"
                .to_owned(),
        ));
    }
    Ok(ApplicationPackBindingV3 {
        id: pack.snapshot().id().clone(),
        version: pack.snapshot().version().clone(),
        content_digest: pack.snapshot().content_digest().clone(),
    })
}

fn ensure_v2_authority(connection: &Connection) -> Result<(), StoreError> {
    let current: Option<String> = connection
        .query_row(
            "SELECT workspace_format FROM workspace_v3_authority WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if current.is_some() {
        return Err(StoreError::WorkspaceMigrationConflict(
            "Workspace v3 authority is already active".to_owned(),
        ));
    }
    let source: String = connection.query_row(
        "SELECT workspace_format FROM workspace_metadata WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    if source != WORKSPACE_FORMAT {
        return Err(StoreError::WorkspaceMigrationConflict(format!(
            "unsupported source Workspace format {source}"
        )));
    }
    Ok(())
}

fn insert_migration_record(
    transaction: &Transaction<'_>,
    migration_id: &EntityId,
    preview: &WorkspaceV3MigrationPreview,
    backup_manifest_sha256: &Sha256Digest,
    started_at: &UtcTimestamp,
    completed_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO workspace_v3_migrations(
            id, source_workspace_format, target_workspace_format,
            source_schema_version, target_schema_version,
            pack_id, pack_version, pack_digest, preview_sha256,
            source_inventory_sha256, source_inventory_count,
            referenced_blob_count, referenced_blob_bytes,
            backup_manifest_sha256, started_at, completed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            migration_id.as_str(),
            preview.source_workspace_format,
            preview.target_workspace_format,
            i64::from(preview.legacy_schema_version),
            i64::from(preview.database_schema_version),
            preview.pack.id.as_str(),
            preview.pack.version.as_str(),
            preview.pack.content_digest.as_str(),
            preview.migration_plan_sha256.as_str(),
            preview.legacy_inventory_sha256.as_str(),
            to_i64(preview.legacy_inventory_count)?,
            to_i64(preview.referenced_blob_count)?,
            to_i64(preview.referenced_blob_bytes)?,
            backup_manifest_sha256.as_str(),
            started_at.as_str(),
            completed_at.as_str(),
        ],
    )?;
    Ok(())
}

fn verify_migrated_transaction(
    transaction: &Transaction<'_>,
    plan: &MigrationPlan,
) -> Result<(), StoreError> {
    let inventory = legacy_inventory(transaction, &plan.associations)?;
    if inventory.digest != plan.inventory.digest || inventory.counts != plan.inventory.counts {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "legacy semantic inventory changed during migration".to_owned(),
        ));
    }
    if referenced_digests(transaction)? != plan.referenced_digests {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "referenced Blob inventory changed during migration".to_owned(),
        ));
    }
    if projection_state_digest(transaction)? != plan.projection_state_sha256 {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "managed projection state changed during migration".to_owned(),
        ));
    }
    let authority: String = transaction.query_row(
        "SELECT workspace_format FROM workspace_v3_authority WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    if authority != WORKSPACE_V3_FORMAT {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "Workspace v3 authority was not activated".to_owned(),
        ));
    }
    let application_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM application_model_v3_heads",
        [],
        |row| row.get(0),
    )?;
    if to_u64(application_count)?
        != u64::try_from(plan.snapshots.len()).expect("Application count fits u64")
    {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "migrated Application count differs from the dry-run".to_owned(),
        ));
    }
    Ok(())
}

fn referenced_digests(connection: &Connection) -> Result<BTreeSet<String>, StoreError> {
    let mut statement =
        connection.prepare("SELECT DISTINCT sha256 FROM blob_references ORDER BY sha256")?;
    statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(StoreError::from)
}

fn projection_state_digest(connection: &Connection) -> Result<Sha256Digest, StoreError> {
    let mut statement = connection.prepare(
        "SELECT artifact_id, revision, relative_path, sha256, projection_kind,
                generated_sha256, observed_sha256, status, last_error, updated_at
         FROM projection_manifests ORDER BY relative_path",
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
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    digest_bytes(&serde_json::to_vec(&rows)?)
}

fn count_projection_conflicts(connection: &Connection) -> Result<u64, StoreError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM projection_manifests WHERE status != 'current'",
        [],
        |row| row.get(0),
    )?;
    to_u64(count)
}

fn required_backup_bytes(
    workspace: &Workspace,
    referenced_blob_bytes: u64,
) -> Result<u64, StoreError> {
    let database_bytes = fs::metadata(&workspace.paths.database)
        .map_err(|source| crate::io_error(&workspace.paths.database, source))?
        .len();
    let configuration_bytes = fs::metadata(&workspace.paths.config)
        .map_err(|source| crate::io_error(&workspace.paths.config, source))?
        .len();
    let wal_path = workspace.paths.database.with_extension("sqlite3-wal");
    let wal_bytes = fs::metadata(&wal_path).map_or(0, |metadata| metadata.len());
    database_bytes
        .checked_add(configuration_bytes)
        .and_then(|value| value.checked_add(wal_bytes))
        .and_then(|value| value.checked_add(referenced_blob_bytes))
        .and_then(|value| value.checked_add(BACKUP_METADATA_ALLOWANCE_BYTES))
        .ok_or_else(|| {
            StoreError::WorkspaceMigrationIntegrity("required backup size overflowed".to_owned())
        })
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

fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::Invariant("value exceeds SQLite i64".to_owned()))
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite value".to_owned()))
}

#[derive(Debug)]
struct LegacyInventory {
    rows: Vec<LegacyInventoryRow>,
    counts: BTreeMap<String, u64>,
    digest: Sha256Digest,
}

#[derive(Debug, Serialize)]
struct LegacyInventoryRow {
    table: String,
    key_sha256: Sha256Digest,
    row_sha256: Sha256Digest,
    application_id: Option<ApplicationId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
enum InventoryCell {
    Null,
    Integer(i64),
    Real(String),
    Text(String),
    Blob(String),
}

impl InventoryCell {
    fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            Self::Null | Self::Integer(_) | Self::Real(_) | Self::Blob(_) => None,
        }
    }
}

fn legacy_inventory(
    connection: &Connection,
    associations: &LegacyAssociations,
) -> Result<LegacyInventory, StoreError> {
    let mut rows = Vec::new();
    let mut counts = BTreeMap::new();
    for table in LEGACY_TABLES {
        let table_rows = inventory_table(connection, table, associations)?;
        counts.insert(
            table.to_owned(),
            u64::try_from(table_rows.len()).expect("table row count fits u64"),
        );
        rows.extend(table_rows);
    }
    let digest = digest_bytes(&serde_json::to_vec(&rows)?)?;
    Ok(LegacyInventory {
        rows,
        counts,
        digest,
    })
}

fn inventory_table(
    connection: &Connection,
    table: &str,
    associations: &LegacyAssociations,
) -> Result<Vec<LegacyInventoryRow>, StoreError> {
    let columns = table_columns(connection, table)?;
    if columns.is_empty() {
        return Err(StoreError::WorkspaceMigrationIntegrity(format!(
            "legacy table {table} has no columns"
        )));
    }
    let column_list = columns
        .iter()
        .map(|column| quote_identifier(&column.name))
        .collect::<Vec<_>>()
        .join(", ");
    let mut primary = columns
        .iter()
        .filter(|column| column.primary_key > 0)
        .collect::<Vec<_>>();
    primary.sort_by_key(|column| column.primary_key);
    let order = if primary.is_empty() {
        column_list.clone()
    } else {
        primary
            .iter()
            .map(|column| quote_identifier(&column.name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let source_filter = if table == "audit_events" {
        " WHERE action NOT LIKE 'workspace-v3.%' AND action NOT LIKE 'application-v3.%'"
    } else {
        ""
    };
    let sql = format!(
        "SELECT {column_list} FROM {}{source_filter} ORDER BY {order}",
        quote_identifier(table)
    );
    let mut statement = connection.prepare(&sql)?;
    let mut query = statement.query([])?;
    let mut inventory = Vec::new();
    while let Some(row) = query.next()? {
        let mut values = BTreeMap::new();
        for (index, column) in columns.iter().enumerate() {
            values.insert(column.name.clone(), inventory_cell(row.get_ref(index)?));
        }
        let key_values = if primary.is_empty() {
            values.clone()
        } else {
            primary
                .iter()
                .map(|column| {
                    (
                        column.name.clone(),
                        values
                            .get(&column.name)
                            .expect("primary-key column is present")
                            .clone(),
                    )
                })
                .collect()
        };
        inventory.push(LegacyInventoryRow {
            table: table.to_owned(),
            key_sha256: digest_bytes(&serde_json::to_vec(&key_values)?)?,
            row_sha256: digest_bytes(&serde_json::to_vec(&values)?)?,
            application_id: associations.application_for_row(table, &values),
        });
    }
    Ok(inventory)
}

#[derive(Debug)]
struct TableColumn {
    name: String,
    primary_key: i64,
}

fn table_columns(connection: &Connection, table: &str) -> Result<Vec<TableColumn>, StoreError> {
    let sql = format!("PRAGMA table_info({})", quote_identifier(table));
    let mut statement = connection.prepare(&sql)?;
    statement
        .query_map([], |row| {
            Ok(TableColumn {
                name: row.get(1)?,
                primary_key: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn inventory_cell(value: ValueRef<'_>) -> InventoryCell {
    match value {
        ValueRef::Null => InventoryCell::Null,
        ValueRef::Integer(value) => InventoryCell::Integer(value),
        ValueRef::Real(value) => InventoryCell::Real(format!("{:016x}", value.to_bits())),
        ValueRef::Text(value) => InventoryCell::Text(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => InventoryCell::Blob(hex::encode(value)),
    }
}

#[derive(Debug)]
struct LegacyAssociations {
    job_applications: BTreeMap<String, ApplicationId>,
    source_applications: BTreeMap<String, ApplicationId>,
    run_applications: BTreeMap<String, ApplicationId>,
    stage_applications: BTreeMap<String, ApplicationId>,
    task_applications: BTreeMap<String, ApplicationId>,
    artifact_applications: BTreeMap<String, ApplicationId>,
    known_subjects: BTreeMap<String, ApplicationId>,
}

impl LegacyAssociations {
    fn load(connection: &Connection, workspace_id: &EntityId) -> Result<Self, StoreError> {
        let job_ids = select_strings(connection, "SELECT id FROM jobs ORDER BY id")?;
        let job_applications = job_ids
            .into_iter()
            .map(|job_id| {
                deterministic_application_id(workspace_id, &job_id)
                    .map(|application_id| (job_id, application_id))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let source_applications = relation_map(
            connection,
            "SELECT id, job_id FROM sources ORDER BY id",
            &job_applications,
        )?;
        let run_applications = relation_map(
            connection,
            "SELECT id, job_id FROM workflow_runs WHERE job_id IS NOT NULL ORDER BY id",
            &job_applications,
        )?;
        let stage_applications = relation_map(
            connection,
            "SELECT id, workflow_run_id FROM stage_executions ORDER BY id",
            &run_applications,
        )?;

        let mut task_applications = BTreeMap::new();
        let mut statement =
            connection.prepare("SELECT id, job_id, stage_execution_id FROM tasks ORDER BY id")?;
        for row in statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })? {
            let (id, job_id, stage_id) = row?;
            let application = job_id
                .as_ref()
                .and_then(|job_id| job_applications.get(job_id))
                .or_else(|| {
                    stage_id
                        .as_ref()
                        .and_then(|stage_id| stage_applications.get(stage_id))
                });
            if let Some(application) = application {
                task_applications.insert(id, application.clone());
            }
        }

        let mut artifact_sets: BTreeMap<String, BTreeSet<ApplicationId>> = BTreeMap::new();
        collect_artifact_associations(
            connection,
            &source_applications,
            &run_applications,
            &task_applications,
            &mut artifact_sets,
        )?;
        let artifact_applications = artifact_sets
            .into_iter()
            .filter_map(|(artifact, applications)| {
                (applications.len() == 1).then(|| {
                    (
                        artifact,
                        applications.into_iter().next().expect("one Application"),
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();

        let mut known_subjects = BTreeMap::new();
        for mapping in [
            &job_applications,
            &source_applications,
            &run_applications,
            &stage_applications,
            &task_applications,
            &artifact_applications,
        ] {
            known_subjects.extend(mapping.iter().map(|(id, app)| (id.clone(), app.clone())));
        }
        Ok(Self {
            job_applications,
            source_applications,
            run_applications,
            stage_applications,
            task_applications,
            artifact_applications,
            known_subjects,
        })
    }

    fn application_for_row(
        &self,
        table: &str,
        row: &BTreeMap<String, InventoryCell>,
    ) -> Option<ApplicationId> {
        let text = |column: &str| row.get(column).and_then(InventoryCell::as_text);
        let direct = match table {
            "jobs" => text("id").and_then(|id| self.job_applications.get(id)),
            "sources" => text("id").and_then(|id| self.source_applications.get(id)),
            "source_revisions" => text("source_id").and_then(|id| self.source_applications.get(id)),
            "workflow_runs" => text("id").and_then(|id| self.run_applications.get(id)),
            "stage_executions" => text("id").and_then(|id| self.stage_applications.get(id)),
            "tasks" => text("id").and_then(|id| self.task_applications.get(id)),
            "task_inputs" | "task_results" => {
                text("task_id").and_then(|id| self.task_applications.get(id))
            }
            "application_plan_heads"
            | "application_plan_documents"
            | "document_heads"
            | "review_heads"
            | "package_heads"
            | "export_heads"
            | "render_heads" => {
                text("workflow_run_id").and_then(|id| self.run_applications.get(id))
            }
            "artifacts" => text("id").and_then(|id| self.artifact_applications.get(id)),
            "artifact_revisions" | "artifact_dependencies" | "projection_manifests" => {
                text("artifact_id").and_then(|id| self.artifact_applications.get(id))
            }
            "blob_references" => text("owner_id").and_then(|id| self.artifact_applications.get(id)),
            "evidence_revisions" => {
                text("artifact_id").and_then(|id| self.artifact_applications.get(id))
            }
            "job_leads" => text("promoted_job_id").and_then(|id| self.job_applications.get(id)),
            "audit_events" => text("subject_id").and_then(|id| self.known_subjects.get(id)),
            _ => None,
        };
        direct.cloned()
    }
}

fn deterministic_application_id(
    workspace_id: &EntityId,
    job_id: &str,
) -> Result<ApplicationId, StoreError> {
    let digest = Sha256::digest(
        format!(
            "canisend.workspace-v2-to-v3/application\0{}\0{job_id}",
            workspace_id.as_str()
        )
        .as_bytes(),
    );
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    ApplicationId::try_new(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
    .map_err(StoreError::from)
}

fn select_strings(connection: &Connection, sql: &str) -> Result<Vec<String>, StoreError> {
    let mut statement = connection.prepare(sql)?;
    statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

fn relation_map(
    connection: &Connection,
    sql: &str,
    parent: &BTreeMap<String, ApplicationId>,
) -> Result<BTreeMap<String, ApplicationId>, StoreError> {
    let mut statement = connection.prepare(sql)?;
    let pairs = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(pairs
        .into_iter()
        .filter_map(|(id, parent_id)| parent.get(&parent_id).cloned().map(|app| (id, app)))
        .collect())
}

fn collect_artifact_associations(
    connection: &Connection,
    sources: &BTreeMap<String, ApplicationId>,
    runs: &BTreeMap<String, ApplicationId>,
    tasks: &BTreeMap<String, ApplicationId>,
    output: &mut BTreeMap<String, BTreeSet<ApplicationId>>,
) -> Result<(), StoreError> {
    collect_optional_artifacts(
        connection,
        "SELECT source_id, original_artifact_id, normalized_artifact_id FROM source_revisions",
        sources,
        output,
    )?;
    collect_optional_artifacts(
        connection,
        "SELECT workflow_run_id, output_artifact_id, NULL FROM stage_executions",
        runs,
        output,
    )?;
    collect_optional_artifacts(
        connection,
        "SELECT task_id, artifact_id, NULL FROM task_inputs",
        tasks,
        output,
    )?;
    collect_optional_artifacts(
        connection,
        "SELECT task_id, artifact_id, NULL FROM task_results",
        tasks,
        output,
    )?;
    for (table, columns) in [
        ("application_plan_heads", "artifact_id, NULL"),
        ("application_plan_documents", "plan_artifact_id, NULL"),
        ("document_heads", "plan_artifact_id, artifact_id"),
        ("review_heads", "document_set_artifact_id, artifact_id"),
        ("package_heads", "artifact_id, plan_artifact_id"),
        ("export_heads", "package_artifact_id, artifact_id"),
        ("render_heads", "package_artifact_id, artifact_id"),
    ] {
        collect_optional_artifacts(
            connection,
            &format!("SELECT workflow_run_id, {columns} FROM {table}"),
            runs,
            output,
        )?;
    }
    let mut statement = connection.prepare(
        "SELECT workflow_run_id, evidence_artifact_id, document_set_artifact_id,
                review_artifact_id
         FROM package_heads",
    )?;
    for row in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })? {
        let (run, first, second, third) = row?;
        if let Some(application) = runs.get(&run) {
            for artifact in [first, second, third] {
                output
                    .entry(artifact)
                    .or_default()
                    .insert(application.clone());
            }
        }
    }
    Ok(())
}

fn collect_optional_artifacts(
    connection: &Connection,
    sql: &str,
    owners: &BTreeMap<String, ApplicationId>,
    output: &mut BTreeMap<String, BTreeSet<ApplicationId>>,
) -> Result<(), StoreError> {
    let mut statement = connection.prepare(sql)?;
    for row in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })? {
        let (owner, first, second) = row?;
        if let Some(application) = owners.get(&owner) {
            for artifact in [first, second].into_iter().flatten() {
                output
                    .entry(artifact)
                    .or_default()
                    .insert(application.clone());
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
struct LegacyJob {
    id: EntityId,
    title: String,
    institution: String,
    archived: bool,
    created_at: UtcTimestamp,
    revision: Revision,
}

fn build_application_snapshots(
    connection: &Connection,
    blobs: &BlobStore,
    pack: &ApplicationPackBindingV3,
    applications: &BTreeMap<String, ApplicationId>,
) -> Result<Vec<ApplicationModelSnapshotV3>, StoreError> {
    let jobs = load_legacy_jobs(connection)?;
    let mut snapshots = Vec::with_capacity(jobs.len());
    for job in jobs {
        let application_id = applications.get(job.id.as_str()).ok_or_else(|| {
            StoreError::WorkspaceMigrationIntegrity(format!(
                "Job {} has no deterministic Application mapping",
                job.id
            ))
        })?;
        let source_ids = select_job_source_ids(connection, &job.id)?;
        let requirements = load_requirements(connection, blobs, &job, application_id, pack)?;
        let plan_data = load_plan(connection, blobs, &job.id)?;
        let plan = plan_data
            .as_ref()
            .map(|data| map_plan(data, application_id, pack, &requirements))
            .transpose()?;
        let deliverables = plan_data
            .as_ref()
            .map(|data| map_deliverables(connection, blobs, data, application_id, pack))
            .transpose()?
            .unwrap_or_default();
        let lifecycle = application_lifecycle(connection, &job)?;
        let mut opportunity_metadata = BTreeMap::new();
        opportunity_metadata.insert(
            item_id("institution")?,
            ApplicationFieldValueV3::ShortText(job.institution.clone()),
        );
        let opportunity_id = OpportunityId::try_new(job.id.to_string())?;
        let snapshot = ApplicationModelSnapshotV3 {
            format: ApplicationModelFormatV3::V3,
            pack: pack.clone(),
            opportunity: OpportunityRecordV3 {
                id: opportunity_id.clone(),
                pack: pack.clone(),
                title: job.title,
                metadata: opportunity_metadata,
                source_ids,
                created_at: job.created_at.clone(),
                revision: job.revision,
                archived: job.archived,
            },
            application: ApplicationRecordV3 {
                id: application_id.clone(),
                opportunity_id,
                pack: pack.clone(),
                metadata: BTreeMap::new(),
                lifecycle,
                created_at: job.created_at.clone(),
                updated_at: job.created_at,
                revision: Revision::try_new(1)?,
            },
            requirements,
            plan,
            deliverables,
        };
        validate_migrated_snapshot(&snapshot)?;
        snapshots.push(snapshot);
    }
    Ok(snapshots)
}

fn load_legacy_jobs(connection: &Connection) -> Result<Vec<LegacyJob>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT id, title, institution, archived, created_at, revision
         FROM jobs ORDER BY id",
    )?;
    statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?
        .map(|row| {
            let (id, title, institution, archived, created_at, revision) = row?;
            Ok(LegacyJob {
                id: EntityId::try_new(id)?,
                title,
                institution,
                archived: archived != 0,
                created_at: UtcTimestamp::try_new(created_at)?,
                revision: Revision::try_new(to_u64(revision)?)?,
            })
        })
        .collect()
}

fn select_job_source_ids(
    connection: &Connection,
    job_id: &EntityId,
) -> Result<Vec<EntityId>, StoreError> {
    let mut statement =
        connection.prepare("SELECT id FROM sources WHERE job_id = ?1 ORDER BY created_at, id")?;
    statement
        .query_map([job_id.as_str()], |row| row.get::<_, String>(0))?
        .map(|row| EntityId::try_new(row?).map_err(StoreError::from))
        .collect()
}

fn load_requirements(
    connection: &Connection,
    blobs: &BlobStore,
    job: &LegacyJob,
    application_id: &ApplicationId,
    pack: &ApplicationPackBindingV3,
) -> Result<Vec<RequirementRecordV3>, StoreError> {
    if let Some((artifact, committed_at)) =
        current_stage_artifact(connection, &job.id, "criteria", ArtifactKind::Criteria)?
    {
        let criteria: CriteriaSetRecord = load_contract(blobs, &artifact)?;
        if criteria.job_id != job.id {
            return Err(StoreError::WorkspaceMigrationIntegrity(
                "confirmed Criteria belong to a different Job".to_owned(),
            ));
        }
        return criteria
            .criteria
            .iter()
            .map(|criterion| {
                map_requirement(
                    criterion,
                    application_id,
                    pack,
                    criterion.confirmed.then_some(&committed_at),
                )
            })
            .collect();
    }
    let Some((artifact, _)) =
        current_stage_artifact(connection, &job.id, "parse", ArtifactKind::ParsedJob)?
    else {
        return Ok(Vec::new());
    };
    let parsed: ParsedJobRecord = load_contract(blobs, &artifact)?;
    if parsed.job_id != job.id {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "parsed Job belongs to a different Job".to_owned(),
        ));
    }
    parsed
        .criteria
        .iter()
        .map(|criterion| map_requirement(criterion, application_id, pack, None))
        .collect()
}

fn map_requirement(
    criterion: &canisend_contracts::CriterionRecord,
    application_id: &ApplicationId,
    pack: &ApplicationPackBindingV3,
    confirmed_at: Option<&UtcTimestamp>,
) -> Result<RequirementRecordV3, StoreError> {
    let confirmed = criterion.confirmed && confirmed_at.is_some();
    Ok(RequirementRecordV3 {
        id: RequirementId::try_new(criterion.id.to_string())?,
        application_id: application_id.clone(),
        pack: pack.clone(),
        category: item_id(&enum_name(criterion.kind)?)?,
        statement: criterion.requirement.clone(),
        priority: match criterion.importance {
            CriterionImportance::Essential => RequirementPriorityV3::Mandatory,
            CriterionImportance::Desirable => RequirementPriorityV3::Recommended,
            CriterionImportance::Informational => RequirementPriorityV3::Informational,
        },
        source_span: ContentSpanV3 {
            content: ContentRevisionReferenceV3 {
                id: criterion.source_span.source.id.clone(),
                revision: criterion.source_span.source.revision,
                sha256: criterion.source_span.source.sha256.clone(),
            },
            start_byte: criterion.source_span.start_byte,
            end_byte: criterion.source_span.end_byte,
        },
        confirmation: if confirmed {
            RequirementConfirmationV3::Confirmed
        } else {
            RequirementConfirmationV3::Proposed
        },
        confirmed_by: confirmed.then_some(ActorKind::User),
        confirmed_at: confirmed_at.cloned().filter(|_| confirmed),
        revision: criterion.revision,
    })
}

struct LegacyPlan {
    run_id: EntityId,
    artifact: ArtifactReference,
    record: ApplicationPlanRecord,
    requirement_inputs: Vec<canisend_contracts::CriterionRevisionReference>,
    stale: bool,
    committed_at: UtcTimestamp,
}

fn load_plan(
    connection: &Connection,
    blobs: &BlobStore,
    job_id: &EntityId,
) -> Result<Option<LegacyPlan>, StoreError> {
    type Row = (String, String, i64, String, String, i64, String);
    let row: Option<Row> = connection
        .query_row(
            "SELECT run.id, head.artifact_id, head.artifact_revision,
                    artifacts.kind, revisions.sha256, artifacts.stale, revisions.created_at
             FROM workflow_runs AS run
             JOIN application_plan_heads AS head ON head.workflow_run_id = run.id
             JOIN artifacts ON artifacts.id = head.artifact_id
             JOIN artifact_revisions AS revisions
               ON revisions.artifact_id = head.artifact_id
              AND revisions.revision = head.artifact_revision
             WHERE run.job_id = ?1
             ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
            [job_id.as_str()],
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
    let Some((run_id, artifact_id, revision, kind, sha256, stale, committed_at)) = row else {
        return Ok(None);
    };
    let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
    if kind != ArtifactKind::ApplicationPlan {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "application-plan head has the wrong Artifact kind".to_owned(),
        ));
    }
    let artifact = ArtifactReference {
        kind,
        id: EntityId::try_new(artifact_id)?,
        revision: Revision::try_new(to_u64(revision)?)?,
        sha256: Sha256Digest::try_new(sha256)?,
    };
    let record: ApplicationPlanRecord = load_contract(blobs, &artifact)?;
    if record.job_id != *job_id {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "Application Plan belongs to a different Job".to_owned(),
        ));
    }
    let matches: EvidenceMatchSetRecord = load_contract(blobs, &record.matches_artifact)?;
    if matches.job_id != *job_id {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "Application Plan matches belong to a different Job".to_owned(),
        ));
    }
    let criteria: CriteriaSetRecord = load_contract(blobs, &matches.criteria_artifact)?;
    if criteria.job_id != *job_id {
        return Err(StoreError::WorkspaceMigrationIntegrity(
            "Application Plan criteria belong to a different Job".to_owned(),
        ));
    }
    Ok(Some(LegacyPlan {
        run_id: EntityId::try_new(run_id)?,
        artifact,
        record,
        requirement_inputs: criteria
            .criteria
            .into_iter()
            .map(|criterion| canisend_contracts::CriterionRevisionReference {
                id: criterion.id,
                revision: criterion.revision,
            })
            .collect(),
        stale: stale != 0,
        committed_at: UtcTimestamp::try_new(committed_at)?,
    }))
}

fn map_plan(
    legacy: &LegacyPlan,
    application_id: &ApplicationId,
    pack: &ApplicationPackBindingV3,
    requirements: &[RequirementRecordV3],
) -> Result<PlanRecordV3, StoreError> {
    let current_requirements = requirements
        .iter()
        .map(|requirement| (requirement.id.as_str(), requirement))
        .collect::<BTreeMap<_, _>>();
    let requirement_inputs = legacy
        .requirement_inputs
        .iter()
        .map(|reference| {
            let requirement = current_requirements
                .get(reference.id.as_str())
                .ok_or_else(|| {
                    StoreError::WorkspaceMigrationIntegrity(format!(
                        "Plan Requirement {} is absent from the migrated Application",
                        reference.id
                    ))
                })?;
            if (!legacy.stale && reference.revision != requirement.revision)
                || (legacy.stale && reference.revision > requirement.revision)
            {
                return Err(StoreError::WorkspaceMigrationIntegrity(format!(
                    "Plan Requirement {} has an invalid historical revision",
                    reference.id
                )));
            }
            Ok(RequirementRevisionReferenceV3 {
                id: requirement.id.clone(),
                revision: reference.revision,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let deliverables = legacy
        .record
        .documents
        .iter()
        .map(|document| {
            let local_kind = document_kind_id(document.kind)?;
            Ok(PlannedDeliverableV3 {
                kind: DeliverableKindId::from_parts(&pack.id, &local_kind),
                disposition: match document.requirement {
                    DocumentRequirement::Required => PlannedDeliverableDispositionV3::Required,
                    DocumentRequirement::Optional => PlannedDeliverableDispositionV3::Optional,
                    DocumentRequirement::Omitted => PlannedDeliverableDispositionV3::Omitted,
                },
                rationale: document.rationale.clone(),
                constraints: document.constraints.clone(),
                execution_mode: document.executor,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let blockers = legacy
        .record
        .blockers
        .iter()
        .map(|blocker| {
            Ok(PlanBlockerV3 {
                code: blocker.code.clone(),
                requirement: Some(RequirementRevisionReferenceV3 {
                    id: RequirementId::try_new(blocker.criterion.id.to_string())?,
                    revision: blocker.criterion.revision,
                }),
                severity: match blocker.severity {
                    canisend_contracts::PlanBlockerSeverity::Blocking => {
                        PlanBlockerSeverityV3::Blocking
                    }
                    canisend_contracts::PlanBlockerSeverity::Warning => {
                        PlanBlockerSeverityV3::Warning
                    }
                },
                description: blocker.description.clone(),
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(PlanRecordV3 {
        id: PlanId::try_new(legacy.record.id.to_string())?,
        application_id: application_id.clone(),
        pack: pack.clone(),
        state: if legacy.stale {
            PlanStateV3::Stale
        } else {
            PlanStateV3::Confirmed
        },
        decision: Some(item_id(&enum_name(legacy.record.decision)?)?),
        requirement_inputs,
        deliverables,
        blockers,
        decided_by: Some(ActorKind::User),
        decided_at: Some(legacy.committed_at.clone()),
        revision: legacy.record.revision,
    })
}

fn map_deliverables(
    connection: &Connection,
    blobs: &BlobStore,
    legacy_plan: &LegacyPlan,
    application_id: &ApplicationId,
    pack: &ApplicationPackBindingV3,
) -> Result<Vec<DeliverableRecordV3>, StoreError> {
    type Row = (String, i64, String, String, i64, String, i64);
    let mut statement = connection.prepare(
        "SELECT head.planned_document_id, head.planned_document_revision, head.kind,
                head.artifact_id, head.artifact_revision, revisions.sha256, artifacts.stale
         FROM document_heads AS head
         JOIN artifacts ON artifacts.id = head.artifact_id
         JOIN artifact_revisions AS revisions
           ON revisions.artifact_id = head.artifact_id
          AND revisions.revision = head.artifact_revision
         WHERE head.workflow_run_id = ?1
         ORDER BY head.kind, head.planned_document_id",
    )?;
    let rows = statement
        .query_map([legacy_plan.run_id.as_str()], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?
        .collect::<Result<Vec<Row>, _>>()?;
    let review_exists = row_exists(
        connection,
        "SELECT 1 FROM review_heads WHERE workflow_run_id = ?1",
        legacy_plan.run_id.as_str(),
    )?;
    let package_ready: Option<String> = connection
        .query_row(
            "SELECT readiness_state FROM package_heads WHERE workflow_run_id = ?1",
            [legacy_plan.run_id.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    rows.into_iter()
        .map(
            |(
                planned_id,
                planned_revision,
                kind,
                artifact_id,
                artifact_revision,
                sha256,
                stale,
            )| {
                let artifact_kind: ArtifactKind =
                    serde_json::from_value(Value::String(kind.clone()))?;
                let artifact = ArtifactReference {
                    kind: artifact_kind,
                    id: EntityId::try_new(artifact_id)?,
                    revision: Revision::try_new(to_u64(artifact_revision)?)?,
                    sha256: Sha256Digest::try_new(sha256)?,
                };
                let document: DocumentRecord = load_contract(blobs, &artifact)?;
                if document.job_id != legacy_plan.record.job_id
                    || document.plan_artifact != legacy_plan.artifact
                    || document.planned_document.id.as_str() != planned_id
                    || document.planned_document.revision.get() != to_u64(planned_revision)?
                {
                    return Err(StoreError::WorkspaceMigrationIntegrity(
                        "Document head does not match its Job, Plan, or planned Deliverable"
                            .to_owned(),
                    ));
                }
                let local_kind = document_kind_id(document.kind)?;
                let expected_artifact_kind = document_artifact_kind(document.kind);
                if artifact.kind != expected_artifact_kind || enum_name(document.kind)? != kind {
                    return Err(StoreError::WorkspaceMigrationIntegrity(
                        "Document head has inconsistent legacy kinds".to_owned(),
                    ));
                }
                let evidence_inputs = document_evidence_inputs(&document);
                let state = if stale != 0 {
                    DeliverableStateV3::Stale
                } else if package_ready
                    .as_deref()
                    .is_some_and(|state| matches!(state, "ready-to-export" | "exported"))
                {
                    DeliverableStateV3::Approved
                } else if review_exists {
                    DeliverableStateV3::ReviewRequired
                } else {
                    DeliverableStateV3::Draft
                };
                Ok(DeliverableRecordV3 {
                    id: DeliverableId::try_new(planned_id)?,
                    application_id: application_id.clone(),
                    pack: pack.clone(),
                    plan: PlanRevisionReferenceV3 {
                        id: PlanId::try_new(legacy_plan.record.id.to_string())?,
                        revision: legacy_plan.record.revision,
                    },
                    kind: DeliverableKindId::from_parts(&pack.id, &local_kind),
                    title: document.title,
                    state,
                    content: Some(ContentRevisionReferenceV3 {
                        id: artifact.id,
                        revision: artifact.revision,
                        sha256: artifact.sha256,
                    }),
                    media_type: Some("application/json".to_owned()),
                    evidence_inputs,
                    revision: document.revision,
                })
            },
        )
        .collect()
}

fn document_evidence_inputs(document: &DocumentRecord) -> Vec<EntityRevisionReferenceV3> {
    let mut evidence = BTreeMap::new();
    for citation in document
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .flat_map(|claim| &claim.citations)
    {
        if let CitationTarget::Evidence {
            evidence: reference,
        } = &citation.target
        {
            evidence.insert(reference.id.clone(), reference.revision);
        }
    }
    evidence
        .into_iter()
        .map(|(id, revision)| EntityRevisionReferenceV3 { id, revision })
        .collect()
}

fn application_lifecycle(
    connection: &Connection,
    job: &LegacyJob,
) -> Result<ApplicationLifecycleV3, StoreError> {
    if job.archived {
        return Ok(ApplicationLifecycleV3::Archived);
    }
    if connection
        .query_row(
            "SELECT 1
             FROM workflow_runs AS run
             JOIN export_heads AS export ON export.workflow_run_id = run.id
             WHERE run.job_id = ?1 LIMIT 1",
            [job.id.as_str()],
            |_| Ok(()),
        )
        .optional()?
        .is_some()
    {
        return Ok(ApplicationLifecycleV3::Completed);
    }
    if row_exists(
        connection,
        "SELECT 1 FROM workflow_runs WHERE job_id = ?1 LIMIT 1",
        job.id.as_str(),
    )? {
        Ok(ApplicationLifecycleV3::Active)
    } else {
        Ok(ApplicationLifecycleV3::Draft)
    }
}

fn current_stage_artifact(
    connection: &Connection,
    job_id: &EntityId,
    stage: &str,
    expected_kind: ArtifactKind,
) -> Result<Option<(ArtifactReference, UtcTimestamp)>, StoreError> {
    type Row = (String, i64, String, String, String);
    let row: Option<Row> = connection
        .query_row(
            "SELECT execution.output_artifact_id, execution.output_artifact_revision,
                    artifacts.kind, revisions.sha256, revisions.created_at
             FROM workflow_runs AS run
             JOIN stage_executions AS execution ON execution.workflow_run_id = run.id
             JOIN artifacts ON artifacts.id = execution.output_artifact_id
             JOIN artifact_revisions AS revisions
               ON revisions.artifact_id = execution.output_artifact_id
              AND revisions.revision = execution.output_artifact_revision
             WHERE run.job_id = ?1 AND execution.stage = ?2
               AND execution.status = 'complete'
             ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
            params![job_id.as_str(), stage],
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
    let Some((id, revision, kind, sha256, committed_at)) = row else {
        return Ok(None);
    };
    let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
    if kind != expected_kind {
        return Err(StoreError::WorkspaceMigrationIntegrity(format!(
            "{stage} stage has unexpected Artifact kind"
        )));
    }
    Ok(Some((
        ArtifactReference {
            kind,
            id: EntityId::try_new(id)?,
            revision: Revision::try_new(to_u64(revision)?)?,
            sha256: Sha256Digest::try_new(sha256)?,
        },
        UtcTimestamp::try_new(committed_at)?,
    )))
}

fn load_contract<T>(blobs: &BlobStore, artifact: &ArtifactReference) -> Result<T, StoreError>
where
    T: DeserializeOwned + schemars::JsonSchema + SemanticValidate,
{
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(|error| match error {
        canisend_contracts::CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        canisend_contracts::CandidateValidationError::Semantic(violations) => {
            StoreError::CandidateSemantic(violations)
        }
    })
}

fn validate_migrated_snapshot(snapshot: &ApplicationModelSnapshotV3) -> Result<(), StoreError> {
    let value = serde_json::to_value(snapshot)?;
    validate_external_candidate::<ApplicationModelSnapshotV3>(&value)
        .map(|_| ())
        .map_err(|error| match error {
            canisend_contracts::CandidateValidationError::Structural(violations) => {
                StoreError::CandidateStructural(violations)
            }
            canisend_contracts::CandidateValidationError::Semantic(violations) => {
                StoreError::CandidateSemantic(violations)
            }
        })
}

fn item_id(value: &str) -> Result<WorkflowPackItemId, StoreError> {
    WorkflowPackItemId::try_new(value).map_err(|error| StoreError::InvalidInput(error.to_string()))
}

fn document_kind_id(kind: DocumentKind) -> Result<WorkflowPackItemId, StoreError> {
    item_id(&enum_name(kind)?)
}

fn document_artifact_kind(kind: DocumentKind) -> ArtifactKind {
    match kind {
        DocumentKind::CoverLetter => ArtifactKind::CoverLetter,
        DocumentKind::ResearchStatement => ArtifactKind::ResearchStatement,
        DocumentKind::TeachingStatement => ArtifactKind::TeachingStatement,
        DocumentKind::Cv => ArtifactKind::Cv,
    }
}

fn row_exists(connection: &Connection, sql: &str, parameter: &str) -> Result<bool, StoreError> {
    connection
        .query_row(sql, [parameter], |_| Ok(()))
        .optional()
        .map(|row| row.is_some())
        .map_err(StoreError::from)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ExecutionMode, ExpectedInputRevision, PrivacyClassification, SourceKind,
        TaskCompletionRequest,
    };
    use canisend_core::{
        WorkflowPackByteLoader, WorkflowPackCapabilityRegistry, WorkflowPackOrigin,
        WorkflowPackRuntime,
    };
    use canisend_resources::academic_job_workflow_pack;

    use super::*;
    use crate::{
        ApplicationModelRepository, CriteriaService, JobService, NewSource, TaskService,
        WorkflowService, verify_backup,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            Self(std::env::temp_dir().join(format!(
                "canisend-v3-migration-{label}-{}-{}",
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

    #[derive(Default)]
    struct RecordingObserver {
        boundaries: Vec<MigrationWriteBoundary>,
    }

    impl MigrationWriteObserver for RecordingObserver {
        fn after_write(&mut self, boundary: MigrationWriteBoundary) -> Result<(), StoreError> {
            self.boundaries.push(boundary);
            Ok(())
        }
    }

    struct InterruptAt {
        target: MigrationWriteBoundary,
    }

    impl MigrationWriteObserver for InterruptAt {
        fn after_write(&mut self, boundary: MigrationWriteBoundary) -> Result<(), StoreError> {
            if boundary == self.target {
                return Err(StoreError::WorkspaceMigrationConflict(format!(
                    "injected interruption at {boundary:?}"
                )));
            }
            Ok(())
        }
    }

    fn simple_migration_fixture(
        label: &str,
    ) -> (
        TestDirectory,
        Workspace,
        VerifiedWorkflowPackBundle,
        WorkspaceV3MigrationPreview,
    ) {
        let root = TestDirectory::new(label);
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Role", "Institution", ActorKind::User)
            .expect("Job");
        let pack = academic_pack();
        let preview = WorkspaceV3MigrationService::new(&mut workspace)
            .preview(&pack)
            .expect("migration preview");
        (root, workspace, pack, preview)
    }

    fn assert_no_v3_migration_writes(workspace: &mut Workspace) {
        for table in [
            "workspace_v3_authority",
            "application_model_v3_heads",
            "application_model_v3_revisions",
            "application_model_v3_dependencies",
            "workspace_v3_migrations",
            "workspace_v3_application_links",
            "workspace_v3_legacy_bindings",
        ] {
            let count: i64 = workspace
                .database
                .connection()
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("v3 table count");
            assert_eq!(count, 0, "{table} retained a partial migration write");
        }
        let migration_audits: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM audit_events
                 WHERE action IN ('workspace-v3.activate', 'application-v3.create',
                                  'workspace-v3.migrate')",
                [],
                |row| row.get(0),
            )
            .expect("migration audit count");
        assert_eq!(migration_audits, 0);
    }

    #[test]
    fn dry_run_is_body_free_and_stale_preview_fails_before_backup() {
        let root = TestDirectory::new("stale-root");
        let backup = TestDirectory::new("stale-backup");
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        JobService::new(&mut workspace.database, &workspace.blobs)
            .create(
                "PRIVATE-MIGRATION-SENTINEL",
                "Private institution",
                ActorKind::User,
            )
            .expect("Job");
        let pack = academic_pack();
        let preview = WorkspaceV3MigrationService::new(&mut workspace)
            .preview(&pack)
            .expect("migration preview");
        let repeated = WorkspaceV3MigrationService::new(&mut workspace)
            .preview(&pack)
            .expect("repeat migration preview");
        assert_eq!(repeated, preview);
        let encoded = serde_json::to_string(&preview).expect("preview JSON");
        assert!(!encoded.contains("PRIVATE-MIGRATION-SENTINEL"));
        assert_eq!(preview.application_count, 1);
        assert_eq!(preview.source_workspace_format, WORKSPACE_FORMAT);
        assert_eq!(preview.target_workspace_format, WORKSPACE_V3_FORMAT);

        JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Second role", "Second institution", ActorKind::User)
            .expect("second Job");
        let error = WorkspaceV3MigrationService::new(&mut workspace)
            .migrate(&pack, &preview.migration_plan_sha256, backup.path())
            .expect_err("stale preview must fail");
        assert!(matches!(error, StoreError::WorkspaceMigrationConflict(_)));
        assert!(!backup.path().exists());
        assert!(matches!(
            ApplicationModelRepository::new(&mut workspace.database).authority(),
            Err(StoreError::ApplicationModelUnavailable)
        ));
    }

    #[test]
    fn invalid_referenced_blob_refuses_migration_without_authority() {
        let root = TestDirectory::new("invalid-blob-root");
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Role", "Institution", ActorKind::User)
            .expect("Job");
        let source = JobService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                &job.id,
                NewSource {
                    kind: SourceKind::LocalFile,
                    original_bytes: b"Original".to_vec(),
                    normalized_text: "Original\n".to_owned(),
                    source_url: None,
                    final_url: None,
                    content_type: "text/plain; charset=utf-8".to_owned(),
                    redirect_chain: Vec::new(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("source");
        fs::remove_file(workspace.blobs.path_for(&source.original.sha256))
            .expect("remove bounded fixture Blob");
        let error = WorkspaceV3MigrationService::new(&mut workspace)
            .preview(&academic_pack())
            .expect_err("invalid Blob must reject migration");
        assert!(matches!(error, StoreError::WorkspaceMigrationIntegrity(_)));
        assert!(matches!(
            ApplicationModelRepository::new(&mut workspace.database).authority(),
            Err(StoreError::ApplicationModelUnavailable)
        ));
    }

    #[test]
    fn external_same_id_pack_cannot_activate_v3_authority() {
        let root = TestDirectory::new("external-pack-root");
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Role", "Institution", ActorKind::User)
            .expect("Job");
        let error = WorkspaceV3MigrationService::new(&mut workspace)
            .preview(&academic_pack_with_origin(WorkflowPackOrigin::External))
            .expect_err("external Pack cannot acquire built-in compatibility authority");
        assert!(matches!(error, StoreError::WorkspaceMigrationConflict(_)));
        assert_no_v3_migration_writes(&mut workspace);
    }

    #[test]
    fn every_logical_write_boundary_rolls_back_and_retries_cleanly() {
        let (_recording_root, mut recording_workspace, recording_pack, recording_preview) =
            simple_migration_fixture("boundary-recording");
        let recording_backup = TestDirectory::new("boundary-recording-backup");
        let mut recorder = RecordingObserver::default();
        WorkspaceV3MigrationService::new(&mut recording_workspace)
            .migrate_observed(
                &recording_pack,
                &recording_preview.migration_plan_sha256,
                recording_backup.path(),
                &mut recorder,
            )
            .expect("record migration boundaries");
        let expected_boundary_count = usize::try_from(recording_preview.legacy_inventory_count)
            .expect("legacy inventory count fits usize")
            + (2 * usize::try_from(recording_preview.application_count)
                .expect("Application count fits usize"))
            + 4;
        assert_eq!(recorder.boundaries.len(), expected_boundary_count);
        assert_eq!(
            recorder.boundaries.first(),
            Some(&MigrationWriteBoundary::AuthorityActivated)
        );
        assert_eq!(
            recorder.boundaries.last(),
            Some(&MigrationWriteBoundary::PreCommitVerified)
        );

        for target in recorder.boundaries {
            let (_root, mut workspace, pack, preview) =
                simple_migration_fixture("boundary-interruption");
            let backup = TestDirectory::new("boundary-interruption-backup");
            let retry_backup = TestDirectory::new("boundary-retry-backup");
            let references_before = workspace.database.referenced_digests().expect("Blob refs");
            let mut observer = InterruptAt {
                target: target.clone(),
            };
            let error = WorkspaceV3MigrationService::new(&mut workspace)
                .migrate_observed(
                    &pack,
                    &preview.migration_plan_sha256,
                    backup.path(),
                    &mut observer,
                )
                .expect_err("injected boundary must interrupt migration");
            assert!(matches!(error, StoreError::WorkspaceMigrationConflict(_)));
            verify_backup(backup.path()).expect("interrupted migration backup verifies");
            assert_no_v3_migration_writes(&mut workspace);
            assert_eq!(
                workspace.database.referenced_digests().expect("Blob refs"),
                references_before
            );
            let after = WorkspaceV3MigrationService::new(&mut workspace)
                .preview(&pack)
                .expect("v2 preview after rollback");
            assert_eq!(after.migration_plan_sha256, preview.migration_plan_sha256);

            WorkspaceV3MigrationService::new(&mut workspace)
                .migrate(&pack, &after.migration_plan_sha256, retry_backup.path())
                .expect("retry migration");
            assert_eq!(
                workspace.status().expect("retry status").workspace_format,
                WORKSPACE_V3_FORMAT,
                "retry after {target:?} did not produce valid v3 authority"
            );
        }
    }

    #[test]
    fn database_busy_leaves_verified_backup_and_retryable_v2_authority() {
        let (_root, mut workspace, pack, preview) = simple_migration_fixture("database-busy");
        let backup = TestDirectory::new("database-busy-backup");
        let retry_backup = TestDirectory::new("database-busy-retry-backup");
        workspace
            .database
            .connection()
            .busy_timeout(std::time::Duration::from_millis(5))
            .expect("short busy timeout");
        let mut blocker = rusqlite::Connection::open(&workspace.paths.database)
            .expect("second database connection");
        let blocker_transaction = blocker
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .expect("hold external writer lock");

        let error = WorkspaceV3MigrationService::new(&mut workspace)
            .migrate(&pack, &preview.migration_plan_sha256, backup.path())
            .expect_err("writer lock must reject migration");
        assert!(matches!(
            &error,
            StoreError::Sqlite(rusqlite::Error::SqliteFailure(code, _))
                if matches!(
                    code.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                )
        ));
        verify_backup(backup.path()).expect("busy-path backup verifies");
        assert_no_v3_migration_writes(&mut workspace);

        blocker_transaction.rollback().expect("release writer lock");
        workspace
            .database
            .connection()
            .busy_timeout(std::time::Duration::from_secs(2))
            .expect("restore busy timeout");
        WorkspaceV3MigrationService::new(&mut workspace)
            .migrate(&pack, &preview.migration_plan_sha256, retry_backup.path())
            .expect("retry after writer releases lock");
        assert_eq!(
            workspace.status().expect("status").workspace_format,
            WORKSPACE_V3_FORMAT
        );
    }

    #[test]
    fn sqlite_full_rolls_back_without_mixed_authority() {
        let (_root, mut workspace, pack, preview) = simple_migration_fixture("sqlite-full");
        let backup = TestDirectory::new("sqlite-full-backup");
        let retry_backup = TestDirectory::new("sqlite-full-retry-backup");
        let page_count = {
            let connection = workspace.database.connection();
            connection
                .execute(
                    "CREATE TABLE local_migration_space_fixture(
                        id INTEGER PRIMARY KEY, payload BLOB NOT NULL
                     ) STRICT",
                    [],
                )
                .expect("create bounded local space fixture");
            let page_count: u32 = connection
                .pragma_query_value(None, "page_count", |row| row.get(0))
                .expect("page count");
            connection
                .pragma_update(None, "max_page_count", page_count)
                .expect("bound database pages");
            let mut observed_full = false;
            for _ in 0..10_000 {
                match connection.execute(
                    "INSERT INTO local_migration_space_fixture(payload) VALUES (zeroblob(2048))",
                    [],
                ) {
                    Ok(_) => {}
                    Err(rusqlite::Error::SqliteFailure(code, _))
                        if code.code == rusqlite::ErrorCode::DiskFull =>
                    {
                        observed_full = true;
                        break;
                    }
                    Err(error) => panic!("unexpected local space fixture error: {error}"),
                }
            }
            assert!(observed_full, "bounded database did not reach SQLITE_FULL");
            page_count
        };

        let error = WorkspaceV3MigrationService::new(&mut workspace)
            .migrate(&pack, &preview.migration_plan_sha256, backup.path())
            .expect_err("full database must reject migration");
        assert!(matches!(
            &error,
            StoreError::Sqlite(rusqlite::Error::SqliteFailure(code, _))
                if code.code == rusqlite::ErrorCode::DiskFull
        ));
        verify_backup(backup.path()).expect("full-path backup verifies");
        assert_no_v3_migration_writes(&mut workspace);

        workspace
            .database
            .connection()
            .pragma_update(None, "max_page_count", page_count + 1_024)
            .expect("restore database capacity");
        workspace
            .database
            .connection()
            .execute("DROP TABLE local_migration_space_fixture", [])
            .expect("drop local space fixture");
        WorkspaceV3MigrationService::new(&mut workspace)
            .migrate(&pack, &preview.migration_plan_sha256, retry_backup.path())
            .expect("retry after capacity recovery");
        assert_eq!(
            workspace.status().expect("status").workspace_format,
            WORKSPACE_V3_FORMAT
        );
    }

    #[test]
    fn edited_legacy_projection_is_counted_and_preserved_byte_for_byte() {
        let root = TestDirectory::new("edited-projection");
        let backup = TestDirectory::new("edited-projection-backup");
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Role", "Institution", ActorKind::User)
            .expect("Job");
        let source = JobService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                &job.id,
                NewSource {
                    kind: SourceKind::LocalFile,
                    original_bytes: b"Original source".to_vec(),
                    normalized_text: "Original source\n".to_owned(),
                    source_url: None,
                    final_url: None,
                    content_type: "text/plain; charset=utf-8".to_owned(),
                    redirect_chain: Vec::new(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("source");
        let relative_path = format!("jobs/{}/migration-user-edit.txt", job.id);
        let projection_path = root.path().join(&relative_path);
        fs::create_dir_all(projection_path.parent().expect("projection parent"))
            .expect("projection directory");
        let user_bytes = b"USER-EDITED-PROJECTION\n";
        fs::write(&projection_path, user_bytes).expect("edited projection");
        workspace
            .database
            .connection()
            .execute(
                "INSERT INTO projection_manifests(
                    artifact_id, revision, relative_path, sha256, projection_kind,
                    generated_sha256, observed_sha256, status, last_error, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, 'raw', ?4, ?5, 'edited', NULL, ?6)",
                params![
                    source.original.id.as_str(),
                    to_i64(source.original.revision.get()).expect("revision"),
                    relative_path,
                    source.original.sha256.as_str(),
                    digest_bytes(user_bytes).expect("edited digest").as_str(),
                    now_utc().expect("timestamp").as_str(),
                ],
            )
            .expect("edited projection manifest");
        let projection_before =
            projection_state_digest(workspace.database.connection()).expect("projection digest");
        let pack = academic_pack();
        let preview = WorkspaceV3MigrationService::new(&mut workspace)
            .preview(&pack)
            .expect("preview");
        assert_eq!(preview.projection_conflict_count, 1);

        WorkspaceV3MigrationService::new(&mut workspace)
            .migrate(&pack, &preview.migration_plan_sha256, backup.path())
            .expect("migration preserves projection");
        assert_eq!(
            fs::read(&projection_path).expect("projection after migration"),
            user_bytes
        );
        assert_eq!(
            projection_state_digest(workspace.database.connection()).expect("projection digest"),
            projection_before
        );
    }

    #[test]
    fn older_schema_gate_refuses_v3_without_mutation_and_backup_restores_v2() {
        let (_root, mut workspace, pack, preview) = simple_migration_fixture("old-binary");
        let backup = TestDirectory::new("old-binary-backup");
        let restored = TestDirectory::new("old-binary-restored");
        WorkspaceV3MigrationService::new(&mut workspace)
            .migrate(&pack, &preview.migration_plan_sha256, backup.path())
            .expect("migration");
        let application_count_before: i64 = workspace
            .database
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM application_model_v3_heads",
                [],
                |row| row.get(0),
            )
            .expect("Application count");
        let error = crate::database::ensure_supported_schema(
            workspace.database.connection(),
            DATABASE_SCHEMA_VERSION - 1,
        )
        .expect_err("older schema gate must refuse migrated Workspace");
        assert!(matches!(
            error,
            StoreError::WorkspaceVersionUnsupported {
                found: DATABASE_SCHEMA_VERSION,
                supported
            } if supported == DATABASE_SCHEMA_VERSION - 1
        ));
        assert_eq!(
            workspace
                .status()
                .expect("v3 remains valid")
                .workspace_format,
            WORKSPACE_V3_FORMAT
        );
        assert_eq!(
            workspace
                .database
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM application_model_v3_heads",
                    [],
                    |row| { row.get::<_, i64>(0) }
                )
                .expect("Application count"),
            application_count_before
        );

        drop(workspace);
        let mut restored_workspace = Workspace::restore(backup.path(), restored.path())
            .expect("restore migration backup with a compatible binary");
        assert_eq!(
            restored_workspace
                .status()
                .expect("restored status")
                .workspace_format,
            WORKSPACE_FORMAT
        );
        assert_no_v3_migration_writes(&mut restored_workspace);
    }

    #[test]
    fn verified_backup_migration_preserves_inventory_blobs_and_v2_restore() {
        let root = TestDirectory::new("golden-root");
        let backup = TestDirectory::new("golden-backup");
        let restored = TestDirectory::new("golden-restored");
        let mut workspace = Workspace::init(root.path()).expect("Workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Lecturer in Economics", "University X", ActorKind::User)
            .expect("Job");
        JobService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                &job.id,
                NewSource {
                    kind: SourceKind::LocalFile,
                    original_bytes: b"Teach economics".to_vec(),
                    normalized_text: "Teach economics\n".to_owned(),
                    source_url: None,
                    final_url: None,
                    content_type: "text/plain; charset=utf-8".to_owned(),
                    redirect_chain: Vec::new(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("source");
        WorkflowService::new(&mut workspace.database)
            .start(&job.id)
            .expect("workflow");
        let descriptor = TaskService::new(&mut workspace.database, &workspace.blobs)
            .prepare_job_parse(&job.id, ExecutionMode::HostAgent)
            .expect("parse task");
        let candidate = serde_json::json!({
            "id": "019f2f55-7c00-7000-8000-000000000301",
            "job_id": job.id,
            "title": "Lecturer in Economics",
            "institution": "University X",
            "summary": "Teach economics",
            "responsibilities": ["Teach economics"],
            "criteria": [{
                "id": "019f2f55-7c00-7000-8000-000000000302",
                "job_id": job.id,
                "kind": "teaching",
                "requirement": "Evidence of university-level teaching",
                "importance": "essential",
                "source_quote": "Teach economics",
                "source_span": {
                    "source": descriptor.input_artifacts[0],
                    "start_byte": 0,
                    "end_byte": 15
                },
                "confidence_milli": 950,
                "confirmed": false,
                "revision": 1
            }],
            "revision": 1
        });
        TaskService::new(&mut workspace.database, &workspace.blobs)
            .complete(&TaskCompletionRequest {
                task_id: descriptor.id.clone(),
                lease_id: descriptor.lease.id,
                expected_job_revision: descriptor.job_revision,
                expected_inputs: descriptor
                    .input_artifacts
                    .iter()
                    .map(|input| ExpectedInputRevision {
                        artifact_id: input.id.clone(),
                        revision: input.revision,
                        sha256: input.sha256.clone(),
                    })
                    .collect(),
                candidate,
            })
            .expect("parse completion");
        let criteria = CriteriaService::new(&mut workspace.database, &workspace.blobs)
            .template(&job.id)
            .expect("criteria template");
        CriteriaService::new(&mut workspace.database, &workspace.blobs)
            .confirm(
                &job.id,
                &serde_json::to_value(criteria).expect("criteria JSON"),
            )
            .expect("criteria confirmation");
        let referenced_before = workspace.database.referenced_digests().expect("Blob refs");
        let projection_before =
            projection_state_digest(workspace.database.connection()).expect("projection digest");
        let pack = academic_pack();
        let preview = WorkspaceV3MigrationService::new(&mut workspace)
            .preview(&pack)
            .expect("preview");
        let result = WorkspaceV3MigrationService::new(&mut workspace)
            .migrate(&pack, &preview.migration_plan_sha256, backup.path())
            .expect("migration");

        assert_eq!(result.application_ids.len(), 1);
        assert_eq!(
            result.source_inventory_sha256,
            preview.legacy_inventory_sha256
        );
        assert_eq!(
            result.post_migration_inventory_sha256,
            preview.legacy_inventory_sha256
        );
        assert_eq!(result.legacy_binding_count, preview.legacy_inventory_count);
        assert_eq!(
            workspace
                .status()
                .expect("migrated status")
                .workspace_format,
            WORKSPACE_V3_FORMAT
        );
        assert_eq!(
            workspace.database.referenced_digests().expect("Blob refs"),
            referenced_before
        );
        assert_eq!(
            projection_state_digest(workspace.database.connection()).expect("projection digest"),
            projection_before
        );
        let applications = ApplicationModelRepository::new(&mut workspace.database)
            .list()
            .expect("v3 Applications");
        assert_eq!(applications.len(), 1);
        assert_eq!(applications[0].snapshot.requirements.len(), 1);
        assert_eq!(
            applications[0].snapshot.requirements[0].confirmation,
            RequirementConfirmationV3::Confirmed
        );
        assert_eq!(
            verify_backup(backup.path())
                .expect("verified backup")
                .format,
            BACKUP_FORMAT
        );

        drop(workspace);
        let mut restored_workspace = Workspace::restore(backup.path(), restored.path())
            .expect("restore pre-migration Workspace");
        assert_eq!(restored_workspace.status().expect("status").job_count, 1);
        assert!(matches!(
            ApplicationModelRepository::new(&mut restored_workspace.database).authority(),
            Err(StoreError::ApplicationModelUnavailable)
        ));
    }

    fn academic_pack() -> VerifiedWorkflowPackBundle {
        academic_pack_with_origin(WorkflowPackOrigin::BuiltIn)
    }

    fn academic_pack_with_origin(origin: WorkflowPackOrigin) -> VerifiedWorkflowPackBundle {
        let embedded = academic_job_workflow_pack();
        WorkflowPackByteLoader::verify(
            embedded.manifest_bytes(),
            embedded.into_resources(),
            origin,
            &WorkflowPackRuntime::parse("1.0.0-alpha.5", "3.0.0-alpha.1", "3.0.0-alpha.1")
                .expect("runtime"),
            &WorkflowPackCapabilityRegistry::built_in(),
        )
        .expect("verified academic Pack")
        .into_bundle()
    }
}
