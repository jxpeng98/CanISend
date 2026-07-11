from __future__ import annotations

from collections.abc import Callable
from copy import deepcopy
from hashlib import sha256
import json
from pathlib import Path
from typing import Any

import pytest

from canisend.decision_models import (
    CriteriaCatalogV1,
    CriterionImportance,
    CriterionV1,
    EvidenceCatalogItemV1,
    EvidenceCatalogState,
    EvidenceCatalogV1,
    EvidenceSourceReceiptV1,
    SemanticInputReceiptV1,
    SourceSpanV1,
)
from canisend.stages.match_stage import (
    CRITERIA_INPUT_PATH,
    EVIDENCE_CATALOG_INPUT_PATH,
    MATCHER_STRATEGY,
    MATCHER_VERSION,
    MatchStageError,
    MatchStageValidationError,
    build_deterministic_match_candidate,
    match_input_fingerprint,
    match_input_projection,
    validate_match_candidate,
)


JOB_ID = "example-role"
SHA_A = "a" * 64
SHA_B = "b" * 64
SHA_C = "c" * 64


def _criterion(
    suffix: str,
    text: str,
    *,
    importance: CriterionImportance = "essential",
) -> CriterionV1:
    return CriterionV1(
        criterion_id="criterion_" + suffix * 32,
        importance=importance,
        text=text,
        parsed_text_sha256=sha256(text.encode("utf-8")).hexdigest(),
        source_text=text,
        source_state="known",
        source_span=SourceSpanV1(
            path="job_advert.md",
            start_line=8,
            end_line=8,
            text_sha256=sha256(text.encode("utf-8")).hexdigest(),
            anchor_sha256=SHA_A,
            occurrence=1,
            occurrence_count=1,
        ),
        confidence="high",
        confirmation_state="unconfirmed",
    )


def _criteria_catalog(
    criteria: tuple[CriterionV1, ...],
    *,
    fingerprint: str = SHA_B,
) -> CriteriaCatalogV1:
    return CriteriaCatalogV1(
        job_id=JOB_ID,
        input_fingerprint=fingerprint,
        semantic_inputs=(
            SemanticInputReceiptV1(
                path="parsed_job.json",
                projection_sha256=SHA_A,
            ),
            SemanticInputReceiptV1(
                path="job_advert.md",
                projection_sha256=SHA_B,
            ),
        ),
        extraction_state="extracted" if criteria else "confirmed_empty",
        empty_confirmation_record_id=(
            None if criteria else "correction_" + "f" * 32
        ),
        criteria=criteria,
        unresolved_criterion_ids=tuple(item.criterion_id for item in criteria),
    )


def _evidence_item(
    suffix: str,
    text: str,
    *,
    section: str,
    locator: str,
    kind: str,
) -> EvidenceCatalogItemV1:
    return EvidenceCatalogItemV1(
        evidence_id="evidence_" + suffix * 32,
        path="profile/generated/cv.evidence.md",
        section=section,
        item_locator=locator,
        kind=kind,
        text=text,
        content_sha256=sha256(text.encode("utf-8")).hexdigest(),
    )


def _generated_receipt(*, item_count: int) -> EvidenceSourceReceiptV1:
    return EvidenceSourceReceiptV1(
        path="profile/generated/cv.evidence.md",
        source_type="generated_evidence",
        content_sha256=SHA_C,
        size_bytes=128,
        item_count=item_count,
    )


def _evidence_catalog(
    items: tuple[EvidenceCatalogItemV1, ...],
    *,
    state: EvidenceCatalogState = "available",
    fingerprint: str = SHA_C,
) -> EvidenceCatalogV1:
    items = tuple(sorted(items, key=lambda item: item.evidence_id))
    receipts = (
        (_generated_receipt(item_count=len(items)),)
        if state in {"available", "empty"}
        else ()
    )
    return EvidenceCatalogV1(
        job_id=JOB_ID,
        input_fingerprint=fingerprint,
        state=state,
        unavailable_reason=(
            "evidence.generated_missing" if state == "unavailable" else None
        ),
        source_receipts=receipts,
        items=items,
    )


def _write_inputs(
    job_dir: Path,
    *,
    criteria: CriteriaCatalogV1,
    evidence: EvidenceCatalogV1,
) -> None:
    job_dir.mkdir(parents=True, exist_ok=True)
    _write_model(job_dir / CRITERIA_INPUT_PATH, criteria)
    _write_model(job_dir / EVIDENCE_CATALOG_INPUT_PATH, evidence)


def _write_model(path: Path, model: object) -> None:
    payload = model.model_dump(mode="json")
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _available_inputs(job_dir: Path) -> None:
    criteria = (
        _criterion("3", "Evidence of teaching excellence"),
        _criterion("1", "PhD in Economics"),
        _criterion(
            "2",
            "Ability to communicate with external stakeholders",
            importance="desirable",
        ),
    )
    evidence = (
        _evidence_item(
            "3",
            "Evidence of teaching excellence through lectures and student feedback.",
            section="Teaching",
            locator="cv-003",
            kind="teaching",
        ),
        _evidence_item(
            "1",
            "Completed a PhD in Economics at Example University.",
            section="Education",
            locator="cv-001",
            kind="education",
        ),
        _evidence_item(
            "2",
            "Demonstrated teaching excellence in econometrics seminars.",
            section="Teaching",
            locator="cv-002",
            kind="teaching",
        ),
    )
    _write_inputs(
        job_dir,
        criteria=_criteria_catalog(criteria),
        evidence=_evidence_catalog(evidence),
    )


def test_match_projection_fingerprints_both_catalogs_and_matcher_contract(
    tmp_path: Path,
) -> None:
    job_dir = tmp_path / JOB_ID
    _available_inputs(job_dir)

    projection = match_input_projection(job_dir)
    first = match_input_fingerprint(job_dir)
    evidence = EvidenceCatalogV1.model_validate(
        json.loads((job_dir / EVIDENCE_CATALOG_INPUT_PATH).read_text(encoding="utf-8"))
    )
    changed = evidence.model_copy(update={"input_fingerprint": "d" * 64})
    _write_model(job_dir / EVIDENCE_CATALOG_INPUT_PATH, changed)

    assert projection["matcher_strategy"] == MATCHER_STRATEGY
    assert projection["matcher_version"] == MATCHER_VERSION
    assert projection["criteria_catalog_sha256"]
    assert projection["evidence_catalog_sha256"]
    assert projection["schema_sha256"]
    assert match_input_fingerprint(job_dir) != first


def test_match_fingerprint_changes_with_output_schema(tmp_path: Path) -> None:
    job_dir = tmp_path / JOB_ID
    _available_inputs(job_dir)
    source_schema = Path("schemas/criterion-matches.schema.json")
    schema = json.loads(source_schema.read_text(encoding="utf-8"))
    first_schema = tmp_path / "first.schema.json"
    second_schema = tmp_path / "second.schema.json"
    first_schema.write_text(json.dumps(schema), encoding="utf-8")
    schema["$comment"] = "A contract-only change"
    second_schema.write_text(json.dumps(schema), encoding="utf-8")

    assert match_input_fingerprint(
        job_dir,
        criterion_matches_schema_path=first_schema,
    ) != match_input_fingerprint(
        job_dir,
        criterion_matches_schema_path=second_schema,
    )


def test_deterministic_match_covers_all_criteria_in_semantic_id_order(
    tmp_path: Path,
) -> None:
    job_dir = tmp_path / JOB_ID
    _available_inputs(job_dir)

    candidate = build_deterministic_match_candidate(job_dir)
    matches = {item.criterion_id: item for item in candidate.matches}

    assert tuple(item.criterion_id for item in candidate.matches) == tuple(
        sorted(matches)
    )
    assert matches["criterion_" + "1" * 32].classification == "partial"
    assert matches["criterion_" + "2" * 32].classification == "missing"
    assert matches["criterion_" + "3" * 32].classification == "strong"
    assert all(item.review_state == "proposed" for item in candidate.matches)
    assert tuple(item.evidence_id for item in candidate.evidence_refs) == tuple(
        sorted(item.evidence_id for item in candidate.evidence_refs)
    )
    assert len(candidate.matches) == 3
    dumped_refs = candidate.model_dump(mode="json")["evidence_refs"]
    assert dumped_refs
    assert all("text" not in item for item in dumped_refs)
    assert {item["path"] for item in dumped_refs} == {"evidence_catalog.json"}
    assert {item["section"] for item in dumped_refs} == {"items"}
    assert {item["kind"] for item in dumped_refs} == {"catalog_item"}
    assert all(item["item_locator"] == item["evidence_id"] for item in dumped_refs)


@pytest.mark.parametrize(
    ("state", "gap_code"),
    [
        ("empty", "evidence.catalog_empty"),
        ("unavailable", "evidence.catalog_unavailable"),
    ],
)
def test_empty_or_unavailable_catalog_produces_fixed_unknown_gap(
    tmp_path: Path,
    state: str,
    gap_code: str,
) -> None:
    job_dir = tmp_path / JOB_ID
    criterion = _criterion("1", "PhD in Economics")
    _write_inputs(
        job_dir,
        criteria=_criteria_catalog((criterion,)),
        evidence=_evidence_catalog((), state=state),
    )

    candidate = build_deterministic_match_candidate(job_dir)
    match = candidate.matches[0]

    assert match.classification == "unknown"
    assert match.evidence_ref_ids == ()
    assert tuple(gap.code for gap in match.gaps) == (gap_code,)
    assert candidate.evidence_refs == ()


def test_available_catalog_without_relevant_evidence_is_missing(tmp_path: Path) -> None:
    job_dir = tmp_path / JOB_ID
    _write_inputs(
        job_dir,
        criteria=_criteria_catalog(
            (_criterion("1", "Ability to communicate with external stakeholders"),)
        ),
        evidence=_evidence_catalog(
            (
                _evidence_item(
                    "1",
                    "Published an article on monetary policy.",
                    section="Research",
                    locator="cv-001",
                    kind="research",
                ),
            )
        ),
    )

    match = build_deterministic_match_candidate(job_dir).matches[0]

    assert match.classification == "missing"
    assert match.evidence_ref_ids == ()
    assert tuple(gap.code for gap in match.gaps) == (
        "evidence.no_relevant_support",
    )


def test_weak_match_always_retains_a_resolvable_reference(tmp_path: Path) -> None:
    job_dir = tmp_path / JOB_ID
    evidence_id = "evidence_" + "1" * 32
    _write_inputs(
        job_dir,
        criteria=_criteria_catalog((_criterion("1", "Teaching experience"),)),
        evidence=_evidence_catalog(
            (
                _evidence_item(
                    "1",
                    "Designed curriculum for undergraduate courses.",
                    section="Teaching",
                    locator="cv-001",
                    kind="teaching",
                ),
            )
        ),
    )

    candidate = build_deterministic_match_candidate(job_dir)
    match = candidate.matches[0]

    assert match.classification == "weak"
    assert match.evidence_ref_ids == (evidence_id,)
    assert candidate.evidence_refs[0].evidence_id == evidence_id
    assert tuple(gap.code for gap in match.gaps) == (
        "evidence.direct_support_missing",
    )


def test_structured_evidence_kind_can_propose_weak_match_without_body_overlap(
    tmp_path: Path,
) -> None:
    job_dir = tmp_path / JOB_ID
    _write_inputs(
        job_dir,
        criteria=_criteria_catalog((_criterion("1", "Teaching experience"),)),
        evidence=_evidence_catalog(
            (
                _evidence_item(
                    "1",
                    "ECON101 coordinator",
                    section="Appointments",
                    locator="cv-001",
                    kind="teaching",
                ),
            )
        ),
    )

    match = build_deterministic_match_candidate(job_dir).matches[0]

    assert match.classification == "weak"
    assert match.evidence_ref_ids == ("evidence_" + "1" * 32,)


def test_criteria_reordering_preserves_semantic_match_order(tmp_path: Path) -> None:
    job_dir = tmp_path / JOB_ID
    _available_inputs(job_dir)
    first = build_deterministic_match_candidate(job_dir)
    criteria = CriteriaCatalogV1.model_validate(
        json.loads((job_dir / CRITERIA_INPUT_PATH).read_text(encoding="utf-8"))
    )
    _write_model(
        job_dir / CRITERIA_INPUT_PATH,
        criteria.model_copy(update={"criteria": tuple(reversed(criteria.criteria))}),
    )

    second = build_deterministic_match_candidate(job_dir)

    assert second.matches == first.matches
    assert second.evidence_refs == first.evidence_refs
    assert second.input_fingerprint != first.input_fingerprint


def _remove_match(payload: dict[str, Any]) -> None:
    payload["matches"] = payload["matches"][:-1]


def _forge_catalog_hash(payload: dict[str, Any]) -> None:
    payload["criteria_catalog_sha256"] = "f" * 64


def _forge_reference_locator(payload: dict[str, Any]) -> None:
    payload["evidence_refs"][0]["path"] = "profile/generated/forged.evidence.md"


def _add_extra_match(payload: dict[str, Any]) -> None:
    payload["matches"].append(
        {
            "criterion_id": "criterion_" + "f" * 32,
            "classification": "unknown",
            "evidence_ref_ids": [],
            "gaps": [
                {
                    "code": "evidence.catalog_unavailable",
                    "message": "The normalized evidence catalog is not available for matching.",
                    "next_action": "Refresh profile evidence and rerun the Evidence stage.",
                }
            ],
            "review_state": "proposed",
        }
    )


@pytest.mark.parametrize(
    "mutate",
    [
        _remove_match,
        _add_extra_match,
        _forge_catalog_hash,
        _forge_reference_locator,
    ],
)
def test_validator_rebuilds_canonical_candidate_and_rejects_tampering(
    tmp_path: Path,
    mutate: Callable[[dict[str, Any]], None],
) -> None:
    job_dir = tmp_path / JOB_ID
    _available_inputs(job_dir)
    candidate = build_deterministic_match_candidate(job_dir)
    payload = deepcopy(candidate.model_dump(mode="json"))
    mutate(payload)

    with pytest.raises(MatchStageValidationError):
        validate_match_candidate(
            payload,
            job_dir=job_dir,
            input_fingerprint=candidate.input_fingerprint,
        )


def test_validator_rejects_candidate_after_evidence_input_changes(
    tmp_path: Path,
) -> None:
    job_dir = tmp_path / JOB_ID
    _available_inputs(job_dir)
    candidate = build_deterministic_match_candidate(job_dir)
    evidence = EvidenceCatalogV1.model_validate(
        json.loads((job_dir / EVIDENCE_CATALOG_INPUT_PATH).read_text(encoding="utf-8"))
    )
    _write_model(
        job_dir / EVIDENCE_CATALOG_INPUT_PATH,
        evidence.model_copy(update={"input_fingerprint": "e" * 64}),
    )

    with pytest.raises(MatchStageValidationError, match="stale"):
        validate_match_candidate(
            candidate.model_dump(mode="json"),
            job_dir=job_dir,
            input_fingerprint=candidate.input_fingerprint,
        )


def test_match_rejects_inputs_owned_by_another_job(tmp_path: Path) -> None:
    job_dir = tmp_path / JOB_ID
    criterion = _criterion("1", "PhD in Economics")
    evidence = _evidence_catalog(
        (
            _evidence_item(
                "1",
                "Completed a PhD in Economics.",
                section="Education",
                locator="cv-001",
                kind="education",
            ),
        )
    )
    _write_inputs(
        job_dir,
        criteria=_criteria_catalog((criterion,)),
        evidence=evidence.model_copy(update={"job_id": "another-role"}),
    )

    with pytest.raises(MatchStageError, match="different job"):
        match_input_fingerprint(job_dir)


def test_match_rejects_unknown_empty_criteria_extraction(tmp_path: Path) -> None:
    job_dir = tmp_path / JOB_ID
    unknown = CriteriaCatalogV1(
        job_id=JOB_ID,
        input_fingerprint=SHA_B,
        semantic_inputs=(
            SemanticInputReceiptV1(
                path="parsed_job.json",
                projection_sha256=SHA_A,
            ),
        ),
        extraction_state="unknown",
        extraction_unknown_reason="criteria.none_extracted",
        criteria=(),
    )
    _write_inputs(
        job_dir,
        criteria=unknown,
        evidence=_evidence_catalog((), state="unavailable"),
    )

    with pytest.raises(MatchStageError, match="criteria extraction"):
        build_deterministic_match_candidate(job_dir)
