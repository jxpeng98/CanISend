use std::collections::BTreeMap;

use canisend_contracts::{
    ActorKind, ArtifactReference, EntityId, ExecutionMode, NextAction, Revision, Sha256Digest,
    StageExecutionStatus, UtcTimestamp, WorkflowBlocker, WorkflowRunStatus, WorkflowStage,
    WorkflowStageState, WorkflowStatusData,
};
use canisend_core::{StageGraph, StageRegistry};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::Value;

use crate::{Database, StoreError, generate_id, now_utc};

pub struct WorkflowService<'a> {
    database: &'a mut Database,
    graph: StageGraph,
}

impl<'a> WorkflowService<'a> {
    #[must_use]
    pub fn new(database: &'a mut Database) -> Self {
        Self {
            database,
            graph: StageGraph::built_in(),
        }
    }

    pub fn start(&mut self, job_id: &EntityId) -> Result<WorkflowStatusData, StoreError> {
        if latest_run_id(self.database.connection(), job_id)?.is_some() {
            return self.status(job_id);
        }
        let run_id = generate_id()?;
        let event_id = generate_id()?;
        let created_at = now_utc()?;
        let transaction = self.database.immediate_transaction()?;
        let (archived, job_revision, source_count): (i64, i64, i64) = transaction
            .query_row(
                "SELECT archived, revision,
                        (SELECT COUNT(*) FROM sources WHERE job_id = jobs.id)
                 FROM jobs WHERE id = ?1",
                params![job_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::JobNotFound(job_id.to_string()))?;
        if archived != 0 {
            return Err(StoreError::JobArchived(job_id.to_string()));
        }
        transaction.execute(
            "INSERT INTO workflow_runs(id, job_id, status, created_at, job_revision)
             VALUES (?1, ?2, 'active', ?3, ?4)",
            params![
                run_id.as_str(),
                job_id.as_str(),
                created_at.as_str(),
                job_revision
            ],
        )?;
        let mut statuses = BTreeMap::new();
        for descriptor in self.graph.descriptors() {
            let status = if descriptor.stage == WorkflowStage::Intake {
                if source_count > 0 {
                    StageExecutionStatus::Complete
                } else {
                    StageExecutionStatus::Blocked
                }
            } else if descriptor.depends_on.is_empty()
                || descriptor.depends_on.iter().all(|dependency| {
                    statuses.get(dependency) == Some(&StageExecutionStatus::Complete)
                })
            {
                StageExecutionStatus::Ready
            } else {
                StageExecutionStatus::Blocked
            };
            let stage_execution_id = generate_id()?;
            transaction.execute(
                "INSERT INTO stage_executions(
                    id, workflow_run_id, stage, status, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![
                    stage_execution_id.as_str(),
                    run_id.as_str(),
                    descriptor.stage.as_str(),
                    enum_name(status)?,
                    created_at.as_str()
                ],
            )?;
            statuses.insert(descriptor.stage, status);
        }
        insert_audit(
            &transaction,
            &event_id,
            "workflow.start",
            &run_id,
            "start workflow from authoritative job state",
            &created_at,
        )?;
        transaction.commit()?;
        load_status(self.database.connection(), &self.graph, &run_id, job_id)
    }

    pub fn status(&mut self, job_id: &EntityId) -> Result<WorkflowStatusData, StoreError> {
        let run_id = latest_run_id(self.database.connection(), job_id)?
            .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
        let updated_at = now_utc()?;
        let transaction = self.database.immediate_transaction()?;
        reconcile_job_revision(&transaction, &self.graph, &run_id, job_id, &updated_at)?;
        refresh_ready_states(&transaction, &self.graph, &run_id, &updated_at)?;
        transaction.commit()?;
        load_status(self.database.connection(), &self.graph, &run_id, job_id)
    }

    pub fn begin_stage(
        &mut self,
        job_id: &EntityId,
        stage: WorkflowStage,
        mode: ExecutionMode,
        actor: ActorKind,
    ) -> Result<WorkflowStatusData, StoreError> {
        if !self.graph.supports_mode(stage, mode) {
            return Err(StoreError::WorkflowConflict(format!(
                "stage {} does not support {}",
                stage.as_str(),
                enum_name(mode)?
            )));
        }
        let run_id = latest_run_id(self.database.connection(), job_id)?
            .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
        let event_id = generate_id()?;
        let updated_at = now_utc()?;
        let transaction = self.database.immediate_transaction()?;
        reconcile_job_revision(&transaction, &self.graph, &run_id, job_id, &updated_at)?;
        refresh_ready_states(&transaction, &self.graph, &run_id, &updated_at)?;
        let current = load_stage_status(&transaction, &run_id, stage)?;
        if current != StageExecutionStatus::Ready {
            return Err(StoreError::WorkflowConflict(format!(
                "stage {} is {:?}, not ready",
                stage.as_str(),
                current
            )));
        }
        let next_status = if mode == ExecutionMode::UserDecision {
            StageExecutionStatus::AwaitingUser
        } else {
            StageExecutionStatus::Running
        };
        transaction.execute(
            "UPDATE stage_executions
             SET status = ?3, execution_mode = ?4, started_at = ?5, updated_at = ?5
             WHERE workflow_run_id = ?1 AND stage = ?2",
            params![
                run_id.as_str(),
                stage.as_str(),
                enum_name(next_status)?,
                enum_name(mode)?,
                updated_at.as_str()
            ],
        )?;
        insert_audit_with_actor(
            &transaction,
            &event_id,
            actor,
            "workflow.stage.begin",
            &run_id,
            &format!("begin {} using {}", stage.as_str(), enum_name(mode)?),
            &updated_at,
        )?;
        transaction.commit()?;
        load_status(self.database.connection(), &self.graph, &run_id, job_id)
    }

    pub fn complete_stage(
        &mut self,
        job_id: &EntityId,
        stage: WorkflowStage,
        output: &ArtifactReference,
        actor: ActorKind,
    ) -> Result<WorkflowStatusData, StoreError> {
        if stage == WorkflowStage::Plan {
            return Err(StoreError::WorkflowConflict(
                "plan must be completed through plan confirm".to_owned(),
            ));
        }
        let descriptor = self.graph.descriptor(stage);
        if output.kind != descriptor.output_kind {
            return Err(StoreError::WorkflowConflict(format!(
                "stage {} requires {:?}, found {:?}",
                stage.as_str(),
                descriptor.output_kind,
                output.kind
            )));
        }
        let run_id = latest_run_id(self.database.connection(), job_id)?
            .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
        let event_id = generate_id()?;
        let updated_at = now_utc()?;
        let transaction = self.database.immediate_transaction()?;
        reconcile_job_revision(&transaction, &self.graph, &run_id, job_id, &updated_at)?;
        let current = load_stage_status(&transaction, &run_id, stage)?;
        if !matches!(
            current,
            StageExecutionStatus::Running | StageExecutionStatus::AwaitingUser
        ) {
            return Err(StoreError::WorkflowConflict(format!(
                "stage {} cannot complete from {:?}",
                stage.as_str(),
                current
            )));
        }
        verify_current_artifact(&transaction, output)?;
        transaction.execute(
            "UPDATE stage_executions
             SET status = 'complete', output_artifact_id = ?3,
                 output_artifact_revision = ?4, completed_at = ?5, updated_at = ?5
             WHERE workflow_run_id = ?1 AND stage = ?2",
            params![
                run_id.as_str(),
                stage.as_str(),
                output.id.as_str(),
                to_i64(output.revision.get())?,
                updated_at.as_str()
            ],
        )?;
        refresh_ready_states(&transaction, &self.graph, &run_id, &updated_at)?;
        let complete_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM stage_executions
             WHERE workflow_run_id = ?1 AND status = 'complete'",
            params![run_id.as_str()],
            |row| row.get(0),
        )?;
        if usize::try_from(complete_count).ok() == Some(self.graph.topological_order().len()) {
            transaction.execute(
                "UPDATE workflow_runs SET status = 'complete' WHERE id = ?1",
                params![run_id.as_str()],
            )?;
        }
        insert_audit_with_actor(
            &transaction,
            &event_id,
            actor,
            "workflow.stage.complete",
            &run_id,
            &format!("complete {} with validated artifact", stage.as_str()),
            &updated_at,
        )?;
        transaction.commit()?;
        load_status(self.database.connection(), &self.graph, &run_id, job_id)
    }

    pub fn rerun(
        &mut self,
        job_id: &EntityId,
        stage: WorkflowStage,
        actor: ActorKind,
    ) -> Result<WorkflowStatusData, StoreError> {
        if stage == WorkflowStage::Intake {
            return Err(StoreError::WorkflowConflict(
                "intake changes must use job import rather than workflow rerun".to_owned(),
            ));
        }
        let run_id = latest_run_id(self.database.connection(), job_id)?
            .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
        let event_id = generate_id()?;
        let updated_at = now_utc()?;
        let transaction = self.database.immediate_transaction()?;
        reconcile_job_revision(&transaction, &self.graph, &run_id, job_id, &updated_at)?;
        let statuses = load_stage_statuses(&transaction, &run_id)?;
        let descriptor = self.graph.descriptor(stage);
        let target_status = if descriptor
            .depends_on
            .iter()
            .all(|dependency| statuses.get(dependency) == Some(&StageExecutionStatus::Complete))
            && stage_gate_allows(&transaction, &run_id, stage)?
        {
            StageExecutionStatus::Ready
        } else {
            StageExecutionStatus::Blocked
        };
        let affected = std::iter::once(stage)
            .chain(self.graph.descendants(stage))
            .collect::<Vec<_>>();
        if affected.contains(&WorkflowStage::Package) {
            transaction.execute(
                "UPDATE artifacts SET stale = 1 WHERE id IN (
                     SELECT artifact_id FROM export_heads WHERE workflow_run_id = ?1
                 )",
                params![run_id.as_str()],
            )?;
            transaction.execute(
                "DELETE FROM export_heads WHERE workflow_run_id = ?1",
                params![run_id.as_str()],
            )?;
            transaction.execute(
                "DELETE FROM package_heads WHERE workflow_run_id = ?1",
                params![run_id.as_str()],
            )?;
        }
        if affected.contains(&WorkflowStage::Review) {
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
        }
        if affected.contains(&WorkflowStage::Draft) {
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
        }
        for affected_stage in &affected {
            let next_status = if *affected_stage == stage {
                target_status
            } else {
                StageExecutionStatus::Stale
            };
            let output_id: Option<String> = transaction
                .query_row(
                    "SELECT output_artifact_id FROM stage_executions
                     WHERE workflow_run_id = ?1 AND stage = ?2",
                    params![run_id.as_str(), affected_stage.as_str()],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            if let Some(output_id) = output_id {
                transaction.execute(
                    "UPDATE artifacts SET stale = 1 WHERE id = ?1",
                    params![output_id],
                )?;
            }
            transaction.execute(
                "UPDATE stage_executions
                 SET status = ?3, execution_mode = NULL, output_artifact_id = NULL,
                     output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
                     updated_at = ?4
                 WHERE workflow_run_id = ?1 AND stage = ?2",
                params![
                    run_id.as_str(),
                    affected_stage.as_str(),
                    enum_name(next_status)?,
                    updated_at.as_str()
                ],
            )?;
            transaction.execute(
                "UPDATE tasks SET status = 'stale'
                 WHERE stage_execution_id = (
                   SELECT id FROM stage_executions
                   WHERE workflow_run_id = ?1 AND stage = ?2
                 ) AND status = 'prepared'",
                params![run_id.as_str(), affected_stage.as_str()],
            )?;
        }
        transaction.execute(
            "UPDATE workflow_runs SET status = 'active' WHERE id = ?1",
            params![run_id.as_str()],
        )?;
        insert_audit_with_actor(
            &transaction,
            &event_id,
            actor,
            "workflow.stage.rerun",
            &run_id,
            &format!("rerun {} and invalidate descendants", stage.as_str()),
            &updated_at,
        )?;
        transaction.commit()?;
        load_status(self.database.connection(), &self.graph, &run_id, job_id)
    }
}

fn latest_run_id(
    connection: &Connection,
    job_id: &EntityId,
) -> Result<Option<EntityId>, StoreError> {
    connection
        .query_row(
            "SELECT id FROM workflow_runs WHERE job_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 1",
            params![job_id.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(EntityId::try_new)
        .transpose()
        .map_err(StoreError::from)
}

fn load_status(
    connection: &Connection,
    graph: &StageGraph,
    run_id: &EntityId,
    job_id: &EntityId,
) -> Result<WorkflowStatusData, StoreError> {
    let run_status: String = connection
        .query_row(
            "SELECT status FROM workflow_runs WHERE id = ?1 AND job_id = ?2",
            params![run_id.as_str(), job_id.as_str()],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
    let mut stages = Vec::with_capacity(graph.topological_order().len());
    for stage in graph.topological_order() {
        type StageRow = (String, Option<String>, Option<String>, Option<i64>, String);
        let row: StageRow = connection.query_row(
            "SELECT status, execution_mode, output_artifact_id, output_artifact_revision,
                    COALESCE(updated_at, created_at)
             FROM stage_executions WHERE workflow_run_id = ?1 AND stage = ?2",
            params![run_id.as_str(), stage.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
        let (status, mode, output_id, output_revision, updated_at) = row;
        let output = match (output_id, output_revision) {
            (Some(id), Some(revision)) => Some(load_artifact_reference(
                connection,
                &EntityId::try_new(id)?,
                Revision::try_new(to_u64(revision)?)?,
            )?),
            (None, None) => None,
            _ => {
                return Err(StoreError::Invariant(
                    "workflow output ID/revision nullability differs".to_owned(),
                ));
            }
        };
        stages.push(WorkflowStageState {
            stage: *stage,
            status: parse_stage_status(&status)?,
            execution_mode: mode
                .map(|value| serde_json::from_value(Value::String(value)))
                .transpose()?,
            output,
            updated_at: UtcTimestamp::try_new(updated_at)?,
        });
    }
    let mut blockers = Vec::new();
    let mut next_actions = Vec::new();
    let statuses = stages
        .iter()
        .map(|state| (state.stage, state.status))
        .collect::<BTreeMap<_, _>>();
    for state in &stages {
        match state.status {
            StageExecutionStatus::Blocked => {
                let missing = graph
                    .descriptor(state.stage)
                    .depends_on
                    .iter()
                    .filter(|dependency| {
                        statuses.get(dependency) != Some(&StageExecutionStatus::Complete)
                    })
                    .map(|stage| stage.as_str())
                    .collect::<Vec<_>>();
                if state.stage == WorkflowStage::Draft && missing.is_empty() {
                    let (code, description) = draft_gate_blocker(connection, run_id)?;
                    blockers.push(WorkflowBlocker {
                        code,
                        stage: state.stage,
                        description,
                    });
                    continue;
                }
                if state.stage == WorkflowStage::Render && missing.is_empty() {
                    let (code, description) = render_gate_blocker(connection, run_id)?;
                    blockers.push(WorkflowBlocker {
                        code,
                        stage: state.stage,
                        description,
                    });
                    continue;
                }
                blockers.push(WorkflowBlocker {
                    code: if state.stage == WorkflowStage::Intake {
                        "workflow.source_required".to_owned()
                    } else {
                        "workflow.dependency_incomplete".to_owned()
                    },
                    stage: state.stage,
                    description: if state.stage == WorkflowStage::Intake {
                        "Import at least one job source".to_owned()
                    } else {
                        format!("Complete dependencies: {}", missing.join(", "))
                    },
                });
            }
            StageExecutionStatus::Stale => blockers.push(WorkflowBlocker {
                code: "workflow.stage_stale".to_owned(),
                stage: state.stage,
                description: "Upstream work changed; rerun this stage before continuing".to_owned(),
            }),
            StageExecutionStatus::AwaitingUser => blockers.push(WorkflowBlocker {
                code: "workflow.user_decision_required".to_owned(),
                stage: state.stage,
                description: "A user decision is required to complete this stage".to_owned(),
            }),
            StageExecutionStatus::Ready => {
                if !StageRegistry::is_available(state.stage) {
                    blockers.push(WorkflowBlocker {
                        code: "workflow.stage_planned".to_owned(),
                        stage: state.stage,
                        description: format!(
                            "The {} stage contract is not available in this build",
                            state.stage.as_str()
                        ),
                    });
                    continue;
                }
                let descriptor = graph.descriptor(state.stage);
                let mode = descriptor.execution_modes.first().copied().ok_or_else(|| {
                    StoreError::Invariant(format!(
                        "stage {} has no execution mode",
                        state.stage.as_str()
                    ))
                })?;
                let (action, description) = match state.stage {
                    WorkflowStage::Parse => (
                        format!(
                            "canisend task prepare --job {} --operation job-parse --mode host-agent --json",
                            job_id
                        ),
                        "Prepare the ready parse stage as a revision-bound task".to_owned(),
                    ),
                    WorkflowStage::Criteria => (
                        format!(
                            "canisend criteria export --job {} --destination criteria.json --json",
                            job_id
                        ),
                        "Export parsed criteria for explicit user review".to_owned(),
                    ),
                    WorkflowStage::Evidence => {
                        let profile_revision: i64 = connection.query_row(
                            "SELECT profile_revision FROM workspace_metadata WHERE singleton = 1",
                            [],
                            |row| row.get(0),
                        )?;
                        if profile_revision > 0 {
                            (
                                format!(
                                    "canisend task prepare --job {} --operation evidence-normalize --mode host-agent --json",
                                    job_id
                                ),
                                "Prepare evidence normalization from the current profile revision"
                                    .to_owned(),
                            )
                        } else {
                            (
                                "canisend profile source add --file PROFILE.md --json".to_owned(),
                                "Import a profile source before normalizing evidence".to_owned(),
                            )
                        }
                    }
                    WorkflowStage::Match => (
                        format!(
                            "canisend task prepare --job {} --operation evidence-match --mode host-agent --json",
                            job_id
                        ),
                        "Prepare revision-bound criterion-to-evidence matching".to_owned(),
                    ),
                    WorkflowStage::Plan => (
                        format!(
                            "canisend plan export --job {} --destination application-plan.json --json",
                            job_id
                        ),
                        "Export the derived blockers and choose an application decision".to_owned(),
                    ),
                    WorkflowStage::Draft => next_document_draft_action(connection, run_id, job_id)?,
                    WorkflowStage::Review => (
                        format!(
                            "canisend task prepare --job {} --operation document-review --mode host-agent --json",
                            job_id
                        ),
                        "Review the exact current document set for deterministic and human findings"
                            .to_owned(),
                    ),
                    WorkflowStage::Package => (
                        format!("canisend package check --job {} --json", job_id),
                        "Compute deterministic readiness from exact current package inputs"
                            .to_owned(),
                    ),
                    _ => (
                        format!(
                            "canisend workflow begin --job {} --stage {} --mode {} --json",
                            job_id,
                            state.stage.as_str(),
                            enum_name(mode)?
                        ),
                        format!("Begin the ready {} stage", state.stage.as_str()),
                    ),
                };
                next_actions.push(NextAction {
                    action,
                    description,
                });
            }
            StageExecutionStatus::Running | StageExecutionStatus::Complete => {}
        }
    }
    Ok(WorkflowStatusData {
        run_id: run_id.clone(),
        job_id: job_id.clone(),
        status: match run_status.as_str() {
            "active" => WorkflowRunStatus::Active,
            "complete" => WorkflowRunStatus::Complete,
            other => {
                return Err(StoreError::Invariant(format!(
                    "unknown workflow run status: {other}"
                )));
            }
        },
        stages,
        blockers,
        next_actions,
    })
}

fn next_document_draft_action(
    connection: &Connection,
    run_id: &EntityId,
    job_id: &EntityId,
) -> Result<(String, String), StoreError> {
    let missing: Option<(String, Option<String>)> = connection
        .query_row(
            "SELECT planned.kind, planned.executor
             FROM application_plan_documents AS planned
             JOIN application_plan_heads AS plan
               ON plan.workflow_run_id = planned.workflow_run_id
              AND plan.artifact_id = planned.plan_artifact_id
              AND plan.artifact_revision = planned.plan_artifact_revision
             LEFT JOIN document_heads AS document
               ON document.workflow_run_id = planned.workflow_run_id
              AND document.planned_document_id = planned.planned_document_id
              AND document.planned_document_revision = planned.planned_document_revision
             WHERE planned.workflow_run_id = ?1
               AND planned.requirement != 'omitted'
               AND document.artifact_id IS NULL
             ORDER BY planned.position ASC LIMIT 1",
            params![run_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((kind, executor)) = missing else {
        return Ok((
            format!(
                "canisend plan export --job {} --destination application-plan.json --json",
                job_id
            ),
            "Reconfirm this pre-R8 application plan before drafting structured documents"
                .to_owned(),
        ));
    };
    let executor = executor.ok_or_else(|| {
        StoreError::Invariant(format!(
            "non-omitted planned document {kind} has no executor"
        ))
    })?;
    Ok((
        format!(
            "canisend task prepare --job {} --operation {}-draft --mode {} --json",
            job_id, kind, executor
        ),
        format!("Prepare the next revision-bound {kind} structured document task"),
    ))
}

fn reconcile_job_revision(
    transaction: &Transaction<'_>,
    graph: &StageGraph,
    run_id: &EntityId,
    job_id: &EntityId,
    updated_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    let (prepared_revision, current_revision, source_count): (Option<i64>, i64, i64) = transaction
        .query_row(
            "SELECT wr.job_revision, j.revision,
                    (SELECT COUNT(*) FROM sources WHERE job_id = j.id)
             FROM workflow_runs AS wr
             JOIN jobs AS j ON j.id = wr.job_id
             WHERE wr.id = ?1 AND j.id = ?2",
            params![run_id.as_str(), job_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::WorkflowNotFound(job_id.to_string()))?;
    if prepared_revision == Some(current_revision) {
        return Ok(());
    }
    transaction.execute(
        "UPDATE artifacts SET stale = 1 WHERE id IN (
             SELECT artifact_id FROM export_heads WHERE workflow_run_id = ?1
         )",
        params![run_id.as_str()],
    )?;
    transaction.execute(
        "DELETE FROM export_heads WHERE workflow_run_id = ?1",
        params![run_id.as_str()],
    )?;
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
    let intake_status = if source_count > 0 {
        StageExecutionStatus::Complete
    } else {
        StageExecutionStatus::Blocked
    };
    transaction.execute(
        "UPDATE stage_executions SET status = ?3, updated_at = ?4
         WHERE workflow_run_id = ?1 AND stage = ?2",
        params![
            run_id.as_str(),
            WorkflowStage::Intake.as_str(),
            enum_name(intake_status)?,
            updated_at.as_str()
        ],
    )?;
    let statuses = load_stage_statuses(transaction, run_id)?;
    for affected_stage in graph.descendants(WorkflowStage::Intake) {
        let current = statuses.get(&affected_stage).copied().ok_or_else(|| {
            StoreError::Invariant(
                "workflow stage state is missing during reconciliation".to_owned(),
            )
        })?;
        let next = if affected_stage == WorkflowStage::Parse {
            if source_count > 0 {
                StageExecutionStatus::Ready
            } else {
                StageExecutionStatus::Blocked
            }
        } else if current == StageExecutionStatus::Blocked {
            StageExecutionStatus::Blocked
        } else {
            StageExecutionStatus::Stale
        };
        let output_id: Option<String> = transaction
            .query_row(
                "SELECT output_artifact_id FROM stage_executions
                 WHERE workflow_run_id = ?1 AND stage = ?2",
                params![run_id.as_str(), affected_stage.as_str()],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        if let Some(output_id) = output_id {
            transaction.execute(
                "UPDATE artifacts SET stale = 1 WHERE id = ?1",
                params![output_id],
            )?;
        }
        transaction.execute(
            "UPDATE stage_executions
             SET status = ?3, execution_mode = NULL, output_artifact_id = NULL,
                 output_artifact_revision = NULL, started_at = NULL, completed_at = NULL,
                 updated_at = ?4
             WHERE workflow_run_id = ?1 AND stage = ?2",
            params![
                run_id.as_str(),
                affected_stage.as_str(),
                enum_name(next)?,
                updated_at.as_str()
            ],
        )?;
        transaction.execute(
            "UPDATE tasks SET status = 'stale'
             WHERE stage_execution_id = (
               SELECT id FROM stage_executions
               WHERE workflow_run_id = ?1 AND stage = ?2
             ) AND status = 'prepared'",
            params![run_id.as_str(), affected_stage.as_str()],
        )?;
    }
    transaction.execute(
        "UPDATE workflow_runs SET job_revision = ?2, status = 'active' WHERE id = ?1",
        params![run_id.as_str(), current_revision],
    )?;
    let event_id = generate_id()?;
    insert_audit(
        transaction,
        &event_id,
        "workflow.invalidate",
        run_id,
        "job revision changed; invalidate only intake descendants",
        updated_at,
    )
}

fn refresh_ready_states(
    transaction: &Transaction<'_>,
    graph: &StageGraph,
    run_id: &EntityId,
    updated_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    let mut statuses = load_stage_statuses(transaction, run_id)?;
    for stage in graph.topological_order() {
        let current = statuses
            .get(stage)
            .copied()
            .ok_or_else(|| StoreError::Invariant("workflow stage state is missing".to_owned()))?;
        if matches!(
            current,
            StageExecutionStatus::Blocked | StageExecutionStatus::Stale
        ) && graph
            .descriptor(*stage)
            .depends_on
            .iter()
            .all(|dependency| statuses.get(dependency) == Some(&StageExecutionStatus::Complete))
            && stage_gate_allows(transaction, run_id, *stage)?
        {
            transaction.execute(
                "UPDATE stage_executions SET status = 'ready', updated_at = ?3
                 WHERE workflow_run_id = ?1 AND stage = ?2",
                params![run_id.as_str(), stage.as_str(), updated_at.as_str()],
            )?;
            statuses.insert(*stage, StageExecutionStatus::Ready);
        }
    }
    Ok(())
}

fn stage_gate_allows(
    connection: &Connection,
    run_id: &EntityId,
    stage: WorkflowStage,
) -> Result<bool, StoreError> {
    match stage {
        WorkflowStage::Draft => Ok(current_plan_gate(connection, run_id)?
            .is_some_and(|(decision, blockers)| decision == "apply" && blockers == 0)),
        WorkflowStage::Render => package_allows_render(connection, run_id),
        _ => Ok(true),
    }
}

fn package_allows_render(connection: &Connection, run_id: &EntityId) -> Result<bool, StoreError> {
    connection
        .query_row(
            "SELECT head.readiness_state
             FROM package_heads AS head
             JOIN stage_executions AS package
               ON package.workflow_run_id = head.workflow_run_id AND package.stage = 'package'
              AND package.status = 'complete'
              AND package.output_artifact_id = head.artifact_id
              AND package.output_artifact_revision = head.artifact_revision
             WHERE head.workflow_run_id = ?1",
            params![run_id.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map(|state| {
            state.is_some_and(|state| matches!(state.as_str(), "ready-to-export" | "exported"))
        })
        .map_err(StoreError::from)
}

fn render_gate_blocker(
    connection: &Connection,
    run_id: &EntityId,
) -> Result<(String, String), StoreError> {
    let state: Option<String> = connection
        .query_row(
            "SELECT head.readiness_state
             FROM package_heads AS head
             JOIN stage_executions AS package
               ON package.workflow_run_id = head.workflow_run_id AND package.stage = 'package'
              AND package.status = 'complete'
              AND package.output_artifact_id = head.artifact_id
              AND package.output_artifact_revision = head.artifact_revision
             WHERE head.workflow_run_id = ?1",
            params![run_id.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    Ok(match state.as_deref() {
        Some("blocked") => (
            "workflow.package_blocked".to_owned(),
            "Package readiness has deterministic blockers; inspect `canisend package show`"
                .to_owned(),
        ),
        Some("needs-review") => (
            "workflow.package_needs_review".to_owned(),
            "Package readiness requires explicit human finding dispositions".to_owned(),
        ),
        Some(other) => {
            return Err(StoreError::Invariant(format!(
                "render is blocked by unexpected package readiness {other}"
            )));
        }
        None => (
            "workflow.package_check_required".to_owned(),
            "Compute deterministic package readiness before rendering".to_owned(),
        ),
    })
}

fn current_plan_gate(
    connection: &Connection,
    run_id: &EntityId,
) -> Result<Option<(String, i64)>, StoreError> {
    connection
        .query_row(
            "SELECT head.decision, head.blocking_count
             FROM application_plan_heads AS head
             JOIN stage_executions AS plan
               ON plan.workflow_run_id = head.workflow_run_id AND plan.stage = 'plan'
              AND plan.status = 'complete'
              AND plan.output_artifact_id = head.artifact_id
              AND plan.output_artifact_revision = head.artifact_revision
             WHERE head.workflow_run_id = ?1",
            params![run_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(StoreError::from)
}

fn draft_gate_blocker(
    connection: &Connection,
    run_id: &EntityId,
) -> Result<(String, String), StoreError> {
    match current_plan_gate(connection, run_id)? {
        Some((decision, 0)) if decision != "apply" => Ok((
            "workflow.decision_not_apply".to_owned(),
            format!("Application decision is {decision}; revise the plan to continue"),
        )),
        Some((_, blockers)) if blockers > 0 => Ok((
            "workflow.plan_blocked".to_owned(),
            format!("Application plan has {blockers} unresolved essential evidence blocker(s)"),
        )),
        Some((decision, blockers)) => Err(StoreError::Invariant(format!(
            "invalid application plan gate state: {decision}/{blockers}"
        ))),
        None => Ok((
            "workflow.plan_decision_required".to_owned(),
            "Confirm a current application decision before drafting".to_owned(),
        )),
    }
}

fn load_stage_statuses(
    connection: &Connection,
    run_id: &EntityId,
) -> Result<BTreeMap<WorkflowStage, StageExecutionStatus>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT stage, status FROM stage_executions WHERE workflow_run_id = ?1 ORDER BY stage",
    )?;
    statement
        .query_map(params![run_id.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .map(|row| {
            let (stage, status) = row?;
            Ok((parse_stage(&stage)?, parse_stage_status(&status)?))
        })
        .collect()
}

fn load_stage_status(
    connection: &Connection,
    run_id: &EntityId,
    stage: WorkflowStage,
) -> Result<StageExecutionStatus, StoreError> {
    let status: String = connection
        .query_row(
            "SELECT status FROM stage_executions WHERE workflow_run_id = ?1 AND stage = ?2",
            params![run_id.as_str(), stage.as_str()],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| StoreError::Invariant(format!("missing {} stage", stage.as_str())))?;
    parse_stage_status(&status)
}

fn verify_current_artifact(
    connection: &Connection,
    artifact: &ArtifactReference,
) -> Result<(), StoreError> {
    let actual: Option<(String, i64, String)> = connection
        .query_row(
            "SELECT kind, head_revision, ar.sha256
             FROM artifacts AS a
             JOIN artifact_revisions AS ar
               ON ar.artifact_id = a.id AND ar.revision = a.head_revision
             WHERE a.id = ?1",
            params![artifact.id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let expected = (
        enum_name(artifact.kind)?,
        to_i64(artifact.revision.get())?,
        artifact.sha256.as_str().to_owned(),
    );
    if actual != Some(expected) {
        return Err(StoreError::DependencyConflict(artifact.id.to_string()));
    }
    Ok(())
}

fn load_artifact_reference(
    connection: &Connection,
    artifact_id: &EntityId,
    revision: Revision,
) -> Result<ArtifactReference, StoreError> {
    let (kind, sha256): (String, String) = connection
        .query_row(
            "SELECT a.kind, ar.sha256 FROM artifacts AS a
             JOIN artifact_revisions AS ar ON ar.artifact_id = a.id
             WHERE a.id = ?1 AND ar.revision = ?2",
            params![artifact_id.as_str(), to_i64(revision.get())?],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::ArtifactNotFound(artifact_id.to_string()))?;
    Ok(ArtifactReference {
        kind: serde_json::from_value(Value::String(kind))?,
        id: artifact_id.clone(),
        revision,
        sha256: Sha256Digest::try_new(sha256)?,
    })
}

fn insert_audit(
    transaction: &Transaction<'_>,
    event_id: &EntityId,
    action: &str,
    subject_id: &EntityId,
    reason: &str,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    insert_audit_with_actor(
        transaction,
        event_id,
        ActorKind::System,
        action,
        subject_id,
        reason,
        created_at,
    )
}

fn insert_audit_with_actor(
    transaction: &Transaction<'_>,
    event_id: &EntityId,
    actor: ActorKind,
    action: &str,
    subject_id: &EntityId,
    reason: &str,
    created_at: &UtcTimestamp,
) -> Result<(), StoreError> {
    transaction.execute(
        "INSERT INTO audit_events(
            id, actor, action, subject_id, subject_revision, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
        params![
            event_id.as_str(),
            enum_name(actor)?,
            action,
            subject_id.as_str(),
            reason,
            created_at.as_str()
        ],
    )?;
    Ok(())
}

fn parse_stage(value: &str) -> Result<WorkflowStage, StoreError> {
    match value {
        "intake" => Ok(WorkflowStage::Intake),
        "parse" => Ok(WorkflowStage::Parse),
        "criteria" => Ok(WorkflowStage::Criteria),
        "evidence" => Ok(WorkflowStage::Evidence),
        "match" => Ok(WorkflowStage::Match),
        "plan" => Ok(WorkflowStage::Plan),
        "draft" => Ok(WorkflowStage::Draft),
        "review" => Ok(WorkflowStage::Review),
        "package" => Ok(WorkflowStage::Package),
        "render" => Ok(WorkflowStage::Render),
        other => Err(StoreError::Invariant(format!(
            "unknown workflow stage: {other}"
        ))),
    }
}

fn parse_stage_status(value: &str) -> Result<StageExecutionStatus, StoreError> {
    match value {
        "blocked" => Ok(StageExecutionStatus::Blocked),
        "ready" => Ok(StageExecutionStatus::Ready),
        "running" => Ok(StageExecutionStatus::Running),
        "awaiting-user" => Ok(StageExecutionStatus::AwaitingUser),
        "complete" => Ok(StageExecutionStatus::Complete),
        "stale" => Ok(StageExecutionStatus::Stale),
        other => Err(StoreError::Invariant(format!(
            "unknown workflow stage status: {other}"
        ))),
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
    u64::try_from(value).map_err(|_| StoreError::Invariant("negative SQLite revision".to_owned()))
}
