from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
import re
from typing import Any

from jsonschema import Draft202012Validator
from pydantic import ValidationError

from canisend.parse import criteria_section_marker
from canisend.decision_models import (
    ConfirmedCorrectionsV1,
    CorrectionReconciliationV1,
    CriteriaCatalogV1,
    CriterionCorrectionV1,
    CriterionImportance,
    CriterionV1,
    ExtractionConfirmationReconciliationV1,
    SemanticInputReceiptV1,
    SourceSpanV1,
)
from canisend.resource_files import read_resource_text
from canisend.stage_store import StageStoreError, read_json_object
from canisend.stages.parse_stage import ParseStageValidationError, validate_parse_candidate
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    read_optional_safe_bytes,
)


CONFIRM_CONTRACT_VERSION = "1.0.0"
CRITERIA_EXTRACTION_BASIS_VERSION = "1.0.0"
CONFIRMED_CORRECTIONS_PATH = "confirmed_corrections.yaml"
CRITERIA_OUTPUT_PATH = "criteria.json"


@dataclass(frozen=True)
class _SourceOccurrence:
    start_line: int
    end_line: int
    anchor_sha256: str
    criteria_section: CriterionImportance | None


class ConfirmStageError(ValueError):
    """Raised when Confirm inputs cannot form a safe semantic projection."""


class ConfirmStageValidationError(ConfirmStageError):
    """Raised when a Confirm candidate cannot be accepted."""


def stable_criterion_id(
    *,
    job_id: str,
    importance: CriterionImportance,
    source_text: str,
    duplicate_occurrence: int = 1,
    semantic_qualifier: str | None = None,
) -> str:
    if duplicate_occurrence < 1:
        raise ConfirmStageError("Criterion duplicate occurrence must be positive.")
    canonical = json.dumps(
        {
            "job_id": job_id,
            "importance": importance,
            "source_text": _normalized_source_text(source_text),
            "duplicate_occurrence": (
                duplicate_occurrence if semantic_qualifier is None else 1
            ),
            "semantic_qualifier": (
                _normalized_source_text(semantic_qualifier or "") or None
            ),
        },
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return f"criterion_{sha256(canonical).hexdigest()[:32]}"


def criterion_source_sha256(source_text: str) -> str:
    normalized = _normalized_source_text(source_text)
    return sha256(normalized.encode("utf-8")).hexdigest()


def criterion_text_sha256(criterion_text: str) -> str:
    normalized = _normalized_source_text(criterion_text)
    return sha256(normalized.encode("utf-8")).hexdigest()


def criteria_extraction_basis_sha256(
    parsed_job: dict[str, Any],
    advert_text: str,
) -> str:
    """Bind an explicit empty confirmation to the current pre-overlay extraction."""

    projection = {
        "basis_version": CRITERIA_EXTRACTION_BASIS_VERSION,
        "advert_sha256": sha256(advert_text.encode("utf-8")).hexdigest(),
        "criteria": {
            "essential": _canonical_parsed_criteria(parsed_job.get("essential_criteria", [])),
            "desirable": _canonical_parsed_criteria(parsed_job.get("desirable_criteria", [])),
        },
    }
    return _projection_hash(projection)


def confirm_input_projection(
    job_dir: Path,
    *,
    parsed_job_schema_path: Path | None = None,
    criteria_schema_path: Path | None = None,
) -> dict[str, object]:
    parsed_job, advert_text = _load_validated_parse_inputs(
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
    )
    corrections = load_confirmed_corrections(job_dir)
    criteria_schema = _criteria_schema_text(criteria_schema_path)
    parsed_criteria = {
        "essential": _canonical_parsed_criteria(parsed_job["essential_criteria"]),
        "desirable": _canonical_parsed_criteria(parsed_job["desirable_criteria"]),
    }
    active_corrections = () if corrections is None else tuple(
        {
            "correction_id": item.correction_id,
            "criterion_id": item.criterion_id,
            "target_source_sha256": item.target_source_sha256,
            "target_criterion_sha256": item.target_criterion_sha256,
            "confirmation": item.confirmation,
            "corrected_text": item.corrected_text,
            "source_occurrence": item.source_occurrence,
            "source_anchor_sha256": item.source_anchor_sha256,
        }
        for item in corrections.criteria
        if item.record_state == "active"
    )
    active_extraction_confirmations = () if corrections is None else tuple(
        {
            "correction_id": item.correction_id,
            "target_extraction_sha256": item.target_extraction_sha256,
            "confirmation": item.confirmation,
        }
        for item in corrections.criteria_extraction_confirmations
        if item.record_state == "active"
    )
    return {
        "stage": "confirm",
        "contract_version": CONFIRM_CONTRACT_VERSION,
        "criteria": parsed_criteria,
        "advert_sha256": sha256(advert_text.encode("utf-8")).hexdigest(),
        "criteria_extraction_basis_sha256": criteria_extraction_basis_sha256(
            parsed_job,
            advert_text,
        ),
        "active_corrections": active_corrections,
        "active_extraction_confirmations": active_extraction_confirmations,
        "schema_sha256": sha256(criteria_schema.encode("utf-8")).hexdigest(),
    }


def confirm_input_fingerprint(
    job_dir: Path,
    *,
    parsed_job_schema_path: Path | None = None,
    criteria_schema_path: Path | None = None,
) -> str:
    projection = confirm_input_projection(
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        criteria_schema_path=criteria_schema_path,
    )
    canonical = json.dumps(
        projection,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(canonical).hexdigest()


def build_deterministic_confirm_candidate(
    job_dir: Path,
    *,
    input_fingerprint: str | None = None,
    parsed_job_schema_path: Path | None = None,
    criteria_schema_path: Path | None = None,
) -> CriteriaCatalogV1:
    parsed_job, advert_text = _load_validated_parse_inputs(
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
    )
    corrections = load_confirmed_corrections(job_dir)
    projection = confirm_input_projection(
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        criteria_schema_path=criteria_schema_path,
    )
    fingerprint = input_fingerprint or _projection_hash(projection)
    semantic_inputs = [
        SemanticInputReceiptV1(
            path="parsed_job.json",
            projection_sha256=_projection_hash(projection["criteria"]),
        ),
        SemanticInputReceiptV1(
            path="job_advert.md",
            projection_sha256=str(projection["advert_sha256"]),
        ),
    ]
    if projection["active_corrections"] or projection["active_extraction_confirmations"]:
        semantic_inputs.append(
            SemanticInputReceiptV1(
                path=CONFIRMED_CORRECTIONS_PATH,
                projection_sha256=_projection_hash(
                    {
                        "criteria": projection["active_corrections"],
                        "criteria_extraction_confirmations": projection[
                            "active_extraction_confirmations"
                        ],
                    }
                ),
            )
        )
    return project_criteria(
        parsed_job=parsed_job,
        advert_text=advert_text,
        job_id=job_dir.name,
        corrections=corrections,
        input_fingerprint=fingerprint,
        semantic_inputs=tuple(semantic_inputs),
        extraction_basis_sha256=str(projection["criteria_extraction_basis_sha256"]),
    )


def project_criteria(
    *,
    parsed_job: dict[str, Any],
    advert_text: str,
    job_id: str,
    corrections: ConfirmedCorrectionsV1 | None,
    input_fingerprint: str,
    semantic_inputs: tuple[SemanticInputReceiptV1, ...],
    extraction_basis_sha256: str | None = None,
) -> CriteriaCatalogV1:
    raw_criteria: list[tuple[CriterionImportance, dict[str, Any]]] = []
    for importance, field in (
        ("essential", "essential_criteria"),
        ("desirable", "desirable_criteria"),
    ):
        values = parsed_job.get(field, [])
        if not isinstance(values, list):
            raise ConfirmStageError("Parsed criteria must be lists.")
        for value in values:
            if not isinstance(value, dict):
                raise ConfirmStageError("Each parsed criterion must be an object.")
            raw_criteria.append((importance, value))

    deduplicated: list[tuple[CriterionImportance, dict[str, Any]]] = []
    seen_semantics: set[tuple[str, str, str]] = set()
    for importance, value in raw_criteria:
        text = str(value.get("criterion") or "").strip()
        source_text = str(value.get("source_text") or "").strip()
        if not text or not source_text:
            raise ConfirmStageError("Parsed criteria require text and a source receipt.")
        semantic_key = (
            importance,
            _normalized_source_text(source_text),
            _normalized_source_text(text),
        )
        if semantic_key in seen_semantics:
            continue
        seen_semantics.add(semantic_key)
        deduplicated.append((importance, value))

    active_corrections = {
        item.criterion_id: item
        for item in (corrections.criteria if corrections is not None else ())
        if item.record_state == "active"
    }
    active_extraction_confirmations = tuple(
        item
        for item in (
            corrections.criteria_extraction_confirmations
            if corrections is not None
            else ()
        )
        if item.record_state == "active"
    )
    current_extraction_basis = extraction_basis_sha256 or criteria_extraction_basis_sha256(
        parsed_job,
        advert_text,
    )
    applied_correction_ids: set[str] = set()
    orphan_reasons: dict[str, str] = {}
    criteria: list[CriterionV1] = []

    for importance, value in deduplicated:
        text = str(value.get("criterion") or "").strip()
        source_text = str(value.get("source_text") or "").strip()
        criterion_id = stable_criterion_id(
            job_id=job_id,
            importance=importance,
            source_text=source_text,
            semantic_qualifier=text,
        )
        source_hash = criterion_source_sha256(source_text)
        parsed_text_hash = criterion_text_sha256(text)
        source_occurrences = _source_occurrences(advert_text, source_text)
        correction = active_corrections.get(criterion_id)
        correction_is_valid = correction is not None
        correction_reason: str | None = None
        if correction is not None and correction.target_source_sha256 != source_hash:
            correction_is_valid = False
            correction_reason = "source_hash.changed"
        if correction is not None and correction.target_criterion_sha256 != parsed_text_hash:
            correction_is_valid = False
            correction_reason = "criterion_text.changed"
        selected_occurrence, source_selection_reason = _selected_source_occurrence(
            correction,
            source_occurrences,
            importance=importance,
        )
        if (
            correction is not None
            and correction.source_occurrence is not None
            and selected_occurrence is None
        ):
            correction_is_valid = False
            correction_reason = source_selection_reason or "source_anchor.changed"
        if (
            correction is not None
            and len(source_occurrences) == 1
            and source_occurrences[0].criteria_section not in {None, importance}
        ):
            correction_is_valid = False
            correction_reason = "source_section.changed"

        source_span: SourceSpanV1 | None = None
        source_candidates: tuple[SourceSpanV1, ...] = ()
        source_state = "unknown"
        confidence = "unknown"
        unknown_reason = "source_receipt.not_found"
        if (
            len(source_occurrences) == 1
            and source_occurrences[0].criteria_section in {None, importance}
        ):
            source_span = _span(
                source_occurrences[0],
                source_hash=source_hash,
                occurrence=1,
                occurrence_count=1,
            )
            source_state = "known"
            confidence = "high"
            unknown_reason = None
        elif len(source_occurrences) > 1 and selected_occurrence is not None:
            source_span = _span(
                source_occurrences[selected_occurrence - 1],
                source_hash=source_hash,
                occurrence=selected_occurrence,
                occurrence_count=len(source_occurrences),
            )
            source_state = "known"
            confidence = "medium"
            unknown_reason = None
        elif len(source_occurrences) > 1:
            unknown_reason = "source_receipt.ambiguous"
            source_candidates = tuple(
                _span(
                    occurrence_data,
                    source_hash=source_hash,
                    occurrence=index,
                    occurrence_count=len(source_occurrences),
                )
                for index, occurrence_data in enumerate(source_occurrences, start=1)
            )
        elif len(source_occurrences) == 1:
            unknown_reason = "source_receipt.importance_mismatch"

        confirmation_state = "unconfirmed"
        confirmation_record_id = None
        projected_text = text
        if correction_is_valid and correction is not None:
            confirmation_state = correction.confirmation
            confirmation_record_id = correction.correction_id
            if correction.confirmation == "corrected":
                projected_text = correction.corrected_text or text
            applied_correction_ids.add(correction.correction_id)
        elif correction is not None:
            orphan_reasons[correction.correction_id] = correction_reason or "criterion.review_required"

        criteria.append(
            CriterionV1(
                criterion_id=criterion_id,
                importance=importance,
                text=projected_text,
                parsed_text_sha256=parsed_text_hash,
                source_text=source_text,
                source_state=source_state,
                source_span=source_span,
                source_candidates=source_candidates,
                confidence=confidence,
                confirmation_state=confirmation_state,
                confirmation_record_id=confirmation_record_id,
                unknown_reason=unknown_reason,
            )
        )

    orphaned = tuple(
        CorrectionReconciliationV1(
            correction_id=item.correction_id,
            criterion_id=item.criterion_id,
            reason=orphan_reasons.get(
                item.correction_id,
                _missing_correction_reason(item, criteria),
            ),
        )
        for item in (corrections.criteria if corrections is not None else ())
        if item.record_state == "active"
        and item.correction_id not in applied_correction_ids
    )
    unresolved = tuple(
        item.criterion_id
        for item in criteria
        if item.source_state == "unknown" or item.confirmation_state == "unconfirmed"
    )
    active_empty_confirmation = (
        active_extraction_confirmations[0] if active_extraction_confirmations else None
    )
    orphaned_extraction_confirmations: tuple[
        ExtractionConfirmationReconciliationV1, ...
    ] = ()
    extraction_state = "extracted" if criteria else "unknown"
    extraction_unknown_reason = None if criteria else "criteria.none_extracted"
    empty_confirmation_record_id = None
    if active_empty_confirmation is not None:
        if criteria:
            orphaned_extraction_confirmations = (
                ExtractionConfirmationReconciliationV1(
                    correction_id=active_empty_confirmation.correction_id,
                    reason="criteria.extraction_changed",
                ),
            )
        elif active_empty_confirmation.target_extraction_sha256 == current_extraction_basis:
            extraction_state = "confirmed_empty"
            extraction_unknown_reason = None
            empty_confirmation_record_id = active_empty_confirmation.correction_id
        else:
            extraction_unknown_reason = "criteria.empty_confirmation_stale"
            orphaned_extraction_confirmations = (
                ExtractionConfirmationReconciliationV1(
                    correction_id=active_empty_confirmation.correction_id,
                    reason="criteria.extraction_basis_changed",
                ),
            )
    return CriteriaCatalogV1(
        job_id=job_id,
        input_fingerprint=input_fingerprint,
        semantic_inputs=semantic_inputs,
        extraction_state=extraction_state,
        extraction_unknown_reason=extraction_unknown_reason,
        empty_confirmation_record_id=empty_confirmation_record_id,
        criteria=tuple(criteria),
        unresolved_criterion_ids=unresolved,
        orphaned_corrections=orphaned,
        orphaned_extraction_confirmations=orphaned_extraction_confirmations,
    )


def validate_confirm_candidate(
    candidate: object,
    *,
    job_dir: Path,
    input_fingerprint: str,
    parsed_job_schema_path: Path | None = None,
    criteria_schema_path: Path | None = None,
) -> CriteriaCatalogV1:
    if not isinstance(candidate, dict):
        raise ConfirmStageValidationError("Confirm candidate must be a JSON object.")
    try:
        schema = json.loads(_criteria_schema_text(criteria_schema_path))
    except json.JSONDecodeError as exc:
        raise ConfirmStageValidationError("The configured Criteria schema is invalid.") from exc
    if list(Draft202012Validator(schema).iter_errors(candidate)):
        raise ConfirmStageValidationError("Confirm candidate failed schema validation.")
    try:
        validated = CriteriaCatalogV1.model_validate(candidate)
    except ValidationError as exc:
        raise ConfirmStageValidationError("Confirm candidate failed semantic validation.") from exc

    current_fingerprint = confirm_input_fingerprint(
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        criteria_schema_path=criteria_schema_path,
    )
    if input_fingerprint != current_fingerprint or validated.input_fingerprint != input_fingerprint:
        raise ConfirmStageValidationError("Confirm candidate input fingerprint is stale.")
    expected = build_deterministic_confirm_candidate(
        job_dir,
        input_fingerprint=input_fingerprint,
        parsed_job_schema_path=parsed_job_schema_path,
        criteria_schema_path=criteria_schema_path,
    )
    if validated.model_dump(mode="json") != expected.model_dump(mode="json"):
        raise ConfirmStageValidationError("Confirm candidate does not match the current reviewed projection.")
    return validated


def load_confirmed_corrections(job_dir: Path) -> ConfirmedCorrectionsV1 | None:
    try:
        snapshot = read_optional_safe_bytes(job_dir, CONFIRMED_CORRECTIONS_PATH)
    except UnsafeUserFileError as exc:
        raise ConfirmStageError(
            "The confirmed correction overlay is not a safe regular file."
        ) from exc
    if snapshot is None:
        return None
    try:
        overlay = ConfirmedCorrectionsV1.model_validate(
            load_strict_yaml(snapshot.data)
        )
    except (InvalidUserFileError, ValidationError) as exc:
        raise ConfirmStageError("The confirmed correction overlay is not valid safe YAML.") from exc
    if overlay.job_id != job_dir.name:
        raise ConfirmStageError("The confirmed correction overlay belongs to a different job.")
    return overlay


def _load_validated_parse_inputs(
    job_dir: Path,
    *,
    parsed_job_schema_path: Path | None,
) -> tuple[dict[str, Any], str]:
    try:
        parsed_job = read_json_object(job_dir / "parsed_job.json")
        advert_text = (job_dir / "job_advert.md").read_text(encoding="utf-8")
        validated = validate_parse_candidate(
            parsed_job,
            advert_text=advert_text,
            schema_path=parsed_job_schema_path,
        )
    except (StageStoreError, OSError, UnicodeError, ParseStageValidationError) as exc:
        raise ConfirmStageError("Confirmed criteria require a current valid Parsed Job and advert.") from exc
    return validated, advert_text


def _canonical_parsed_criteria(values: object) -> tuple[dict[str, str], ...]:
    if not isinstance(values, list):
        raise ConfirmStageError("Parsed criteria must be lists.")
    result: list[dict[str, str]] = []
    for value in values:
        if not isinstance(value, dict):
            raise ConfirmStageError("Each parsed criterion must be an object.")
        text = str(value.get("criterion") or "").strip()
        source_text = str(value.get("source_text") or "").strip()
        if not text or not source_text:
            raise ConfirmStageError("Parsed criteria require text and a source receipt.")
        result.append(
            {
                "criterion": text,
                "source_text": source_text,
            }
        )
    return tuple(result)


def _criteria_schema_text(schema_path: Path | None) -> str:
    try:
        return read_resource_text(
            "schemas/criteria.schema.json",
            local_path=schema_path,
        )
    except (OSError, UnicodeError) as exc:
        raise ConfirmStageError("The Criteria schema is not readable.") from exc


def _selected_source_occurrence(
    correction: CriterionCorrectionV1 | None,
    occurrences: tuple[_SourceOccurrence, ...],
    *,
    importance: CriterionImportance,
) -> tuple[int | None, str | None]:
    if correction is None or correction.source_occurrence is None:
        return None, None
    if correction.source_occurrence > len(occurrences):
        return None, "source_occurrence.out_of_range"
    selected = occurrences[correction.source_occurrence - 1]
    if selected.anchor_sha256 != correction.source_anchor_sha256:
        return None, "source_anchor.changed"
    if selected.criteria_section not in {None, importance}:
        return None, "source_section.changed"
    matching_anchors = sum(
        item.anchor_sha256 == correction.source_anchor_sha256
        for item in occurrences
    )
    if matching_anchors != 1:
        return None, "source_anchor.ambiguous"
    return correction.source_occurrence, None


def _missing_correction_reason(
    correction: CriterionCorrectionV1,
    criteria: list[CriterionV1],
) -> str:
    if any(
        criterion_source_sha256(item.source_text) == correction.target_source_sha256
        or item.parsed_text_sha256 == correction.target_criterion_sha256
        for item in criteria
    ):
        return "criterion.identity_changed"
    return "criterion.missing"


def _span(
    source_occurrence: _SourceOccurrence,
    *,
    source_hash: str,
    occurrence: int,
    occurrence_count: int,
) -> SourceSpanV1:
    return SourceSpanV1(
        path="job_advert.md",
        start_line=source_occurrence.start_line,
        end_line=source_occurrence.end_line,
        text_sha256=source_hash,
        anchor_sha256=source_occurrence.anchor_sha256,
        occurrence=occurrence,
        occurrence_count=occurrence_count,
    )


def _source_occurrences(
    advert_text: str,
    source_text: str,
) -> tuple[_SourceOccurrence, ...]:
    normalized_advert, line_map = _normalized_text_with_lines(advert_text)
    criteria_sections = _criteria_sections_by_line(advert_text)
    normalized_source = _normalized_source_text(source_text)
    if not normalized_source:
        return ()
    occurrences: list[_SourceOccurrence] = []
    start = 0
    while True:
        index = normalized_advert.find(normalized_source, start)
        if index < 0:
            break
        end_index = index + len(normalized_source) - 1
        start_line = line_map[index]
        end_line = line_map[end_index]
        occurrences.append(
            _SourceOccurrence(
                start_line=start_line,
                end_line=end_line,
                anchor_sha256=_source_anchor_sha256(
                    advert_text,
                    start_line=start_line,
                    end_line=end_line,
                ),
                criteria_section=criteria_sections[start_line - 1],
            )
        )
        start = index + 1
    return tuple(occurrences)


def _criteria_sections_by_line(
    advert_text: str,
) -> tuple[CriterionImportance | None, ...]:
    active: CriterionImportance | None = None
    sections: list[CriterionImportance | None] = []
    lines = advert_text.splitlines()
    for index, line in enumerate(lines):
        marker = _source_importance_marker(line)
        if marker in {"essential", "desirable"}:
            active = marker  # type: ignore[assignment]
        elif (
            _atx_heading(line) is not None
            or (
                index + 1 < len(lines)
                and _setext_heading_level(lines[index + 1]) is not None
            )
            or line.strip().endswith(":")
        ):
            active = None
        sections.append(active)
    return tuple(sections)


def _source_anchor_sha256(
    advert_text: str,
    *,
    start_line: int,
    end_line: int,
) -> str:
    lines = advert_text.splitlines()
    context_start = max(0, start_line - 2)
    context_end = min(len(lines), end_line + 1)
    context = _normalized_source_text("\n".join(lines[context_start:context_end]))
    block_context, preceding_block_label = _source_block_context(
        lines,
        start_line=start_line,
        end_line=end_line,
    )
    headings: dict[int, str] = {}
    section_label: str | None = None
    criteria_section: str | None = None
    for index, line in enumerate(lines[: start_line - 1]):
        marker = _source_importance_marker(line)
        if marker is not None:
            criteria_section = marker
        elif (
            _atx_heading(line) is not None
            or (
                index + 1 < start_line - 1
                and _setext_heading_level(lines[index + 1]) is not None
            )
            or line.strip().endswith(":")
        ):
            criteria_section = None
        heading = _atx_heading(line)
        if heading is not None:
            level = len(heading.group(1))
            headings = {
                key: value for key, value in headings.items() if key < level
            }
            headings[level] = _normalized_source_text(heading.group(2))
            section_label = None
            continue
        setext_level = _setext_heading_level(line)
        if setext_level is not None and index > 0:
            title = _normalized_source_text(lines[index - 1])
            if title:
                headings = {
                    key: value
                    for key, value in headings.items()
                    if key < setext_level
                }
                headings[setext_level] = title
                section_label = None
            continue
        stripped = line.strip()
        if (
            stripped.endswith(":")
            and len(stripped) <= 120
            and not stripped.startswith(("-", "*", "+"))
        ):
            section_label = _normalized_source_text(stripped)
    projection = json.dumps(
        {
            "heading_path": [headings[key] for key in sorted(headings)],
            "section_label": section_label,
            "criteria_section": criteria_section,
            "local_context": context,
            "block_context": block_context,
            "preceding_block_label": preceding_block_label,
        },
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )
    return sha256(projection.encode("utf-8")).hexdigest()


def _source_importance_marker(
    line: str,
) -> CriterionImportance | None:
    parser_marker = criteria_section_marker(line)
    if parser_marker in {"essential", "desirable"}:
        return parser_marker  # type: ignore[return-value]
    stripped = re.sub(r"^\s{0,3}#{1,6}\s+", "", line).strip().rstrip(":")
    if (
        not stripped
        or len(stripped) > 120
        or stripped.startswith(("-", "*", "+"))
        or stripped.endswith((".", "!", "?"))
        or len(stripped.split()) > 10
    ):
        return None
    normalized = _normalized_source_text(stripped)
    first_word = normalized.split(" ", 1)[0]
    if first_word in {"required", "mandatory", "minimum", "essential"}:
        return "essential"
    if first_word in {"preferred", "desired", "desirable"}:
        return "desirable"
    return None


def _source_block_context(
    lines: list[str],
    *,
    start_line: int,
    end_line: int,
) -> tuple[str, str | None]:
    block_start = start_line - 1
    while block_start > 0 and lines[block_start - 1].strip():
        block_start -= 1
    block_end = end_line
    while block_end < len(lines) and lines[block_end].strip():
        block_end += 1
    block_context = _normalized_source_text("\n".join(lines[block_start:block_end]))

    previous_end = block_start - 1
    while previous_end >= 0 and not lines[previous_end].strip():
        previous_end -= 1
    if previous_end < 0:
        return block_context, None
    previous_start = previous_end
    while previous_start > 0 and lines[previous_start - 1].strip():
        previous_start -= 1
    previous_block = lines[previous_start : previous_end + 1]
    label: str | None = None
    if len(previous_block) == 1:
        candidate = previous_block[0].strip()
        if (
            candidate
            and len(candidate) <= 120
            and not candidate.startswith(("-", "*", "+"))
            and not candidate.endswith((".", "!", "?"))
        ):
            label = _normalized_source_text(candidate)
    elif (
        len(previous_block) == 2
        and _setext_heading_level(previous_block[1]) is not None
    ):
        label = _normalized_source_text(previous_block[0])
    return block_context, label


def _atx_heading(line: str) -> re.Match[str] | None:
    return re.match(r"^\s{0,3}(#{1,6})\s+(.+?)\s*#*\s*$", line)


def _setext_heading_level(line: str) -> int | None:
    stripped = line.strip()
    if re.fullmatch(r"=+", stripped):
        return 1
    if re.fullmatch(r"-+", stripped):
        return 2
    return None


def _normalized_text_with_lines(value: str) -> tuple[str, tuple[int, ...]]:
    characters: list[str] = []
    line_numbers: list[int] = []
    line_number = 1
    for character in value:
        character_line = line_number
        if character == "\n":
            line_number += 1
        folded = character.casefold()
        for folded_character in folded:
            if folded_character.isspace():
                if characters and characters[-1] != " ":
                    characters.append(" ")
                    line_numbers.append(character_line)
            else:
                characters.append(folded_character)
                line_numbers.append(character_line)
    if characters and characters[-1] == " ":
        characters.pop()
        line_numbers.pop()
    return "".join(characters), tuple(line_numbers)


def _normalized_source_text(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip().casefold()


def _projection_hash(value: object) -> str:
    canonical = json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(canonical).hexdigest()
