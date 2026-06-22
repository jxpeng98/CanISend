import json
import sys

import yaml
from typer.testing import CliRunner

from canisend.cli import app


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
        "07_material_review_checklist.md",
        "typst/cover_letter_content.json",
        "typst/cover_letter.typ",
        "typst/application_package_content.json",
        "typst/application_package.typ",
    ]
    for output in expected_outputs:
        assert (job_dir / output).exists()
    assert "Remaining Actions Before Submission" in (job_dir / "06_final_application_package.md").read_text()
    cover_source = (job_dir / "typst" / "cover_letter.typ").read_text()
    package_source = (job_dir / "typst" / "application_package.typ").read_text()
    cover_content = json.loads((job_dir / "typst" / "cover_letter_content.json").read_text())
    assert '@preview/modernpro-coverletter:0.0.8' in cover_source
    assert '@preview/modernpro-coverletter:0.0.8' in package_source
    assert 'json("cover_letter_content.json")' not in cover_source
    assert 'json("application_package_content.json")' not in package_source
    assert "// CANISEND: section research_fit" in cover_source
    assert "// CANISEND: section criteria_checklist" in package_source
    assert "# Cover Letter Draft" not in cover_source
    assert "## Research Fit" not in cover_source
    assert cover_content["recipient"]["institution"] == "University X"
    assert cover_content["job"]["title"] == "Lecturer in Economics"
    review_checklist = (job_dir / "07_material_review_checklist.md").read_text()
    assert "03_cover_letter_draft.md" in review_checklist
    assert "04_cv_tailoring_notes.md" in review_checklist
    assert "Manual judgement required" in review_checklist


def test_run_pipeline_writes_material_review_checklist_with_item_level_evidence(tmp_path):
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
        "# Lecturer in Economics\n\n"
        "Teaching fields: Econometrics\n\n"
        "Essential criteria:\n"
        "- Evidence of teaching excellence\n"
    )
    profile_dir = tmp_path / "profile"
    generated_dir = profile_dir / "generated"
    generated_dir.mkdir(parents=True)
    (generated_dir / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        "## Teaching\n\n"
        "- [cv-001] `job`: Teaching Assistant for Econometrics\n"
    )
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--profile-dir", str(profile_dir)])

    assert result.exit_code == 0
    review_checklist = (job_dir / "07_material_review_checklist.md").read_text()
    assert "`profile/generated/cv.evidence.md#Teaching/cv-001`" in review_checklist
    assert "Cover Letter Draft" in review_checklist
    assert "CV Tailoring Notes" in review_checklist
    assert "Do not edit `profile/typst/cv.typ` unless the user explicitly asks." in review_checklist


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
    assert "Evidence of teaching excellence" in criteria_checklist
    assert "coverage" in criteria_checklist.lower()


def test_run_pipeline_can_use_llm_parser_with_command_provider(tmp_path, monkeypatch):
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
                "source_url": "https://example.edu/jobs/123",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        )
    )
    (job_dir / "job_advert.md").write_text("Raw advert text")
    prompt_dir = tmp_path / "prompts"
    prompt_dir.mkdir()
    (prompt_dir / "job_parser.md").write_text("Parse this:\n{job_metadata}\n{job_advert}")
    captured_prompt = tmp_path / "captured_prompt.txt"
    fake_parser = tmp_path / "fake_parser.py"
    fake_parser.write_text(
        "import json\n"
        "import pathlib\n"
        "import sys\n"
        f"pathlib.Path({str(captured_prompt)!r}).write_text(sys.stdin.read())\n"
        "print(json.dumps({\n"
        "  'title': 'LLM Parsed Lecturer',\n"
        "  'institution': 'University X',\n"
        "  'department': 'Economics',\n"
        "  'location': 'United Kingdom',\n"
        "  'deadline': '2026-06-15',\n"
        "  'salary': 'unknown',\n"
        "  'contract_type': 'unknown',\n"
        "  'role_type': 'Lecturer',\n"
        "  'research_fields': ['Economics'],\n"
        "  'teaching_fields': ['Econometrics'],\n"
        "  'essential_criteria': [{'criterion': 'PhD', 'source_text': 'PhD'}],\n"
        "  'desirable_criteria': [],\n"
        "  'required_documents': ['CV'],\n"
        "  'application_url': 'https://example.edu/jobs/123',\n"
        "  'unknown_fields': [],\n"
        "  'notes': ''\n"
        "}))\n"
    )
    monkeypatch.setenv("ACADEMIC_PREP_LLM_PROVIDER", "command")
    monkeypatch.setenv("ACADEMIC_PREP_LLM_COMMAND", f"{sys.executable} {fake_parser}")
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "run",
            "--job",
            str(job_dir),
            "--llm-parser",
            "--prompt-dir",
            str(prompt_dir),
        ],
    )

    assert result.exit_code == 0
    parsed_job = json.loads((job_dir / "parsed_job.json").read_text())
    assert parsed_job["title"] == "LLM Parsed Lecturer"
    prompt = captured_prompt.read_text()
    assert prompt.count("Raw advert text") == 1
    assert "University X" in prompt


def test_run_pipeline_can_use_llm_drafts_with_command_provider(tmp_path, monkeypatch):
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
        "# Lecturer in Economics\n\n"
        "Teaching fields: Econometrics\n\n"
        "Essential criteria:\n"
        "- Evidence of teaching excellence\n"
    )
    profile_dir = tmp_path / "profile"
    generated_dir = profile_dir / "generated"
    generated_dir.mkdir(parents=True)
    (generated_dir / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        "## Teaching\n\n"
        "- `job`: Teaching Assistant for Econometrics\n"
    )
    fake_generator = tmp_path / "fake_generator.py"
    fake_generator.write_text(
        "import sys\n"
        "prompt = sys.stdin.read()\n"
        "citation = '`profile/generated/cv.evidence.md#Teaching`'\n"
        "if '# Profile Matcher' in prompt:\n"
        "    print(f'# Fit Report\\n\\n- Strong teaching fit for econometrics ({citation}).')\n"
        "elif '# Cover Letter Writer' in prompt:\n"
        "    print(f'# Cover Letter Draft\\n\\nI can contribute to econometrics teaching ({citation}).')\n"
        "elif '# CV Tailor' in prompt:\n"
        "    print(f'# CV Tailoring Notes\\n\\n- Move teaching evidence higher ({citation}).')\n"
        "elif '# Criteria Checker' in prompt:\n"
        "    print(f'# Criteria Coverage Checklist\\n\\n| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\\n|---|---|---|---|---|\\n| Evidence of teaching excellence | strong | {citation} | low | Keep the evidence visible. |')\n"
        "elif '# Package Builder' in prompt:\n"
        "    print(f'# Final Application Package\\n\\n## Job Information\\n\\n- Title: Lecturer in Economics\\n- Institution: University X\\n- Evidence: {citation}')\n"
        "else:\n"
        "    raise SystemExit('unexpected prompt')\n"
    )
    monkeypatch.setenv("ACADEMIC_PREP_LLM_PROVIDER", "command")
    monkeypatch.setenv("ACADEMIC_PREP_LLM_COMMAND", f"{sys.executable} {fake_generator}")
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "run",
            "--job",
            str(job_dir),
            "--profile-dir",
            str(profile_dir),
            "--llm-drafts",
        ],
    )

    assert result.exit_code == 0
    assert "Strong teaching fit for econometrics" in (job_dir / "02_fit_report.md").read_text()
    assert "I can contribute to econometrics teaching" in (job_dir / "03_cover_letter_draft.md").read_text()
    assert "Move teaching evidence higher" in (job_dir / "04_cv_tailoring_notes.md").read_text()
    assert "strong" in (job_dir / "05_criteria_checklist.md").read_text()
    package_content = json.loads((job_dir / "typst" / "application_package_content.json").read_text())
    assert "Strong teaching fit for econometrics" in package_content["fit_report"]
    assert "I can contribute to econometrics teaching" in package_content["cover_letter"]
