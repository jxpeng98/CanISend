use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, CandidateValidationError, ContractViolation,
    EntityId, EvidenceCatalogRecord, EvidenceRecord, Revision, Sha256Digest, UtcTimestamp,
    validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::Value;

use crate::{
    BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, now_utc,
    task::EVIDENCE_NORMALIZE_OPERATION,
};

pub struct EvidenceService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> EvidenceService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn proposed(&self, job_id: &EntityId) -> Result<EvidenceCatalogRecord, StoreError> {
        let context = load_context(self.database.connection(), job_id)?;
        let proposal = context.proposal_output.ok_or_else(|| {
            StoreError::WorkflowConflict(
                "current profile revision has no evidence proposal to review".to_owned(),
            )
        })?;
        let catalog = load_catalog(self.blobs, &proposal)?;
        if catalog.items.iter().any(|item| item.confirmed) {
            return Err(StoreError::Invariant(
                "agent evidence proposal contains a confirmed item".to_owned(),
            ));
        }
        Ok(catalog)
    }

    pub fn template(&self, job_id: &EntityId) -> Result<EvidenceCatalogRecord, StoreError> {
        let context = load_context(self.database.connection(), job_id)?;
        let mut catalog = if context.evidence_status == "complete" {
            let output = context.stage_output.ok_or_else(|| {
                StoreError::Invariant("complete evidence stage has no output".to_owned())
            })?;
            load_catalog(self.blobs, &output)?
        } else {
            let proposal = context.proposal_output.ok_or_else(|| {
                StoreError::WorkflowConflict(
                    "prepare and complete evidence normalization before exporting a template"
                        .to_owned(),
                )
            })?;
            load_catalog(self.blobs, &proposal)?
        };
        for item in &mut catalog.items {
            item.confirmed = true;
        }
        Ok(catalog)
    }

    pub fn confirmed(&self, job_id: &EntityId) -> Result<EvidenceCatalogRecord, StoreError> {
        let context = load_context(self.database.connection(), job_id)?;
        if context.evidence_status != "complete" {
            return Err(StoreError::WorkflowConflict(format!(
                "evidence stage is {}, not complete",
                context.evidence_status
            )));
        }
        let output = context.stage_output.ok_or_else(|| {
            StoreError::Invariant("complete evidence stage has no output".to_owned())
        })?;
        let catalog = load_catalog(self.blobs, &output)?;
        ensure_confirmed(&catalog)?;
        Ok(catalog)
    }

    pub fn confirm(
        &mut self,
        job_id: &EntityId,
        value: &Value,
    ) -> Result<ArtifactReference, StoreError> {
        let candidate = validate_catalog(value)?;
        ensure_confirmed(&candidate)?;
        let initial = load_context(self.database.connection(), job_id)?;
        let base_reference = confirmation_base(&initial)?;
        let base = load_catalog(self.blobs, &base_reference)?;
        validate_confirmation_identity(&base, &candidate)?;
        validate_source_spans(
            self.database.connection(),
            self.blobs,
            &initial.allowed_sources,
            &candidate,
        )?;
        let mut final_catalog = candidate;
        let revising = initial.evidence_status == "complete";
        if revising {
            increment_revisions(&mut final_catalog, &base)?;
        }
        let bytes = canonical_json_bytes(&serde_json::to_value(&final_catalog)?)?;
        let digest = self.blobs.put_bytes(&bytes)?;
        let size = self.blobs.verify(&digest, DEFAULT_MAX_BLOB_BYTES)?;
        let artifact_id = if revising {
            base_reference.id.clone()
        } else {
            generate_id()?
        };
        let artifact_revision = final_catalog.revision;
        let event_id = generate_id()?;
        let committed_at = now_utc()?;

        let transaction = self.database.immediate_transaction()?;
        let current = load_context(&transaction, job_id)?;
        if current != initial {
            return Err(StoreError::WorkflowConflict(
                "evidence or profile state changed while confirming evidence".to_owned(),
            ));
        }
        let current_base_reference = confirmation_base(&current)?;
        if current_base_reference != base_reference {
            return Err(StoreError::WorkflowConflict(
                "evidence proposal changed while confirming evidence".to_owned(),
            ));
        }
        validate_source_spans(
            &transaction,
            self.blobs,
            &current.allowed_sources,
            &final_catalog,
        )?;
        verify_artifact_revision(&transaction, &current_base_reference)?;

        if revising {
            let updated = transaction.execute(
                "UPDATE artifacts SET head_revision = ?2, stale = 0
                 WHERE id = ?1 AND head_revision = ?3 AND stale = 0",
                params![
                    artifact_id.as_str(),
                    to_i64(artifact_revision.get())?,
                    to_i64(base.revision.get())?
                ],
            )?;
            if updated != 1 {
                return Err(StoreError::WorkflowConflict(
                    "evidence artifact changed while committing its revision".to_owned(),
                ));
            }
        } else {
            transaction.execute(
                "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
                 VALUES (?1, 'evidence-catalog', ?2, 0, ?3)",
                params![
                    artifact_id.as_str(),
                    to_i64(artifact_revision.get())?,
                    committed_at.as_str()
                ],
            )?;
        }
        transaction.execute(
            "INSERT INTO artifact_revisions(
                artifact_id, revision, sha256, size, actor, reason, created_at
             ) VALUES (?1, ?2, ?3, ?4, 'user', ?5, ?6)",
            params![
                artifact_id.as_str(),
                to_i64(artifact_revision.get())?,
                digest.as_str(),
                to_i64(size)?,
                if revising {
                    "revise confirmed profile evidence"
                } else {
                    "confirm corrected profile evidence"
                },
                committed_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
             VALUES (?1, 'artifact', ?2, ?3, ?4)",
            params![
                digest.as_str(),
                artifact_id.as_str(),
                to_i64(artifact_revision.get())?,
                committed_at.as_str()
            ],
        )?;
        insert_dependency(
            &transaction,
            &artifact_id,
            artifact_revision,
            &current_base_reference,
        )?;
        let mut dependency_ids = BTreeSet::new();
        dependency_ids.insert(current_base_reference.id.clone());
        for source in final_catalog
            .items
            .iter()
            .map(|item| &item.source_span.source)
        {
            if dependency_ids.insert(source.id.clone()) {
                verify_artifact_revision(&transaction, source)?;
                insert_dependency(&transaction, &artifact_id, artifact_revision, source)?;
            }
        }
        persist_evidence_items(
            &transaction,
            &final_catalog,
            &artifact_id,
            artifact_revision,
            revising,
            &committed_at,
        )?;
        let completed = transaction.execute(
            "UPDATE stage_executions
             SET status = 'complete', execution_mode = 'user-decision',
                 input_profile_revision = ?3, output_artifact_id = ?4,
                 output_artifact_revision = ?5, completed_at = ?6, updated_at = ?6
             WHERE id = ?1 AND workflow_run_id = ?2 AND status = ?7",
            params![
                current.evidence_execution_id.as_str(),
                current.run_id.as_str(),
                to_i64(final_catalog.profile_revision.get())?,
                artifact_id.as_str(),
                to_i64(artifact_revision.get())?,
                committed_at.as_str(),
                if revising {
                    "complete"
                } else {
                    "awaiting-user"
                }
            ],
        )?;
        if completed != 1 {
            return Err(StoreError::WorkflowConflict(
                "evidence stage changed while committing confirmation".to_owned(),
            ));
        }
        invalidate_downstream(&transaction, &current.run_id, &committed_at)?;
        insert_audit(
            &transaction,
            &event_id,
            if revising {
                "evidence.revise"
            } else {
                "evidence.confirm"
            },
            &artifact_id,
            artifact_revision,
            if revising {
                "validate corrections and revise confirmed profile evidence"
            } else {
                "validate source-bound corrections and confirm profile evidence"
            },
            &committed_at,
        )?;
        transaction.commit()?;
        Ok(ArtifactReference {
            kind: ArtifactKind::EvidenceCatalog,
            id: artifact_id,
            revision: artifact_revision,
            sha256: digest,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvidenceContext {
    run_id: EntityId,
    evidence_execution_id: EntityId,
    evidence_status: String,
    input_profile_revision: Option<i64>,
    current_profile_revision: i64,
    proposal_output: Option<ArtifactReference>,
    stage_output: Option<ArtifactReference>,
    allowed_sources: Vec<ArtifactReference>,
}

fn load_context(connection: &Connection, job_id: &EntityId) -> Result<EvidenceContext, StoreError> {
    type Row = (
        String,
        String,
        String,
        Option<i64>,
        i64,
        Option<String>,
        Option<i64>,
    );
    let row: Row = connection
        .query_row(
            "SELECT run.id, evidence.id, evidence.status, evidence.input_profile_revision,
                    metadata.profile_revision, evidence.output_artifact_id,
                    evidence.output_artifact_revision
             FROM workflow_runs AS run
             JOIN jobs ON jobs.id = run.job_id
             JOIN stage_executions AS evidence
               ON evidence.workflow_run_id = run.id AND evidence.stage = 'evidence'
             JOIN workspace_metadata AS metadata ON metadata.singleton = 1
             WHERE run.job_id = ?1 AND run.job_revision = jobs.revision
             ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
            params![job_id.as_str()],
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
        .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
    let (
        run_id,
        evidence_id,
        status,
        input_profile_revision,
        profile_revision,
        output_id,
        output_rev,
    ) = row;
    if profile_revision <= 0 {
        return Err(StoreError::WorkflowConflict(
            "import at least one profile source before using evidence".to_owned(),
        ));
    }
    if input_profile_revision.is_some_and(|revision| revision != profile_revision) {
        return Err(StoreError::WorkflowConflict(
            "profile changed; normalize evidence again".to_owned(),
        ));
    }
    let stage_output = output_id
        .zip(output_rev)
        .map(|(id, revision)| {
            load_artifact_reference(
                connection,
                &EntityId::try_new(id)?,
                Revision::try_new(to_u64(revision)?)?,
                ArtifactKind::EvidenceCatalog,
            )
        })
        .transpose()?;
    let proposal_output = load_proposal_output(connection, &evidence_id, profile_revision)?;
    let allowed_sources = load_profile_inputs(connection)?;
    if allowed_sources.is_empty() {
        return Err(StoreError::Invariant(
            "positive profile revision has no normalized source artifacts".to_owned(),
        ));
    }
    Ok(EvidenceContext {
        run_id: EntityId::try_new(run_id)?,
        evidence_execution_id: EntityId::try_new(evidence_id)?,
        evidence_status: status,
        input_profile_revision,
        current_profile_revision: profile_revision,
        proposal_output,
        stage_output,
        allowed_sources,
    })
}

fn load_proposal_output(
    connection: &Connection,
    stage_execution_id: &str,
    profile_revision: i64,
) -> Result<Option<ArtifactReference>, StoreError> {
    let output: Option<(String, i64)> = connection
        .query_row(
            "SELECT result.artifact_id, result.revision
             FROM tasks AS task
             JOIN task_results AS result ON result.task_id = task.id
             WHERE task.stage_execution_id = ?1 AND task.operation = ?2
               AND task.status = 'committed' AND task.profile_revision = ?3
             ORDER BY task.completed_at DESC, task.id DESC LIMIT 1",
            params![
                stage_execution_id,
                EVIDENCE_NORMALIZE_OPERATION,
                profile_revision
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    output
        .map(|(id, revision)| {
            load_artifact_reference(
                connection,
                &EntityId::try_new(id)?,
                Revision::try_new(to_u64(revision)?)?,
                ArtifactKind::EvidenceCatalog,
            )
        })
        .transpose()
}

fn load_profile_inputs(connection: &Connection) -> Result<Vec<ArtifactReference>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT revision.normalized_artifact_id, revision.normalized_artifact_revision,
                artifact.sha256
         FROM profile_sources AS source
         JOIN profile_source_revisions AS revision ON revision.source_id = source.id
         JOIN artifact_revisions AS artifact
           ON artifact.artifact_id = revision.normalized_artifact_id
          AND artifact.revision = revision.normalized_artifact_revision
         WHERE revision.revision = (
             SELECT MAX(head.revision) FROM profile_source_revisions AS head
             WHERE head.source_id = source.id
         )
         ORDER BY source.created_at, source.id",
    )?;
    statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .map(|row| {
            let (id, revision, sha256) = row?;
            Ok(ArtifactReference {
                kind: ArtifactKind::SourceNormalizedText,
                id: EntityId::try_new(id)?,
                revision: Revision::try_new(to_u64(revision)?)?,
                sha256: Sha256Digest::try_new(sha256)?,
            })
        })
        .collect()
}

fn confirmation_base(context: &EvidenceContext) -> Result<ArtifactReference, StoreError> {
    match context.evidence_status.as_str() {
        "awaiting-user" => context.proposal_output.clone().ok_or_else(|| {
            StoreError::Invariant("awaiting evidence stage has no proposal output".to_owned())
        }),
        "complete" => context.stage_output.clone().ok_or_else(|| {
            StoreError::Invariant("complete evidence stage has no confirmed output".to_owned())
        }),
        status => Err(StoreError::WorkflowConflict(format!(
            "evidence stage is {status}, not awaiting confirmation"
        ))),
    }
}

fn validate_catalog(value: &Value) -> Result<EvidenceCatalogRecord, StoreError> {
    validate_external_candidate(value).map_err(candidate_error)
}

fn load_catalog(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<EvidenceCatalogRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_catalog(&value)
}

fn candidate_error(error: CandidateValidationError) -> StoreError {
    match error {
        CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        CandidateValidationError::Semantic(violations) => StoreError::CandidateSemantic(violations),
    }
}

fn ensure_confirmed(catalog: &EvidenceCatalogRecord) -> Result<(), StoreError> {
    let violations = catalog
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.confirmed)
        .map(|(index, _)| {
            ContractViolation::new(
                "evidence.confirmation_required",
                format!("/items/{index}/confirmed"),
                "every evidence item must be explicitly confirmed",
            )
        })
        .collect::<Vec<_>>();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(StoreError::CandidateSemantic(violations))
    }
}

fn validate_confirmation_identity(
    base: &EvidenceCatalogRecord,
    candidate: &EvidenceCatalogRecord,
) -> Result<(), StoreError> {
    let mut violations = Vec::new();
    if candidate.id != base.id {
        violations.push(ContractViolation::new(
            "evidence.catalog_id_changed",
            "/id",
            "confirmation must preserve the core-generated catalog ID",
        ));
    }
    if candidate.profile_revision != base.profile_revision {
        violations.push(ContractViolation::new(
            "evidence.profile_revision_changed",
            "/profile_revision",
            "confirmation must preserve the normalized profile revision",
        ));
    }
    if candidate.revision != base.revision {
        violations.push(ContractViolation::new(
            "evidence.revision_changed",
            "/revision",
            "input must repeat the current catalog revision; core advances it",
        ));
    }
    let base_items = base
        .items
        .iter()
        .map(|item| (&item.id, item))
        .collect::<BTreeMap<_, _>>();
    let candidate_ids = candidate
        .items
        .iter()
        .map(|item| &item.id)
        .collect::<BTreeSet<_>>();
    if base_items.keys().copied().collect::<BTreeSet<_>>() != candidate_ids {
        violations.push(ContractViolation::new(
            "evidence.item_ids_changed",
            "/items",
            "confirmation cannot add, remove, or replace core-generated evidence IDs; use excluded instead",
        ));
    }
    for (index, item) in candidate.items.iter().enumerate() {
        if let Some(base_item) = base_items.get(&item.id)
            && item.revision != base_item.revision
        {
            violations.push(ContractViolation::new(
                "evidence.item_revision_changed",
                format!("/items/{index}/revision"),
                "input must repeat the current item revision; core advances it",
            ));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(StoreError::CandidateSemantic(violations))
    }
}

fn increment_revisions(
    catalog: &mut EvidenceCatalogRecord,
    base: &EvidenceCatalogRecord,
) -> Result<(), StoreError> {
    catalog.revision = next_revision(base.revision)?;
    let revisions = base
        .items
        .iter()
        .map(|item| (&item.id, item.revision))
        .collect::<BTreeMap<_, _>>();
    for item in &mut catalog.items {
        item.revision = next_revision(*revisions.get(&item.id).ok_or_else(|| {
            StoreError::Invariant("validated evidence ID disappeared".to_owned())
        })?)?;
    }
    Ok(())
}

fn next_revision(revision: Revision) -> Result<Revision, StoreError> {
    Revision::try_new(
        revision
            .get()
            .checked_add(1)
            .ok_or_else(|| StoreError::Invariant("revision overflow".to_owned()))?,
    )
    .map_err(StoreError::from)
}

fn validate_source_spans(
    connection: &Connection,
    blobs: &BlobStore,
    allowed_sources: &[ArtifactReference],
    catalog: &EvidenceCatalogRecord,
) -> Result<(), StoreError> {
    let allowed = allowed_sources
        .iter()
        .map(|artifact| (artifact.id.as_str(), artifact))
        .collect::<BTreeMap<_, _>>();
    let mut cached = BTreeMap::new();
    let mut violations = Vec::new();
    for (index, item) in catalog.items.iter().enumerate() {
        let source = &item.source_span.source;
        let Some(expected) = allowed.get(source.id.as_str()) else {
            violations.push(ContractViolation::new(
                "candidate.source_out_of_scope",
                format!("/items/{index}/source_span/source"),
                "evidence source is outside the current profile input scope",
            ));
            continue;
        };
        if *expected != source {
            violations.push(ContractViolation::new(
                "candidate.source_revision_mismatch",
                format!("/items/{index}/source_span/source"),
                "evidence source revision/hash does not match the current profile input",
            ));
            continue;
        }
        verify_artifact_revision(connection, source)?;
        let bytes = if let Some(bytes) = cached.get(source.id.as_str()) {
            bytes
        } else {
            let bytes = blobs.read_verified(&source.sha256, DEFAULT_MAX_BLOB_BYTES)?;
            cached.insert(source.id.as_str().to_owned(), bytes);
            cached
                .get(source.id.as_str())
                .expect("inserted source bytes are present")
        };
        let start = usize::try_from(item.source_span.start_byte).unwrap_or(usize::MAX);
        let end = usize::try_from(item.source_span.end_byte).unwrap_or(usize::MAX);
        if !bytes
            .get(start..end)
            .is_some_and(|span| span == item.source_quote.as_bytes())
        {
            violations.push(ContractViolation::new(
                "candidate.source_span_mismatch",
                format!("/items/{index}/source_span"),
                "source span must select bytes exactly equal to source_quote",
            ));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(StoreError::CandidateSemantic(violations))
    }
}

fn persist_evidence_items(
    transaction: &Transaction<'_>,
    catalog: &EvidenceCatalogRecord,
    artifact_id: &EntityId,
    artifact_revision: Revision,
    revising: bool,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    for item in &catalog.items {
        let kind = enum_name(item.kind)?;
        if revising {
            let updated = transaction.execute(
                "UPDATE evidence_items SET kind = ?2, source_artifact_id = ?3 WHERE id = ?1",
                params![item.id.as_str(), kind, item.source_span.source.id.as_str()],
            )?;
            if updated != 1 {
                return Err(StoreError::Invariant(format!(
                    "confirmed evidence item {} is missing",
                    item.id
                )));
            }
        } else {
            transaction.execute(
                "INSERT INTO evidence_items(id, kind, created_at, source_artifact_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    item.id.as_str(),
                    kind,
                    created_at.as_str(),
                    item.source_span.source.id.as_str()
                ],
            )?;
        }
        let item_digest = item_digest(item)?;
        transaction.execute(
            "INSERT INTO evidence_revisions(
                evidence_id, revision, sha256, confirmed, created_at,
                artifact_id, artifact_revision, excluded, sensitivity
             ) VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6, ?7, ?8)",
            params![
                item.id.as_str(),
                to_i64(item.revision.get())?,
                item_digest.as_str(),
                created_at.as_str(),
                artifact_id.as_str(),
                to_i64(artifact_revision.get())?,
                i64::from(item.excluded),
                enum_name(item.sensitivity)?
            ],
        )?;
    }
    Ok(())
}

fn item_digest(item: &EvidenceRecord) -> Result<Sha256Digest, StoreError> {
    use sha2::{Digest, Sha256};
    let bytes = canonical_json_bytes(&serde_json::to_value(item)?)?;
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes))).map_err(StoreError::from)
}

fn invalidate_downstream(
    transaction: &Transaction<'_>,
    run_id: &EntityId,
    updated_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "DELETE FROM package_heads WHERE workflow_run_id = ?1",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT artifact_id FROM review_heads WHERE workflow_run_id = ?1
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "DELETE FROM review_heads WHERE workflow_run_id = ?1",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT artifact_id FROM document_heads WHERE workflow_run_id = ?1
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "DELETE FROM document_heads WHERE workflow_run_id = ?1",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT output_artifact_id FROM stage_executions
             WHERE workflow_run_id = ?1
               AND stage IN ('match', 'plan', 'draft', 'review', 'package', 'render')
               AND output_artifact_id IS NOT NULL
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE tasks SET status = 'stale' WHERE status = 'prepared'
         AND stage_execution_id IN (
             SELECT id FROM stage_executions WHERE workflow_run_id = ?1
               AND stage IN ('match', 'plan', 'draft', 'review', 'package', 'render')
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE stage_executions
         SET status = CASE WHEN EXISTS (
                 SELECT 1 FROM stage_executions AS criteria
                 WHERE criteria.workflow_run_id = ?1 AND criteria.stage = 'criteria'
                   AND criteria.status = 'complete'
             ) THEN 'ready' ELSE 'blocked' END,
             execution_mode = NULL, output_artifact_id = NULL,
             output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
             updated_at = ?2
         WHERE workflow_run_id = ?1 AND stage = 'match'",
        params![run_id.as_str(), updated_at.as_str()],
    )?;
    transaction.execute(
        "UPDATE stage_executions
         SET status = CASE WHEN status = 'blocked' THEN 'blocked' ELSE 'stale' END,
             execution_mode = NULL, output_artifact_id = NULL,
             output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
             updated_at = ?2
         WHERE workflow_run_id = ?1
           AND stage IN ('plan', 'draft', 'review', 'package', 'render')",
        params![run_id.as_str(), updated_at.as_str()],
    )?;
    transaction.execute(
        "UPDATE workflow_runs SET status = 'active' WHERE id = ?1",
        params![run_id.as_str()],
    )?;
    Ok(())
}

fn insert_dependency(
    transaction: &Transaction<'_>,
    artifact_id: &EntityId,
    artifact_revision: Revision,
    dependency: &ArtifactReference,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO artifact_dependencies(
            artifact_id, revision, depends_on_artifact_id, depends_on_revision,
            depends_on_sha256
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            artifact_id.as_str(),
            to_i64(artifact_revision.get())?,
            dependency.id.as_str(),
            to_i64(dependency.revision.get())?,
            dependency.sha256.as_str()
        ],
    )?;
    Ok(())
}

fn load_artifact_reference(
    connection: &Connection,
    id: &EntityId,
    revision: Revision,
    expected_kind: ArtifactKind,
) -> Result<ArtifactReference, StoreError> {
    let row: Option<(String, String, i64)> = connection
        .query_row(
            "SELECT artifact.kind, revision.sha256, artifact.stale
             FROM artifacts AS artifact
             JOIN artifact_revisions AS revision
               ON revision.artifact_id = artifact.id AND revision.revision = ?2
             WHERE artifact.id = ?1 AND artifact.head_revision = ?2",
            params![id.as_str(), to_i64(revision.get())?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (kind, sha256, stale) = row.ok_or_else(|| StoreError::ArtifactNotFound(id.to_string()))?;
    let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
    if kind != expected_kind || stale != 0 {
        return Err(StoreError::WorkflowConflict(format!(
            "artifact {id} is stale or has the wrong kind"
        )));
    }
    Ok(ArtifactReference {
        kind,
        id: id.clone(),
        revision,
        sha256: Sha256Digest::try_new(sha256)?,
    })
}

fn verify_artifact_revision(
    connection: &Connection,
    artifact: &ArtifactReference,
) -> Result<(), StoreError> {
    let actual: Option<String> = connection
        .query_row(
            "SELECT sha256 FROM artifact_revisions WHERE artifact_id = ?1 AND revision = ?2",
            params![artifact.id.as_str(), to_i64(artifact.revision.get())?],
            |row| row.get(0),
        )
        .optional()?;
    if actual.as_deref() == Some(artifact.sha256.as_str()) {
        Ok(())
    } else {
        Err(StoreError::DependencyConflict(artifact.id.to_string()))
    }
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, StoreError> {
    fn canonicalize(value: &Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, value)| (key.clone(), canonicalize(value)))
                    .collect::<BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
            other => other.clone(),
        }
    }
    let mut bytes = serde_json::to_vec(&canonicalize(value))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn insert_audit(
    transaction: &Transaction<'_>,
    event_id: &EntityId,
    action: &str,
    subject_id: &EntityId,
    subject_revision: Revision,
    reason: &str,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event_id.as_str(),
            enum_name(ActorKind::User)?,
            action,
            subject_id.as_str(),
            to_i64(subject_revision.get())?,
            reason,
            created_at.as_str()
        ],
    )?;
    Ok(())
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
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}
