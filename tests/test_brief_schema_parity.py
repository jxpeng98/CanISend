from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import pytest

from canisend.decision_models import ApplicationBriefV1, RequiredDocumentPlanV1


DOCUMENT_ID = "document_" + "4" * 32
CRITERION_ID = "criterion_" + "1" * 32
EVIDENCE_ID = "evidence_" + "2" * 32
SHA_A = "a" * 64
SHA_B = "b" * 64
SHA_C = "c" * 64
NOW = "2026-07-11T12:00:00Z"


def _schema(name: str) -> dict[str, Any]:
    schema = json.loads((Path("schemas") / name).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    return schema


def _brief_payload() -> dict[str, Any]:
    return {
        "schema_version": "1.0.0",
        "job_id": "example-role",
        "revision": 1,
        "updated_at": NOW,
        "decision_sha256": SHA_A,
        "language": {"value": "uk", "confirmation_state": "confirmed"},
        "writing_style": {
            "value": "direct and evidence-led",
            "confirmation_state": "confirmed",
        },
        "motivation": {"value": "", "confirmation_state": "confirmed"},
        "emphasis": {
            "criterion_ids": [CRITERION_ID],
            "evidence_ref_ids": [EVIDENCE_ID],
            "confirmation_state": "confirmed",
        },
        "exclusions": {"items": [], "confirmation_state": "confirmed"},
        "document_requirements_confirmation": {
            "state": "confirmed",
            "basis_sha256": SHA_B,
            "confirmed_at": NOW,
        },
        "document_choices": [
            {
                "document_id": DOCUMENT_ID,
                "action": "prepare",
                "confirmation_state": "confirmed",
            }
        ],
    }


def _known_requirement() -> dict[str, Any]:
    return {
        "document_id": DOCUMENT_ID,
        "label": "Cover letter",
        "normalized_kind": "cover_letter",
        "requirement": "required",
        "source_text": "Required documents: CV, Cover letter",
        "source_state": "known",
        "source_span": {
            "path": "job_advert.md",
            "start_line": 5,
            "end_line": 5,
            "text_sha256": SHA_C,
            "anchor_sha256": SHA_B,
            "occurrence": 1,
            "occurrence_count": 1,
        },
        "confirmation_state": "confirmed",
        "unknown_reason": None,
    }


def _plan_payload() -> dict[str, Any]:
    return {
        "schema_version": "1.0.0",
        "job_id": "example-role",
        "input_fingerprint": SHA_A,
        "requirements_state": "confirmed",
        "requirements_basis_sha256": SHA_B,
        "requirements": [_known_requirement()],
        "tasks": [
            {
                "document_id": DOCUMENT_ID,
                "action": "prepare",
                "confirmation_state": "confirmed",
                "blockers": [],
            }
        ],
        "unresolved_brief_fields": [],
        "unresolved_document_ids": [],
        "blocking_document_ids": [],
        "orphaned_document_choice_ids": [],
        "blockers": [],
    }


def _unconfirmed_plan_payload() -> dict[str, Any]:
    payload = _plan_payload()
    payload["requirements_state"] = "unconfirmed"
    payload["requirements"][0]["confirmation_state"] = "unconfirmed"
    payload["tasks"][0] = {
        "document_id": DOCUMENT_ID,
        "action": "needs_confirmation",
        "confirmation_state": "unconfirmed",
        "blockers": ["documents.needs_confirmation"],
    }
    payload["unresolved_document_ids"] = [DOCUMENT_ID]
    payload["blocking_document_ids"] = [DOCUMENT_ID]
    payload["blockers"] = [
        "documents.needs_confirmation",
        "documents.requirements_unconfirmed",
    ]
    return payload


def _assert_brief_rejected(payload: dict[str, Any]) -> None:
    with pytest.raises(ValidationError):
        ApplicationBriefV1.model_validate(payload)
    assert list(
        Draft202012Validator(_schema("application-brief.schema.json")).iter_errors(payload)
    )


def _assert_plan_rejected(payload: dict[str, Any]) -> None:
    with pytest.raises(ValidationError):
        RequiredDocumentPlanV1.model_validate(payload)
    assert list(
        Draft202012Validator(
            _schema("required-document-plan.schema.json")
        ).iter_errors(payload)
    )


def test_application_brief_schema_accepts_the_model_contract() -> None:
    payload = _brief_payload()

    ApplicationBriefV1.model_validate(payload)
    Draft202012Validator(_schema("application-brief.schema.json")).validate(payload)


@pytest.mark.parametrize(
    "mutate",
    [
        lambda value: value.update(updated_at="2026-07-11"),
        lambda value: value["language"].update(value=None),
        lambda value: value["writing_style"].update(value=None),
        lambda value: value["emphasis"].update(criterion_ids=["criterion_invalid"]),
        lambda value: value["emphasis"].update(
            criterion_ids=[CRITERION_ID, CRITERION_ID]
        ),
        lambda value: value["emphasis"].update(evidence_ref_ids=["evidence_invalid"]),
        lambda value: value["exclusions"].update(items=["   "]),
        lambda value: value["exclusions"].update(items=["duplicate", "duplicate"]),
        lambda value: value["document_choices"][0].update(document_id="document_invalid"),
        lambda value: value["document_choices"][0].update(
            action="needs_confirmation", confirmation_state="confirmed"
        ),
        lambda value: value["document_choices"][0].update(
            action="prepare", confirmation_state="unconfirmed"
        ),
        lambda value: value.update(
            document_choices=[value["document_choices"][0]] * 2
        ),
        lambda value: value["document_requirements_confirmation"].update(
            confirmed_at="2026-07-11"
        ),
    ],
    ids=[
        "updated-at-rfc3339",
        "confirmed-language-value",
        "confirmed-text-value",
        "criterion-id-pattern",
        "criterion-id-unique",
        "evidence-id-pattern",
        "exclusion-non-empty",
        "exclusion-unique",
        "document-id-pattern",
        "needs-confirmation-state",
        "resolved-confirmation-state",
        "exact-document-choice-unique",
        "requirements-confirmed-at-rfc3339",
    ],
)
def test_application_brief_schema_rejects_nested_runtime_invariants(mutate: Any) -> None:
    payload = deepcopy(_brief_payload())
    mutate(payload)

    _assert_brief_rejected(payload)


def test_required_document_plan_schema_accepts_the_model_contract() -> None:
    payload = _plan_payload()

    RequiredDocumentPlanV1.model_validate(payload)
    Draft202012Validator(_schema("required-document-plan.schema.json")).validate(payload)


@pytest.mark.parametrize(
    "mutate",
    [
        lambda value: value["requirements"][0].update(document_id="document_invalid"),
        lambda value: value["requirements"][0].update(normalized_kind="Cover Letter"),
        lambda value: value["requirements"][0].update(label="   "),
        lambda value: value["requirements"][0].update(source_text=None),
        lambda value: value["requirements"][0]["source_span"].update(
            text_sha256="not-a-hash"
        ),
        lambda value: value["tasks"][0].update(document_id="document_invalid"),
        lambda value: value["tasks"][0].update(
            action="needs_confirmation", confirmation_state="confirmed", blockers=[]
        ),
        lambda value: value["tasks"][0].update(blockers=["documents.not-valid!"]),
        lambda value: value["tasks"][0].update(blockers=["documents.same"] * 2),
        lambda value: value.update(unresolved_document_ids=["document_invalid"]),
        lambda value: value.update(
            unresolved_brief_fields=["language", "language"]
        ),
        lambda value: value.update(requirements=value["requirements"] * 2),
        lambda value: value["requirements"][0].update(confirmation_state="unconfirmed"),
        lambda value: value.update(requirements_state="confirmed_empty"),
    ],
    ids=[
        "requirement-document-id-pattern",
        "normalized-kind-pattern",
        "label-non-whitespace",
        "known-source-text",
        "source-span-hash",
        "task-document-id-pattern",
        "task-action-state-and-blocker",
        "task-blocker-pattern",
        "task-blocker-unique",
        "plan-document-id-pattern",
        "brief-field-unique",
        "exact-requirement-unique",
        "confirmed-requirement-state",
        "confirmed-empty-collections",
    ],
)
def test_required_document_plan_schema_rejects_nested_runtime_invariants(
    mutate: Any,
) -> None:
    payload = deepcopy(_plan_payload())
    mutate(payload)

    _assert_plan_rejected(payload)


@pytest.mark.parametrize(
    "mutate",
    [
        lambda value: value["tasks"][0].update(
            action="prepare", confirmation_state="confirmed", blockers=[]
        ),
        lambda value: value.update(blockers=["documents.needs_confirmation"]),
        lambda value: value["requirements"][0].update(confirmation_state="confirmed"),
    ],
    ids=[
        "unconfirmed-task-must-stay-unresolved",
        "unconfirmed-plan-blocker-required",
        "unconfirmed-requirement-state",
    ],
)
def test_required_document_plan_schema_rejects_unconfirmed_state_invariants(
    mutate: Any,
) -> None:
    payload = deepcopy(_unconfirmed_plan_payload())
    mutate(payload)

    _assert_plan_rejected(payload)


def test_required_document_plan_schema_requires_unresolved_task_control_fields() -> None:
    payload = deepcopy(_unconfirmed_plan_payload())
    del payload["tasks"][0]["blockers"]

    _assert_plan_rejected(payload)


def test_required_document_plan_schema_requires_unconfirmed_plan_blockers_field() -> None:
    payload = deepcopy(_unconfirmed_plan_payload())
    del payload["blockers"]

    _assert_plan_rejected(payload)


@pytest.mark.parametrize(
    "field_name",
    ["unresolved_document_ids", "blocking_document_ids"],
)
def test_nonempty_unconfirmed_plan_schema_requires_document_control_id_fields(
    field_name: str,
) -> None:
    payload = deepcopy(_unconfirmed_plan_payload())
    del payload[field_name]

    _assert_plan_rejected(payload)


def test_empty_unconfirmed_plan_may_omit_document_control_id_fields() -> None:
    payload = deepcopy(_unconfirmed_plan_payload())
    payload["requirements"] = []
    payload["tasks"] = []
    payload["blockers"] = ["documents.requirements_unconfirmed"]
    del payload["unresolved_document_ids"]
    del payload["blocking_document_ids"]

    RequiredDocumentPlanV1.model_validate(payload)
    Draft202012Validator(_schema("required-document-plan.schema.json")).validate(payload)


def test_required_document_plan_schema_requires_known_sources_when_confirmed() -> None:
    payload = deepcopy(_plan_payload())
    payload["requirements"][0].update(
        source_text=None,
        source_state="unknown",
        source_span=None,
        unknown_reason="documents.source_unknown",
    )

    _assert_plan_rejected(payload)
