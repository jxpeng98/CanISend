use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    ActorKind, ApplicationDecision, ApplicationPlanCandidate, ApplicationPlanRecord,
    ApplicationStrategyRecord, ArtifactKind, ArtifactReference, CandidateValidationError,
    ContractViolation, CriteriaSetRecord, CriterionImportance, CriterionRevisionReference,
    DocumentKind, DocumentPlanCandidateRecord, DocumentRequirement, EntityId,
    EvidenceMatchSetRecord, ExecutionMode, MatchStrength, PlanBlockerRecord, PlanBlockerSeverity,
    PlannedDocumentRecord, Revision, Sha256Digest, UtcTimestamp, validate_external_candidate,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::Value;

use crate::{BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, now_utc};

pub struct PlanService<'a> {
    database: &'a mut Database,
    blobs: &'a BlobStore,
}

impl<'a> PlanService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database, blobs: &'a BlobStore) -> Self {
        Self { database, blobs }
    }

    pub fn template(&self, job_id: &EntityId) -> Result<ApplicationPlanCandidate, StoreError> {
        let context = load_context(self.database.connection(), job_id)?;
        let matches = load_matches(self.blobs, &context.matches_artifact)?;
        let blockers = derive_blockers(self.database.connection(), self.blobs, &matches)?;
        if context.plan_status == "complete" {
            let output = context.plan_output.as_ref().ok_or_else(|| {
                StoreError::Invariant("complete plan stage has no output".to_owned())
            })?;
            let current = load_plan(self.blobs, output)?;
            return Ok(ApplicationPlanCandidate {
                job_id: current.job_id,
                matches_artifact: context.matches_artifact,
                decision: current.decision,
                strategy: current.strategy,
                documents: current
                    .documents
                    .into_iter()
                    .map(|document| DocumentPlanCandidateRecord {
                        kind: document.kind,
                        requirement: document.requirement,
                        rationale: document.rationale,
                        constraints: document.constraints,
                        executor: document.executor,
                    })
                    .collect(),
                blockers,
            });
        }
        let risks = matches
            .matches
            .iter()
            .flat_map(|item| item.prohibited_claims.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(100)
            .collect();
        Ok(ApplicationPlanCandidate {
            job_id: job_id.clone(),
            matches_artifact: context.matches_artifact,
            decision: ApplicationDecision::Hold,
            strategy: ApplicationStrategyRecord {
                positioning: "Review the validated matches and define an application strategy."
                    .to_owned(),
                priorities: vec!["Address essential criteria with confirmed evidence".to_owned()],
                risks,
            },
            documents: default_documents(),
            blockers,
        })
    }

    pub fn current(&self, job_id: &EntityId) -> Result<ApplicationPlanRecord, StoreError> {
        let context = load_context(self.database.connection(), job_id)?;
        if context.plan_status != "complete" {
            return Err(StoreError::WorkflowConflict(format!(
                "plan stage is {}, not complete",
                context.plan_status
            )));
        }
        let output = context
            .plan_output
            .ok_or_else(|| StoreError::Invariant("complete plan stage has no output".to_owned()))?;
        load_plan(self.blobs, &output)
    }

    pub fn confirm(
        &mut self,
        job_id: &EntityId,
        value: &Value,
    ) -> Result<ArtifactReference, StoreError> {
        let candidate: ApplicationPlanCandidate =
            validate_external_candidate(value).map_err(candidate_error)?;
        if candidate.job_id != *job_id {
            return Err(semantic_error(
                "plan.job_mismatch",
                "/job_id",
                "application plan job ID must match the command subject",
            ));
        }
        let initial = load_context(self.database.connection(), job_id)?;
        validate_candidate_context(&candidate, &initial)?;
        let matches = load_matches(self.blobs, &initial.matches_artifact)?;
        let blockers = derive_blockers(self.database.connection(), self.blobs, &matches)?;
        if candidate.blockers != blockers {
            return Err(semantic_error(
                "plan.blockers_changed",
                "/blockers",
                "derived blockers must exactly match the current criterion-to-evidence matches",
            ));
        }
        if !matches!(
            initial.plan_status.as_str(),
            "ready" | "awaiting-user" | "complete"
        ) {
            return Err(StoreError::WorkflowConflict(format!(
                "plan stage is {}, not ready for confirmation",
                initial.plan_status
            )));
        }

        let current_plan = initial
            .plan_output
            .as_ref()
            .map(|output| load_plan(self.blobs, output))
            .transpose()?;
        let plan_revision = current_plan.as_ref().map_or(Ok(1), |plan| {
            plan.revision
                .get()
                .checked_add(1)
                .ok_or_else(|| StoreError::Invariant("plan revision overflow".to_owned()))
        })?;
        let plan_id = current_plan
            .as_ref()
            .map_or_else(generate_id, |plan| Ok(plan.id.clone()))?;
        let existing_documents = current_plan
            .as_ref()
            .map(|plan| {
                plan.documents
                    .iter()
                    .map(|document| (document.kind, (document.id.clone(), document.revision)))
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        let documents = candidate
            .documents
            .iter()
            .map(|document| {
                let (id, revision) =
                    if let Some((id, revision)) = existing_documents.get(&document.kind) {
                        let next = revision.get().checked_add(1).ok_or_else(|| {
                            StoreError::Invariant("document plan revision overflow".to_owned())
                        })?;
                        (id.clone(), Revision::try_new(next)?)
                    } else {
                        (generate_id()?, Revision::try_new(1)?)
                    };
                Ok(PlannedDocumentRecord {
                    id,
                    kind: document.kind,
                    requirement: document.requirement,
                    rationale: document.rationale.clone(),
                    constraints: document.constraints.clone(),
                    executor: document.executor,
                    revision,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let plan = ApplicationPlanRecord {
            id: plan_id.clone(),
            job_id: job_id.clone(),
            matches_artifact: initial.matches_artifact.clone(),
            decision: candidate.decision,
            strategy: candidate.strategy,
            documents,
            blockers,
            decided_by: ActorKind::User,
            revision: Revision::try_new(plan_revision)?,
        };
        validate_external_candidate::<ApplicationPlanRecord>(&serde_json::to_value(&plan)?)
            .map_err(candidate_error)?;
        let bytes = canonical_json_bytes(&serde_json::to_value(&plan)?)?;
        let digest = self.blobs.put_bytes(&bytes)?;
        let size = self.blobs.verify(&digest, DEFAULT_MAX_BLOB_BYTES)?;
        let artifact_id = initial
            .plan_output
            .as_ref()
            .map_or_else(generate_id, |output| Ok(output.id.clone()))?;
        let committed_at = now_utc()?;
        let event_id = generate_id()?;

        let transaction = self.database.immediate_transaction()?;
        let current = load_context(&transaction, job_id)?;
        if current != initial {
            return Err(StoreError::WorkflowConflict(
                "matches or plan state changed while confirming the plan".to_owned(),
            ));
        }
        verify_artifact_revision(&transaction, &current.matches_artifact)?;
        let current_matches = load_matches(self.blobs, &current.matches_artifact)?;
        if derive_blockers(&transaction, self.blobs, &current_matches)? != plan.blockers {
            return Err(StoreError::WorkflowConflict(
                "derived blockers changed while confirming the plan".to_owned(),
            ));
        }
        if let Some(output) = &current.plan_output {
            let updated = transaction.execute(
                "UPDATE artifacts SET head_revision = ?2, stale = 0
                 WHERE id = ?1 AND head_revision = ?3 AND stale = 0",
                params![
                    artifact_id.as_str(),
                    to_i64(plan_revision)?,
                    to_i64(output.revision.get())?
                ],
            )?;
            if updated != 1 {
                return Err(StoreError::WorkflowConflict(
                    "plan artifact changed while committing its revision".to_owned(),
                ));
            }
        } else {
            transaction.execute(
                "INSERT INTO artifacts(id, kind, head_revision, stale, created_at)
                 VALUES (?1, 'application-plan', ?2, 0, ?3)",
                params![
                    artifact_id.as_str(),
                    to_i64(plan_revision)?,
                    committed_at.as_str()
                ],
            )?;
        }
        transaction.execute(
            "INSERT INTO artifact_revisions(
                artifact_id, revision, sha256, size, actor, reason, created_at
             ) VALUES (?1, ?2, ?3, ?4, 'user', 'confirm application decision and document plan', ?5)",
            params![
                artifact_id.as_str(),
                to_i64(plan_revision)?,
                digest.as_str(),
                to_i64(size)?,
                committed_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO blob_references(sha256, owner_type, owner_id, owner_revision, created_at)
             VALUES (?1, 'artifact', ?2, ?3, ?4)",
            params![
                digest.as_str(),
                artifact_id.as_str(),
                to_i64(plan_revision)?,
                committed_at.as_str()
            ],
        )?;
        transaction.execute(
            "INSERT INTO artifact_dependencies(
                artifact_id, revision, depends_on_artifact_id, depends_on_revision,
                depends_on_sha256
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                artifact_id.as_str(),
                to_i64(plan_revision)?,
                current.matches_artifact.id.as_str(),
                to_i64(current.matches_artifact.revision.get())?,
                current.matches_artifact.sha256.as_str()
            ],
        )?;
        let completed = transaction.execute(
            "UPDATE stage_executions
             SET status = 'complete', execution_mode = 'user-decision',
                 output_artifact_id = ?3, output_artifact_revision = ?4,
                 started_at = COALESCE(started_at, ?5), completed_at = ?5, updated_at = ?5
             WHERE workflow_run_id = ?1 AND id = ?2 AND stage = 'plan'
               AND status IN ('ready', 'awaiting-user', 'complete')",
            params![
                current.run_id.as_str(),
                current.plan_execution_id.as_str(),
                artifact_id.as_str(),
                to_i64(plan_revision)?,
                committed_at.as_str()
            ],
        )?;
        if completed != 1 {
            return Err(StoreError::WorkflowConflict(
                "plan stage changed while committing confirmation".to_owned(),
            ));
        }
        let blocking_count = plan
            .blockers
            .iter()
            .filter(|blocker| blocker.severity == PlanBlockerSeverity::Blocking)
            .count();
        transaction.execute(
            "INSERT INTO application_plan_heads(
                workflow_run_id, artifact_id, artifact_revision, decision, blocking_count, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(workflow_run_id) DO UPDATE SET
                artifact_id = excluded.artifact_id,
                artifact_revision = excluded.artifact_revision,
                decision = excluded.decision,
                blocking_count = excluded.blocking_count,
                updated_at = excluded.updated_at",
            params![
                current.run_id.as_str(),
                artifact_id.as_str(),
                to_i64(plan_revision)?,
                enum_name(plan.decision)?,
                to_i64(u64::try_from(blocking_count).map_err(|_| {
                    StoreError::Invariant("blocking count overflow".to_owned())
                })?)?,
                committed_at.as_str()
            ],
        )?;
        invalidate_downstream(
            &transaction,
            &current.run_id,
            plan.decision == ApplicationDecision::Apply && blocking_count == 0,
            &committed_at,
        )?;
        transaction.execute(
            "DELETE FROM application_plan_documents WHERE workflow_run_id = ?1",
            params![current.run_id.as_str()],
        )?;
        for (position, document) in plan.documents.iter().enumerate() {
            transaction.execute(
                "INSERT INTO application_plan_documents(
                    workflow_run_id, plan_artifact_id, plan_artifact_revision,
                    planned_document_id, planned_document_revision, kind,
                    requirement, executor, position
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    current.run_id.as_str(),
                    artifact_id.as_str(),
                    to_i64(plan_revision)?,
                    document.id.as_str(),
                    to_i64(document.revision.get())?,
                    enum_name(document.kind)?,
                    enum_name(document.requirement)?,
                    document.executor.map(enum_name).transpose()?,
                    to_i64(u64::try_from(position).map_err(|_| {
                        StoreError::Invariant("document position overflow".to_owned())
                    })?)?
                ],
            )?;
        }
        insert_audit(
            &transaction,
            &event_id,
            &plan_id,
            plan.revision,
            &committed_at,
        )?;
        transaction.commit()?;
        Ok(ArtifactReference {
            kind: ArtifactKind::ApplicationPlan,
            id: artifact_id,
            revision: plan.revision,
            sha256: digest,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanContext {
    run_id: EntityId,
    plan_execution_id: EntityId,
    plan_status: String,
    matches_artifact: ArtifactReference,
    plan_output: Option<ArtifactReference>,
}

fn load_context(connection: &Connection, job_id: &EntityId) -> Result<PlanContext, StoreError> {
    type Row = (
        String,
        String,
        String,
        String,
        i64,
        Option<String>,
        Option<i64>,
    );
    let row: Row = connection
        .query_row(
            "SELECT run.id, plan.id, plan.status,
                    matching.output_artifact_id, matching.output_artifact_revision,
                    plan.output_artifact_id, plan.output_artifact_revision
             FROM workflow_runs AS run
             JOIN jobs ON jobs.id = run.job_id
             JOIN stage_executions AS matching
               ON matching.workflow_run_id = run.id AND matching.stage = 'match'
             JOIN stage_executions AS plan
               ON plan.workflow_run_id = run.id AND plan.stage = 'plan'
             WHERE run.job_id = ?1 AND run.job_revision = jobs.revision
               AND matching.status = 'complete'
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
        .ok_or_else(|| {
            StoreError::WorkflowConflict("match stage has no current completed output".to_owned())
        })?;
    let (run_id, plan_id, plan_status, match_id, match_revision, output_id, output_revision) = row;
    let matches_artifact = load_artifact_reference(
        connection,
        &EntityId::try_new(match_id)?,
        Revision::try_new(to_u64(match_revision)?)?,
        ArtifactKind::EvidenceMatches,
    )?;
    let plan_output = match (output_id, output_revision) {
        (Some(id), Some(revision)) => Some(load_artifact_reference(
            connection,
            &EntityId::try_new(id)?,
            Revision::try_new(to_u64(revision)?)?,
            ArtifactKind::ApplicationPlan,
        )?),
        (None, None) => None,
        _ => {
            return Err(StoreError::Invariant(
                "plan output ID/revision nullability differs".to_owned(),
            ));
        }
    };
    Ok(PlanContext {
        run_id: EntityId::try_new(run_id)?,
        plan_execution_id: EntityId::try_new(plan_id)?,
        plan_status,
        matches_artifact,
        plan_output,
    })
}

fn validate_candidate_context(
    candidate: &ApplicationPlanCandidate,
    context: &PlanContext,
) -> Result<(), StoreError> {
    if candidate.matches_artifact != context.matches_artifact {
        return Err(semantic_error(
            "plan.matches_changed",
            "/matches_artifact",
            "application plan must reference the exact current match artifact revision",
        ));
    }
    Ok(())
}

fn derive_blockers(
    connection: &Connection,
    blobs: &BlobStore,
    matches: &EvidenceMatchSetRecord,
) -> Result<Vec<PlanBlockerRecord>, StoreError> {
    let criteria = load_criteria(
        blobs,
        &load_artifact_reference(
            connection,
            &matches.criteria_artifact.id,
            matches.criteria_artifact.revision,
            ArtifactKind::Criteria,
        )?,
    )?;
    let criteria_by_id = criteria
        .criteria
        .iter()
        .map(|criterion| (criterion.id.clone(), criterion))
        .collect::<BTreeMap<_, _>>();
    let mut blockers = Vec::new();
    for evidence_match in &matches.matches {
        if evidence_match.strength == MatchStrength::Strong {
            continue;
        }
        let criterion = criteria_by_id
            .get(&evidence_match.criterion.id)
            .ok_or_else(|| {
                StoreError::Invariant("match references a missing criterion".to_owned())
            })?;
        let (severity, importance) = match criterion.importance {
            CriterionImportance::Essential => (PlanBlockerSeverity::Blocking, "essential"),
            CriterionImportance::Desirable => (PlanBlockerSeverity::Warning, "desirable"),
            CriterionImportance::Informational => continue,
        };
        let strength = match evidence_match.strength {
            MatchStrength::Strong => unreachable!("strong matches were skipped"),
            MatchStrength::Partial => "partial",
            MatchStrength::Gap => "gap",
            MatchStrength::Unknown => "unknown",
        };
        blockers.push(PlanBlockerRecord {
            code: format!("plan.{importance}_{strength}"),
            criterion: CriterionRevisionReference {
                id: criterion.id.clone(),
                revision: criterion.revision,
            },
            severity,
            description: format!(
                "The {importance} criterion does not have strong confirmed evidence ({strength})"
            ),
        });
    }
    blockers.sort_by(|left, right| {
        (&left.criterion.id, &left.code).cmp(&(&right.criterion.id, &right.code))
    });
    Ok(blockers)
}

fn default_documents() -> Vec<DocumentPlanCandidateRecord> {
    [
        (DocumentKind::CoverLetter, DocumentRequirement::Required),
        (DocumentKind::Cv, DocumentRequirement::Required),
        (
            DocumentKind::ResearchStatement,
            DocumentRequirement::Optional,
        ),
        (
            DocumentKind::TeachingStatement,
            DocumentRequirement::Optional,
        ),
    ]
    .into_iter()
    .map(|(kind, requirement)| DocumentPlanCandidateRecord {
        kind,
        requirement,
        rationale: match kind {
            DocumentKind::CoverLetter => "Position the application against the validated criteria",
            DocumentKind::Cv => "Present the confirmed evidence in a concise academic record",
            DocumentKind::ResearchStatement => "Prepare if the application requests research plans",
            DocumentKind::TeachingStatement => "Prepare if the application requests teaching plans",
        }
        .to_owned(),
        constraints: vec!["Use only confirmed evidence and respect prohibited claims".to_owned()],
        executor: Some(ExecutionMode::HostAgent),
    })
    .collect()
}

fn invalidate_downstream(
    transaction: &Transaction<'_>,
    run_id: &EntityId,
    draft_allowed: bool,
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
               AND stage IN ('draft', 'review', 'package', 'render')
               AND output_artifact_id IS NOT NULL
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE tasks SET status = 'stale' WHERE status = 'prepared'
         AND stage_execution_id IN (
             SELECT id FROM stage_executions WHERE workflow_run_id = ?1
               AND stage IN ('draft', 'review', 'package', 'render')
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "UPDATE stage_executions
         SET status = ?3, execution_mode = NULL, output_artifact_id = NULL,
             output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
             updated_at = ?2
         WHERE workflow_run_id = ?1 AND stage = 'draft'",
        params![
            run_id.as_str(),
            updated_at.as_str(),
            if draft_allowed { "ready" } else { "blocked" }
        ],
    )?;
    transaction.execute(
        "UPDATE stage_executions
         SET status = CASE WHEN output_artifact_id IS NULL THEN 'blocked' ELSE 'stale' END,
             execution_mode = NULL, output_artifact_id = NULL,
             output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
             updated_at = ?2
         WHERE workflow_run_id = ?1 AND stage IN ('review', 'package', 'render')",
        params![run_id.as_str(), updated_at.as_str()],
    )?;
    transaction.execute(
        "UPDATE workflow_runs SET status = 'active' WHERE id = ?1",
        params![run_id.as_str()],
    )?;
    Ok(())
}

fn load_matches(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<EvidenceMatchSetRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
}

fn load_criteria(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<CriteriaSetRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
}

fn load_plan(
    blobs: &BlobStore,
    artifact: &ArtifactReference,
) -> Result<ApplicationPlanRecord, StoreError> {
    let bytes = blobs.read_verified(&artifact.sha256, DEFAULT_MAX_BLOB_BYTES)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_external_candidate(&value).map_err(candidate_error)
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
    let actual: Option<(String, i64, i64)> = connection
        .query_row(
            "SELECT revision.sha256, artifact.head_revision, artifact.stale
             FROM artifact_revisions AS revision
             JOIN artifacts AS artifact ON artifact.id = revision.artifact_id
             WHERE revision.artifact_id = ?1 AND revision.revision = ?2",
            params![artifact.id.as_str(), to_i64(artifact.revision.get())?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    if actual.as_ref().is_some_and(|(sha256, head, stale)| {
        sha256 == artifact.sha256.as_str()
            && *head == i64::try_from(artifact.revision.get()).unwrap_or(-1)
            && *stale == 0
    }) {
        Ok(())
    } else {
        Err(StoreError::DependencyConflict(artifact.id.to_string()))
    }
}

fn candidate_error(error: CandidateValidationError) -> StoreError {
    match error {
        CandidateValidationError::Structural(violations) => {
            StoreError::CandidateStructural(violations)
        }
        CandidateValidationError::Semantic(violations) => StoreError::CandidateSemantic(violations),
    }
}

fn semantic_error(code: &str, pointer: &str, message: &str) -> StoreError {
    StoreError::CandidateSemantic(vec![ContractViolation::new(code, pointer, message)])
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
    plan_id: &EntityId,
    revision: Revision,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, 'user', 'plan.confirm', ?2, ?3,
                   'confirm application decision and document plan', ?4)",
        params![
            event_id.as_str(),
            plan_id.as_str(),
            to_i64(revision.get())?,
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
