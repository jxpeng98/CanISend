from __future__ import annotations

import json
from pathlib import Path
import shutil
import sys

import pytest
import yaml
from typer.testing import CliRunner

from canisend.cli import app
from canisend.llm import LLMProvider, LLMResponse
from canisend.orchestrator import (
    OrchestrationError,
    RegisteredStageTask,
    load_orchestration_plan,
    run_orchestration,
)
from canisend.stage_runtime import inspect_stage_status, run_configured_provider_stage
from canisend.stage_store import read_json_object
from canisend.stages.parse_stage import build_deterministic_parse_candidate


def _write_plan(path: Path, data: dict[str, object]) -> Path:
    path.write_text(yaml.safe_dump(data, sort_keys=False), encoding="utf-8")
    return path


def _registered_plan(
    command: str,
    *,
    kind: str = "custom",
    prompt_mode: str | None = None,
) -> dict[str, object]:
    worker: dict[str, object] = {
        "kind": kind,
        "command": command,
        "max_parallel_tasks": 2,
        "privacy_tier_limit": 2,
    }
    if prompt_mode is not None:
        worker["prompt_mode"] = prompt_mode
    return {
        "workers": {"agent": worker},
        "tasks": [
            {
                "id": "parse-stage",
                "worker": "agent",
                "role": "job_parser",
                "privacy_tier": 2,
                "registered_stage": {"stage": "parse"},
            }
        ],
    }


def _base_workspace(tmp_path: Path) -> tuple[Path, str]:
    workspace = tmp_path / "base"
    advert = tmp_path / "advert.md"
    advert.write_text(
        """# Lecturer in Economics

Department: Economics
Location: London
Required documents: CV, Cover letter

Essential criteria:
- PhD in Economics

Desirable criteria:
- Experience teaching econometrics
""",
        encoding="utf-8",
    )
    runner = CliRunner()
    initialized = runner.invoke(app, ["init-workspace", "--workspace", str(workspace)])
    created = runner.invoke(
        app,
        [
            "new-job",
            "--workspace",
            str(workspace),
            "--title",
            "Lecturer in Economics",
            "--institution",
            "Example University",
            "--deadline",
            "2026-08-01",
            "--advert-file",
            str(advert),
            "--format",
            "json",
        ],
    )
    assert initialized.exit_code == 0, initialized.output
    assert created.exit_code == 0, created.output
    job_path = str(json.loads(created.stdout)["job"]["path"])
    return workspace, job_path


def _clone_workspace(base: Path, target: Path, job_path: str) -> tuple[Path, Path]:
    shutil.copytree(base, target)
    return target, target / job_path


def _invoke_json(args: list[str]) -> dict[str, object]:
    result = CliRunner().invoke(app, args)
    assert result.exit_code == 0, result.output
    return json.loads(result.stdout)


def _run_cli_host(workspace: Path, job_path: str, candidate_file: Path) -> None:
    common = ["--workspace", str(workspace), "--job", job_path, "--format", "json"]
    prepared = _invoke_json(
        ["stage", "prepare", "--stage", "parse", "--mode", "host-agent", *common]
    )
    task_workspace_path = next(
        str(item["path"])
        for item in prepared["artifacts"]  # type: ignore[index]
        if item["kind"] == "stage_task_spec"
    )
    task_job_path = str(Path(task_workspace_path).relative_to(job_path))
    task = read_json_object(workspace / task_workspace_path)
    _invoke_json(
        [
            "stage",
            "submit",
            "--task",
            task_job_path,
            "--candidate-file",
            str(candidate_file),
            *common,
        ]
    )
    _invoke_json(
        [
            "stage",
            "apply",
            "--task",
            task_job_path,
            "--result",
            str(task["result_output"]),
            *common,
        ]
    )


def _receipt_signature(job_dir: Path) -> dict[str, object]:
    successful = []
    for manifest_path in (job_dir / "workflow" / "runs").glob("*/manifest.json"):
        manifest = read_json_object(manifest_path)
        if manifest.get("status") == "succeeded":
            successful.append((manifest_path, manifest))
    assert len(successful) == 1
    manifest_path, manifest = successful[0]
    run_dir = manifest_path.parent
    validation = read_json_object(run_dir / "validation" / "report.json")
    promotion = read_json_object(run_dir / "promotion.json")
    submission = read_json_object(run_dir / "submission.json")
    claim = read_json_object(run_dir / "terminal-claim.json")
    return {
        "stage": manifest["stage"],
        "status": manifest["status"],
        "input_fingerprint": manifest["input_fingerprint"],
        "input_hashes": tuple(item["sha256"] for item in manifest["inputs"]),
        "candidate_sha256": submission["candidate"]["sha256"],
        "promoted_sha256": manifest["promoted_outputs"][0]["sha256"],
        "promotion_candidate_sha256": promotion["candidate_sha256"],
        "promotion_authoritative_sha256": promotion["authoritative_sha256"],
        "terminal_action": claim["action"],
        "validation": {
            key: validation[key]
            for key in (
                "status",
                "input_hashes_match",
                "schema_valid",
                "scope_valid",
                "citations_valid",
                "errors",
                "warnings",
            )
        },
    }


def test_registered_stage_contract_rejects_unsupported_or_expanded_authority(
    tmp_path: Path,
) -> None:
    command = sys.executable
    plan = _registered_plan(command)
    plan["tasks"][0]["inputs"] = ["job_advert.md"]  # type: ignore[index]
    plan_path = _write_plan(tmp_path / "expanded.yaml", plan)

    with pytest.raises(OrchestrationError, match="must not declare inputs"):
        load_orchestration_plan(plan_path)

    unsupported = _registered_plan(command)
    unsupported["tasks"][0]["registered_stage"] = {"stage": "confirm"}  # type: ignore[index]
    unsupported_path = _write_plan(tmp_path / "unsupported.yaml", unsupported)

    with pytest.raises(OrchestrationError, match="does not support host-agent"):
        load_orchestration_plan(unsupported_path)

    understated = _registered_plan(command)
    understated["tasks"][0]["privacy_tier"] = 1  # type: ignore[index]
    understated_path = _write_plan(tmp_path / "understated.yaml", understated)

    with pytest.raises(OrchestrationError, match="requires privacy tier 2"):
        load_orchestration_plan(understated_path)


def test_packaged_registered_parse_example_loads_with_guarded_authority() -> None:
    plan = load_orchestration_plan(Path("examples/orchestration/registered-parse.example.yaml"))

    task = plan.tasks["guarded-parse"]
    assert task.registered_stage == RegisteredStageTask(stage="parse")
    assert task.privacy_tier == 2
    assert task.inputs == ()
    assert task.outputs == ()
    assert task.writes == ()


def test_registered_stage_dry_run_is_read_only_and_reports_core_readiness(
    tmp_path: Path,
) -> None:
    workspace, job_path = _base_workspace(tmp_path)
    job_dir = workspace / job_path
    plan_path = _write_plan(tmp_path / "plan.yaml", _registered_plan(sys.executable))

    result = run_orchestration(
        workspace=workspace,
        job_dir=job_dir,
        plan_path=plan_path,
        dry_run=True,
        allow_private_sources=True,
    )

    assert result.ok
    assert result.task_statuses == {"parse-stage": "ready"}
    assert not (job_dir / "workflow").exists()
    assert not (job_dir / "orchestration").exists()


def test_registered_stage_invalid_candidate_is_cancelled_without_direct_promotion(
    tmp_path: Path,
) -> None:
    workspace, job_path = _base_workspace(tmp_path)
    job_dir = workspace / job_path
    worker = tmp_path / "invalid.py"
    worker.write_text("print('not-json')\n", encoding="utf-8")
    plan_path = _write_plan(
        tmp_path / "plan.yaml",
        _registered_plan(f"{sys.executable} {worker}"),
    )

    result = run_orchestration(
        workspace=workspace,
        job_dir=job_dir,
        plan_path=plan_path,
        run_id="invalid-stage",
        allow_private_sources=True,
    )

    status = read_json_object(
        job_dir
        / "orchestration"
        / "runs"
        / "invalid-stage"
        / "tasks"
        / "parse-stage"
        / "status.json"
    )
    inspection = inspect_stage_status(workspace, job_dir, stage="parse")
    assert not result.ok
    assert status["execution_kind"] == "registered_stage"
    assert status["promotion"] == "guarded_stage_runtime"
    assert status["error"].startswith("stage.invalid_candidate:")
    assert inspection.stage.status == "cancelled"
    assert not (job_dir / "parsed_job.json").exists()


def test_registered_stage_prompt_and_status_derive_authority_from_task_spec(
    tmp_path: Path,
) -> None:
    workspace, job_path = _base_workspace(tmp_path)
    job_dir = workspace / job_path
    candidate = build_deterministic_parse_candidate(job_dir)
    candidate_file = tmp_path / "candidate.json"
    candidate_file.write_text(json.dumps(candidate), encoding="utf-8")
    worker = tmp_path / "worker.py"
    worker.write_text(
        "from pathlib import Path\n"
        "import sys\n"
        "sys.stdout.write(Path(sys.argv[1]).read_text(encoding='utf-8'))\n",
        encoding="utf-8",
    )
    plan_path = _write_plan(
        tmp_path / "plan.yaml",
        _registered_plan(f"{sys.executable} {worker} {candidate_file}"),
    )

    result = run_orchestration(
        workspace=workspace,
        job_dir=job_dir,
        plan_path=plan_path,
        run_id="guarded-stage",
        allow_private_sources=True,
    )

    task_dir = job_dir / "orchestration" / "runs" / "guarded-stage" / "tasks" / "parse-stage"
    prompt = (task_dir / "prompt.md").read_text(encoding="utf-8")
    status = read_json_object(task_dir / "status.json")
    assert result.ok
    assert "read-full-job-advert" in prompt
    assert "- job.yaml" in prompt
    assert "- job_advert.md" in prompt
    assert status["allowed_reads"] == ["job.yaml", "job_advert.md"]
    assert status["execution_kind"] == "registered_stage"
    assert status["promotion"] == "guarded_stage_runtime"
    assert (job_dir / status["manifest_path"]).is_file()
    assert read_json_object(job_dir / "parsed_job.json") == candidate


def test_registered_stage_task_spec_exists_before_worker_dispatch(tmp_path: Path) -> None:
    workspace, job_path = _base_workspace(tmp_path)
    job_dir = workspace / job_path
    candidate = build_deterministic_parse_candidate(job_dir)
    candidate_file = tmp_path / "candidate.json"
    candidate_file.write_text(json.dumps(candidate), encoding="utf-8")
    worker = tmp_path / "preparation_check.py"
    worker.write_text(
        "from pathlib import Path\n"
        "import sys\n"
        "workspace = Path.cwd()\n"
        "specs = list(workspace.glob('jobs/*/workflow/runs/*/task-spec.json'))\n"
        "if len(specs) != 1:\n"
        "    raise SystemExit(9)\n"
        "sys.stdout.write(Path(sys.argv[1]).read_text(encoding='utf-8'))\n",
        encoding="utf-8",
    )
    plan_path = _write_plan(
        tmp_path / "plan.yaml",
        _registered_plan(f"{sys.executable} {worker} {candidate_file}"),
    )

    result = run_orchestration(
        workspace=workspace,
        job_dir=job_dir,
        plan_path=plan_path,
        run_id="prepared-before-dispatch",
        allow_private_sources=True,
    )

    assert result.ok
    assert read_json_object(job_dir / "parsed_job.json") == candidate


def test_all_host_surfaces_produce_equivalent_guarded_parse_receipts(
    tmp_path: Path,
) -> None:
    base, job_path = _base_workspace(tmp_path)
    base_job = base / job_path
    candidate = build_deterministic_parse_candidate(base_job)
    candidate_file = tmp_path / "candidate.json"
    candidate_file.write_text(json.dumps(candidate), encoding="utf-8")
    worker = tmp_path / "candidate_worker.py"
    worker.write_text(
        "from pathlib import Path\n"
        "import sys\n"
        "sys.stdout.write(Path(sys.argv[1]).read_text(encoding='utf-8'))\n",
        encoding="utf-8",
    )
    command = f"{sys.executable} {worker} {candidate_file}"

    workspaces: dict[str, tuple[Path, Path]] = {
        name: _clone_workspace(base, tmp_path / name, job_path)
        for name in ("cli", "provider", "codex", "claude", "orchestrated")
    }
    cli_workspace, _ = workspaces["cli"]
    _run_cli_host(cli_workspace, job_path, candidate_file)

    class Provider(LLMProvider):
        def complete(self, prompt: str) -> LLMResponse:
            assert "PhD in Economics" in prompt
            return LLMResponse(content=json.dumps(candidate), provider="test")

    provider_workspace, provider_job = workspaces["provider"]
    run_configured_provider_stage(
        provider_workspace,
        provider_job,
        stage="parse",
        allow_provider_backed=True,
        provider=Provider(),
    )

    for name, kind, prompt_mode in (
        ("codex", "codex", "stdin"),
        ("claude", "claude", None),
        ("orchestrated", "custom", "none"),
    ):
        workspace, job_dir = workspaces[name]
        plan_path = _write_plan(
            tmp_path / f"{name}.yaml",
            _registered_plan(command, kind=kind, prompt_mode=prompt_mode),
        )
        result = run_orchestration(
            workspace=workspace,
            job_dir=job_dir,
            plan_path=plan_path,
            run_id=f"{name}-stage",
            allow_private_sources=True,
        )
        assert result.ok

    signatures = {
        name: _receipt_signature(job_dir)
        for name, (_, job_dir) in workspaces.items()
    }
    assert all(signature == signatures["cli"] for signature in signatures.values())
