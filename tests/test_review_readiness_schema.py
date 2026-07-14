from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator, ValidationError as SchemaValidationError
import pytest

from canisend.review_readiness import (
    DocumentReadinessV1,
    ReviewDispositionsV1,
)
from tests.test_review_readiness import (
    DISPOSITIONS_SHA,
    DRAFT_SHA,
    REVIEW_SHA,
    _dispositions,
    _review,
)


def _schema(name: str) -> dict[str, object]:
    return json.loads((Path("schemas") / name).read_text(encoding="utf-8"))


def test_review_disposition_schema_matches_runtime_contract() -> None:
    review = _review()
    stored = _schema("review-dispositions.schema.json")

    Draft202012Validator.check_schema(stored)
    assert stored == ReviewDispositionsV1.model_json_schema(mode="validation")
    assert stored["additionalProperties"] is False
    Draft202012Validator(stored).validate(
        _dispositions(review).model_dump(mode="json")
    )
    research = _dispositions(review).model_copy(
        update={"document_kind": "research_statement"}
    )
    Draft202012Validator(stored).validate(research.model_dump(mode="json"))


def test_document_readiness_schema_matches_runtime_contract() -> None:
    review = _review()
    finding_id = review.findings[0].finding_id
    readiness = DocumentReadinessV1(
        job_id=review.job_id,
        document_id=review.document_id,
        state="reviewed",
        draft_sha256=DRAFT_SHA,
        review_findings_sha256=REVIEW_SHA,
        review_dispositions_sha256=DISPOSITIONS_SHA,
        accepted_finding_ids=(finding_id,),
    )
    stored = _schema("document-readiness.schema.json")

    Draft202012Validator.check_schema(stored)
    assert stored == DocumentReadinessV1.model_json_schema(mode="validation")
    assert stored["additionalProperties"] is False
    Draft202012Validator(stored).validate(readiness.model_dump(mode="json"))

    forged = readiness.model_dump(mode="json")
    forged["review_dispositions_sha256"] = None
    with pytest.raises(SchemaValidationError):
        Draft202012Validator(stored).validate(forged)

    research = readiness.model_copy(update={"document_kind": "research_statement"})
    Draft202012Validator(stored).validate(research.model_dump(mode="json"))
