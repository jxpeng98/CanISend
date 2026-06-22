import re
import sys

from typer.testing import CliRunner

from canisend import __version__
from canisend.cli import app
from canisend.versioning import PyPIVersionInfo


ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")


def strip_ansi(text: str) -> str:
    return ANSI_ESCAPE_RE.sub("", text)


def test_cli_help_shows_core_commands():
    runner = CliRunner()

    result = runner.invoke(app, ["--help"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "--version" in output
    assert "version" in output
    assert "init-profile" in output
    assert "init-workspace" in output
    assert "new-job" in output
    assert "new-job-from-lead" in output
    assert "list-jobs" in output
    assert "fetch-jobs-ac-uk" in output
    assert "extract-profile-evidence" in output
    assert "run" in output
    assert "check-package" in output
    assert "orchestrate" in output
    assert "render-typst" in output


def test_run_help_shows_llm_draft_flag():
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--help"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "--llm-parser" in output
    assert "--llm-drafts" in output
    assert "--git-add-materials" in output
    assert "--no-git-add-materi" in output


def test_extract_profile_evidence_help_shows_llm_augment_flag():
    runner = CliRunner()

    result = runner.invoke(app, ["extract-profile-evidence", "--help"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "--llm-augment" in output


def test_orchestrate_help_shows_profile_input_edit_safety_flags():
    runner = CliRunner()

    result = runner.invoke(app, ["orchestrate", "--help"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "--allow-profile-input-edits" in output
    assert "--confirm-profile-input-edit" in output
    assert "Second confirmation" in output


def test_cli_version_option_shows_local_and_remote_versions(monkeypatch):
    runner = CliRunner()

    monkeypatch.setattr(
        "canisend.cli.fetch_remote_versions",
        lambda: PyPIVersionInfo(stable="0.2.1", prerelease="0.3.0rc1"),
    )

    result = runner.invoke(app, ["--version"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "CanISend version" in output
    assert f"Local package      {__version__}" in output
    assert "Remote stable      0.2.1" in output
    assert "Remote prerelease  0.3.0rc1" in output
    assert "Stable update available: 0.2.1" in output
    assert "Prerelease available: 0.3.0rc1" in output
    assert "Upgrade" in output


def test_cli_version_command_shows_local_and_remote_versions(monkeypatch):
    runner = CliRunner()

    monkeypatch.setattr(
        "canisend.cli.fetch_remote_versions",
        lambda: PyPIVersionInfo(stable="0.2.0", prerelease="0.3.0rc1"),
    )

    result = runner.invoke(app, ["version"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "CanISend version" in output
    assert f"Local package      {__version__}" in output
    assert "Remote stable      0.2.0" in output
    assert "Remote prerelease  0.3.0rc1" in output


def test_orchestrate_dry_run_lists_ready_tasks(tmp_path):
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "job"
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text("jobs_dir: jobs\n", encoding="utf-8")
    (job_dir / "parsed_job.json").write_text('{"title": "Lecturer"}\n', encoding="utf-8")
    worker = tmp_path / "worker.py"
    worker.write_text("print('ok')\n", encoding="utf-8")
    plan = tmp_path / "plan.yaml"
    plan.write_text(
        f"""
workers:
  python:
    command: "{sys.executable} {worker}"
    max_parallel_tasks: 1
tasks:
  - id: review
    worker: python
    role: job_parser_reviewer
    privacy_tier: 1
    inputs:
      - parsed_job.json
    outputs:
      - orchestration/reviews/review.md
    writes:
      - orchestration/reviews/review.md
""",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(
        app,
        ["orchestrate", "--workspace", str(workspace), "--job", "job", "--plan", str(plan), "--dry-run"],
    )

    assert result.exit_code == 0
    assert "review: ready" in result.output


def test_orchestrate_executes_plan_and_reports_run_dir(tmp_path):
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "job"
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text("jobs_dir: jobs\n", encoding="utf-8")
    (job_dir / "parsed_job.json").write_text('{"title": "Lecturer"}\n', encoding="utf-8")
    worker = tmp_path / "worker.py"
    worker.write_text(
        "import sys\n"
        "prompt = sys.stdin.read()\n"
        "print('CLI:' + prompt.splitlines()[0])\n",
        encoding="utf-8",
    )
    plan = tmp_path / "plan.yaml"
    plan.write_text(
        f"""
workers:
  python:
    command: "{sys.executable} {worker}"
    max_parallel_tasks: 1
tasks:
  - id: review
    worker: python
    role: job_parser_reviewer
    privacy_tier: 1
    inputs:
      - parsed_job.json
    outputs:
      - orchestration/reviews/review.md
    writes:
      - orchestration/reviews/review.md
""",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(app, ["orchestrate", "--workspace", str(workspace), "--job", "job", "--plan", str(plan)])

    assert result.exit_code == 0
    assert "Orchestration run:" in result.output
    assert "review: succeeded" in result.output
    assert "CLI:Role: job_parser_reviewer" in (job_dir / "orchestration" / "reviews" / "review.md").read_text()


def test_orchestrate_profile_input_edits_require_two_cli_confirmations(tmp_path):
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "job"
    profile_dir = workspace / "profile" / "generated"
    job_dir.mkdir(parents=True)
    profile_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text("jobs_dir: jobs\n", encoding="utf-8")
    (job_dir / "parsed_job.json").write_text('{"title": "Lecturer"}\n', encoding="utf-8")
    (profile_dir / "cv.evidence.md").write_text("# Evidence\n", encoding="utf-8")
    worker = tmp_path / "worker.py"
    worker.write_text("print('ok')\n", encoding="utf-8")
    plan = tmp_path / "plan.yaml"
    plan.write_text(
        f"""
workers:
  python:
    command: "{sys.executable} {worker}"
    max_parallel_tasks: 1
    privacy_tier_limit: 2
tasks:
  - id: review
    worker: python
    role: profile_improvement_reviewer
    privacy_tier: 1
    inputs:
      - parsed_job.json
    outputs:
      - orchestration/reviews/profile.md
    writes:
      - orchestration/reviews/profile.md
  - id: profile-edit
    worker: python
    role: profile_source_editor
    privacy_tier: 2
    inputs:
      - profile/generated/cv.evidence.md
    outputs:
      - profile/typst/cv.typ
    writes:
      - profile/typst/cv.typ
    depends_on:
      - review
    edits_profile_input: true
""",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "orchestrate",
            "--workspace",
            str(workspace),
            "--job",
            "job",
            "--plan",
            str(plan),
            "--dry-run",
            "--allow-private-sources",
            "--allow-profile-input-edits",
            "--confirm-profile-input-edit",
        ],
    )

    assert result.exit_code == 1
    assert "two profile input edit confirmations" in result.output

    result = runner.invoke(
        app,
        [
            "orchestrate",
            "--workspace",
            str(workspace),
            "--job",
            "job",
            "--plan",
            str(plan),
            "--dry-run",
            "--allow-private-sources",
            "--allow-profile-input-edits",
            "--confirm-profile-input-edit",
            "--confirm-profile-input-edit-again",
        ],
    )

    assert result.exit_code == 0
    assert "profile-edit: ready" in result.output
