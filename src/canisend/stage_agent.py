from __future__ import annotations

from pathlib import Path

from pydantic import ValidationError

from canisend.agent_protocol import (
    AgentResponse,
    ConsentRequirement,
    NextAction,
    WorkflowSnapshotReference,
    artifact_reference_from_path,
    error_response,
    success_response,
)
from canisend.decision_models import (
    CriteriaCatalogV1,
    CriterionMatchesV1,
    EvidenceCatalogV1,
)
from canisend.stage_adapters import get_stage_adapter
from canisend.stage_models import CandidateSubmissionV1, TaskSpecV1
from canisend.stage_registry import DEFAULT_STAGE_REGISTRY
from canisend.stage_runtime import (
    AppliedStage,
    CancelledStage,
    PreparedStage,
    SubmittedStage,
    StageRunOutcome,
    StageRuntimeError,
    StageStatusInspection,
    inspect_stage_status,
)
from canisend.stage_store import StageStoreError, read_json_object


def stage_status_agent_response(
    workspace: Path,
    job_dir: Path,
    inspection: StageStatusInspection,
) -> AgentResponse:
    stage_id = inspection.stage.stage
    adapter = get_stage_adapter(stage_id)
    artifacts = []
    pending_task = _pending_task_spec(inspection.pending_task_path)
    status_consents: list[ConsentRequirement] = (
        [
            ConsentRequirement(
                id=consent_id,
                purpose=(
                    f"Allow this host to read the declared private inputs and produce "
                    f"the {stage_id.title()} candidate."
                ),
                privacy_tier=pending_task.privacy_tier,
                artifact_kinds=["job_advert", "stage_input"],
            )
            for consent_id in pending_task.required_consents
        ]
        if pending_task is not None
        else []
    )
    state_path = job_dir / "workflow" / "state.json"
    if state_path.is_file():
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=state_path,
                kind="workflow_state",
                privacy_tier=1,
                trust_level="trusted_local" if inspection.reconstructed else "validated",
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
        preparation_path = inspection.pending_task_path.parent / "preparation.json"
        if preparation_path.is_file():
            artifacts.append(
                artifact_reference_from_path(
                    workspace=workspace,
                    path=preparation_path,
                    kind="stage_preparation_receipt",
                    privacy_tier=1,
                    trust_level="validated",
                    media_type="application/json",
                    include_hash=True,
                )
            )
        submission_path = inspection.pending_task_path.parent / "submission.json"
        if submission_path.is_file():
            artifacts.append(
                artifact_reference_from_path(
                    workspace=workspace,
                    path=submission_path,
                    kind="stage_candidate_submission",
                    privacy_tier=1,
                    trust_level="validated",
                    media_type="application/json",
                    include_hash=True,
                )
            )
            submission = _pending_submission(submission_path)
            if (
                pending_task is not None
                and submission is not None
                and submission.task_id == pending_task.task_id
                and submission.run_id == pending_task.run_id
                and submission.job_id == pending_task.job_id
                and submission.stage == pending_task.stage
                and submission.candidate.path == pending_task.candidate_output
                and submission.result_path == pending_task.result_output
            ):
                candidate_path = job_dir / pending_task.candidate_output
                result_path = job_dir / pending_task.result_output
                if candidate_path.is_file() and not candidate_path.is_symlink():
                    artifacts.append(
                        artifact_reference_from_path(
                            workspace=workspace,
                            path=candidate_path,
                            kind=f"{adapter.artifact_kind}_candidate",
                            privacy_tier=adapter.privacy_tier,
                            trust_level="generated_candidate",
                            media_type=adapter.media_type,
                            include_hash=True,
                        )
                    )
                if result_path.is_file() and not result_path.is_symlink():
                    artifacts.append(
                        artifact_reference_from_path(
                            workspace=workspace,
                            path=result_path,
                            kind="stage_task_result",
                            privacy_tier=1,
                            trust_level="validated",
                            media_type="application/json",
                            include_hash=True,
                        )
                    )
                status_consents = [
                    ConsentRequirement(
                        id=consent_id,
                        purpose=(
                            f"Allow this host to review the submitted {stage_id.title()} candidate "
                            "and its declared private inputs."
                        ),
                        privacy_tier=pending_task.privacy_tier,
                        artifact_kinds=[adapter.artifact_kind, "stage_input"],
                    )
                    for consent_id in pending_task.required_consents
                ]
    authoritative = job_dir / adapter.authoritative_target
    if authoritative.is_file():
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=authoritative,
                kind=adapter.artifact_kind,
                privacy_tier=adapter.privacy_tier,
                trust_level=(
                    "generated_candidate"
                    if "promotion_recovery" in inspection.reasons
                    else "trusted_local"
                    if inspection.output_drift
                    else "validated"
                ),
                media_type=adapter.media_type,
                include_hash=True,
            )
        )

    readiness = {
        "ready": "ready_for_next_stage",
        "running": "action_required",
        "succeeded": "ready_for_next_stage",
        "stale": "action_required",
        "failed": "blocked",
        "cancelled": "action_required",
        "blocked": "blocked",
    }.get(inspection.stage.status, "unknown")
    semantic_readiness = "ready_for_next_stage"
    semantic_extensions: dict[str, int | str | None] = {}
    semantic_actions: list[NextAction] = []
    if inspection.stage.status == "succeeded" and not inspection.output_drift:
        semantic_readiness, semantic_extensions, semantic_actions = _semantic_status(
            stage_id,
            authoritative,
        )
        if semantic_readiness == "ready_for_next_stage":
            semantic_actions.extend(_downstream_actions(workspace, job_dir, stage_id))
        readiness = semantic_readiness
    elif inspection.output_drift:
        readiness = "review_required"

    actions: list[NextAction] = []
    terminal_promote = "terminal_claim:promote" in inspection.reasons
    terminal_cancel = "terminal_claim:cancel" in inspection.reasons
    if inspection.stage.status in {"ready", "stale", "failed", "cancelled"}:
        actions.append(_stage_start_action(stage_id))
    elif inspection.stage.status == "blocked":
        if "input_not_ready:criteria_review" in inspection.reasons:
            actions.append(
                NextAction(
                    id="criteria.review_confirmations",
                    label="Resolve the unknown criteria extraction before matching",
                )
            )
        else:
            actions.append(
                NextAction(
                    id="stage.resolve_dependencies",
                    label="Run the required upstream stages",
                )
            )
    elif inspection.stage.status == "running" and terminal_cancel:
        actions.append(
            NextAction(
                id="stage.cancel_active_task",
                label="Resume the claimed task cancellation",
            )
        )
    elif inspection.stage.status == "running" and terminal_promote:
        consent_ids = [item.id for item in status_consents]
        actions.append(
            NextAction(
                id=f"stage.apply_{stage_id}_candidate",
                label=f"Resume the claimed {stage_id.title()} promotion",
                requires_consent=bool(consent_ids),
                consent_ids=consent_ids,
            )
        )
    elif inspection.stage.status == "running" and (
        inspection.output_drift
        or "input_changed" in inspection.reasons
        or "prepared_input_changed" in inspection.reasons
        or any(
            reason.startswith("dependency_not_current:")
            for reason in inspection.reasons
        )
    ):
        actions.append(
            NextAction(
                id="stage.cancel_active_task",
                label="Cancel the stale active task before rerunning the workflow",
            )
        )
    elif inspection.stage.status == "running":
        submission_exists = bool(
            inspection.pending_task_path is not None
            and (inspection.pending_task_path.parent / "submission.json").is_file()
        )
        if submission_exists:
            consent_ids = [item.id for item in status_consents]
            actions.append(
                NextAction(
                    id=f"stage.apply_{stage_id}_candidate",
                    label=f"Review and apply the submitted {stage_id.title()} candidate",
                    requires_consent=bool(consent_ids),
                    consent_ids=consent_ids,
                )
            )
        else:
            if pending_task is not None and pending_task.execution_mode == "deterministic":
                actions.append(_stage_run_action(stage_id))
            else:
                consent_ids = _pending_consent_ids(inspection.pending_task_path)
                actions.append(
                    NextAction(
                        id=f"stage.submit_{stage_id}_candidate",
                        label=f"Submit the prepared {stage_id.title()} candidate through the guarded CLI",
                        requires_consent=bool(consent_ids),
                        consent_ids=list(consent_ids),
                    )
                )
    elif inspection.stage.status == "succeeded":
        actions.extend(semantic_actions)

    blockers = []
    if inspection.output_drift:
        blockers.append("Review the changed authoritative stage output before rerunning.")
    if inspection.stage.status == "blocked":
        blockers.append(
            "Criteria extraction requires review before Match can run."
            if "input_not_ready:criteria_review" in inspection.reasons
            else "One or more required upstream stages are not current."
        )
    elif not (terminal_promote or terminal_cancel) and any(
        reason.startswith("dependency_not_current:") for reason in inspection.reasons
    ):
        blockers.append("The active task has an upstream dependency that is no longer current.")
    elif (
        not (terminal_promote or terminal_cancel)
        and inspection.stage.status == "running"
        and (
            "input_changed" in inspection.reasons
            or "prepared_input_changed" in inspection.reasons
        )
    ):
        blockers.append("The active task inputs are no longer current.")
    extensions = {
        "canisend.stage_id": stage_id,
        "canisend.stage_status": inspection.stage.status,
        "canisend.input_fingerprint": inspection.input_fingerprint,
        "canisend.output_drift": inspection.output_drift,
        "canisend.state_reconstructed": inspection.reconstructed,
        **semantic_extensions,
    }
    return success_response(
        operation="workflow.stage_status",
        workflow=WorkflowSnapshotReference(
            phase=_agent_phase(stage_id),  # type: ignore[arg-type]
            readiness=readiness,  # type: ignore[arg-type]
        ),
        artifacts=artifacts,
        warnings=["The authoritative stage output has local drift."]
        if inspection.output_drift
        else [],
        blockers=blockers,
        required_consents=status_consents,
        next_actions=actions,
        extensions=extensions,
    )


def stage_prepare_agent_response(
    workspace: Path,
    job_dir: Path,
    prepared: PreparedStage,
) -> AgentResponse:
    stage_id = prepared.task_spec.stage
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
            path=prepared.task_spec_path.parent / "preparation.json",
            kind="stage_preparation_receipt",
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
    consents = [
        ConsentRequirement(
            id=consent_id,
            purpose=f"Allow the current host agent to read the declared private inputs for {stage_id.title()}.",
            privacy_tier=prepared.task_spec.privacy_tier,
            artifact_kinds=["job_advert"] if consent_id == "read-full-job-advert" else ["stage_input"],
        )
        for consent_id in prepared.task_spec.required_consents
    ]
    return success_response(
        operation="workflow.stage_prepare",
        workflow=WorkflowSnapshotReference(
            phase=_agent_phase(stage_id),  # type: ignore[arg-type]
            readiness="action_required",
        ),
        artifacts=artifacts,
        required_consents=consents,
        next_actions=(
            [_stage_run_action(stage_id)]
            if prepared.task_spec.execution_mode == "deterministic"
            else [
                NextAction(
                    id=f"stage.submit_{stage_id}_candidate",
                    label="Submit candidate JSON through the guarded stage submit command",
                    requires_consent=bool(consents),
                    consent_ids=[item.id for item in consents],
                )
            ]
        ),
        extensions={
            "canisend.stage_id": stage_id,
            "canisend.stage_status": "running",
            "canisend.run_id": prepared.task_spec.run_id,
            "canisend.task_id": prepared.task_spec.task_id,
            "canisend.input_fingerprint": prepared.task_spec.input_fingerprint,
            "canisend.reused": prepared.reused,
        },
    )


def stage_submit_agent_response(
    workspace: Path,
    submitted: SubmittedStage,
) -> AgentResponse:
    stage_id = submitted.task_spec.stage
    adapter = get_stage_adapter(stage_id)
    return success_response(
        operation="workflow.stage_submit",
        workflow=WorkflowSnapshotReference(
            phase=_agent_phase(stage_id),  # type: ignore[arg-type]
            readiness="review_required",
        ),
        artifacts=[
            artifact_reference_from_path(
                workspace=workspace,
                path=submitted.candidate_path,
                kind=f"{adapter.artifact_kind}_candidate",
                privacy_tier=adapter.privacy_tier,
                trust_level="generated_candidate",
                media_type=adapter.media_type,
                include_hash=True,
            ),
            artifact_reference_from_path(
                workspace=workspace,
                path=submitted.result_path,
                kind="stage_task_result",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            ),
            artifact_reference_from_path(
                workspace=workspace,
                path=submitted.submission_path,
                kind="stage_candidate_submission",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            ),
        ],
        next_actions=[
            NextAction(
                id=f"stage.apply_{stage_id}_candidate",
                label=f"Review and apply the submitted {stage_id.title()} candidate",
            )
        ],
        extensions={
            "canisend.stage_id": stage_id,
            "canisend.stage_status": "running",
            "canisend.run_id": submitted.task_spec.run_id,
            "canisend.task_id": submitted.task_spec.task_id,
        },
    )


def stage_apply_agent_response(workspace: Path, applied: AppliedStage) -> AgentResponse:
    stage_id = applied.manifest.stage
    adapter = get_stage_adapter(stage_id)
    readiness, semantic_extensions, semantic_actions = _semantic_status(
        stage_id,
        applied.authoritative_path,
    )
    if readiness == "ready_for_next_stage":
        semantic_actions.extend(
            _downstream_actions(
                workspace,
                applied.authoritative_path.parent,
                stage_id,
            )
        )
    return success_response(
        operation="workflow.stage_apply",
        workflow=WorkflowSnapshotReference(
            phase=_agent_phase(stage_id),  # type: ignore[arg-type]
            readiness=readiness,  # type: ignore[arg-type]
        ),
        artifacts=[
            artifact_reference_from_path(
                workspace=workspace,
                path=applied.authoritative_path,
                kind=adapter.artifact_kind,
                privacy_tier=adapter.privacy_tier,
                trust_level="validated",
                media_type=adapter.media_type,
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
        next_actions=semantic_actions,
        extensions={
            "canisend.stage_id": stage_id,
            "canisend.stage_status": "succeeded",
            "canisend.run_id": applied.manifest.run_id,
            "canisend.cache_hit": False,
            **semantic_extensions,
        },
    )


def stage_cancel_agent_response(
    workspace: Path,
    cancelled: CancelledStage,
) -> AgentResponse:
    stage_id = cancelled.manifest.stage
    return success_response(
        operation="workflow.stage_cancel",
        workflow=WorkflowSnapshotReference(
            phase=_agent_phase(stage_id),  # type: ignore[arg-type]
            readiness="action_required",
        ),
        artifacts=[
            artifact_reference_from_path(
                workspace=workspace,
                path=cancelled.manifest_path,
                kind="stage_run_manifest",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            )
        ],
        next_actions=[
            _stage_start_action(stage_id)
        ],
        extensions={
            "canisend.stage_id": stage_id,
            "canisend.stage_status": "cancelled",
            "canisend.run_id": cancelled.manifest.run_id,
        },
    )


def stage_run_agent_response(workspace: Path, outcome: StageRunOutcome) -> AgentResponse:
    stage_id = outcome.stage
    adapter = get_stage_adapter(stage_id)
    readiness, semantic_extensions, semantic_actions = _semantic_status(
        stage_id,
        outcome.authoritative_path,
    )
    if readiness == "ready_for_next_stage":
        semantic_actions.extend(
            _downstream_actions(
                workspace,
                outcome.authoritative_path.parent,
                stage_id,
            )
        )
    artifacts = [
        artifact_reference_from_path(
            workspace=workspace,
            path=outcome.authoritative_path,
            kind=adapter.artifact_kind,
            privacy_tier=adapter.privacy_tier,
            trust_level="validated",
            media_type=adapter.media_type,
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
        workflow=WorkflowSnapshotReference(
            phase=_agent_phase(stage_id),  # type: ignore[arg-type]
            readiness=readiness,  # type: ignore[arg-type]
        ),
        artifacts=artifacts,
        next_actions=semantic_actions,
        extensions={
            "canisend.stage_id": stage_id,
            "canisend.stage_status": "succeeded",
            "canisend.cache_hit": outcome.cache_hit,
            "canisend.run_id": outcome.manifest.run_id if outcome.manifest is not None else None,
            **semantic_extensions,
        },
    )


def stage_error_response(operation: str, error: StageRuntimeError) -> AgentResponse:
    return error_response(
        operation=operation,
        code=error.code,
        message=str(error),
        retryable=error.code in {"stage.stale_input", "stage.store_failed"},
    )


def _agent_phase(stage_id: str) -> str:
    if stage_id in {"evidence", "parse"}:
        return stage_id
    return "unknown"


def _stage_start_action(stage_id: str) -> NextAction:
    definition = DEFAULT_STAGE_REGISTRY.get(stage_id)
    if definition.execution_modes == ("deterministic",):
        return _stage_run_action(stage_id)
    return NextAction(
        id=f"stage.prepare_{stage_id}",
        label=f"Prepare the {stage_id.title()} stage",
    )


def _stage_run_action(stage_id: str) -> NextAction:
    return NextAction(
        id=f"stage.run_{stage_id}",
        label=f"Run the {stage_id.title()} stage deterministically",
    )


def _downstream_actions(
    workspace: Path,
    job_dir: Path,
    stage_id: str,
) -> list[NextAction]:
    if stage_id == "parse":
        return _next_action_for_stage(workspace, job_dir, "confirm")
    if stage_id == "confirm":
        evidence_actions = _next_action_for_stage(workspace, job_dir, "evidence")
        if evidence_actions:
            return evidence_actions
        evidence_readiness, evidence_semantic_actions = _current_semantic_status(
            workspace,
            job_dir,
            "evidence",
        )
        if evidence_readiness != "ready_for_next_stage":
            return evidence_semantic_actions
        return _next_action_for_stage(workspace, job_dir, "match")
    if stage_id == "evidence":
        try:
            confirm = inspect_stage_status(workspace, job_dir, stage="confirm")
        except StageRuntimeError:
            return []
        if confirm.stage.status == "succeeded" and not confirm.reasons:
            confirm_readiness, confirm_semantic_actions = _current_semantic_status(
                workspace,
                job_dir,
                "confirm",
            )
            if confirm_readiness != "ready_for_next_stage":
                return confirm_semantic_actions
            return _next_action_for_stage(workspace, job_dir, "match")
        confirm_actions = _next_action_for_stage(workspace, job_dir, "confirm")
        if confirm_actions:
            return confirm_actions
        return _next_action_for_stage(workspace, job_dir, "parse")
    return []


def _next_action_for_stage(
    workspace: Path,
    job_dir: Path,
    stage_id: str,
) -> list[NextAction]:
    try:
        inspection = inspect_stage_status(
            workspace,
            job_dir,
            stage=stage_id,  # type: ignore[arg-type]
        )
    except StageRuntimeError:
        return []
    if inspection.stage.status in {"ready", "stale", "failed", "cancelled"}:
        return [_stage_start_action(stage_id)]
    return []


def _current_semantic_status(
    workspace: Path,
    job_dir: Path,
    stage_id: str,
) -> tuple[str, list[NextAction]]:
    try:
        inspection = inspect_stage_status(
            workspace,
            job_dir,
            stage=stage_id,  # type: ignore[arg-type]
        )
        adapter = get_stage_adapter(stage_id)
    except (KeyError, StageRuntimeError):
        return "blocked", []
    if (
        inspection.stage.status != "succeeded"
        or inspection.reasons
        or inspection.output_drift
    ):
        return "blocked", []
    readiness, _extensions, actions = _semantic_status(
        stage_id,
        job_dir / adapter.authoritative_target,
    )
    return readiness, actions


def _semantic_status(
    stage_id: str,
    authoritative_path: Path,
) -> tuple[str, dict[str, int | str | None], list[NextAction]]:
    if not authoritative_path.is_file():
        return "ready_for_next_stage", {}, []
    if stage_id == "evidence":
        try:
            catalog = EvidenceCatalogV1.model_validate(read_json_object(authoritative_path))
        except (StageStoreError, ValidationError):
            return "review_required", {}, [
                NextAction(
                    id="evidence.review_catalog",
                    label="Review the invalid evidence catalog",
                )
            ]
        extensions = {
            "canisend.evidence_count": len(catalog.items),
            "canisend.evidence_gap_count": 0 if catalog.state == "available" else 1,
            "canisend.evidence_state": catalog.state,
            "canisend.evidence_reason": catalog.unavailable_reason,
        }
        if catalog.state == "available":
            return "ready_for_next_stage", extensions, []
        if catalog.state == "empty":
            return "review_required", extensions, [
                NextAction(
                    id="profile.add_evidence",
                    label="Add profile evidence or review the valid empty catalog",
                )
            ]
        if catalog.unavailable_reason == "evidence.profile_missing":
            return "review_required", extensions, [
                NextAction(
                    id="profile.initialize",
                    label="Initialize the applicant profile",
                )
            ]
        return "review_required", extensions, [
            NextAction(
                id="profile.extract_evidence",
                label="Extract or add profile evidence, then rerun Evidence",
            )
        ]
    if stage_id == "match":
        try:
            matches = CriterionMatchesV1.model_validate(read_json_object(authoritative_path))
        except (StageStoreError, ValidationError):
            return "review_required", {}, [
                NextAction(
                    id="matches.review_catalog",
                    label="Review the invalid criterion match catalog",
                )
            ]
        proposed_count = sum(item.review_state == "proposed" for item in matches.matches)
        missing_count = sum(item.classification == "missing" for item in matches.matches)
        unknown_count = sum(item.classification == "unknown" for item in matches.matches)
        extensions = {
            "canisend.match_count": len(matches.matches),
            "canisend.proposed_count": proposed_count,
            "canisend.missing_count": missing_count,
            "canisend.unknown_count": unknown_count,
        }
        if proposed_count == 0 and missing_count == 0 and unknown_count == 0:
            return "ready_for_next_stage", extensions, []
        return "review_required", extensions, [
            NextAction(
                id="matches.review_proposals",
                label="Review proposed criterion classifications and evidence gaps",
            )
        ]
    if stage_id != "confirm":
        return "ready_for_next_stage", {}, []
    try:
        catalog = CriteriaCatalogV1.model_validate(read_json_object(authoritative_path))
    except (StageStoreError, ValidationError):
        return "review_required", {}, [
            NextAction(
                id="criteria.review_catalog",
                label="Review the invalid criteria catalog",
            )
        ]
    unresolved_count = len(catalog.unresolved_criterion_ids) + len(
        catalog.orphaned_correction_ids
    ) + (1 if catalog.extraction_state == "unknown" else 0)
    if unresolved_count == 0:
        return "ready_for_next_stage", {"canisend.unresolved_count": 0}, []
    return "review_required", {"canisend.unresolved_count": unresolved_count}, [
        NextAction(
            id="criteria.review_confirmations",
            label="Review unresolved criteria and correction records",
        )
    ]


def _pending_consent_ids(task_path: Path | None) -> tuple[str, ...]:
    if task_path is None:
        return ()
    try:
        return TaskSpecV1.model_validate(read_json_object(task_path)).required_consents
    except (StageStoreError, ValidationError):
        return ()


def _pending_task_spec(task_path: Path | None) -> TaskSpecV1 | None:
    if task_path is None:
        return None
    try:
        return TaskSpecV1.model_validate(read_json_object(task_path))
    except (StageStoreError, ValidationError):
        return None


def _pending_submission(path: Path) -> CandidateSubmissionV1 | None:
    try:
        return CandidateSubmissionV1.model_validate(read_json_object(path))
    except (StageStoreError, ValidationError):
        return None
