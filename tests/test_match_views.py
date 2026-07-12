from __future__ import annotations

from hashlib import sha256

import pytest

from canisend.decision_models import (
    CriteriaCatalogV1,
    CriterionMatchV1,
    CriterionMatchesV1,
    CriterionV1,
    EvidenceCatalogItemV1,
    EvidenceCatalogV1,
    EvidenceGapV1,
    EvidenceRefV1,
    EvidenceSourceReceiptV1,
    SemanticInputReceiptV1,
    SourceSpanV1,
)
from canisend.match_views import MatchViewError, render_structured_match_views
from canisend.material_review import (
    StructuredCriteriaReview,
    StructuredEssentialCriterion,
    _markdown_table_cells,
    build_material_review_checklist,
)
from canisend.materials import ApplicationMaterials


HASH = "a" * 64
CRITERION_A = "criterion_" + "a" * 32
CRITERION_B = "criterion_" + "b" * 32
EVIDENCE_A = "evidence_" + "a" * 32
EVIDENCE_B = "evidence_" + "b" * 32
PRIVATE_EVIDENCE_BODY = "PRIVATE-EVIDENCE-BODY-STRUCTURED-VIEW-7391"
CORRECTION_A = "correction_" + "a" * 32
CORRECTION_B = "correction_" + "b" * 32


def _criterion(criterion_id: str, importance: str, text: str, line: int) -> CriterionV1:
    text_hash = sha256(text.casefold().encode("utf-8")).hexdigest()
    return CriterionV1(
        criterion_id=criterion_id,
        importance=importance,
        text=text,
        parsed_text_sha256=text_hash,
        source_text=text,
        source_state="known",
        source_span=SourceSpanV1(
            path="job_advert.md",
            start_line=line,
            end_line=line,
            text_sha256=text_hash,
            anchor_sha256=text_hash,
            occurrence=1,
            occurrence_count=1,
        ),
        confidence="high",
    )


def _graph():
    criteria = CriteriaCatalogV1(
        job_id="example-role",
        input_fingerprint=HASH,
        semantic_inputs=(
            SemanticInputReceiptV1(path="parsed_job.json", projection_sha256=HASH),
        ),
        extraction_state="extracted",
        criteria=(
            _criterion(
                CRITERION_A,
                "essential",
                "Teaching | research\nleadership",
                10,
            ),
            _criterion(CRITERION_B, "desirable", "Grant writing", 11),
        ),
        unresolved_criterion_ids=(CRITERION_A, CRITERION_B),
    )
    item_a = EvidenceCatalogItemV1(
        evidence_id=EVIDENCE_A,
        path="profile/generated/cv.evidence.md",
        section="Teaching",
        item_locator="cv-001",
        kind="teaching",
        text=PRIVATE_EVIDENCE_BODY,
        content_sha256="b" * 64,
    )
    item_b = EvidenceCatalogItemV1(
        evidence_id=EVIDENCE_B,
        path="profile/generated/research.evidence.md",
        section="Research",
        item_locator="research-001",
        kind="research",
        text="A second private evidence body",
        content_sha256="c" * 64,
    )
    evidence = EvidenceCatalogV1(
        job_id="example-role",
        input_fingerprint=HASH,
        state="available",
        source_receipts=(
            EvidenceSourceReceiptV1(
                path="profile/generated/cv.evidence.md",
                source_type="generated_evidence",
                content_sha256=HASH,
                size_bytes=100,
                item_count=1,
            ),
            EvidenceSourceReceiptV1(
                path="profile/generated/research.evidence.md",
                source_type="generated_evidence",
                content_sha256="d" * 64,
                size_bytes=100,
                item_count=1,
            ),
        ),
        items=(item_a, item_b),
    )
    matches = CriterionMatchesV1(
        job_id="example-role",
        input_fingerprint=HASH,
        criteria_catalog_sha256=HASH,
        evidence_catalog_sha256=HASH,
        matcher_strategy="deterministic.keyword",
        matcher_version="1.0.0",
        evidence_refs=(
            EvidenceRefV1(
                evidence_id=EVIDENCE_A,
                path="evidence_catalog.json",
                section="items",
                item_locator=EVIDENCE_A,
                kind="catalog_item",
                content_sha256=item_a.content_sha256,
            ),
        ),
        matches=(
            CriterionMatchV1(
                criterion_id=CRITERION_A,
                classification="partial",
                evidence_ref_ids=(EVIDENCE_A,),
                gaps=(
                    EvidenceGapV1(
                        code="evidence.more_detail_needed",
                        message="More detail is needed.",
                        next_action="Add relevant context.",
                    ),
                ),
            ),
            CriterionMatchV1(
                criterion_id=CRITERION_B,
                classification="missing",
                gaps=(
                    EvidenceGapV1(
                        code="evidence.no_relevant_support",
                        message="No support is linked.",
                        next_action="Add supported evidence.",
                    ),
                ),
            ),
        ),
    )
    return criteria, matches, evidence


def _resolved_criteria(criteria: CriteriaCatalogV1) -> CriteriaCatalogV1:
    resolved = tuple(
        CriterionV1.model_validate(
            {
                **item.model_dump(mode="json"),
                "confirmation_state": "confirmed",
                "confirmation_record_id": (
                    CORRECTION_A if item.criterion_id == CRITERION_A else CORRECTION_B
                ),
            }
        )
        for item in criteria.criteria
    )
    return CriteriaCatalogV1.model_validate(
        {
            **criteria.model_dump(mode="json"),
            "criteria": [item.model_dump(mode="json") for item in resolved],
            "unresolved_criterion_ids": [],
        }
    )


def test_structured_match_views_preserve_semantics_without_evidence_bodies() -> None:
    criteria, matches, evidence = _graph()

    views = render_structured_match_views(criteria, matches, evidence)

    assert "Deterministic proposal" in views.fit_report
    assert "PARTIAL (PROPOSED)" in views.fit_report
    assert "MISSING (PROPOSED)" in views.fit_report
    assert "Teaching \\| research leadership" in views.criteria_checklist
    assert "profile/generated/cv.evidence.md#Teaching/cv-001" in views.fit_report
    assert "2 evidence items available" in views.fit_report
    assert "| partial |" in views.criteria_checklist
    assert "| missing |" in views.criteria_checklist
    assert tuple(item.text for item in views.criteria_review.essential_criteria) == (
        "Teaching | research\nleadership",
    )
    assert views.criteria_review.essential_criteria[0].criterion_id == CRITERION_A
    rendered = views.fit_report + views.criteria_checklist
    assert PRIVATE_EVIDENCE_BODY not in rendered
    assert "A second private evidence body" not in rendered


def test_structured_match_views_make_unknown_an_hr_blocker() -> None:
    criteria, matches, evidence = _graph()
    criteria = _resolved_criteria(criteria)
    unknown = matches.model_copy(
        update={
            "evidence_refs": (),
            "matches": (
                CriterionMatchV1(
                    criterion_id=CRITERION_A,
                    classification="unknown",
                    gaps=(
                        EvidenceGapV1(
                            code="evidence.catalog_unavailable",
                            message="Catalog unavailable.",
                            next_action="Refresh evidence.",
                        ),
                    ),
                ),
                matches.matches[1],
            ),
        }
    )
    views = render_structured_match_views(criteria, unknown, evidence)
    materials = ApplicationMaterials(
        fit_report=views.fit_report,
        cover_letter_draft="# Cover\n",
        cv_tailoring_notes="# CV\n",
        criteria_checklist=views.criteria_checklist,
    )
    parsed = {
        "title": "Role",
        "institution": "University",
        "deadline": "2026-08-01",
        "required_documents": [],
        "essential_criteria": [{"criterion": "Old parsed wording"}],
    }

    review = build_material_review_checklist(
        parsed,
        materials,
        structured_criteria=views.criteria_review,
    )

    assert "Teaching \\| research leadership" in review
    assert "Coverage is unknown" in review
    assert "Old parsed wording" not in review.split("## Strict University HR Review", 1)[1]


def test_structured_match_views_make_unresolved_strong_criterion_an_hr_blocker() -> None:
    criteria, matches, evidence = _graph()
    strong = matches.model_copy(
        update={
            "matches": (
                CriterionMatchV1(
                    criterion_id=CRITERION_A,
                    classification="strong",
                    evidence_ref_ids=(EVIDENCE_A,),
                ),
                matches.matches[1],
            )
        }
    )
    views = render_structured_match_views(criteria, strong, evidence)
    materials = ApplicationMaterials(
        fit_report=views.fit_report,
        cover_letter_draft="# Cover\n",
        cv_tailoring_notes="# CV\n",
        criteria_checklist=views.criteria_checklist,
    )

    review = build_material_review_checklist(
        {
            "title": "Role",
            "institution": "University",
            "deadline": "2026-08-01",
            "required_documents": [],
            "essential_criteria": [],
        },
        materials,
        structured_criteria=views.criteria_review,
    )

    hr_section = review.split("## Strict University HR Review", 1)[1]
    assert "Criterion is unresolved" in hr_section
    assert "| OK | Strong coverage recorded" not in hr_section
    assert "Review state: unresolved" in views.fit_report


def test_structured_match_views_escape_adversarial_citation_in_table() -> None:
    criteria, matches, evidence = _graph()
    hostile_item = evidence.items[0].model_copy(
        update={"section": "Teaching \\| injected | `tag` <unsafe>"}
    )
    hostile_evidence = evidence.model_copy(
        update={"items": (hostile_item, evidence.items[1])}
    )

    views = render_structured_match_views(criteria, matches, hostile_evidence)

    row = next(
        line
        for line in views.criteria_checklist.splitlines()
        if "profile/generated/cv.evidence.md" in line
    )
    assert "Teaching \\\\\\| injected \\| 'tag' <unsafe>" in row
    assert len(_markdown_table_cells(row)) == 5


def test_structured_hr_contract_rejects_unknown_states() -> None:
    with pytest.raises(ValueError, match="classification"):
        StructuredEssentialCriterion(
            criterion_id=CRITERION_A,
            text="Criterion",
            classification="unexpected",  # type: ignore[arg-type]
            evidence_linked=True,
            unresolved_reasons=(),
        )
    with pytest.raises(ValueError, match="extraction state"):
        StructuredCriteriaReview(
            extraction_state="unknown",  # type: ignore[arg-type]
            essential_criteria=(),
        )


def test_confirmed_empty_criteria_have_accurate_view_and_hr_semantics() -> None:
    _, matches, evidence = _graph()
    criteria = CriteriaCatalogV1(
        job_id="example-role",
        input_fingerprint=HASH,
        semantic_inputs=(
            SemanticInputReceiptV1(path="parsed_job.json", projection_sha256=HASH),
        ),
        extraction_state="confirmed_empty",
        empty_confirmation_record_id=CORRECTION_A,
        criteria=(),
    )
    empty_matches = matches.model_copy(update={"evidence_refs": (), "matches": ()})
    views = render_structured_match_views(criteria, empty_matches, evidence)
    materials = ApplicationMaterials(
        fit_report=views.fit_report,
        cover_letter_draft="# Cover\n",
        cv_tailoring_notes="# CV\n",
        criteria_checklist=views.criteria_checklist,
    )

    review = build_material_review_checklist(
        {
            "title": "Role",
            "institution": "University",
            "deadline": "2026-08-01",
            "required_documents": [],
            "essential_criteria": [],
        },
        materials,
        structured_criteria=views.criteria_review,
    )

    assert "explicitly confirmed empty" in views.fit_report
    assert "| No criteria advertised | confirmed_empty |" in views.criteria_checklist
    assert "| No essential criteria advertised | OK |" in review


def test_structured_match_views_reject_incomplete_or_inconsistent_graphs() -> None:
    criteria, matches, evidence = _graph()

    with pytest.raises(MatchViewError, match="exactly one match"):
        render_structured_match_views(
            criteria,
            matches.model_copy(update={"matches": matches.matches[:1]}),
            evidence,
        )

    bad_ref = matches.evidence_refs[0].model_copy(update={"content_sha256": "f" * 64})
    with pytest.raises(MatchViewError, match="metadata is inconsistent"):
        render_structured_match_views(
            criteria,
            matches.model_copy(update={"evidence_refs": (bad_ref,)}),
            evidence,
        )


def test_structured_match_views_require_resolved_criteria() -> None:
    criteria, matches, evidence = _graph()
    unknown_criteria = CriteriaCatalogV1(
        job_id=criteria.job_id,
        input_fingerprint=criteria.input_fingerprint,
        semantic_inputs=criteria.semantic_inputs,
        extraction_state="unknown",
        extraction_unknown_reason="criteria.source_not_found",
        criteria=(),
    )
    empty_matches = matches.model_copy(update={"evidence_refs": (), "matches": ()})

    with pytest.raises(MatchViewError, match="resolved criteria"):
        render_structured_match_views(unknown_criteria, empty_matches, evidence)
