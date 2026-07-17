from __future__ import annotations

from concurrent.futures import FIRST_COMPLETED, Future, ThreadPoolExecutor, wait
from dataclasses import dataclass
from datetime import UTC, datetime
import glob
import json
from pathlib import Path
import re
import shlex
import shutil
import subprocess
from typing import Any
from uuid import uuid4

import yaml

from canisend.stage_models import TaskSpecV1
from canisend.stage_registry import DEFAULT_STAGE_REGISTRY
from canisend.stage_runtime import (
    PreparedStage,
    StageRuntimeError,
    apply_stage_result,
    cancel_stage_task,
    inspect_stage_status,
    prepare_stage,
    submit_stage_candidate,
)
from canisend.stage_store import resolve_job_relative_path


SAFE_SEGMENT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]*$")
PROMPT_MODES = {"arg", "none", "stdin"}
WORKER_KIND_ALIASES = {"antigravity": "agy"}
WORKER_PRESETS = {
    "codex": {"command": "codex exec", "prompt_mode": "stdin"},
    "claude": {"command": "claude", "prompt_mode": "arg"},
    "agy": {"command": "agy --print", "prompt_mode": "arg"},
}


class OrchestrationError(ValueError):
    pass


@dataclass(frozen=True)
class WorkerConfig:
    name: str
    command: str
    kind: str = "custom"
    prompt_mode: str = "stdin"
    max_parallel_tasks: int = 1
    supports_native_subagents: bool = False
    privacy_tier_limit: int = 1
    timeout_seconds: int = 300


@dataclass(frozen=True)
class RegisteredStageTask:
    stage: str
    document_id: str | None = None


@dataclass(frozen=True)
class OrchestrationTask:
    id: str
    worker: str
    role: str
    privacy_tier: int = 1
    inputs: tuple[str, ...] = ()
    outputs: tuple[str, ...] = ()
    writes: tuple[str, ...] = ()
    depends_on: tuple[str, ...] = ()
    agent_count: int = 1
    edits_profile_input: bool = False
    registered_stage: RegisteredStageTask | None = None


@dataclass(frozen=True)
class OrchestrationPlan:
    workers: dict[str, WorkerConfig]
    tasks: dict[str, OrchestrationTask]


@dataclass(frozen=True)
class OrchestrationResult:
    ok: bool
    run_dir: Path | None
    dry_run: bool
    task_statuses: dict[str, str]


@dataclass(frozen=True)
class TaskExecution:
    task_id: str
    status: str
    exit_code: int | None


def load_orchestration_plan(path: Path) -> OrchestrationPlan:
    try:
        raw = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        raise OrchestrationError(f"Invalid orchestration YAML: {exc}") from exc

    if not isinstance(raw, dict):
        raise OrchestrationError("Orchestration plan must be a mapping")

    workers = _parse_workers(raw.get("workers", {}))
    tasks = _parse_tasks(raw.get("tasks", []), workers)
    _validate_dependencies(tasks)
    _validate_write_conflicts(tasks)
    _validate_profile_input_edit_declarations(tasks)
    return OrchestrationPlan(workers=workers, tasks=tasks)


def run_orchestration(
    *,
    workspace: Path,
    job_dir: Path,
    plan_path: Path,
    dry_run: bool = False,
    allow_private_sources: bool = False,
    allow_provider_backed: bool = False,
    allow_profile_input_edits: bool = False,
    profile_input_edit_confirmations: int = 0,
    fail_fast: bool = False,
    run_id: str | None = None,
) -> OrchestrationResult:
    plan = load_orchestration_plan(plan_path)
    _validate_privacy_flags(
        plan,
        allow_private_sources=allow_private_sources,
        allow_provider_backed=allow_provider_backed,
    )
    _validate_profile_input_edit_flags(
        plan,
        allow_profile_input_edits=allow_profile_input_edits,
        profile_input_edit_confirmations=profile_input_edit_confirmations,
    )
    _validate_inputs(plan, workspace=workspace, job_dir=job_dir)
    if dry_run:
        task_statuses = _dry_run_task_statuses(plan, workspace=workspace, job_dir=job_dir)
        return OrchestrationResult(
            ok=all(status in {"current", "ready"} for status in task_statuses.values()),
            run_dir=None,
            dry_run=True,
            task_statuses=task_statuses,
        )
    return _execute_plan(
        plan,
        workspace=workspace,
        job_dir=job_dir,
        plan_path=plan_path,
        fail_fast=fail_fast,
        run_id=run_id,
    )


def _parse_workers(raw_workers: Any) -> dict[str, WorkerConfig]:
    if not isinstance(raw_workers, dict) or not raw_workers:
        raise OrchestrationError("Orchestration plan must define workers")

    workers: dict[str, WorkerConfig] = {}
    for name, raw_worker in raw_workers.items():
        if not isinstance(raw_worker, dict):
            raise OrchestrationError(f"Worker {name!r} must be a mapping")

        kind = _worker_kind(raw_worker, str(name))
        preset = WORKER_PRESETS.get(kind, {})
        command = str(raw_worker.get("command") or preset.get("command", "")).strip()
        if not command:
            raise OrchestrationError(f"Worker {name!r} must define command")
        prompt_mode = str(raw_worker.get("prompt_mode") or preset.get("prompt_mode", "stdin")).strip().lower()
        if prompt_mode not in PROMPT_MODES:
            allowed = ", ".join(sorted(PROMPT_MODES))
            raise OrchestrationError(f"Worker {name!r} prompt_mode must be one of: {allowed}")

        max_parallel_tasks = _int_field(raw_worker, "max_parallel_tasks", default=1)
        if max_parallel_tasks < 1:
            raise OrchestrationError(f"Worker {name!r} max_parallel_tasks must be at least 1")

        privacy_tier_limit = _privacy_tier(raw_worker.get("privacy_tier_limit", 1), f"worker {name!r}")
        timeout_seconds = _int_field(raw_worker, "timeout_seconds", default=300)
        if timeout_seconds < 1:
            raise OrchestrationError(f"Worker {name!r} timeout_seconds must be at least 1")

        workers[str(name)] = WorkerConfig(
            name=str(name),
            command=command,
            kind=kind,
            prompt_mode=prompt_mode,
            max_parallel_tasks=max_parallel_tasks,
            supports_native_subagents=bool(raw_worker.get("supports_native_subagents", False)),
            privacy_tier_limit=privacy_tier_limit,
            timeout_seconds=timeout_seconds,
        )
    return workers


def _worker_kind(raw_worker: dict[str, Any], worker_name: str) -> str:
    raw_kind = raw_worker.get("kind")
    if raw_kind is None:
        return "custom"
    kind = str(raw_kind).strip().lower()
    kind = WORKER_KIND_ALIASES.get(kind, kind)
    if kind == "custom" or kind in WORKER_PRESETS:
        return kind
    supported = ", ".join(sorted([*WORKER_PRESETS, "antigravity", "custom"]))
    raise OrchestrationError(f"Unknown worker kind for {worker_name!r}: {raw_kind!r}. Supported kinds: {supported}")


def _parse_tasks(raw_tasks: Any, workers: dict[str, WorkerConfig]) -> dict[str, OrchestrationTask]:
    if not isinstance(raw_tasks, list) or not raw_tasks:
        raise OrchestrationError("Orchestration plan must define tasks")

    tasks: dict[str, OrchestrationTask] = {}
    for raw_task in raw_tasks:
        if not isinstance(raw_task, dict):
            raise OrchestrationError("Each orchestration task must be a mapping")

        task_id = str(raw_task.get("id", "")).strip()
        if not task_id:
            raise OrchestrationError("Task id is required")
        _validate_safe_segment(task_id, label="Task id")
        if task_id in tasks:
            raise OrchestrationError(f"Duplicate task id: {task_id}")

        worker_name = str(raw_task.get("worker", "")).strip()
        if worker_name not in workers:
            raise OrchestrationError(f"Task {task_id!r} references unknown worker: {worker_name}")

        role = str(raw_task.get("role", "")).strip()
        if not role:
            raise OrchestrationError(f"Task {task_id!r} must define role")

        registered_stage = _parse_registered_stage(raw_task, task_id=task_id)
        default_privacy_tier = 2 if registered_stage is not None else 1
        privacy_tier = _privacy_tier(
            raw_task.get("privacy_tier", default_privacy_tier),
            f"task {task_id!r}",
        )
        if registered_stage is not None and privacy_tier < 2:
            raise OrchestrationError(
                f"Registered stage task {task_id!r} requires privacy tier 2 or higher"
            )
        if privacy_tier > workers[worker_name].privacy_tier_limit:
            raise OrchestrationError(
                f"Task {task_id!r} privacy tier exceeds worker {worker_name!r} privacy tier limit"
            )

        inputs = _normalized_relative_paths(raw_task, "inputs", f"task {task_id!r} inputs")
        outputs = _normalized_relative_paths(raw_task, "outputs", f"task {task_id!r} outputs")
        writes = _normalized_relative_paths(raw_task, "writes", f"task {task_id!r} writes") or outputs
        edits_profile_input = bool(raw_task.get("edits_profile_input", False))
        if registered_stage is not None and (inputs or outputs or writes or edits_profile_input):
            raise OrchestrationError(
                f"Registered stage task {task_id!r} must not declare inputs, outputs, writes, "
                "or profile input edits; its immutable TaskSpec owns that authority"
            )
        task = OrchestrationTask(
            id=task_id,
            worker=worker_name,
            role=role,
            privacy_tier=privacy_tier,
            inputs=inputs,
            outputs=outputs,
            writes=writes,
            depends_on=_tuple_field(raw_task, "depends_on"),
            agent_count=_agent_count(raw_task.get("agent_count", 1), task_id),
            edits_profile_input=edits_profile_input,
            registered_stage=registered_stage,
        )
        tasks[task_id] = task
    return tasks


def _parse_registered_stage(
    raw_task: dict[str, Any],
    *,
    task_id: str,
) -> RegisteredStageTask | None:
    raw_contract = raw_task.get("registered_stage")
    if raw_contract is None:
        return None
    if not isinstance(raw_contract, dict):
        raise OrchestrationError(f"Task {task_id!r} registered_stage must be a mapping")
    unexpected = set(raw_contract).difference({"stage", "document_id"})
    if unexpected:
        fields = ", ".join(sorted(unexpected))
        raise OrchestrationError(
            f"Task {task_id!r} registered_stage has unsupported fields: {fields}"
        )
    stage = str(raw_contract.get("stage", "")).strip()
    if not stage:
        raise OrchestrationError(f"Task {task_id!r} registered_stage.stage is required")
    try:
        definition = DEFAULT_STAGE_REGISTRY.get(stage)
    except KeyError as exc:
        raise OrchestrationError(
            f"Task {task_id!r} references unknown registered stage: {stage}"
        ) from exc
    if not definition.implemented or definition.execution_kind != "task":
        raise OrchestrationError(
            f"Task {task_id!r} registered stage {stage!r} is not an executable task stage"
        )
    if "host_agent" not in definition.execution_modes:
        raise OrchestrationError(
            f"Task {task_id!r} registered stage {stage!r} does not support host-agent execution"
        )

    raw_document_id = raw_contract.get("document_id")
    document_id = None if raw_document_id is None else str(raw_document_id).strip()
    if document_id == "":
        raise OrchestrationError(
            f"Task {task_id!r} registered_stage.document_id must not be empty"
        )
    if document_id is not None and not re.fullmatch(r"document_[0-9a-f]{32}", document_id):
        raise OrchestrationError(
            f"Task {task_id!r} registered_stage.document_id is invalid"
        )
    if stage != "draft" and document_id is not None:
        raise OrchestrationError(
            f"Task {task_id!r} registered stage {stage!r} does not accept document_id"
        )
    return RegisteredStageTask(stage=stage, document_id=document_id)


def _validate_dependencies(tasks: dict[str, OrchestrationTask]) -> None:
    for task in tasks.values():
        for dependency in task.depends_on:
            if dependency not in tasks:
                raise OrchestrationError(f"Task {task.id!r} references unknown dependency: {dependency}")

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(task_id: str, stack: tuple[str, ...]) -> None:
        if task_id in visiting:
            cycle = " -> ".join((*stack, task_id))
            raise OrchestrationError(f"Dependency cycle detected: {cycle}")
        if task_id in visited:
            return
        visiting.add(task_id)
        for dependency in tasks[task_id].depends_on:
            visit(dependency, (*stack, task_id))
        visiting.remove(task_id)
        visited.add(task_id)

    for task_id in tasks:
        visit(task_id, ())


def _validate_write_conflicts(tasks: dict[str, OrchestrationTask]) -> None:
    task_list = list(tasks.values())
    for index, first in enumerate(task_list):
        first_writes = _task_write_set(first)
        if not first_writes:
            continue
        for second in task_list[index + 1 :]:
            shared = first_writes.intersection(_task_write_set(second))
            if not shared:
                continue
            if _depends_on(tasks, first.id, second.id) or _depends_on(tasks, second.id, first.id):
                continue
            paths = ", ".join(sorted(shared))
            raise OrchestrationError(
                f"Task {first.id!r} and task {second.id!r} have a write conflict: {paths}"
            )


def _validate_profile_input_edit_declarations(tasks: dict[str, OrchestrationTask]) -> None:
    for task in tasks.values():
        writes_profile_input = any(_is_profile_input_path(path) for path in task.writes)
        if writes_profile_input and not task.edits_profile_input:
            raise OrchestrationError(
                f"Task {task.id!r} writes profile input files and must set edits_profile_input: true"
            )
        if task.edits_profile_input:
            if not writes_profile_input:
                raise OrchestrationError(
                    f"Task {task.id!r} sets edits_profile_input but does not write a profile input path"
                )
            if task.privacy_tier < 2:
                raise OrchestrationError(f"Task {task.id!r} profile input edits require privacy tier 2 or higher")
            if not task.depends_on:
                raise OrchestrationError(
                    f"Task {task.id!r} profile input edits must depend on at least one prior review task"
                )


def _validate_profile_input_edit_flags(
    plan: OrchestrationPlan,
    *,
    allow_profile_input_edits: bool,
    profile_input_edit_confirmations: int,
) -> None:
    profile_edit_tasks = [task for task in plan.tasks.values() if task.edits_profile_input]
    if not profile_edit_tasks:
        return
    if not allow_profile_input_edits:
        raise OrchestrationError("Profile input edit tasks require --allow-profile-input-edits")
    if profile_input_edit_confirmations < 2:
        raise OrchestrationError("Profile input edit tasks require two profile input edit confirmations")


def _is_profile_input_path(value: str) -> bool:
    parts = Path(value).parts
    if not parts or parts[0] != "profile":
        return False
    return len(parts) == 1 or parts[1] != "generated"


def _depends_on(tasks: dict[str, OrchestrationTask], task_id: str, candidate_dependency: str) -> bool:
    task = tasks[task_id]
    if candidate_dependency in task.depends_on:
        return True
    return any(_depends_on(tasks, dependency, candidate_dependency) for dependency in task.depends_on)


def _validate_privacy_flags(
    plan: OrchestrationPlan,
    *,
    allow_private_sources: bool,
    allow_provider_backed: bool,
) -> None:
    max_privacy_tier = max((task.privacy_tier for task in plan.tasks.values()), default=0)
    if max_privacy_tier >= 2 and not allow_private_sources:
        raise OrchestrationError("Tier 2 tasks require --allow-private-sources")
    if max_privacy_tier >= 3 and not allow_provider_backed:
        raise OrchestrationError("Tier 3 tasks require --allow-provider-backed")


def _validate_inputs(plan: OrchestrationPlan, *, workspace: Path, job_dir: Path) -> None:
    for task in plan.tasks.values():
        for input_path in task.inputs:
            if not _resolve_input_paths(input_path, workspace=workspace, job_dir=job_dir):
                raise OrchestrationError(f"Task {task.id!r} missing input: {input_path}")


def _dry_run_task_statuses(
    plan: OrchestrationPlan,
    *,
    workspace: Path,
    job_dir: Path,
) -> dict[str, str]:
    statuses: dict[str, str] = {}
    for task_id, task in plan.tasks.items():
        contract = task.registered_stage
        if contract is None:
            statuses[task_id] = "ready"
            continue
        try:
            inspection = inspect_stage_status(
                workspace,
                job_dir,
                stage=contract.stage,  # type: ignore[arg-type]
                document_id=contract.document_id,
            )
        except StageRuntimeError as exc:
            raise OrchestrationError(
                f"Registered stage task {task_id!r} cannot be inspected ({exc.code}): {exc}"
            ) from exc
        if (
            inspection.stage.status == "succeeded"
            and not inspection.reasons
            and not inspection.output_drift
        ):
            statuses[task_id] = "current"
        elif (
            inspection.input_fingerprint is None
            or inspection.output_drift
            or inspection.stage.status == "blocked"
        ):
            statuses[task_id] = "blocked"
        else:
            statuses[task_id] = "ready"
    return statuses


def _resolve_input_paths(input_path: str, *, workspace: Path, job_dir: Path) -> list[Path]:
    path = Path(input_path)
    if glob.has_magic(input_path):
        matches = [*job_dir.glob(input_path), *workspace.glob(input_path)]
        return sorted({match.resolve() for match in matches if match.exists()})

    job_path = job_dir / path
    if job_path.exists():
        return [job_path]
    workspace_path = workspace / path
    if workspace_path.exists():
        return [workspace_path]
    return []


def _execute_plan(
    plan: OrchestrationPlan,
    *,
    workspace: Path,
    job_dir: Path,
    plan_path: Path,
    fail_fast: bool,
    run_id: str | None,
) -> OrchestrationResult:
    worker_argv = {worker.name: _worker_argv(worker) for worker in plan.workers.values()}
    run_id = run_id or _default_run_id()
    _validate_safe_segment(run_id, label="run_id")
    run_dir = job_dir / "orchestration" / "runs" / run_id
    run_dir.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(plan_path, run_dir / "plan.yaml")

    task_statuses = {task_id: "pending" for task_id in plan.tasks}
    worker_in_flight = {worker_name: 0 for worker_name in plan.workers}
    running_writes: dict[str, set[str]] = {}
    pending = set(plan.tasks)
    futures: dict[Future[TaskExecution], str] = {}
    started_at = _utc_now()

    max_workers = sum(worker.max_parallel_tasks for worker in plan.workers.values())
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        while pending or futures:
            _skip_blocked_tasks(plan, pending=pending, task_statuses=task_statuses, run_dir=run_dir)
            scheduled = _schedule_ready_tasks(
                plan,
                pending=pending,
                task_statuses=task_statuses,
                worker_in_flight=worker_in_flight,
                running_writes=running_writes,
                futures=futures,
                executor=executor,
                worker_argv=worker_argv,
                workspace=workspace,
                job_dir=job_dir,
                run_dir=run_dir,
            )
            if not futures:
                if pending and not scheduled:
                    for task_id in sorted(pending):
                        task_statuses[task_id] = "skipped"
                        _write_skipped_task_status(plan.tasks[task_id], run_dir=run_dir, reason="not dependency-ready")
                    pending.clear()
                break

            done, _ = wait(futures, return_when=FIRST_COMPLETED)
            for future in done:
                task_id = futures.pop(future)
                task = plan.tasks[task_id]
                worker_in_flight[task.worker] -= 1
                running_writes.pop(task_id, None)
                try:
                    execution = future.result()
                except Exception as exc:  # pragma: no cover - defensive artifact path
                    execution = _write_internal_task_failure(task, run_dir=run_dir, error=exc)
                task_statuses[task_id] = execution.status
                if fail_fast and execution.status == "failed":
                    for pending_task_id in sorted(pending):
                        task_statuses[pending_task_id] = "skipped"
                        _write_skipped_task_status(
                            plan.tasks[pending_task_id],
                            run_dir=run_dir,
                            reason=f"fail-fast after {task_id}",
                        )
                    pending.clear()

    ok = all(status == "succeeded" for status in task_statuses.values())
    _write_json(
        run_dir / "status.json",
        {
            "ok": ok,
            "run_id": run_id,
            "started_at": started_at,
            "finished_at": _utc_now(),
            "task_statuses": task_statuses,
        },
    )
    return OrchestrationResult(ok=ok, run_dir=run_dir, dry_run=False, task_statuses=task_statuses)


def _schedule_ready_tasks(
    plan: OrchestrationPlan,
    *,
    pending: set[str],
    task_statuses: dict[str, str],
    worker_in_flight: dict[str, int],
    running_writes: dict[str, set[str]],
    futures: dict[Future[TaskExecution], str],
    executor: ThreadPoolExecutor,
    worker_argv: dict[str, list[str]],
    workspace: Path,
    job_dir: Path,
    run_dir: Path,
) -> bool:
    scheduled = False
    registered_stage_in_flight = any(
        plan.tasks[task_id].registered_stage is not None for task_id in futures.values()
    )
    for task_id in sorted(pending):
        task = plan.tasks[task_id]
        worker = plan.workers[task.worker]
        if not all(task_statuses[dependency] == "succeeded" for dependency in task.depends_on):
            continue
        if worker_in_flight[worker.name] >= worker.max_parallel_tasks:
            continue
        if task.registered_stage is not None and registered_stage_in_flight:
            continue
        if _has_running_write_conflict(task, running_writes):
            continue

        task_statuses[task_id] = "running"
        pending.remove(task_id)
        worker_in_flight[worker.name] += 1
        running_writes[task_id] = _task_write_set(task)
        future = executor.submit(
            _run_task,
            task,
            worker,
            worker_argv[worker.name],
            workspace=workspace,
            job_dir=job_dir,
            run_dir=run_dir,
        )
        futures[future] = task_id
        if task.registered_stage is not None:
            registered_stage_in_flight = True
        scheduled = True
    return scheduled


def _skip_blocked_tasks(
    plan: OrchestrationPlan,
    *,
    pending: set[str],
    task_statuses: dict[str, str],
    run_dir: Path,
) -> None:
    changed = True
    while changed:
        changed = False
        for task_id in sorted(pending):
            task = plan.tasks[task_id]
            blocked_by = [
                dependency
                for dependency in task.depends_on
                if task_statuses[dependency] in {"failed", "skipped"}
            ]
            if not blocked_by:
                continue
            task_statuses[task_id] = "skipped"
            pending.remove(task_id)
            _write_skipped_task_status(task, run_dir=run_dir, reason=f"blocked by {', '.join(blocked_by)}")
            changed = True


def _has_running_write_conflict(task: OrchestrationTask, running_writes: dict[str, set[str]]) -> bool:
    task_writes = _task_write_set(task)
    return any(task_writes.intersection(writes) for writes in running_writes.values())


def _task_write_set(task: OrchestrationTask) -> set[str]:
    contract = task.registered_stage
    if contract is None:
        return set(task.writes)
    return set(DEFAULT_STAGE_REGISTRY.get(contract.stage).authoritative_outputs)


def _run_task(
    task: OrchestrationTask,
    worker: WorkerConfig,
    argv: list[str],
    *,
    workspace: Path,
    job_dir: Path,
    run_dir: Path,
) -> TaskExecution:
    if task.registered_stage is not None:
        return _run_registered_stage_task(
            task,
            worker,
            argv,
            workspace=workspace,
            job_dir=job_dir,
            run_dir=run_dir,
        )

    task_dir = run_dir / "tasks" / task.id
    task_dir.mkdir(parents=True, exist_ok=True)
    prompt = _task_prompt(task, workspace=workspace, job_dir=job_dir)
    (task_dir / "prompt.md").write_text(prompt, encoding="utf-8")
    started_at = _utc_now()

    try:
        invocation_argv, input_text, stdin = _worker_invocation(worker, argv, prompt)
        run_kwargs: dict[str, Any] = {
            "cwd": workspace,
            "text": True,
            "capture_output": True,
            "timeout": worker.timeout_seconds,
            "check": False,
        }
        if input_text is not None:
            run_kwargs["input"] = input_text
        else:
            run_kwargs["stdin"] = stdin
        completed = subprocess.run(
            invocation_argv,
            **run_kwargs,
        )
        stdout = completed.stdout
        stderr = completed.stderr
        exit_code: int | None = completed.returncode
    except subprocess.TimeoutExpired as exc:
        stdout = _timeout_output(exc.stdout)
        stderr = (_timeout_output(exc.stderr) + f"\nTimed out after {worker.timeout_seconds} seconds").strip()
        exit_code = None

    status = "succeeded" if exit_code == 0 else "failed"
    (task_dir / "stdout.txt").write_text(stdout, encoding="utf-8")
    (task_dir / "stderr.txt").write_text(stderr, encoding="utf-8")
    (task_dir / "result.md").write_text(stdout, encoding="utf-8")
    if status == "succeeded":
        _promote_task_output(task, workspace=workspace, job_dir=job_dir, stdout=stdout)

    _write_json(
        task_dir / "status.json",
        {
            "task_id": task.id,
            "status": status,
            "command": _redact_command(argv),
            "started_at": started_at,
            "finished_at": _utc_now(),
            "exit_code": exit_code,
            "inputs": list(task.inputs),
            "outputs": list(task.outputs),
            "writes": list(task.writes),
            "privacy_tier": task.privacy_tier,
            "edits_profile_input": task.edits_profile_input,
            "execution_kind": "generic",
            "promotion": "generic_declared_output",
        },
    )
    return TaskExecution(task_id=task.id, status=status, exit_code=exit_code)


def _run_registered_stage_task(
    task: OrchestrationTask,
    worker: WorkerConfig,
    argv: list[str],
    *,
    workspace: Path,
    job_dir: Path,
    run_dir: Path,
) -> TaskExecution:
    contract = task.registered_stage
    if contract is None:  # pragma: no cover - guarded by caller
        raise OrchestrationError("Registered stage execution requires a stage contract")
    task_dir = run_dir / "tasks" / task.id
    task_dir.mkdir(parents=True, exist_ok=True)
    started_at = _utc_now()
    prepared: PreparedStage | None = None
    stdout = ""
    stderr = ""
    exit_code: int | None = None

    try:
        inspection = inspect_stage_status(
            workspace,
            job_dir,
            stage=contract.stage,  # type: ignore[arg-type]
            document_id=contract.document_id,
        )
        if (
            inspection.stage.status == "succeeded"
            and not inspection.reasons
            and not inspection.output_drift
        ):
            (task_dir / "stdout.txt").write_text("", encoding="utf-8")
            (task_dir / "stderr.txt").write_text("", encoding="utf-8")
            (task_dir / "result.md").write_text("", encoding="utf-8")
            _write_json(
                task_dir / "status.json",
                {
                    "task_id": task.id,
                    "status": "succeeded",
                    "stage": contract.stage,
                    "document_id": contract.document_id,
                    "cache_hit": True,
                    "execution_kind": "registered_stage",
                    "promotion": "not_required_current",
                    "started_at": started_at,
                    "finished_at": _utc_now(),
                },
            )
            return TaskExecution(task_id=task.id, status="succeeded", exit_code=None)

        prepared = prepare_stage(
            workspace,
            job_dir,
            stage=contract.stage,  # type: ignore[arg-type]
            execution_mode="host_agent",
            document_id=contract.document_id,
        )
        _validate_prepared_stage_authority(task, worker, prepared.task_spec)
        prompt = _registered_stage_prompt(
            task,
            prepared.task_spec,
            job_dir=job_dir,
        )
        (task_dir / "prompt.md").write_text(prompt, encoding="utf-8")
        invocation_argv, input_text, stdin = _worker_invocation(worker, argv, prompt)
        run_kwargs: dict[str, Any] = {
            "cwd": workspace,
            "text": True,
            "capture_output": True,
            "timeout": worker.timeout_seconds,
            "check": False,
        }
        if input_text is not None:
            run_kwargs["input"] = input_text
        else:
            run_kwargs["stdin"] = stdin
        completed = subprocess.run(invocation_argv, **run_kwargs)
        stdout = completed.stdout
        stderr = completed.stderr
        exit_code = completed.returncode
        (task_dir / "stdout.txt").write_text(stdout, encoding="utf-8")
        (task_dir / "stderr.txt").write_text(stderr, encoding="utf-8")
        (task_dir / "result.md").write_text(stdout, encoding="utf-8")
        if exit_code != 0:
            _cancel_prepared_stage_safely(workspace, job_dir, prepared)
            _write_registered_stage_status(
                task,
                argv,
                prepared,
                task_dir=task_dir,
                job_dir=job_dir,
                status="failed",
                started_at=started_at,
                exit_code=exit_code,
                error="Worker process did not return a successful candidate.",
            )
            return TaskExecution(task_id=task.id, status="failed", exit_code=exit_code)

        submitted = submit_stage_candidate(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=stdout.encode("utf-8"),
        )
        applied = apply_stage_result(
            workspace,
            job_dir,
            task_spec_path=prepared.task_spec_path,
            task_result_path=submitted.result_path,
        )
        _write_registered_stage_status(
            task,
            argv,
            prepared,
            task_dir=task_dir,
            job_dir=job_dir,
            status="succeeded",
            started_at=started_at,
            exit_code=exit_code,
            manifest_path=applied.manifest_path,
            authoritative_path=applied.authoritative_path,
        )
        return TaskExecution(task_id=task.id, status="succeeded", exit_code=exit_code)
    except subprocess.TimeoutExpired as exc:
        stdout = _timeout_output(exc.stdout)
        stderr = (
            _timeout_output(exc.stderr) + f"\nTimed out after {worker.timeout_seconds} seconds"
        ).strip()
        exit_code = None
        if prepared is not None:
            _cancel_prepared_stage_safely(workspace, job_dir, prepared)
        (task_dir / "stdout.txt").write_text(stdout, encoding="utf-8")
        (task_dir / "stderr.txt").write_text(stderr, encoding="utf-8")
        (task_dir / "result.md").write_text(stdout, encoding="utf-8")
        _write_registered_stage_status(
            task,
            argv,
            prepared,
            task_dir=task_dir,
            job_dir=job_dir,
            status="failed",
            started_at=started_at,
            exit_code=exit_code,
            error=f"Worker timed out after {worker.timeout_seconds} seconds.",
        )
        return TaskExecution(task_id=task.id, status="failed", exit_code=exit_code)
    except (OrchestrationError, StageRuntimeError, OSError, UnicodeError, ValueError) as exc:
        if prepared is not None:
            _cancel_prepared_stage_safely(workspace, job_dir, prepared)
        if not (task_dir / "stdout.txt").exists():
            (task_dir / "stdout.txt").write_text(stdout, encoding="utf-8")
        error = _safe_stage_error(exc)
        (task_dir / "stderr.txt").write_text(error, encoding="utf-8")
        (task_dir / "result.md").write_text(stdout, encoding="utf-8")
        _write_registered_stage_status(
            task,
            argv,
            prepared,
            task_dir=task_dir,
            job_dir=job_dir,
            status="failed",
            started_at=started_at,
            exit_code=exit_code,
            error=error,
        )
        return TaskExecution(task_id=task.id, status="failed", exit_code=exit_code)
    except Exception:
        if prepared is not None:
            _cancel_prepared_stage_safely(workspace, job_dir, prepared)
        error = "Registered stage execution failed before guarded promotion."
        (task_dir / "stdout.txt").write_text(stdout, encoding="utf-8")
        (task_dir / "stderr.txt").write_text(error, encoding="utf-8")
        (task_dir / "result.md").write_text(stdout, encoding="utf-8")
        _write_registered_stage_status(
            task,
            argv,
            prepared,
            task_dir=task_dir,
            job_dir=job_dir,
            status="failed",
            started_at=started_at,
            exit_code=exit_code,
            error=error,
        )
        return TaskExecution(task_id=task.id, status="failed", exit_code=exit_code)


def _worker_invocation(
    worker: WorkerConfig,
    argv: list[str],
    prompt: str,
) -> tuple[list[str], str | None, int | None]:
    if worker.prompt_mode == "stdin":
        return argv, prompt, None
    if worker.prompt_mode == "arg":
        return [*argv, prompt], None, subprocess.DEVNULL
    if worker.prompt_mode == "none":
        return argv, None, subprocess.DEVNULL
    raise OrchestrationError(f"Worker {worker.name!r} prompt_mode is unsupported: {worker.prompt_mode}")


def _validate_prepared_stage_authority(
    task: OrchestrationTask,
    worker: WorkerConfig,
    task_spec: TaskSpecV1,
) -> None:
    contract = task.registered_stage
    if contract is None:  # pragma: no cover - guarded by caller
        raise OrchestrationError("Registered stage authority requires a stage contract")
    if (
        task_spec.stage != contract.stage
        or task_spec.document_id != contract.document_id
        or task_spec.execution_mode != "host_agent"
    ):
        raise OrchestrationError("Prepared TaskSpec does not match the registered stage contract")
    if task_spec.privacy_tier > task.privacy_tier:
        raise OrchestrationError(
            "Prepared TaskSpec privacy tier exceeds the task's declared privacy ceiling"
        )
    if task_spec.privacy_tier > worker.privacy_tier_limit:
        raise OrchestrationError(
            "Prepared TaskSpec privacy tier exceeds the selected worker's privacy limit"
        )
    if set(task_spec.allowed_reads) != {item.path for item in task_spec.inputs}:
        raise OrchestrationError("Prepared TaskSpec read authority does not match its bound inputs")
    if set(task_spec.allowed_writes) != {
        task_spec.candidate_output,
        task_spec.result_output,
    }:
        raise OrchestrationError("Prepared TaskSpec write authority is not isolated to its run")


def _registered_stage_prompt(
    task: OrchestrationTask,
    task_spec: TaskSpecV1,
    *,
    job_dir: Path,
) -> str:
    input_blocks: list[str] = []
    for allowed_read in task_spec.allowed_reads:
        path = resolve_job_relative_path(job_dir, allowed_read)
        input_blocks.append(
            f"## {allowed_read}\n\n{path.read_text(encoding='utf-8', errors='strict')}"
        )
    consents = [f"- {consent}" for consent in task_spec.required_consents] or ["- none"]
    reads = [f"- {path}" for path in task_spec.allowed_reads] or ["- none"]
    writes = [f"- {path}" for path in task_spec.allowed_writes] or ["- none"]
    return "\n".join(
        [
            f"Role: {task.role}",
            f"Orchestration task: {task.id}",
            f"Registered stage: {task_spec.stage}",
            f"Document id: {task_spec.document_id or 'none'}",
            f"TaskSpec id: {task_spec.task_id}",
            f"Run id: {task_spec.run_id}",
            f"Input fingerprint: {task_spec.input_fingerprint}",
            f"Privacy tier: {task_spec.privacy_tier}",
            "",
            "Required consents:",
            *consents,
            "",
            "Allowed reads:",
            *reads,
            "",
            "Core-service writes (do not write these paths directly):",
            *writes,
            "",
            "Output contract:",
            f"- schema: {task_spec.output_schema}",
            f"- authoritative target: {task_spec.authoritative_target}",
            "- Return exactly one UTF-8 JSON candidate on stdout.",
            "- Do not wrap the candidate in Markdown or explanatory text.",
            "- Do not modify workspace files; CanISend validates and promotes the candidate.",
            "",
            "Inputs:",
            *input_blocks,
        ]
    )


def _cancel_prepared_stage_safely(
    workspace: Path,
    job_dir: Path,
    prepared: PreparedStage,
) -> None:
    try:
        cancel_stage_task(
            workspace,
            job_dir,
            stage=prepared.task_spec.stage,  # type: ignore[arg-type]
            document_id=prepared.task_spec.document_id,
        )
    except StageRuntimeError:
        # Submit/apply may already have established an immutable terminal outcome.
        return


def _write_registered_stage_status(
    task: OrchestrationTask,
    argv: list[str],
    prepared: PreparedStage | None,
    *,
    task_dir: Path,
    job_dir: Path,
    status: str,
    started_at: str,
    exit_code: int | None,
    error: str | None = None,
    manifest_path: Path | None = None,
    authoritative_path: Path | None = None,
) -> None:
    contract = task.registered_stage
    payload: dict[str, Any] = {
        "task_id": task.id,
        "status": status,
        "stage": contract.stage if contract is not None else None,
        "document_id": contract.document_id if contract is not None else None,
        "command": _redact_command(argv),
        "started_at": started_at,
        "finished_at": _utc_now(),
        "exit_code": exit_code,
        "privacy_tier": (
            prepared.task_spec.privacy_tier if prepared is not None else task.privacy_tier
        ),
        "required_consents": (
            list(prepared.task_spec.required_consents) if prepared is not None else []
        ),
        "execution_kind": "registered_stage",
        "promotion": "guarded_stage_runtime",
    }
    if prepared is not None:
        payload.update(
            {
                "stage_task_id": prepared.task_spec.task_id,
                "stage_run_id": prepared.task_spec.run_id,
                "task_spec_path": _job_relative_path(job_dir, prepared.task_spec_path),
                "task_result_path": _job_relative_path(job_dir, prepared.result_path),
                "allowed_reads": list(prepared.task_spec.allowed_reads),
                "allowed_writes": list(prepared.task_spec.allowed_writes),
            }
        )
    if manifest_path is not None:
        payload["manifest_path"] = _job_relative_path(job_dir, manifest_path)
    if authoritative_path is not None:
        payload["authoritative_path"] = _job_relative_path(job_dir, authoritative_path)
    if error is not None:
        payload["error"] = error
    _write_json(task_dir / "status.json", payload)


def _job_relative_path(job_dir: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(job_dir.resolve()).as_posix()
    except ValueError:
        return path.name


def _safe_stage_error(error: Exception) -> str:
    if isinstance(error, StageRuntimeError):
        return f"{error.code}: {error}"
    return str(error)


def _task_prompt(task: OrchestrationTask, *, workspace: Path, job_dir: Path) -> str:
    return "\n".join(
        [
            f"Role: {task.role}",
            f"Task: {task.id}",
            f"Privacy tier: {task.privacy_tier}",
            f"Agent count: {task.agent_count}",
            f"Edits profile input: {'yes' if task.edits_profile_input else 'no'}",
            "",
            *_profile_input_edit_prompt_lines(task),
            "Inputs:",
            *_input_blocks(task, workspace=workspace, job_dir=job_dir),
        ]
    )


def _profile_input_edit_prompt_lines(task: OrchestrationTask) -> list[str]:
    if not task.edits_profile_input:
        return []
    return [
        "Profile input edit constraints:",
        "- Modify original profile input only within the declared writes list.",
        "- Preserve truthful evidence and do not add unsupported claims.",
        "- Keep the edit minimal; unresolved judgement should remain as a review note, not a source change.",
        "",
    ]


def _input_blocks(task: OrchestrationTask, *, workspace: Path, job_dir: Path) -> list[str]:
    blocks: list[str] = []
    for input_path in task.inputs:
        for resolved in _resolve_input_paths(input_path, workspace=workspace, job_dir=job_dir):
            blocks.append(f"## {input_path}\n\n{resolved.read_text(encoding='utf-8', errors='replace')}")
    return blocks


def _promote_task_output(task: OrchestrationTask, *, workspace: Path, job_dir: Path, stdout: str) -> None:
    if not task.outputs:
        return
    output_path = _resolve_output_path(task.outputs[0], workspace=workspace, job_dir=job_dir)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(stdout, encoding="utf-8")


def _resolve_output_path(output_path: str, *, workspace: Path, job_dir: Path) -> Path:
    path = Path(output_path)
    if _is_profile_input_path(output_path):
        return workspace / path
    return job_dir / path


def _write_skipped_task_status(task: OrchestrationTask, *, run_dir: Path, reason: str) -> None:
    task_dir = run_dir / "tasks" / task.id
    task_dir.mkdir(parents=True, exist_ok=True)
    _write_json(
        task_dir / "status.json",
        {
            "task_id": task.id,
            "status": "skipped",
            "reason": reason,
            "finished_at": _utc_now(),
            "inputs": list(task.inputs),
            "outputs": list(task.outputs),
            "writes": list(task.writes),
            "privacy_tier": task.privacy_tier,
            "execution_kind": (
                "registered_stage" if task.registered_stage is not None else "generic"
            ),
        },
    )


def _write_internal_task_failure(task: OrchestrationTask, *, run_dir: Path, error: Exception) -> TaskExecution:
    task_dir = run_dir / "tasks" / task.id
    task_dir.mkdir(parents=True, exist_ok=True)
    message = str(error)
    (task_dir / "stderr.txt").write_text(message, encoding="utf-8")
    (task_dir / "stdout.txt").write_text("", encoding="utf-8")
    (task_dir / "result.md").write_text("", encoding="utf-8")
    _write_json(
        task_dir / "status.json",
        {
            "task_id": task.id,
            "status": "failed",
            "error": message,
            "finished_at": _utc_now(),
            "execution_kind": (
                "registered_stage" if task.registered_stage is not None else "generic"
            ),
        },
    )
    return TaskExecution(task_id=task.id, status="failed", exit_code=None)


def _worker_argv(worker: WorkerConfig) -> list[str]:
    argv = shlex.split(worker.command)
    if not argv:
        raise OrchestrationError(f"Worker {worker.name!r} command is empty")

    executable = argv[0]
    if Path(executable).name == executable:
        resolved = shutil.which(executable)
        if resolved is None:
            raise OrchestrationError(f"Worker {worker.name!r} command is unavailable: {executable}")
        argv[0] = resolved
    elif not Path(executable).exists():
        raise OrchestrationError(f"Worker {worker.name!r} command is unavailable: {executable}")
    return argv


def _redact_command(argv: list[str]) -> list[str]:
    redacted: list[str] = []
    redact_next = False
    for token in argv:
        if redact_next:
            redacted.append("[redacted]")
            redact_next = False
            continue
        if _is_sensitive_key(token):
            if "=" in token:
                redacted.append(token.split("=", 1)[0] + "=[redacted]")
            else:
                redacted.append(token)
                redact_next = token.startswith("-")
            continue
        redacted.append(token)
    return redacted


def _is_sensitive_key(token: str) -> bool:
    key = token.split("=", 1)[0].lstrip("-")
    normalized = re.sub(r"[^a-z0-9]", "", key.lower())
    return any(name in normalized for name in ("apikey", "password", "secret", "token"))


def _timeout_output(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value


def _write_json(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _utc_now() -> str:
    return datetime.now(UTC).isoformat()


def _default_run_id() -> str:
    return f"{datetime.now(UTC).strftime('%Y%m%dT%H%M%SZ')}-{uuid4().hex[:8]}"


def _tuple_field(raw: dict[str, Any], field: str) -> tuple[str, ...]:
    value = raw.get(field, [])
    if value is None:
        return ()
    if not isinstance(value, list):
        raise OrchestrationError(f"{field} must be a list")
    return tuple(str(item) for item in value)


def _int_field(raw: dict[str, Any], field: str, *, default: int) -> int:
    value = raw.get(field, default)
    if isinstance(value, bool):
        raise OrchestrationError(f"{field} must be an integer")
    try:
        return int(value)
    except (TypeError, ValueError) as exc:
        raise OrchestrationError(f"{field} must be an integer") from exc


def _agent_count(value: Any, task_id: str) -> int:
    if isinstance(value, bool):
        raise OrchestrationError(f"Task {task_id!r} agent_count must be an integer")
    try:
        agent_count = int(value)
    except (TypeError, ValueError) as exc:
        raise OrchestrationError(f"Task {task_id!r} agent_count must be an integer") from exc
    if agent_count < 1:
        raise OrchestrationError(f"Task {task_id!r} agent_count must be at least 1")
    return agent_count


def _privacy_tier(value: Any, label: str) -> int:
    if isinstance(value, bool):
        raise OrchestrationError(f"{label} privacy tier must be an integer from 0 to 3")
    try:
        privacy_tier = int(value)
    except (TypeError, ValueError) as exc:
        raise OrchestrationError(f"{label} privacy tier must be an integer from 0 to 3") from exc
    if privacy_tier < 0 or privacy_tier > 3:
        raise OrchestrationError(f"{label} privacy tier must be from 0 to 3")
    return privacy_tier


def _normalized_relative_paths(raw: dict[str, Any], field: str, label: str) -> tuple[str, ...]:
    return tuple(_normalize_relative_path(raw_path, label) for raw_path in _tuple_field(raw, field))


def _normalize_relative_path(raw_path: str, label: str) -> str:
    path = Path(raw_path)
    normalized = path.as_posix()
    if path.is_absolute() or ".." in path.parts or normalized in {"", "."}:
        raise OrchestrationError(f"{label} must use workspace-relative file paths: {raw_path}")
    return normalized


def _validate_safe_segment(value: str, *, label: str) -> None:
    if value in {".", ".."} or not SAFE_SEGMENT_RE.fullmatch(value):
        raise OrchestrationError(f"{label} must be a single safe path segment: {value}")
