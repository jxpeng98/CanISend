import re

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
    assert "render-typst" in output


def test_run_help_shows_llm_draft_flag():
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--help"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "--llm-parser" in output
    assert "--llm-drafts" in output


def test_cli_version_option_shows_local_and_remote_versions(monkeypatch):
    runner = CliRunner()

    monkeypatch.setattr(
        "canisend.cli.fetch_remote_versions",
        lambda: PyPIVersionInfo(stable="0.2.0", prerelease="0.3.0rc1"),
    )

    result = runner.invoke(app, ["--version"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "CanISend version" in output
    assert f"Local package      {__version__}" in output
    assert "Remote stable      0.2.0" in output
    assert "Remote prerelease  0.3.0rc1" in output
    assert "Stable update available: 0.2.0" in output
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
