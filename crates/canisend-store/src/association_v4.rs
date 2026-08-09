use std::collections::BTreeSet;

use canisend_contracts::{
    ActorKind, ApplicationEvidenceAssociationV4, ApplicationId, ApplicationProfileAssociationV4,
    ApplicationSourceAssociationV4, ConsentScope, ContentRevisionReferenceV3, EntityId,
    PrivacyClassification, Revision, Sha256Digest, UtcTimestamp, WorkspaceEvidenceSummaryV4,
    WorkspaceSourceKindV4, WorkspaceSourceRevisionV4,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::{
    BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError,
    application_storage::ApplicationStorage,
    application_v3::{enum_name, to_i64},
    generate_id, now_utc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewWorkspaceSourceV4 {
    pub kind: WorkspaceSourceKindV4,
    pub locator: String,
    pub final_locator: Option<String>,
    pub redirect_chain: Vec<String>,
    pub content_type: String,
    pub original_bytes: Vec<u8>,
    pub normalized_text: String,
    pub privacy: PrivacyClassification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedWorkspaceSourceV4 {
    pub record: WorkspaceSourceRevisionV4,
}

pub struct ApplicationAssociationServiceV4<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> ApplicationAssociationServiceV4<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn import_source(
        &mut self,
        source: NewWorkspaceSourceV4,
        actor: ActorKind,
    ) -> Result<WorkspaceSourceRevisionV4, StoreError> {
        let prepared = prepare_source(self.blobs, source, generate_id()?, Revision::try_new(1)?)?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        insert_source_revision(&transaction, &prepared.record, true)?;
        insert_source_blob_references(&transaction, &prepared.record)?;
        insert_association_audit(
            &transaction,
            &event_id,
            actor,
            "source-v4.import",
            &prepared.record.id,
            Some(prepared.record.revision),
            "workspace-source-import",
            &prepared.record.created_at,
        )?;
        transaction.commit()?;
        Ok(prepared.record)
    }

    pub fn revise_source(
        &mut self,
        source_id: &EntityId,
        expected_revision: Revision,
        source: NewWorkspaceSourceV4,
        actor: ActorKind,
    ) -> Result<WorkspaceSourceRevisionV4, StoreError> {
        let next_revision =
            Revision::try_new(expected_revision.get().checked_add(1).ok_or_else(|| {
                StoreError::ApplicationAssociationConflict("Source revision overflow".to_owned())
            })?)?;
        let prepared = prepare_source(self.blobs, source, source_id.clone(), next_revision)?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        let (stored_kind, head_revision): (String, i64) = transaction
            .query_row(
                "SELECT kind, head_revision FROM workspace_source_v4_heads WHERE source_id = ?1",
                [source_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::ApplicationAssociationNotFound(source_id.to_string()))?;
        if stored_kind != enum_name(prepared.record.kind)? {
            return Err(StoreError::ApplicationAssociationConflict(
                "Source kind is immutable".to_owned(),
            ));
        }
        if to_u64(head_revision)? != expected_revision.get() {
            return Err(StoreError::ApplicationAssociationConflict(format!(
                "Source revision is stale; expected {}, found {}",
                expected_revision.get(),
                head_revision
            )));
        }
        insert_source_revision(&transaction, &prepared.record, false)?;
        let updated = transaction.execute(
            "UPDATE workspace_source_v4_heads SET head_revision = ?2
             WHERE source_id = ?1 AND head_revision = ?3",
            params![
                source_id.as_str(),
                to_i64(next_revision.get())?,
                to_i64(expected_revision.get())?
            ],
        )?;
        if updated != 1 {
            return Err(StoreError::ApplicationAssociationConflict(
                "Source changed while committing its revision".to_owned(),
            ));
        }
        insert_source_blob_references(&transaction, &prepared.record)?;
        insert_association_audit(
            &transaction,
            &event_id,
            actor,
            "source-v4.revise",
            source_id,
            Some(next_revision),
            "workspace-source-revise",
            &prepared.record.created_at,
        )?;
        transaction.commit()?;
        Ok(prepared.record)
    }

    pub fn source(
        &self,
        source_id: &EntityId,
        revision: Revision,
    ) -> Result<WorkspaceSourceRevisionV4, StoreError> {
        load_source_revision(self.database.connection(), source_id, revision)
    }

    pub fn source_duplicates(
        &self,
        original_sha256: &Sha256Digest,
        normalized_sha256: &Sha256Digest,
    ) -> Result<Vec<WorkspaceSourceRevisionV4>, StoreError> {
        let references = {
            let mut statement = self.database.connection().prepare(
                "SELECT source_id, revision
                 FROM workspace_source_v4_revisions
                 WHERE original_sha256 = ?1 OR normalized_sha256 = ?2
                 ORDER BY source_id, revision",
            )?;
            statement
                .query_map(
                    params![original_sha256.as_str(), normalized_sha256.as_str()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )?
                .collect::<Result<Vec<_>, _>>()?
        };
        references
            .into_iter()
            .map(|(source_id, revision)| {
                load_source_revision(
                    self.database.connection(),
                    &EntityId::try_new(source_id)?,
                    Revision::try_new(to_u64(revision)?)?,
                )
            })
            .collect()
    }

    pub fn associate_source(
        &mut self,
        application_id: &ApplicationId,
        source: &ContentRevisionReferenceV3,
        consent: Option<ConsentScope>,
        actor: ActorKind,
    ) -> Result<ApplicationSourceAssociationV4, StoreError> {
        let record = load_source_revision(self.database.connection(), &source.id, source.revision)?;
        if record.normalized_sha256 != source.sha256 {
            return Err(StoreError::ApplicationAssociationConflict(
                "Source digest does not match the exact Source revision".to_owned(),
            ));
        }
        require_consent(source_consent(&record), consent)?;
        let associated_at = now_utc()?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        ensure_application(&transaction, application_id)?;
        insert_source_association(
            &transaction,
            application_id,
            source,
            consent,
            &associated_at,
        )?;
        insert_association_audit(
            &transaction,
            &event_id,
            actor,
            "application-v4.source-associate",
            &source.id,
            Some(source.revision),
            "explicit-application-source-link",
            &associated_at,
        )?;
        transaction.commit()?;
        Ok(ApplicationSourceAssociationV4 {
            application_id: application_id.clone(),
            source: source.clone(),
            consent_scope: consent,
            associated_at,
            stale: false,
        })
    }

    pub fn source_associations(
        &self,
        application_id: &ApplicationId,
    ) -> Result<Vec<ApplicationSourceAssociationV4>, StoreError> {
        ensure_application(self.database.connection(), application_id)?;
        let storage = ApplicationStorage::detect(self.database.connection())?;
        let mut statement = self.database.connection().prepare(&format!(
            "SELECT association.source_id, association.source_revision,
                    association.source_sha256, association.consent_scope,
                    association.associated_at,
                    association.source_revision <> head.head_revision
             FROM {} AS association
             JOIN workspace_source_v4_heads AS head ON head.source_id = association.source_id
             WHERE association.application_id = ?1
             ORDER BY association.associated_at, association.source_id",
            storage.source_associations()
        ))?;
        statement
            .query_map([application_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, bool>(5)?,
                ))
            })?
            .map(|row| {
                let (id, revision, sha256, consent, associated_at, stale) = row?;
                Ok(ApplicationSourceAssociationV4 {
                    application_id: application_id.clone(),
                    source: exact_reference(id, revision, sha256)?,
                    consent_scope: optional_enum(consent)?,
                    associated_at: UtcTimestamp::try_new(associated_at)?,
                    stale,
                })
            })
            .collect()
    }

    pub fn associate_profile_source(
        &mut self,
        application_id: &ApplicationId,
        profile_source: &ContentRevisionReferenceV3,
        consent: Option<ConsentScope>,
        actor: ActorKind,
    ) -> Result<ApplicationProfileAssociationV4, StoreError> {
        let (sha256, sensitivity) = self
            .database
            .connection()
            .query_row(
                "SELECT sha256, sensitivity FROM profile_source_revisions
                 WHERE source_id = ?1 AND revision = ?2",
                params![
                    profile_source.id.as_str(),
                    to_i64(profile_source.revision.get())?
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| {
                StoreError::ApplicationAssociationNotFound(profile_source.id.to_string())
            })?;
        if sha256 != profile_source.sha256.as_str() {
            return Err(StoreError::ApplicationAssociationConflict(
                "Profile Source digest does not match the exact revision".to_owned(),
            ));
        }
        require_consent(privacy_consent(enum_value(&sensitivity)?), consent)?;
        let associated_at = now_utc()?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        ensure_application(&transaction, application_id)?;
        let storage = ApplicationStorage::detect(&transaction)?;
        transaction.execute(
            &format!(
                "INSERT INTO {}(
                application_id, profile_source_id, profile_source_revision,
                profile_source_sha256, consent_scope, associated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                storage.profile_associations()
            ),
            params![
                application_id.as_str(),
                profile_source.id.as_str(),
                to_i64(profile_source.revision.get())?,
                profile_source.sha256.as_str(),
                optional_enum_name(consent)?,
                associated_at.as_str()
            ],
        )?;
        insert_association_audit(
            &transaction,
            &event_id,
            actor,
            "application-v4.profile-associate",
            &profile_source.id,
            Some(profile_source.revision),
            "explicit-application-profile-link",
            &associated_at,
        )?;
        transaction.commit()?;
        Ok(ApplicationProfileAssociationV4 {
            application_id: application_id.clone(),
            profile_source: profile_source.clone(),
            consent_scope: consent,
            associated_at,
            stale: false,
        })
    }

    pub fn profile_associations(
        &self,
        application_id: &ApplicationId,
    ) -> Result<Vec<ApplicationProfileAssociationV4>, StoreError> {
        ensure_application(self.database.connection(), application_id)?;
        let storage = ApplicationStorage::detect(self.database.connection())?;
        let mut statement = self.database.connection().prepare(&format!(
            "SELECT association.profile_source_id, association.profile_source_revision,
                    association.profile_source_sha256, association.consent_scope,
                    association.associated_at,
                    association.profile_source_revision <> latest.revision
             FROM {} AS association
             JOIN (
                 SELECT source_id, MAX(revision) AS revision
                 FROM profile_source_revisions GROUP BY source_id
             ) AS latest ON latest.source_id = association.profile_source_id
             WHERE association.application_id = ?1
             ORDER BY association.associated_at, association.profile_source_id",
            storage.profile_associations()
        ))?;
        statement
            .query_map([application_id.as_str()], association_row)?
            .map(|row| {
                let (reference, consent_scope, associated_at, stale) = row?;
                Ok(ApplicationProfileAssociationV4 {
                    application_id: application_id.clone(),
                    profile_source: reference,
                    consent_scope,
                    associated_at,
                    stale,
                })
            })
            .collect()
    }

    pub fn associate_evidence(
        &mut self,
        application_id: &ApplicationId,
        evidence: &ContentRevisionReferenceV3,
        consent: Option<ConsentScope>,
        actor: ActorKind,
    ) -> Result<ApplicationEvidenceAssociationV4, StoreError> {
        let (sha256, confirmed, excluded, sensitivity) = self
            .database
            .connection()
            .query_row(
                "SELECT sha256, confirmed, excluded, sensitivity FROM evidence_revisions
                 WHERE evidence_id = ?1 AND revision = ?2",
                params![evidence.id.as_str(), to_i64(evidence.revision.get())?],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, bool>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| StoreError::ApplicationAssociationNotFound(evidence.id.to_string()))?;
        if sha256 != evidence.sha256.as_str() {
            return Err(StoreError::ApplicationAssociationConflict(
                "Evidence digest does not match the exact revision".to_owned(),
            ));
        }
        if !confirmed || excluded {
            return Err(StoreError::ApplicationAssociationConflict(
                "only confirmed, non-excluded Evidence may be associated".to_owned(),
            ));
        }
        let sensitivity = sensitivity.ok_or_else(|| {
            StoreError::ApplicationAssociationConflict(
                "Evidence revision has no sensitivity classification".to_owned(),
            )
        })?;
        require_consent(privacy_consent(enum_value(&sensitivity)?), consent)?;
        let associated_at = now_utc()?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        ensure_application(&transaction, application_id)?;
        let storage = ApplicationStorage::detect(&transaction)?;
        transaction.execute(
            &format!(
                "INSERT INTO {}(
                application_id, evidence_id, evidence_revision, evidence_sha256,
                consent_scope, associated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                storage.evidence_associations()
            ),
            params![
                application_id.as_str(),
                evidence.id.as_str(),
                to_i64(evidence.revision.get())?,
                evidence.sha256.as_str(),
                optional_enum_name(consent)?,
                associated_at.as_str()
            ],
        )?;
        insert_association_audit(
            &transaction,
            &event_id,
            actor,
            "application-v4.evidence-associate",
            &evidence.id,
            Some(evidence.revision),
            "explicit-application-evidence-link",
            &associated_at,
        )?;
        transaction.commit()?;
        Ok(ApplicationEvidenceAssociationV4 {
            application_id: application_id.clone(),
            evidence: evidence.clone(),
            consent_scope: consent,
            associated_at,
            stale: false,
        })
    }

    pub fn evidence_associations(
        &self,
        application_id: &ApplicationId,
    ) -> Result<Vec<ApplicationEvidenceAssociationV4>, StoreError> {
        ensure_application(self.database.connection(), application_id)?;
        let storage = ApplicationStorage::detect(self.database.connection())?;
        let mut statement = self.database.connection().prepare(&format!(
            "SELECT association.evidence_id, association.evidence_revision,
                    association.evidence_sha256, association.consent_scope,
                    association.associated_at,
                    association.evidence_revision <> latest.revision
             FROM {} AS association
             JOIN (
                 SELECT evidence_id, MAX(revision) AS revision
                 FROM evidence_revisions GROUP BY evidence_id
             ) AS latest ON latest.evidence_id = association.evidence_id
             WHERE association.application_id = ?1
             ORDER BY association.associated_at, association.evidence_id",
            storage.evidence_associations()
        ))?;
        statement
            .query_map([application_id.as_str()], association_row)?
            .map(|row| {
                let (reference, consent_scope, associated_at, stale) = row?;
                Ok(ApplicationEvidenceAssociationV4 {
                    application_id: application_id.clone(),
                    evidence: reference,
                    consent_scope,
                    associated_at,
                    stale,
                })
            })
            .collect()
    }

    /// Lists only current, confirmed, non-excluded Evidence metadata. Private bodies and source
    /// quotes are intentionally absent from this routine Workspace inventory.
    pub fn confirmed_evidence(&self) -> Result<Vec<WorkspaceEvidenceSummaryV4>, StoreError> {
        let mut statement = self.database.connection().prepare(
            "SELECT item.id, item.kind, revision.revision, revision.sha256,
                    revision.sensitivity, revision.created_at
             FROM evidence_items AS item
             JOIN evidence_revisions AS revision ON revision.evidence_id = item.id
             WHERE revision.revision = (
                 SELECT MAX(head.revision) FROM evidence_revisions AS head
                 WHERE head.evidence_id = item.id
             )
               AND revision.confirmed = 1
               AND revision.excluded = 0
               AND revision.sensitivity IS NOT NULL
             ORDER BY revision.created_at, item.id",
        )?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .map(|row| {
                let (id, kind, revision, sha256, sensitivity, created_at) = row?;
                Ok(WorkspaceEvidenceSummaryV4 {
                    evidence: exact_reference(id, revision, sha256)?,
                    kind,
                    sensitivity: enum_value(&sensitivity)?,
                    created_at: UtcTimestamp::try_new(created_at)?,
                })
            })
            .collect()
    }

    pub fn evidence_revision_summary(
        &self,
        evidence: &ContentRevisionReferenceV3,
    ) -> Result<WorkspaceEvidenceSummaryV4, StoreError> {
        let row = self
            .database
            .connection()
            .query_row(
                "SELECT item.kind, revision.sha256, revision.sensitivity, revision.created_at
                 FROM evidence_items AS item
                 JOIN evidence_revisions AS revision ON revision.evidence_id = item.id
                 WHERE item.id = ?1 AND revision.revision = ?2",
                params![evidence.id.as_str(), to_i64(evidence.revision.get())?],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| StoreError::ApplicationAssociationNotFound(evidence.id.to_string()))?;
        let (kind, sha256, sensitivity, created_at) = row;
        if sha256 != evidence.sha256.as_str() {
            return Err(StoreError::ApplicationAssociationConflict(
                "Evidence digest does not match the exact revision".to_owned(),
            ));
        }
        let sensitivity = sensitivity.ok_or_else(|| {
            StoreError::ApplicationAssociationConflict(
                "Evidence revision has no sensitivity classification".to_owned(),
            )
        })?;
        Ok(WorkspaceEvidenceSummaryV4 {
            evidence: evidence.clone(),
            kind,
            sensitivity: enum_value(&sensitivity)?,
            created_at: UtcTimestamp::try_new(created_at)?,
        })
    }

    pub fn unlink_source(
        &mut self,
        application_id: &ApplicationId,
        source_id: &EntityId,
        actor: ActorKind,
    ) -> Result<(), StoreError> {
        let table = ApplicationStorage::detect(self.database.connection())?.source_associations();
        self.unlink(
            table,
            "source_id",
            application_id,
            source_id,
            actor,
            "application-v4.source-unlink",
        )
    }

    pub fn unlink_profile_source(
        &mut self,
        application_id: &ApplicationId,
        source_id: &EntityId,
        actor: ActorKind,
    ) -> Result<(), StoreError> {
        let table = ApplicationStorage::detect(self.database.connection())?.profile_associations();
        self.unlink(
            table,
            "profile_source_id",
            application_id,
            source_id,
            actor,
            "application-v4.profile-unlink",
        )
    }

    pub fn unlink_evidence(
        &mut self,
        application_id: &ApplicationId,
        evidence_id: &EntityId,
        actor: ActorKind,
    ) -> Result<(), StoreError> {
        let table = ApplicationStorage::detect(self.database.connection())?.evidence_associations();
        self.unlink(
            table,
            "evidence_id",
            application_id,
            evidence_id,
            actor,
            "application-v4.evidence-unlink",
        )
    }

    pub fn delete_source(
        &mut self,
        source_id: &EntityId,
        actor: ActorKind,
    ) -> Result<(), StoreError> {
        let deleted_at = now_utc()?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        let storage = ApplicationStorage::detect(&transaction)?;
        let links: i64 = transaction.query_row(
            &format!(
                "SELECT COUNT(*) FROM {} WHERE source_id = ?1",
                storage.source_associations()
            ),
            [source_id.as_str()],
            |row| row.get(0),
        )?;
        if links != 0 {
            return Err(StoreError::ApplicationAssociationConflict(format!(
                "Source {source_id} remains associated with {links} Application(s)"
            )));
        }
        let revisions: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM workspace_source_v4_revisions WHERE source_id = ?1",
            [source_id.as_str()],
            |row| row.get(0),
        )?;
        if revisions == 0 {
            return Err(StoreError::ApplicationAssociationNotFound(
                source_id.to_string(),
            ));
        }
        transaction.execute(
            "DELETE FROM blob_references
             WHERE owner_type IN ('workspace-v4-source-original', 'workspace-v4-source-normalized')
               AND owner_id = ?1",
            [source_id.as_str()],
        )?;
        transaction.execute(
            "DELETE FROM workspace_source_v4_revisions WHERE source_id = ?1",
            [source_id.as_str()],
        )?;
        transaction.execute(
            "DELETE FROM workspace_source_v4_heads WHERE source_id = ?1",
            [source_id.as_str()],
        )?;
        insert_association_audit(
            &transaction,
            &event_id,
            actor,
            "source-v4.delete",
            source_id,
            None,
            "workspace-source-delete",
            &deleted_at,
        )?;
        transaction.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn unlink(
        &mut self,
        table: &str,
        id_column: &str,
        application_id: &ApplicationId,
        resource_id: &EntityId,
        actor: ActorKind,
        action: &str,
    ) -> Result<(), StoreError> {
        let allowed = BTreeSet::from([
            ("application_source_associations_v4", "source_id"),
            ("application_profile_associations_v4", "profile_source_id"),
            ("application_evidence_associations_v4", "evidence_id"),
            ("application_source_v4_associations", "source_id"),
            ("application_profile_v4_associations", "profile_source_id"),
            ("application_evidence_v4_associations", "evidence_id"),
        ]);
        if !allowed.contains(&(table, id_column)) {
            return Err(StoreError::Invariant(
                "unrecognized association table".to_owned(),
            ));
        }
        let unlinked_at = now_utc()?;
        let event_id = generate_id()?;
        let transaction = self.database.immediate_transaction()?;
        ensure_application(&transaction, application_id)?;
        let deleted = transaction.execute(
            &format!("DELETE FROM {table} WHERE application_id = ?1 AND {id_column} = ?2"),
            params![application_id.as_str(), resource_id.as_str()],
        )?;
        if deleted != 1 {
            return Err(StoreError::ApplicationAssociationNotFound(format!(
                "{application_id}:{resource_id}"
            )));
        }
        insert_association_audit(
            &transaction,
            &event_id,
            actor,
            action,
            resource_id,
            None,
            "explicit-application-resource-unlink",
            &unlinked_at,
        )?;
        transaction.commit()?;
        Ok(())
    }
}

pub(crate) fn prepare_source(
    blobs: &BlobStore,
    source: NewWorkspaceSourceV4,
    id: EntityId,
    revision: Revision,
) -> Result<PreparedWorkspaceSourceV4, StoreError> {
    validate_source_input(&source)?;
    let original_sha256 = blobs.put_bytes(&source.original_bytes)?;
    let normalized_sha256 = blobs.put_bytes(source.normalized_text.as_bytes())?;
    Ok(PreparedWorkspaceSourceV4 {
        record: WorkspaceSourceRevisionV4 {
            id,
            revision,
            kind: source.kind,
            locator: source.locator,
            final_locator: source.final_locator,
            redirect_chain: source.redirect_chain,
            content_type: source.content_type,
            original_sha256,
            normalized_sha256,
            original_bytes: u64::try_from(source.original_bytes.len())
                .map_err(|_| StoreError::BlobTooLarge { limit: u64::MAX })?,
            normalized_text_bytes: u64::try_from(source.normalized_text.len())
                .map_err(|_| StoreError::BlobTooLarge { limit: u64::MAX })?,
            privacy: source.privacy,
            created_at: now_utc()?,
        },
    })
}

pub(crate) fn insert_prepared_source_association(
    transaction: &Transaction<'_>,
    application_id: &ApplicationId,
    prepared: &PreparedWorkspaceSourceV4,
    consent: Option<ConsentScope>,
) -> Result<(), StoreError> {
    let record = &prepared.record;
    require_consent(source_consent(record), consent)?;
    insert_source_revision(transaction, record, true)?;
    insert_source_blob_references(transaction, record)?;
    insert_source_association(
        transaction,
        application_id,
        &ContentRevisionReferenceV3 {
            id: record.id.clone(),
            revision: record.revision,
            sha256: record.normalized_sha256.clone(),
        },
        consent,
        &record.created_at,
    )
}

fn insert_source_revision(
    transaction: &Transaction<'_>,
    record: &WorkspaceSourceRevisionV4,
    insert_head: bool,
) -> Result<(), StoreError> {
    let redirect_chain_json = serde_json::to_string(&record.redirect_chain)?;
    if insert_head {
        transaction.execute(
            "INSERT INTO workspace_source_v4_heads(source_id, kind, head_revision, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                record.id.as_str(),
                enum_name(record.kind)?,
                to_i64(record.revision.get())?,
                record.created_at.as_str()
            ],
        )?;
    }
    transaction.execute(
        "INSERT INTO workspace_source_v4_revisions(
            source_id, revision, locator, final_locator, redirect_chain_json, content_type,
            original_sha256, normalized_sha256, original_bytes, normalized_text_bytes,
            privacy, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            record.id.as_str(),
            to_i64(record.revision.get())?,
            record.locator,
            record.final_locator.as_deref(),
            redirect_chain_json,
            record.content_type,
            record.original_sha256.as_str(),
            record.normalized_sha256.as_str(),
            to_i64(record.original_bytes)?,
            to_i64(record.normalized_text_bytes)?,
            enum_name(record.privacy)?,
            record.created_at.as_str()
        ],
    )?;
    Ok(())
}

fn insert_source_blob_references(
    transaction: &Transaction<'_>,
    record: &WorkspaceSourceRevisionV4,
) -> Result<(), StoreError> {
    for (owner_type, digest) in [
        ("workspace-v4-source-original", &record.original_sha256),
        ("workspace-v4-source-normalized", &record.normalized_sha256),
    ] {
        transaction.execute(
            "INSERT OR IGNORE INTO blob_references(
                sha256, owner_type, owner_id, owner_revision, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                digest.as_str(),
                owner_type,
                record.id.as_str(),
                to_i64(record.revision.get())?,
                record.created_at.as_str()
            ],
        )?;
    }
    Ok(())
}

fn insert_source_association(
    transaction: &Transaction<'_>,
    application_id: &ApplicationId,
    source: &ContentRevisionReferenceV3,
    consent: Option<ConsentScope>,
    associated_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    let storage = ApplicationStorage::detect(transaction)?;
    transaction.execute(
        &format!(
            "INSERT INTO {}(
            application_id, source_id, source_revision, source_sha256,
            consent_scope, associated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            storage.source_associations()
        ),
        params![
            application_id.as_str(),
            source.id.as_str(),
            to_i64(source.revision.get())?,
            source.sha256.as_str(),
            optional_enum_name(consent)?,
            associated_at.as_str()
        ],
    )?;
    Ok(())
}

fn load_source_revision(
    connection: &Connection,
    source_id: &EntityId,
    revision: Revision,
) -> Result<WorkspaceSourceRevisionV4, StoreError> {
    type SourceRow = (
        String,
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        i64,
        i64,
        String,
        String,
    );
    let row: Option<SourceRow> = connection
        .query_row(
            "SELECT head.kind, revision.locator, revision.final_locator,
                    revision.redirect_chain_json, revision.content_type,
                    revision.original_sha256, revision.normalized_sha256,
                    revision.original_bytes, revision.normalized_text_bytes,
                    revision.privacy, revision.created_at
             FROM workspace_source_v4_revisions AS revision
             JOIN workspace_source_v4_heads AS head ON head.source_id = revision.source_id
             WHERE revision.source_id = ?1 AND revision.revision = ?2",
            params![source_id.as_str(), to_i64(revision.get())?],
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
        .optional()?;
    let (
        kind,
        locator,
        final_locator,
        redirect_chain_json,
        content_type,
        original,
        normalized,
        original_bytes,
        normalized_bytes,
        privacy,
        created_at,
    ) = row.ok_or_else(|| StoreError::ApplicationAssociationNotFound(source_id.to_string()))?;
    Ok(WorkspaceSourceRevisionV4 {
        id: source_id.clone(),
        revision,
        kind: enum_value(&kind)?,
        locator,
        final_locator,
        redirect_chain: serde_json::from_str(&redirect_chain_json)?,
        content_type,
        original_sha256: Sha256Digest::try_new(original)?,
        normalized_sha256: Sha256Digest::try_new(normalized)?,
        original_bytes: to_u64(original_bytes)?,
        normalized_text_bytes: to_u64(normalized_bytes)?,
        privacy: enum_value(&privacy)?,
        created_at: UtcTimestamp::try_new(created_at)?,
    })
}

fn ensure_application(
    connection: &Connection,
    application_id: &ApplicationId,
) -> Result<(), StoreError> {
    let storage = ApplicationStorage::detect(connection)?;
    let exists = connection
        .query_row(
            &format!(
                "SELECT 1 FROM {} WHERE application_id = ?1",
                storage.heads()
            ),
            [application_id.as_str()],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(StoreError::ApplicationModelNotFound(
            application_id.to_string(),
        ))
    }
}

fn source_consent(record: &WorkspaceSourceRevisionV4) -> Option<ConsentScope> {
    source_kind_consent(record.kind, record.privacy)
}

pub(crate) fn validate_new_source_consent(
    source: &NewWorkspaceSourceV4,
    provided: Option<ConsentScope>,
) -> Result<(), StoreError> {
    let required = if source.kind == WorkspaceSourceKindV4::Url {
        Some(ConsentScope::FetchUserSuppliedUrl)
    } else {
        source_kind_consent(source.kind, source.privacy)
    };
    require_consent(required, provided)
}

fn source_kind_consent(
    kind: WorkspaceSourceKindV4,
    privacy: PrivacyClassification,
) -> Option<ConsentScope> {
    match kind {
        WorkspaceSourceKindV4::PastedText | WorkspaceSourceKindV4::Url => None,
        WorkspaceSourceKindV4::LocalFile | WorkspaceSourceKindV4::TextPdf => {
            privacy_consent(privacy)
        }
    }
}

fn privacy_consent(privacy: PrivacyClassification) -> Option<ConsentScope> {
    match privacy {
        PrivacyClassification::Public => None,
        PrivacyClassification::PrivateLocal
        | PrivacyClassification::ProviderBound
        | PrivacyClassification::Secret => Some(ConsentScope::ReadPrivateInputs),
    }
}

fn require_consent(
    required: Option<ConsentScope>,
    provided: Option<ConsentScope>,
) -> Result<(), StoreError> {
    match (required, provided) {
        (None, _) => Ok(()),
        (Some(required), Some(provided)) if required == provided => Ok(()),
        (Some(required), _) => Err(StoreError::ApplicationAssociationConsentRequired(
            enum_name(required)?,
        )),
    }
}

fn validate_source_input(source: &NewWorkspaceSourceV4) -> Result<(), StoreError> {
    if invalid_locator(&source.locator) {
        return Err(StoreError::InvalidInput(
            "Source locator must contain 1 to 4096 bytes".to_owned(),
        ));
    }
    if source
        .final_locator
        .as_ref()
        .is_some_and(|locator| invalid_locator(locator))
        || source.redirect_chain.len() > 5
        || source
            .redirect_chain
            .iter()
            .any(|locator| invalid_locator(locator))
    {
        return Err(StoreError::InvalidInput(
            "Source provenance locators must be bounded non-control text with at most 5 redirects"
                .to_owned(),
        ));
    }
    if source.kind == WorkspaceSourceKindV4::Url {
        if source.final_locator.is_none() {
            return Err(StoreError::InvalidInput(
                "URL Source provenance requires a final locator".to_owned(),
            ));
        }
    } else if source.final_locator.is_some() || !source.redirect_chain.is_empty() {
        return Err(StoreError::InvalidInput(
            "non-URL Source provenance cannot contain a final locator or redirect chain".to_owned(),
        ));
    }
    if source.content_type.trim().is_empty() || source.content_type.len() > 255 {
        return Err(StoreError::InvalidInput(
            "Source content type must contain 1 to 255 bytes".to_owned(),
        ));
    }
    let limit = usize::try_from(DEFAULT_MAX_BLOB_BYTES).unwrap_or(usize::MAX);
    if source.original_bytes.len() > limit || source.normalized_text.len() > limit {
        return Err(StoreError::BlobTooLarge {
            limit: DEFAULT_MAX_BLOB_BYTES,
        });
    }
    if source.normalized_text.trim().is_empty() {
        return Err(StoreError::InvalidInput(
            "normalized Source text cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn invalid_locator(locator: &str) -> bool {
    locator.trim().is_empty() || locator.len() > 4096 || locator.chars().any(char::is_control)
}

fn association_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(
    ContentRevisionReferenceV3,
    Option<ConsentScope>,
    UtcTimestamp,
    bool,
)> {
    let id = row.get::<_, String>(0)?;
    let revision = row.get::<_, i64>(1)?;
    let sha256 = row.get::<_, String>(2)?;
    let consent = row.get::<_, Option<String>>(3)?;
    let associated_at = row.get::<_, String>(4)?;
    let stale = row.get::<_, bool>(5)?;
    Ok((
        exact_reference(id, revision, sha256).map_err(to_sql_error)?,
        optional_enum(consent).map_err(to_sql_error)?,
        UtcTimestamp::try_new(associated_at).map_err(to_sql_error)?,
        stale,
    ))
}

fn exact_reference(
    id: String,
    revision: i64,
    sha256: String,
) -> Result<ContentRevisionReferenceV3, StoreError> {
    Ok(ContentRevisionReferenceV3 {
        id: EntityId::try_new(id)?,
        revision: Revision::try_new(to_u64(revision)?)?,
        sha256: Sha256Digest::try_new(sha256)?,
    })
}

fn optional_enum_name<T: serde::Serialize>(value: Option<T>) -> Result<Option<String>, StoreError> {
    value.map(enum_name).transpose()
}

fn optional_enum<T: serde::de::DeserializeOwned>(
    value: Option<String>,
) -> Result<Option<T>, StoreError> {
    value.map(|value| enum_value(&value)).transpose()
}

fn enum_value<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, StoreError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(StoreError::from)
}

fn to_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite integer".to_owned()))
}

fn to_sql_error(error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        )),
    )
}

#[allow(clippy::too_many_arguments)]
fn insert_association_audit(
    transaction: &Transaction<'_>,
    event_id: &EntityId,
    actor: ActorKind,
    action: &str,
    subject_id: &EntityId,
    subject_revision: Option<Revision>,
    reason: &str,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event_id.as_str(),
            enum_name(actor)?,
            action,
            subject_id.as_str(),
            subject_revision
                .map(Revision::get)
                .map(to_i64)
                .transpose()?,
            reason,
            created_at.as_str()
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, path::PathBuf};

    use canisend_contracts::{
        ApplicationFieldValueV3, RequirementPriorityV3, WorkflowPackId, WorkflowPackItemId,
    };
    use canisend_core::{
        VerifiedWorkflowPackBundle, WorkflowPackByteLoader, WorkflowPackCapabilityRegistry,
        WorkflowPackOrigin, WorkflowPackRuntime,
    };
    use canisend_resources::{
        EmbeddedWorkflowPack, academic_job_workflow_pack, generic_application_workflow_pack,
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::{
        ApplicationFlowCreateRequestV3, ApplicationFlowRequirementDraftV3,
        ApplicationFlowServiceV3, NewProfileSource, ProfileService, Workspace,
    };

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-association-v4-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn bundle(embedded: EmbeddedWorkflowPack) -> VerifiedWorkflowPackBundle {
        WorkflowPackByteLoader::verify(
            embedded.manifest_bytes(),
            embedded.into_resources(),
            WorkflowPackOrigin::BuiltIn,
            &WorkflowPackRuntime::parse(
                env!("CARGO_PKG_VERSION"),
                "3.0.0-alpha.1",
                "3.0.0-alpha.1",
            )
            .expect("runtime"),
            &WorkflowPackCapabilityRegistry::built_in(),
        )
        .expect("verified Pack")
        .into_bundle()
    }

    fn create_application(
        workspace: &mut Workspace,
        pack: &VerifiedWorkflowPackBundle,
        title: &str,
        source_text: &str,
        category: &str,
        opportunity_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
        application_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    ) -> ApplicationId {
        let root = workspace.paths.root.clone();
        ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &root)
            .create(
                pack,
                ApplicationFlowCreateRequestV3 {
                    title: title.to_owned(),
                    opportunity_metadata,
                    application_metadata,
                    source_text: source_text.to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: item(category),
                        statement: source_text.to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: u64::try_from(source_text.len()).expect("source length"),
                    }],
                },
            )
            .expect("Application")
            .stored
            .snapshot
            .application
            .id
    }

    fn mixed_applications(workspace: &mut Workspace) -> (ApplicationId, ApplicationId) {
        let generic = bundle(generic_application_workflow_pack());
        let academic = bundle(academic_job_workflow_pack());
        let generic_id = create_application(
            workspace,
            &generic,
            "Community programme",
            "Provide a project narrative.",
            "format",
            BTreeMap::from([
                (
                    item("organization"),
                    ApplicationFieldValueV3::ShortText("Example Foundation".to_owned()),
                ),
                (
                    item("reference"),
                    ApplicationFieldValueV3::ShortText("ASSOC-001".to_owned()),
                ),
            ]),
            BTreeMap::from([(
                item("status"),
                ApplicationFieldValueV3::Choice(item("planning")),
            )]),
        );
        let academic_id = create_application(
            workspace,
            &academic,
            "Research fellowship",
            "Applicants must provide an academic CV.",
            "qualification",
            BTreeMap::from([(
                item("institution"),
                ApplicationFieldValueV3::ShortText("Example University".to_owned()),
            )]),
            BTreeMap::new(),
        );
        (generic_id, academic_id)
    }

    fn source_reference(source: &WorkspaceSourceRevisionV4) -> ContentRevisionReferenceV3 {
        ContentRevisionReferenceV3 {
            id: source.id.clone(),
            revision: source.revision,
            sha256: source.normalized_sha256.clone(),
        }
    }

    #[test]
    fn application_creation_atomically_persists_exact_source_links_for_both_packs() {
        let root = root("create-links");
        let mut workspace = Workspace::init_v4(&root).expect("Workspace v4");
        let (generic_id, academic_id) = mixed_applications(&mut workspace);
        let service =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);

        let generic = service
            .source_associations(&generic_id)
            .expect("generic Source link");
        let academic = service
            .source_associations(&academic_id)
            .expect("academic Source link");
        assert_eq!(generic.len(), 1);
        assert_eq!(academic.len(), 1);
        assert_ne!(generic[0].source.id, academic[0].source.id);
        assert!(!generic[0].stale);
        assert!(!academic[0].stale);

        let source = service
            .source(&generic[0].source.id, generic[0].source.revision)
            .expect("exact Source revision");
        assert_eq!(source.kind, WorkspaceSourceKindV4::PastedText);
        assert_eq!(source.normalized_sha256, generic[0].source.sha256);
        assert_eq!(source.locator, "pasted-text");
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn private_source_consent_cross_application_isolation_stale_unlink_and_delete_are_enforced() {
        let root = root("source-boundaries");
        let mut workspace = Workspace::init_v4(&root).expect("Workspace v4");
        let (generic_id, academic_id) = mixed_applications(&mut workspace);
        let mut service =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let imported = service
            .import_source(
                NewWorkspaceSourceV4 {
                    kind: WorkspaceSourceKindV4::LocalFile,
                    locator: "fixtures/opportunity.txt".to_owned(),
                    final_locator: None,
                    redirect_chain: Vec::new(),
                    content_type: "text/plain".to_owned(),
                    original_bytes: b"First source revision".to_vec(),
                    normalized_text: "First source revision".to_owned(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("import Source");
        let reference = source_reference(&imported);
        let before = service
            .source_associations(&generic_id)
            .expect("initial links")
            .len();
        let denied = service
            .associate_source(&generic_id, &reference, None, ActorKind::User)
            .expect_err("private file association requires consent");
        assert!(matches!(
            denied,
            StoreError::ApplicationAssociationConsentRequired(_)
        ));
        assert_eq!(
            service
                .source_associations(&generic_id)
                .expect("links after denial")
                .len(),
            before
        );

        service
            .associate_source(
                &generic_id,
                &reference,
                Some(ConsentScope::ReadPrivateInputs),
                ActorKind::User,
            )
            .expect("consented association");
        assert!(
            service
                .source_associations(&academic_id)
                .expect("isolated academic links")
                .iter()
                .all(|association| association.source.id != imported.id)
        );

        service
            .revise_source(
                &imported.id,
                imported.revision,
                NewWorkspaceSourceV4 {
                    kind: WorkspaceSourceKindV4::LocalFile,
                    locator: "fixtures/opportunity.txt".to_owned(),
                    final_locator: None,
                    redirect_chain: Vec::new(),
                    content_type: "text/plain".to_owned(),
                    original_bytes: b"Second source revision".to_vec(),
                    normalized_text: "Second source revision".to_owned(),
                    privacy: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("revise Source");
        let generic_links = service
            .source_associations(&generic_id)
            .expect("generic links after revision");
        assert!(
            generic_links
                .iter()
                .any(|association| association.source.id == imported.id && association.stale)
        );
        assert!(
            service
                .source_associations(&academic_id)
                .expect("unrelated links after revision")
                .iter()
                .all(|association| !association.stale)
        );
        let delete_denied = service
            .delete_source(&imported.id, ActorKind::User)
            .expect_err("linked Source cannot be deleted");
        assert!(matches!(
            delete_denied,
            StoreError::ApplicationAssociationConflict(_)
        ));
        service
            .unlink_source(&generic_id, &imported.id, ActorKind::User)
            .expect("unlink Source");
        service
            .delete_source(&imported.id, ActorKind::User)
            .expect("delete unlinked Source");
        assert!(matches!(
            service.source(&imported.id, imported.revision),
            Err(StoreError::ApplicationAssociationNotFound(_))
        ));
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn profile_and_confirmed_evidence_require_typed_consent_bound_links() {
        let root = root("profile-evidence");
        let mut workspace = Workspace::init_v4(&root).expect("Workspace v4");
        let (generic_id, academic_id) = mixed_applications(&mut workspace);
        let profile = ProfileService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                NewProfileSource {
                    kind: canisend_contracts::ProfileSourceKind::PlainText,
                    original_bytes: b"Private profile facts".to_vec(),
                    normalized_text: "Private profile facts".to_owned(),
                    content_type: "text/plain".to_owned(),
                    sensitivity: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("Profile Source");
        let profile_reference = ContentRevisionReferenceV3 {
            id: profile.id.clone(),
            revision: profile.revision,
            sha256: profile.original.sha256.clone(),
        };

        let evidence_id = generate_id().expect("Evidence ID");
        let evidence_sha = Sha256Digest::try_new(hex::encode(Sha256::digest(b"evidence-v1")))
            .expect("Evidence digest");
        let created_at = now_utc().expect("timestamp");
        workspace
            .database
            .connection()
            .execute(
                "INSERT INTO evidence_items(id, kind, created_at)
                 VALUES (?1, 'employment', ?2)",
                params![evidence_id.as_str(), created_at.as_str()],
            )
            .expect("Evidence item");
        workspace
            .database
            .connection()
            .execute(
                "INSERT INTO evidence_revisions(
                    evidence_id, revision, sha256, confirmed, created_at, excluded, sensitivity
                 ) VALUES (?1, 1, ?2, 1, ?3, 0, 'private-local')",
                params![
                    evidence_id.as_str(),
                    evidence_sha.as_str(),
                    created_at.as_str()
                ],
            )
            .expect("Evidence revision");
        let evidence_reference = ContentRevisionReferenceV3 {
            id: evidence_id.clone(),
            revision: Revision::try_new(1).expect("revision"),
            sha256: evidence_sha,
        };

        let mut service =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        assert!(matches!(
            service.associate_profile_source(
                &generic_id,
                &profile_reference,
                None,
                ActorKind::User
            ),
            Err(StoreError::ApplicationAssociationConsentRequired(_))
        ));
        assert!(matches!(
            service.associate_evidence(&generic_id, &evidence_reference, None, ActorKind::User),
            Err(StoreError::ApplicationAssociationConsentRequired(_))
        ));
        service
            .associate_profile_source(
                &generic_id,
                &profile_reference,
                Some(ConsentScope::ReadPrivateInputs),
                ActorKind::User,
            )
            .expect("Profile association");
        service
            .associate_evidence(
                &generic_id,
                &evidence_reference,
                Some(ConsentScope::ReadPrivateInputs),
                ActorKind::User,
            )
            .expect("Evidence association");
        assert_eq!(
            service
                .profile_associations(&generic_id)
                .expect("Profile links")
                .len(),
            1
        );
        assert_eq!(
            service
                .evidence_associations(&generic_id)
                .expect("Evidence links")
                .len(),
            1
        );
        assert!(
            service
                .profile_associations(&academic_id)
                .expect("isolated Profile links")
                .is_empty()
        );
        assert!(
            service
                .evidence_associations(&academic_id)
                .expect("isolated Evidence links")
                .is_empty()
        );
        service
            .unlink_profile_source(&generic_id, &profile.id, ActorKind::User)
            .expect("unlink Profile");
        service
            .unlink_evidence(&generic_id, &evidence_id, ActorKind::User)
            .expect("unlink Evidence");
        assert!(
            service
                .profile_associations(&generic_id)
                .expect("Profile links after unlink")
                .is_empty()
        );
        assert!(
            service
                .evidence_associations(&generic_id)
                .expect("Evidence links after unlink")
                .is_empty()
        );
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn wrong_digest_and_stale_source_revision_fail_without_association_mutation() {
        let root = root("no-mutation");
        let mut workspace = Workspace::init_v4(&root).expect("Workspace v4");
        let (generic_id, _) = mixed_applications(&mut workspace);
        let mut service =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let imported = service
            .import_source(
                NewWorkspaceSourceV4 {
                    kind: WorkspaceSourceKindV4::Url,
                    locator: "https://example.invalid/opportunity".to_owned(),
                    final_locator: Some("https://example.invalid/opportunity".to_owned()),
                    redirect_chain: Vec::new(),
                    content_type: "text/plain".to_owned(),
                    original_bytes: b"Public opportunity".to_vec(),
                    normalized_text: "Public opportunity".to_owned(),
                    privacy: PrivacyClassification::Public,
                },
                ActorKind::User,
            )
            .expect("Source");
        let initial_count = service
            .source_associations(&generic_id)
            .expect("initial links")
            .len();
        let wrong_digest = ContentRevisionReferenceV3 {
            id: imported.id.clone(),
            revision: imported.revision,
            sha256: Sha256Digest::try_new("f".repeat(64)).expect("digest"),
        };
        assert!(matches!(
            service.associate_source(&generic_id, &wrong_digest, None, ActorKind::User),
            Err(StoreError::ApplicationAssociationConflict(_))
        ));
        let direct_wrong_digest = service.database.connection().execute(
            "INSERT INTO application_source_v4_associations(
                application_id, source_id, source_revision, source_sha256,
                consent_scope, associated_at
             ) VALUES (?1, ?2, 1, ?3, NULL, ?4)",
            params![
                generic_id.as_str(),
                imported.id.as_str(),
                wrong_digest.sha256.as_str(),
                now_utc().expect("timestamp").as_str()
            ],
        );
        assert!(
            direct_wrong_digest.is_err(),
            "SQLite must reject a link whose digest is not part of the exact Source revision key"
        );
        let stale = ContentRevisionReferenceV3 {
            id: imported.id,
            revision: Revision::try_new(2).expect("revision"),
            sha256: imported.normalized_sha256,
        };
        assert!(matches!(
            service.associate_source(&generic_id, &stale, None, ActorKind::User),
            Err(StoreError::ApplicationAssociationNotFound(_))
        ));
        assert_eq!(
            service
                .source_associations(&generic_id)
                .expect("links after failures")
                .len(),
            initial_count
        );
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn association_contract_keeps_pack_identity_out_of_workspace_source_types() {
        let value = serde_json::to_value(WorkspaceSourceKindV4::PastedText).expect("serialize");
        assert_eq!(value, serde_json::json!("pasted-text"));
        let generic = WorkflowPackId::try_new("org.canisend.generic-application").expect("Pack");
        let academic = WorkflowPackId::try_new("org.canisend.academic-job").expect("Pack");
        assert_ne!(generic, academic);
    }
}
