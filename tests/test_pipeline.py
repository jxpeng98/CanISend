import json

import yaml
from typer.testing import CliRunner

from academic_prep.cli import app


def test_run_pipeline_generates_parsed_job_and_application_outputs(tmp_path):
    job_dir = tmp_path / "jobs" / "2026-06-15_university-x_lecturer-in-economics"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "2026-06-15_university-x_lecturer-in-economics",
                "title": "Lecturer in Economics",
                "institution": "University X",
                "department": "Department of Economics",
                "location": "United Kingdom",
                "deadline": "2026-06-15",
                "source_url": "https://example.edu/jobs/123",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        )
    )
    (job_dir / "job_advert.md").write_text(
        """# Lecturer in Economics

Department: Department of Economics
Location: United Kingdom
Salary: Grade 7
Contract: Permanent
Role type: Lecturer
Research fields: Economics, Finance, Econometrics
Teaching fields: Statistics, Econometrics
Required documents: CV, Cover letter, Research statement, Teaching statement

Essential criteria:
- PhD or near completion in Economics or related field
- Evidence of teaching excellence

Desirable criteria:
- Experience supervising dissertations
"""
    )
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir)])

    assert result.exit_code == 0
    parsed_job = json.loads((job_dir / "parsed_job.json").read_text())
    assert parsed_job["title"] == "Lecturer in Economics"
    assert parsed_job["institution"] == "University X"
    assert parsed_job["department"] == "Department of Economics"
    assert parsed_job["research_fields"] == ["Economics", "Finance", "Econometrics"]
    assert parsed_job["teaching_fields"] == ["Statistics", "Econometrics"]
    assert parsed_job["essential_criteria"][0]["criterion"] == "PhD or near completion in Economics or related field"
    assert parsed_job["desirable_criteria"][0]["criterion"] == "Experience supervising dissertations"
    assert parsed_job["required_documents"] == [
        "CV",
        "Cover letter",
        "Research statement",
        "Teaching statement",
    ]
    expected_outputs = [
        "01_job_summary.md",
        "02_fit_report.md",
        "03_cover_letter_draft.md",
        "04_cv_tailoring_notes.md",
        "05_criteria_checklist.md",
        "06_final_application_package.md",
        "typst/cover_letter.typ",
        "typst/application_package.typ",
    ]
    for output in expected_outputs:
        assert (job_dir / output).exists()
    assert "Remaining Actions Before Submission" in (job_dir / "06_final_application_package.md").read_text()
    assert '@preview/modernpro-coverletter:0.0.8' in (job_dir / "typst" / "cover_letter.typ").read_text()
    assert '@preview/modernpro-coverletter:0.0.8' in (job_dir / "typst" / "application_package.typ").read_text()


def test_run_pipeline_reads_generated_profile_evidence(tmp_path):
    job_dir = tmp_path / "jobs" / "2026-06-15_university-x_lecturer-in-economics"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "2026-06-15_university-x_lecturer-in-economics",
                "title": "Lecturer in Economics",
                "institution": "University X",
                "department": "",
                "location": "",
                "deadline": "2026-06-15",
                "source_url": "",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        )
    )
    (job_dir / "job_advert.md").write_text(
        "# Lecturer in Economics\n\n"
        "Essential criteria:\n"
        "- Evidence of teaching excellence\n"
    )
    profile_dir = tmp_path / "profile"
    generated_dir = profile_dir / "generated"
    generated_dir.mkdir(parents=True)
    (generated_dir / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        "## Teaching\n\n"
        "- `job`: position: Teaching Assistant, institution: University X\n"
    )
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--profile-dir", str(profile_dir)])

    assert result.exit_code == 0
    fit_report = (job_dir / "02_fit_report.md").read_text()
    criteria_checklist = (job_dir / "05_criteria_checklist.md").read_text()
    assert "profile/generated/cv.evidence.md#Teaching" in fit_report
    assert "Teaching Assistant" in criteria_checklist
