from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
import json
from pathlib import Path
from typing import Literal
from uuid import uuid4

from pydantic import ValidationError

from canisend.stage_adapters import StageAdapter, get_stage_adapter
from canisend.stage_models import (
    ArtifactFingerprint,
    CandidateSubmissionV1,
    RunManifestV1,
    StageRecord,
    TaskResultV1,
    TaskSpecV1,
    ValidationReportV1,
    WorkflowStateV1,
)
from canisend.stage_registry import DEFAULT_STAGE_REGISTRY, StageDefinition
from canisend.stage_store import (
    ImmutableRecordError,
    StageStoreError,
    UnsafeStagePathError,
    atomic_write_bytes,
    atomic_write_json,
    read_json_object,
    resolve_job_relative_path,
    sha256_bytes,
    sha256_file,
    write_immutable_json,
)
from canisend.stages.confirm_stage import ConfirmStageError
from canisend.stages.parse_stage import ParseStageError


SupportedStage = Literal["parse", "confirm"]
SupportedExecutionMode = Literal["deterministic", "host_agent"]


class StageRuntimeError(RuntimeError):
    """A safe, stable failure at the stage-runtime boundary."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class PreparedStage:
    task_spec: TaskSpecV1
    task_spec_path: Path
    candidate_path: Path
    result_path: Path
    state: WorkflowStateV1
    reused: bool = False


@dataclass(frozen=True)
class StageStatusInspection:
    state: WorkflowStateV1
    stage: StageRecord
    input_fingerprint: str | None
    output_drift: bool = False
    reasons: tuple[str, ...] = ()
    pending_task_path: Path | None = None
    reconstructed: bool = False


@dataclass(frozen=True)
class AppliedStage:
    manifest: RunManifestV1
    manifest_path: Path
    state: WorkflowStateV1
    authoritative_path: Path


@dataclass(frozen=True)
class CancelledStage:
    manifest: RunManifestV1
    manifest_path: Path
    state: WorkflowStateV1


@dataclass(frozen=True)
class SubmittedStage:
    task_spec: TaskSpecV1
    candidate_path: Path
    result: TaskResultV1
    result_path: Path
    submission_path: Path


@dataclass(frozen=True)
class StageRunOutcome:
    stage: SupportedStage
    cache_hit: bool
    state: WorkflowStateV1
    authoritative_path: Path
    manifest: RunManifestV1 | None = None
    manifest_path: Path | None = None


def inspect_stage_status(
    workspace: Path,
    job_dir: Path,
    *,
    stage: SupportedStage = "parse",
) -> StageStatusInspection:
    root, job = _runtime_paths(workspace, job_dir)
    definition = _implemented_stage(stage)
    adapter = _adapter(stage)
    state, reconstructed = _load_or_reconstruct_state(job)
    dependency_reasons: list[str] = []
    for dependency in definition.depends_on:
        dependency_definition = DEFAULT_STAGE_REGISTRY.get(dependency)
        if not dependency_definition.implemented:
            continue
        dependency_status = inspect_stage_status(
            root,
            job,
            stage=dependency,  # type: ignore[arg-type]
        )
        state = dependency_status.state
        reconstructed = reconstructed or dependency_status.reconstructed
        dependency_is_current = (
            dependency_status.stage.status == "succeeded"
            and not dependency_status.reasons
            and not dependency_status.output_drift
        )
        if not dependency_is_current:
            dependency_reasons.append(f"dependency_not_current:{dependency}")

    record = _stage_record(state, stage)
    if dependency_reasons:
        if record.status in {"succeeded", "stale"}:
            record = record.model_copy(update={"status": "stale"})
        elif record.status != "running":
            record = StageRecord(stage=stage, status="blocked")
        output_drift, output_reasons = _inspect_output_receipt(job, record, adapter)
        pending_task = (
            _pending_task_for_run(job, record.run_id)
            if record.status == "running"
            else None
        )
        if pending_task is not None:
            pending_drift, pending_reasons = _inspect_pending_output_expectation(
                job,
                pending_task[0],
                adapter,
            )
            output_drift = output_drift or pending_drift
            output_reasons = tuple((*output_reasons, *pending_reasons))
            claim_action = _terminal_claim_action(job, pending_task[0])
            if claim_action is not None:
                output_reasons = tuple(
                    (*output_reasons, f"terminal_claim:{claim_action}")
                )
                if (
                    claim_action == "promote"
                    and _claimed_candidate_is_authoritative(
                        job,
                        pending_task[0],
                        adapter,
                    )
                ):
                    output_drift = False
                    output_reasons = tuple(
                        reason
                        for reason in output_reasons
                        if reason not in {"output_drift", "output_missing"}
                    ) + ("promotion_recovery",)
        visible_state = _state_with_stage(
            state,
            record,
            active_run_id=state.active_run_id,
        )
        return StageStatusInspection(
            state=visible_state,
            stage=record,
            input_fingerprint=None,
            output_drift=output_drift,
            reasons=tuple((*dependency_reasons, *output_reasons)),
            pending_task_path=pending_task[1] if pending_task is not None else None,
            reconstructed=reconstructed,
        )

    try:
        current_fingerprint = adapter.input_fingerprint(root, job)
    except (ConfirmStageError, ParseStageError, StageStoreError, OSError, ValueError) as exc:
        raise StageRuntimeError(
            "stage.invalid_input",
            "The requested stage inputs are missing or invalid.",
        ) from exc

    reasons: list[str] = []

    if record.status in {"succeeded", "stale", "running"}:
        if record.input_fingerprint != current_fingerprint:
            reasons.append("input_changed")
            if record.status not in {"stale", "running"}:
                record = record.model_copy(update={"status": "stale"})

    output_drift, output_reasons = _inspect_output_receipt(job, record, adapter)
    reasons.extend(output_reasons)
    if "output_missing" in output_reasons and record.status != "stale":
        record = record.model_copy(update={"status": "stale"})

    pending_task = _pending_task_for_run(job, record.run_id) if record.status == "running" else None
    if pending_task is None:
        pending_task = _pending_task_for_fingerprint(
            job,
            stage=stage,
            input_fingerprint=current_fingerprint,
            execution_mode=None,
        )
    if pending_task is not None:
        pending_drift, pending_reasons = _inspect_pending_output_expectation(
            job,
            pending_task[0],
            adapter,
        )
        output_drift = output_drift or pending_drift
        reasons.extend(reason for reason in pending_reasons if reason not in reasons)
        claim_action = _terminal_claim_action(job, pending_task[0])
        if claim_action is not None:
            claim_reason = f"terminal_claim:{claim_action}"
            if claim_reason not in reasons:
                reasons.append(claim_reason)
            if (
                claim_action == "promote"
                and _claimed_candidate_is_authoritative(
                    job,
                    pending_task[0],
                    adapter,
                )
            ):
                output_drift = False
                reasons = [
                    reason
                    for reason in reasons
                    if reason not in {"output_drift", "output_missing"}
                ]
                reasons.append("promotion_recovery")
    pending_path = pending_task[1] if pending_task is not None else None
    visible_state = _state_with_stage(state, record, active_run_id=state.active_run_id)
    if "input_changed" in reasons:
        visible_state = _with_stale_descendants(visible_state, stage)
    return StageStatusInspection(
        state=visible_state,
        stage=record,
        input_fingerprint=current_fingerprint,
        output_drift=output_drift,
        reasons=tuple(reasons),
        pending_task_path=pending_path,
        reconstructed=reconstructed,
    )


def prepare_stage(
    workspace: Path,
    job_dir: Path,
    *,
    stage: SupportedStage,
    execution_mode: SupportedExecutionMode,
) -> PreparedStage:
    root, job = _runtime_paths(workspace, job_dir)
    definition = _implemented_stage(stage)
    adapter = _adapter(stage)
    if execution_mode not in definition.execution_modes:
        raise StageRuntimeError(
            "stage.unsupported_mode",
            "The requested execution mode is not supported for this stage.",
        )

    _finalize_recoverable_promotions(job)

    status = inspect_stage_status(root, job, stage=stage)
    if status.input_fingerprint is None:
        raise StageRuntimeError(
            "stage.dependency_not_current",
            "The requested stage requires current upstream stages.",
        )
    if status.output_drift:
        raise StageRuntimeError(
            "stage.output_conflict",
            "The authoritative stage output has changed since its last successful run.",
        )
    if status.stage.status == "succeeded" and not status.reasons:
        raise StageRuntimeError("stage.already_current", "The requested stage is already current.")

    existing = _pending_task_for_fingerprint(
        job,
        stage=stage,
        input_fingerprint=status.input_fingerprint,
        execution_mode=execution_mode,
    )
    if existing is not None:
        task, task_path = existing
        return PreparedStage(
            task_spec=task,
            task_spec_path=task_path,
            candidate_path=resolve_job_relative_path(job, task.candidate_output),
            result_path=resolve_job_relative_path(job, task.result_output),
            state=status.state,
            reused=True,
        )

    active_record = next(
        (record for record in status.state.stages if record.status == "running"),
        None,
    )
    if active_record is not None:
        raise StageRuntimeError(
            "stage.concurrent_run",
            "Another workflow task is active; cancel or complete it before preparing a new task.",
        )

    now = _utc_now()
    run_id = f"run_{uuid4().hex}"
    task_id = f"task_{uuid4().hex}"
    run_root = f"workflow/runs/{run_id}"
    candidate_output = f"{run_root}/candidates/{adapter.candidate_name}"
    result_output = f"{run_root}/tasks/{task_id}/result.json"
    try:
        inputs = adapter.input_artifacts(root, job)
    except (ConfirmStageError, ParseStageError, StageStoreError, OSError, ValueError) as exc:
        raise StageRuntimeError(
            "stage.invalid_input",
            "The requested stage inputs are missing or invalid.",
        ) from exc
    expected_output = _file_hash_or_none(job / adapter.authoritative_target)
    baseline_outputs = (
        (_artifact(job, adapter.authoritative_target),)
        if expected_output is not None
        else ()
    )
    task = TaskSpecV1(
        task_id=task_id,
        run_id=run_id,
        job_id=job.name,
        stage=stage,
        operation=f"stage.{stage}",
        execution_mode=execution_mode,
        created_at=now,
        input_fingerprint=status.input_fingerprint,
        inputs=inputs,
        allowed_reads=tuple(item.path for item in inputs),
        allowed_writes=(candidate_output, result_output),
        candidate_output=candidate_output,
        result_output=result_output,
        authoritative_target=adapter.authoritative_target,
        expected_output_sha256=expected_output,
        output_schema=adapter.output_schema,
        privacy_tier=adapter.task_privacy_tier,
        required_consents=adapter.required_consents(execution_mode),
    )
    task_path = resolve_job_relative_path(job, f"{run_root}/task-spec.json")
    candidate_path = resolve_job_relative_path(job, candidate_output)
    result_path = resolve_job_relative_path(job, result_output)
    candidate_path.parent.mkdir(parents=True, exist_ok=True)
    result_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        write_immutable_json(task_path, task.model_dump(mode="json"))
        preparation = RunManifestV1(
            run_id=run_id,
            task_id=task_id,
            job_id=job.name,
            stage=stage,
            attempt=status.stage.attempt_count + 1,
            execution_mode=execution_mode,
            status="prepared",
            created_at=now,
            inputs=inputs,
            input_fingerprint=status.input_fingerprint,
            task_spec_sha256=sha256_file(task_path),
        )
        write_immutable_json(
            resolve_job_relative_path(job, f"{run_root}/preparation.json"),
            preparation.model_dump(mode="json"),
        )
    except StageStoreError as exc:
        raise StageRuntimeError(
            "stage.store_failed",
            "The immutable stage task could not be stored safely.",
        ) from exc

    previous_attempts = status.stage.attempt_count
    running = StageRecord(
        stage=stage,
        status="running",
        attempt_count=previous_attempts + 1,
        run_id=run_id,
        input_fingerprint=status.input_fingerprint,
        inputs=inputs,
        outputs=baseline_outputs,
        started_at=now,
    )
    state = _state_with_stage(
        status.state,
        running,
        active_run_id=run_id,
        increment_revision=True,
        updated_at=now,
    )
    _write_state(job, state)
    return PreparedStage(
        task_spec=task,
        task_spec_path=task_path,
        candidate_path=candidate_path,
        result_path=result_path,
        state=state,
    )


def apply_stage_result(
    workspace: Path,
    job_dir: Path,
    *,
    task_spec_path: Path,
    task_result_path: Path,
) -> AppliedStage:
    root, job = _runtime_paths(workspace, job_dir)
    task: TaskSpecV1 | None = None
    result: TaskResultV1 | None = None
    trusted_task = False
    promotion_claimed = False
    try:
        task_relative, task_path = _validated_supplied_path(job, task_spec_path)
        task = TaskSpecV1.model_validate(read_json_object(task_path))
        expected_task_relative = f"workflow/runs/{task.run_id}/task-spec.json"
        if task_relative != expected_task_relative:
            raise StageRuntimeError("stage.task_identity_mismatch", "TaskSpec path does not match its run.")
        definition = _implemented_stage(task.stage)
        adapter = _adapter(task.stage)
        _validate_task_contract(job, task, definition, adapter)
        _validate_task_preparation(job, task_path, task)
        _validate_active_task(job, task, adapter)
        trusted_task = True

        result_relative, result_path = _validated_supplied_path(job, task_result_path)
        if result_relative != task.result_output:
            raise StageRuntimeError(
                "stage.result_identity_mismatch",
                "TaskResult path does not match the immutable task contract.",
            )
        result = TaskResultV1.model_validate(read_json_object(result_path))
        _validate_task_result_identity(task, result)
        _validate_candidate_submission(
            job,
            task_path=task_path,
            task=task,
            result_path=result_path,
            result=result,
        )

        target = _validate_task_freshness(
            root,
            job,
            task,
            adapter,
            accepted_output_sha256=result.outputs[0].sha256,
        )

        if len(result.outputs) != 1 or result.outputs[0].path != task.candidate_output:
            raise StageRuntimeError(
                "stage.result_scope_mismatch",
                "TaskResult does not contain exactly the declared stage candidate.",
            )
        candidate = result.outputs[0]
        try:
            candidate_path = resolve_job_relative_path(job, candidate.path)
        except UnsafeStagePathError as exc:
            raise StageRuntimeError(
                "stage.unsafe_path",
                "The stage candidate path is outside its declared staging scope.",
            ) from exc
        declared_candidate = resolve_job_relative_path(job, task.candidate_output)
        if candidate_path != declared_candidate:
            raise StageRuntimeError(
                "stage.result_scope_mismatch",
                "TaskResult candidate does not match the declared staging path.",
            )
        _require_unaliased_regular_file(candidate_path, label="stage candidate")
        try:
            candidate_bytes = candidate_path.read_bytes()
        except OSError as exc:
            raise StageRuntimeError(
                "stage.candidate_missing",
                "The declared stage candidate cannot be read.",
            ) from exc
        if sha256_bytes(candidate_bytes) != candidate.sha256 or (
            candidate.size_bytes is not None and len(candidate_bytes) != candidate.size_bytes
        ):
            raise StageRuntimeError(
                "stage.candidate_hash_mismatch",
                "The stage candidate does not match its declared fingerprint.",
            )
        try:
            candidate_object = json.loads(candidate_bytes)
        except (UnicodeError, json.JSONDecodeError) as exc:
            raise StageRuntimeError(
                "stage.invalid_candidate",
                "The stage candidate is not valid UTF-8 JSON.",
            ) from exc
        try:
            adapter.validate_candidate(
                root,
                job,
                candidate_object,
                input_fingerprint=task.input_fingerprint,
            )
        except (ConfirmStageError, ParseStageError, OSError, UnicodeError, ValueError) as exc:
            raise StageRuntimeError(
                "stage.invalid_candidate",
                "The stage candidate failed schema, semantic, or source-receipt validation.",
            ) from exc

        validation_relative = f"workflow/runs/{task.run_id}/validation/report.json"
        validation_path = resolve_job_relative_path(job, validation_relative)
        validation = _load_or_write_passed_validation(
            validation_path,
            task=task,
            citations_valid=adapter.citations_validated,
        )
        checked_at = validation.checked_at

        _validate_active_task(job, task, adapter)
        _claim_terminal_action(
            job,
            task=task,
            task_path=task_path,
            action="promote",
            candidate_sha256=candidate.sha256,
        )
        promotion_claimed = True
        atomic_write_bytes(target, candidate_bytes)
        promoted = _artifact(job, task.authoritative_target)
        promotion_relative = f"workflow/runs/{task.run_id}/promotion.json"
        promotion_path = resolve_job_relative_path(job, promotion_relative)
        write_immutable_json(
            promotion_path,
            {
                "schema_version": "1.0.0",
                "run_id": task.run_id,
                "task_id": task.task_id,
                "stage": task.stage,
                "attempt": _attempt_for_run(job, task.run_id, task.stage),
                "input_fingerprint": task.input_fingerprint,
                "candidate_sha256": candidate.sha256,
                "authoritative_target": task.authoritative_target,
                "authoritative_sha256": promoted.sha256,
                "promoted_at": checked_at.isoformat().replace("+00:00", "Z"),
            },
        )

        manifest = RunManifestV1(
            run_id=task.run_id,
            task_id=task.task_id,
            job_id=task.job_id,
            stage=task.stage,
            attempt=_attempt_for_run(job, task.run_id, task.stage),
            execution_mode=task.execution_mode,
            status="succeeded",
            created_at=task.created_at,
            started_at=task.created_at,
            completed_at=checked_at,
            inputs=task.inputs,
            input_fingerprint=task.input_fingerprint,
            task_spec_sha256=sha256_file(task_path),
            candidate_outputs=result.outputs,
            promoted_outputs=(promoted,),
            validation_report_path=validation_relative,
        )
        manifest_path = resolve_job_relative_path(
            job,
            f"workflow/runs/{task.run_id}/manifest.json",
        )
        write_immutable_json(manifest_path, manifest.model_dump(mode="json"))
        state, _ = _load_or_reconstruct_state(job)
        succeeded = StageRecord(
            stage=task.stage,
            status="succeeded",
            attempt_count=manifest.attempt,
            run_id=task.run_id,
            input_fingerprint=task.input_fingerprint,
            inputs=task.inputs,
            outputs=(promoted,),
            started_at=task.created_at,
            completed_at=checked_at,
        )
        updated_state = _state_with_stage(
            state,
            succeeded,
            active_run_id=None,
            increment_revision=True,
            updated_at=checked_at,
        )
        updated_state = _with_stale_descendants(updated_state, task.stage)
        _write_state(job, updated_state)
        return AppliedStage(
            manifest=manifest,
            manifest_path=manifest_path,
            state=updated_state,
            authoritative_path=target,
        )
    except StageRuntimeError as exc:
        if (
            task is not None
            and trusted_task
            and not promotion_claimed
            and exc.code != "stage.task_not_active"
        ):
            _record_rejected_run(job, task, result, error=exc)
        raise
    except (
        ConfirmStageError,
        StageStoreError,
        ValidationError,
        ParseStageError,
        OSError,
        ValueError,
    ) as exc:
        wrapped = StageRuntimeError(
            "stage.invalid_result",
            "The stage result could not be loaded or validated safely.",
        )
        if task is not None and trusted_task and not promotion_claimed:
            _record_rejected_run(job, task, result, error=wrapped)
        raise wrapped from exc


def submit_stage_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    task_spec_path: Path,
    candidate_bytes: bytes,
) -> SubmittedStage:
    root, job = _runtime_paths(workspace, job_dir)
    try:
        task_relative, task_path = _validated_supplied_path(job, task_spec_path)
        task = TaskSpecV1.model_validate(read_json_object(task_path))
        if task_relative != f"workflow/runs/{task.run_id}/task-spec.json":
            raise StageRuntimeError(
                "stage.task_identity_mismatch",
                "TaskSpec path does not match its run.",
            )
        definition = _implemented_stage(task.stage)
        adapter = _adapter(task.stage)
        _validate_task_contract(job, task, definition, adapter)
        _validate_task_preparation(job, task_path, task)
        _validate_active_task(job, task, adapter)
        _validate_task_freshness(root, job, task, adapter)
        try:
            candidate_object = json.loads(candidate_bytes)
        except (UnicodeError, json.JSONDecodeError) as exc:
            raise StageRuntimeError(
                "stage.invalid_candidate",
                "The submitted stage candidate is not valid UTF-8 JSON.",
            ) from exc
        try:
            validated = adapter.validate_candidate(
                root,
                job,
                candidate_object,
                input_fingerprint=task.input_fingerprint,
            )
        except (ConfirmStageError, ParseStageError, OSError, UnicodeError, ValueError) as exc:
            raise StageRuntimeError(
                "stage.invalid_candidate",
                "The submitted stage candidate failed schema or semantic validation.",
            ) from exc
        canonical_bytes = (
            json.dumps(validated, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
        ).encode("utf-8")
        candidate_path = resolve_job_relative_path(job, task.candidate_output)
        _write_candidate_payload(candidate_path, canonical_bytes)
        candidate = ArtifactFingerprint(
            path=task.candidate_output,
            sha256=sha256_bytes(canonical_bytes),
            size_bytes=len(canonical_bytes),
        )
        result_path = resolve_job_relative_path(job, task.result_output)
        now = max(_utc_now(), task.created_at)
        if result_path.exists() or result_path.is_symlink():
            _require_unaliased_regular_file(result_path, label="task result")
            result = TaskResultV1.model_validate(read_json_object(result_path))
            _validate_task_result_identity(task, result)
            if result.outputs != (candidate,):
                raise StageRuntimeError(
                    "stage.result_scope_mismatch",
                    "Existing TaskResult does not match the submitted candidate.",
                )
        else:
            result = TaskResultV1(
                task_id=task.task_id,
                run_id=task.run_id,
                job_id=task.job_id,
                stage=task.stage,
                status="succeeded",
                input_fingerprint=task.input_fingerprint,
                started_at=task.created_at,
                completed_at=now,
                outputs=(candidate,),
            )
            write_immutable_json(result_path, result.model_dump(mode="json"))
        submission_path = resolve_job_relative_path(
            job,
            f"workflow/runs/{task.run_id}/submission.json",
        )
        submission = CandidateSubmissionV1(
            task_id=task.task_id,
            run_id=task.run_id,
            job_id=task.job_id,
            stage=task.stage,
            submitted_at=now,
            task_spec_sha256=sha256_file(task_path),
            candidate=candidate,
            result_path=task.result_output,
            task_result_sha256=sha256_file(result_path),
        )
        if submission_path.exists() or submission_path.is_symlink():
            existing = CandidateSubmissionV1.model_validate(
                read_json_object(submission_path)
            )
            if existing.model_copy(update={"submitted_at": submission.submitted_at}) != submission:
                raise StageRuntimeError(
                    "stage.submission_conflict",
                    "Existing candidate submission does not match this task result.",
                )
        else:
            write_immutable_json(submission_path, submission.model_dump(mode="json"))
        return SubmittedStage(
            task_spec=task,
            candidate_path=candidate_path,
            result=result,
            result_path=result_path,
            submission_path=submission_path,
        )
    except StageRuntimeError:
        raise
    except UnsafeStagePathError as exc:
        raise StageRuntimeError(
            "stage.unsafe_path",
            "The submitted candidate path is not an isolated run path.",
        ) from exc
    except (
        ConfirmStageError,
        StageStoreError,
        ValidationError,
        ParseStageError,
        OSError,
        ValueError,
    ) as exc:
        raise StageRuntimeError(
            "stage.invalid_result",
            "The submitted candidate could not be stored safely.",
        ) from exc


def cancel_stage_task(
    workspace: Path,
    job_dir: Path,
    *,
    stage: SupportedStage,
) -> CancelledStage:
    _, job = _runtime_paths(workspace, job_dir)
    _implemented_stage(stage)
    state, _ = _load_or_reconstruct_state(job)
    running = next(
        (record for record in state.stages if record.status == "running"),
        None,
    )
    if running is None or running.stage != stage:
        raise StageRuntimeError(
            "stage.no_active_run",
            "The requested stage has no active task to cancel.",
        )
    preparation = _load_preparation_for_run(job, running.run_id)
    if (
        preparation.status != "prepared"
        or preparation.job_id != job.name
        or preparation.stage != stage
        or preparation.input_fingerprint != running.input_fingerprint
        or preparation.task_id is None
    ):
        raise StageRuntimeError(
            "stage.task_integrity_mismatch",
            "The active task does not match its immutable preparation receipt.",
        )
    now = _utc_now()
    task_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{preparation.run_id}/task-spec.json",
    )
    task = TaskSpecV1.model_validate(read_json_object(task_path))
    definition = _implemented_stage(task.stage)
    adapter = _adapter(task.stage)
    _validate_task_contract(job, task, definition, adapter)
    _validate_task_preparation(job, task_path, task)
    _validate_active_task(job, task, adapter)
    _claim_terminal_action(
        job,
        task=task,
        task_path=task_path,
        action="cancel",
    )
    manifest = RunManifestV1(
        run_id=preparation.run_id,
        task_id=preparation.task_id,
        job_id=preparation.job_id,
        stage=preparation.stage,
        attempt=preparation.attempt,
        execution_mode=preparation.execution_mode,
        status="cancelled",
        created_at=preparation.created_at,
        started_at=preparation.created_at,
        completed_at=now,
        inputs=preparation.inputs,
        input_fingerprint=preparation.input_fingerprint,
        task_spec_sha256=preparation.task_spec_sha256,
        error_code="stage.cancelled",
        error_message="The prepared task was explicitly cancelled before promotion.",
    )
    manifest_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{preparation.run_id}/manifest.json",
    )
    try:
        write_immutable_json(manifest_path, manifest.model_dump(mode="json"))
    except StageStoreError as exc:
        raise StageRuntimeError(
            "stage.store_failed",
            "The cancellation manifest could not be stored safely.",
        ) from exc
    cancelled = StageRecord(
        stage=stage,
        status="cancelled",
        attempt_count=manifest.attempt,
        run_id=preparation.run_id,
        input_fingerprint=preparation.input_fingerprint,
        inputs=preparation.inputs,
        outputs=running.outputs,
        started_at=preparation.created_at,
        completed_at=now,
    )
    updated = _state_with_stage(
        state,
        cancelled,
        active_run_id=None,
        increment_revision=True,
        updated_at=now,
    )
    _write_state(job, updated)
    return CancelledStage(
        manifest=manifest,
        manifest_path=manifest_path,
        state=updated,
    )


def run_deterministic_stage(
    workspace: Path,
    job_dir: Path,
    *,
    stage: SupportedStage,
) -> StageRunOutcome:
    root, job = _runtime_paths(workspace, job_dir)
    adapter = _adapter(stage)
    _finalize_recoverable_promotions(job)
    status = inspect_stage_status(root, job, stage=stage)
    target = job / adapter.authoritative_target
    if status.input_fingerprint is None:
        raise StageRuntimeError(
            "stage.dependency_not_current",
            "The requested stage requires current upstream stages.",
        )
    if status.output_drift:
        raise StageRuntimeError(
            "stage.output_conflict",
            "The authoritative stage output has changed since its last successful run.",
        )
    if status.stage.status == "succeeded" and not status.reasons:
        return StageRunOutcome(
            stage=stage,
            cache_hit=True,
            state=status.state,
            authoritative_path=target,
        )

    prepared = prepare_stage(
        root,
        job,
        stage=stage,
        execution_mode="deterministic",
    )
    candidate = adapter.build_deterministic_candidate(
        root,
        job,
        input_fingerprint=prepared.task_spec.input_fingerprint,
    )
    candidate_bytes = (
        json.dumps(candidate, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")
    submitted = submit_stage_candidate(
        root,
        job,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=candidate_bytes,
    )
    applied = apply_stage_result(
        root,
        job,
        task_spec_path=prepared.task_spec_path,
        task_result_path=submitted.result_path,
    )
    return StageRunOutcome(
        stage=stage,
        cache_hit=False,
        state=applied.state,
        authoritative_path=applied.authoritative_path,
        manifest=applied.manifest,
        manifest_path=applied.manifest_path,
    )


def _runtime_paths(workspace: Path, job_dir: Path) -> tuple[Path, Path]:
    root = workspace.expanduser().resolve()
    job = job_dir.expanduser()
    if not job.is_absolute():
        job = (root / job).resolve()
    else:
        job = job.resolve()
    try:
        job.relative_to(root)
    except ValueError as exc:
        raise StageRuntimeError(
            "stage.job_outside_workspace",
            "Stage runtime requires a job inside the selected workspace.",
        ) from exc
    if not job.is_dir():
        raise StageRuntimeError("job.not_found", "The requested job directory does not exist.")
    return root, job


def _implemented_stage(stage: str):
    try:
        definition = DEFAULT_STAGE_REGISTRY.get(stage)
    except KeyError as exc:
        raise StageRuntimeError("stage.unknown", "The requested workflow stage is unknown.") from exc
    if not definition.implemented:
        raise StageRuntimeError(
            "stage.unsupported",
            "The requested workflow stage is declared but not implemented.",
        )
    return definition


def _adapter(stage: str) -> StageAdapter:
    try:
        return get_stage_adapter(stage)
    except KeyError as exc:
        raise StageRuntimeError(
            "stage.unsupported",
            "The requested workflow stage has no executable adapter.",
        ) from exc


def _artifact(job: Path, relative_path: str) -> ArtifactFingerprint:
    path = resolve_job_relative_path(job, relative_path)
    try:
        size = path.stat().st_size
        digest = sha256_file(path)
    except (OSError, StageStoreError) as exc:
        raise StageRuntimeError(
            "stage.artifact_unreadable",
            "A declared stage artifact cannot be inspected safely.",
        ) from exc
    return ArtifactFingerprint(path=relative_path, sha256=digest, size_bytes=size)


def _file_hash_or_none(path: Path) -> str | None:
    if not path.is_file():
        return None
    try:
        return sha256_file(path)
    except StageStoreError as exc:
        raise StageRuntimeError(
            "stage.output_unreadable",
            "The authoritative stage output cannot be inspected safely.",
        ) from exc


def _require_unaliased_regular_file(path: Path, *, label: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as exc:
        raise StageRuntimeError(
            "stage.candidate_missing",
            f"The declared {label} cannot be inspected safely.",
        ) from exc
    if path.is_symlink() or not path.is_file() or metadata.st_nlink != 1:
        raise StageRuntimeError(
            "stage.unsafe_path",
            f"The declared {label} must be one unaliased regular file.",
        )


def _write_candidate_payload(path: Path, payload: bytes) -> None:
    if path.exists() or path.is_symlink():
        _require_unaliased_regular_file(path, label="stage candidate")
        try:
            existing = path.read_bytes()
        except OSError as exc:
            raise StageRuntimeError(
                "stage.candidate_missing",
                "The existing stage candidate cannot be inspected safely.",
            ) from exc
        if existing != payload:
            raise StageRuntimeError(
                "stage.submission_conflict",
                "A different candidate has already been submitted for this task.",
            )
        return
    try:
        atomic_write_bytes(path, payload)
    except (StageStoreError, UnsafeStagePathError) as exc:
        raise StageRuntimeError(
            "stage.unsafe_path",
            "The stage candidate could not be written inside its isolated run path.",
        ) from exc


def _inspect_output_receipt(
    job: Path,
    record: StageRecord,
    adapter: StageAdapter,
) -> tuple[bool, tuple[str, ...]]:
    if not record.outputs:
        return False, ()
    receipt = next(
        (item for item in record.outputs if item.path == adapter.authoritative_target),
        None,
    )
    if receipt is None:
        return False, ("output_missing",)
    try:
        target = resolve_job_relative_path(job, adapter.authoritative_target)
    except UnsafeStagePathError as exc:
        raise StageRuntimeError(
            "stage.output_unreadable",
            "The authoritative stage output cannot be inspected safely.",
        ) from exc
    if not target.is_file():
        return False, ("output_missing",)
    try:
        actual_hash = sha256_file(target)
    except StageStoreError as exc:
        raise StageRuntimeError(
            "stage.output_unreadable",
            "The authoritative stage output cannot be inspected safely.",
        ) from exc
    if actual_hash != receipt.sha256:
        return True, ("output_drift",)
    return False, ()


def _inspect_pending_output_expectation(
    job: Path,
    task: TaskSpecV1,
    adapter: StageAdapter,
) -> tuple[bool, tuple[str, ...]]:
    if task.authoritative_target != adapter.authoritative_target:
        return True, ("task_contract_mismatch",)
    target = resolve_job_relative_path(job, adapter.authoritative_target)
    actual = _file_hash_or_none(target)
    if actual == task.expected_output_sha256:
        return False, ()
    return True, ("output_drift",)


def _state_path(job: Path) -> Path:
    return resolve_job_relative_path(job, "workflow/state.json")


def _load_or_reconstruct_state(job: Path) -> tuple[WorkflowStateV1, bool]:
    path = _state_path(job)
    if path.is_file() and not path.is_symlink():
        try:
            loaded = WorkflowStateV1.model_validate(read_json_object(path))
            rebuilt = _reconstruct_state(job)
            rebuilt_terminal_runs = {
                record.run_id
                for record in rebuilt.stages
                if record.run_id is not None
                and record.status in {"succeeded", "failed", "cancelled"}
            }
            loaded_runs = {
                record.run_id for record in loaded.stages if record.run_id is not None
            }
            terminal_evidence_supersedes_view = bool(
                loaded.active_run_id is not None
                and loaded.active_run_id in rebuilt_terminal_runs
            ) or bool(
                rebuilt.updated_at > loaded.updated_at
                and rebuilt_terminal_runs - loaded_runs
            )
            active_evidence_supersedes_view = (
                rebuilt.active_run_id != loaded.active_run_id
                and (
                    rebuilt.active_run_id is not None
                    or loaded.active_run_id is not None
                )
            )
            if terminal_evidence_supersedes_view or active_evidence_supersedes_view:
                return rebuilt, True
            return loaded, False
        except (StageStoreError, ValidationError):
            pass
    return _reconstruct_state(job), True


def _reconstruct_state(job: Path) -> WorkflowStateV1:
    manifests: list[RunManifestV1] = []
    implemented = {
        definition.id for definition in DEFAULT_STAGE_REGISTRY.implemented_stages()
    }
    runs_dir = job / "workflow" / "runs"
    if runs_dir.is_dir():
        for run_dir in sorted(path for path in runs_dir.iterdir() if path.is_dir()):
            path = run_dir / "manifest.json"
            manifest = None
            if path.is_file() and not path.is_symlink():
                try:
                    manifest = RunManifestV1.model_validate(read_json_object(path))
                except (StageStoreError, ValidationError):
                    manifest = None
            if manifest is None:
                manifest = _recoverable_manifest(job, run_dir)
            if manifest is None:
                continue
            if manifest.job_id == job.name and manifest.stage in implemented:
                manifests.append(manifest)
    latest_by_stage: dict[str, RunManifestV1] = {}
    latest_success_by_stage: dict[str, RunManifestV1] = {}
    for manifest in manifests:
        current = latest_by_stage.get(manifest.stage)
        if current is None or _manifest_order_key(manifest) > _manifest_order_key(current):
            latest_by_stage[manifest.stage] = manifest
        successful = latest_success_by_stage.get(manifest.stage)
        if manifest.status == "succeeded" and (
            successful is None
            or _manifest_order_key(manifest) > _manifest_order_key(successful)
        ):
            latest_success_by_stage[manifest.stage] = manifest

    eligible_pending: list[tuple[TaskSpecV1, Path]] = []
    for task, path in _all_pending_tasks(job):
        if task.stage not in implemented:
            continue
        latest = latest_by_stage.get(task.stage)
        if latest is not None and task.created_at <= latest.created_at:
            continue
        eligible_pending.append((task, path))
    active_task = (
        max(eligible_pending, key=lambda item: item[0].created_at)[0]
        if eligible_pending
        else None
    )

    if manifests or active_task is not None:
        records = {
            stage: _record_from_manifest(
                manifest,
                fallback_outputs=(
                    latest_success_by_stage[stage].promoted_outputs
                    if stage in latest_success_by_stage
                    else ()
                ),
            )
            for stage, manifest in latest_by_stage.items()
        }
        if active_task is not None:
            previous = records.get(active_task.stage)
            records[active_task.stage] = StageRecord(
                stage=active_task.stage,
                status="running",
                attempt_count=(previous.attempt_count if previous is not None else 0) + 1,
                run_id=active_task.run_id,
                input_fingerprint=active_task.input_fingerprint,
                inputs=active_task.inputs,
                outputs=previous.outputs if previous is not None else (),
                started_at=active_task.created_at,
            )
        _apply_reconstructed_dependency_staleness(records)
        order = {
            definition.id: index
            for index, definition in enumerate(DEFAULT_STAGE_REGISTRY.topological_order())
        }
        ordered_records = tuple(sorted(records.values(), key=lambda item: order[item.stage]))
        created_values = [item.created_at for item in manifests]
        updated_values = [
            item.completed_at or item.started_at or item.created_at
            for item in manifests
        ]
        if active_task is not None:
            created_values.append(active_task.created_at)
            updated_values.append(active_task.created_at)
        return WorkflowStateV1(
            job_id=job.name,
            revision=len(manifests) + len(eligible_pending),
            created_at=min(created_values),
            updated_at=max(updated_values),
            active_run_id=active_task.run_id if active_task is not None else None,
            stages=ordered_records,
        )

    now = _utc_now()
    return WorkflowStateV1(
        job_id=job.name,
        revision=0,
        created_at=now,
        updated_at=now,
        stages=(StageRecord(stage="parse", status="ready"),),
    )


def _manifest_order_key(manifest: RunManifestV1) -> tuple[datetime, str]:
    return manifest.created_at, manifest.run_id


def _record_from_manifest(
    manifest: RunManifestV1,
    *,
    fallback_outputs: tuple[ArtifactFingerprint, ...] = (),
) -> StageRecord:
    if manifest.status == "succeeded":
        return StageRecord(
            stage=manifest.stage,
            status="succeeded",
            attempt_count=manifest.attempt,
            run_id=manifest.run_id,
            input_fingerprint=manifest.input_fingerprint,
            inputs=manifest.inputs,
            outputs=manifest.promoted_outputs,
            started_at=manifest.started_at,
            completed_at=manifest.completed_at,
        )
    return StageRecord(
        stage=manifest.stage,
        status="failed" if manifest.status == "failed" else "cancelled",
        attempt_count=manifest.attempt,
        run_id=manifest.run_id,
        input_fingerprint=manifest.input_fingerprint,
        inputs=manifest.inputs,
        outputs=fallback_outputs,
        started_at=manifest.started_at,
        completed_at=manifest.completed_at,
        error_code=manifest.error_code if manifest.status == "failed" else None,
    )


def _apply_reconstructed_dependency_staleness(
    records: dict[str, StageRecord],
) -> None:
    for definition in DEFAULT_STAGE_REGISTRY.topological_order():
        record = records.get(definition.id)
        if record is None or record.status != "succeeded":
            continue
        for dependency in definition.depends_on:
            dependency_definition = DEFAULT_STAGE_REGISTRY.get(dependency)
            if not dependency_definition.implemented:
                continue
            dependency_record = records.get(dependency)
            dependency_output_path = _adapter(dependency).authoritative_target
            upstream_receipt = (
                next(
                    (
                        item
                        for item in dependency_record.outputs
                        if item.path == dependency_output_path
                    ),
                    None,
                )
                if dependency_record is not None
                else None
            )
            downstream_receipt = next(
                (item for item in record.inputs if item.path == dependency_output_path),
                None,
            )
            dependency_is_current = (
                dependency_record is not None
                and dependency_record.status == "succeeded"
                and upstream_receipt is not None
                and downstream_receipt is not None
                and upstream_receipt.sha256 == downstream_receipt.sha256
                and upstream_receipt.size_bytes == downstream_receipt.size_bytes
            )
            if not dependency_is_current:
                records[definition.id] = record.model_copy(update={"status": "stale"})
                break


def _stage_record(state: WorkflowStateV1, stage: str) -> StageRecord:
    for record in state.stages:
        if record.stage == stage:
            return record
    return StageRecord(stage=stage, status="ready")


def _state_with_stage(
    state: WorkflowStateV1,
    stage: StageRecord,
    *,
    active_run_id: str | None,
    increment_revision: bool = False,
    updated_at: datetime | None = None,
) -> WorkflowStateV1:
    records = [record for record in state.stages if record.stage != stage.stage]
    records.append(stage)
    order = {definition.id: index for index, definition in enumerate(DEFAULT_STAGE_REGISTRY.topological_order())}
    records.sort(key=lambda item: order[item.stage])
    return WorkflowStateV1(
        job_id=state.job_id,
        revision=state.revision + (1 if increment_revision else 0),
        created_at=state.created_at,
        updated_at=updated_at or state.updated_at,
        active_run_id=active_run_id,
        stages=tuple(records),
    )


def _with_stale_descendants(state: WorkflowStateV1, stage: str) -> WorkflowStateV1:
    descendant_ids = {definition.id for definition in DEFAULT_STAGE_REGISTRY.descendants(stage)}
    records = tuple(
        record.model_copy(update={"status": "stale"})
        if record.stage in descendant_ids and record.status == "succeeded"
        else record
        for record in state.stages
    )
    return state.model_copy(update={"stages": records})


def _write_state(job: Path, state: WorkflowStateV1) -> None:
    try:
        atomic_write_json(_state_path(job), state.model_dump(mode="json"))
    except StageStoreError as exc:
        raise StageRuntimeError(
            "stage.state_write_failed",
            "The workflow state view could not be updated safely.",
        ) from exc


def _all_pending_tasks(job: Path) -> list[tuple[TaskSpecV1, Path]]:
    pending: list[tuple[TaskSpecV1, Path]] = []
    runs_dir = job / "workflow" / "runs"
    if not runs_dir.is_dir():
        return pending
    for path in sorted(runs_dir.glob("*/task-spec.json")):
        if path.is_symlink() or (path.parent / "manifest.json").exists():
            continue
        try:
            task = TaskSpecV1.model_validate(read_json_object(path))
            expected_path = resolve_job_relative_path(
                job,
                f"workflow/runs/{task.run_id}/task-spec.json",
            )
            if path.resolve() != expected_path:
                continue
            definition = _implemented_stage(task.stage)
            adapter = _adapter(task.stage)
            _validate_task_contract(job, task, definition, adapter)
            _validate_task_preparation(job, path, task)
        except (StageRuntimeError, StageStoreError, ValidationError):
            continue
        if task.job_id == job.name:
            pending.append((task, path))
    return pending


def _pending_task_for_fingerprint(
    job: Path,
    *,
    stage: str,
    input_fingerprint: str,
    execution_mode: str | None,
) -> tuple[TaskSpecV1, Path] | None:
    candidates = [
        item
        for item in _all_pending_tasks(job)
        if item[0].stage == stage
        and item[0].input_fingerprint == input_fingerprint
        and (execution_mode is None or item[0].execution_mode == execution_mode)
    ]
    if not candidates:
        return None
    return max(candidates, key=lambda item: item[0].created_at)


def _pending_task_for_run(
    job: Path,
    run_id: str | None,
) -> tuple[TaskSpecV1, Path] | None:
    if run_id is None:
        return None
    return next(
        (item for item in _all_pending_tasks(job) if item[0].run_id == run_id),
        None,
    )


def _validated_supplied_path(job: Path, supplied: Path) -> tuple[str, Path]:
    try:
        raw = supplied.expanduser()
        if raw.is_absolute():
            relative = raw.resolve().relative_to(job.resolve()).as_posix()
        else:
            relative = raw.as_posix()
        path = resolve_job_relative_path(job, relative)
    except (OSError, RuntimeError, ValueError, UnsafeStagePathError) as exc:
        raise StageRuntimeError(
            "stage.unsafe_path",
            "A supplied stage path is outside the selected job.",
        ) from exc
    return relative, path


def _validate_task_contract(
    job: Path,
    task: TaskSpecV1,
    definition: StageDefinition,
    adapter: StageAdapter,
) -> None:
    expected_candidate = (
        f"workflow/runs/{task.run_id}/candidates/{adapter.candidate_name}"
    )
    expected_result = (
        f"workflow/runs/{task.run_id}/tasks/{task.task_id}/result.json"
    )
    expected_reads = tuple(item.path for item in task.inputs)
    expected_writes = (expected_candidate, expected_result)
    if (
        task.job_id != job.name
        or task.stage != adapter.stage_id
        or task.execution_mode not in definition.execution_modes
        or task.candidate_output != expected_candidate
        or task.result_output != expected_result
        or task.allowed_reads != expected_reads
        or task.allowed_writes != expected_writes
        or task.authoritative_target != adapter.authoritative_target
        or task.output_schema != adapter.output_schema
        or task.privacy_tier != adapter.task_privacy_tier
        or task.required_consents != adapter.required_consents(task.execution_mode)
    ):
        raise StageRuntimeError(
            "stage.task_contract_mismatch",
            "TaskSpec does not match the registered stage contract.",
        )


def _validate_task_preparation(
    job: Path,
    task_path: Path,
    task: TaskSpecV1,
) -> RunManifestV1:
    try:
        preparation = _load_preparation_for_run(job, task.run_id)
        task_spec_sha256 = sha256_file(task_path)
    except (StageRuntimeError, StageStoreError) as exc:
        raise StageRuntimeError(
            "stage.task_integrity_mismatch",
            "TaskSpec is not anchored by its immutable preparation receipt.",
        ) from exc
    if (
        preparation.status != "prepared"
        or preparation.run_id != task.run_id
        or preparation.task_id != task.task_id
        or preparation.job_id != task.job_id
        or preparation.stage != task.stage
        or preparation.attempt < 1
        or preparation.execution_mode != task.execution_mode
        or preparation.created_at != task.created_at
        or preparation.inputs != task.inputs
        or preparation.input_fingerprint != task.input_fingerprint
        or preparation.task_spec_sha256 != task_spec_sha256
    ):
        raise StageRuntimeError(
            "stage.task_integrity_mismatch",
            "TaskSpec does not match its immutable preparation receipt.",
        )
    return preparation


def _validate_task_freshness(
    workspace: Path,
    job: Path,
    task: TaskSpecV1,
    adapter: StageAdapter,
    accepted_output_sha256: str | None = None,
) -> Path:
    target = resolve_job_relative_path(job, task.authoritative_target)
    if (
        accepted_output_sha256 is not None
        and _terminal_claim_matches(
            job,
            task=task,
            action="promote",
            candidate_sha256=accepted_output_sha256,
        )
    ):
        actual = _file_hash_or_none(target)
        if actual in {task.expected_output_sha256, accepted_output_sha256}:
            return target
        raise StageRuntimeError(
            "stage.output_conflict",
            "The claimed promotion target no longer matches its transaction baseline.",
        )
    status = inspect_stage_status(
        workspace,
        job,
        stage=task.stage,  # type: ignore[arg-type]
    )
    if status.input_fingerprint is None:
        raise StageRuntimeError(
            "stage.dependency_not_current",
            "The stage dependencies changed after this task was prepared.",
        )
    current_fingerprint = adapter.input_fingerprint(workspace, job)
    if current_fingerprint != task.input_fingerprint:
        raise StageRuntimeError(
            "stage.stale_input",
            "The stage inputs changed after this task was prepared.",
        )
    try:
        current_inputs = adapter.input_artifacts(workspace, job)
    except (ConfirmStageError, ParseStageError, StageStoreError, OSError, ValueError) as exc:
        raise StageRuntimeError(
            "stage.stale_input",
            "The declared stage input receipts changed after this task was prepared.",
        ) from exc
    if current_inputs != task.inputs:
        raise StageRuntimeError(
            "stage.stale_input",
            "The declared stage input receipts changed after this task was prepared.",
        )
    actual_output_sha256 = _file_hash_or_none(target)
    accepted_claimed_output = (
        accepted_output_sha256 is not None
        and actual_output_sha256 == accepted_output_sha256
        and _terminal_claim_matches(
            job,
            task=task,
            action="promote",
            candidate_sha256=accepted_output_sha256,
        )
    )
    if (
        actual_output_sha256 != task.expected_output_sha256
        and not accepted_claimed_output
    ):
        raise StageRuntimeError(
            "stage.output_conflict",
            "The authoritative stage output changed after this task was prepared.",
        )
    return target


def _load_or_write_passed_validation(
    path: Path,
    *,
    task: TaskSpecV1,
    citations_valid: bool,
) -> ValidationReportV1:
    desired = ValidationReportV1(
        task_id=task.task_id,
        run_id=task.run_id,
        job_id=task.job_id,
        stage=task.stage,
        status="passed",
        checked_at=max(_utc_now(), task.created_at),
        input_hashes_match=True,
        schema_valid=True,
        scope_valid=True,
        citations_valid=citations_valid,
    )
    if path.exists() or path.is_symlink():
        try:
            existing = ValidationReportV1.model_validate(read_json_object(path))
        except (StageStoreError, ValidationError) as exc:
            raise StageRuntimeError(
                "stage.invalid_result",
                "Existing validation evidence is invalid.",
            ) from exc
        if existing.model_copy(update={"checked_at": desired.checked_at}) != desired:
            raise StageRuntimeError(
                "stage.invalid_result",
                "Existing validation evidence does not match this task.",
            )
        return existing
    write_immutable_json(path, desired.model_dump(mode="json"))
    return desired


def _validate_candidate_submission(
    job: Path,
    *,
    task_path: Path,
    task: TaskSpecV1,
    result_path: Path,
    result: TaskResultV1,
) -> CandidateSubmissionV1:
    submission_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{task.run_id}/submission.json",
    )
    try:
        submission = CandidateSubmissionV1.model_validate(
            read_json_object(submission_path)
        )
        result_sha256 = sha256_file(result_path)
        task_sha256 = sha256_file(task_path)
    except (StageStoreError, ValidationError) as exc:
        raise StageRuntimeError(
            "stage.submission_missing",
            "TaskResult is not anchored by a core-written candidate submission.",
        ) from exc
    if (
        submission.task_id != task.task_id
        or submission.run_id != task.run_id
        or submission.job_id != task.job_id
        or submission.stage != task.stage
        or submission.task_spec_sha256 != task_sha256
        or submission.result_path != task.result_output
        or submission.task_result_sha256 != result_sha256
        or result.outputs != (submission.candidate,)
    ):
        raise StageRuntimeError(
            "stage.submission_conflict",
            "Candidate submission does not match the TaskSpec and TaskResult.",
        )
    return submission


def _validate_active_task(
    job: Path,
    task: TaskSpecV1,
    adapter: StageAdapter,
) -> None:
    manifest_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{task.run_id}/manifest.json",
    )
    if manifest_path.exists():
        raise StageRuntimeError(
            "stage.task_not_active",
            "The stage task already has a terminal manifest.",
        )
    state, _ = _load_or_reconstruct_state(job)
    record = _stage_record(state, task.stage)
    if (
        state.active_run_id != task.run_id
        or record.status != "running"
        or record.run_id != task.run_id
        or record.input_fingerprint != task.input_fingerprint
    ):
        raise StageRuntimeError(
            "stage.task_not_active",
            "The stage task is no longer the active workflow task.",
        )
    baseline = next(
        (
            item
            for item in record.outputs
            if item.path == adapter.authoritative_target
        ),
        None,
    )
    baseline_sha256 = baseline.sha256 if baseline is not None else None
    if baseline_sha256 != task.expected_output_sha256:
        raise StageRuntimeError(
            "stage.task_integrity_mismatch",
            "The task output baseline does not match the active workflow receipt.",
        )


def _claim_terminal_action(
    job: Path,
    *,
    task: TaskSpecV1,
    task_path: Path,
    action: Literal["promote", "cancel"],
    candidate_sha256: str | None = None,
) -> Path:
    if action == "promote" and candidate_sha256 is None:
        raise StageRuntimeError(
            "stage.transition_conflict",
            "Promotion requires a validated candidate receipt.",
        )
    claim_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{task.run_id}/terminal-claim.json",
    )
    claim = {
        "schema_version": "1.0.0",
        "run_id": task.run_id,
        "task_id": task.task_id,
        "job_id": task.job_id,
        "stage": task.stage,
        "action": action,
        "task_spec_sha256": sha256_file(task_path),
        "candidate_sha256": candidate_sha256,
    }
    try:
        write_immutable_json(claim_path, claim)
    except ImmutableRecordError as exc:
        raise StageRuntimeError(
            "stage.transition_conflict",
            "Another terminal action already owns this stage run.",
        ) from exc
    except StageStoreError as exc:
        raise StageRuntimeError(
            "stage.store_failed",
            "The terminal stage action could not be claimed safely.",
        ) from exc
    return claim_path


def _terminal_claim_matches(
    job: Path,
    *,
    task: TaskSpecV1,
    action: Literal["promote", "cancel"],
    candidate_sha256: str | None,
) -> bool:
    claim_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{task.run_id}/terminal-claim.json",
    )
    if not claim_path.is_file() or claim_path.is_symlink():
        return False
    try:
        claim = read_json_object(claim_path)
        task_spec_sha256 = sha256_file(
            resolve_job_relative_path(
                job,
                f"workflow/runs/{task.run_id}/task-spec.json",
            )
        )
    except StageStoreError:
        return False
    return claim == {
        "schema_version": "1.0.0",
        "run_id": task.run_id,
        "task_id": task.task_id,
        "job_id": task.job_id,
        "stage": task.stage,
        "action": action,
        "task_spec_sha256": task_spec_sha256,
        "candidate_sha256": candidate_sha256,
    }


def _terminal_claim_action(
    job: Path,
    task: TaskSpecV1,
) -> Literal["promote", "cancel"] | None:
    claim_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{task.run_id}/terminal-claim.json",
    )
    if not claim_path.is_file() or claim_path.is_symlink():
        return None
    try:
        claim = read_json_object(claim_path)
    except StageStoreError:
        return None
    action = claim.get("action")
    candidate_sha256 = claim.get("candidate_sha256")
    if action == "cancel" and candidate_sha256 is None:
        return (
            "cancel"
            if _terminal_claim_matches(
                job,
                task=task,
                action="cancel",
                candidate_sha256=None,
            )
            else None
        )
    if (
        action == "promote"
        and isinstance(candidate_sha256, str)
        and len(candidate_sha256) == 64
    ):
        return (
            "promote"
            if _terminal_claim_matches(
                job,
                task=task,
                action="promote",
                candidate_sha256=candidate_sha256,
            )
            else None
        )
    return None


def _claimed_candidate_is_authoritative(
    job: Path,
    task: TaskSpecV1,
    adapter: StageAdapter,
) -> bool:
    claim_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{task.run_id}/terminal-claim.json",
    )
    try:
        claim = read_json_object(claim_path)
        candidate_sha256 = claim.get("candidate_sha256")
        target = resolve_job_relative_path(job, adapter.authoritative_target)
        return (
            isinstance(candidate_sha256, str)
            and _terminal_claim_matches(
                job,
                task=task,
                action="promote",
                candidate_sha256=candidate_sha256,
            )
            and _file_hash_or_none(target) == candidate_sha256
        )
    except (StageRuntimeError, StageStoreError):
        return False


def _load_preparation_for_run(
    job: Path,
    run_id: str | None,
) -> RunManifestV1:
    if run_id is None:
        raise StageRuntimeError(
            "stage.task_integrity_mismatch",
            "The active task has no run identifier.",
        )
    preparation_path = resolve_job_relative_path(
        job,
        f"workflow/runs/{run_id}/preparation.json",
    )
    try:
        return RunManifestV1.model_validate(read_json_object(preparation_path))
    except (StageStoreError, ValidationError) as exc:
        raise StageRuntimeError(
            "stage.task_integrity_mismatch",
            "The immutable preparation receipt is missing or invalid.",
        ) from exc


def _validate_task_result_identity(task: TaskSpecV1, result: TaskResultV1) -> None:
    if (
        result.task_id != task.task_id
        or result.run_id != task.run_id
        or result.job_id != task.job_id
        or result.stage != task.stage
    ):
        raise StageRuntimeError(
            "stage.result_identity_mismatch",
            "TaskResult identity does not match the immutable TaskSpec.",
        )
    if result.input_fingerprint != task.input_fingerprint:
        raise StageRuntimeError(
            "stage.stale_input",
            "TaskResult does not echo the prepared input fingerprint.",
        )
    if result.status != "succeeded":
        raise StageRuntimeError(
            "stage.execution_failed",
            "The stage executor returned a non-success result.",
        )


def _attempt_for_run(job: Path, run_id: str, stage: str) -> int:
    state, _ = _load_or_reconstruct_state(job)
    record = _stage_record(state, stage)
    if record.run_id == run_id and record.attempt_count > 0:
        return record.attempt_count
    return max(1, record.attempt_count + 1)


def _record_rejected_run(
    job: Path,
    task: TaskSpecV1,
    result: TaskResultV1 | None,
    *,
    error: StageRuntimeError,
) -> None:
    now = _utc_now()
    started = task.created_at
    completed = max(now, started)
    validation_relative = f"workflow/runs/{task.run_id}/validation/report.json"
    validation_path = resolve_job_relative_path(job, validation_relative)
    validation = ValidationReportV1(
        task_id=task.task_id,
        run_id=task.run_id,
        job_id=task.job_id,
        stage=task.stage,
        status="failed",
        checked_at=now,
        input_hashes_match=error.code not in {
            "stage.stale_input",
            "stage.dependency_not_current",
        },
        schema_valid=error.code != "stage.invalid_candidate",
        scope_valid=error.code not in {"stage.unsafe_path", "stage.result_scope_mismatch"},
        citations_valid=False if error.code == "stage.invalid_candidate" else None,
        errors=(error.code,),
    )
    try:
        write_immutable_json(validation_path, validation.model_dump(mode="json"))
        task_path = resolve_job_relative_path(
            job,
            f"workflow/runs/{task.run_id}/task-spec.json",
        )
        manifest = RunManifestV1(
            run_id=task.run_id,
            task_id=task.task_id,
            job_id=task.job_id,
            stage=task.stage,
            attempt=_attempt_for_run(job, task.run_id, task.stage),
            execution_mode=task.execution_mode,
            status="failed",
            created_at=task.created_at,
            started_at=started,
            completed_at=completed,
            inputs=task.inputs,
            input_fingerprint=task.input_fingerprint,
            task_spec_sha256=sha256_file(task_path),
            candidate_outputs=result.outputs if result is not None else (),
            validation_report_path=validation_relative,
            error_code=error.code,
            error_message=str(error),
        )
        manifest_path = resolve_job_relative_path(
            job,
            f"workflow/runs/{task.run_id}/manifest.json",
        )
        write_immutable_json(manifest_path, manifest.model_dump(mode="json"))
        state, _ = _load_or_reconstruct_state(job)
        previous_outputs = _stage_record(state, task.stage).outputs
        failed = StageRecord(
            stage=task.stage,
            status="failed",
            attempt_count=manifest.attempt,
            run_id=task.run_id,
            input_fingerprint=task.input_fingerprint,
            inputs=task.inputs,
            outputs=previous_outputs,
            started_at=started,
            completed_at=completed,
            error_code=error.code,
        )
        updated = _state_with_stage(
            state,
            failed,
            active_run_id=None,
            increment_revision=True,
            updated_at=completed,
        )
        _write_state(job, updated)
    except (StageStoreError, StageRuntimeError, ValidationError, OSError, ValueError):
        return


def _finalize_recoverable_promotions(job: Path) -> bool:
    runs_dir = job / "workflow" / "runs"
    if not runs_dir.is_dir():
        return False
    recovered = False
    for run_dir in sorted(path for path in runs_dir.iterdir() if path.is_dir()):
        manifest_path = run_dir / "manifest.json"
        if manifest_path.exists():
            continue
        manifest = _recoverable_manifest(job, run_dir)
        if manifest is None:
            continue
        try:
            write_immutable_json(manifest_path, manifest.model_dump(mode="json"))
        except StageStoreError as exc:
            raise StageRuntimeError(
                "stage.recovery_failed",
                "A promoted stage run could not be finalized safely.",
            ) from exc
        recovered = True
    if recovered:
        _write_state(job, _reconstruct_state(job))
    return recovered


def _recoverable_manifest(job: Path, run_dir: Path) -> RunManifestV1 | None:
    promotion_path = run_dir / "promotion.json"
    task_path = run_dir / "task-spec.json"
    if not promotion_path.is_file() or not task_path.is_file():
        return None
    try:
        promotion = read_json_object(promotion_path)
        task = TaskSpecV1.model_validate(read_json_object(task_path))
        definition = _implemented_stage(task.stage)
        adapter = _adapter(task.stage)
        _validate_task_contract(job, task, definition, adapter)
        _validate_task_preparation(job, task_path, task)
        result_path = resolve_job_relative_path(job, task.result_output)
        result = TaskResultV1.model_validate(read_json_object(result_path))
        _validate_task_result_identity(task, result)
        _validate_candidate_submission(
            job,
            task_path=task_path,
            task=task,
            result_path=result_path,
            result=result,
        )
        if len(result.outputs) != 1 or result.outputs[0].path != task.candidate_output:
            return None
        if promotion.get("run_id") != task.run_id or promotion.get("task_id") != task.task_id:
            return None
        if promotion.get("input_fingerprint") != task.input_fingerprint:
            return None
        if promotion.get("candidate_sha256") != result.outputs[0].sha256:
            return None
        if promotion.get("authoritative_target") != adapter.authoritative_target:
            return None
        target = resolve_job_relative_path(job, task.authoritative_target)
        promoted = _artifact(job, task.authoritative_target)
        if promoted.sha256 != promotion.get("authoritative_sha256"):
            return None
        attempt = promotion.get("attempt")
        if type(attempt) is not int or attempt < 1:
            return None
        validation_relative = f"workflow/runs/{task.run_id}/validation/report.json"
        validation_path = resolve_job_relative_path(job, validation_relative)
        validation = ValidationReportV1.model_validate(read_json_object(validation_path))
        if validation.status != "passed":
            return None
        return RunManifestV1(
            run_id=task.run_id,
            task_id=task.task_id,
            job_id=task.job_id,
            stage=task.stage,
            attempt=attempt,
            execution_mode=task.execution_mode,
            status="succeeded",
            created_at=task.created_at,
            started_at=task.created_at,
            completed_at=max(validation.checked_at, task.created_at),
            inputs=task.inputs,
            input_fingerprint=task.input_fingerprint,
            task_spec_sha256=sha256_file(task_path),
            candidate_outputs=result.outputs,
            promoted_outputs=(promoted,),
            validation_report_path=validation_relative,
        )
    except (
        IndexError,
        OSError,
        StageRuntimeError,
        StageStoreError,
        UnsafeStagePathError,
        ValidationError,
        ValueError,
    ):
        return None


def _utc_now() -> datetime:
    return datetime.now(UTC)
