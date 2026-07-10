from __future__ import annotations

import json
from pathlib import Path

from typer.testing import CliRunner

from canisend.cli import app


def test_agent_contract_survives_fresh_host_session_without_private_body_leaks(tmp_path: Path) -> None:
    expected_capabilities = json.loads(
        Path("examples/agent_handoff/expected_capabilities.json").read_text(encoding="utf-8")
    )
    expected_context_shape = json.loads(
        Path("examples/agent_handoff/expected_context_shape.json").read_text(encoding="utf-8")
    )
    workspace = tmp_path / "workspace"
    advert_fixture = Path("examples/end_to_end/full_job_advert.md").resolve()
    profile_marker = "FAKE PRIVATE PROFILE BODY MUST NOT APPEAR"
    advert_marker = "Experience contributing to programme administration"

    host_a = CliRunner()
    initialized = host_a.invoke(
        app,
        ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"],
    )
    assert initialized.exit_code == 0
    profile_source = workspace / "profile" / "typst" / "cv.typ"
    profile_source.write_text(profile_source.read_text(encoding="utf-8") + f"\n// {profile_marker}\n")

    capabilities = _invoke_json(host_a, ["agent", "capabilities", "--format", "json"])
    _assert_subset(expected_capabilities, capabilities)

    intake = _invoke_json(
        host_a,
        [
            "new-job",
            "--workspace",
            str(workspace),
            "--title",
            "Lecturer in Applied Economics",
            "--institution",
            "Example University",
            "--deadline",
            "2026-08-01",
            "--english-variant",
            "uk",
            "--writing-style",
            "direct and evidence-led",
            "--advert-file",
            str(advert_fixture),
            "--format",
            "json",
        ],
    )
    job_path = intake["job"]["path"]

    context_args = [
        "agent",
        "context",
        "--workspace",
        str(workspace),
        "--job",
        job_path,
        "--format",
        "json",
    ]
    host_a_context = _invoke_json(host_a, context_args)
    listed = _invoke_json(
        host_a,
        ["list-jobs", "--workspace", str(workspace), "--format", "json"],
    )

    host_b = CliRunner()
    host_b_context = _invoke_json(host_b, context_args)

    assert _without_request_id(host_a_context) == _without_request_id(host_b_context)
    assert set(expected_context_shape["required_top_level"]) <= set(host_b_context)
    assert set(expected_context_shape["required_job_fields"]) <= set(host_b_context["job"])
    assert set(expected_context_shape["required_workflow_fields"]) <= set(host_b_context["workflow"])
    assert listed["jobs"][0]["path"] == job_path

    blocked = _invoke_json(
        host_b,
        [
            "check-package",
            "--workspace",
            str(workspace),
            "--job",
            job_path,
            "--format",
            "json",
        ],
        expected_exit=1,
    )
    assert blocked["ok"] is True
    assert blocked["error"] is None
    assert blocked["gate"]["status"] == "FAIL"
    assert blocked["blockers"]

    all_responses = json.dumps(
        [capabilities, intake, host_a_context, listed, host_b_context, blocked],
        sort_keys=True,
    )
    assert profile_marker not in all_responses
    assert advert_marker not in all_responses
    for forbidden_field in expected_context_shape["forbidden_body_fields"]:
        assert forbidden_field not in all_responses
    assert str(workspace) not in all_responses


def _invoke_json(
    runner: CliRunner,
    args: list[str],
    *,
    expected_exit: int = 0,
) -> dict[str, object]:
    result = runner.invoke(app, args)

    assert result.exit_code == expected_exit, result.output
    assert result.stdout.endswith("\n")
    assert result.stdout.count("\n") == 1
    return json.loads(result.stdout)


def _assert_subset(expected: object, actual: object) -> None:
    if isinstance(expected, dict):
        assert isinstance(actual, dict)
        for key, value in expected.items():
            assert key in actual
            _assert_subset(value, actual[key])
        return
    assert expected == actual


def _without_request_id(payload: dict[str, object]) -> dict[str, object]:
    normalized = dict(payload)
    normalized.pop("request_id")
    return normalized
