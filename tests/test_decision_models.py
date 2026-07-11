from __future__ import annotations

from datetime import UTC, datetime
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import pytest

from canisend.decision_models import (
    APPLICATION_BRIEF_SCHEMA_VERSION,
    APPLICATION_DECISION_SCHEMA_VERSION,
    CONFIRMED_CORRECTIONS_SCHEMA_VERSION,
    CRITERIA_SCHEMA_VERSION,
    CRITERION_MATCHES_SCHEMA_VERSION,
    EVIDENCE_CATALOG_SCHEMA_VERSION,
    REQUIRED_DOCUMENT_PLAN_SCHEMA_VERSION,
    ApplicationBriefV1,
    ApplicationDecisionV1,
    ConfirmedCorrectionsV1,
    ConfirmedIdSelectionV1,
    ConfirmedStringListV1,
    ConfirmedTextV1,
    CriteriaExtractionConfirmationV1,
    CriteriaCatalogV1,
    CriterionCorrectionV1,
    CriterionMatchV1,
    CriterionMatchesV1,
    CriterionV1,
    DecisionBasisV1,
    DocumentChoiceV1,
    DocumentRequirementV1,
    DocumentTaskV1,
    EvidenceGapV1,
    EvidenceCatalogItemV1,
    EvidenceCatalogV1,
    EvidenceRefV1,
    EvidenceSourceReceiptV1,
    LanguagePreferenceV1,
    RequiredDocumentPlanV1,
    SemanticInputReceiptV1,
    SourceSpanV1,
)


NOW = datetime(2026, 7, 11, 12, 0, tzinfo=UTC)
SHA_A = "a" * 64
SHA_B = "b" * 64
SHA_C = "c" * 64
JOB_ID = "example-role"
CRITERION_ID = "criterion_" + "1" * 32
EVIDENCE_ID = "evidence_" + "2" * 32
CORRECTION_ID = "correction_" + "3" * 32
DOCUMENT_ID = "document_" + "4" * 32


def source_span() -> SourceSpanV1:
    return SourceSpanV1(
        path="job_advert.md",
        start_line=8,
        end_line=8,
        text_sha256=SHA_A,
        anchor_sha256=SHA_B,
        occurrence=1,
        occurrence_count=1,
    )


def criterion() -> CriterionV1:
    return CriterionV1(
        criterion_id=CRITERION_ID,
        importance="essential",
        text="PhD in Economics",
        parsed_text_sha256=SHA_C,
        source_text="PhD in Economics",
        source_state="known",
        source_span=source_span(),
        confidence="high",
        confirmation_state="unconfirmed",
    )


def criteria_catalog() -> CriteriaCatalogV1:
    return CriteriaCatalogV1(
        job_id=JOB_ID,
        input_fingerprint=SHA_B,
        semantic_inputs=(
            SemanticInputReceiptV1(path="parsed_job.json", projection_sha256=SHA_A),
            SemanticInputReceiptV1(path="job_advert.md", projection_sha256=SHA_B),
        ),
        extraction_state="extracted",
        criteria=(criterion(),),
        unresolved_criterion_ids=(CRITERION_ID,),
    )


def evidence_ref() -> EvidenceRefV1:
    return EvidenceRefV1(
        evidence_id=EVIDENCE_ID,
        path="profile/generated/cv.evidence.md",
        section="Education",
        item_locator="cv-001",
        kind="education",
        content_sha256=SHA_C,
    )


def evidence_catalog() -> EvidenceCatalogV1:
    return EvidenceCatalogV1(
        job_id=JOB_ID,
        input_fingerprint=SHA_A,
        state="available",
        source_receipts=(
            EvidenceSourceReceiptV1(
                path="profile/profile.yaml",
                source_type="manifest",
                content_sha256=SHA_A,
                size_bytes=100,
                item_count=0,
            ),
            EvidenceSourceReceiptV1(
                path="profile/generated/cv.evidence.md",
                source_type="generated_evidence",
                content_sha256=SHA_B,
                size_bytes=200,
                item_count=1,
            ),
        ),
        items=(
            EvidenceCatalogItemV1(
                evidence_id=EVIDENCE_ID,
                path="profile/generated/cv.evidence.md",
                section="Education",
                item_locator="cv-001",
                kind="education",
                text="PhD in Economics",
                content_sha256=SHA_C,
            ),
        ),
    )


def criterion_matches() -> CriterionMatchesV1:
    return CriterionMatchesV1(
        job_id=JOB_ID,
        input_fingerprint=SHA_A,
        criteria_catalog_sha256=SHA_B,
        evidence_catalog_sha256=SHA_C,
        matcher_strategy="deterministic.keyword",
        matcher_version="1.0.0",
        evidence_refs=(evidence_ref(),),
        matches=(
            CriterionMatchV1(
                criterion_id=CRITERION_ID,
                classification="partial",
                evidence_ref_ids=(EVIDENCE_ID,),
                gaps=(
                    EvidenceGapV1(
                        code="evidence.more_detail_needed",
                        message="Direct evidence needs more detail.",
                        next_action="Review the linked evidence and add context.",
                    ),
                ),
                review_state="proposed",
            ),
        ),
    )


def application_decision() -> ApplicationDecisionV1:
    return ApplicationDecisionV1(
        job_id=JOB_ID,
        revision=1,
        updated_at=NOW,
        decision="apply",
        confirmation_state="confirmed",
        confirmed_at=NOW,
        rationale="The role is a strong fit.",
        basis=DecisionBasisV1(
            criteria_sha256=SHA_A,
            matches_sha256=SHA_B,
            status="current",
        ),
    )


def application_brief() -> ApplicationBriefV1:
    return ApplicationBriefV1(
        job_id=JOB_ID,
        revision=0,
        updated_at=NOW,
        decision_sha256=SHA_A,
        language=LanguagePreferenceV1(value="uk", confirmation_state="confirmed"),
        writing_style=ConfirmedTextV1(value="direct and evidence-led", confirmation_state="confirmed"),
        motivation=ConfirmedTextV1(value="", confirmation_state="confirmed"),
        emphasis=ConfirmedIdSelectionV1(
            criterion_ids=(CRITERION_ID,),
            evidence_ref_ids=(EVIDENCE_ID,),
            confirmation_state="confirmed",
        ),
        exclusions=ConfirmedStringListV1(items=(), confirmation_state="confirmed"),
        document_choices=(
            DocumentChoiceV1(
                document_id=DOCUMENT_ID,
                action="prepare",
                confirmation_state="confirmed",
            ),
        ),
    )


def document_plan() -> RequiredDocumentPlanV1:
    requirement = DocumentRequirementV1(
        document_id=DOCUMENT_ID,
        label="Cover letter",
        normalized_kind="cover_letter",
        requirement="required",
        source_text="Required documents: CV, Cover letter",
        source_state="known",
        source_span=SourceSpanV1(
            path="job_advert.md",
            start_line=5,
            end_line=5,
            text_sha256=SHA_C,
            anchor_sha256=SHA_B,
            occurrence=1,
            occurrence_count=1,
        ),
        confirmation_state="unconfirmed",
    )
    task = DocumentTaskV1(
        document_id=DOCUMENT_ID,
        action="prepare",
        confirmation_state="confirmed",
        blockers=(),
    )
    return RequiredDocumentPlanV1(
        job_id=JOB_ID,
        input_fingerprint=SHA_A,
        requirements=(requirement,),
        tasks=(task,),
        unresolved_document_ids=(),
    )


@pytest.mark.parametrize(
    "unsafe_path",
    ["", ".", "../job_advert.md", "/tmp/job_advert.md", r"C:\\job_advert.md", "a//b"],
)
def test_source_span_rejects_unsafe_paths(unsafe_path: str) -> None:
    with pytest.raises(ValidationError):
        SourceSpanV1(
            path=unsafe_path,
            start_line=1,
            end_line=1,
            text_sha256=SHA_A,
            anchor_sha256=SHA_B,
            occurrence=1,
            occurrence_count=1,
        )


def test_source_span_requires_ordered_lines_and_valid_occurrence() -> None:
    with pytest.raises(ValidationError):
        SourceSpanV1.model_validate(
            {**source_span().model_dump(), "start_line": 9, "end_line": 8}
        )
    with pytest.raises(ValidationError):
        SourceSpanV1(
            path="job_advert.md",
            start_line=1,
            end_line=1,
            text_sha256=SHA_A,
            occurrence=2,
            occurrence_count=1,
        )


def test_criterion_separates_source_unknown_from_user_confirmation() -> None:
    unresolved = CriterionV1(
        criterion_id=CRITERION_ID,
        importance="essential",
        text="Requirement needs review",
        parsed_text_sha256=SHA_C,
        source_text="Requirement needs review",
        source_state="unknown",
        source_span=None,
        source_candidates=(
            source_span().model_copy(update={"occurrence_count": 2}),
            source_span().model_copy(
                update={
                    "start_line": 12,
                    "end_line": 12,
                    "occurrence": 2,
                    "occurrence_count": 2,
                    "anchor_sha256": SHA_C,
                }
            ),
        ),
        confidence="unknown",
        confirmation_state="confirmed",
        confirmation_record_id=CORRECTION_ID,
        unknown_reason="source_receipt.ambiguous",
    )

    assert unresolved.source_state == "unknown"
    assert unresolved.confirmation_state == "confirmed"

    with pytest.raises(ValidationError):
        CriterionV1.model_validate(
            {
                **criterion().model_dump(),
                "source_state": "known",
                "source_span": None,
            }
        )
    with pytest.raises(ValidationError):
        CriterionV1.model_validate(
            {
                **criterion().model_dump(),
                "source_state": "unknown",
                "source_span": None,
                "confidence": "unknown",
                "unknown_reason": None,
            }
        )


def test_criteria_catalog_rejects_duplicate_ids_and_inconsistent_unresolved_ids() -> None:
    assert criteria_catalog().schema_version == CRITERIA_SCHEMA_VERSION

    with pytest.raises(ValidationError):
        CriteriaCatalogV1.model_validate(
            {
                **criteria_catalog().model_dump(),
                "criteria": [criterion().model_dump(), criterion().model_dump()],
            }
        )
    with pytest.raises(ValidationError):
        CriteriaCatalogV1.model_validate(
            {
                **criteria_catalog().model_dump(),
                "unresolved_criterion_ids": ["criterion_" + "9" * 32],
            }
        )


def test_confirmed_corrections_require_consistent_active_records() -> None:
    corrected = CriterionCorrectionV1(
        correction_id=CORRECTION_ID,
        criterion_id=CRITERION_ID,
        target_source_sha256=SHA_A,
        target_criterion_sha256=SHA_B,
        confirmation="corrected",
        corrected_text="Doctorate in Economics or a related discipline",
        source_occurrence=1,
        source_anchor_sha256=SHA_C,
        record_state="active",
        confirmed_at=NOW,
    )
    overlay = ConfirmedCorrectionsV1(
        job_id=JOB_ID,
        revision=1,
        updated_at=NOW,
        criteria=(corrected,),
    )

    assert overlay.schema_version == CONFIRMED_CORRECTIONS_SCHEMA_VERSION

    with pytest.raises(ValidationError):
        CriterionCorrectionV1.model_validate(
            {**corrected.model_dump(), "confirmation": "corrected", "corrected_text": None}
        )
    with pytest.raises(ValidationError):
        ConfirmedCorrectionsV1(
            job_id=JOB_ID,
            revision=2,
            updated_at=NOW,
            criteria=(
                corrected,
                corrected.model_copy(update={"correction_id": "correction_" + "4" * 32}),
            ),
        )


def test_confirmed_correction_history_may_terminate_in_withdrawn_record() -> None:
    withdrawn_id = "correction_" + "4" * 32
    history = CriterionCorrectionV1(
        correction_id=CORRECTION_ID,
        criterion_id=CRITERION_ID,
        target_source_sha256=SHA_A,
        target_criterion_sha256=SHA_B,
        confirmation="confirmed",
        record_state="superseded",
        superseded_by=withdrawn_id,
        confirmed_at=NOW,
    )
    withdrawn = CriterionCorrectionV1(
        correction_id=withdrawn_id,
        criterion_id=CRITERION_ID,
        target_source_sha256=SHA_A,
        target_criterion_sha256=SHA_B,
        confirmation="corrected",
        corrected_text="Reviewed wording",
        record_state="withdrawn",
        confirmed_at=NOW,
    )

    overlay = ConfirmedCorrectionsV1(
        job_id=JOB_ID,
        revision=3,
        updated_at=NOW,
        criteria=(history, withdrawn),
    )

    assert tuple(item.record_state for item in overlay.criteria) == (
        "superseded",
        "withdrawn",
    )


def test_confirmed_corrections_preserve_one_active_empty_extraction_history() -> None:
    active = CriteriaExtractionConfirmationV1(
        correction_id=CORRECTION_ID,
        target_extraction_sha256=SHA_A,
        confirmation="confirmed_empty",
        confirmed_at=NOW,
    )
    superseded_id = "correction_" + "5" * 32
    history = active.model_copy(
        update={
            "correction_id": superseded_id,
            "record_state": "superseded",
            "superseded_by": CORRECTION_ID,
        }
    )

    overlay = ConfirmedCorrectionsV1(
        job_id=JOB_ID,
        revision=2,
        updated_at=NOW,
        criteria_extraction_confirmations=(history, active),
    )

    assert overlay.criteria_extraction_confirmations[-1].confirmation == "confirmed_empty"
    schema = json.loads(
        (Path("schemas") / "confirmed-corrections.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator(schema).validate(overlay.model_dump(mode="json"))
    with pytest.raises(ValidationError, match="only one.*active"):
        ConfirmedCorrectionsV1(
            job_id=JOB_ID,
            revision=3,
            updated_at=NOW,
            criteria_extraction_confirmations=(
                active,
                active.model_copy(update={"correction_id": "correction_" + "6" * 32}),
            ),
        )
    with pytest.raises(ValidationError, match="correction IDs"):
        ConfirmedCorrectionsV1(
            job_id=JOB_ID,
            revision=3,
            updated_at=NOW,
            criteria=(
                CriterionCorrectionV1(
                    correction_id=CORRECTION_ID,
                    criterion_id=CRITERION_ID,
                    target_source_sha256=SHA_A,
                    target_criterion_sha256=SHA_B,
                    confirmation="confirmed",
                    confirmed_at=NOW,
                ),
            ),
            criteria_extraction_confirmations=(active,),
        )


def test_evidence_reference_is_locator_only_and_rejects_unsafe_paths() -> None:
    reference = evidence_ref()

    assert reference.citation == "profile/generated/cv.evidence.md#Education/cv-001"
    assert "text" not in reference.model_dump()

    with pytest.raises(ValidationError):
        EvidenceRefV1.model_validate({**reference.model_dump(), "path": "../profile/cv.md"})
    with pytest.raises(ValidationError):
        EvidenceRefV1.model_validate({**reference.model_dump(), "text": "private evidence body"})


def test_evidence_catalog_keeps_bodies_separate_from_locator_only_references() -> None:
    catalog = evidence_catalog()
    item = catalog.items[0]

    assert item.text == "PhD in Economics"
    assert item.reference == evidence_ref()
    assert "text" not in item.reference.model_dump()


def test_evidence_catalog_requires_state_and_receipt_consistency() -> None:
    available = evidence_catalog()
    manifest = available.source_receipts[0]
    generated = available.source_receipts[1]

    empty = EvidenceCatalogV1(
        job_id=JOB_ID,
        input_fingerprint=SHA_A,
        state="empty",
        source_receipts=(generated.model_copy(update={"item_count": 0}),),
        items=(),
    )
    unavailable = EvidenceCatalogV1(
        job_id=JOB_ID,
        input_fingerprint=SHA_A,
        state="unavailable",
        unavailable_reason="evidence.generated_missing",
        source_receipts=(manifest,),
        items=(),
    )

    assert empty.state == "empty"
    assert unavailable.state == "unavailable"
    with pytest.raises(ValidationError):
        EvidenceCatalogV1.model_validate(
            {**available.model_dump(), "state": "unavailable", "unavailable_reason": None}
        )
    with pytest.raises(ValidationError):
        EvidenceCatalogV1.model_validate(
            {
                **available.model_dump(),
                "source_receipts": list(reversed(available.source_receipts)),
            }
        )
    with pytest.raises(ValidationError):
        EvidenceCatalogV1.model_validate(
            {
                **available.model_dump(),
                "items": [available.items[0], available.items[0]],
            }
        )


def test_criterion_match_requires_explicit_gaps_and_resolvable_evidence_ids() -> None:
    missing = CriterionMatchV1(
        criterion_id=CRITERION_ID,
        classification="missing",
        evidence_ref_ids=(),
        gaps=(
            EvidenceGapV1(
                code="evidence.no_direct_support",
                message="No direct evidence is linked.",
                next_action="Add or confirm relevant profile evidence.",
            ),
        ),
        review_state="proposed",
    )
    assert missing.classification == "missing"

    with pytest.raises(ValidationError):
        CriterionMatchV1(
            criterion_id=CRITERION_ID,
            classification="missing",
            evidence_ref_ids=(),
            gaps=(),
            review_state="proposed",
        )
    with pytest.raises(ValidationError):
        CriterionMatchesV1.model_validate(
            {
                **criterion_matches().model_dump(),
                "matches": [
                    {
                        **criterion_matches().matches[0].model_dump(),
                        "evidence_ref_ids": ["evidence_" + "9" * 32],
                    }
                ],
            }
        )


def test_decision_requires_explicit_confirmation_and_basis() -> None:
    decision = application_decision()

    assert decision.schema_version == APPLICATION_DECISION_SCHEMA_VERSION

    with pytest.raises(ValidationError):
        ApplicationDecisionV1.model_validate(
            {**decision.model_dump(), "confirmation_state": "unconfirmed", "confirmed_at": None}
        )
    undecided = ApplicationDecisionV1(
        job_id=JOB_ID,
        revision=0,
        updated_at=NOW,
        decision="undecided",
        confirmation_state="unconfirmed",
    )
    assert undecided.basis is None
    with pytest.raises(ValidationError):
        ApplicationDecisionV1.model_validate(
            {
                **undecided.model_dump(mode="json"),
                "rationale": "Do not persist an unresolved rationale",
            }
        )


@pytest.mark.parametrize(
    ("schema_name", "payload"),
    [
        (
            "criteria.schema.json",
            {
                **criteria_catalog().model_dump(mode="json"),
                "input_fingerprint": "not-a-hash",
            },
        ),
        (
            "criteria.schema.json",
            {
                **criteria_catalog().model_dump(mode="json"),
                "criteria": [
                    {
                        **criterion().model_dump(mode="json"),
                        "criterion_id": "criterion_invalid",
                    }
                ],
                "unresolved_criterion_ids": ["criterion_invalid"],
            },
        ),
        (
            "criteria.schema.json",
            {
                **criteria_catalog().model_dump(mode="json"),
                "extraction_state": "confirmed_empty",
                "criteria": [],
                "unresolved_criterion_ids": [],
                "empty_confirmation_record_id": None,
            },
        ),
        (
            "confirmed-corrections.schema.json",
            {
                "schema_version": "1.0.0",
                "job_id": JOB_ID,
                "revision": 1,
                "updated_at": NOW.isoformat(),
                "criteria": [
                    {
                        "correction_id": "bad",
                        "criterion_id": "bad",
                        "target_source_sha256": "bad",
                        "target_criterion_sha256": "bad",
                        "confirmation": "confirmed",
                        "confirmed_at": NOW.isoformat(),
                    }
                ],
            },
        ),
        (
            "confirmed-corrections.schema.json",
            {
                "schema_version": "1.0.0",
                "job_id": JOB_ID,
                "revision": 1,
                "updated_at": NOW.isoformat(),
                "criteria": [],
                "criteria_extraction_confirmations": [
                    {
                        "correction_id": CORRECTION_ID,
                        "target_extraction_sha256": SHA_A,
                        "confirmation": "confirmed_empty",
                        "record_state": "active",
                        "superseded_by": "correction_" + "8" * 32,
                        "confirmed_at": NOW.isoformat(),
                    }
                ],
            },
        ),
        (
            "confirmed-corrections.schema.json",
            {
                "schema_version": "1.0.0",
                "job_id": JOB_ID,
                "revision": 1,
                "updated_at": NOW.isoformat(),
                "criteria": [
                    {
                        "correction_id": CORRECTION_ID,
                        "criterion_id": CRITERION_ID,
                        "target_source_sha256": SHA_A,
                        "target_criterion_sha256": SHA_B,
                        "confirmation": "corrected",
                        "confirmed_at": NOW.isoformat(),
                    }
                ],
            },
        ),
        (
            "application-decision.schema.json",
            {
                "schema_version": "1.0.0",
                "job_id": JOB_ID,
                "revision": 1,
                "updated_at": NOW.isoformat(),
                "decision": "apply",
                "confirmation_state": "unconfirmed",
            },
        ),
        (
            "application-decision.schema.json",
            {
                "schema_version": "1.0.0",
                "job_id": JOB_ID,
                "revision": 0,
                "updated_at": NOW.isoformat(),
                "decision": "undecided",
                "rationale": "An unresolved decision must not carry a rationale.",
            },
        ),
        (
            "application-decision.schema.json",
            {
                **application_decision().model_dump(mode="json"),
                "basis": {
                    "criteria_sha256": "bad",
                    "matches_sha256": SHA_A,
                    "status": "current",
                },
            },
        ),
    ],
)
def test_standalone_decision_schemas_reject_invalid_semantics(
    schema_name: str,
    payload: dict[str, object],
) -> None:
    schema = json.loads((Path("schemas") / schema_name).read_text(encoding="utf-8"))

    assert list(Draft202012Validator(schema).iter_errors(payload))


def test_standalone_confirmed_corrections_schema_accepts_historical_v1_shape() -> None:
    schema = json.loads(
        (Path("schemas") / "confirmed-corrections.schema.json").read_text(encoding="utf-8")
    )
    historical = {
        "schema_version": "1.0.0",
        "job_id": JOB_ID,
        "revision": 0,
        "updated_at": NOW.isoformat(),
        "criteria": [],
    }

    Draft202012Validator(schema).validate(historical)
    loaded = ConfirmedCorrectionsV1.model_validate(historical)
    assert loaded.criteria_extraction_confirmations == ()


def test_brief_distinguishes_unanswered_from_confirmed_empty() -> None:
    brief = application_brief()

    assert brief.schema_version == APPLICATION_BRIEF_SCHEMA_VERSION
    assert brief.motivation.value == ""
    assert brief.motivation.confirmation_state == "confirmed"
    assert brief.exclusions.items == ()
    assert brief.exclusions.confirmation_state == "confirmed"

    unanswered = ConfirmedTextV1(value=None, confirmation_state="unconfirmed")
    assert unanswered.value is None
    with pytest.raises(ValidationError):
        ConfirmedTextV1(value=None, confirmation_state="confirmed")


def test_document_plan_requires_one_task_per_requirement_and_valid_unresolved_ids() -> None:
    plan = document_plan()

    assert plan.schema_version == REQUIRED_DOCUMENT_PLAN_SCHEMA_VERSION

    with pytest.raises(ValidationError):
        RequiredDocumentPlanV1.model_validate({**plan.model_dump(), "tasks": []})
    with pytest.raises(ValidationError):
        RequiredDocumentPlanV1.model_validate(
            {**plan.model_dump(), "unresolved_document_ids": ["document_" + "9" * 32]}
        )


@pytest.mark.parametrize(
    ("schema_name", "model"),
    [
        ("criteria.schema.json", criteria_catalog),
        ("evidence-catalog.schema.json", evidence_catalog),
        ("criterion-matches.schema.json", criterion_matches),
        (
            "confirmed-corrections.schema.json",
            lambda: ConfirmedCorrectionsV1(
                job_id=JOB_ID,
                revision=0,
                updated_at=NOW,
                criteria=(),
            ),
        ),
        ("application-decision.schema.json", application_decision),
        ("application-brief.schema.json", application_brief),
        ("required-document-plan.schema.json", document_plan),
    ],
)
def test_static_decision_schema_is_strict_and_accepts_model_dump(
    schema_name: str,
    model: object,
) -> None:
    schema = json.loads((Path("schemas") / schema_name).read_text(encoding="utf-8"))

    Draft202012Validator.check_schema(schema)
    assert schema["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert schema["additionalProperties"] is False
    Draft202012Validator(schema).validate(model().model_dump(mode="json"))


def test_schema_version_constants_are_frozen_at_v1() -> None:
    assert EVIDENCE_CATALOG_SCHEMA_VERSION == "1.0.0"
    assert CRITERION_MATCHES_SCHEMA_VERSION == "1.0.0"


@pytest.mark.parametrize(
    ("schema_name", "model_class"),
    [
        ("criteria.schema.json", CriteriaCatalogV1),
        ("evidence-catalog.schema.json", EvidenceCatalogV1),
        ("criterion-matches.schema.json", CriterionMatchesV1),
        ("confirmed-corrections.schema.json", ConfirmedCorrectionsV1),
        ("application-decision.schema.json", ApplicationDecisionV1),
        ("application-brief.schema.json", ApplicationBriefV1),
        ("required-document-plan.schema.json", RequiredDocumentPlanV1),
    ],
)
def test_generated_decision_schemas_match_model_contracts(
    schema_name: str,
    model_class: type,
) -> None:
    schema = json.loads((Path("schemas") / schema_name).read_text(encoding="utf-8"))

    assert schema == model_class.model_json_schema(mode="validation")
