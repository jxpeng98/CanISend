from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml


class OrchestrationError(ValueError):
    pass


@dataclass(frozen=True)
class WorkerConfig:
    name: str
    command: str
    max_parallel_tasks: int = 1
    supports_native_subagents: bool = False
    privacy_tier_limit: int = 1
    timeout_seconds: int = 300


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
    return OrchestrationPlan(workers=workers, tasks=tasks)


def run_orchestration(
    *,
    workspace: Path,
    job_dir: Path,
    plan_path: Path,
    dry_run: bool = False,
    allow_private_sources: bool = False,
    allow_provider_backed: bool = False,
    fail_fast: bool = False,
    run_id: str | None = None,
) -> OrchestrationResult:
    plan = load_orchestration_plan(plan_path)
    _validate_privacy_flags(
        plan,
        allow_private_sources=allow_private_sources,
        allow_provider_backed=allow_provider_backed,
    )
    _validate_inputs(plan, workspace=workspace, job_dir=job_dir)
    if dry_run:
        return OrchestrationResult(
            ok=True,
            run_dir=None,
            dry_run=True,
            task_statuses={task_id: "ready" for task_id in plan.tasks},
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
        command = str(raw_worker.get("command", "")).strip()
        if not command:
            raise OrchestrationError(f"Worker {name!r} must define command")

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
            max_parallel_tasks=max_parallel_tasks,
            supports_native_subagents=bool(raw_worker.get("supports_native_subagents", False)),
            privacy_tier_limit=privacy_tier_limit,
            timeout_seconds=timeout_seconds,
        )
    return workers


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
        if task_id in tasks:
            raise OrchestrationError(f"Duplicate task id: {task_id}")

        worker_name = str(raw_task.get("worker", "")).strip()
        if worker_name not in workers:
            raise OrchestrationError(f"Task {task_id!r} references unknown worker: {worker_name}")

        role = str(raw_task.get("role", "")).strip()
        if not role:
            raise OrchestrationError(f"Task {task_id!r} must define role")

        privacy_tier = _privacy_tier(raw_task.get("privacy_tier", 1), f"task {task_id!r}")
        if privacy_tier > workers[worker_name].privacy_tier_limit:
            raise OrchestrationError(
                f"Task {task_id!r} privacy tier exceeds worker {worker_name!r} privacy tier limit"
            )

        outputs = _tuple_field(raw_task, "outputs")
        writes = _tuple_field(raw_task, "writes") or outputs
        task = OrchestrationTask(
            id=task_id,
            worker=worker_name,
            role=role,
            privacy_tier=privacy_tier,
            inputs=_tuple_field(raw_task, "inputs"),
            outputs=outputs,
            writes=writes,
            depends_on=_tuple_field(raw_task, "depends_on"),
            agent_count=_agent_count(raw_task.get("agent_count", 1), task_id),
        )
        _validate_relative_paths(task.inputs, f"task {task_id!r} inputs")
        _validate_relative_paths(task.outputs, f"task {task_id!r} outputs")
        _validate_relative_paths(task.writes, f"task {task_id!r} writes")
        tasks[task_id] = task
    return tasks


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
        first_writes = set(first.writes)
        if not first_writes:
            continue
        for second in task_list[index + 1 :]:
            shared = first_writes.intersection(second.writes)
            if not shared:
                continue
            if _depends_on(tasks, first.id, second.id) or _depends_on(tasks, second.id, first.id):
                continue
            paths = ", ".join(sorted(shared))
            raise OrchestrationError(
                f"Task {first.id!r} and task {second.id!r} have a write conflict: {paths}"
            )


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
            if not _input_exists(input_path, workspace=workspace, job_dir=job_dir):
                raise OrchestrationError(f"Task {task.id!r} missing input: {input_path}")


def _input_exists(input_path: str, *, workspace: Path, job_dir: Path) -> bool:
    path = Path(input_path)
    return (job_dir / path).exists() or (workspace / path).exists()


def _execute_plan(
    plan: OrchestrationPlan,
    *,
    workspace: Path,
    job_dir: Path,
    plan_path: Path,
    fail_fast: bool,
    run_id: str | None,
) -> OrchestrationResult:
    raise OrchestrationError("Orchestration execution is not implemented yet; use dry_run=True")


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


def _validate_relative_paths(paths: tuple[str, ...], label: str) -> None:
    for raw_path in paths:
        path = Path(raw_path)
        if path.is_absolute() or ".." in path.parts:
            raise OrchestrationError(f"{label} must use workspace-relative paths: {raw_path}")
