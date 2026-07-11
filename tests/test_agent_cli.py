from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

import yaml
from typer.testing import CliRunner

from canisend import __version__
from canisend.agent_protocol import AGENT_PROTOCOL, AGENT_SCHEMA_VERSION
from canisend.cli import app


def test_agent_capabilities_reports_versioned_phase_one_contract() -> None:
    result = CliRunner().invoke(app, ["agent", "capabilities", "--format", "json"])
    payload = json.loads(result.stdout)

    assert result.exit_code == 0
    assert result.stdout.count("\n") == 1
    assert payload["protocol"] == AGENT_PROTOCOL
    assert payload["schema_version"] == AGENT_SCHEMA_VERSION
    assert payload["operation"] == "agent.capabilities"
    assert payload["ok"] is True
    capabilities = payload["capabilities"]
    assert capabilities["package_version"] == __version__
    assert capabilities["protocol_versions"] == [AGENT_PROTOCOL]
    assert capabilities["schema_versions"] == [AGENT_SCHEMA_VERSION]
    assert {
        "agent.capabilities",
        "agent.context",
        "workspace.inspect",
        "job.intake",
        "job.intake_from_lead",
        "job.list",
        "package.check",
        "workflow.stage_status",
        "workflow.stage_prepare",
        "workflow.stage_submit",
        "workflow.stage_apply",
        "workflow.stage_cancel",
        "workflow.stage_run",
    } <= set(capabilities["operations"])
    assert set(capabilities["intake_types"]) == {
        "manual_metadata",
        "local_text",
        "local_pdf",
        "explicit_url",
        "feed_lead",
    }
    assert set(capabilities["execution_modes"]) == {
        "local_service",
        "host_agent",
        "configured_provider",
    }


def test_agent_capabilities_is_workspace_and_provider_independent(monkeypatch) -> None:
    def fail(*args, **kwargs):
        raise AssertionError("capability discovery must not inspect a workspace or call a provider")

    monkeypatch.setattr("canisend.cli.workspace_report", fail, raising=False)
    monkeypatch.setattr("canisend.llm.subprocess.run", fail)
    monkeypatch.setattr("canisend.llm.urlopen", fail)
    monkeypatch.setattr("canisend.llm.provider_from_config", fail)

    result = CliRunner().invoke(app, ["agent", "capabilities", "--format", "json"])

    assert result.exit_code == 0
    assert json.loads(result.stdout)["operation"] == "agent.capabilities"


def test_agent_context_without_job_returns_safe_workspace_report(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    before = _workspace_digest(workspace)

    result = runner.invoke(
        app,
        ["agent", "context", "--workspace", str(workspace), "--format", "json"],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 0
    assert payload["operation"] == "agent.context"
    assert payload["job"] is None
    assert any(artifact["path"] == "canisend.yaml" for artifact in payload["artifacts"])
    assert str(workspace) not in result.stdout
    assert _workspace_digest(workspace) == before


def test_agent_context_with_job_returns_derived_snapshot_without_private_bodies(
    tmp_path: Path,
    monkeypatch,
) -> None:
    workspace = _context_workspace(tmp_path)
    before = _workspace_digest(workspace)

    def fail_provider_call(*args, **kwargs):
        raise AssertionError("agent context must not call a provider or the network")

    monkeypatch.setattr("canisend.llm.subprocess.run", fail_provider_call)
    monkeypatch.setattr("canisend.llm.urlopen", fail_provider_call)
    monkeypatch.setattr("canisend.llm.provider_from_config", fail_provider_call)

    result = CliRunner().invoke(
        app,
        [
            "agent",
            "context",
            "--workspace",
            str(workspace),
            "--job",
            "jobs/example-role",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 0
    assert payload["operation"] == "agent.context"
    assert payload["job"]["id"] == "example-role"
    assert payload["job"]["path"] == "jobs/example-role"
    assert payload["workflow"] == {
        "phase": "parse",
        "readiness": "ready_for_next_stage",
        "derived": True,
    }
    assert payload["missing_fields"] == ["parsed_job.json"]
    assert [consent["id"] for consent in payload["required_consents"]] == ["read-full-job-advert"]
    assert [action["id"] for action in payload["next_actions"]] == ["job.parse"]
    assert "PRIVATE ADVERT BODY" not in result.stdout
    assert "PRIVATE PROFILE BODY" not in result.stdout
    assert "private-token" not in result.stdout
    assert str(workspace) not in result.stdout
    assert _workspace_digest(workspace) == before


def test_agent_context_missing_job_returns_stable_json_error(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    workspace.mkdir()
    (workspace / "canisend.yaml").write_text("jobs_dir: jobs\n", encoding="utf-8")

    result = CliRunner().invoke(
        app,
        [
            "agent",
            "context",
            "--workspace",
            str(workspace),
            "--job",
            "jobs/missing-role",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 1
    assert payload["operation"] == "agent.context"
    assert payload["ok"] is False
    assert payload["error"]["code"] == "job.not_found"
    assert str(workspace) not in result.stdout


def test_agent_context_missing_workspace_configuration_is_json_error(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    workspace.mkdir()

    result = CliRunner().invoke(
        app,
        ["agent", "context", "--workspace", str(workspace), "--format", "json"],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 1
    assert result.stdout.count("\n") == 1
    assert payload["ok"] is False
    assert payload["error"]["code"] == "workspace.not_initialized"
    assert str(workspace) not in result.stdout


def test_agent_context_invalid_job_metadata_is_json_error(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "invalid-role"
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text("jobs_dir: jobs\n", encoding="utf-8")
    (job_dir / "job.yaml").write_text("- invalid\n- metadata\n", encoding="utf-8")

    result = CliRunner().invoke(
        app,
        [
            "agent",
            "context",
            "--workspace",
            str(workspace),
            "--job",
            "jobs/invalid-role",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 1
    assert payload["error"]["code"] == "job.invalid_metadata"
    assert "invalid\n- metadata" not in result.stdout


def test_agent_text_presenter_uses_the_same_typed_context(tmp_path: Path) -> None:
    workspace = _context_workspace(tmp_path)
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "agent",
            "context",
            "--workspace",
            str(workspace),
            "--job",
            "jobs/example-role",
            "--format",
            "text",
        ],
    )

    assert result.exit_code == 0
    assert "Operation: agent.context" in result.stdout
    assert "Workflow: parse (ready_for_next_stage)" in result.stdout
    assert "Next action: job.parse" in result.stdout
    assert "PRIVATE ADVERT BODY" not in result.stdout


def test_agent_context_and_job_list_prioritize_active_decision_spine(tmp_path: Path) -> None:
    workspace = _context_workspace(tmp_path)
    job = "jobs/example-role"
    parsed = CliRunner().invoke(
        app,
        [
            "stage",
            "run",
            "--workspace",
            str(workspace),
            "--job",
            job,
            "--stage",
            "parse",
            "--format",
            "json",
        ],
    )
    assert parsed.exit_code == 0, parsed.output

    context = CliRunner().invoke(
        app,
        [
            "agent",
            "context",
            "--workspace",
            str(workspace),
            "--job",
            job,
            "--format",
            "json",
        ],
    )
    listed = CliRunner().invoke(
        app,
        ["list-jobs", "--workspace", str(workspace), "--format", "json"],
    )
    context_payload = json.loads(context.stdout)
    listed_payload = json.loads(listed.stdout)

    assert context.exit_code == 0
    assert [item["id"] for item in context_payload["next_actions"]] == [
        "stage.run_confirm"
    ]
    assert context_payload["workflow"]["phase"] == "unknown"
    assert listed.exit_code == 0
    assert listed_payload["jobs"][0]["next_action"]["id"] == "stage.run_confirm"
    assert "package.generate" not in context.stdout
    assert "package.generate" not in listed.stdout


def _context_workspace(tmp_path: Path) -> Path:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "example-role"
    profile_dir = workspace / "profile"
    source = profile_dir / "cv.md"
    evidence = profile_dir / "generated" / "cv.evidence.md"
    job_dir.mkdir(parents=True)
    evidence.parent.mkdir(parents=True)
    source.write_text("PRIVATE PROFILE BODY\n", encoding="utf-8")
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\n",
        encoding="utf-8",
    )
    evidence.write_text("generated evidence\n", encoding="utf-8")
    os.utime(source, (100, 100))
    os.utime(evidence, (200, 200))
    (profile_dir / "profile.yaml").write_text(
        yaml.safe_dump(
            {
                "sources": {"cv": "cv.md"},
                "generated": {"cv_evidence": "generated/cv.evidence.md"},
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "example-role",
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/jobs/1?token=private-token",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": "direct and evidence-led",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job_dir / "job_advert.md").write_text(
        "# Full role\n\nPRIVATE ADVERT BODY\n",
        encoding="utf-8",
    )
    return workspace


def _workspace_digest(workspace: Path) -> dict[str, str]:
    return {
        path.relative_to(workspace).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(workspace.rglob("*"))
        if path.is_file()
    }
