import json
import shutil
import subprocess

from canisend.materials import ApplicationMaterials
from canisend.typst_mapping import (
    build_application_package_content,
    build_cover_letter_content,
    markdown_to_typst,
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


def test_render_modernpro_cover_letter_source_embeds_editable_content():
    content = build_cover_letter_content(parsed_job(), materials())

    source = render_modernpro_cover_letter_source(content)

    assert '@preview/modernpro-coverletter:0.0.8' in source
    assert 'json("cover_letter_content.json")' not in source
    assert "coverletter.with" in source
    assert "// CANISEND: section opening" in source
    assert "// CANISEND: section research_fit" in source
    assert "I am writing to apply for the Lecturer in Economics role." in source
    assert "My research fits the department's applied economics focus." in source
    assert "# Cover Letter Draft" not in source


def test_render_modernpro_cover_letter_source_preserves_unknown_sections():
    base = materials()
    custom = ApplicationMaterials(
        fit_report=base.fit_report,
        cover_letter_draft=base.cover_letter_draft.replace(
            "## Teaching Fit",
            "## Motivation\n\nI am motivated by the department's public economics group.\n\n## Teaching Fit",
        ),
        cv_tailoring_notes=base.cv_tailoring_notes,
        criteria_checklist=base.criteria_checklist,
    )
    content = build_cover_letter_content(parsed_job(), custom)

    source = render_modernpro_cover_letter_source(content)

    assert content["additional_sections"]["Motivation"].startswith("I am motivated")
    assert "// CANISEND: section additional_sections" in source
    assert "== Motivation" in source
    assert "I am motivated by the department's public economics group." in source


def test_render_modernpro_application_package_source_embeds_editable_content():
    content = build_application_package_content(
        parsed_job(),
        materials(),
        "# Final Application Package\n\n## Remaining Actions\n\n- Review manually.",
    )

    source = render_modernpro_application_package_source(content)

    assert '@preview/modernpro-coverletter:0.0.8' in source
    assert 'json("application_package_content.json")' not in source
    assert "statement.with" in source
    assert "// CANISEND: section job_information" in source
    assert "// CANISEND: section fit_report" in source
    assert "// CANISEND: section criteria_checklist" in source
    assert "// CANISEND: section remaining_actions" in source
    assert "= Fit Report" in source
    assert "- Move econometrics teaching higher." in source
    assert "- Review manually." in source


def test_render_modernpro_sources_escape_typst_special_references():
    custom_job = parsed_job()
    custom_job["application_url"] = "https://example.edu/jobs/123?contact=user@example.edu"
    base = materials()
    custom = ApplicationMaterials(
        fit_report="Contact user@example.edu and see <https://example.edu/profile>.",
        cover_letter_draft=base.cover_letter_draft.replace(
            "My research fits the department's applied economics focus.",
            "My research fits; contact user@example.edu or <https://example.edu/profile>.",
        ),
        cv_tailoring_notes=base.cv_tailoring_notes,
        criteria_checklist=base.criteria_checklist,
    )

    cover_source = render_modernpro_cover_letter_source(build_cover_letter_content(custom_job, custom))
    package_source = render_modernpro_application_package_source(
        build_application_package_content(custom_job, custom, "# Final Application Package\n")
    )

    assert "user\\@example.edu" in cover_source
    assert "\\<https://example.edu/profile\\>" in cover_source
    assert "contact=user\\@example.edu" in package_source


def test_markdown_to_typst_escaped_references_compile_when_typst_is_available(tmp_path):
    typst = shutil.which("typst")
    if typst is None:
        return
    source = tmp_path / "escaped.typ"
    output = tmp_path / "escaped.pdf"
    source.write_text(
        markdown_to_typst("Contact user@example.edu and <https://example.edu/profile>."),
        encoding="utf-8",
    )

    result = subprocess.run(
        [typst, "compile", str(source), str(output)],
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr


def test_cover_letter_content_is_json_serializable():
    content = build_cover_letter_content(parsed_job(), materials())

    assert json.loads(json.dumps(content))["job"]["title"] == "Lecturer in Economics"
