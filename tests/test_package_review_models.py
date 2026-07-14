from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, ValidationError as JsonSchemaValidationError
from pydantic import ValidationError

from canisend.package_review_models import (
    PackageCorrectionProposalV1,
    PackageDocumentReviewV1,
    PackageReviewFindingV1,
    PackageReviewFindingsV1,
    stable_package_finding_id,
    stable_package_proposal_id,
)


JOB_ID = "lecturer-economics"
DOCUMENT_ID = "document_" + "d" * 32
CLAIM_ID = "claim_" + "c" * 32


def _reviewed_document() -> PackageDocumentReviewV1:
    return PackageDocumentReviewV1(
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


def _aggregate() -> PackageReviewFindingsV1:
    instruction = "Submit any revision through a new guarded Draft candidate."
    proposal = PackageCorrectionProposalV1(
        proposal_id=stable_package_proposal_id(
            job_id=JOB_ID,
            document_id=DOCUMENT_ID,
            claim_ids=(CLAIM_ID,),
            reason_code="package.receipt_conflict",
            instruction=instruction,
        ),
        document_id=DOCUMENT_ID,
        claim_ids=(CLAIM_ID,),
        reason_code="package.receipt_conflict",
        instruction=instruction,
    )
    message = "One exact receipt conflict requires review."
    finding = PackageReviewFindingV1(
        finding_id=stable_package_finding_id(
            job_id=JOB_ID,
            code="package.receipt_conflict",
            message=message,
            document_ids=(DOCUMENT_ID,),
            claim_ids=(CLAIM_ID,),
            correction_proposal_ids=(proposal.proposal_id,),
        ),
        code="package.receipt_conflict",
        severity="blocker",
        category="contradiction",
        message=message,
        next_action="Verify the bound receipt.",
        document_ids=(DOCUMENT_ID,),
        claim_ids=(CLAIM_ID,),
        correction_proposal_ids=(proposal.proposal_id,),
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
        documents=(_reviewed_document(),),
        required_document_ids=(DOCUMENT_ID,),
        selected_document_ids=(DOCUMENT_ID,),
        reviewed_document_ids=(DOCUMENT_ID,),
        findings=(finding,),
        correction_proposals=(proposal,),
        blocker_finding_ids=(finding.finding_id,),
    )


def test_package_review_model_binds_exact_documents_findings_and_proposals() -> None:
    aggregate = _aggregate()

    assert aggregate.reviewed_document_ids == (DOCUMENT_ID,)
    assert aggregate.correction_proposals[0].application_route == (
        "guarded_draft_candidate"
    )
    assert aggregate.blocker_finding_ids == (aggregate.findings[0].finding_id,)


def test_package_review_model_rejects_unbound_claim_and_authoritative_escape() -> None:
    payload = _aggregate().model_dump(mode="json")
    payload["findings"][0]["claim_ids"] = ["claim_" + "a" * 32]

    with pytest.raises(ValidationError, match="Claim references must resolve"):
        PackageReviewFindingsV1.model_validate(payload)

    payload = _aggregate().model_dump(mode="json")
    payload["submission_ready"] = True
    with pytest.raises(ValidationError):
        PackageReviewFindingsV1.model_validate(payload)


def test_package_document_state_cannot_claim_false_reviewed_receipts() -> None:
    payload = _reviewed_document().model_dump(mode="json")
    payload["review_dispositions_sha256"] = None

    with pytest.raises(ValidationError, match="require dispositions"):
        PackageDocumentReviewV1.model_validate(payload)


def test_package_review_static_schema_matches_runtime_contract() -> None:
    stored = json.loads(
        Path("schemas/package-review-findings.schema.json").read_text(
            encoding="utf-8"
        )
    )

    Draft202012Validator.check_schema(stored)
    assert stored == PackageReviewFindingsV1.model_json_schema(mode="validation")
    assert stored["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert stored["additionalProperties"] is False
    Draft202012Validator(stored).validate(_aggregate().model_dump(mode="json"))

    invalid = _aggregate().model_dump(mode="json")
    invalid["ready"] = True
    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(stored).validate(invalid)

    invalid = _aggregate().model_dump(mode="json")
    invalid["documents"][0]["review_dispositions_sha256"] = None
    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(stored).validate(invalid)
