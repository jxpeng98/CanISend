import json
from types import SimpleNamespace

from typer.testing import CliRunner

from canisend.bundle_models import ArtifactBundleV1, BundleEntryV1
from canisend.cli import app


def test_render_typst_compiles_generated_typst_files_to_pdf(tmp_path):
    job_dir = tmp_path / "jobs" / "job"
    typst_dir = job_dir / "typst"
    typst_dir.mkdir(parents=True)
    (typst_dir / "cover_letter.typ").write_text("#text[Cover letter]")
    (typst_dir / "application_package.typ").write_text("#text[Application package]")
    (typst_dir / "research_statement.typ").write_text("#text[Research statement]")
    (typst_dir / "notes.typ").write_text("#text[Do not render]")
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
    assert (job_dir / "pdf" / "research_statement.pdf").read_text().startswith("fake pdf")
    assert not (job_dir / "pdf" / "notes.pdf").exists()


def test_render_typst_rejects_pending_generated_candidate(tmp_path):
    job_dir = tmp_path / "jobs" / "job"
    typst_dir = job_dir / "typst"
    typst_dir.mkdir(parents=True)
    (typst_dir / "cover_letter.typ").write_text("#text[Cover letter]")
    (typst_dir / "application_package.typ").write_text("#text[Application package]")
    candidate_path = typst_dir / "cover_letter.generated.typ"
    candidate_path.write_text("#text[Pending candidate]")
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
    assert "generated Typst candidates require reconciliation" in result.output
    assert candidate_path.name in result.output
    assert not (job_dir / "pdf").exists()


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


def test_render_typst_routes_guarded_package_through_render_stage(
    tmp_path,
    monkeypatch,
):
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "job"
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text("jobs_dir: jobs\n")
    (job_dir / "package_bundle.json").write_text("{}\n")
    bundle = ArtifactBundleV1(
        job_id=job_dir.name,
        stage="render",
        input_fingerprint="a" * 64,
        entries=(
            BundleEntryV1.from_bytes(
                path="pdf/cover_letter.pdf",
                media_type="application/pdf",
                data=b"%PDF-1.7 guarded",
            ),
        ),
    )
    bundle_path = job_dir / "render_bundle.json"
    bundle_path.write_text(
        json.dumps(bundle.model_dump(mode="json"), sort_keys=True) + "\n"
    )
    calls = []

    def run_guarded(root, job, *, typst_bin):
        calls.append((root, job, typst_bin))
        return SimpleNamespace(authoritative_path=bundle_path)

    monkeypatch.setattr(
        "canisend.cli.run_render_stage_with_compiler",
        run_guarded,
    )

    result = CliRunner().invoke(
        app,
        [
            "render-typst",
            "--workspace",
            str(workspace),
            "--job",
            "job",
            "--typst-bin",
            "explicit-typst",
        ],
    )

    assert result.exit_code == 0
    assert calls == [(workspace.resolve(), job_dir.resolve(), "explicit-typst")]
    assert (job_dir / "pdf" / "cover_letter.pdf").read_bytes() == b"%PDF-1.7 guarded"
    assert (job_dir / "workflow" / "projections" / "render.json").is_file()
