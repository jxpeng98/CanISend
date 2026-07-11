from __future__ import annotations

import json
import os
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
    criterion_text_sha256,
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
    source_anchor_sha256: str | None = None,
    criterion_text: str | None = None,
) -> Path:
    correction = CriterionCorrectionV1(
        correction_id=CORRECTION_ID,
        criterion_id=criterion_id,
        target_source_sha256=criterion_source_sha256(source_text),
        target_criterion_sha256=criterion_text_sha256(criterion_text or source_text),
        confirmation=confirmation,
        corrected_text=corrected_text,
        source_occurrence=source_occurrence,
        source_anchor_sha256=source_anchor_sha256,
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
    assert [item.path for item in corrected.semantic_inputs] == [
        "parsed_job.json",
        "job_advert.md",
        "confirmed_corrections.yaml",
    ]
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
        source_anchor_sha256=item.source_candidates[1].anchor_sha256,
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
    before_candidate = build_deterministic_confirm_candidate(job_dir)
    parsed_path = job_dir / "parsed_job.json"
    parsed = json.loads(parsed_path.read_text(encoding="utf-8"))
    parsed["salary"] = "A changed but Confirm-irrelevant salary"
    parsed["notes"] = "A changed note"
    parsed_path.write_text(json.dumps(parsed, indent=4) + "\n", encoding="utf-8")

    assert confirm_input_fingerprint(job_dir) == before
    assert build_deterministic_confirm_candidate(job_dir) == before_candidate


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
        target_criterion_sha256=criterion_text_sha256(target.text),
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


def test_correction_overlay_rejects_duplicate_keys(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    overlay_path = job_dir / "confirmed_corrections.yaml"
    overlay_path.write_text(
        """schema_version: 1.0.0
job_id: example-role
job_id: another-role
revision: 0
updated_at: 2026-07-11T12:00:00Z
criteria: []
""",
        encoding="utf-8",
    )

    with pytest.raises(ConfirmStageError):
        build_deterministic_confirm_candidate(job_dir)


@pytest.mark.skipif(os.name == "nt", reason="symlink creation may require elevated privileges")
def test_dangling_correction_symlink_is_not_treated_as_absent(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    (job_dir / "confirmed_corrections.yaml").symlink_to(job_dir / "missing.yaml")

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
        semantic_inputs=(),
    )

    assert projected.criteria[0].source_state == "unknown"
    assert projected.criteria[0].unknown_reason == "source_receipt.not_found"
    assert projected.unresolved_criterion_ids == (projected.criteria[0].criterion_id,)


def test_duplicate_source_receipts_use_semantic_qualifiers_not_list_order() -> None:
    parsed = {
        "essential_criteria": [
            {"criterion": "Doctoral qualification", "source_text": "Shared source receipt"},
            {"criterion": "Economics expertise", "source_text": "Shared source receipt"},
            {"criterion": "Doctoral qualification", "source_text": "Shared source receipt"},
        ],
        "desirable_criteria": [],
    }
    first = project_criteria(
        parsed_job=parsed,
        advert_text="Essential criteria:\n- Shared source receipt\n",
        job_id="example-role",
        corrections=None,
        input_fingerprint="a" * 64,
        semantic_inputs=(),
    )
    parsed["essential_criteria"].reverse()
    second = project_criteria(
        parsed_job=parsed,
        advert_text="Essential criteria:\n- Shared source receipt\n",
        job_id="example-role",
        corrections=None,
        input_fingerprint="a" * 64,
        semantic_inputs=(),
    )

    assert len(first.criteria) == 2
    assert {item.text: item.criterion_id for item in first.criteria} == {
        item.text: item.criterion_id for item in second.criteria
    }

    parsed["essential_criteria"] = [
        item
        for item in parsed["essential_criteria"]
        if item["criterion"] == "Doctoral qualification"
    ]
    without_sibling = project_criteria(
        parsed_job=parsed,
        advert_text="Essential criteria:\n- Shared source receipt\n",
        job_id="example-role",
        corrections=None,
        input_fingerprint="a" * 64,
        semantic_inputs=(),
    )
    doctoral_id = next(
        item.criterion_id
        for item in first.criteria
        if item.text == "Doctoral qualification"
    )
    assert without_sibling.criteria[0].criterion_id == doctoral_id


def test_confirmed_interpretation_change_becomes_reconciliation_action(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    initial = build_deterministic_confirm_candidate(job_dir)
    target = next(item for item in initial.criteria if item.source_text == "PhD in Economics")
    _write_correction(
        job_dir,
        criterion_id=target.criterion_id,
        source_text=target.source_text,
        criterion_text=target.text,
        confirmation="confirmed",
        corrected_text=None,
    )
    parsed_path = job_dir / "parsed_job.json"
    parsed = json.loads(parsed_path.read_text(encoding="utf-8"))
    parsed["essential_criteria"][0]["criterion"] = "Doctoral qualification in a relevant subject"
    parsed_path.write_text(json.dumps(parsed, indent=2) + "\n", encoding="utf-8")

    changed = build_deterministic_confirm_candidate(job_dir)
    projected = next(item for item in changed.criteria if item.source_text == target.source_text)

    assert projected.criterion_id != target.criterion_id
    assert projected.confirmation_state == "unconfirmed"
    assert changed.orphaned_corrections[0].correction_id == CORRECTION_ID
    assert changed.orphaned_corrections[0].reason == "criterion.identity_changed"


def test_inserted_source_occurrence_cannot_retarget_confirmed_anchor(tmp_path: Path) -> None:
    advert = """# Lecturer

The role requires evidence of teaching excellence.

Essential criteria:
- Evidence of teaching excellence
"""
    job_dir = _write_job(tmp_path, advert_text=advert)
    initial = build_deterministic_confirm_candidate(job_dir)
    item = initial.criteria[0]
    selected = item.source_candidates[1]
    _write_correction(
        job_dir,
        criterion_id=item.criterion_id,
        source_text=item.source_text,
        criterion_text=item.text,
        confirmation="confirmed",
        corrected_text=None,
        source_occurrence=selected.occurrence,
        source_anchor_sha256=selected.anchor_sha256,
    )
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8").replace(
            "The role requires evidence of teaching excellence.",
            "Evidence of teaching excellence.\n\nThe role requires evidence of teaching excellence.",
        ),
        encoding="utf-8",
    )

    shifted = build_deterministic_confirm_candidate(job_dir)
    projected = shifted.criteria[0]

    assert projected.confirmation_state == "unconfirmed"
    assert projected.source_state == "unknown"
    assert shifted.orphaned_corrections[0].reason == "source_anchor.changed"


def test_non_unique_source_anchor_cannot_retarget_confirmation() -> None:
    parsed = {
        "essential_criteria": [
            {
                "criterion": "Teaching evidence",
                "source_text": "Evidence of teaching excellence",
            }
        ],
        "desirable_criteria": [],
    }
    advert = (
        "Essential criteria:\n"
        "- Evidence of teaching excellence and Evidence of teaching excellence\n"
    )
    initial = project_criteria(
        parsed_job=parsed,
        advert_text=advert,
        job_id="example-role",
        corrections=None,
        input_fingerprint="a" * 64,
        semantic_inputs=(),
    )
    item = initial.criteria[0]
    selected = item.source_candidates[1]
    assert item.source_candidates[0].anchor_sha256 == selected.anchor_sha256
    correction = CriterionCorrectionV1(
        correction_id=CORRECTION_ID,
        criterion_id=item.criterion_id,
        target_source_sha256=criterion_source_sha256(item.source_text),
        target_criterion_sha256=criterion_text_sha256(item.text),
        confirmation="confirmed",
        corrected_text=None,
        source_occurrence=selected.occurrence,
        source_anchor_sha256=selected.anchor_sha256,
        record_state="active",
        confirmed_at="2026-07-11T12:00:00Z",
    )
    overlay = ConfirmedCorrectionsV1(
        job_id="example-role",
        revision=1,
        updated_at="2026-07-11T12:00:00Z",
        criteria=(correction,),
    )

    projected = project_criteria(
        parsed_job=parsed,
        advert_text=advert,
        job_id="example-role",
        corrections=overlay,
        input_fingerprint="a" * 64,
        semantic_inputs=(),
    )

    assert projected.criteria[0].confirmation_state == "unconfirmed"
    assert projected.criteria[0].source_state == "unknown"
    assert projected.orphaned_corrections[0].reason == "source_anchor.ambiguous"


def test_source_heading_change_invalidates_confirmed_anchor(tmp_path: Path) -> None:
    advert = """# Lecturer

## Essential criteria
Before
- Evidence of teaching excellence
After

## Background
Before
Evidence of teaching excellence
After
"""
    job_dir = _write_job(tmp_path, advert_text=advert)
    initial = build_deterministic_confirm_candidate(job_dir)
    item = initial.criteria[0]
    selected = item.source_candidates[1]
    _write_correction(
        job_dir,
        criterion_id=item.criterion_id,
        source_text=item.source_text,
        criterion_text=item.text,
        confirmation="confirmed",
        corrected_text=None,
        source_occurrence=selected.occurrence,
        source_anchor_sha256=selected.anchor_sha256,
    )
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8").replace(
            "## Background",
            "## Desirable criteria",
        ),
        encoding="utf-8",
    )

    changed = build_deterministic_confirm_candidate(job_dir)

    assert changed.criteria[0].confirmation_state == "unconfirmed"
    assert changed.orphaned_corrections[0].reason == "source_anchor.changed"


def test_bare_parser_section_change_invalidates_confirmed_anchor(tmp_path: Path) -> None:
    advert = """# Lecturer

## First block
Essential criteria
Before
- Evidence of teaching excellence
After

## Second block
Essential criteria
Before
- Evidence of teaching excellence
After
"""
    job_dir = _write_job(tmp_path, advert_text=advert)
    initial = build_deterministic_confirm_candidate(job_dir)
    item = initial.criteria[0]
    selected = item.source_candidates[1]
    _write_correction(
        job_dir,
        criterion_id=item.criterion_id,
        source_text=item.source_text,
        criterion_text=item.text,
        confirmation="confirmed",
        corrected_text=None,
        source_occurrence=selected.occurrence,
        source_anchor_sha256=selected.anchor_sha256,
    )
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8").replace(
            "## Second block\nEssential criteria",
            "## Second block\nDesirable criteria",
        ),
        encoding="utf-8",
    )

    changed = build_deterministic_confirm_candidate(job_dir)

    assert changed.criteria[0].confirmation_state == "unconfirmed"
    assert changed.orphaned_corrections[0].reason == "source_anchor.changed"


def test_common_setext_qualification_change_invalidates_confirmed_anchor(
    tmp_path: Path,
) -> None:
    advert = """# Lecturer

Essential criteria:
- Evidence of teaching excellence

## First block
Required qualifications
-----------------------
Before
Evidence of teaching excellence
After

## Second block
Required qualifications
-----------------------
Before
Evidence of teaching excellence
After
"""
    job_dir = _write_job(tmp_path, advert_text=advert)
    initial = build_deterministic_confirm_candidate(job_dir)
    item = initial.criteria[0]
    selected = item.source_candidates[2]
    _write_correction(
        job_dir,
        criterion_id=item.criterion_id,
        source_text=item.source_text,
        criterion_text=item.text,
        confirmation="confirmed",
        corrected_text=None,
        source_occurrence=selected.occurrence,
        source_anchor_sha256=selected.anchor_sha256,
    )
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8").replace(
            "## Second block\nRequired qualifications",
            "## Second block\nPreferred qualifications",
        ),
        encoding="utf-8",
    )

    changed = build_deterministic_confirm_candidate(job_dir)

    assert changed.criteria[0].confirmation_state == "unconfirmed"
    assert changed.orphaned_corrections[0].reason == "source_anchor.changed"


@pytest.mark.parametrize(
    ("before_heading", "after_heading"),
    [
        ("Required skills", "Preferred skills"),
        ("Mandatory experience", "Desirable experience"),
    ],
)
def test_common_bare_section_change_invalidates_confirmed_anchor(
    tmp_path: Path,
    before_heading: str,
    after_heading: str,
) -> None:
    advert = f"""# Lecturer

Essential criteria:
- Evidence of teaching excellence

## First block
{before_heading}

Before
Evidence of teaching excellence
After

## Second block
{before_heading}

Before
Evidence of teaching excellence
After
"""
    job_dir = _write_job(tmp_path, advert_text=advert)
    initial = build_deterministic_confirm_candidate(job_dir)
    item = initial.criteria[0]
    selected = item.source_candidates[2]
    _write_correction(
        job_dir,
        criterion_id=item.criterion_id,
        source_text=item.source_text,
        criterion_text=item.text,
        confirmation="confirmed",
        corrected_text=None,
        source_occurrence=selected.occurrence,
        source_anchor_sha256=selected.anchor_sha256,
    )
    advert_path = job_dir / "job_advert.md"
    advert_path.write_text(
        advert_path.read_text(encoding="utf-8").replace(
            f"## Second block\n{before_heading}",
            f"## Second block\n{after_heading}",
        ),
        encoding="utf-8",
    )

    changed = build_deterministic_confirm_candidate(job_dir)

    assert changed.criteria[0].confirmation_state == "unconfirmed"
    assert changed.orphaned_corrections[0].reason == "source_anchor.changed"


def test_empty_extraction_is_unknown_not_ready() -> None:
    projected = project_criteria(
        parsed_job={"essential_criteria": [], "desirable_criteria": []},
        advert_text="# Lecturer\nNo selection criteria were extracted.\n",
        job_id="example-role",
        corrections=None,
        input_fingerprint="a" * 64,
        semantic_inputs=(),
    )

    assert projected.extraction_state == "unknown"
    assert projected.extraction_unknown_reason == "criteria.none_extracted"
    assert projected.criteria == ()
