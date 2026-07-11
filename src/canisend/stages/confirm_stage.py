from __future__ import annotations

from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
import re
from typing import Any

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import yaml

from canisend.decision_models import (
    ConfirmedCorrectionsV1,
    CriteriaCatalogV1,
    CriterionCorrectionV1,
    CriterionImportance,
    CriterionV1,
    SourceSpanV1,
)
from canisend.resource_files import read_resource_text
from canisend.stage_models import ArtifactFingerprint
from canisend.stage_store import StageStoreError, read_json_object, sha256_file
from canisend.stages.parse_stage import ParseStageValidationError, validate_parse_candidate


CONFIRM_CONTRACT_VERSION = "1.0.0"
CONFIRMED_CORRECTIONS_PATH = "confirmed_corrections.yaml"
CRITERIA_OUTPUT_PATH = "criteria.json"


class ConfirmStageError(ValueError):
    """Raised when Confirm inputs cannot form a safe semantic projection."""


class ConfirmStageValidationError(ConfirmStageError):
    """Raised when a Confirm candidate cannot be accepted."""


def stable_criterion_id(
    *,
    job_id: str,
    importance: CriterionImportance,
    source_text: str,
    duplicate_occurrence: int,
) -> str:
    if duplicate_occurrence < 1:
        raise ConfirmStageError("Criterion duplicate occurrence must be positive.")
    canonical = json.dumps(
        {
            "job_id": job_id,
            "importance": importance,
            "source_text": _normalized_source_text(source_text),
            "duplicate_occurrence": duplicate_occurrence,
        },
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return f"criterion_{sha256(canonical).hexdigest()[:32]}"


def criterion_source_sha256(source_text: str) -> str:
    normalized = _normalized_source_text(source_text)
    return sha256(normalized.encode("utf-8")).hexdigest()


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
    active_corrections = () if corrections is None else tuple(
        {
            "correction_id": item.correction_id,
            "criterion_id": item.criterion_id,
            "target_source_sha256": item.target_source_sha256,
            "confirmation": item.confirmation,
            "corrected_text": item.corrected_text,
            "source_occurrence": item.source_occurrence,
        }
        for item in corrections.criteria
        if item.record_state == "active"
    )
    return {
        "stage": "confirm",
        "contract_version": CONFIRM_CONTRACT_VERSION,
        "criteria": {
            "essential": _canonical_parsed_criteria(parsed_job["essential_criteria"]),
            "desirable": _canonical_parsed_criteria(parsed_job["desirable_criteria"]),
        },
        "advert_sha256": sha256(advert_text.encode("utf-8")).hexdigest(),
        "active_corrections": active_corrections,
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
    fingerprint = input_fingerprint or confirm_input_fingerprint(
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        criteria_schema_path=criteria_schema_path,
    )
    return project_criteria(
        parsed_job=parsed_job,
        advert_text=advert_text,
        job_id=job_dir.name,
        corrections=corrections,
        input_fingerprint=fingerprint,
        inputs=(
            _artifact(job_dir, "parsed_job.json"),
            _artifact(job_dir, "job_advert.md"),
        ),
    )


def project_criteria(
    *,
    parsed_job: dict[str, Any],
    advert_text: str,
    job_id: str,
    corrections: ConfirmedCorrectionsV1 | None,
    input_fingerprint: str,
    inputs: tuple[ArtifactFingerprint, ...],
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

    duplicate_seen: Counter[tuple[str, str]] = Counter()
    active_corrections = {
        item.criterion_id: item
        for item in (corrections.criteria if corrections is not None else ())
        if item.record_state == "active"
    }
    applied_correction_ids: set[str] = set()
    criteria: list[CriterionV1] = []

    for importance, value in raw_criteria:
        text = str(value.get("criterion") or "").strip()
        source_text = str(value.get("source_text") or "").strip()
        if not text or not source_text:
            raise ConfirmStageError("Parsed criteria require text and a source receipt.")
        normalized_key = (importance, _normalized_source_text(source_text))
        duplicate_seen[normalized_key] += 1
        duplicate_occurrence = duplicate_seen[normalized_key]
        criterion_id = stable_criterion_id(
            job_id=job_id,
            importance=importance,
            source_text=source_text,
            duplicate_occurrence=duplicate_occurrence,
        )
        source_hash = criterion_source_sha256(source_text)
        source_occurrences = _source_occurrences(advert_text, source_text)
        correction = active_corrections.get(criterion_id)
        correction_is_valid = correction is not None and correction.target_source_sha256 == source_hash
        selected_occurrence = _selected_source_occurrence(correction, len(source_occurrences))
        if correction is not None and correction.source_occurrence is not None and selected_occurrence is None:
            correction_is_valid = False

        source_span: SourceSpanV1 | None = None
        source_state = "unknown"
        confidence = "unknown"
        unknown_reason = "source_receipt.not_found"
        if len(source_occurrences) == 1:
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

        confirmation_state = "unconfirmed"
        confirmation_record_id = None
        projected_text = text
        if correction_is_valid and correction is not None:
            confirmation_state = correction.confirmation
            confirmation_record_id = correction.correction_id
            if correction.confirmation == "corrected":
                projected_text = correction.corrected_text or text
            applied_correction_ids.add(correction.correction_id)

        criteria.append(
            CriterionV1(
                criterion_id=criterion_id,
                importance=importance,
                text=projected_text,
                source_text=source_text,
                source_state=source_state,
                source_span=source_span,
                confidence=confidence,
                confirmation_state=confirmation_state,
                confirmation_record_id=confirmation_record_id,
                unknown_reason=unknown_reason,
            )
        )

    orphaned = tuple(
        item.correction_id
        for item in (corrections.criteria if corrections is not None else ())
        if item.record_state == "active" and item.correction_id not in applied_correction_ids
    )
    unresolved = tuple(
        item.criterion_id
        for item in criteria
        if item.source_state == "unknown" or item.confirmation_state == "unconfirmed"
    )
    return CriteriaCatalogV1(
        job_id=job_id,
        input_fingerprint=input_fingerprint,
        inputs=inputs,
        criteria=tuple(criteria),
        unresolved_criterion_ids=unresolved,
        orphaned_correction_ids=orphaned,
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
    path = job_dir / CONFIRMED_CORRECTIONS_PATH
    if not path.exists():
        return None
    if path.is_symlink() or not path.is_file():
        raise ConfirmStageError("The confirmed correction overlay is not a safe regular file.")
    try:
        loaded = yaml.safe_load(path.read_text(encoding="utf-8"))
        overlay = ConfirmedCorrectionsV1.model_validate(loaded)
    except (OSError, UnicodeError, yaml.YAMLError, ValidationError) as exc:
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


def _artifact(job_dir: Path, relative_path: str) -> ArtifactFingerprint:
    path = job_dir / relative_path
    try:
        return ArtifactFingerprint(
            path=relative_path,
            sha256=sha256_file(path),
            size_bytes=path.stat().st_size,
        )
    except (OSError, StageStoreError, ValidationError) as exc:
        raise ConfirmStageError("A Confirm input artifact cannot be inspected safely.") from exc


def _selected_source_occurrence(
    correction: CriterionCorrectionV1 | None,
    occurrence_count: int,
) -> int | None:
    if correction is None or correction.source_occurrence is None:
        return None
    if correction.source_occurrence > occurrence_count:
        return None
    return correction.source_occurrence


def _span(
    lines: tuple[int, int],
    *,
    source_hash: str,
    occurrence: int,
    occurrence_count: int,
) -> SourceSpanV1:
    return SourceSpanV1(
        path="job_advert.md",
        start_line=lines[0],
        end_line=lines[1],
        text_sha256=source_hash,
        occurrence=occurrence,
        occurrence_count=occurrence_count,
    )


def _source_occurrences(advert_text: str, source_text: str) -> tuple[tuple[int, int], ...]:
    normalized_advert, line_map = _normalized_text_with_lines(advert_text)
    normalized_source = _normalized_source_text(source_text)
    if not normalized_source:
        return ()
    occurrences: list[tuple[int, int]] = []
    start = 0
    while True:
        index = normalized_advert.find(normalized_source, start)
        if index < 0:
            break
        end_index = index + len(normalized_source) - 1
        occurrences.append((line_map[index], line_map[end_index]))
        start = index + 1
    return tuple(occurrences)


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
