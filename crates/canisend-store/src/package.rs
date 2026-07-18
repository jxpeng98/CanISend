use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    ApplicationPlanRecord, ArtifactKind, ArtifactReference, CandidateValidationError, DocumentKind,
    DocumentRecord, DocumentRequirement, DocumentSetRecord, EntityId, EvidenceCatalogRecord,
    EvidenceMatchSetRecord, FindingAuthority, FindingStatus, FindingTarget, PackageManifestRecord,
    ReadinessReasonCode, ReadinessReasonRecord, ReadinessRecord, ReadinessState,
    ReviewFindingsRecord, Revision, Sha256Digest, validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::Value;

use crate::{BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, now_utc};

pub struct PackageService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> PackageService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn check(&mut self, job_id: &EntityId) -> Result<ArtifactReference, StoreError> {
        let context = load_context(self.database.connection(), job_id)?;
        if context.package_status == "complete" {
            let output = context.package_output.as_ref().ok_or_else(|| {
                StoreError::Invariant("complete package stage has no output".to_owned())
            })?;
            let manifest = load_record::<PackageManifestRecord>(self.blobs, output)?;
            if manifest_matches_context(&manifest, &context) {
                return Ok(output.clone());
            }
            return Err(StoreError::DependencyConflict(
                "completed package manifest does not freeze the current inputs".to_owned(),
            ));
        }
        if context.package_status != "ready" {
            return Err(StoreError::WorkflowConflict(format!(
                "package stage is {}, not ready",
                context.package_status
            )));
        }

        let plan = load_record::<ApplicationPlanRecord>(self.blobs, &context.plan_artifact)?;
        let matches = load_record::<EvidenceMatchSetRecord>(self.blobs, &context.match_artifact)?;
        let evidence =
            load_record::<EvidenceCatalogRecord>(self.blobs, &context.evidence_artifact)?;
        let document_set =
            load_record::<DocumentSetRecord>(self.blobs, &context.document_set_artifact)?;
        let review = load_record::<ReviewFindingsRecord>(self.blobs, &context.review_artifact)?;
        let documents = document_set
            .documents
            .iter()
            .map(|artifact| {
                Ok((
                    artifact.clone(),
                    load_record::<DocumentRecord>(self.blobs, artifact)?,
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let reasons = derive_reasons(
            self.database.connection(),
            &context,
            &plan,
            &matches,
            &evidence,
            &document_set,
            &documents,
            &review,
        )?;
        let has_blocker = reasons
            .iter()
            .any(|reason| reason.code != ReadinessReasonCode::PendingHumanFinding);
        let state = if has_blocker {
            ReadinessState::Blocked
        } else if reasons.is_empty() {
            ReadinessState::ReadyToExport
        } else {
            ReadinessState::NeedsReview
        };
        let checked_at = now_utc()?;
        let manifest_id = generate_id()?;
        let manifest = PackageManifestRecord {
            id: manifest_id.clone(),
            job_id: job_id.clone(),
            plan_artifact: context.plan_artifact.clone(),
            evidence_artifact: context.evidence_artifact.clone(),
            profile_revision: context.profile_revision,
            document_set_artifact: context.document_set_artifact.clone(),
            documents: document_set.documents.clone(),
            review_artifact: context.review_artifact.clone(),
            readiness: ReadinessRecord {
                job_id: job_id.clone(),
                state,
                reasons,
                checked_at: checked_at.clone(),
            },
            submission_performed: false,
            revision: Revision::try_new(1)?,
        };
        validate_external_candidate::<PackageManifestRecord>(&serde_json::to_value(&manifest)?)
            .map_err(candidate_error)?;
        let bytes = canonical_json_bytes(&serde_json::to_value(&manifest)?)?;
        let digest = self.blobs.put_bytes(&bytes)?;
        let size = self.blobs.verify(&digest, DEFAULT_MAX_BLOB_BYTES)?;
        let artifact_id = generate_id()?;
        let event_id = generate_id()?;

        let transaction = self.database.immediate_transaction()?;
        let current = load_context(&transaction, job_id)?;
        if current != context {
            return Err(StoreError::TaskStale(
                "package inputs changed while computing readiness".to_owned(),
            ));
        }
        verify_inputs(&transaction, &context)?;
        transaction.execute(
            "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
             VALUES (?1, 'package-manifest', 1, 0, ?2)",
            params![artifact_id.as_str(), checked_at.as_str()],
        )?;
        transaction.execute(
            "INSERT INTO artifact_revisions(
                artifact_id, revision, sha256, size, actor, reason, created_at
             ) VALUES (?1, 1, ?2, ?3, 'system', 'compute deterministic package readiness', ?4)",
            params![
                artifact_id.as_str(),
                digest.as_str(),
                to_i64(size)?,
                checked_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
             VALUES (?1, 'artifact', ?2, 1, ?3)",
            params![digest.as_str(), artifact_id.as_str(), checked_at.as_str()],
        )?;
        let mut dependencies = vec![
            context.plan_artifact.clone(),
            context.match_artifact.clone(),
            context.evidence_artifact.clone(),
            context.document_set_artifact.clone(),
            context.review_artifact.clone(),
        ];
        dependencies.extend(document_set.documents.iter().cloned());
        let mut seen = BTreeSet::new();
        for dependency in dependencies {
            if !seen.insert((dependency.id.clone(), dependency.revision)) {
                continue;
            }
            transaction.execute(
                "INSERT INTO artifact_dependencies(
                    artifact_id, revision, depends_on_artifact_id, depends_on_revision,
                    depends_on_sha256
                 ) VALUES (?1, 1, ?2, ?3, ?4)",
                params![
                    artifact_id.as_str(),
                    dependency.id.as_str(),
                    to_i64(dependency.revision.get())?,
                    dependency.sha256.as_str()
                ],
            )?;
        }
        let completed = transaction.execute(
            "UPDATE stage_executions
             SET status = 'complete', execution_mode = 'deterministic',
                 output_artifact_id = ?3, output_artifact_revision = 1,
                 started_at = COALESCE(started_at, ?4), completed_at = ?4, updated_at = ?4
             WHERE workflow_run_id = ?1 AND id = ?2 AND stage = 'package' AND status = 'ready'",
            params![
                context.run_id.as_str(),
                context.package_execution_id.as_str(),
                artifact_id.as_str(),
                checked_at.as_str()
            ],
        )?;
        if completed != 1 {
            return Err(StoreError::TaskStale(
                "package stage changed before readiness commit".to_owned(),
            ));
        }
        transaction.execute(
            "INSERT INTO package_heads(
                workflow_run_id, artifact_id, artifact_revision,
                plan_artifact_id, plan_artifact_revision,
                evidence_artifact_id, evidence_artifact_revision, profile_revision,
                document_set_artifact_id, document_set_artifact_revision,
                review_artifact_id, review_artifact_revision, readiness_state, checked_at
             ) VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(workflow_run_id) DO UPDATE SET
                artifact_id = excluded.artifact_id,
                artifact_revision = excluded.artifact_revision,
                plan_artifact_id = excluded.plan_artifact_id,
                plan_artifact_revision = excluded.plan_artifact_revision,
                evidence_artifact_id = excluded.evidence_artifact_id,
                evidence_artifact_revision = excluded.evidence_artifact_revision,
                profile_revision = excluded.profile_revision,
                document_set_artifact_id = excluded.document_set_artifact_id,
                document_set_artifact_revision = excluded.document_set_artifact_revision,
                review_artifact_id = excluded.review_artifact_id,
                review_artifact_revision = excluded.review_artifact_revision,
                readiness_state = excluded.readiness_state,
                checked_at = excluded.checked_at",
            params![
                context.run_id.as_str(),
                artifact_id.as_str(),
                context.plan_artifact.id.as_str(),
                to_i64(context.plan_artifact.revision.get())?,
                context.evidence_artifact.id.as_str(),
                to_i64(context.evidence_artifact.revision.get())?,
                to_i64(context.profile_revision.get())?,
                context.document_set_artifact.id.as_str(),
                to_i64(context.document_set_artifact.revision.get())?,
                context.review_artifact.id.as_str(),
                to_i64(context.review_artifact.revision.get())?,
                enum_name(state)?,
                checked_at.as_str()
            ],
        )?;
        invalidate_render(&transaction, &context.run_id, state, &checked_at)?;
        transaction.execute(
            "UPDATE workflow_runs SET status = 'active' WHERE id = ?1",
            params![context.run_id.as_str()],
        )?;
        transaction.execute(
            "INSERT INTO audit_events(
                id, actor, action, subject_id, subject_revision, reason, created_at
             ) VALUES (?1, 'system', 'package.readiness.check', ?2, 1,
                       'freeze exact package inputs and compute body-free readiness', ?3)",
            params![event_id.as_str(), manifest_id.as_str(), checked_at.as_str()],
        )?;
        transaction.commit()?;
        Ok(ArtifactReference {
            kind: ArtifactKind::PackageManifest,
            id: artifact_id,
            revision: Revision::try_new(1)?,
            sha256: digest,
        })
    }

    pub fn current(&self, job_id: &EntityId) -> Result<PackageManifestRecord, StoreError> {
        let context = load_context(self.database.connection(), job_id)?;
        if context.package_status != "complete" {
            return Err(StoreError::WorkflowConflict(format!(
                "package stage is {}, not complete",
                context.package_status
            )));
        }
        let output = context.package_output.as_ref().ok_or_else(|| {
            StoreError::Invariant("complete package stage has no output".to_owned())
        })?;
        let manifest = load_record::<PackageManifestRecord>(self.blobs, output)?;
        if !manifest_matches_context(&manifest, &context) {
            return Err(StoreError::DependencyConflict(
                "package manifest no longer freezes the current inputs".to_owned(),
            ));
        }
        Ok(manifest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageContext {
    run_id: EntityId,
    package_execution_id: EntityId,
    package_status: String,
    profile_revision: Revision,
    plan_artifact: ArtifactReference,
    match_artifact: ArtifactReference,
    evidence_artifact: ArtifactReference,
    document_set_artifact: ArtifactReference,
    review_artifact: ArtifactReference,
    package_output: Option<ArtifactReference>,
}

fn load_context(connection: &Connection, job_id: &EntityId) -> Result<PackageContext, StoreError> {
    type Row = (
        String,
        String,
        String,
        i64,
        String,
        i64,
        String,
        i64,
        String,
        i64,
        String,
        i64,
        String,
        i64,
        Option<String>,
        Option<i64>,
    );
    let row: Row = connection
        .query_row(
            "SELECT run.id, package.id, package.status, metadata.profile_revision,
                    plan.output_artifact_id, plan.output_artifact_revision,
                    matching.output_artifact_id, matching.output_artifact_revision,
                    evidence.output_artifact_id, evidence.output_artifact_revision,
                    draft.output_artifact_id, draft.output_artifact_revision,
                    review.output_artifact_id, review.output_artifact_revision,
                    package.output_artifact_id, package.output_artifact_revision
             FROM workflow_runs AS run
             JOIN jobs ON jobs.id = run.job_id AND jobs.revision = run.job_revision
             JOIN workspace_metadata AS metadata ON metadata.singleton = 1
             JOIN stage_executions AS plan
               ON plan.workflow_run_id = run.id AND plan.stage = 'plan' AND plan.status = 'complete'
             JOIN stage_executions AS matching
               ON matching.workflow_run_id = run.id AND matching.stage = 'match'
              AND matching.status = 'complete'
             JOIN stage_executions AS evidence
               ON evidence.workflow_run_id = run.id AND evidence.stage = 'evidence'
              AND evidence.status = 'complete'
             JOIN stage_executions AS draft
               ON draft.workflow_run_id = run.id AND draft.stage = 'draft'
              AND draft.status = 'complete'
             JOIN stage_executions AS review
               ON review.workflow_run_id = run.id AND review.stage = 'review'
              AND review.status = 'complete'
             JOIN stage_executions AS package
               ON package.workflow_run_id = run.id AND package.stage = 'package'
             WHERE run.job_id = ?1
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
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| {
            StoreError::WorkflowConflict(
                "review and all package inputs must be complete and current".to_owned(),
            )
        })?;
    let (
        run_id,
        package_execution_id,
        package_status,
        profile_revision,
        plan_id,
        plan_revision,
        match_id,
        match_revision,
        evidence_id,
        evidence_revision,
        document_set_id,
        document_set_revision,
        review_id,
        review_revision,
        output_id,
        output_revision,
    ) = row;
    if profile_revision <= 0 {
        return Err(StoreError::WorkflowConflict(
            "a current profile revision is required for package readiness".to_owned(),
        ));
    }
    Ok(PackageContext {
        run_id: EntityId::try_new(run_id)?,
        package_execution_id: EntityId::try_new(package_execution_id)?,
        package_status,
        profile_revision: Revision::try_new(to_u64(profile_revision)?)?,
        plan_artifact: load_reference(
            connection,
            &plan_id,
            plan_revision,
            ArtifactKind::ApplicationPlan,
        )?,
        match_artifact: load_reference(
            connection,
            &match_id,
            match_revision,
            ArtifactKind::EvidenceMatches,
        )?,
        evidence_artifact: load_reference(
            connection,
            &evidence_id,
            evidence_revision,
            ArtifactKind::EvidenceCatalog,
        )?,
        document_set_artifact: load_reference(
            connection,
            &document_set_id,
            document_set_revision,
            ArtifactKind::DocumentSet,
        )?,
        review_artifact: load_reference(
            connection,
            &review_id,
            review_revision,
            ArtifactKind::ReviewFindings,
        )?,
        package_output: match (output_id, output_revision) {
            (Some(id), Some(revision)) => Some(load_reference(
                connection,
                &id,
                revision,
                ArtifactKind::PackageManifest,
            )?),
            (None, None) => None,
            _ => {
                return Err(StoreError::Invariant(
                    "package output ID/revision nullability differs".to_owned(),
                ));
            }
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn derive_reasons(
    connection: &Connection,
    context: &PackageContext,
    plan: &ApplicationPlanRecord,
    matches: &EvidenceMatchSetRecord,
    evidence: &EvidenceCatalogRecord,
    document_set: &DocumentSetRecord,
    documents: &[(ArtifactReference, DocumentRecord)],
    review: &ReviewFindingsRecord,
) -> Result<Vec<ReadinessReasonRecord>, StoreError> {
    let mut reasons = Vec::new();
    if document_set.plan_artifact != context.plan_artifact {
        reasons.push(reason(
            ReadinessReasonCode::MixedPlanRevision,
            None,
            Some(document_set.plan_artifact.clone()),
            None,
        ));
    }
    if matches.evidence_artifact != context.evidence_artifact {
        reasons.push(reason(
            ReadinessReasonCode::MixedEvidenceRevision,
            None,
            Some(matches.evidence_artifact.clone()),
            None,
        ));
    }
    if evidence.profile_revision != context.profile_revision {
        reasons.push(reason(
            ReadinessReasonCode::MixedProfileRevision,
            None,
            None,
            None,
        ));
    }

    let planned = plan
        .documents
        .iter()
        .map(|document| (document.kind, document))
        .collect::<BTreeMap<_, _>>();
    let actual = documents
        .iter()
        .map(|(artifact, document)| (document.kind, (artifact, document)))
        .collect::<BTreeMap<_, _>>();
    for document in &plan.documents {
        if document.requirement == DocumentRequirement::Required
            && !actual.contains_key(&document.kind)
        {
            reasons.push(reason(
                ReadinessReasonCode::MissingRequiredDocument,
                Some(document.kind),
                None,
                None,
            ));
        }
    }
    for (kind, (artifact, document)) in &actual {
        let Some(planned_document) = planned.get(kind) else {
            reasons.push(reason(
                ReadinessReasonCode::DocumentPlanMismatch,
                Some(*kind),
                Some((*artifact).clone()),
                None,
            ));
            continue;
        };
        if planned_document.requirement == DocumentRequirement::Omitted
            || document.plan_artifact != context.plan_artifact
            || document.planned_document.id != planned_document.id
            || document.planned_document.revision != planned_document.revision
        {
            reasons.push(reason(
                ReadinessReasonCode::DocumentPlanMismatch,
                Some(*kind),
                Some((*artifact).clone()),
                None,
            ));
        }
        if !artifact_is_current(connection, artifact)? {
            reasons.push(reason(
                ReadinessReasonCode::StaleDocument,
                Some(*kind),
                Some((*artifact).clone()),
                None,
            ));
        }
    }

    let document_references = document_set.documents.iter().collect::<Vec<_>>();
    let review_scope_matches = review.document_set_artifact == context.document_set_artifact
        && review.findings.iter().all(|finding| {
            std::iter::once(&finding.target)
                .chain(finding.related_targets.iter())
                .all(|target| {
                    target_is_in_set(target, &context.document_set_artifact, &document_references)
                })
        });
    if !review_scope_matches {
        reasons.push(reason(
            ReadinessReasonCode::ReviewDocumentSetMismatch,
            None,
            Some(context.review_artifact.clone()),
            None,
        ));
    }
    for finding in &review.findings {
        if finding.status != FindingStatus::Open {
            continue;
        }
        if finding.authority == FindingAuthority::Deterministic
            && finding.severity == canisend_contracts::FindingSeverity::Blocker
        {
            reasons.push(reason(
                ReadinessReasonCode::OpenDeterministicFinding,
                None,
                None,
                Some(finding.id.clone()),
            ));
        } else if finding.authority == FindingAuthority::HumanReview {
            reasons.push(reason(
                ReadinessReasonCode::PendingHumanFinding,
                None,
                None,
                Some(finding.id.clone()),
            ));
        }
    }
    Ok(reasons)
}

fn reason(
    code: ReadinessReasonCode,
    document_kind: Option<DocumentKind>,
    artifact: Option<ArtifactReference>,
    finding_id: Option<EntityId>,
) -> ReadinessReasonRecord {
    ReadinessReasonRecord {
        code,
        document_kind,
        artifact,
        finding_id,
    }
}

fn target_is_in_set(
    target: &FindingTarget,
    document_set: &ArtifactReference,
    documents: &[&ArtifactReference],
) -> bool {
    match target {
        FindingTarget::DocumentSet {
            document_set: target,
        } => target == document_set,
        FindingTarget::Document { document, .. }
        | FindingTarget::Section { document, .. }
        | FindingTarget::Claim { document, .. }
        | FindingTarget::Placeholder { document, .. } => documents.contains(&document),
    }
}

fn artifact_is_current(
    connection: &Connection,
    artifact: &ArtifactReference,
) -> Result<bool, StoreError> {
    let actual: Option<(i64, i64, String)> = connection
        .query_row(
            "SELECT head_revision, stale, revision.sha256
             FROM artifacts
             JOIN artifact_revisions AS revision
               ON revision.artifact_id = artifacts.id AND revision.revision = ?2
             WHERE artifacts.id = ?1",
            params![artifact.id.as_str(), to_i64(artifact.revision.get())?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    Ok(actual.is_some_and(|(head, stale, sha256)| {
        head == i64::try_from(artifact.revision.get()).unwrap_or(-1)
            && stale == 0
            && sha256 == artifact.sha256.as_str()
    }))
}

fn manifest_matches_context(manifest: &PackageManifestRecord, context: &PackageContext) -> bool {
    manifest.plan_artifact == context.plan_artifact
        && manifest.evidence_artifact == context.evidence_artifact
        && manifest.profile_revision == context.profile_revision
        && manifest.document_set_artifact == context.document_set_artifact
        && manifest.review_artifact == context.review_artifact
        && !manifest.submission_performed
}

fn verify_inputs(
    transaction: &Transaction<'_>,
    context: &PackageContext,
) -> Result<(), StoreError> {
    for artifact in [
        &context.plan_artifact,
        &context.match_artifact,
        &context.evidence_artifact,
        &context.document_set_artifact,
        &context.review_artifact,
    ] {
        if !artifact_is_current(transaction, artifact)? {
            return Err(StoreError::TaskStale(format!(
                "package input {} is no longer current",
                artifact.id
            )));
        }
    }
    let current_profile: i64 = transaction.query_row(
        "SELECT profile_revision FROM workspace_metadata WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    if current_profile != to_i64(context.profile_revision.get())? {
        return Err(StoreError::TaskStale(
            "profile revision changed while computing readiness".to_owned(),
        ));
    }
    Ok(())
}

fn invalidate_render(
    transaction: &Transaction<'_>,
    run_id: &EntityId,
    state: ReadinessState,
    updated_at: &canisend_contracts::UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT output_artifact_id FROM stage_executions
             WHERE workflow_run_id = ?1 AND stage = 'render'
               AND output_artifact_id IS NOT NULL
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE stage_executions
         SET status = ?2, execution_mode = NULL, output_artifact_id = NULL,
             output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
             updated_at = ?3
         WHERE workflow_run_id = ?1 AND stage = 'render'",
        params![
            run_id.as_str(),
            if state == ReadinessState::ReadyToExport {
                "ready"
            } else {
                "blocked"
            },
            updated_at.as_str()
        ],
    )?;
    Ok(())
}

fn load_reference(
    connection: &Connection,
    id: &str,
    revision: i64,
    expected_kind: ArtifactKind,
) -> Result<ArtifactReference, StoreError> {
    let row: Option<(String, String, i64, i64)> = connection
        .query_row(
            "SELECT artifact.kind, revision.sha256, artifact.head_revision, artifact.stale
             FROM artifacts AS artifact
             JOIN artifact_revisions AS revision
               ON revision.artifact_id = artifact.id AND revision.revision = ?2
             WHERE artifact.id = ?1",
            params![id, revision],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let (kind, sha256, head_revision, stale) =
        row.ok_or_else(|| StoreError::ArtifactNotFound(id.to_owned()))?;
    let kind: ArtifactKind = serde_json::from_value(Value::String(kind))?;
    if kind != expected_kind || head_revision != revision || stale != 0 {
        return Err(StoreError::TaskStale(format!(
            "package input {id} is stale or has the wrong kind"
        )));
    }
    Ok(ArtifactReference {
        kind,
        id: EntityId::try_new(id.to_owned())?,
        revision: Revision::try_new(to_u64(revision)?)?,
        sha256: Sha256Digest::try_new(sha256)?,
    })
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
