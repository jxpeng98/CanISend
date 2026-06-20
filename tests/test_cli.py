import re

from typer.testing import CliRunner

from canisend.cli import app


ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")


def strip_ansi(text: str) -> str:
    return ANSI_ESCAPE_RE.sub("", text)


def test_cli_help_shows_core_commands():
    runner = CliRunner()

    result = runner.invoke(app, ["--help"])
    output = strip_ansi(result.output)

    assert result.exit_code == 0
    assert "init-profile" in output
    assert "new-job" in output
    assert "new-job-from-lead" in output
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
