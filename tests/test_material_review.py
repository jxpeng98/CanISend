from canisend.material_review import build_material_review_checklist
from canisend.materials import ApplicationMaterials


def test_material_review_includes_strict_hr_review_and_blocks_missing_essentials():
    parsed_job = {
        "title": "Lecturer in Economics",
        "institution": "University X",
        "deadline": "2026-06-15",
        "required_documents": ["CV", "Cover letter"],
        "essential_criteria": [
            {"criterion": "PhD in Economics", "source_text": "PhD in Economics"},
            {
                "criterion": "Evidence of teaching excellence",
                "source_text": "Evidence of teaching excellence",
            },
        ],
        "desirable_criteria": [],
    }
    materials = ApplicationMaterials(
        fit_report="# Fit\n\nClaim (`profile/generated/cv.evidence.md#Teaching/cv-001`).",
        cover_letter_draft="# Cover\n\nI can teach econometrics.",
        cv_tailoring_notes="# Notes\n\nMove teaching higher.",
        criteria_checklist=(
            "# Criteria Coverage Checklist\n\n"
            "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\n"
            "|---|---|---|---|---|\n"
            "| PhD in Economics | strong | `profile/generated/cv.evidence.md#Education/cv-001` | low | Keep visible. |\n"
        ),
    )

    checklist = build_material_review_checklist(parsed_job, materials)

    assert "## Strict University HR Review" in checklist
    assert "Review lens: strict university HR / shortlisting panel" in checklist
    assert "Evidence of teaching excellence" in checklist
    assert "BLOCKER" in checklist
    assert "Missing from criteria checklist" in checklist


def test_material_review_flags_weak_or_missing_essential_coverage():
    parsed_job = {
        "title": "Lecturer",
        "institution": "University X",
        "deadline": "2026-06-15",
        "required_documents": [],
        "essential_criteria": [
            {"criterion": "Strong research record", "source_text": "Strong research record"},
            {"criterion": "Teaching excellence", "source_text": "Teaching excellence"},
        ],
        "desirable_criteria": [],
    }
    materials = ApplicationMaterials(
        fit_report="# Fit\n",
        cover_letter_draft="# Cover\n",
        cv_tailoring_notes="# Notes\n",
        criteria_checklist=(
            "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\n"
            "|---|---|---|---|---|\n"
            "| Strong research record | weak | Not yet linked | High | Add evidence. |\n"
            "| Teaching excellence | missing | Not yet linked | High | Add evidence. |\n"
        ),
    )

    checklist = build_material_review_checklist(parsed_job, materials)

    assert "Strong research record" in checklist
    assert "Teaching excellence" in checklist
    assert checklist.count("BLOCKER") == 2


def test_material_review_blocks_strong_coverage_without_evidence_source():
    parsed_job = {
        "title": "Lecturer",
        "institution": "University X",
        "deadline": "2026-06-15",
        "required_documents": [],
        "essential_criteria": [
            {"criterion": "Teaching excellence", "source_text": "Teaching excellence"},
        ],
        "desirable_criteria": [],
    }
    materials = ApplicationMaterials(
        fit_report="# Fit\n",
        cover_letter_draft="# Cover\n",
        cv_tailoring_notes="# Notes\n",
        criteria_checklist=(
            "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\n"
            "|---|---|---|---|---|\n"
            "| Teaching excellence | strong | Not yet linked | Low | Add citation. |\n"
        ),
    )

    checklist = build_material_review_checklist(parsed_job, materials)

    assert "| Teaching excellence | BLOCKER | Coverage is strong but evidence source is not linked. |" in checklist


def test_material_review_escapes_criteria_for_markdown_tables():
    parsed_job = {
        "title": "Lecturer",
        "institution": "University X",
        "deadline": "2026-06-15",
        "required_documents": [],
        "essential_criteria": [
            {
                "criterion": "Criterion 1: teaching | research\nleadership",
                "source_text": "Criterion 1: teaching | research\nleadership",
            },
        ],
        "desirable_criteria": [],
    }
    materials = ApplicationMaterials(
        fit_report="# Fit\n",
        cover_letter_draft="# Cover\n",
        cv_tailoring_notes="# Notes\n",
        criteria_checklist=(
            "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\n"
            "|---|---|---|---|---|\n"
            "| Criterion 1: teaching \\| research leadership | partial | `profile/generated/cv.evidence.md#Teaching/cv-001` | Medium | Clarify. |\n"
        ),
    )

    checklist = build_material_review_checklist(parsed_job, materials)

    assert "Criterion 1: teaching \\| research leadership" in checklist
    assert "| Criterion 1: teaching \\| research leadership | REVIEW |" in checklist
