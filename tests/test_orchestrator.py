from __future__ import annotations

import json
import sys
import time
from pathlib import Path

import pytest
import yaml

from canisend.orchestrator import OrchestrationError, load_orchestration_plan, run_orchestration


def write_plan(path: Path, data: dict) -> Path:
    path.write_text(yaml.safe_dump(data, sort_keys=False), encoding="utf-8")
    return path


def base_job(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "job"
    job_dir.mkdir(parents=True)
    (job_dir / "parsed_job.json").write_text('{"title": "Lecturer"}\n', encoding="utf-8")
    (job_dir / "05_criteria_checklist.md").write_text("# Criteria\n", encoding="utf-8")
    profile = workspace / "profile" / "generated"
    profile.mkdir(parents=True)
    (profile / "cv.evidence.md").write_text("# Evidence\n", encoding="utf-8")
    return workspace, job_dir


def base_plan() -> dict:
    return {
        "workers": {
            "echo": {
                "command": sys.executable,
                "max_parallel_tasks": 1,
                "privacy_tier_limit": 1,
            }
        },
        "tasks": [
            {
                "id": "review",
                "worker": "echo",
                "role": "job_parser_reviewer",
                "privacy_tier": 1,
                "inputs": ["parsed_job.json"],
                "outputs": ["orchestration/reviews/review.md"],
                "writes": ["orchestration/reviews/review.md"],
            }
        ],
    }


def test_load_orchestration_plan_rejects_duplicate_task_ids(tmp_path):
    plan = base_plan()
    plan["tasks"].append(
        {
            "id": "review",
            "worker": "echo",
            "role": "criteria_reviewer",
            "inputs": ["05_criteria_checklist.md"],
            "outputs": ["orchestration/reviews/criteria.md"],
        }
    )
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="Duplicate task id"):
        load_orchestration_plan(plan_path)


def test_load_orchestration_plan_rejects_unsafe_task_ids(tmp_path):
    plan = base_plan()
    plan["tasks"][0]["id"] = "../escape"
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="safe path segment"):
        load_orchestration_plan(plan_path)


def test_load_orchestration_plan_rejects_missing_worker(tmp_path):
    plan = base_plan()
    plan["tasks"][0]["worker"] = "missing"
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="unknown worker"):
        load_orchestration_plan(plan_path)


def test_load_orchestration_plan_rejects_missing_dependency(tmp_path):
    plan = base_plan()
    plan["tasks"][0]["depends_on"] = ["missing"]
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="unknown dependency"):
        load_orchestration_plan(plan_path)


def test_load_orchestration_plan_rejects_dependency_cycles(tmp_path):
    plan = base_plan()
    plan["tasks"] = [
        {
            "id": "a",
            "worker": "echo",
            "role": "a",
            "depends_on": ["b"],
            "outputs": ["a.md"],
            "writes": ["a.md"],
        },
        {
            "id": "b",
            "worker": "echo",
            "role": "b",
            "depends_on": ["a"],
            "outputs": ["b.md"],
            "writes": ["b.md"],
        },
    ]
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="cycle"):
        load_orchestration_plan(plan_path)


def test_load_orchestration_plan_rejects_parallel_write_conflicts(tmp_path):
    plan = base_plan()
    plan["tasks"] = [
        {
            "id": "a",
            "worker": "echo",
            "role": "a",
            "outputs": ["shared.md"],
            "writes": ["shared.md"],
        },
        {
            "id": "b",
            "worker": "echo",
            "role": "b",
            "outputs": ["shared.md"],
            "writes": ["shared.md"],
        },
    ]
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="write conflict"):
        load_orchestration_plan(plan_path)


def test_load_orchestration_plan_rejects_normalized_parallel_write_conflicts(tmp_path):
    plan = base_plan()
    plan["tasks"] = [
        {
            "id": "a",
            "worker": "echo",
            "role": "a",
            "outputs": ["shared.md"],
            "writes": ["shared.md"],
        },
        {
            "id": "b",
            "worker": "echo",
            "role": "b",
            "outputs": ["./shared.md"],
            "writes": ["./shared.md"],
        },
    ]
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="write conflict"):
        load_orchestration_plan(plan_path)


def test_load_orchestration_plan_rejects_worker_parallelism_below_one(tmp_path):
    plan = base_plan()
    plan["workers"]["echo"]["max_parallel_tasks"] = 0
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="max_parallel_tasks"):
        load_orchestration_plan(plan_path)


def test_load_orchestration_plan_rejects_worker_privacy_limit_excess(tmp_path):
    plan = base_plan()
    plan["tasks"][0]["privacy_tier"] = 2
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="privacy tier"):
        load_orchestration_plan(plan_path)


def test_run_orchestration_dry_run_returns_ready_tasks(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    plan_path = write_plan(tmp_path / "plan.yaml", base_plan())

    result = run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, dry_run=True)

    assert result.ok
    assert result.dry_run
    assert result.run_dir is None
    assert result.task_statuses["review"] == "ready"


def test_run_orchestration_rejects_missing_inputs_before_dry_run(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    (job_dir / "parsed_job.json").unlink()
    plan_path = write_plan(tmp_path / "plan.yaml", base_plan())

    with pytest.raises(OrchestrationError, match="missing input"):
        run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, dry_run=True)


def test_run_orchestration_rejects_unsafe_run_id(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    plan_path = write_plan(tmp_path / "plan.yaml", base_plan())

    with pytest.raises(OrchestrationError, match="safe path segment"):
        run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, run_id="../../escape")


def test_run_orchestration_requires_private_source_opt_in(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    plan = base_plan()
    plan["workers"]["echo"]["privacy_tier_limit"] = 2
    plan["tasks"][0]["privacy_tier"] = 2
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="allow-private-sources"):
        run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, dry_run=True)

    result = run_orchestration(
        workspace=workspace,
        job_dir=job_dir,
        plan_path=plan_path,
        dry_run=True,
        allow_private_sources=True,
    )
    assert result.task_statuses["review"] == "ready"


def test_run_orchestration_requires_provider_backed_opt_in(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    plan = base_plan()
    plan["workers"]["echo"]["privacy_tier_limit"] = 3
    plan["tasks"][0]["privacy_tier"] = 3
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    with pytest.raises(OrchestrationError, match="allow-provider-backed"):
        run_orchestration(
            workspace=workspace,
            job_dir=job_dir,
            plan_path=plan_path,
            dry_run=True,
            allow_private_sources=True,
        )

    result = run_orchestration(
        workspace=workspace,
        job_dir=job_dir,
        plan_path=plan_path,
        dry_run=True,
        allow_private_sources=True,
        allow_provider_backed=True,
    )
    assert result.task_statuses["review"] == "ready"


def test_run_orchestration_executes_worker_and_writes_artifacts(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    worker = tmp_path / "worker.py"
    worker.write_text(
        "import sys\n"
        "prompt = sys.stdin.read()\n"
        "print('RESULT:' + prompt.splitlines()[0])\n",
        encoding="utf-8",
    )
    plan_path = write_plan(
        tmp_path / "plan.yaml",
        {
            "workers": {"python": {"command": f"{sys.executable} {worker}", "max_parallel_tasks": 1}},
            "tasks": [
                {
                    "id": "review",
                    "worker": "python",
                    "role": "job_parser_reviewer",
                    "privacy_tier": 1,
                    "inputs": ["parsed_job.json"],
                    "outputs": ["orchestration/reviews/review.md"],
                    "writes": ["orchestration/reviews/review.md"],
                }
            ],
        },
    )

    result = run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, run_id="run-test")

    run_task_dir = job_dir / "orchestration" / "runs" / "run-test" / "tasks" / "review"
    assert result.ok
    assert result.task_statuses["review"] == "succeeded"
    assert (run_task_dir / "stdout.txt").exists()
    assert "RESULT:Role: job_parser_reviewer" in (job_dir / "orchestration" / "reviews" / "review.md").read_text()


def test_run_orchestration_redacts_api_key_spellings_in_status(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    worker = tmp_path / "worker.py"
    worker.write_text("print('ok')\n", encoding="utf-8")
    plan = base_plan()
    plan["workers"]["echo"]["command"] = (
        f"{sys.executable} {worker} --api_key sk-next OPENAI_API_KEY=sk-env"
    )
    plan_path = write_plan(tmp_path / "plan.yaml", plan)

    run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, run_id="redact")

    status = json.loads(
        (job_dir / "orchestration" / "runs" / "redact" / "tasks" / "review" / "status.json").read_text()
    )
    command = " ".join(status["command"])
    assert "sk-next" not in command
    assert "sk-env" not in command
    assert "--api_key [redacted]" in command
    assert "OPENAI_API_KEY=[redacted]" in command


def test_run_orchestration_runs_independent_tasks_in_parallel_for_one_worker(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    worker = tmp_path / "slow_worker.py"
    worker.write_text(
        "import time\n"
        "time.sleep(0.4)\n"
        "print('done')\n",
        encoding="utf-8",
    )
    plan_path = write_plan(
        tmp_path / "plan.yaml",
        {
            "workers": {"python": {"command": f"{sys.executable} {worker}", "max_parallel_tasks": 2}},
            "tasks": [
                {
                    "id": "a",
                    "worker": "python",
                    "role": "r",
                    "inputs": ["parsed_job.json"],
                    "outputs": ["orchestration/reviews/a.md"],
                    "writes": ["orchestration/reviews/a.md"],
                },
                {
                    "id": "b",
                    "worker": "python",
                    "role": "r",
                    "inputs": ["parsed_job.json"],
                    "outputs": ["orchestration/reviews/b.md"],
                    "writes": ["orchestration/reviews/b.md"],
                },
            ],
        },
    )

    started = time.monotonic()
    result = run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, run_id="parallel")
    elapsed = time.monotonic() - started

    assert result.ok
    assert result.task_statuses == {"a": "succeeded", "b": "succeeded"}
    assert elapsed < 0.75


def test_run_orchestration_skips_downstream_after_failure(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    worker = tmp_path / "fail_worker.py"
    worker.write_text("import sys\nsys.exit(2)\n", encoding="utf-8")
    plan_path = write_plan(
        tmp_path / "plan.yaml",
        {
            "workers": {"python": {"command": f"{sys.executable} {worker}", "max_parallel_tasks": 1}},
            "tasks": [
                {
                    "id": "a",
                    "worker": "python",
                    "role": "r",
                    "inputs": ["parsed_job.json"],
                    "outputs": ["orchestration/reviews/a.md"],
                    "writes": ["orchestration/reviews/a.md"],
                },
                {
                    "id": "b",
                    "worker": "python",
                    "role": "r",
                    "depends_on": ["a"],
                    "inputs": ["parsed_job.json"],
                    "outputs": ["orchestration/reviews/b.md"],
                    "writes": ["orchestration/reviews/b.md"],
                },
            ],
        },
    )

    result = run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, run_id="failed")

    assert not result.ok
    assert result.task_statuses["a"] == "failed"
    assert result.task_statuses["b"] == "skipped"
