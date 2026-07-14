from __future__ import annotations

from copy import deepcopy

import pytest
from pydantic import ValidationError

from canisend.draft_models import (
    COVER_LETTER_DRAFT_SCHEMA_VERSION,
    REVIEW_FINDINGS_SCHEMA_VERSION,
    ClaimV1,
    CoverLetterDraftV1,
    DraftBasisV1,
    DraftSectionV1,
    RESEARCH_STATEMENT_DRAFT_SCHEMA_VERSION,
    ResearchStatementDraftV1,
    ReviewFindingV1,
    ReviewFindingsV1,
    stable_claim_id,
    stable_finding_id,
)


JOB_ID = "lecturer-economics"
DOCUMENT_ID = "document_" + "d" * 32
CRITERION_ID = "criterion_" + "c" * 32
EVIDENCE_ID = "evidence_" + "e" * 32
EVIDENCE_ID_2 = "evidence_" + "f" * 32
HASH = "a" * 64


def basis() -> DraftBasisV1:
    return DraftBasisV1(
        parsed_job_sha256=HASH,
        criteria_sha256="b" * 64,
        evidence_catalog_sha256="c" * 64,
        criterion_matches_sha256="d" * 64,
        application_decision_sha256="e" * 64,
        application_brief_sha256="f" * 64,
        required_document_plan_sha256="0" * 64,
    )


def claim(
    text: str = "I designed and taught applied econometrics modules.",
    *,
    kind: str = "factual",
    support_strength: str = "strong",
    criterion_ids: tuple[str, ...] = (CRITERION_ID,),
    evidence_ref_ids: tuple[str, ...] = (EVIDENCE_ID,),
    brief_field_refs: tuple[str, ...] = (),
    job_field_refs: tuple[str, ...] = (),
    blockers: tuple[str, ...] = (),
) -> ClaimV1:
    return ClaimV1(
        claim_id=stable_claim_id(
            job_id=JOB_ID,
            document_id=DOCUMENT_ID,
            kind=kind,  # type: ignore[arg-type]
            text=text,
        ),
        text=text,
        kind=kind,
        support_strength=support_strength,
        criterion_ids=criterion_ids,
        evidence_ref_ids=evidence_ref_ids,
        brief_field_refs=brief_field_refs,
        job_field_refs=job_field_refs,
        blockers=blockers,
    )


def draft(*claims: ClaimV1, blockers: tuple[str, ...] = ()) -> CoverLetterDraftV1:
    selected = claims or (claim(),)
    return CoverLetterDraftV1(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        input_fingerprint="1" * 64,
        basis=basis(),
        generation_mode="host_agent",
        generator_strategy="host_agent.cover_letter",
        generator_version="1.0.0",
        sections=(DraftSectionV1(section_id="body", claims=selected),),
        blockers=blockers,
    )


def finding(
    *,
    code: str = "claim.unsupported",
    severity: str = "blocker",
    message: str = "A factual statement has no current Evidence reference.",
    claim_ids: tuple[str, ...] | None = None,
) -> ReviewFindingV1:
    selected_claim_ids = claim_ids or (claim().claim_id,)
    return ReviewFindingV1(
        finding_id=stable_finding_id(
            job_id=JOB_ID,
            document_id=DOCUMENT_ID,
            code=code,
            message=message,
            claim_ids=selected_claim_ids,
            criterion_ids=(CRITERION_ID,),
        ),
        code=code,
        severity=severity,
        category="support",
        message=message,
        next_action="Remove the statement or attach current Evidence and narrow the wording.",
        claim_ids=selected_claim_ids,
        criterion_ids=(CRITERION_ID,),
    )


def test_cover_letter_draft_is_frozen_structured_and_proposed() -> None:
    candidate = draft()

    assert candidate.schema_version == COVER_LETTER_DRAFT_SCHEMA_VERSION
    assert candidate.review_state == "proposed"
    assert candidate.sections[0].claims[0].review_state == "proposed"
    assert candidate.blockers == ()
    with pytest.raises(ValidationError):
        CoverLetterDraftV1.model_validate(
            {**candidate.model_dump(), "final": True}
        )


def test_research_statement_draft_reuses_the_guarded_claim_graph() -> None:
    cover = draft()
    candidate = ResearchStatementDraftV1(
        job_id=cover.job_id,
        document_id=cover.document_id,
        input_fingerprint=cover.input_fingerprint,
        basis=cover.basis,
        generation_mode="host_agent",
        generator_strategy="host_agent.research_statement",
        generator_version="1.0.0",
        sections=(
            DraftSectionV1(
                section_id="research_overview",
                claims=cover.sections[0].claims,
            ),
        ),
    )

    assert candidate.schema_version == RESEARCH_STATEMENT_DRAFT_SCHEMA_VERSION
    assert candidate.review_state == "proposed"
    assert candidate.blockers == ()
    with pytest.raises(ValidationError):
        ResearchStatementDraftV1.model_validate(
            {**candidate.model_dump(), "submission_ready": True}
        )


def test_draft_section_rejects_untracked_applicant_facing_heading() -> None:
    payload = draft().model_dump()
    payload["sections"][0]["heading"] = "I won an unverified national teaching award."

    with pytest.raises(ValidationError):
        CoverLetterDraftV1.model_validate(payload)


@pytest.mark.parametrize(
    "payload",
    [
        {
            "kind": "factual",
            "support_strength": "strong",
            "evidence_ref_ids": (),
            "blockers": (),
        },
        {
            "kind": "factual",
            "support_strength": "partial",
            "evidence_ref_ids": (EVIDENCE_ID,),
            "blockers": (),
        },
        {
            "kind": "factual",
            "support_strength": "unsupported",
            "evidence_ref_ids": (),
            "blockers": (),
        },
        {
            "kind": "motivation",
            "support_strength": "not_applicable",
            "evidence_ref_ids": (),
            "brief_field_refs": (),
        },
        {
            "kind": "future_intent",
            "support_strength": "not_applicable",
            "criterion_ids": (),
            "evidence_ref_ids": (),
            "brief_field_refs": (),
        },
        {
            "kind": "role_context",
            "support_strength": "not_applicable",
            "criterion_ids": (),
            "evidence_ref_ids": (),
            "job_field_refs": (),
        },
        {
            "kind": "administrative",
            "support_strength": "not_applicable",
            "criterion_ids": (CRITERION_ID,),
            "evidence_ref_ids": (),
        },
    ],
)
def test_claim_rejects_inconsistent_support_and_basis(payload: dict[str, object]) -> None:
    base = claim().model_dump()
    text = "A deliberately invalid claim."
    base.update(payload)
    base["text"] = text
    base["claim_id"] = stable_claim_id(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        kind=base["kind"],  # type: ignore[arg-type]
        text=text,
    )

    with pytest.raises(ValidationError):
        ClaimV1.model_validate(base)


def test_claim_accepts_each_declared_non_evidence_basis() -> None:
    motivation = claim(
        "The department's public-economics focus motivates my application.",
        kind="motivation",
        support_strength="not_applicable",
        criterion_ids=(),
        evidence_ref_ids=(),
        brief_field_refs=("motivation",),
    )
    future = claim(
        "I would develop an applied policy-evaluation workshop.",
        kind="future_intent",
        support_strength="not_applicable",
        criterion_ids=(CRITERION_ID,),
        evidence_ref_ids=(),
    )
    context = claim(
        "The role includes teaching applied microeconomics.",
        kind="role_context",
        support_strength="not_applicable",
        criterion_ids=(),
        evidence_ref_ids=(),
        job_field_refs=("title",),
    )
    administrative = claim(
        "Dear Selection Committee,",
        kind="administrative",
        support_strength="not_applicable",
        criterion_ids=(),
        evidence_ref_ids=(),
    )

    candidate = draft(administrative, motivation, future, context)
    assert len(candidate.sections[0].claims) == 4


def test_partial_and_unsupported_claims_roll_up_exact_draft_blockers() -> None:
    partial = claim(
        "I have extensive experience teaching every econometrics method.",
        support_strength="partial",
        blockers=("claim.partial_support",),
    )
    unsupported = claim(
        "I won a national teaching award.",
        support_strength="unsupported",
        evidence_ref_ids=(),
        blockers=("claim.unsupported",),
    )

    candidate = draft(
        partial,
        unsupported,
        blockers=("claim.partial_support", "claim.unsupported"),
    )
    assert candidate.blockers == ("claim.partial_support", "claim.unsupported")

    with pytest.raises(ValidationError):
        draft(partial, unsupported, blockers=("claim.unsupported",))


def test_claim_ids_are_content_derived_and_ignore_layout_and_reference_order() -> None:
    normalized = stable_claim_id(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        kind="factual",
        text="  I TAUGHT\nApplied Econometrics. ",
    )
    equivalent = stable_claim_id(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        kind="factual",
        text="i taught applied econometrics.",
    )
    changed = stable_claim_id(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        kind="factual",
        text="I taught introductory econometrics.",
    )

    assert normalized == equivalent
    assert changed != normalized

    supported = claim(evidence_ref_ids=(EVIDENCE_ID, EVIDENCE_ID_2))
    moved = CoverLetterDraftV1.model_validate(
        {
            **draft(supported).model_dump(),
            "sections": [{"section_id": "experience", "claims": [supported.model_dump()]}],
        }
    )
    assert moved.sections[0].claims[0].claim_id == supported.claim_id


def test_reference_sets_must_be_unique_and_lexically_ordered() -> None:
    with pytest.raises(ValidationError):
        claim(evidence_ref_ids=(EVIDENCE_ID_2, EVIDENCE_ID))
    with pytest.raises(ValidationError):
        claim(evidence_ref_ids=(EVIDENCE_ID, EVIDENCE_ID))


def test_draft_recomputes_claim_ids_and_rejects_duplicate_or_empty_graphs() -> None:
    payload = draft().model_dump()
    wrong_id = deepcopy(payload)
    wrong_id["sections"][0]["claims"][0]["claim_id"] = "claim_" + "0" * 32
    with pytest.raises(ValidationError):
        CoverLetterDraftV1.model_validate(wrong_id)

    duplicate = deepcopy(payload)
    duplicate["sections"] = [
        deepcopy(duplicate["sections"][0]),
        deepcopy(duplicate["sections"][0]),
    ]
    duplicate["sections"][1]["section_id"] = "second"
    with pytest.raises(ValidationError):
        CoverLetterDraftV1.model_validate(duplicate)

    only_admin = claim(
        "Sincerely,",
        kind="administrative",
        support_strength="not_applicable",
        criterion_ids=(),
        evidence_ref_ids=(),
    )
    with pytest.raises(ValidationError):
        draft(only_admin)


def test_review_findings_use_stable_ids_and_exact_blocker_projection() -> None:
    blocker = finding()
    warning = finding(
        code="style.long_sentence",
        severity="warning",
        message="One claim may be too long for a Cover Letter.",
    )
    ordered = tuple(sorted((blocker, warning), key=lambda item: item.finding_id))
    expected_blockers = tuple(
        item.finding_id for item in ordered if item.severity == "blocker"
    )

    review = ReviewFindingsV1(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        input_fingerprint="2" * 64,
        draft_sha256="3" * 64,
        reviewer_strategy="deterministic.cover_letter_review",
        reviewer_version="1.0.0",
        findings=ordered,
        blocker_finding_ids=expected_blockers,
    )

    assert review.schema_version == REVIEW_FINDINGS_SCHEMA_VERSION
    assert review.review_state == "proposed"
    assert review.blocker_finding_ids == expected_blockers

    with pytest.raises(ValidationError):
        ReviewFindingsV1.model_validate(
            {**review.model_dump(), "blocker_finding_ids": []}
        )


def test_finding_id_is_independent_of_reference_order_but_not_message_content() -> None:
    first = stable_finding_id(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        code="claim.contradiction",
        message="  The CLAIM conflicts with current evidence. ",
        evidence_ref_ids=(EVIDENCE_ID_2, EVIDENCE_ID),
    )
    equivalent = stable_finding_id(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        code="claim.contradiction",
        message="the claim conflicts with current evidence.",
        evidence_ref_ids=(EVIDENCE_ID, EVIDENCE_ID_2),
    )
    changed = stable_finding_id(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        code="claim.contradiction",
        message="The claim is not supported by current evidence.",
        evidence_ref_ids=(EVIDENCE_ID, EVIDENCE_ID_2),
    )

    assert first == equivalent
    assert changed != first


def test_review_collection_rejects_tampered_or_unordered_findings() -> None:
    one = finding()
    two = finding(
        code="style.long_sentence",
        severity="warning",
        message="One claim may be too long for a Cover Letter.",
    )
    ordered = tuple(sorted((one, two), key=lambda item: item.finding_id))
    payload = {
        "job_id": JOB_ID,
        "document_id": DOCUMENT_ID,
        "input_fingerprint": "2" * 64,
        "draft_sha256": "3" * 64,
        "reviewer_strategy": "deterministic.cover_letter_review",
        "reviewer_version": "1.0.0",
        "findings": [item.model_dump() for item in ordered],
        "blocker_finding_ids": [
            item.finding_id for item in ordered if item.severity == "blocker"
        ],
    }

    tampered = deepcopy(payload)
    tampered["findings"][0]["message"] = "Changed after the identifier was calculated."
    with pytest.raises(ValidationError):
        ReviewFindingsV1.model_validate(tampered)

    reversed_payload = deepcopy(payload)
    reversed_payload["findings"].reverse()
    with pytest.raises(ValidationError):
        ReviewFindingsV1.model_validate(reversed_payload)
