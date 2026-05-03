from typer.testing import CliRunner

from academic_prep.cli import app


def test_cli_help_shows_core_commands():
    runner = CliRunner()

    result = runner.invoke(app, ["--help"])

    assert result.exit_code == 0
    assert "init-profile" in result.output
    assert "new-job" in result.output
    assert "run" in result.output
    assert "render-typst" in result.output
