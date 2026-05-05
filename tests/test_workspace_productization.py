import json
from pathlib import Path
import shutil
import sys
import tomllib

import yaml
from typer.testing import CliRunner

from academic_prep.cli import app


def test_init_workspace_creates_user_layout_and_default_resources(tmp_path):
    workspace = tmp_path / "my-academic-apps"
    runner = CliRunner()

    result = runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    assert result.exit_code == 0
    assert (workspace / "academic-prep.yaml").exists()
    assert (workspace / ".env.example").exists()
    assert (workspace / ".gitignore").exists()
    assert (workspace / "profile" / "profile.yaml").exists()
    assert (workspace / "profile" / "typst" / "cv.typ").exists()
    assert (workspace / "jobs" / ".gitkeep").exists()
    assert (workspace / "job_leads" / ".gitkeep").exists()
    assert (workspace / "prompts" / "job_parser.md").exists()
    assert (workspace / "templates" / "typst" / "cover_letter.typ").exists()
    assert (workspace / "schemas" / "parsed_job.schema.json").exists()
    assert (workspace / "agent-skills" / "academic-application-prep" / "SKILL.md").exists()
    config = yaml.safe_load((workspace / "academic-prep.yaml").read_text())
    profile_manifest = yaml.safe_load((workspace / "profile" / "profile.yaml").read_text())
    assert config["profile_dir"] == "profile"
    assert config["jobs_dir"] == "jobs"
    assert profile_manifest["profile_mode"] == "typst"


def test_init_workspace_does_not_overwrite_local_prompt_by_default(tmp_path):
    workspace = tmp_path / "workspace"
    local_prompt = workspace / "prompts" / "job_parser.md"
    local_prompt.parent.mkdir(parents=True)
    local_prompt.write_text("custom prompt\n")
    runner = CliRunner()

    result = runner.invoke(app, ["init-workspace", "--workspace", str(workspace)])

    assert result.exit_code == 0
    assert local_prompt.read_text() == "custom prompt\n"


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


def test_pyproject_packages_runtime_resources():
    config = tomllib.loads(Path("pyproject.toml").read_text())
    force_include = config["tool"]["hatch"]["build"]["targets"]["wheel"]["force-include"]

    assert force_include["prompts"] == "academic_prep/resources/prompts"
    assert force_include["templates"] == "academic_prep/resources/templates"
    assert force_include["schemas"] == "academic_prep/resources/schemas"
    assert force_include["agent-skills"] == "academic_prep/resources/agent-skills"
    assert force_include["examples"] == "academic_prep/resources/examples"
