from __future__ import annotations

import json
from pathlib import Path

from typer.testing import CliRunner

from canisend.cli import app
from canisend.stage_models import TaskSpecV1
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
    assert [item["id"] for item in status["required_consents"]] == [
        "read-full-job-advert"
    ]
    assert status["next_actions"][0]["id"] == "stage.submit_parse_candidate"
    assert status["next_actions"][0]["requires_consent"] is True
    assert any(
        item["kind"] == "stage_task_spec" and item["path"] == task_artifact["path"]
        for item in status["artifacts"]
    )


def test_stage_cancel_clears_active_task_through_agent_contract(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)
    _invoke_json(
        CliRunner(),
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

    cancelled = _invoke_json(
        CliRunner(),
        [
            "stage",
            "cancel",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
    )
    status = _invoke_json(
        CliRunner(),
        [
            "stage",
            "status",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
    )

    assert cancelled["operation"] == "workflow.stage_cancel"
    assert cancelled["extensions"]["canisend.stage_status"] == "cancelled"
    assert cancelled["workflow"]["readiness"] == "action_required"
    assert cancelled["error"] is None
    assert any(item["kind"] == "stage_run_manifest" for item in cancelled["artifacts"])
    assert status["extensions"]["canisend.stage_status"] == "cancelled"
    assert status["workflow"]["readiness"] == "action_required"
    assert [item["id"] for item in status["next_actions"]] == ["stage.prepare_parse"]

    missing = _invoke_json(
        CliRunner(),
        [
            "stage",
            "cancel",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
        exit_code=1,
    )
    assert missing["error"]["code"] == "stage.no_active_run"
    assert str(workspace) not in json.dumps(missing)


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
    assert [item["id"] for item in first["next_actions"]] == ["stage.run_confirm"]
    assert [item["id"] for item in second["next_actions"]] == ["stage.run_confirm"]
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
    source_candidate = tmp_path / "candidate.json"
    source_candidate.write_bytes(candidate_bytes)
    submitted = _invoke_json(
        CliRunner(),
        [
            "stage",
            "submit",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--task",
            task_job_path,
            "--candidate-file",
            str(source_candidate),
            "--format",
            "json",
        ],
    )
    assert submitted["operation"] == "workflow.stage_submit"
    resumed = _invoke_json(
        CliRunner(),
        [
            "stage",
            "status",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
    )
    resumed_kinds = {item["kind"] for item in resumed["artifacts"]}
    assert "parsed_job_candidate" in resumed_kinds
    assert "stage_task_result" in resumed_kinds
    assert [item["id"] for item in resumed["required_consents"]] == [
        "read-full-job-advert"
    ]
    assert resumed["next_actions"][0]["id"] == "stage.apply_parse_candidate"
    assert resumed["next_actions"][0]["requires_consent"] is True

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
    assert [item["id"] for item in applied["next_actions"]] == ["stage.run_confirm"]
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


def test_confirm_stage_cli_exposes_reviewable_catalog_without_agent_v1_change(
    tmp_path: Path,
) -> None:
    workspace, job_path = _workspace(tmp_path)
    common = [
        "--workspace",
        str(workspace),
        "--job",
        job_path,
        "--mode",
        "deterministic",
        "--format",
        "json",
    ]
    _invoke_json(CliRunner(), ["stage", "run", "--stage", "parse", *common])
    confirmed = _invoke_json(
        CliRunner(),
        ["stage", "run", "--stage", "confirm", *common],
    )

    assert confirmed["protocol"] == "canisend.agent/v1"
    assert confirmed["schema_version"] == "1.0.0"
    assert confirmed["workflow"]["phase"] == "unknown"
    assert confirmed["workflow"]["readiness"] == "review_required"
    assert confirmed["extensions"]["canisend.stage_id"] == "confirm"
    assert confirmed["extensions"]["canisend.unresolved_count"] == 1
    criteria = next(item for item in confirmed["artifacts"] if item["kind"] == "criteria_catalog")
    assert criteria["privacy_tier"] == 2
    assert str(workspace) not in json.dumps(confirmed)


def test_evidence_and_match_cli_keep_agent_v1_and_private_bodies_out_of_stdout(
    tmp_path: Path,
) -> None:
    workspace, job_path = _workspace(tmp_path)
    private_marker = "PRIVATE-CLI-EVIDENCE-1842"
    cv_path = workspace / "profile" / "typst" / "cv.typ"
    cv_path.write_text(
        cv_path.read_text(encoding="utf-8")
        + f'\n#education(institution: [Example University], major: [PhD Economics {private_marker}])\n',
        encoding="utf-8",
    )
    extracted = CliRunner().invoke(
        app,
        [
            "extract-profile-evidence",
            "--profile-dir",
            str(workspace / "profile"),
        ],
    )
    assert extracted.exit_code == 0, extracted.output

    common = [
        "--workspace",
        str(workspace),
        "--job",
        job_path,
        "--format",
        "json",
    ]
    _invoke_json(CliRunner(), ["stage", "run", "--stage", "parse", *common])
    _invoke_json(CliRunner(), ["stage", "run", "--stage", "confirm", *common])
    evidence = _invoke_json(
        CliRunner(),
        ["stage", "run", "--stage", "evidence", *common],
    )
    matched = _invoke_json(
        CliRunner(),
        ["stage", "run", "--stage", "match", *common],
    )

    assert evidence["protocol"] == "canisend.agent/v1"
    assert evidence["workflow"]["phase"] == "evidence"
    assert evidence["extensions"]["canisend.stage_id"] == "evidence"
    assert evidence["extensions"]["canisend.evidence_count"] >= 1
    assert any(item["kind"] == "evidence_catalog" for item in evidence["artifacts"])
    assert matched["protocol"] == "canisend.agent/v1"
    assert matched["workflow"]["phase"] == "unknown"
    assert matched["workflow"]["readiness"] == "review_required"
    assert matched["extensions"]["canisend.stage_id"] == "match"
    assert matched["extensions"]["canisend.match_count"] >= 1
    assert matched["extensions"]["canisend.proposed_count"] >= 1
    matches = next(item for item in matched["artifacts"] if item["kind"] == "criterion_matches")
    assert matches["privacy_tier"] == 2
    assert private_marker not in json.dumps(evidence)
    assert private_marker not in json.dumps(matched)
    assert private_marker not in (workspace / job_path / "criterion_matches.json").read_text(
        encoding="utf-8"
    )


def test_confirm_ready_status_advertises_deterministic_run_action(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)
    _invoke_json(
        CliRunner(),
        [
            "stage",
            "run",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
    )

    status = _invoke_json(
        CliRunner(),
        [
            "stage",
            "status",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "confirm",
            "--format",
            "json",
        ],
    )

    assert status["extensions"]["canisend.stage_status"] == "ready"
    assert [item["id"] for item in status["next_actions"]] == ["stage.run_confirm"]
    parse_status = _invoke_json(
        CliRunner(),
        [
            "stage",
            "status",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
    )
    assert [item["id"] for item in parse_status["next_actions"]] == [
        "stage.run_confirm"
    ]


def test_stage_status_marks_drifted_output_for_review_and_lowers_trust(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)
    _invoke_json(
        CliRunner(),
        [
            "stage",
            "run",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
    )
    parsed_path = workspace / job_path / "parsed_job.json"
    parsed_path.write_text('{"manual":"drift"}\n', encoding="utf-8")

    status = _invoke_json(
        CliRunner(),
        [
            "stage",
            "status",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
    )

    parsed_artifact = next(item for item in status["artifacts"] if item["kind"] == "parsed_job")
    assert status["workflow"]["readiness"] == "review_required"
    assert status["blockers"]
    assert parsed_artifact["trust_level"] == "trusted_local"


def test_confirm_cli_treats_empty_extraction_as_unresolved(tmp_path: Path) -> None:
    workspace, job_path = _workspace(tmp_path)
    (workspace / job_path / "job_advert.md").write_text(
        "# Lecturer in Economics\n\nNo criteria were extracted from this advert.\n",
        encoding="utf-8",
    )
    common = [
        "--workspace",
        str(workspace),
        "--job",
        job_path,
        "--mode",
        "deterministic",
        "--format",
        "json",
    ]
    _invoke_json(CliRunner(), ["stage", "run", "--stage", "parse", *common])
    confirmed = _invoke_json(
        CliRunner(),
        ["stage", "run", "--stage", "confirm", *common],
    )

    assert confirmed["workflow"]["readiness"] == "review_required"
    assert confirmed["extensions"]["canisend.unresolved_count"] == 1
