from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, ValidationError as JsonSchemaValidationError

from canisend.draft_models import CoverLetterDraftV1, ReviewFindingsV1
from tests.test_draft_models import draft, finding


def schema(name: str) -> dict[str, object]:
    return json.loads((Path("schemas") / name).read_text(encoding="utf-8"))


@pytest.mark.parametrize(
    ("name", "model_class", "payload"),
    [
        (
            "cover-letter-draft.schema.json",
            CoverLetterDraftV1,
            lambda: draft().model_dump(mode="json"),
        ),
        (
            "review-findings.schema.json",
            ReviewFindingsV1,
            lambda: ReviewFindingsV1(
                job_id="lecturer-economics",
                document_id="document_" + "d" * 32,
                input_fingerprint="2" * 64,
                draft_sha256="3" * 64,
                reviewer_strategy="deterministic.cover_letter_review",
                reviewer_version="1.0.0",
                findings=(finding(),),
                blocker_finding_ids=(finding().finding_id,),
            ).model_dump(mode="json"),
        ),
    ],
)
def test_static_draft_schema_matches_runtime_contract_and_accepts_dump(
    name: str,
    model_class: type,
    payload: object,
) -> None:
    loaded = schema(name)

    Draft202012Validator.check_schema(loaded)
    assert loaded == model_class.model_json_schema(mode="validation")
    assert loaded["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert loaded["additionalProperties"] is False
    Draft202012Validator(loaded).validate(payload())


@pytest.mark.parametrize(
    ("support_strength", "evidence_ref_ids", "blockers"),
    [
        ("strong", [], []),
        ("partial", ["evidence_" + "e" * 32], []),
        ("unsupported", [], []),
        ("unsupported", ["evidence_" + "e" * 32], ["claim.unsupported"]),
    ],
)
def test_standalone_schema_rejects_invalid_factual_support(
    support_strength: str,
    evidence_ref_ids: list[str],
    blockers: list[str],
) -> None:
    payload = draft().model_dump(mode="json")
    claim_payload = payload["sections"][0]["claims"][0]
    claim_payload["support_strength"] = support_strength
    claim_payload["evidence_ref_ids"] = evidence_ref_ids
    claim_payload["blockers"] = blockers
    payload["blockers"] = blockers

    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(schema("cover-letter-draft.schema.json")).validate(payload)


@pytest.mark.parametrize(
    ("kind", "brief_field_refs", "criterion_ids", "job_field_refs"),
    [
        ("motivation", [], [], []),
        ("future_intent", [], [], []),
        ("role_context", [], [], []),
        ("administrative", [], ["criterion_" + "c" * 32], []),
    ],
)
def test_standalone_schema_rejects_missing_or_forbidden_non_evidence_basis(
    kind: str,
    brief_field_refs: list[str],
    criterion_ids: list[str],
    job_field_refs: list[str],
) -> None:
    payload = draft().model_dump(mode="json")
    claim_payload = payload["sections"][0]["claims"][0]
    claim_payload.update(
        {
            "kind": kind,
            "support_strength": "not_applicable",
            "brief_field_refs": brief_field_refs,
            "criterion_ids": criterion_ids,
            "job_field_refs": job_field_refs,
            "evidence_ref_ids": [],
        }
    )

    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(schema("cover-letter-draft.schema.json")).validate(payload)


@pytest.mark.parametrize("field", ["prose", "ready", "final", "submission_ready"])
def test_schema_has_no_hidden_prose_or_readiness_escape_hatch(field: str) -> None:
    payload = draft().model_dump(mode="json")
    payload[field] = True

    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(schema("cover-letter-draft.schema.json")).validate(payload)


def test_review_schema_rejects_private_or_authoritative_escape_fields() -> None:
    item = finding()
    payload = ReviewFindingsV1(
        job_id="lecturer-economics",
        document_id="document_" + "d" * 32,
        input_fingerprint="2" * 64,
        draft_sha256="3" * 64,
        reviewer_strategy="deterministic.cover_letter_review",
        reviewer_version="1.0.0",
        findings=(item,),
        blocker_finding_ids=(item.finding_id,),
    ).model_dump(mode="json")
    payload["ready"] = True

    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(schema("review-findings.schema.json")).validate(payload)
