import json
import os
from pathlib import Path
import re
import shutil
import sys
import tomllib

import yaml
from typer.testing import CliRunner

from canisend.cli import app
from canisend.workspace import doctor_lines, workspace_report


def test_init_workspace_creates_user_layout_and_default_resources(tmp_path):
    workspace = tmp_path / "my-academic-apps"
    runner = CliRunner()

    result = runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    assert result.exit_code == 0
    assert (workspace / "canisend.yaml").exists()
    assert (workspace / ".env.example").exists()
    assert (workspace / ".gitignore").exists()
    assert (workspace / "profile" / "profile.yaml").exists()
    assert (workspace / "profile" / "typst" / "cv.typ").exists()
    assert (workspace / "jobs" / ".gitkeep").exists()
    assert (workspace / "job_leads" / ".gitkeep").exists()
    assert (workspace / "prompts" / "job_parser.md").exists()
    assert (workspace / "prompts" / "profile_evidence_augmenter.md").exists()
    assert (workspace / "templates" / "typst" / "cover_letter.typ").exists()
    assert (workspace / "schemas" / "parsed_job.schema.json").exists()
    assert (workspace / "schemas" / "workflow-state.schema.json").exists()
    assert (workspace / "schemas" / "task-spec.schema.json").exists()
    assert (workspace / "schemas" / "task-result.schema.json").exists()
    assert (workspace / "schemas" / "run-manifest.schema.json").exists()
    assert (workspace / "schemas" / "criteria.schema.json").exists()
    assert (workspace / "schemas" / "evidence-catalog.schema.json").exists()
    assert (workspace / "schemas" / "criterion-matches.schema.json").exists()
    assert (workspace / "schemas" / "confirmed-corrections.schema.json").exists()
    assert (workspace / "schemas" / "application-decision.schema.json").exists()
    assert (workspace / "schemas" / "application-brief.schema.json").exists()
    assert (workspace / "schemas" / "required-document-plan.schema.json").exists()
    assert (workspace / "agent-skills" / "canisend" / "SKILL.md").exists()
    assert (workspace / "agent-skills" / "canisend-job-intake" / "SKILL.md").exists()
    assert (workspace / "agent-skills" / "canisend-application-package" / "SKILL.md").exists()
    assert (workspace / "agent-skills" / "canisend-submission-readiness" / "SKILL.md").exists()
    assert (workspace / "AGENTS.md").exists()
    assert (workspace / "CLAUDE.md").exists()
    assert not (workspace / "GEMINI.md").exists()
    assert "agent-skills/canisend/SKILL.md" in (workspace / "AGENTS.md").read_text()
    assert "Agent-assisted mode is not local-only" in (workspace / "AGENTS.md").read_text()
    assert "agent model provider" in (workspace / "CLAUDE.md").read_text()
    config = yaml.safe_load((workspace / "canisend.yaml").read_text())
    profile_manifest = yaml.safe_load((workspace / "profile" / "profile.yaml").read_text())
    assert config["profile_dir"] == "profile"
    assert config["jobs_dir"] == "jobs"
    assert profile_manifest["profile_mode"] == "typst"


def test_workspace_skill_pack_resolves_every_focused_route(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    main_skill = (workspace / "agent-skills" / "canisend" / "SKILL.md").read_text(encoding="utf-8")
    routed_skills = set(re.findall(r"\$(canisend-[a-z0-9-]+)", main_skill))

    assert routed_skills
    assert all(
        (workspace / "agent-skills" / skill_name / "SKILL.md").is_file()
        for skill_name in routed_skills
    )


def test_update_workspace_adds_focused_skills_without_overwriting_local_main_skill(tmp_path):
    workspace = tmp_path / "legacy-workspace"
    local_main = workspace / "agent-skills" / "canisend" / "SKILL.md"
    local_main.parent.mkdir(parents=True)
    local_main.write_text("local workspace customization\n", encoding="utf-8")
    runner = CliRunner()

    result = runner.invoke(app, ["update-workspace", "--workspace", str(workspace)])

    assert result.exit_code == 0
    assert local_main.read_text(encoding="utf-8") == "local workspace customization\n"
    assert (workspace / "agent-skills" / "canisend-job-intake" / "SKILL.md").is_file()
    assert (workspace / "agent-skills" / "canisend-application-package" / "SKILL.md").is_file()
    assert (workspace / "agent-skills" / "canisend-submission-readiness" / "SKILL.md").is_file()


def test_update_workspace_overwrite_refreshes_main_skill_from_canonical_pack(tmp_path):
    workspace = tmp_path / "legacy-workspace"
    local_main = workspace / "agent-skills" / "canisend" / "SKILL.md"
    local_main.parent.mkdir(parents=True)
    local_main.write_text("local workspace customization\n", encoding="utf-8")

    result = CliRunner().invoke(
        app,
        ["update-workspace", "--workspace", str(workspace), "--overwrite"],
    )

    assert result.exit_code == 0
    assert local_main.read_text(encoding="utf-8") == Path("skills/canisend/SKILL.md").read_text(encoding="utf-8")


def test_init_workspace_does_not_overwrite_local_prompt_by_default(tmp_path):
    workspace = tmp_path / "workspace"
    local_prompt = workspace / "prompts" / "job_parser.md"
    local_prompt.parent.mkdir(parents=True)
    local_prompt.write_text("custom prompt\n")
    runner = CliRunner()

    result = runner.invoke(app, ["init-workspace", "--workspace", str(workspace)])

    assert result.exit_code == 0
    assert local_prompt.read_text() == "custom prompt\n"


def test_init_workspace_does_not_overwrite_platform_bridge_by_default(tmp_path):
    workspace = tmp_path / "workspace"
    bridge = workspace / "AGENTS.md"
    bridge.parent.mkdir(parents=True)
    bridge.write_text("custom agent instructions\n")
    runner = CliRunner()

    result = runner.invoke(app, ["init-workspace", "--workspace", str(workspace)])

    assert result.exit_code == 0
    assert bridge.read_text() == "custom agent instructions\n"


def test_doctor_reports_deprecated_workspace_bridge(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    (workspace / "GEMINI.md").write_text("old bridge\n", encoding="utf-8")

    lines = doctor_lines(workspace)

    assert "- Deprecated files: GEMINI.md (run `canisend update-workspace --prune-deprecated`)" in lines


def test_update_workspace_prunes_deprecated_bridge_when_requested(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    (workspace / "GEMINI.md").write_text("old bridge\n", encoding="utf-8")

    result = runner.invoke(app, ["update-workspace", "--workspace", str(workspace), "--prune-deprecated"])

    assert result.exit_code == 0
    assert not (workspace / "GEMINI.md").exists()
    assert "Removed 1 deprecated file." in result.output


def test_run_uses_packaged_prompts_when_workspace_has_no_prompt_overrides(tmp_path, monkeypatch):
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "2026-06-15_example-university_lecturer-in-applied-economics"
    profile_dir = workspace / "profile"
    runner = CliRunner()
    example_dir = Path(__file__).resolve().parents[1] / "examples" / "end_to_end"
    fake_provider = example_dir / "fake_llm_provider.py"

    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    shutil.copytree(example_dir / "profile", profile_dir, dirs_exist_ok=True)
    # Simulate an installed-package user workspace that has no local prompt overrides.
    for path in (workspace / "prompts").glob("*.md"):
        path.unlink()

    runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Applied Economics",
            "--institution",
            "Example University",
            "--deadline",
            "2026-06-15",
            "--jobs-dir",
            str(workspace / "jobs"),
            "--advert-file",
            str(example_dir / "full_job_advert.md"),
        ],
    )
    runner.invoke(app, ["extract-profile-evidence", "--profile-dir", str(profile_dir)])
    monkeypatch.chdir(workspace)
    monkeypatch.setenv("ACADEMIC_PREP_LLM_PROVIDER", "command")
    monkeypatch.setenv("ACADEMIC_PREP_LLM_COMMAND", f"{sys.executable} {fake_provider}")

    result = runner.invoke(
        app,
        [
            "run",
            "--job",
            str(job_dir),
            "--profile-dir",
            str(profile_dir),
            "--llm-parser",
            "--llm-drafts",
        ],
    )

    assert result.exit_code == 0
    parsed_job = json.loads((job_dir / "parsed_job.json").read_text())
    assert parsed_job["title"] == "Lecturer in Applied Economics"


def test_doctor_reports_workspace_and_provider_status(tmp_path, monkeypatch):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    monkeypatch.setenv("ACADEMIC_PREP_LLM_PROVIDER", "command")
    monkeypatch.setenv("ACADEMIC_PREP_LLM_COMMAND", "python examples/end_to_end/fake_llm_provider.py")

    result = runner.invoke(app, ["doctor", "--workspace", str(workspace)])

    assert result.exit_code == 0
    assert "Workspace" in result.output
    assert "profile/profile.yaml" in result.output
    assert "agent skill" in result.output
    assert "LLM provider: command" in result.output


def test_workspace_report_contains_typed_checks_and_version(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    report = workspace_report(workspace, env={})

    assert report.version
    assert report.root == workspace.resolve()
    assert report.check("workspace_config").status == "ok"
    assert report.check("profile_manifest").path == "profile/profile.yaml"
    assert report.check("evidence_freshness").status == "not_generated"
    assert all(check.id and check.label for check in report.checks)


def test_workspace_report_marks_missing_profile_manifest(tmp_path):
    workspace = tmp_path / "workspace"
    workspace.mkdir()

    report = workspace_report(workspace, env={})

    check = report.check("profile_manifest")
    assert check.status == "missing"
    assert check.path == "profile/profile.yaml"


def test_workspace_report_reports_evidence_freshness_without_private_text(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    source = workspace / "profile" / "typst" / "cv.typ"
    source.write_text("PRIVATE APPLICANT BODY", encoding="utf-8")
    manifest = workspace / "profile" / "profile.yaml"
    manifest_data = yaml.safe_load(manifest.read_text(encoding="utf-8"))
    manifest_data["sources"] = {"cv": "typst/cv.typ"}
    manifest.write_text(yaml.safe_dump(manifest_data, sort_keys=False), encoding="utf-8")

    report = workspace_report(workspace, env={})
    rendered = repr(report)

    assert report.check("evidence_freshness").status == "not_generated"
    assert "PRIVATE APPLICANT BODY" not in rendered


def test_workspace_report_has_no_provider_call_side_effect(tmp_path, monkeypatch):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    def fail_provider_call(*args, **kwargs):
        raise AssertionError("workspace inspection must not contact or execute a provider")

    monkeypatch.setattr("canisend.llm.subprocess.run", fail_provider_call)
    monkeypatch.setattr("canisend.llm.urlopen", fail_provider_call)

    report = workspace_report(
        workspace,
        env={
            "ACADEMIC_PREP_LLM_PROVIDER": "command",
            "ACADEMIC_PREP_LLM_COMMAND": "would-run-if-called",
        },
    )

    assert report.check("llm_provider").status == "configured"


def test_doctor_text_output_remains_compatible(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    result = runner.invoke(app, ["doctor", "--workspace", str(workspace), "--format", "text"])

    assert result.exit_code == 0
    assert result.output.splitlines() == doctor_lines(workspace)


def test_doctor_json_output_is_one_valid_agent_response(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    result = runner.invoke(app, ["doctor", "--workspace", str(workspace), "--format", "json"])
    payload = json.loads(result.stdout)

    assert result.exit_code == 0
    assert result.stdout.endswith("\n")
    assert result.stdout.count("\n") == 1
    assert payload["operation"] == "workspace.inspect"
    assert payload["ok"] is True
    assert payload["error"] is None
    assert any(artifact["path"] == "profile/profile.yaml" for artifact in payload["artifacts"])
    assert str(workspace) not in result.stdout


def test_doctor_includes_config_warning_details(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    config_path = workspace / "canisend.yaml"
    config = yaml.safe_load(config_path.read_text())
    config["surprise_key"] = "unexpected"
    config_path.write_text(yaml.safe_dump(config, sort_keys=False))

    config_line = next(line for line in doctor_lines(workspace) if line.startswith("- Config validation:"))

    assert "Unknown key in canisend.yaml: 'surprise_key'" in config_line
    assert "see doctor output" not in config_line


def test_doctor_reports_stale_default_resources(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    (workspace / "prompts" / "job_parser.md").write_text("local stale prompt\n", encoding="utf-8")

    lines = doctor_lines(workspace)

    assert "- Default resources: stale/local edits (prompts/job_parser.md)" in lines


def test_update_workspace_overwrite_restores_stale_default_resources(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    (workspace / "prompts" / "job_parser.md").write_text("local stale prompt\n", encoding="utf-8")

    result = runner.invoke(app, ["update-workspace", "--workspace", str(workspace), "--overwrite"])

    assert result.exit_code == 0
    assert "- Default resources: up to date" in doctor_lines(workspace)


def test_doctor_checks_staleness_for_custom_generated_evidence_path(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    profile_dir = workspace / "profile"
    manifest_path = profile_dir / "profile.yaml"
    manifest = yaml.safe_load(manifest_path.read_text())
    manifest["sources"] = {"cv": "typst/cv.typ"}
    manifest["generated"] = {"cv_evidence": "custom/cv-items.md"}
    manifest_path.write_text(yaml.safe_dump(manifest, sort_keys=False))
    source_path = profile_dir / "typst" / "cv.typ"
    evidence_path = profile_dir / "custom" / "cv-items.md"
    evidence_path.parent.mkdir(parents=True)
    evidence_path.write_text("# Evidence\n")
    os.utime(source_path, (100, 100))
    os.utime(evidence_path, (200, 200))

    lines = doctor_lines(workspace)

    assert "- Evidence staleness: up to date" in lines

    os.utime(source_path, (300, 300))

    lines = doctor_lines(workspace)

    assert "- Evidence staleness: STALE (cv source(s) newer than generated evidence)" in lines


def test_new_job_uses_workspace_config_when_called_from_elsewhere(tmp_path, monkeypatch):
    workspace = tmp_path / "workspace"
    outside = tmp_path / "outside"
    outside.mkdir()
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    config_path = workspace / "canisend.yaml"
    config = yaml.safe_load(config_path.read_text())
    config["jobs_dir"] = "applications"
    config_path.write_text(yaml.safe_dump(config, sort_keys=False))
    monkeypatch.chdir(outside)

    result = runner.invoke(
        app,
        [
            "new-job",
            "--workspace",
            str(workspace),
            "--title",
            "Lecturer in Applied Economics",
            "--institution",
            "Example University",
            "--deadline",
            "2026-06-15",
        ],
    )

    assert result.exit_code == 0
    assert (workspace / "applications" / "2026-06-15_example-university_lecturer-in-applied-economics" / "job.yaml").exists()
    assert not (outside / "applications").exists()


def test_rss_lead_workflow_uses_workspace_default_paths(tmp_path, monkeypatch):
    workspace = tmp_path / "workspace"
    outside = tmp_path / "outside"
    outside.mkdir()
    runner = CliRunner()
    example_dir = Path(__file__).resolve().parents[1] / "examples" / "end_to_end"
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    monkeypatch.chdir(outside)

    fetch_result = runner.invoke(
        app,
        [
            "fetch-jobs-ac-uk",
            "--workspace",
            str(workspace),
            "--rss-file",
            str(example_dir / "jobs_ac_uk_sample.xml"),
            "--include",
            "lecturer",
        ],
    )
    create_result = runner.invoke(
        app,
        [
            "new-job-from-lead",
            "--workspace",
            str(workspace),
            "--lead-index",
            "0",
            "--institution",
            "Example University",
            "--deadline",
            "2026-06-15",
        ],
    )

    assert fetch_result.exit_code == 0
    assert create_result.exit_code == 0
    assert (workspace / "job_leads" / "jobs_ac_uk.json").exists()
    assert (workspace / "jobs" / "2026-06-15_example-university_lecturer-in-applied-economics" / "job_advert.md").exists()
    assert not (outside / "job_leads").exists()
    assert not (outside / "jobs").exists()


def test_run_resolves_job_and_profile_paths_against_workspace(tmp_path, monkeypatch):
    workspace = tmp_path / "workspace"
    outside = tmp_path / "outside"
    outside.mkdir()
    runner = CliRunner()
    example_dir = Path(__file__).resolve().parents[1] / "examples" / "end_to_end"
    job_slug = "2026-06-15_example-university_lecturer-in-applied-economics"
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    shutil.copytree(example_dir / "profile", workspace / "profile", dirs_exist_ok=True)
    runner.invoke(
        app,
        [
            "new-job",
            "--workspace",
            str(workspace),
            "--title",
            "Lecturer in Applied Economics",
            "--institution",
            "Example University",
            "--deadline",
            "2026-06-15",
            "--advert-file",
            str(example_dir / "full_job_advert.md"),
        ],
    )
    monkeypatch.chdir(outside)

    evidence_result = runner.invoke(app, ["extract-profile-evidence", "--workspace", str(workspace)])
    run_result = runner.invoke(app, ["run", "--workspace", str(workspace), "--job", f"jobs/{job_slug}"])

    assert evidence_result.exit_code == 0
    assert run_result.exit_code == 0
    assert (workspace / "jobs" / job_slug / "parsed_job.json").exists()
    assert not (outside / "profile").exists()


def test_pyproject_packages_runtime_resources():
    config = tomllib.loads(Path("pyproject.toml").read_text())
    force_include = config["tool"]["hatch"]["build"]["targets"]["wheel"]["force-include"]

    assert force_include["prompts"] == "canisend/resources/prompts"
    assert force_include["templates"] == "canisend/resources/templates"
    assert force_include["schemas"] == "canisend/resources/schemas"
    assert force_include["skills"] == "canisend/resources/skills"
    assert force_include["agent-skills"] == "canisend/resources/agent-skills"
    assert force_include["platform-bridges"] == "canisend/resources/platform-bridges"
    assert force_include["examples"] == "canisend/resources/examples"
