from __future__ import annotations

from pathlib import PurePosixPath, PureWindowsPath
import re
from typing import Literal

from pydantic import AwareDatetime, BaseModel, ConfigDict, Field, field_validator, model_validator

from canisend.stage_models import ArtifactFingerprint


CRITERIA_SCHEMA_VERSION = "1.0.0"
CRITERION_MATCHES_SCHEMA_VERSION = "1.0.0"
CONFIRMED_CORRECTIONS_SCHEMA_VERSION = "1.0.0"
APPLICATION_DECISION_SCHEMA_VERSION = "1.0.0"
APPLICATION_BRIEF_SCHEMA_VERSION = "1.0.0"
REQUIRED_DOCUMENT_PLAN_SCHEMA_VERSION = "1.0.0"

JSON_SCHEMA_DIALECT = "https://json-schema.org/draft/2020-12/schema"
SCHEMA_BASE_ID = "https://github.com/jxpeng98/CanISend/schemas"

CriterionImportance = Literal["essential", "desirable"]
SourceState = Literal["known", "unknown"]
ExtractionConfidence = Literal["high", "medium", "low", "unknown"]
CriterionConfirmationState = Literal["unconfirmed", "confirmed", "corrected"]
RecordState = Literal["active", "superseded", "withdrawn"]
MatchClassification = Literal["strong", "partial", "weak", "missing", "unknown"]
MatchReviewState = Literal["proposed", "confirmed", "corrected"]
ConfirmationState = Literal["unconfirmed", "confirmed"]
DecisionValue = Literal["undecided", "apply", "hold", "skip"]
BasisStatus = Literal["current", "review_required"]
DocumentRequirement = Literal["required", "optional", "unknown"]
DocumentAction = Literal["prepare", "omit", "needs_confirmation"]

_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_JOB_ID_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]*$")
_CRITERION_ID_RE = re.compile(r"^criterion_[0-9a-f]{32}$")
_EVIDENCE_ID_RE = re.compile(r"^evidence_[0-9a-f]{32}$")
_CORRECTION_ID_RE = re.compile(r"^correction_[0-9a-f]{32}$")
_DOCUMENT_ID_RE = re.compile(r"^document_[0-9a-f]{32}$")
_DOTTED_ID_RE = re.compile(r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)+$")
_REASON_ID_RE = re.compile(r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$")
_SLUG_RE = re.compile(r"^[a-z][a-z0-9_]*$")
_SEMVER_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")


class DecisionContractModel(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
    )


class SourceSpanV1(DecisionContractModel):
    path: str
    start_line: int = Field(ge=1)
    end_line: int = Field(ge=1)
    text_sha256: str
    occurrence: int = Field(ge=1)
    occurrence_count: int = Field(ge=1)

    @field_validator("path")
    @classmethod
    def _valid_path(cls, value: str) -> str:
        return _relative_path(value)

    @field_validator("text_sha256")
    @classmethod
    def _valid_text_hash(cls, value: str) -> str:
        return _sha256(value, label="text_sha256")

    @model_validator(mode="after")
    def _consistent_span(self) -> SourceSpanV1:
        if self.end_line < self.start_line:
            raise ValueError("source span end_line must not precede start_line")
        if self.occurrence > self.occurrence_count:
            raise ValueError("source span occurrence must not exceed occurrence_count")
        return self


class CriterionV1(DecisionContractModel):
    criterion_id: str
    importance: CriterionImportance
    text: str = Field(min_length=1)
    source_text: str = Field(min_length=1)
    source_state: SourceState
    source_span: SourceSpanV1 | None = None
    confidence: ExtractionConfidence
    confirmation_state: CriterionConfirmationState = "unconfirmed"
    unknown_reason: str | None = None

    @field_validator("criterion_id")
    @classmethod
    def _valid_criterion_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CRITERION_ID_RE, label="criterion_id")

    @field_validator("unknown_reason")
    @classmethod
    def _valid_unknown_reason(cls, value: str | None) -> str | None:
        if value is not None and _REASON_ID_RE.fullmatch(value) is None:
            raise ValueError("unknown_reason must be a lowercase reason identifier")
        return value

    @model_validator(mode="after")
    def _consistent_source_state(self) -> CriterionV1:
        if self.source_state == "known":
            if self.source_span is None or self.confidence == "unknown":
                raise ValueError("a known criterion source requires a span and non-unknown confidence")
            if self.unknown_reason is not None:
                raise ValueError("a known criterion source must not include unknown_reason")
        else:
            if self.source_span is not None or self.confidence != "unknown":
                raise ValueError("an unknown criterion source must omit span and use unknown confidence")
            if self.unknown_reason is None:
                raise ValueError("an unknown criterion source requires unknown_reason")
        return self


class CriteriaCatalogV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendCriteriaCatalogV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/criteria.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = CRITERIA_SCHEMA_VERSION
    job_id: str
    input_fingerprint: str
    inputs: tuple[ArtifactFingerprint, ...]
    criteria: tuple[CriterionV1, ...]
    unresolved_criterion_ids: tuple[str, ...] = ()
    orphaned_correction_ids: tuple[str, ...] = ()

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("input_fingerprint")
    @classmethod
    def _valid_input_fingerprint(cls, value: str) -> str:
        return _sha256(value, label="input_fingerprint")

    @field_validator("unresolved_criterion_ids")
    @classmethod
    def _valid_unresolved_ids(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_unique(values, label="unresolved criterion IDs")
        for value in values:
            _semantic_id(value, pattern=_CRITERION_ID_RE, label="criterion_id")
        return values

    @field_validator("orphaned_correction_ids")
    @classmethod
    def _valid_orphaned_ids(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_unique(values, label="orphaned correction IDs")
        for value in values:
            _semantic_id(value, pattern=_CORRECTION_ID_RE, label="correction_id")
        return values

    @model_validator(mode="after")
    def _consistent_catalog(self) -> CriteriaCatalogV1:
        criterion_ids = tuple(item.criterion_id for item in self.criteria)
        _require_unique(criterion_ids, label="criterion IDs")
        input_paths = tuple(item.path for item in self.inputs)
        _require_unique(input_paths, label="criteria input paths")
        expected_unresolved = {
            item.criterion_id
            for item in self.criteria
            if item.source_state == "unknown" or item.confirmation_state == "unconfirmed"
        }
        if set(self.unresolved_criterion_ids) != expected_unresolved:
            raise ValueError("unresolved_criterion_ids must name every unresolved criterion exactly once")
        return self


class CriterionCorrectionV1(DecisionContractModel):
    correction_id: str
    criterion_id: str
    target_source_sha256: str
    confirmation: Literal["confirmed", "corrected"]
    corrected_text: str | None = None
    source_occurrence: int | None = Field(default=None, ge=1)
    record_state: RecordState = "active"
    superseded_by: str | None = None
    confirmed_at: AwareDatetime

    @field_validator("correction_id")
    @classmethod
    def _valid_correction_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CORRECTION_ID_RE, label="correction_id")

    @field_validator("criterion_id")
    @classmethod
    def _valid_criterion_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CRITERION_ID_RE, label="criterion_id")

    @field_validator("target_source_sha256")
    @classmethod
    def _valid_source_hash(cls, value: str) -> str:
        return _sha256(value, label="target_source_sha256")

    @field_validator("superseded_by")
    @classmethod
    def _valid_superseded_by(cls, value: str | None) -> str | None:
        if value is not None:
            return _semantic_id(value, pattern=_CORRECTION_ID_RE, label="superseded_by")
        return None

    @model_validator(mode="after")
    def _consistent_correction(self) -> CriterionCorrectionV1:
        if self.confirmation == "corrected" and not self.corrected_text:
            raise ValueError("a corrected criterion requires corrected_text")
        if self.confirmation == "confirmed" and self.corrected_text is not None:
            raise ValueError("a confirmed criterion must not include corrected_text")
        if self.record_state == "superseded":
            if self.superseded_by is None or self.superseded_by == self.correction_id:
                raise ValueError("a superseded correction requires a different superseded_by ID")
        elif self.superseded_by is not None:
            raise ValueError("only a superseded correction may include superseded_by")
        return self


class ConfirmedCorrectionsV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendConfirmedCorrectionsV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/confirmed-corrections.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = CONFIRMED_CORRECTIONS_SCHEMA_VERSION
    job_id: str
    revision: int = Field(ge=0)
    updated_at: AwareDatetime
    criteria: tuple[CriterionCorrectionV1, ...] = ()

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @model_validator(mode="after")
    def _consistent_overlay(self) -> ConfirmedCorrectionsV1:
        correction_ids = tuple(item.correction_id for item in self.criteria)
        _require_unique(correction_ids, label="correction IDs")
        active_targets = tuple(
            item.criterion_id for item in self.criteria if item.record_state == "active"
        )
        _require_unique(active_targets, label="active correction targets")
        known_ids = set(correction_ids)
        for item in self.criteria:
            if item.superseded_by is not None and item.superseded_by not in known_ids:
                raise ValueError("superseded_by must resolve within the correction overlay")
        return self


class EvidenceRefV1(DecisionContractModel):
    evidence_id: str
    path: str
    section: str = Field(min_length=1)
    item_locator: str | None = None
    kind: str = Field(min_length=1)
    content_sha256: str

    @field_validator("evidence_id")
    @classmethod
    def _valid_evidence_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_EVIDENCE_ID_RE, label="evidence_id")

    @field_validator("path")
    @classmethod
    def _valid_path(cls, value: str) -> str:
        return _relative_path(value)

    @field_validator("item_locator")
    @classmethod
    def _valid_item_locator(cls, value: str | None) -> str | None:
        if value is not None and any(character in value for character in "#/\\"):
            raise ValueError("item_locator must not contain path or fragment separators")
        return value

    @field_validator("kind")
    @classmethod
    def _valid_kind(cls, value: str) -> str:
        if _SLUG_RE.fullmatch(value) is None:
            raise ValueError("evidence kind must be a lowercase identifier")
        return value

    @field_validator("content_sha256")
    @classmethod
    def _valid_content_hash(cls, value: str) -> str:
        return _sha256(value, label="content_sha256")

    @property
    def citation(self) -> str:
        base = f"{self.path}#{self.section}"
        return f"{base}/{self.item_locator}" if self.item_locator else base


class EvidenceGapV1(DecisionContractModel):
    code: str
    message: str = Field(min_length=1)
    next_action: str = Field(min_length=1)

    @field_validator("code")
    @classmethod
    def _valid_code(cls, value: str) -> str:
        if _DOTTED_ID_RE.fullmatch(value) is None:
            raise ValueError("evidence gap code must be a lowercase dotted identifier")
        return value


class CriterionMatchV1(DecisionContractModel):
    criterion_id: str
    classification: MatchClassification
    evidence_ref_ids: tuple[str, ...] = ()
    gaps: tuple[EvidenceGapV1, ...] = ()
    review_state: MatchReviewState = "proposed"

    @field_validator("criterion_id")
    @classmethod
    def _valid_criterion_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CRITERION_ID_RE, label="criterion_id")

    @field_validator("evidence_ref_ids")
    @classmethod
    def _valid_evidence_ids(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_unique(values, label="match evidence IDs")
        for value in values:
            _semantic_id(value, pattern=_EVIDENCE_ID_RE, label="evidence_id")
        return values

    @model_validator(mode="after")
    def _consistent_match(self) -> CriterionMatchV1:
        _require_unique(tuple(gap.code for gap in self.gaps), label="evidence gap codes")
        if self.classification in {"missing", "unknown"}:
            if self.evidence_ref_ids:
                raise ValueError("missing or unknown matches must not link evidence")
            if not self.gaps:
                raise ValueError("missing or unknown matches require an explicit evidence gap")
        elif not self.evidence_ref_ids:
            raise ValueError("supported matches require at least one evidence reference")
        return self


class CriterionMatchesV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendCriterionMatchesV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/criterion-matches.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = CRITERION_MATCHES_SCHEMA_VERSION
    job_id: str
    input_fingerprint: str
    criteria_catalog_sha256: str
    evidence_catalog_sha256: str
    matcher_strategy: str
    matcher_version: str
    evidence_refs: tuple[EvidenceRefV1, ...] = ()
    matches: tuple[CriterionMatchV1, ...]

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator(
        "input_fingerprint",
        "criteria_catalog_sha256",
        "evidence_catalog_sha256",
    )
    @classmethod
    def _valid_hashes(cls, value: str, info: object) -> str:
        return _sha256(value, label=getattr(info, "field_name", "sha256"))

    @field_validator("matcher_strategy")
    @classmethod
    def _valid_strategy(cls, value: str) -> str:
        if _DOTTED_ID_RE.fullmatch(value) is None:
            raise ValueError("matcher_strategy must be a lowercase dotted identifier")
        return value

    @field_validator("matcher_version")
    @classmethod
    def _valid_matcher_version(cls, value: str) -> str:
        if _SEMVER_RE.fullmatch(value) is None:
            raise ValueError("matcher_version must be a semantic version")
        return value

    @model_validator(mode="after")
    def _consistent_matches(self) -> CriterionMatchesV1:
        evidence_ids = tuple(item.evidence_id for item in self.evidence_refs)
        _require_unique(evidence_ids, label="evidence reference IDs")
        criterion_ids = tuple(item.criterion_id for item in self.matches)
        _require_unique(criterion_ids, label="criterion match IDs")
        known_evidence = set(evidence_ids)
        for match in self.matches:
            if not set(match.evidence_ref_ids).issubset(known_evidence):
                raise ValueError("criterion match evidence references must resolve in evidence_refs")
        return self


class DecisionBasisV1(DecisionContractModel):
    criteria_sha256: str
    matches_sha256: str
    status: BasisStatus = "current"

    @field_validator("criteria_sha256", "matches_sha256")
    @classmethod
    def _valid_hashes(cls, value: str, info: object) -> str:
        return _sha256(value, label=getattr(info, "field_name", "sha256"))


class ApplicationDecisionV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendApplicationDecisionV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/application-decision.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = APPLICATION_DECISION_SCHEMA_VERSION
    job_id: str
    revision: int = Field(ge=0)
    updated_at: AwareDatetime
    decision: DecisionValue = "undecided"
    confirmation_state: ConfirmationState = "unconfirmed"
    confirmed_at: AwareDatetime | None = None
    rationale: str | None = None
    basis: DecisionBasisV1 | None = None

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @model_validator(mode="after")
    def _consistent_decision(self) -> ApplicationDecisionV1:
        if self.decision == "undecided":
            if self.confirmation_state != "unconfirmed" or self.confirmed_at is not None or self.basis is not None:
                raise ValueError("an undecided application must remain unconfirmed without a basis")
        elif self.confirmation_state != "confirmed" or self.confirmed_at is None or self.basis is None:
            raise ValueError("apply, hold, or skip requires explicit confirmation and a decision basis")
        return self


class LanguagePreferenceV1(DecisionContractModel):
    value: Literal["uk", "us"] | None = None
    confirmation_state: ConfirmationState = "unconfirmed"

    @model_validator(mode="after")
    def _consistent_value(self) -> LanguagePreferenceV1:
        if self.confirmation_state == "confirmed" and self.value is None:
            raise ValueError("a confirmed language preference requires a value")
        return self


class ConfirmedTextV1(DecisionContractModel):
    value: str | None = None
    confirmation_state: ConfirmationState = "unconfirmed"

    @model_validator(mode="after")
    def _consistent_value(self) -> ConfirmedTextV1:
        if self.confirmation_state == "confirmed" and self.value is None:
            raise ValueError("confirmed text requires a value; use an empty string for confirmed-empty")
        return self


class ConfirmedIdSelectionV1(DecisionContractModel):
    criterion_ids: tuple[str, ...] = ()
    evidence_ref_ids: tuple[str, ...] = ()
    confirmation_state: ConfirmationState = "unconfirmed"

    @field_validator("criterion_ids")
    @classmethod
    def _valid_criterion_ids(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_unique(values, label="emphasis criterion IDs")
        for value in values:
            _semantic_id(value, pattern=_CRITERION_ID_RE, label="criterion_id")
        return values

    @field_validator("evidence_ref_ids")
    @classmethod
    def _valid_evidence_ids(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_unique(values, label="emphasis evidence IDs")
        for value in values:
            _semantic_id(value, pattern=_EVIDENCE_ID_RE, label="evidence_id")
        return values


class ConfirmedStringListV1(DecisionContractModel):
    items: tuple[str, ...] = ()
    confirmation_state: ConfirmationState = "unconfirmed"

    @field_validator("items")
    @classmethod
    def _unique_items(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_unique(values, label="confirmed string-list items")
        return values


class DocumentChoiceV1(DecisionContractModel):
    document_id: str
    action: DocumentAction = "needs_confirmation"
    confirmation_state: ConfirmationState = "unconfirmed"

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_DOCUMENT_ID_RE, label="document_id")

    @model_validator(mode="after")
    def _consistent_choice(self) -> DocumentChoiceV1:
        if self.action == "needs_confirmation" and self.confirmation_state != "unconfirmed":
            raise ValueError("needs_confirmation must remain unconfirmed")
        if self.action != "needs_confirmation" and self.confirmation_state != "confirmed":
            raise ValueError("prepare or omit requires explicit confirmation")
        return self


class ApplicationBriefV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendApplicationBriefV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/application-brief.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = APPLICATION_BRIEF_SCHEMA_VERSION
    job_id: str
    revision: int = Field(ge=0)
    updated_at: AwareDatetime
    decision_sha256: str | None = None
    language: LanguagePreferenceV1 = Field(default_factory=LanguagePreferenceV1)
    writing_style: ConfirmedTextV1 = Field(default_factory=ConfirmedTextV1)
    motivation: ConfirmedTextV1 = Field(default_factory=ConfirmedTextV1)
    emphasis: ConfirmedIdSelectionV1 = Field(default_factory=ConfirmedIdSelectionV1)
    exclusions: ConfirmedStringListV1 = Field(default_factory=ConfirmedStringListV1)
    document_choices: tuple[DocumentChoiceV1, ...] = ()

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("decision_sha256")
    @classmethod
    def _valid_decision_hash(cls, value: str | None) -> str | None:
        return _sha256(value, label="decision_sha256") if value is not None else None

    @model_validator(mode="after")
    def _unique_document_choices(self) -> ApplicationBriefV1:
        _require_unique(
            tuple(item.document_id for item in self.document_choices),
            label="brief document choices",
        )
        return self


class DocumentRequirementV1(DecisionContractModel):
    document_id: str
    label: str = Field(min_length=1)
    normalized_kind: str
    requirement: DocumentRequirement
    source_text: str = Field(min_length=1)
    source_state: SourceState
    source_span: SourceSpanV1 | None = None
    confirmation_state: CriterionConfirmationState = "unconfirmed"
    unknown_reason: str | None = None

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_DOCUMENT_ID_RE, label="document_id")

    @field_validator("normalized_kind")
    @classmethod
    def _valid_normalized_kind(cls, value: str) -> str:
        if _SLUG_RE.fullmatch(value) is None:
            raise ValueError("normalized document kind must be a lowercase identifier")
        return value

    @field_validator("unknown_reason")
    @classmethod
    def _valid_unknown_reason(cls, value: str | None) -> str | None:
        if value is not None and _REASON_ID_RE.fullmatch(value) is None:
            raise ValueError("unknown_reason must be a lowercase reason identifier")
        return value

    @model_validator(mode="after")
    def _consistent_source(self) -> DocumentRequirementV1:
        if self.source_state == "known":
            if self.source_span is None or self.unknown_reason is not None:
                raise ValueError("a known document requirement needs a span and no unknown_reason")
        elif self.source_span is not None or self.unknown_reason is None:
            raise ValueError("an unknown document requirement must omit span and name unknown_reason")
        return self


class DocumentTaskV1(DecisionContractModel):
    document_id: str
    action: DocumentAction
    confirmation_state: ConfirmationState
    blockers: tuple[str, ...] = ()

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_DOCUMENT_ID_RE, label="document_id")

    @field_validator("blockers")
    @classmethod
    def _valid_blockers(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_unique(values, label="document blockers")
        for value in values:
            if _DOTTED_ID_RE.fullmatch(value) is None:
                raise ValueError("document blocker must be a lowercase dotted identifier")
        return values

    @model_validator(mode="after")
    def _consistent_action(self) -> DocumentTaskV1:
        if self.action == "needs_confirmation":
            if self.confirmation_state != "unconfirmed" or not self.blockers:
                raise ValueError("an unresolved document task must be unconfirmed with a blocker")
        elif self.confirmation_state != "confirmed":
            raise ValueError("a prepare or omit document task requires confirmation")
        return self


class RequiredDocumentPlanV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendRequiredDocumentPlanV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/required-document-plan.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = REQUIRED_DOCUMENT_PLAN_SCHEMA_VERSION
    job_id: str
    input_fingerprint: str
    requirements: tuple[DocumentRequirementV1, ...]
    tasks: tuple[DocumentTaskV1, ...]
    unresolved_document_ids: tuple[str, ...] = ()

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("input_fingerprint")
    @classmethod
    def _valid_input_fingerprint(cls, value: str) -> str:
        return _sha256(value, label="input_fingerprint")

    @field_validator("unresolved_document_ids")
    @classmethod
    def _valid_unresolved_ids(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_unique(values, label="unresolved document IDs")
        for value in values:
            _semantic_id(value, pattern=_DOCUMENT_ID_RE, label="document_id")
        return values

    @model_validator(mode="after")
    def _consistent_plan(self) -> RequiredDocumentPlanV1:
        requirement_ids = tuple(item.document_id for item in self.requirements)
        task_ids = tuple(item.document_id for item in self.tasks)
        _require_unique(requirement_ids, label="document requirement IDs")
        _require_unique(task_ids, label="document task IDs")
        if set(requirement_ids) != set(task_ids):
            raise ValueError("document plan requires exactly one task per requirement")
        if not set(self.unresolved_document_ids).issubset(set(requirement_ids)):
            raise ValueError("unresolved document IDs must resolve to requirements")
        task_by_id = {item.document_id: item for item in self.tasks}
        expected_unresolved = {
            document_id
            for document_id, task in task_by_id.items()
            if task.action == "needs_confirmation"
        }
        if set(self.unresolved_document_ids) != expected_unresolved:
            raise ValueError("unresolved document IDs must match needs-confirmation tasks")
        return self


def _job_id(value: str) -> str:
    if _JOB_ID_RE.fullmatch(value) is None:
        raise ValueError("job_id must be a lowercase safe identifier")
    return value


def _semantic_id(value: str, *, pattern: re.Pattern[str], label: str) -> str:
    if pattern.fullmatch(value) is None:
        raise ValueError(f"{label} is invalid")
    return value


def _sha256(value: str, *, label: str) -> str:
    if _SHA256_RE.fullmatch(value) is None:
        raise ValueError(f"{label} must be a lowercase 64-character digest")
    return value


def _relative_path(value: str) -> str:
    if not value or value == "." or "\\" in value or "\x00" in value:
        raise ValueError("path must be a normalized relative POSIX path")
    posix = PurePosixPath(value)
    windows = PureWindowsPath(value)
    if (
        posix.is_absolute()
        or windows.is_absolute()
        or bool(windows.drive)
        or any(part in {"", ".", ".."} for part in value.split("/"))
    ):
        raise ValueError("path must be a normalized relative POSIX path")
    return posix.as_posix()


def _require_unique(values: tuple[str, ...], *, label: str) -> None:
    if len(values) != len(set(values)):
        raise ValueError(f"{label} must be unique")
