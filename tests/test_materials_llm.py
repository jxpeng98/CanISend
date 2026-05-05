import pytest

from academic_prep.evidence import EvidenceReference
from academic_prep.llm import LLMResponse
from academic_prep.materials import (
    ApplicationMaterials,
    MaterialValidationError,
    generate_materials_with_provider,
    validate_material_citations,
)


class RoutingProvider:
    def __init__(self) -> None:
        self.prompts: list[str] = []

    def complete(self, prompt: str) -> LLMResponse:
        self.prompts.append(prompt)
        citation = "`profile/generated/cv.evidence.md#Teaching`"
        if "# Profile Matcher" in prompt:
            return LLMResponse(
                content=f"# Fit Report\n\n- Strong teaching fit: taught econometrics ({citation}).",
                provider="fake",
            )
        if "# Cover Letter Writer" in prompt:
            return LLMResponse(
                content=f"# Cover Letter Draft\n\nI can support econometrics teaching ({citation}).",
                provider="fake",
            )
        if "# CV Tailor" in prompt:
            return LLMResponse(
                content=f"# CV Tailoring Notes\n\n- Move teaching evidence higher ({citation}).",
                provider="fake",
            )
        if "# Criteria Checker" in prompt:
            return LLMResponse(
                content=(
                    "# Criteria Coverage Checklist\n\n"
                    "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\n"
                    "|---|---|---|---|---|\n"
                    f"| Evidence of teaching excellence | strong | {citation} | low | Keep the evidence visible. |\n"
                ),
                provider="fake",
            )
        raise AssertionError(f"Unexpected prompt: {prompt[:80]}")


def parsed_job() -> dict:
    return {
        "title": "Lecturer in Economics",
        "institution": "University X",
        "department": "Economics",
        "location": "United Kingdom",
        "deadline": "2026-06-15",
        "salary": "unknown",
        "contract_type": "Permanent",
        "role_type": "Lecturer",
        "research_fields": ["Economics"],
        "teaching_fields": ["Econometrics"],
        "essential_criteria": [
            {
                "criterion": "Evidence of teaching excellence",
                "source_text": "Evidence of teaching excellence",
            }
        ],
        "desirable_criteria": [],
        "required_documents": ["CV", "Cover letter"],
        "application_url": "https://example.edu/jobs/123",
        "unknown_fields": [],
        "notes": "",
    }


def evidence() -> list[EvidenceReference]:
    return [
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Teaching",
            text="`job`: Teaching Assistant for Econometrics",
        )
    ]


def test_generate_materials_with_provider_injects_context_and_validates_citations():
    provider = RoutingProvider()

    materials = generate_materials_with_provider(
        parsed_job=parsed_job(),
        evidence=evidence(),
        provider=provider,
    )

    assert "Strong teaching fit" in materials.fit_report
    assert "support econometrics teaching" in materials.cover_letter_draft
    assert "Move teaching evidence higher" in materials.cv_tailoring_notes
    assert "Evidence of teaching excellence" in materials.criteria_checklist
    assert len(provider.prompts) == 4
    assert all("profile/generated/cv.evidence.md#Teaching" in prompt for prompt in provider.prompts)
    assert all("Evidence of teaching excellence" in prompt for prompt in provider.prompts)


def test_validate_material_citations_rejects_unknown_profile_reference():
    materials = ApplicationMaterials(
        fit_report="# Fit Report\n\nClaim (`profile/generated/other.evidence.md#Teaching`).",
        cover_letter_draft="# Cover Letter Draft\n\nClaim (`profile/generated/cv.evidence.md#Teaching`).",
        cv_tailoring_notes="# CV Tailoring Notes\n\nClaim (`profile/generated/cv.evidence.md#Teaching`).",
        criteria_checklist="# Criteria Coverage Checklist\n\nClaim (`profile/generated/cv.evidence.md#Teaching`).",
    )

    with pytest.raises(MaterialValidationError, match="unknown evidence citation"):
        validate_material_citations(materials, evidence())


def test_validate_material_citations_requires_citations_when_evidence_exists():
    materials = ApplicationMaterials(
        fit_report="# Fit Report\n\nStrong teaching fit.",
        cover_letter_draft="# Cover Letter Draft\n\nI can support teaching.",
        cv_tailoring_notes="# CV Tailoring Notes\n\nMove teaching higher.",
        criteria_checklist="# Criteria Coverage Checklist\n\nTeaching excellence: strong.",
    )

    with pytest.raises(MaterialValidationError, match="must cite at least one"):
        validate_material_citations(materials, evidence())
