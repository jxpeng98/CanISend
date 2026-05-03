from typer.testing import CliRunner

from academic_prep.cli import app


def test_render_typst_compiles_generated_typst_files_to_pdf(tmp_path):
    job_dir = tmp_path / "jobs" / "job"
    typst_dir = job_dir / "typst"
    typst_dir.mkdir(parents=True)
    (typst_dir / "cover_letter.typ").write_text("#text[Cover letter]")
    (typst_dir / "application_package.typ").write_text("#text[Application package]")
    fake_typst = tmp_path / "fake_typst.py"
    fake_typst.write_text(
        "#!/usr/bin/env python3\n"
        "import pathlib\n"
        "import sys\n"
        "output = pathlib.Path(sys.argv[3])\n"
        "output.parent.mkdir(parents=True, exist_ok=True)\n"
        "output.write_text('fake pdf for ' + sys.argv[2])\n"
    )
    fake_typst.chmod(0o755)
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "render-typst",
            "--job",
            str(job_dir),
            "--typst-bin",
            str(fake_typst),
        ],
    )

    assert result.exit_code == 0
    assert (job_dir / "pdf" / "cover_letter.pdf").read_text().startswith("fake pdf")
    assert (job_dir / "pdf" / "application_package.pdf").read_text().startswith("fake pdf")


def test_render_typst_reports_missing_typst_binary(tmp_path):
    job_dir = tmp_path / "jobs" / "job"
    (job_dir / "typst").mkdir(parents=True)
    (job_dir / "typst" / "cover_letter.typ").write_text("#text[Cover letter]")
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "render-typst",
            "--job",
            str(job_dir),
            "--typst-bin",
            str(tmp_path / "missing-typst"),
        ],
    )

    assert result.exit_code != 0
    assert "Typst binary not found" in result.output
