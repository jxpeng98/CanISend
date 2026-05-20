from typer.testing import CliRunner

from canisend.cli import app


def test_cli_help_shows_core_commands():
    runner = CliRunner()

    result = runner.invoke(app, ["--help"])

    assert result.exit_code == 0
    assert "init-profile" in result.output
    assert "new-job" in result.output
    assert "new-job-from-lead" in result.output
    assert "fetch-jobs-ac-uk" in result.output
    assert "extract-profile-evidence" in result.output
    assert "run" in result.output
    assert "render-typst" in result.output


def test_run_help_shows_llm_draft_flag():
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--help"])

    assert result.exit_code == 0
    assert "--llm-parser" in result.output
    assert "--llm-drafts" in result.output
