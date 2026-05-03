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
