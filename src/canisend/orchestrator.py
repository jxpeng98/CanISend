from __future__ import annotations

from concurrent.futures import FIRST_COMPLETED, Future, ThreadPoolExecutor, wait
from dataclasses import dataclass
from datetime import UTC, datetime
import glob
import json
from pathlib import Path
import shlex
import shutil
import subprocess
from typing import Any
from uuid import uuid4

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
            if not _resolve_input_paths(input_path, workspace=workspace, job_dir=job_dir):
                raise OrchestrationError(f"Task {task.id!r} missing input: {input_path}")


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
    for task_id in sorted(pending):
        task = plan.tasks[task_id]
        worker = plan.workers[task.worker]
        if not all(task_statuses[dependency] == "succeeded" for dependency in task.depends_on):
            continue
        if worker_in_flight[worker.name] >= worker.max_parallel_tasks:
            continue
        if _has_running_write_conflict(task, running_writes):
            continue

        task_statuses[task_id] = "running"
        pending.remove(task_id)
        worker_in_flight[worker.name] += 1
        running_writes[task_id] = set(task.writes)
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
    task_writes = set(task.writes)
    return any(task_writes.intersection(writes) for writes in running_writes.values())


def _run_task(
    task: OrchestrationTask,
    worker: WorkerConfig,
    argv: list[str],
    *,
    workspace: Path,
    job_dir: Path,
    run_dir: Path,
) -> TaskExecution:
    task_dir = run_dir / "tasks" / task.id
    task_dir.mkdir(parents=True, exist_ok=True)
    prompt = _task_prompt(task, workspace=workspace, job_dir=job_dir)
    (task_dir / "prompt.md").write_text(prompt, encoding="utf-8")
    started_at = _utc_now()

    try:
        completed = subprocess.run(
            argv,
            cwd=workspace,
            input=prompt,
            text=True,
            capture_output=True,
            timeout=worker.timeout_seconds,
            check=False,
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
        _promote_task_output(task, job_dir=job_dir, stdout=stdout)

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
        },
    )
    return TaskExecution(task_id=task.id, status=status, exit_code=exit_code)


def _task_prompt(task: OrchestrationTask, *, workspace: Path, job_dir: Path) -> str:
    return "\n".join(
        [
            f"Role: {task.role}",
            f"Task: {task.id}",
            f"Privacy tier: {task.privacy_tier}",
            f"Agent count: {task.agent_count}",
            "",
            "Inputs:",
            *_input_blocks(task, workspace=workspace, job_dir=job_dir),
        ]
    )


def _input_blocks(task: OrchestrationTask, *, workspace: Path, job_dir: Path) -> list[str]:
    blocks: list[str] = []
    for input_path in task.inputs:
        for resolved in _resolve_input_paths(input_path, workspace=workspace, job_dir=job_dir):
            blocks.append(f"## {input_path}\n\n{resolved.read_text(encoding='utf-8', errors='replace')}")
    return blocks


def _promote_task_output(task: OrchestrationTask, *, job_dir: Path, stdout: str) -> None:
    if not task.outputs:
        return
    output_path = job_dir / task.outputs[0]
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(stdout, encoding="utf-8")


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
    sensitive_names = ("api-key", "apikey", "password", "secret", "token")
    for token in argv:
        lower = token.lower()
        if redact_next:
            redacted.append("[redacted]")
            redact_next = False
            continue
        if any(name in lower for name in sensitive_names):
            if "=" in token:
                redacted.append(token.split("=", 1)[0] + "=[redacted]")
            else:
                redacted.append(token)
                redact_next = token.startswith("-")
            continue
        redacted.append(token)
    return redacted


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


def _validate_relative_paths(paths: tuple[str, ...], label: str) -> None:
    for raw_path in paths:
        path = Path(raw_path)
        if path.is_absolute() or ".." in path.parts:
            raise OrchestrationError(f"{label} must use workspace-relative paths: {raw_path}")
