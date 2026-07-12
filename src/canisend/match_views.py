from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
from typing import Any

from pydantic import ValidationError

from canisend.decision_models import (
    CriteriaCatalogV1,
    CriterionMatchV1,
    CriterionMatchesV1,
    CriterionV1,
    EvidenceCatalogItemV1,
    EvidenceCatalogV1,
)
from canisend.material_review import (
    StructuredCriteriaReview,
    StructuredEssentialCriterion,
)
from canisend.stage_runtime import StageRuntimeError, inspect_stage_status
from canisend.stage_store import StageStoreError, read_json_object, sha256_file


PARSED_JOB_PATH = "parsed_job.json"
CRITERIA_PATH = "criteria.json"
EVIDENCE_CATALOG_PATH = "evidence_catalog.json"
CRITERION_MATCHES_PATH = "criterion_matches.json"


class MatchViewError(ValueError):
    """Raised when structured Match artifacts cannot form a safe Markdown view."""


@dataclass(frozen=True)
class StructuredMatchViews:
    fit_report: str
    criteria_checklist: str
    criteria_review: StructuredCriteriaReview


def load_current_structured_match_views(
    workspace: Path,
    job_dir: Path,
    *,
    parsed_job: dict[str, Any],
) -> StructuredMatchViews | None:
    """Return current structured views, or ``None`` for a safe legacy fallback."""

    try:
        inspection = inspect_stage_status(workspace, job_dir, stage="match")
        if (
            inspection.stage.status != "succeeded"
            or inspection.reasons
            or inspection.output_drift
        ):
            return None
        if read_json_object(job_dir / PARSED_JOB_PATH) != parsed_job:
            return None
        criteria = CriteriaCatalogV1.model_validate(
            read_json_object(job_dir / CRITERIA_PATH)
        )
        evidence = EvidenceCatalogV1.model_validate(
            read_json_object(job_dir / EVIDENCE_CATALOG_PATH)
        )
        matches = CriterionMatchesV1.model_validate(
            read_json_object(job_dir / CRITERION_MATCHES_PATH)
        )
        if (
            matches.criteria_catalog_sha256
            != sha256_file(job_dir / CRITERIA_PATH)
            or matches.evidence_catalog_sha256
            != sha256_file(job_dir / EVIDENCE_CATALOG_PATH)
        ):
            return None
        views = render_structured_match_views(criteria, matches, evidence)
        final_inspection = inspect_stage_status(workspace, job_dir, stage="match")
        if (
            final_inspection.stage.status != "succeeded"
            or final_inspection.reasons
            or final_inspection.output_drift
            or read_json_object(job_dir / PARSED_JOB_PATH) != parsed_job
            or matches.criteria_catalog_sha256
            != sha256_file(job_dir / CRITERIA_PATH)
            or matches.evidence_catalog_sha256
            != sha256_file(job_dir / EVIDENCE_CATALOG_PATH)
        ):
            return None
        return views
    except (
        MatchViewError,
        OSError,
        StageRuntimeError,
        StageStoreError,
        UnicodeError,
        ValidationError,
    ):
        return None


def render_structured_match_views(
    criteria: CriteriaCatalogV1,
    matches: CriterionMatchesV1,
    evidence: EvidenceCatalogV1,
) -> StructuredMatchViews:
    """Render deterministic proposal views from one validated structured graph."""

    _validate_structured_graph(criteria, matches, evidence)
    match_by_id = {item.criterion_id: item for item in matches.matches}
    evidence_by_id = {item.evidence_id: item for item in evidence.items}
    unresolved_ids = set(criteria.unresolved_criterion_ids)

    fit_lines = [
        "# Fit Report",
        "",
        "> **Deterministic proposal:** classifications and linked evidence require human review; "
        "this view is not an application decision or readiness result.",
        "",
    ]
    for importance, heading in (
        ("essential", "Essential Criteria Match"),
        ("desirable", "Desirable Criteria Match"),
    ):
        selected = tuple(item for item in criteria.criteria if item.importance == importance)
        if importance == "desirable" and not selected:
            continue
        fit_lines.extend((f"## {heading}", ""))
        if not selected:
            message = (
                "- The criteria extraction is explicitly confirmed empty."
                if criteria.extraction_state == "confirmed_empty"
                else "- No criteria recorded in this importance category."
            )
            fit_lines.extend((message, ""))
            continue
        for criterion in selected:
            match = match_by_id[criterion.criterion_id]
            fit_lines.append(
                f"- {_classification_icon(match.classification)} "
                f"**{match.classification.upper()} (PROPOSED)** — "
                f"{_markdown_inline(criterion.text)}"
            )
            citations = _linked_citations(match, evidence_by_id)
            fit_lines.append(
                "  Linked evidence: "
                + (", ".join(_markdown_code(value) for value in citations) if citations else "none")
            )
            review_note = _criterion_review_note(criterion)
            if review_note:
                fit_lines.append(f"  Review state: unresolved — {review_note}.")
            fit_lines.append(f"  {_suggestion(match, criterion)}")
            fit_lines.append("")

    fit_lines.extend(("## Evidence Index", ""))
    if evidence.items:
        kinds = sorted({item.kind for item in evidence.items})
        fit_lines.append(
            f"{len(evidence.items)} evidence items available across "
            f"{', '.join(kinds) if kinds else 'uncategorized evidence'}."
        )
    elif evidence.state == "empty":
        fit_lines.append("The current evidence catalog is valid and explicitly empty.")
    else:
        fit_lines.append("The current evidence catalog is unavailable for substantive matching.")

    essential_matches = tuple(
        match_by_id[item.criterion_id]
        for item in criteria.criteria
        if item.importance == "essential"
    )
    fit_lines.extend(("", "## Application Risks", ""))
    missing_count = sum(item.classification == "missing" for item in essential_matches)
    unknown_count = sum(item.classification == "unknown" for item in essential_matches)
    if missing_count:
        fit_lines.append(f"- {missing_count} essential criteria have no linked evidence.")
    if unknown_count:
        fit_lines.append(f"- {unknown_count} essential criteria have unresolved evidence state.")
    unresolved_essential_count = sum(
        item.criterion_id in unresolved_ids
        for item in criteria.criteria
        if item.importance == "essential"
    )
    if unresolved_essential_count:
        fit_lines.append(
            f"- {unresolved_essential_count} essential criteria require source or confirmation resolution."
        )
    if criteria.extraction_state == "confirmed_empty":
        fit_lines.append(
            "- Criteria extraction is explicitly confirmed empty; verify this still reflects the advert."
        )
    elif not missing_count and not unknown_count and not unresolved_essential_count:
        fit_lines.append("- No essential criterion is classified as missing or unknown.")
    fit_lines.append("- Review every proposed classification before drafting or submission decisions.")

    checklist_lines = [
        "# Criteria Coverage Checklist",
        "",
        "> Deterministic Match proposals only; confirm every row before relying on this checklist.",
        "",
        "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |",
        "|---|---|---|---|---|",
    ]
    for criterion in criteria.criteria:
        match = match_by_id[criterion.criterion_id]
        citations = _linked_citations(match, evidence_by_id)
        evidence_source = (
            ", ".join(_markdown_table_code(value) for value in citations)
            if citations
            else "Not yet linked"
        )
        checklist_lines.append(
            "| "
            + " | ".join(
                (
                    _markdown_table_cell(criterion.text),
                    match.classification,
                    evidence_source,
                    _risk(
                        match.classification,
                        essential=criterion.importance == "essential",
                        unresolved=criterion.criterion_id in unresolved_ids,
                    ),
                    _markdown_table_cell(_suggestion(match, criterion)),
                )
            )
            + " |"
        )
    if not criteria.criteria:
        if criteria.extraction_state == "confirmed_empty":
            checklist_lines.append(
                "| No criteria advertised | confirmed_empty | Not applicable | Low | "
                "Verify the explicit empty confirmation still reflects the advert. |"
            )

    return StructuredMatchViews(
        fit_report="\n".join(fit_lines).rstrip() + "\n",
        criteria_checklist="\n".join(checklist_lines).rstrip() + "\n",
        criteria_review=StructuredCriteriaReview(
            extraction_state=criteria.extraction_state,
            essential_criteria=tuple(
                StructuredEssentialCriterion(
                    criterion_id=item.criterion_id,
                    text=item.text,
                    classification=match_by_id[item.criterion_id].classification,
                    evidence_linked=bool(
                        match_by_id[item.criterion_id].evidence_ref_ids
                    ),
                    unresolved_reasons=_criterion_unresolved_reasons(item),
                )
                for item in criteria.criteria
                if item.importance == "essential"
            ),
        ),
    )


def _validate_structured_graph(
    criteria: CriteriaCatalogV1,
    matches: CriterionMatchesV1,
    evidence: EvidenceCatalogV1,
) -> None:
    if criteria.job_id != matches.job_id or criteria.job_id != evidence.job_id:
        raise MatchViewError("Structured Match view inputs belong to different jobs.")
    if criteria.extraction_state == "unknown":
        raise MatchViewError("Structured Match views require resolved criteria extraction.")
    criterion_ids = tuple(item.criterion_id for item in criteria.criteria)
    match_ids = tuple(item.criterion_id for item in matches.matches)
    if set(criterion_ids) != set(match_ids) or len(criterion_ids) != len(match_ids):
        raise MatchViewError("Structured Match views require exactly one match per criterion.")

    evidence_by_id = {item.evidence_id: item for item in evidence.items}
    ref_by_id = {item.evidence_id: item for item in matches.evidence_refs}
    referenced_ids = {
        evidence_id for item in matches.matches for evidence_id in item.evidence_ref_ids
    }
    if set(ref_by_id) != referenced_ids:
        raise MatchViewError("Structured Match references must name exactly the linked evidence IDs.")
    for evidence_id, reference in ref_by_id.items():
        item = evidence_by_id.get(evidence_id)
        if item is None:
            raise MatchViewError("Structured Match evidence references must resolve in the catalog.")
        if (
            reference.path != EVIDENCE_CATALOG_PATH
            or reference.section != "items"
            or reference.item_locator != evidence_id
            or reference.kind != "catalog_item"
            or reference.content_sha256 != item.content_sha256
        ):
            raise MatchViewError("Structured Match evidence reference metadata is inconsistent.")


def _linked_citations(
    match: CriterionMatchV1,
    evidence_by_id: dict[str, EvidenceCatalogItemV1],
) -> tuple[str, ...]:
    return tuple(
        evidence_by_id[evidence_id].citation
        for evidence_id in sorted(match.evidence_ref_ids)
    )


def _suggestion(match: CriterionMatchV1, criterion: CriterionV1) -> str:
    review_note = _criterion_review_note(criterion)
    prefix = f"Resolve {review_note}. " if review_note else ""
    if match.gaps:
        return prefix + " ".join(gap.next_action for gap in match.gaps)
    return prefix + "Evidence linked; verify that any claim remains proportional to the source."


def _criterion_unresolved_reasons(criterion: CriterionV1) -> tuple[str, ...]:
    reasons: list[str] = []
    if criterion.confirmation_state == "unconfirmed":
        reasons.append("confirmation is unconfirmed")
    if criterion.source_state == "unknown":
        reasons.append("source receipt is unresolved")
    return tuple(reasons)


def _criterion_review_note(criterion: CriterionV1) -> str:
    return "; ".join(_criterion_unresolved_reasons(criterion))


def _classification_icon(classification: str) -> str:
    return {
        "strong": "✅",
        "partial": "⚠️",
        "weak": "🔸",
        "missing": "❌",
        "unknown": "❓",
    }[classification]


def _risk(classification: str, *, essential: bool, unresolved: bool) -> str:
    if unresolved:
        return "High" if essential else "Medium"
    if classification == "strong":
        return "Low"
    if classification == "partial":
        return "Medium" if essential else "Low"
    if classification == "weak":
        return "High" if essential else "Medium"
    return "High" if essential else "Medium"


def _markdown_inline(value: str) -> str:
    normalized = re.sub(r"\s+", " ", value).strip()
    return re.sub(r"([\\`*_<>{}\[\]])", r"\\\1", normalized)


def _markdown_table_cell(value: str) -> str:
    return _markdown_inline(value).replace("|", "\\|")


def _markdown_code(value: str) -> str:
    normalized = re.sub(r"\s+", " ", value).strip().replace("`", "'")
    return f"`{normalized}`"


def _markdown_table_code(value: str) -> str:
    normalized = re.sub(r"\s+", " ", value).strip()
    normalized = normalized.replace("\\", "\\\\").replace("`", "'")
    escaped = normalized.replace("|", "\\|")
    return f"`{escaped}`"
