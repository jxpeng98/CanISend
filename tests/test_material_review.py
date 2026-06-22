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
