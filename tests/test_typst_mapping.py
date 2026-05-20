import json

from canisend.materials import ApplicationMaterials
from canisend.typst_mapping import (
    build_cover_letter_content,
    render_modernpro_application_package_source,
    render_modernpro_cover_letter_source,
)


def parsed_job() -> dict:
    return {
        "title": "Lecturer in Economics",
        "institution": "University X",
        "department": "Department of Economics",
        "location": "United Kingdom",
        "deadline": "2026-06-15",
        "salary": "unknown",
        "contract_type": "Permanent",
        "role_type": "Lecturer",
        "research_fields": ["Economics"],
        "teaching_fields": ["Econometrics"],
        "essential_criteria": [],
        "desirable_criteria": [],
        "required_documents": ["CV", "Cover letter"],
        "application_url": "https://example.edu/jobs/123",
        "unknown_fields": [],
        "notes": "",
    }


def materials() -> ApplicationMaterials:
    return ApplicationMaterials(
        fit_report="# Fit Report\n\nStrong teaching fit.",
        cover_letter_draft="""# Cover Letter Draft

Dear Selection Committee,

I am writing to apply for the Lecturer in Economics role.

## Research Fit

My research fits the department's applied economics focus.

## Teaching Fit

I can contribute to econometrics teaching.

## Departmental Contribution

I would support the department's quantitative methods provision.

## Service and Leadership

I can contribute to collegial service.

Yours sincerely,

[Applicant name]
""",
        cv_tailoring_notes="# CV Tailoring Notes\n\n- Move econometrics teaching higher.",
        criteria_checklist="# Criteria Coverage Checklist\n\n| Criterion | Coverage |\n|---|---|\n",
    )


def test_build_cover_letter_content_extracts_structured_sections():
    content = build_cover_letter_content(parsed_job(), materials())

    assert content["recipient"]["institution"] == "University X"
    assert content["recipient"]["department"] == "Department of Economics"
    assert content["recipient"]["cl_title"] == "Application for Lecturer in Economics"
    assert content["opening"] == "I am writing to apply for the Lecturer in Economics role."
    assert content["sections"]["research_fit"].startswith("My research fits")
    assert content["sections"]["teaching_fit"].startswith("I can contribute")
    assert "Yours sincerely" not in content["sections"]["service_leadership"]
    assert "[Applicant name]" not in content["sections"]["service_leadership"]
    assert content["closing"].startswith("I would welcome")


def test_render_modernpro_cover_letter_source_uses_content_json_contract():
    content = build_cover_letter_content(parsed_job(), materials())

    source = render_modernpro_cover_letter_source(content)

    assert '@preview/modernpro-coverletter:0.0.8' in source
    assert 'json("cover_letter_content.json")' in source
    assert "coverletter.with" in source
    assert "content.recipient.institution" in source
    assert "content.sections.research_fit" in source
    assert "# Cover Letter Draft" not in source
    assert "## Research Fit" not in source


def test_render_modernpro_application_package_source_uses_structured_content_json():
    source = render_modernpro_application_package_source()

    assert '@preview/modernpro-coverletter:0.0.8' in source
    assert 'json("application_package_content.json")' in source
    assert "statement.with" in source
    assert "package.cover_letter" in source


def test_cover_letter_content_is_json_serializable():
    content = build_cover_letter_content(parsed_job(), materials())

    assert json.loads(json.dumps(content))["job"]["title"] == "Lecturer in Economics"
