from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
import json
from pathlib import Path
from typing import Literal
from uuid import uuid4

from pydantic import ValidationError

from canisend.stage_models import (
    ArtifactFingerprint,
    RunManifestV1,
    StageRecord,
    TaskResultV1,
    TaskSpecV1,
    ValidationReportV1,
    WorkflowStateV1,
)
from canisend.stage_registry import DEFAULT_STAGE_REGISTRY
from canisend.stage_store import (
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
from canisend.stages.parse_stage import (
    ParseStageError,
    ParseStageValidationError,
    build_deterministic_parse_candidate,
    parse_input_fingerprint,
    validate_parse_candidate,
)
from canisend.workspace import load_workspace_config


SupportedStage = Literal["parse"]
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
    input_fingerprint: str
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
class StageRunOutcome:
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
    _implemented_stage(stage)
    schema_path = _schema_path(root)
    try:
        current_fingerprint = parse_input_fingerprint(job, schema_path=schema_path)
    except (ParseStageError, OSError, ValueError) as exc:
        raise StageRuntimeError("stage.invalid_input", "Parse inputs are missing or invalid.") from exc

    state, reconstructed = _load_or_reconstruct_state(job)
    record = _stage_record(state, stage)
    reasons: list[str] = []
    output_drift = False

    if record.status in {"succeeded", "stale"}:
        if record.input_fingerprint != current_fingerprint:
            reasons.append("input_changed")
            if record.status != "stale":
                record = record.model_copy(update={"status": "stale"})

        receipt = next((item for item in record.outputs if item.path == "parsed_job.json"), None)
        target = job / "parsed_job.json"
        if receipt is not None:
            if not target.is_file():
                reasons.append("output_missing")
                if record.status != "stale":
                    record = record.model_copy(update={"status": "stale"})
            else:
                try:
                    actual_hash = sha256_file(target)
                except StageStoreError as exc:
                    raise StageRuntimeError(
                        "stage.output_unreadable",
                        "The authoritative Parse output cannot be inspected safely.",
                    ) from exc
                if actual_hash != receipt.sha256:
                    output_drift = True
                    reasons.append("output_drift")

    pending_task = _pending_task_for_fingerprint(
        job,
        stage=stage,
        input_fingerprint=current_fingerprint,
        execution_mode=None,
    )
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
    if execution_mode not in definition.execution_modes:
        raise StageRuntimeError(
            "stage.unsupported_mode",
            "The requested execution mode is not supported for this stage.",
        )

    _finalize_recoverable_promotions(job)

    status = inspect_stage_status(root, job, stage=stage)
    if status.output_drift:
        raise StageRuntimeError(
            "stage.output_conflict",
            "The authoritative Parse output has changed since its last successful run.",
        )
    if status.stage.status == "succeeded" and not status.reasons:
        raise StageRuntimeError("stage.already_current", "The Parse stage is already current.")

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

    now = _utc_now()
    run_id = f"run_{uuid4().hex}"
    task_id = f"task_{uuid4().hex}"
    run_root = f"workflow/runs/{run_id}"
    candidate_output = f"{run_root}/candidates/parsed_job.json"
    result_output = f"{run_root}/tasks/{task_id}/result.json"
    inputs = _parse_input_artifacts(job)
    expected_output = _file_hash_or_none(job / "parsed_job.json")
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
        authoritative_target="parsed_job.json",
        expected_output_sha256=expected_output,
        output_schema="canisend.parsed-job/v1",
        privacy_tier=2,
        required_consents=("read-full-job-advert",) if execution_mode == "host_agent" else (),
    )
    task_path = resolve_job_relative_path(job, f"{run_root}/task-spec.json")
    candidate_path = resolve_job_relative_path(job, candidate_output)
    result_path = resolve_job_relative_path(job, result_output)
    candidate_path.parent.mkdir(parents=True, exist_ok=True)
    result_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        write_immutable_json(task_path, task.model_dump(mode="json"))
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
    try:
        task_relative, task_path = _validated_supplied_path(job, task_spec_path)
        task = TaskSpecV1.model_validate(read_json_object(task_path))
        expected_task_relative = f"workflow/runs/{task.run_id}/task-spec.json"
        if task_relative != expected_task_relative:
            raise StageRuntimeError("stage.task_identity_mismatch", "TaskSpec path does not match its run.")
        _implemented_stage(task.stage)

        result_relative, result_path = _validated_supplied_path(job, task_result_path)
        if result_relative != task.result_output:
            raise StageRuntimeError(
                "stage.result_identity_mismatch",
                "TaskResult path does not match the immutable task contract.",
            )
        result = TaskResultV1.model_validate(read_json_object(result_path))
        _validate_task_result_identity(task, result)

        schema_path = _schema_path(root)
        current_fingerprint = parse_input_fingerprint(
            job,
            executor_mode=task.execution_mode,
            schema_path=schema_path,
        )
        if current_fingerprint != task.input_fingerprint:
            raise StageRuntimeError(
                "stage.stale_input",
                "The Parse inputs changed after this task was prepared.",
            )

        target = resolve_job_relative_path(job, task.authoritative_target)
        if _file_hash_or_none(target) != task.expected_output_sha256:
            raise StageRuntimeError(
                "stage.output_conflict",
                "The authoritative Parse output changed after this task was prepared.",
            )

        if len(result.outputs) != 1 or result.outputs[0].path != task.candidate_output:
            raise StageRuntimeError(
                "stage.result_scope_mismatch",
                "TaskResult does not contain exactly the declared Parse candidate.",
            )
        candidate = result.outputs[0]
        try:
            candidate_path = resolve_job_relative_path(job, candidate.path)
        except UnsafeStagePathError as exc:
            raise StageRuntimeError(
                "stage.unsafe_path",
                "The Parse candidate path is outside its declared staging scope.",
            ) from exc
        declared_candidate = resolve_job_relative_path(job, task.candidate_output)
        if candidate_path != declared_candidate:
            raise StageRuntimeError(
                "stage.result_scope_mismatch",
                "TaskResult candidate does not match the declared staging path.",
            )
        try:
            candidate_bytes = candidate_path.read_bytes()
        except OSError as exc:
            raise StageRuntimeError(
                "stage.candidate_missing",
                "The declared Parse candidate cannot be read.",
            ) from exc
        if sha256_bytes(candidate_bytes) != candidate.sha256 or (
            candidate.size_bytes is not None and len(candidate_bytes) != candidate.size_bytes
        ):
            raise StageRuntimeError(
                "stage.candidate_hash_mismatch",
                "The Parse candidate does not match its declared fingerprint.",
            )
        try:
            candidate_object = json.loads(candidate_bytes)
        except (UnicodeError, json.JSONDecodeError) as exc:
            raise StageRuntimeError(
                "stage.invalid_candidate",
                "The Parse candidate is not valid UTF-8 JSON.",
            ) from exc
        try:
            advert_text = (job / "job_advert.md").read_text(encoding="utf-8")
            validate_parse_candidate(
                candidate_object,
                advert_text=advert_text,
                schema_path=schema_path,
            )
        except (OSError, UnicodeError, ParseStageValidationError) as exc:
            raise StageRuntimeError(
                "stage.invalid_candidate",
                "The Parse candidate failed schema, semantic, or source-receipt validation.",
            ) from exc

        checked_at = _utc_now()
        validation = ValidationReportV1(
            task_id=task.task_id,
            run_id=task.run_id,
            job_id=task.job_id,
            stage=task.stage,
            status="passed",
            checked_at=checked_at,
            input_hashes_match=True,
            schema_valid=True,
            scope_valid=True,
            citations_valid=True,
        )
        validation_relative = f"workflow/runs/{task.run_id}/validation/report.json"
        validation_path = resolve_job_relative_path(job, validation_relative)
        write_immutable_json(validation_path, validation.model_dump(mode="json"))

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
                "attempt": _attempt_for_run(job, task.run_id),
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
            attempt=_attempt_for_run(job, task.run_id),
            execution_mode=task.execution_mode,
            status="succeeded",
            created_at=task.created_at,
            started_at=result.started_at,
            completed_at=result.completed_at,
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
            started_at=result.started_at,
            completed_at=result.completed_at,
        )
        updated_state = _state_with_stage(
            state,
            succeeded,
            active_run_id=None,
            increment_revision=True,
            updated_at=result.completed_at,
        )
        _write_state(job, updated_state)
        return AppliedStage(
            manifest=manifest,
            manifest_path=manifest_path,
            state=updated_state,
            authoritative_path=target,
        )
    except StageRuntimeError as exc:
        if task is not None:
            _record_rejected_run(job, task, result, error=exc)
        raise
    except (StageStoreError, ValidationError, ParseStageError, OSError, ValueError) as exc:
        wrapped = StageRuntimeError(
            "stage.invalid_result",
            "The stage result could not be loaded or validated safely.",
        )
        if task is not None:
            _record_rejected_run(job, task, result, error=wrapped)
        raise wrapped from exc


def run_deterministic_stage(
    workspace: Path,
    job_dir: Path,
    *,
    stage: SupportedStage,
) -> StageRunOutcome:
    root, job = _runtime_paths(workspace, job_dir)
    _finalize_recoverable_promotions(job)
    status = inspect_stage_status(root, job, stage=stage)
    target = job / "parsed_job.json"
    if status.output_drift:
        raise StageRuntimeError(
            "stage.output_conflict",
            "The authoritative Parse output has changed since its last successful run.",
        )
    if status.stage.status == "succeeded" and not status.reasons:
        return StageRunOutcome(
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
    candidate = build_deterministic_parse_candidate(job)
    candidate_bytes = (
        json.dumps(candidate, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")
    atomic_write_bytes(prepared.candidate_path, candidate_bytes)
    completed = _utc_now()
    if completed <= prepared.task_spec.created_at:
        completed = prepared.task_spec.created_at
    result = TaskResultV1(
        task_id=prepared.task_spec.task_id,
        run_id=prepared.task_spec.run_id,
        job_id=prepared.task_spec.job_id,
        stage=stage,
        status="succeeded",
        input_fingerprint=prepared.task_spec.input_fingerprint,
        started_at=prepared.task_spec.created_at,
        completed_at=completed,
        outputs=(
            ArtifactFingerprint(
                path=prepared.task_spec.candidate_output,
                sha256=sha256_bytes(candidate_bytes),
                size_bytes=len(candidate_bytes),
            ),
        ),
    )
    write_immutable_json(prepared.result_path, result.model_dump(mode="json"))
    applied = apply_stage_result(
        root,
        job,
        task_spec_path=prepared.task_spec_path,
        task_result_path=prepared.result_path,
    )
    return StageRunOutcome(
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


def _schema_path(workspace: Path) -> Path:
    return load_workspace_config(workspace).path("schema_dir") / "parsed_job.schema.json"


def _parse_input_artifacts(job: Path) -> tuple[ArtifactFingerprint, ...]:
    return (
        _artifact(job, "job.yaml"),
        _artifact(job, "job_advert.md"),
    )


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


def _state_path(job: Path) -> Path:
    return resolve_job_relative_path(job, "workflow/state.json")


def _load_or_reconstruct_state(job: Path) -> tuple[WorkflowStateV1, bool]:
    path = _state_path(job)
    if path.is_file() and not path.is_symlink():
        try:
            loaded = WorkflowStateV1.model_validate(read_json_object(path))
            rebuilt = _reconstruct_state(job)
            loaded_stage = _stage_record(loaded, "parse")
            rebuilt_stage = _stage_record(rebuilt, "parse")
            terminal_evidence_supersedes_view = (
                loaded.active_run_id is not None
                and rebuilt_stage.run_id == loaded.active_run_id
                and rebuilt_stage.status in {"succeeded", "failed", "cancelled"}
            )
            if terminal_evidence_supersedes_view:
                return rebuilt, True
            return loaded, False
        except (StageStoreError, ValidationError):
            pass
    return _reconstruct_state(job), True


def _reconstruct_state(job: Path) -> WorkflowStateV1:
    manifests: list[RunManifestV1] = []
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
            if manifest.job_id == job.name and manifest.stage == "parse":
                manifests.append(manifest)
    if manifests:
        manifest = max(
            manifests,
            key=lambda item: item.completed_at or item.started_at or item.created_at,
        )
        if manifest.status == "succeeded":
            record = StageRecord(
                stage="parse",
                status="succeeded",
                attempt_count=manifest.attempt,
                run_id=manifest.run_id,
                input_fingerprint=manifest.input_fingerprint,
                inputs=manifest.inputs,
                outputs=manifest.promoted_outputs,
                started_at=manifest.started_at,
                completed_at=manifest.completed_at,
            )
        else:
            record = StageRecord(
                stage="parse",
                status="failed" if manifest.status == "failed" else "cancelled",
                attempt_count=manifest.attempt,
                run_id=manifest.run_id,
                input_fingerprint=manifest.input_fingerprint,
                inputs=manifest.inputs,
                started_at=manifest.started_at,
                completed_at=manifest.completed_at,
                error_code=manifest.error_code if manifest.status == "failed" else None,
            )
        created = min(item.created_at for item in manifests)
        updated = manifest.completed_at or manifest.created_at
        return WorkflowStateV1(
            job_id=job.name,
            revision=len(manifests),
            created_at=created,
            updated_at=updated,
            stages=(record,),
        )

    pending = _all_pending_tasks(job)
    if pending:
        task, _ = max(pending, key=lambda item: item[0].created_at)
        record = StageRecord(
            stage=task.stage,
            status="running",
            attempt_count=1,
            run_id=task.run_id,
            input_fingerprint=task.input_fingerprint,
            inputs=task.inputs,
            started_at=task.created_at,
        )
        return WorkflowStateV1(
            job_id=job.name,
            revision=1,
            created_at=task.created_at,
            updated_at=task.created_at,
            active_run_id=task.run_id,
            stages=(record,),
        )

    now = _utc_now()
    return WorkflowStateV1(
        job_id=job.name,
        revision=0,
        created_at=now,
        updated_at=now,
        stages=(StageRecord(stage="parse", status="ready"),),
    )


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
        except (StageStoreError, ValidationError):
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
        and _file_hash_or_none(job / item[0].authoritative_target)
        == item[0].expected_output_sha256
    ]
    if not candidates:
        return None
    return max(candidates, key=lambda item: item[0].created_at)


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


def _attempt_for_run(job: Path, run_id: str) -> int:
    state, _ = _load_or_reconstruct_state(job)
    record = _stage_record(state, "parse")
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
    started = result.started_at if result is not None else task.created_at
    completed = result.completed_at if result is not None else now
    if completed < started:
        completed = started
    validation_relative = f"workflow/runs/{task.run_id}/validation/report.json"
    validation_path = resolve_job_relative_path(job, validation_relative)
    validation = ValidationReportV1(
        task_id=task.task_id,
        run_id=task.run_id,
        job_id=task.job_id,
        stage=task.stage,
        status="failed",
        checked_at=now,
        input_hashes_match=error.code != "stage.stale_input",
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
            attempt=_attempt_for_run(job, task.run_id),
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
        failed = StageRecord(
            stage=task.stage,
            status="failed",
            attempt_count=manifest.attempt,
            run_id=task.run_id,
            input_fingerprint=task.input_fingerprint,
            inputs=task.inputs,
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
                "A promoted Parse run could not be finalized safely.",
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
        result_path = resolve_job_relative_path(job, task.result_output)
        result = TaskResultV1.model_validate(read_json_object(result_path))
        _validate_task_result_identity(task, result)
        if promotion.get("run_id") != task.run_id or promotion.get("task_id") != task.task_id:
            return None
        if promotion.get("input_fingerprint") != task.input_fingerprint:
            return None
        if promotion.get("candidate_sha256") != result.outputs[0].sha256:
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
        ValidationReportV1.model_validate(read_json_object(validation_path))
        return RunManifestV1(
            run_id=task.run_id,
            task_id=task.task_id,
            job_id=task.job_id,
            stage=task.stage,
            attempt=attempt,
            execution_mode=task.execution_mode,
            status="succeeded",
            created_at=task.created_at,
            started_at=result.started_at,
            completed_at=result.completed_at,
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
