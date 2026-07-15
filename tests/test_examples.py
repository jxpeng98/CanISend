import json
from pathlib import Path
import shutil
import sys

from typer.testing import CliRunner

from canisend.cli import app


def test_run_example_command_creates_complete_local_workspace(tmp_path, monkeypatch):
    workspace = tmp_path / "canisend-example"
    runner = CliRunner()
    monkeypatch.setenv("OPENAI_API_KEY", "secret-from-user-shell")

    result = runner.invoke(app, ["run-example", "--workspace", str(workspace)])

    assert result.exit_code == 0, result.output
    assert "Example workflow complete" in result.output
    assert "jobs/2026-06-15_example-university_lecturer-in-applied-economics" in result.output
    assert (workspace / "canisend.yaml").exists()
    assert (workspace / "example_inputs" / "jobs_ac_uk_sample.xml").exists()
    assert (workspace / "example_inputs" / "full_job_advert.md").exists()
    assert (workspace / "example_inputs" / "fake_llm_provider.py").exists()
    assert (workspace / "job_leads" / "jobs_ac_uk.json").exists()
    assert (workspace / "profile" / "generated" / "cv.evidence.md").exists()

    job_dir = workspace / "jobs" / "2026-06-15_example-university_lecturer-in-applied-economics"
    parsed_job = json.loads((job_dir / "parsed_job.json").read_text())
    cover_content = json.loads((job_dir / "typst" / "cover_letter_content.json").read_text())
    package_content = json.loads((job_dir / "typst" / "application_package_content.json").read_text())
    job_metadata = (job_dir / "job.yaml").read_text()
    final_package = (job_dir / "06_final_application_package.md").read_text()

    assert parsed_job["title"] == "Lecturer in Applied Economics"
    assert cover_content["recipient"]["institution"] == "Example University"
    assert final_package.startswith("# Final Application Package\n")
    assert "profile/generated/cv.evidence.md#Teaching" in package_content["cover_letter"]
    assert (job_dir / "07_material_review_checklist.md").exists()
    assert (job_dir / "typst" / "application_package.typ").exists()
    assert "status: packaged" in job_metadata


def test_run_example_overwrite_refuses_unmarked_workspace(tmp_path):
    workspace = tmp_path / "private-workspace"
    private_file = workspace / "profile" / "cv.typ"
    private_file.parent.mkdir(parents=True)
    private_file.write_text("real private cv\n")
    runner = CliRunner()

    result = runner.invoke(app, ["run-example", "--workspace", str(workspace), "--overwrite"])

    assert result.exit_code != 0
    assert "CanISend example workspace" in result.output
    assert private_file.read_text() == "real private cv\n"


def test_run_example_reports_only_body_free_internal_failure_code(
    tmp_path, monkeypatch
):
    private_detail = "PRIVATE EXAMPLE FAILURE DETAIL"

    def fail_example(*args, **kwargs):
        raise RuntimeError(private_detail)

    monkeypatch.setattr("canisend.cli.run_packaged_example", fail_example)
    result = CliRunner().invoke(
        app,
        ["run-example", "--workspace", str(tmp_path / "example")],
    )

    assert result.exit_code == 1
    assert "example.RuntimeError" in result.output
    assert private_detail not in result.output
    assert "Traceback" not in result.output


def test_end_to_end_example_runs_full_local_workflow(tmp_path, monkeypatch):
    root = Path(__file__).resolve().parents[1]
    example_dir = root / "examples" / "end_to_end"
    profile_dir = tmp_path / "profile"
    jobs_dir = tmp_path / "jobs"
    leads_file = tmp_path / "job_leads" / "jobs_ac_uk.json"
    full_advert = example_dir / "full_job_advert.md"
    fake_provider = example_dir / "fake_llm_provider.py"
    runner = CliRunner()

    shutil.copytree(example_dir / "profile", profile_dir)

    fetch_result = runner.invoke(
        app,
        [
            "fetch-jobs-ac-uk",
            "--rss-file",
            str(example_dir / "jobs_ac_uk_sample.xml"),
            "--output",
            str(leads_file),
            "--include",
            "economics",
        ],
    )
    assert fetch_result.exit_code == 0
    assert json.loads(leads_file.read_text())[0]["title"] == "Lecturer in Applied Economics"

    job_result = runner.invoke(
        app,
        [
            "new-job-from-lead",
            "--leads-file",
            str(leads_file),
            "--lead-index",
            "0",
            "--institution",
            "Example University",
            "--deadline",
            "2026-06-15",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )
    assert job_result.exit_code == 0
    job_dir = jobs_dir / "2026-06-15_example-university_lecturer-in-applied-economics"
    shutil.copyfile(full_advert, job_dir / "job_advert.md")

    evidence_result = runner.invoke(app, ["extract-profile-evidence", "--profile-dir", str(profile_dir)])
    assert evidence_result.exit_code == 0
    assert (profile_dir / "generated" / "cv.evidence.md").exists()

    monkeypatch.setenv("ACADEMIC_PREP_LLM_PROVIDER", "command")
    monkeypatch.setenv("ACADEMIC_PREP_LLM_COMMAND", f"{sys.executable} {fake_provider}")
    run_result = runner.invoke(
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
    assert run_result.exit_code == 0

    parsed_job = json.loads((job_dir / "parsed_job.json").read_text())
    cover_content = json.loads((job_dir / "typst" / "cover_letter_content.json").read_text())
    package_content = json.loads((job_dir / "typst" / "application_package_content.json").read_text())
    cover_source = (job_dir / "typst" / "cover_letter.typ").read_text()
    final_package = (job_dir / "06_final_application_package.md").read_text()

    assert parsed_job["title"] == "Lecturer in Applied Economics"
    assert cover_content["recipient"]["institution"] == "Example University"
    assert "econometrics teaching" in cover_content["sections"]["teaching_fit"]
    assert final_package.startswith("# Final Application Package\n")
    assert "profile/generated/cv.evidence.md#Teaching" in package_content["cover_letter"]
    assert 'json("cover_letter_content.json")' not in cover_source
    assert "// CANISEND: section research_fit" in cover_source
    assert '@preview/modernpro-coverletter:0.0.8' in cover_source
