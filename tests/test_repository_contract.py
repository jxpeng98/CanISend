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
