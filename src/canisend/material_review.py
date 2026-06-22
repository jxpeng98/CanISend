from __future__ import annotations

from dataclasses import dataclass
import re
from typing import Any

from canisend.materials import ApplicationMaterials


MATERIAL_FILES = {
    "Cover Letter Draft": "03_cover_letter_draft.md",
    "CV Tailoring Notes": "04_cv_tailoring_notes.md",
}


@dataclass(frozen=True)
class CriteriaCoverage:
    label: str
    evidence_source: str


def build_material_review_checklist(parsed_job: dict[str, Any], materials: ApplicationMaterials) -> str:
    all_citations = sorted(
        _markdown_citations(
            "\n\n".join(
                [
                    materials.fit_report,
                    materials.cover_letter_draft,
                    materials.cv_tailoring_notes,
                    materials.criteria_checklist,
                ]
            )
        )
    )
    lines = [
        "# Material Review Checklist",
        "",
        "Use this management checklist before treating the cover letter draft, CV tailoring notes, or Typst content JSON as ready for user review.",
        "",
        "## Job Context",
        "",
        f"- Title: {parsed_job['title']}",
        f"- Institution: {parsed_job['institution']}",
        f"- Deadline: {parsed_job['deadline']}",
        f"- Required documents: {', '.join(parsed_job['required_documents']) or 'unknown'}",
        "",
        "## Evidence Citations Found",
        "",
    ]

    if all_citations:
        lines.extend(f"- `{citation}`" for citation in all_citations)
    else:
        lines.append("- No profile evidence citations found in generated materials.")

    lines.extend(
        [
            "",
            _review_section("Cover Letter Draft", materials.cover_letter_draft),
            "",
            _review_section("CV Tailoring Notes", materials.cv_tailoring_notes),
            "",
            _strict_hr_review_section(parsed_job, materials),
            "",
            "## Management Actions",
            "",
            "- Resolve placeholders before copying text into a final cover letter.",
            "- Confirm every strong-fit claim has item-level evidence when evidence exists.",
            "- Do not edit `profile/typst/cv.typ` unless the user explicitly asks.",
            "- Apply CV changes manually in the private profile source, then rerun `extract-profile-evidence` and `run` if the evidence changed.",
            "- Keep `typst/cover_letter_content.json` aligned with the reviewed cover letter draft before rendering.",
        ]
    )

    return "\n".join(lines).rstrip() + "\n"


def _review_section(label: str, markdown: str) -> str:
    citations = sorted(_markdown_citations(markdown))
    placeholders = _placeholders(markdown)
    lines = [
        f"## {label}",
        "",
        f"- File: `{MATERIAL_FILES[label]}`",
        "- Status: needs human review",
        f"- Evidence citations: {', '.join(f'`{citation}`' for citation in citations) if citations else 'none found'}",
        f"- Placeholder count: {len(placeholders)}",
    ]
    if placeholders:
        lines.append("- Manual judgement required: resolve bracketed placeholders before use.")
    else:
        lines.append("- Manual judgement required: confirm wording, emphasis, and proportionality before use.")
    return "\n".join(lines)


def _strict_hr_review_section(parsed_job: dict[str, Any], materials: ApplicationMaterials) -> str:
    coverage = _criteria_coverage(materials.criteria_checklist)
    lines = [
        "## Strict University HR Review",
        "",
        "- Review lens: strict university HR / shortlisting panel.",
        "- Standard: every advertised essential criterion must be visible, proportionate, and evidence-backed before submission.",
        "",
        "| Essential Criterion | HR Status | Reason |",
        "|---|---|---|",
    ]
    essentials = parsed_job.get("essential_criteria", [])
    if not essentials:
        lines.append(
            "| No essential criteria extracted | BLOCKER | Review the JD manually before relying on generated materials. |"
        )
        return "\n".join(lines)

    for item in essentials:
        criterion = str(item.get("criterion", "")).strip()
        escaped_criterion = _markdown_table_cell(criterion)
        item_coverage = coverage.get(_criterion_key(criterion))
        if item_coverage is None:
            lines.append(f"| {escaped_criterion} | BLOCKER | Missing from criteria checklist. |")
            continue

        label = item_coverage.label
        if label == "strong" and not _has_linked_evidence_source(item_coverage.evidence_source):
            lines.append(
                f"| {escaped_criterion} | BLOCKER | Coverage is strong but evidence source is not linked. |"
            )
        elif label in {"weak", "missing"}:
            lines.append(
                f"| {escaped_criterion} | BLOCKER | Coverage is {label}; strengthen evidence and JD wording. |"
            )
        elif label == "partial":
            lines.append(
                f"| {escaped_criterion} | REVIEW | Partial coverage; make fit more explicit for HR screening. |"
            )
        else:
            lines.append(
                f"| {escaped_criterion} | OK | Strong coverage recorded; confirm claim wording stays proportional. |"
            )
    return "\n".join(lines)


def _criteria_coverage(markdown: str) -> dict[str, CriteriaCoverage]:
    coverage: dict[str, CriteriaCoverage] = {}
    for line in markdown.splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        cells = _markdown_table_cells(stripped)
        if len(cells) < 2:
            continue
        if _is_table_header(cells) or _is_table_separator(cells):
            continue
        label = cells[1].strip().lower()
        if label in {"strong", "partial", "weak", "missing"}:
            evidence_source = cells[2].strip() if len(cells) > 2 else ""
            coverage[_criterion_key(cells[0])] = CriteriaCoverage(
                label=label,
                evidence_source=evidence_source,
            )
    return coverage


def _criterion_key(value: str) -> str:
    normalized = value.replace("\\|", "|")
    return re.sub(r"\s+", " ", normalized).strip().lower()


def _markdown_table_cell(value: str) -> str:
    normalized = re.sub(r"\s+", " ", value).strip()
    return normalized.replace("|", "\\|")


def _markdown_table_cells(row: str) -> list[str]:
    cells: list[str] = []
    current: list[str] = []
    escaped = False
    for char in row.strip().strip("|"):
        if escaped:
            current.append("\\" + char if char == "|" else char)
            escaped = False
            continue
        if char == "\\":
            escaped = True
            continue
        if char == "|":
            cells.append("".join(current).strip())
            current = []
            continue
        current.append(char)
    if escaped:
        current.append("\\")
    cells.append("".join(current).strip())
    return cells


def _is_table_header(cells: list[str]) -> bool:
    return len(cells) >= 2 and cells[0].strip().lower() == "criterion" and cells[1].strip().lower() == "coverage"


def _is_table_separator(cells: list[str]) -> bool:
    return all(re.fullmatch(r":?-{3,}:?", cell.strip()) for cell in cells if cell.strip())


def _has_linked_evidence_source(value: str) -> bool:
    lowered = value.strip().lower()
    if not lowered or lowered in {"not yet linked", "none", "n/a", "missing"}:
        return False
    return "profile/generated/" in lowered and ".md#" in lowered


def _markdown_citations(markdown: str) -> set[str]:
    return set(re.findall(r"`([^`]+\.md#[^`]+)`", markdown))


def _placeholders(markdown: str) -> list[str]:
    return re.findall(r"\[[^\]\n]+\]", markdown)
