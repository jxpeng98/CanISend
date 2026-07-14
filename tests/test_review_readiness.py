from __future__ import annotations

from datetime import UTC, datetime

import pytest
from pydantic import ValidationError

from canisend.draft_models import ReviewFindingsV1
from canisend.review_readiness import (
    FindingDispositionV1,
    ReviewDispositionDocumentKind,
    ReviewDispositionsV1,
    derive_document_readiness,
)
from tests.test_draft_models import DOCUMENT_ID, JOB_ID, finding


NOW = datetime(2026, 7, 14, 12, 0, tzinfo=UTC)
DRAFT_SHA = "a" * 64
REVIEW_SHA = "b" * 64
DISPOSITIONS_SHA = "c" * 64


def _review(*, blocker: bool = False) -> ReviewFindingsV1:
    item = finding(severity="blocker" if blocker else "review")
    return ReviewFindingsV1(
        job_id=JOB_ID,
        document_id=DOCUMENT_ID,
        input_fingerprint="1" * 64,
        draft_sha256=DRAFT_SHA,
        reviewer_strategy="deterministic.cover_letter_review",
        reviewer_version="1.0.0",
        findings=(item,),
        blocker_finding_ids=(item.finding_id,) if blocker else (),
    )


def _dispositions(
    review: ReviewFindingsV1,
    *,
    value: str = "accepted",
    review_sha: str = REVIEW_SHA,
    document_kind: ReviewDispositionDocumentKind = "cover_letter",
) -> ReviewDispositionsV1:
    return ReviewDispositionsV1(
        job_id=review.job_id,
        document_id=review.document_id,
        document_kind=document_kind,
        revision=0,
        updated_at=NOW,
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=review_sha,
        dispositions=(
            FindingDispositionV1(
                finding_id=review.findings[0].finding_id,
                disposition=value,
                decided_at=NOW,
            ),
        ),
    )


def test_current_complete_acceptance_derives_reviewed_without_mutating_review() -> None:
    review = _review()
    readiness = derive_document_readiness(
        review,
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review),
        review_dispositions_sha256=DISPOSITIONS_SHA,
    )

    assert readiness.state == "reviewed"
    assert readiness.accepted_finding_ids == (review.findings[0].finding_id,)
    assert readiness.review_dispositions_sha256 == DISPOSITIONS_SHA
    assert readiness.reason_codes == ()
    assert review.review_state == "proposed"


def test_research_statement_readiness_requires_matching_document_kind() -> None:
    review = _review()
    research = derive_document_readiness(
        review,
        document_kind="research_statement",
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review, document_kind="research_statement"),
        review_dispositions_sha256=DISPOSITIONS_SHA,
    )
    mismatched = derive_document_readiness(
        review,
        document_kind="research_statement",
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review),
        review_dispositions_sha256=DISPOSITIONS_SHA,
    )

    assert research.document_kind == "research_statement"
    assert research.state == "reviewed"
    assert mismatched.state == "review_required"
    assert mismatched.reason_codes == ("review.dispositions_stale",)


def test_legacy_cover_dispositions_default_document_kind() -> None:
    review = _review()
    payload = _dispositions(review).model_dump(mode="json")
    payload.pop("document_kind")

    restored = ReviewDispositionsV1.model_validate(payload)

    assert restored.document_kind == "cover_letter"


def test_missing_or_stale_dispositions_fail_closed() -> None:
    review = _review()

    missing = derive_document_readiness(
        review,
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=REVIEW_SHA,
        dispositions=None,
        review_dispositions_sha256=None,
    )
    stale = derive_document_readiness(
        review,
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review, review_sha="9" * 64),
        review_dispositions_sha256=DISPOSITIONS_SHA,
    )

    assert missing.state == "review_required"
    assert missing.reason_codes == ("review.dispositions_missing",)
    assert stale.state == "review_required"
    assert stale.review_dispositions_sha256 is None
    assert stale.reason_codes == ("review.dispositions_stale",)

    mismatched_draft = derive_document_readiness(
        review,
        draft_sha256="9" * 64,
        review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review),
        review_dispositions_sha256=DISPOSITIONS_SHA,
    )
    assert mismatched_draft.state == "review_required"
    assert "review.draft_receipt_mismatch" in mismatched_draft.reason_codes
    assert mismatched_draft.review_dispositions_sha256 is None


def test_revision_disposition_prevents_reviewed_state() -> None:
    review = _review()
    readiness = derive_document_readiness(
        review,
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review, value="revision_required"),
        review_dispositions_sha256=DISPOSITIONS_SHA,
    )

    assert readiness.state == "revision_required"
    assert readiness.revision_required_finding_ids == (
        review.findings[0].finding_id,
    )


def test_blocker_cannot_be_accepted_into_readiness() -> None:
    review = _review(blocker=True)
    readiness = derive_document_readiness(
        review,
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review),
        review_dispositions_sha256=DISPOSITIONS_SHA,
    )

    assert readiness.state == "blocked"
    assert readiness.accepted_finding_ids == ()
    assert readiness.blocker_finding_ids == (review.findings[0].finding_id,)
    assert "review.blocker_nonwaivable" in readiness.reason_codes


def test_disposition_model_requires_stable_unique_order_and_control_time() -> None:
    review = _review()
    item = _dispositions(review).dispositions[0]
    duplicate = _dispositions(review).model_dump(mode="json")
    duplicate["dispositions"] = [item.model_dump(mode="json")] * 2

    with pytest.raises(ValidationError):
        ReviewDispositionsV1.model_validate(duplicate)
    with pytest.raises(ValidationError):
        ReviewDispositionsV1.model_validate(
            {
                **_dispositions(review).model_dump(mode="json"),
                "dispositions": [
                    {
                        **item.model_dump(mode="json"),
                        "decided_at": "2026-07-14T13:00:00Z",
                    }
                ],
            }
        )
