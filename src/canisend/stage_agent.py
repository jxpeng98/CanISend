from __future__ import annotations

from pathlib import Path

from canisend.agent_protocol import (
    AgentResponse,
    ConsentRequirement,
    NextAction,
    WorkflowSnapshotReference,
    artifact_reference_from_path,
    error_response,
    success_response,
)
from canisend.stage_runtime import (
    AppliedStage,
    PreparedStage,
    StageRunOutcome,
    StageRuntimeError,
    StageStatusInspection,
)


def stage_status_agent_response(
    workspace: Path,
    job_dir: Path,
    inspection: StageStatusInspection,
) -> AgentResponse:
    artifacts = []
    state_path = job_dir / "workflow" / "state.json"
    if state_path.is_file():
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=state_path,
                kind="workflow_state",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            )
        )
    if inspection.pending_task_path is not None:
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=inspection.pending_task_path,
                kind="stage_task_spec",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            )
        )
    authoritative = job_dir / "parsed_job.json"
    if authoritative.is_file():
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=authoritative,
                kind="parsed_job",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            )
        )
    readiness = {
        "ready": "ready_for_next_stage",
        "running": "action_required",
        "succeeded": "ready_for_next_stage",
        "stale": "action_required",
        "failed": "blocked",
        "blocked": "blocked",
    }.get(inspection.stage.status, "unknown")
    actions: list[NextAction] = []
    if inspection.stage.status in {"ready", "stale", "failed"}:
        actions.append(NextAction(id="stage.prepare_parse", label="Prepare the Parse stage"))
    elif inspection.stage.status == "running":
        actions.append(
            NextAction(
                id="stage.complete_parse_task",
                label="Complete and apply the prepared Parse task",
                requires_consent=True,
                consent_ids=["read-full-job-advert"],
            )
        )
    return success_response(
        operation="workflow.stage_status",
        workflow=WorkflowSnapshotReference(phase="parse", readiness=readiness),  # type: ignore[arg-type]
        artifacts=artifacts,
        warnings=["The authoritative Parse output has local drift."] if inspection.output_drift else [],
        blockers=["Review the changed authoritative Parse output before rerunning."]
        if inspection.output_drift
        else [],
        next_actions=actions,
        extensions={
            "canisend.stage_id": inspection.stage.stage,
            "canisend.stage_status": inspection.stage.status,
            "canisend.input_fingerprint": inspection.input_fingerprint,
            "canisend.output_drift": inspection.output_drift,
            "canisend.state_reconstructed": inspection.reconstructed,
        },
    )


def stage_prepare_agent_response(
    workspace: Path,
    job_dir: Path,
    prepared: PreparedStage,
) -> AgentResponse:
    artifacts = [
        artifact_reference_from_path(
            workspace=workspace,
            path=prepared.task_spec_path,
            kind="stage_task_spec",
            privacy_tier=1,
            trust_level="validated",
            media_type="application/json",
            include_hash=True,
        ),
        artifact_reference_from_path(
            workspace=workspace,
            path=job_dir / "workflow" / "state.json",
            kind="workflow_state",
            privacy_tier=1,
            trust_level="validated",
            media_type="application/json",
            include_hash=True,
        ),
    ]
    consents = []
    if prepared.task_spec.execution_mode == "host_agent":
        consents.append(
            ConsentRequirement(
                id="read-full-job-advert",
                purpose="Allow the current host agent to read the reviewed job advert for Parse.",
                privacy_tier=2,
                artifact_kinds=["job_advert"],
            )
        )
    return success_response(
        operation="workflow.stage_prepare",
        workflow=WorkflowSnapshotReference(phase="parse", readiness="action_required"),
        artifacts=artifacts,
        required_consents=consents,
        next_actions=[
            NextAction(
                id="stage.complete_parse_task",
                label="Write the declared candidate and TaskResult, then apply them",
                requires_consent=bool(consents),
                consent_ids=["read-full-job-advert"] if consents else [],
            )
        ],
        extensions={
            "canisend.stage_id": prepared.task_spec.stage,
            "canisend.stage_status": "running",
            "canisend.run_id": prepared.task_spec.run_id,
            "canisend.task_id": prepared.task_spec.task_id,
            "canisend.input_fingerprint": prepared.task_spec.input_fingerprint,
            "canisend.reused": prepared.reused,
        },
    )


def stage_apply_agent_response(workspace: Path, applied: AppliedStage) -> AgentResponse:
    return success_response(
        operation="workflow.stage_apply",
        workflow=WorkflowSnapshotReference(phase="parse", readiness="ready_for_next_stage"),
        artifacts=[
            artifact_reference_from_path(
                workspace=workspace,
                path=applied.authoritative_path,
                kind="parsed_job",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            ),
            artifact_reference_from_path(
                workspace=workspace,
                path=applied.manifest_path,
                kind="stage_run_manifest",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            ),
        ],
        extensions={
            "canisend.stage_id": applied.manifest.stage,
            "canisend.stage_status": "succeeded",
            "canisend.run_id": applied.manifest.run_id,
            "canisend.cache_hit": False,
        },
    )


def stage_run_agent_response(workspace: Path, outcome: StageRunOutcome) -> AgentResponse:
    artifacts = [
        artifact_reference_from_path(
            workspace=workspace,
            path=outcome.authoritative_path,
            kind="parsed_job",
            privacy_tier=1,
            trust_level="validated",
            media_type="application/json",
            include_hash=True,
        )
    ]
    if outcome.manifest_path is not None:
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=outcome.manifest_path,
                kind="stage_run_manifest",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            )
        )
    return success_response(
        operation="workflow.stage_run",
        workflow=WorkflowSnapshotReference(phase="parse", readiness="ready_for_next_stage"),
        artifacts=artifacts,
        extensions={
            "canisend.stage_id": "parse",
            "canisend.stage_status": "succeeded",
            "canisend.cache_hit": outcome.cache_hit,
            "canisend.run_id": outcome.manifest.run_id if outcome.manifest is not None else None,
        },
    )


def stage_error_response(operation: str, error: StageRuntimeError) -> AgentResponse:
    return error_response(
        operation=operation,
        code=error.code,
        message=str(error),
        retryable=error.code in {"stage.stale_input", "stage.store_failed"},
    )
