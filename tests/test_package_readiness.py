from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, ValidationError as JsonSchemaValidationError
from pydantic import ValidationError

from canisend.package_readiness import (
    ApplicationPackageReadinessV1,
    PackageReviewDispositionsV1,
    derive_application_package_readiness,
)
from canisend.package_review_models import (
    PackageDocumentReviewV1,
    PackageReviewFindingV1,
    PackageReviewFindingsV1,
    stable_package_finding_id,
)
from canisend.review_readiness import FindingDispositionV1


JOB_ID = "lecturer-economics"
DOCUMENT_ID = "document_" + "d" * 32
CLAIM_ID = "claim_" + "c" * 32
REVIEW_SHA = "a" * 64
DISPOSITIONS_SHA = "b" * 64
NOW = "2026-07-14T12:00:00Z"


def _package_review(
    *,
    document_reviewed: bool = True,
    severity: str = "review",
) -> PackageReviewFindingsV1:
    if document_reviewed:
        document = PackageDocumentReviewV1(
            document_id=DOCUMENT_ID,
            normalized_kind="cover_letter",
            requirement="required",
            action="prepare",
            executor_availability="available",
            state="reviewed",
            draft_sha256="1" * 64,
            review_findings_sha256="2" * 64,
            review_dispositions_sha256="3" * 64,
            document_readiness_sha256="4" * 64,
            readiness_state="reviewed",
            claim_ids=(CLAIM_ID,),
        )
    else:
        document = PackageDocumentReviewV1(
            document_id=DOCUMENT_ID,
            normalized_kind="cover_letter",
            requirement="required",
            action="prepare",
            executor_availability="available",
            state="draft_missing",
            reason_codes=("package.document_draft_missing",),
        )
    message = "Cross-document package consistency requires explicit review."
    finding_id = stable_package_finding_id(
        job_id=JOB_ID,
        code="package.semantic_alignment_review",
        message=message,
        document_ids=(DOCUMENT_ID,),
        claim_ids=((CLAIM_ID,) if document_reviewed else ()),
    )
    finding = PackageReviewFindingV1(
        finding_id=finding_id,
        code="package.semantic_alignment_review",
        severity=severity,  # type: ignore[arg-type]
        category="consistency",
        message=message,
        next_action="Review the package before approval.",
        document_ids=(DOCUMENT_ID,),
        claim_ids=((CLAIM_ID,) if document_reviewed else ()),
    )
    return PackageReviewFindingsV1(
        job_id=JOB_ID,
        input_fingerprint="5" * 64,
        parsed_job_sha256="6" * 64,
        application_brief_sha256="7" * 64,
        required_document_plan_sha256="8" * 64,
        document_execution_plan_sha256="9" * 64,
        reviewer_strategy="deterministic.package_consistency_review",
        reviewer_version="1.0.0",
        documents=(document,),
        required_document_ids=(DOCUMENT_ID,),
        selected_document_ids=(DOCUMENT_ID,),
        reviewed_document_ids=((DOCUMENT_ID,) if document_reviewed else ()),
        findings=(finding,),
        blocker_finding_ids=((finding_id,) if severity == "blocker" else ()),
    )


def _dispositions(review: PackageReviewFindingsV1) -> PackageReviewDispositionsV1:
    return PackageReviewDispositionsV1(
        job_id=JOB_ID,
        revision=1,
        updated_at=NOW,
        package_review_findings_sha256=REVIEW_SHA,
        dispositions=(
            FindingDispositionV1(
                finding_id=review.findings[0].finding_id,
                disposition="accepted",
                decided_at=NOW,
            ),
        ),
    )


def test_package_readiness_requires_current_complete_user_dispositions() -> None:
    review = _package_review()
    missing = derive_application_package_readiness(
        review,
        package_review_findings_sha256=REVIEW_SHA,
        dispositions=None,
        package_review_dispositions_sha256=None,
    )
    reviewed = derive_application_package_readiness(
        review,
        package_review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review),
        package_review_dispositions_sha256=DISPOSITIONS_SHA,
    )

    assert missing.state == "review_required"
    assert missing.reason_codes == ("package.dispositions_missing",)
    assert reviewed.state == "reviewed"
    assert reviewed.required_document_ids == (DOCUMENT_ID,)
    assert reviewed.reviewed_required_document_ids == (DOCUMENT_ID,)
    assert reviewed.package_review_dispositions_sha256 == DISPOSITIONS_SHA


def test_stale_dispositions_and_missing_required_documents_fail_closed() -> None:
    review = _package_review()
    stale_dispositions = _dispositions(review).model_copy(
        update={"package_review_findings_sha256": "0" * 64}
    )
    stale = derive_application_package_readiness(
        review,
        package_review_findings_sha256=REVIEW_SHA,
        dispositions=stale_dispositions,
        package_review_dispositions_sha256=DISPOSITIONS_SHA,
    )
    incomplete_review = _package_review(document_reviewed=False)
    incomplete = derive_application_package_readiness(
        incomplete_review,
        package_review_findings_sha256=REVIEW_SHA,
        dispositions=None,
        package_review_dispositions_sha256=None,
    )

    assert stale.state == "review_required"
    assert stale.package_review_dispositions_sha256 is None
    assert "package.dispositions_stale" in stale.reason_codes
    assert incomplete.state == "blocked"
    assert incomplete.reviewed_required_document_ids == ()
    assert "package.required_document_not_reviewed" in incomplete.reason_codes


def test_package_blockers_cannot_be_accepted_or_waived_by_derivation() -> None:
    review = _package_review(severity="blocker")
    readiness = derive_application_package_readiness(
        review,
        package_review_findings_sha256=REVIEW_SHA,
        dispositions=_dispositions(review),
        package_review_dispositions_sha256=DISPOSITIONS_SHA,
    )

    assert readiness.state == "blocked"
    assert readiness.accepted_finding_ids == ()
    assert readiness.blocker_finding_ids == (review.findings[0].finding_id,)
    assert "package.blocker_nonwaivable" in readiness.reason_codes


def test_package_readiness_models_and_static_schemas_are_strict() -> None:
    review = _package_review()
    dispositions = _dispositions(review)
    readiness = derive_application_package_readiness(
        review,
        package_review_findings_sha256=REVIEW_SHA,
        dispositions=dispositions,
        package_review_dispositions_sha256=DISPOSITIONS_SHA,
    )

    disposition_schema = json.loads(
        Path("schemas/package-review-dispositions.schema.json").read_text(
            encoding="utf-8"
        )
    )
    readiness_schema = json.loads(
        Path("schemas/application-package-readiness.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(disposition_schema)
    Draft202012Validator.check_schema(readiness_schema)
    assert disposition_schema == PackageReviewDispositionsV1.model_json_schema(
        mode="validation"
    )
    assert readiness_schema == ApplicationPackageReadinessV1.model_json_schema(
        mode="validation"
    )
    Draft202012Validator(disposition_schema).validate(
        dispositions.model_dump(mode="json")
    )
    Draft202012Validator(readiness_schema).validate(readiness.model_dump(mode="json"))

    forged = readiness.model_dump(mode="json")
    forged["package_review_dispositions_sha256"] = None
    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(readiness_schema).validate(forged)
    with pytest.raises(ValidationError):
        ApplicationPackageReadinessV1.model_validate(forged)
