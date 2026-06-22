import json
from pathlib import Path

import yaml
from typer.testing import CliRunner

from canisend.cli import app


def test_check_package_reports_missing_required_outputs(tmp_path):
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "example-role"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text("status: advert_imported\n", encoding="utf-8")
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 1
    assert "Package check failed" in result.output
    assert "missing parsed_job.json" in result.output
    assert "missing 00_preparation_questions.md" in result.output
    assert "missing 07_material_review_checklist.md" in result.output


def test_check_package_reports_unknown_citations_and_placeholders(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    (job_dir / "03_cover_letter_draft.md").write_text(
        "# Cover Letter Draft\n\n"
        "I can support teaching (`profile/generated/other.evidence.md#Teaching`).\n\n"
        "Yours sincerely,\n\n"
        "[Applicant name]\n",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 1
    assert "unknown evidence citation" in result.output
    assert "placeholder [Applicant name]" in result.output


def test_check_package_passes_complete_minimal_package(tmp_path):
    workspace, _job_dir = _complete_workspace(tmp_path)
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 0
    assert "Package check passed" in result.output


def _complete_workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    profile_dir = workspace / "profile"
    generated_dir = profile_dir / "generated"
    job_dir = workspace / "jobs" / "example-role"
    generated_dir.mkdir(parents=True)
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        yaml.safe_dump(
            {
                "profile_dir": "profile",
                "jobs_dir": "jobs",
                "job_leads_dir": "job_leads",
                "prompt_dir": "prompts",
                "template_dir": "templates",
                "schema_dir": "schemas",
                "agent_skills_dir": "agent-skills",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (profile_dir / "typst").mkdir()
    (profile_dir / "typst" / "cv.typ").write_text("#section(\"Teaching\")\n", encoding="utf-8")
    (profile_dir / "profile.yaml").write_text(
        yaml.safe_dump(
            {
                "profile_mode": "typst",
                "sources": {"cv": "typst/cv.typ"},
                "generated": {"cv_evidence": "generated/cv.evidence.md"},
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (generated_dir / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        "## Teaching\n\n"
        "- [cv-001] `job`: Teaching Assistant for Econometrics\n",
        encoding="utf-8",
    )
    (job_dir / "job.yaml").write_text("status: packaged\n", encoding="utf-8")
    (job_dir / "job_advert.md").write_text("# Advert\n", encoding="utf-8")
    (job_dir / "parsed_job.json").write_text(
        json.dumps({"title": "Lecturer", "institution": "Example University"}) + "\n",
        encoding="utf-8",
    )
    citation = "`profile/generated/cv.evidence.md#Teaching/cv-001`"
    markdown_files = {
        "00_preparation_questions.md": "# Preparation Questions\n\n- English variant: us\n- Writing style: direct\n",
        "01_job_summary.md": "# Job Summary\n\n- Title: Lecturer\n",
        "02_fit_report.md": f"# Fit Report\n\nTeaching evidence {citation}.\n",
        "03_cover_letter_draft.md": f"# Cover Letter Draft\n\nI can support teaching ({citation}).\n",
        "04_cv_tailoring_notes.md": f"# CV Tailoring Notes\n\n- Move teaching higher ({citation}).\n",
        "05_criteria_checklist.md": f"# Criteria Coverage Checklist\n\nTeaching: strong ({citation}).\n",
        "06_final_application_package.md": f"# Final Application Package\n\nTeaching fit {citation}.\n",
        "07_material_review_checklist.md": f"# Material Review Checklist\n\n- Citation: {citation}\n",
    }
    for filename, contents in markdown_files.items():
        (job_dir / filename).write_text(contents, encoding="utf-8")
    typst_dir = job_dir / "typst"
    typst_dir.mkdir()
    (typst_dir / "cover_letter_content.json").write_text("{}\n", encoding="utf-8")
    (typst_dir / "application_package_content.json").write_text("{}\n", encoding="utf-8")
    return workspace, job_dir
