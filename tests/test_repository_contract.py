from pathlib import Path


def test_v1_contract_files_exist():
    root = Path(__file__).resolve().parents[1]
    expected_files = [
        "templates/typst/cover_letter.typ",
        "templates/typst/cv_notes.typ",
        "templates/typst/application_package.typ",
        "skills/job_parser.md",
        "skills/profile_matcher.md",
        "skills/cover_letter_writer.md",
        "skills/cv_tailor.md",
        "skills/criteria_checker.md",
        "skills/package_builder.md",
        "schemas/parsed_job.schema.json",
        "schemas/fit_report.schema.json",
        "schemas/criteria_check.schema.json",
    ]

    for path in expected_files:
        assert (root / path).exists()


def test_typst_templates_use_modernpro_packages():
    root = Path(__file__).resolve().parents[1]

    cover_template = (root / "templates/typst/cover_letter.typ").read_text()
    cv_template = (root / "templates/typst/cv_notes.typ").read_text()

    assert '@preview/modernpro-coverletter:0.0.8' in cover_template
    assert '@preview/modernpro-cv:1.3.0' in cv_template


def test_docs_record_rss_and_privacy_contracts():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()
    proposal = (root / "academic_application_prep_copilot_proposal.md").read_text()

    assert "jobs.ac.uk RSS" in readme
    assert "profile/ is ignored by git" in readme
    assert "jobs.ac.uk RSS" in proposal
    assert "RSS import." not in proposal


def test_readme_documents_complete_workflow_and_round_two_tasks():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()

    expected_sections = [
        "## Complete Workflow",
        "### 1. Install and verify the CLI",
        "### 2. Prepare local private profile data",
        "### 3. Fetch jobs.ac.uk RSS leads",
        "### 4. Select one advert and create a job workspace",
        "### 5. Run the application preparation pipeline",
        "### 6. Review and edit generated materials",
        "### 7. Render Typst outputs when needed",
        "### 8. Submit manually outside the tool",
        "## Round 2 Task Queue",
    ]

    for section in expected_sections:
        assert section in readme
    assert "LLM-backed parser" in readme
    assert "evidence citation" in readme
