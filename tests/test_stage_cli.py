from __future__ import annotations

from datetime import UTC, datetime, timedelta
import json
from pathlib import Path

from typer.testing import CliRunner

from canisend.cli import app
from canisend.stage_models import ArtifactFingerprint, TaskResultV1, TaskSpecV1
from canisend.stage_store import sha256_bytes
from canisend.stages.parse_stage import build_deterministic_parse_candidate


def _workspace(tmp_path: Path) -> tuple[Path, str]:
    workspace = tmp_path / "workspace"
    advert = tmp_path / "advert.md"
    advert.write_text(
        """# Lecturer in Economics

Department: Economics
Location: London
Required documents: CV, Cover letter

Essential criteria:
- PhD in Economics
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
            "--english-variant",
            "uk",
            "--writing-style",
            "direct",
            "--format",
            "json",
        ],
    )
    assert initialized.exit_code == 0
    assert created.exit_code == 0
    job_path = json.loads(created.stdout)["job"]["path"]
    return workspace, job_path


def _invoke_json(runner: CliRunner, args: list[str], *, exit_code: int = 0) -> dict[str, object]:
    result = runner.invoke(app, args)

    assert result.exit_code == exit_code, result.output
    assert result.stdout.count("\n") == 1
    return json.loads(result.stdout)


def test_stage_status_is_read_only_and_machine_safe(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)
    job_dir = workspace / job_path

    payload = _invoke_json(
        CliRunner(),
        [
            "stage",
            "status",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--format",
            "json",
        ],
    )

    assert payload["operation"] == "workflow.stage_status"
    assert payload["extensions"]["canisend.stage_id"] == "parse"
    assert payload["extensions"]["canisend.stage_status"] == "ready"
    assert payload["extensions"]["canisend.output_drift"] is False
    assert not (job_dir / "workflow").exists()
    assert str(workspace) not in json.dumps(payload)


def test_stage_prepare_and_fresh_cli_status_share_task_state(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)
    first_host = CliRunner()
    prepared = _invoke_json(
        first_host,
        [
            "stage",
            "prepare",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--mode",
            "host-agent",
            "--format",
            "json",
        ],
    )
    task_artifact = next(
        item for item in prepared["artifacts"] if item["kind"] == "stage_task_spec"
    )

    second_host = CliRunner()
    status = _invoke_json(
        second_host,
        [
            "stage",
            "status",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--format",
            "json",
        ],
    )

    assert prepared["extensions"]["canisend.reused"] is False
    assert status["extensions"]["canisend.stage_status"] == "running"
    assert any(
        item["kind"] == "stage_task_spec" and item["path"] == task_artifact["path"]
        for item in status["artifacts"]
    )


def test_stage_run_reports_cache_hit_without_rewriting_output(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)
    args = [
        "stage",
        "run",
        "--workspace",
        str(workspace),
        "--job",
        job_path,
        "--stage",
        "parse",
        "--mode",
        "deterministic",
        "--format",
        "json",
    ]
    runner = CliRunner()
    first = _invoke_json(runner, args)
    parsed_path = workspace / job_path / "parsed_job.json"
    first_mtime = parsed_path.stat().st_mtime_ns
    second = _invoke_json(CliRunner(), args)

    assert first["extensions"]["canisend.cache_hit"] is False
    assert second["extensions"]["canisend.cache_hit"] is True
    assert parsed_path.stat().st_mtime_ns == first_mtime
    assert any(item["kind"] == "parsed_job" for item in second["artifacts"])


def test_stage_apply_promotes_host_candidate_through_cli(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)
    prepare_payload = _invoke_json(
        CliRunner(),
        [
            "stage",
            "prepare",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--mode",
            "host-agent",
            "--format",
            "json",
        ],
    )
    task_workspace_path = next(
        item["path"]
        for item in prepare_payload["artifacts"]
        if item["kind"] == "stage_task_spec"
    )
    job_dir = workspace / job_path
    task_job_path = str(Path(task_workspace_path).relative_to(job_path))
    spec = TaskSpecV1.model_validate(
        json.loads((workspace / task_workspace_path).read_text(encoding="utf-8"))
    )
    candidate = build_deterministic_parse_candidate(job_dir)
    candidate_bytes = (json.dumps(candidate, indent=2, sort_keys=True) + "\n").encode()
    candidate_path = job_dir / spec.candidate_output
    candidate_path.write_bytes(candidate_bytes)
    result = TaskResultV1(
        task_id=spec.task_id,
        run_id=spec.run_id,
        job_id=spec.job_id,
        stage="parse",
        status="succeeded",
        input_fingerprint=spec.input_fingerprint,
        started_at=spec.created_at,
        completed_at=max(datetime.now(UTC), spec.created_at + timedelta(microseconds=1)),
        outputs=(
            ArtifactFingerprint(
                path=spec.candidate_output,
                sha256=sha256_bytes(candidate_bytes),
                size_bytes=len(candidate_bytes),
            ),
        ),
    )
    result_path = job_dir / spec.result_output
    result_path.write_text(
        json.dumps(result.model_dump(mode="json"), sort_keys=True),
        encoding="utf-8",
    )

    applied = _invoke_json(
        CliRunner(),
        [
            "stage",
            "apply",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--task",
            task_job_path,
            "--result",
            spec.result_output,
            "--format",
            "json",
        ],
    )

    assert applied["operation"] == "workflow.stage_apply"
    assert applied["extensions"]["canisend.stage_status"] == "succeeded"
    assert (job_dir / "parsed_job.json").is_file()


def test_stage_cli_returns_stable_safe_error_for_unsupported_stage(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)

    payload = _invoke_json(
        CliRunner(),
        [
            "stage",
            "prepare",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "draft",
            "--format",
            "json",
        ],
        exit_code=1,
    )

    assert payload["ok"] is False
    assert payload["error"]["code"] == "stage.unsupported"
    assert str(workspace) not in json.dumps(payload)
    assert "private=token" not in json.dumps(payload)
