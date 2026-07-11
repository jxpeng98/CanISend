from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml

from canisend.decision_models import ConfirmedCorrectionsV1, CriterionCorrectionV1
from canisend.parse import parse_job_advert
from canisend.stages.confirm_stage import (
    ConfirmStageError,
    ConfirmStageValidationError,
    build_deterministic_confirm_candidate,
    confirm_input_fingerprint,
    criterion_source_sha256,
    project_criteria,
    stable_criterion_id,
    validate_confirm_candidate,
)
from canisend.stage_store import sha256_file


CORRECTION_ID = "correction_" + "a" * 32


def _write_job(tmp_path: Path, *, advert_text: str | None = None) -> Path:
    job_dir = tmp_path / "example-role"
    job_dir.mkdir()
    advert = advert_text or """# Lecturer in Economics

Department: Economics
Salary: Competitive
Required documents: CV, Cover letter

Essential criteria:
- PhD in Economics
- Evidence of teaching excellence

Desirable criteria:
- Experience teaching econometrics
"""
    metadata = {
        "title": "Lecturer in Economics",
        "institution": "Example University",
        "department": "Economics",
        "location": "London",
        "deadline": "2026-08-01",
        "source_url": "https://example.edu/job",
    }
    parsed = parse_job_advert(advert, metadata)
    (job_dir / "job_advert.md").write_text(advert, encoding="utf-8")
    (job_dir / "parsed_job.json").write_text(
        json.dumps(parsed, indent=2) + "\n",
        encoding="utf-8",
    )
    return job_dir


def _write_correction(
    job_dir: Path,
    *,
    criterion_id: str,
    source_text: str,
    confirmation: str = "corrected",
    corrected_text: str | None = "Doctorate in Economics or a related discipline",
    source_occurrence: int | None = None,
) -> Path:
    correction = CriterionCorrectionV1(
        correction_id=CORRECTION_ID,
        criterion_id=criterion_id,
        target_source_sha256=criterion_source_sha256(source_text),
        confirmation=confirmation,
        corrected_text=corrected_text,
        source_occurrence=source_occurrence,
        record_state="active",
        confirmed_at="2026-07-11T12:00:00Z",
    )
    overlay = ConfirmedCorrectionsV1(
        job_id=job_dir.name,
        revision=1,
        updated_at="2026-07-11T12:00:00Z",
        criteria=(correction,),
    )
    path = job_dir / "confirmed_corrections.yaml"
    path.write_text(
        yaml.safe_dump(overlay.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )
    return path


def test_projected_criteria_have_stable_ids_and_exact_source_spans(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)

    first = build_deterministic_confirm_candidate(job_dir)
    first_ids = {item.source_text: item.criterion_id for item in first.criteria}
    original_lines = (job_dir / "job_advert.md").read_text(encoding="utf-8").splitlines()

    for item in first.criteria:
        assert item.source_state == "known"
        assert item.source_span is not None
        span_text = "\n".join(
            original_lines[item.source_span.start_line - 1 : item.source_span.end_line]
        )
        assert item.source_text.casefold() in span_text.casefold()

    parsed_path = job_dir / "parsed_job.json"
    parsed = json.loads(parsed_path.read_text(encoding="utf-8"))
    parsed["essential_criteria"].reverse()
    parsed_path.write_text(json.dumps(parsed, indent=2) + "\n", encoding="utf-8")
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8").replace(
            "Department: Economics",
            "Department: Economics\n\nA new unrelated advert line.",
        ),
        encoding="utf-8",
    )

    second = build_deterministic_confirm_candidate(job_dir)
    second_ids = {item.source_text: item.criterion_id for item in second.criteria}

    assert second_ids == first_ids
    assert second.input_fingerprint != first.input_fingerprint


def test_corrected_text_preserves_criterion_identity_and_links_confirmation(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    initial = build_deterministic_confirm_candidate(job_dir)
    target = next(item for item in initial.criteria if item.source_text == "PhD in Economics")
    correction_path = _write_correction(
        job_dir,
        criterion_id=target.criterion_id,
        source_text=target.source_text,
    )
    correction_hash = sha256_file(correction_path)
    correction_mtime = correction_path.stat().st_mtime_ns

    corrected = build_deterministic_confirm_candidate(job_dir)
    projected = next(item for item in corrected.criteria if item.source_text == "PhD in Economics")

    assert projected.criterion_id == target.criterion_id
    assert projected.text == "Doctorate in Economics or a related discipline"
    assert projected.confirmation_state == "corrected"
    assert projected.confirmation_record_id == CORRECTION_ID
    assert target.criterion_id not in corrected.unresolved_criterion_ids
    assert sha256_file(correction_path) == correction_hash
    assert correction_path.stat().st_mtime_ns == correction_mtime


def test_changed_source_creates_new_id_and_preserves_old_correction_as_orphan(
    tmp_path: Path,
) -> None:
    job_dir = _write_job(tmp_path)
    initial = build_deterministic_confirm_candidate(job_dir)
    target = next(item for item in initial.criteria if item.source_text == "PhD in Economics")
    correction_path = _write_correction(
        job_dir,
        criterion_id=target.criterion_id,
        source_text=target.source_text,
        confirmation="confirmed",
        corrected_text=None,
    )
    correction_bytes = correction_path.read_bytes()
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8").replace(
            "PhD in Economics",
            "PhD in Economics or a related field",
        ),
        encoding="utf-8",
    )
    parsed_path = job_dir / "parsed_job.json"
    parsed = json.loads(parsed_path.read_text(encoding="utf-8"))
    parsed["essential_criteria"][0] = {
        "criterion": "PhD in Economics or a related field",
        "source_text": "PhD in Economics or a related field",
    }
    parsed_path.write_text(json.dumps(parsed, indent=2) + "\n", encoding="utf-8")

    changed = build_deterministic_confirm_candidate(job_dir)
    replacement = next(
        item for item in changed.criteria if item.source_text == "PhD in Economics or a related field"
    )

    assert replacement.criterion_id != target.criterion_id
    assert replacement.confirmation_state == "unconfirmed"
    assert changed.orphaned_correction_ids == (CORRECTION_ID,)
    assert correction_path.read_bytes() == correction_bytes


def test_ambiguous_source_is_unknown_until_user_selects_occurrence(tmp_path: Path) -> None:
    advert = """# Lecturer

The role requires evidence of teaching excellence.

Essential criteria:
- Evidence of teaching excellence
"""
    job_dir = _write_job(tmp_path, advert_text=advert)

    ambiguous = build_deterministic_confirm_candidate(job_dir)
    item = ambiguous.criteria[0]

    assert item.source_state == "unknown"
    assert item.source_span is None
    assert item.confidence == "unknown"
    assert item.unknown_reason == "source_receipt.ambiguous"
    assert item.criterion_id in ambiguous.unresolved_criterion_ids

    _write_correction(
        job_dir,
        criterion_id=item.criterion_id,
        source_text=item.source_text,
        confirmation="confirmed",
        corrected_text=None,
        source_occurrence=2,
    )
    resolved = build_deterministic_confirm_candidate(job_dir)
    confirmed = resolved.criteria[0]

    assert confirmed.source_state == "known"
    assert confirmed.source_span is not None
    assert confirmed.source_span.occurrence == 2
    assert confirmed.source_span.occurrence_count == 2
    assert confirmed.confidence == "medium"
    assert confirmed.confirmation_state == "confirmed"
    assert confirmed.criterion_id not in resolved.unresolved_criterion_ids


def test_confirm_fingerprint_uses_semantic_parsed_projection(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    before = confirm_input_fingerprint(job_dir)
    parsed_path = job_dir / "parsed_job.json"
    parsed = json.loads(parsed_path.read_text(encoding="utf-8"))
    parsed["salary"] = "A changed but Confirm-irrelevant salary"
    parsed["notes"] = "A changed note"
    parsed_path.write_text(json.dumps(parsed, indent=4) + "\n", encoding="utf-8")

    assert confirm_input_fingerprint(job_dir) == before


def test_inactive_correction_history_does_not_change_confirm_output_fingerprint(
    tmp_path: Path,
) -> None:
    job_dir = _write_job(tmp_path)
    before = confirm_input_fingerprint(job_dir)
    initial = build_deterministic_confirm_candidate(job_dir)
    target = initial.criteria[0]
    correction = CriterionCorrectionV1(
        correction_id=CORRECTION_ID,
        criterion_id=target.criterion_id,
        target_source_sha256=criterion_source_sha256(target.source_text),
        confirmation="confirmed",
        record_state="withdrawn",
        confirmed_at="2026-07-11T12:00:00Z",
    )
    overlay = ConfirmedCorrectionsV1(
        job_id=job_dir.name,
        revision=7,
        updated_at="2026-07-11T13:00:00Z",
        criteria=(correction,),
    )
    (job_dir / "confirmed_corrections.yaml").write_text(
        yaml.safe_dump(overlay.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )

    assert confirm_input_fingerprint(job_dir) == before


def test_confirm_candidate_validation_rejects_drift_and_unknown_fields(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    candidate = build_deterministic_confirm_candidate(job_dir)
    payload = candidate.model_dump(mode="json")

    validated = validate_confirm_candidate(
        payload,
        job_dir=job_dir,
        input_fingerprint=candidate.input_fingerprint,
    )
    assert validated == candidate

    changed = json.loads(json.dumps(payload))
    changed["criteria"][0]["text"] = "An unconfirmed agent rewrite"
    with pytest.raises(ConfirmStageValidationError):
        validate_confirm_candidate(
            changed,
            job_dir=job_dir,
            input_fingerprint=candidate.input_fingerprint,
        )

    extra = json.loads(json.dumps(payload))
    extra["private_body"] = "must be rejected"
    with pytest.raises(ConfirmStageValidationError):
        validate_confirm_candidate(
            extra,
            job_dir=job_dir,
            input_fingerprint=candidate.input_fingerprint,
        )


def test_invalid_or_wrong_job_correction_overlay_fails_without_rewrite(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    overlay_path = job_dir / "confirmed_corrections.yaml"
    overlay_path.write_text("!!python/object:unsafe {}\n", encoding="utf-8")
    original = overlay_path.read_bytes()

    with pytest.raises(ConfirmStageError):
        build_deterministic_confirm_candidate(job_dir)
    assert overlay_path.read_bytes() == original

    overlay_path.write_text(
        yaml.safe_dump(
            {
                "schema_version": "1.0.0",
                "job_id": "different-job",
                "revision": 0,
                "updated_at": "2026-07-11T12:00:00Z",
                "criteria": [],
            }
        ),
        encoding="utf-8",
    )
    with pytest.raises(ConfirmStageError):
        build_deterministic_confirm_candidate(job_dir)


def test_stable_id_is_not_derived_from_corrected_text_or_source_line() -> None:
    first = stable_criterion_id(
        job_id="example-role",
        importance="essential",
        source_text="PhD in Economics",
        duplicate_occurrence=1,
    )
    second = stable_criterion_id(
        job_id="example-role",
        importance="essential",
        source_text="  PhD   in ECONOMICS  ",
        duplicate_occurrence=1,
    )

    assert first == second


def test_project_criteria_can_represent_missing_source_as_explicit_unknown() -> None:
    parsed = {
        "essential_criteria": [
            {"criterion": "Missing receipt", "source_text": "Missing receipt"}
        ],
        "desirable_criteria": [],
    }

    projected = project_criteria(
        parsed_job=parsed,
        advert_text="# Different text\n",
        job_id="example-role",
        corrections=None,
        input_fingerprint="f" * 64,
        inputs=(),
    )

    assert projected.criteria[0].source_state == "unknown"
    assert projected.criteria[0].unknown_reason == "source_receipt.not_found"
    assert projected.unresolved_criterion_ids == (projected.criteria[0].criterion_id,)
