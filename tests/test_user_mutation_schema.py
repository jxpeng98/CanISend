from __future__ import annotations

from datetime import UTC, datetime
import json
from pathlib import Path

from jsonschema import Draft202012Validator
import pytest
from pydantic import ValidationError

from canisend.user_mutations import (
    ConfirmCriterionPatch,
    USER_MUTATION_SCHEMA_VERSION,
    UserMutationClaimV1,
    UserMutationReceiptV1,
)
from canisend.review_readiness import ReviewDispositionsV1
from canisend.decision_models import (
    ApplicationBriefV1,
    ApplicationDecisionV1,
    ConfirmedCorrectionsV1,
    MAX_USER_REVISION,
)


SHA_A = "a" * 64
SHA_B = "b" * 64
MUTATION_ID = "mutation_" + "1" * 32
COMMITTED_AT = datetime(2026, 7, 11, 12, 0, tzinfo=UTC)
SCHEMA_PATH = Path("schemas/user-mutation-receipt.schema.json")


def _receipt() -> UserMutationReceiptV1:
    return UserMutationReceiptV1(
        mutation_id=MUTATION_ID,
        job_id="example-role",
        artifact="decision",
        target_path="application_decision.yaml",
        expected_revision=2,
        expected_sha256=SHA_A,
        result_revision=3,
        result_sha256=SHA_B,
        committed_at=COMMITTED_AT,
    )


def test_user_mutation_receipt_schema_is_strict_v1_and_accepts_model_dump() -> None:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))

    Draft202012Validator.check_schema(schema)
    assert schema["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert schema["$id"] == (
        "https://github.com/jxpeng98/CanISend/schemas/"
        "user-mutation-receipt.schema.json"
    )
    assert schema["additionalProperties"] is False
    assert USER_MUTATION_SCHEMA_VERSION == "1.0.0"
    Draft202012Validator(schema).validate(_receipt().model_dump(mode="json"))


def test_generated_user_mutation_receipt_schema_matches_model_contract() -> None:
    generated = UserMutationReceiptV1.model_json_schema(mode="validation")
    stored = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))

    assert stored == generated


def test_receipt_schema_rejects_public_transition_and_path_mismatches() -> None:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    validator = Draft202012Validator(schema)
    valid = _receipt().model_dump(mode="json")

    assert list(
        validator.iter_errors(
            {**valid, "artifact": "corrections", "target_path": "application_decision.yaml"}
        )
    )
    brief = {
        **valid,
        "artifact": "brief",
        "target_path": "application_brief.yaml",
    }
    validator.validate(brief)
    review_dispositions = {
        **valid,
        "artifact": "review_dispositions",
        "target_path": "review_dispositions.yaml",
    }
    validator.validate(review_dispositions)
    assert list(
        validator.iter_errors({**brief, "target_path": "application_decision.yaml"})
    )
    assert list(
        validator.iter_errors(
            {
                **valid,
                "expected_revision": None,
                "expected_sha256": None,
                "result_revision": 1,
            }
        )
    )


def test_receipt_model_rejects_non_sequential_transition() -> None:
    with pytest.raises(ValidationError):
        UserMutationReceiptV1.model_validate(
            {**_receipt().model_dump(mode="json"), "result_revision": 7}
        )


def test_private_mutation_control_schemas_are_not_published() -> None:
    assert not Path("schemas/user-mutation-claim.schema.json").exists()
    assert not Path("schemas/user-mutation-candidate.schema.json").exists()


def _claim() -> UserMutationClaimV1:
    return UserMutationClaimV1(
        mutation_id=MUTATION_ID,
        job_id="example-role",
        artifact="decision",
        target_path="application_decision.yaml",
        expected_revision=2,
        expected_sha256=SHA_A,
        result_revision=3,
        result_sha256=SHA_B,
        candidate_path=f"workflow/user-mutations/events/{MUTATION_ID}/candidate.yaml",
        candidate_sha256=SHA_B,
        claimed_at=COMMITTED_AT,
        consent_confirmed=True,
    )


@pytest.mark.parametrize("value", [True, 2.0, "2"])
def test_mutation_control_revisions_are_strict_integers(value: object) -> None:
    receipt = _receipt().model_dump(mode="json")
    claim = _claim().model_dump(mode="json")
    for field in ("expected_revision", "result_revision"):
        with pytest.raises(ValidationError):
            UserMutationReceiptV1.model_validate({**receipt, field: value})
        with pytest.raises(ValidationError):
            UserMutationClaimV1.model_validate({**claim, field: value})
    for model in (ConfirmedCorrectionsV1, ApplicationDecisionV1, ApplicationBriefV1):
        with pytest.raises(ValidationError):
            model.model_validate(
                {
                    "job_id": "example-role",
                    "revision": value,
                    "updated_at": "2026-07-11T12:00:00Z",
                }
            )


def test_user_revision_maximum_matches_runtime_and_static_schemas() -> None:
    near_limit = {
        **_receipt().model_dump(mode="json"),
        "expected_revision": MAX_USER_REVISION - 1,
        "result_revision": MAX_USER_REVISION,
    }
    assert UserMutationReceiptV1.model_validate(near_limit).result_revision == MAX_USER_REVISION
    with pytest.raises(ValidationError):
        UserMutationReceiptV1.model_validate(
            {**near_limit, "result_revision": MAX_USER_REVISION + 1}
        )

    for model in (ConfirmedCorrectionsV1, ApplicationDecisionV1, ApplicationBriefV1):
        payload = {
            "job_id": "example-role",
            "revision": MAX_USER_REVISION,
            "updated_at": "2026-07-11T12:00:00Z",
        }
        assert model.model_validate(payload).revision == MAX_USER_REVISION
        with pytest.raises(ValidationError):
            model.model_validate({**payload, "revision": MAX_USER_REVISION + 1})

    receipt_schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    assert receipt_schema["properties"]["result_revision"]["maximum"] == MAX_USER_REVISION
    for path in (
        Path("schemas/confirmed-corrections.schema.json"),
        Path("schemas/application-decision.schema.json"),
        Path("schemas/application-brief.schema.json"),
    ):
        schema = json.loads(path.read_text(encoding="utf-8"))
        assert schema["properties"]["revision"]["maximum"] == MAX_USER_REVISION

    review_dispositions = ReviewDispositionsV1(
        job_id="example-role",
        document_id="document_" + "d" * 32,
        revision=MAX_USER_REVISION,
        updated_at=COMMITTED_AT,
        draft_sha256=SHA_A,
        review_findings_sha256=SHA_B,
    )
    assert review_dispositions.revision == MAX_USER_REVISION
    with pytest.raises(ValidationError):
        ReviewDispositionsV1.model_validate(
            {
                **review_dispositions.model_dump(mode="json"),
                "revision": MAX_USER_REVISION + 1,
            }
        )


@pytest.mark.parametrize(
    "value",
    [1, 1.0, True, "1", "1.0", "1700000000", "1e3", "2026-07-11 12:00:00Z"],
)
def test_control_timestamps_reject_numeric_or_non_rfc3339_inputs(value: object) -> None:
    with pytest.raises(ValidationError):
        UserMutationReceiptV1.model_validate(
            {**_receipt().model_dump(mode="json"), "committed_at": value}
        )
    with pytest.raises(ValidationError):
        UserMutationClaimV1.model_validate(
            {**_claim().model_dump(mode="json"), "claimed_at": value}
        )
    with pytest.raises(ValidationError):
        ApplicationDecisionV1.model_validate(
            {
                "job_id": "example-role",
                "revision": 0,
                "updated_at": value,
            }
        )
    with pytest.raises(ValidationError):
        ConfirmedCorrectionsV1.model_validate(
            {
                "job_id": "example-role",
                "revision": 0,
                "updated_at": value,
            }
        )
    with pytest.raises(ValidationError):
        ApplicationBriefV1.model_validate(
            {
                "job_id": "example-role",
                "revision": 0,
                "updated_at": value,
            }
        )


@pytest.mark.parametrize("value", [1, "true", False])
def test_claim_consent_accepts_only_real_true(value: object) -> None:
    with pytest.raises(ValidationError):
        UserMutationClaimV1.model_validate(
            {**_claim().model_dump(mode="json"), "consent_confirmed": value}
        )


@pytest.mark.parametrize("value", [True, 1.0, "1"])
def test_patch_source_occurrence_is_a_strict_integer(value: object) -> None:
    with pytest.raises(ValidationError):
        ConfirmCriterionPatch.model_validate(
            {
                "criterion_id": "criterion_" + "1" * 32,
                "source_occurrence": value,
            }
        )
