from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import re
from typing import Any

from academic_prep.evidence import EvidenceReference
from academic_prep.llm import LLMProvider
from academic_prep.resource_files import read_resource_text


@dataclass(frozen=True)
class ApplicationMaterials:
    fit_report: str
    cover_letter_draft: str
    cv_tailoring_notes: str
    criteria_checklist: str


class MaterialValidationError(ValueError):
    pass


def generate_materials_with_provider(
    *,
    parsed_job: dict[str, Any],
    evidence: list[EvidenceReference],
    provider: LLMProvider,
    prompt_dir: Path = Path("prompts"),
) -> ApplicationMaterials:
    fit_report = _complete_material(
        provider=provider,
        prompt_path=prompt_dir / "profile_matcher.md",
        parsed_job=parsed_job,
        evidence=evidence,
    )
    cover_letter = _complete_material(
        provider=provider,
        prompt_path=prompt_dir / "cover_letter_writer.md",
        parsed_job=parsed_job,
        evidence=evidence,
        fit_report=fit_report,
    )
    cv_notes = _complete_material(
        provider=provider,
        prompt_path=prompt_dir / "cv_tailor.md",
        parsed_job=parsed_job,
        evidence=evidence,
        fit_report=fit_report,
    )
    checklist = _complete_material(
        provider=provider,
        prompt_path=prompt_dir / "criteria_checker.md",
        parsed_job=parsed_job,
        evidence=evidence,
        fit_report=fit_report,
        cover_letter_draft=cover_letter,
    )
    materials = ApplicationMaterials(
        fit_report=fit_report,
        cover_letter_draft=cover_letter,
        cv_tailoring_notes=cv_notes,
        criteria_checklist=checklist,
    )
    validate_material_citations(materials, evidence)
    return materials


def validate_material_citations(materials: ApplicationMaterials, evidence: list[EvidenceReference]) -> None:
    allowed = _allowed_citations(evidence)
    required = {
        "fit_report": materials.fit_report,
        "cover_letter_draft": materials.cover_letter_draft,
        "cv_tailoring_notes": materials.cv_tailoring_notes,
        "criteria_checklist": materials.criteria_checklist,
    }

    for name, markdown in required.items():
        citations = _markdown_citations(markdown)
        unknown = sorted(citations - allowed)
        if unknown:
            raise MaterialValidationError(f"{name} contains unknown evidence citation: {unknown[0]}")
        if evidence and not citations:
            raise MaterialValidationError(f"{name} must cite at least one profile evidence reference")


def _complete_material(
    *,
    provider: LLMProvider,
    prompt_path: Path,
    parsed_job: dict[str, Any],
    evidence: list[EvidenceReference],
    fit_report: str = "",
    cover_letter_draft: str = "",
) -> str:
    prompt = _render_material_prompt(
        read_resource_text(f"prompts/{prompt_path.name}", local_path=prompt_path),
        parsed_job=parsed_job,
        evidence=evidence,
        fit_report=fit_report,
        cover_letter_draft=cover_letter_draft,
    )
    return provider.complete(prompt).content.strip() + "\n"


def _render_material_prompt(
    prompt_text: str,
    *,
    parsed_job: dict[str, Any],
    evidence: list[EvidenceReference],
    fit_report: str = "",
    cover_letter_draft: str = "",
) -> str:
    rendered = prompt_text.replace("{parsed_job}", json.dumps(parsed_job, indent=2, default=str))
    rendered = rendered.replace("{profile_evidence}", _evidence_json(evidence))
    rendered = rendered.replace("{fit_report}", fit_report)
    rendered = rendered.replace("{cover_letter_draft}", cover_letter_draft)
    return rendered


def _evidence_json(evidence: list[EvidenceReference]) -> str:
    data = [
        {
            "citation": item.citation,
            "section_citation": item.section_citation,
            "source_file": item.source_file,
            "section": item.section,
            "item_id": item.item_id,
            "text": item.text,
        }
        for item in evidence
    ]
    return json.dumps(data, indent=2, default=str)


def _allowed_citations(evidence: list[EvidenceReference]) -> set[str]:
    allowed: set[str] = set()
    for item in evidence:
        allowed.add(item.section_citation)
        allowed.add(item.citation)
    return allowed


def _markdown_citations(markdown: str) -> set[str]:
    return set(re.findall(r"`([^`]+\.md#[^`]+)`", markdown))
