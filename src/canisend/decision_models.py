from __future__ import annotations

from pathlib import PurePosixPath, PureWindowsPath
import re
from typing import Annotated, Literal

from pydantic import (
    AwareDatetime,
    BaseModel,
    BeforeValidator,
    ConfigDict,
    Field,
    StrictInt,
    field_validator,
    model_validator,
)

CRITERIA_SCHEMA_VERSION = "1.0.0"
EVIDENCE_CATALOG_SCHEMA_VERSION = "1.0.0"
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
CriteriaExtractionState = Literal["extracted", "unknown", "confirmed_empty"]
EvidenceCatalogState = Literal["available", "empty", "unavailable"]
EvidenceSourceType = Literal["manifest", "profile_source", "generated_evidence"]
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

MAX_USER_REVISION = 2**63 - 1
_RFC3339_DATETIME_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:[Zz]|[+-]\d{2}:\d{2})$"
)


def _reject_numeric_control_timestamp(value: object) -> object:
    """Keep user-owned control timestamps textual or already-typed datetimes.

    Pydantic's datetime parser intentionally accepts Unix timestamps.  That is
    useful for many APIs, but it is ambiguous in durable user/control records
    and diverges from their JSON Schemas, which require date-time strings.
    """

    if isinstance(value, (bool, int, float)):
        raise ValueError("control timestamps must be aware datetimes or date-time strings")
    if isinstance(value, str):
        normalized = value.strip()
        if _RFC3339_DATETIME_RE.fullmatch(normalized) is None:
            raise ValueError("control timestamp strings must use RFC 3339 date-time syntax")
        return normalized
    return value

Sha256Value = Annotated[str, Field(pattern=_SHA256_RE.pattern)]
JobIdentifier = Annotated[str, Field(pattern=_JOB_ID_RE.pattern)]
CriterionIdentifier = Annotated[str, Field(pattern=_CRITERION_ID_RE.pattern)]
CorrectionIdentifier = Annotated[str, Field(pattern=_CORRECTION_ID_RE.pattern)]
ReasonIdentifier = Annotated[str, Field(pattern=_REASON_ID_RE.pattern)]
UserRevision = Annotated[StrictInt, Field(ge=0, le=MAX_USER_REVISION)]
UserControlTimestamp = Annotated[
    AwareDatetime,
    BeforeValidator(_reject_numeric_control_timestamp),
]


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
    anchor_sha256: str
    occurrence: int = Field(ge=1)
    occurrence_count: int = Field(ge=1)

    @field_validator("path")
    @classmethod
    def _valid_path(cls, value: str) -> str:
        return _relative_path(value)

    @field_validator("text_sha256", "anchor_sha256")
    @classmethod
    def _valid_span_hashes(cls, value: str, info: object) -> str:
        return _sha256(value, label=getattr(info, "field_name", "sha256"))

    @model_validator(mode="after")
    def _consistent_span(self) -> SourceSpanV1:
        if self.end_line < self.start_line:
            raise ValueError("source span end_line must not precede start_line")
        if self.occurrence > self.occurrence_count:
            raise ValueError("source span occurrence must not exceed occurrence_count")
        return self


class SemanticInputReceiptV1(DecisionContractModel):
    path: str
    projection_sha256: Sha256Value

    @field_validator("path")
    @classmethod
    def _valid_path(cls, value: str) -> str:
        return _relative_path(value)

    @field_validator("projection_sha256")
    @classmethod
    def _valid_projection_hash(cls, value: str) -> str:
        return _sha256(value, label="projection_sha256")


class CriterionV1(DecisionContractModel):
    model_config = ConfigDict(
        json_schema_extra={
            "allOf": [
                {
                    "if": {"properties": {"source_state": {"const": "known"}}},
                    "then": {
                        "required": ["source_span"],
                        "properties": {
                            "source_span": {"not": {"type": "null"}},
                            "source_candidates": {"maxItems": 0},
                            "confidence": {"enum": ["high", "medium", "low"]},
                            "unknown_reason": {"type": "null"},
                        },
                    },
                    "else": {
                        "required": ["unknown_reason"],
                        "properties": {
                            "source_span": {"type": "null"},
                            "confidence": {"const": "unknown"},
                            "unknown_reason": {"type": "string", "minLength": 1},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {
                            "unknown_reason": {"const": "source_receipt.ambiguous"}
                        },
                        "required": ["unknown_reason"],
                    },
                    "then": {"properties": {"source_candidates": {"minItems": 2}}},
                    "else": {"properties": {"source_candidates": {"maxItems": 0}}},
                },
                {
                    "if": {
                        "properties": {"confirmation_state": {"const": "unconfirmed"}}
                    },
                    "then": {"properties": {"confirmation_record_id": {"type": "null"}}},
                    "else": {
                        "required": ["confirmation_record_id"],
                        "properties": {
                            "confirmation_record_id": {"not": {"type": "null"}}
                        },
                    },
                },
            ]
        }
    )

    criterion_id: CriterionIdentifier
    importance: CriterionImportance
    text: str = Field(min_length=1)
    parsed_text_sha256: Sha256Value
    source_text: str = Field(min_length=1)
    source_state: SourceState
    source_span: SourceSpanV1 | None = None
    source_candidates: tuple[SourceSpanV1, ...] = ()
    confidence: ExtractionConfidence
    confirmation_state: CriterionConfirmationState = "unconfirmed"
    confirmation_record_id: CorrectionIdentifier | None = None
    unknown_reason: ReasonIdentifier | None = None

    @field_validator("criterion_id")
    @classmethod
    def _valid_criterion_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CRITERION_ID_RE, label="criterion_id")

    @field_validator("parsed_text_sha256")
    @classmethod
    def _valid_parsed_text_hash(cls, value: str) -> str:
        return _sha256(value, label="parsed_text_sha256")

    @field_validator("unknown_reason")
    @classmethod
    def _valid_unknown_reason(cls, value: str | None) -> str | None:
        if value is not None and _REASON_ID_RE.fullmatch(value) is None:
            raise ValueError("unknown_reason must be a lowercase reason identifier")
        return value

    @field_validator("confirmation_record_id")
    @classmethod
    def _valid_confirmation_record_id(cls, value: str | None) -> str | None:
        if value is not None:
            return _semantic_id(
                value,
                pattern=_CORRECTION_ID_RE,
                label="confirmation_record_id",
            )
        return None

    @model_validator(mode="after")
    def _consistent_source_state(self) -> CriterionV1:
        if self.source_state == "known":
            if self.source_span is None or self.confidence == "unknown":
                raise ValueError("a known criterion source requires a span and non-unknown confidence")
            if self.unknown_reason is not None:
                raise ValueError("a known criterion source must not include unknown_reason")
            if self.source_candidates:
                raise ValueError("a known criterion source must not retain candidate spans")
        else:
            if self.source_span is not None or self.confidence != "unknown":
                raise ValueError("an unknown criterion source must omit span and use unknown confidence")
            if self.unknown_reason is None:
                raise ValueError("an unknown criterion source requires unknown_reason")
            if self.unknown_reason == "source_receipt.ambiguous" and len(self.source_candidates) < 2:
                raise ValueError("an ambiguous source requires at least two candidate spans")
            if self.unknown_reason != "source_receipt.ambiguous" and self.source_candidates:
                raise ValueError("only an ambiguous source may include candidate spans")
        if self.confirmation_state == "unconfirmed" and self.confirmation_record_id is not None:
            raise ValueError("an unconfirmed criterion must not link a confirmation record")
        if self.confirmation_state != "unconfirmed" and self.confirmation_record_id is None:
            raise ValueError("a confirmed or corrected criterion must link its confirmation record")
        return self


class CorrectionReconciliationV1(DecisionContractModel):
    correction_id: CorrectionIdentifier
    criterion_id: CriterionIdentifier
    reason: ReasonIdentifier

    @field_validator("correction_id")
    @classmethod
    def _valid_correction_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CORRECTION_ID_RE, label="correction_id")

    @field_validator("criterion_id")
    @classmethod
    def _valid_criterion_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CRITERION_ID_RE, label="criterion_id")

    @field_validator("reason")
    @classmethod
    def _valid_reason(cls, value: str) -> str:
        if _REASON_ID_RE.fullmatch(value) is None:
            raise ValueError("reconciliation reason must be a lowercase reason identifier")
        return value


class ExtractionConfirmationReconciliationV1(DecisionContractModel):
    correction_id: CorrectionIdentifier
    reason: ReasonIdentifier

    @field_validator("correction_id")
    @classmethod
    def _valid_correction_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CORRECTION_ID_RE, label="correction_id")

    @field_validator("reason")
    @classmethod
    def _valid_reason(cls, value: str) -> str:
        if _REASON_ID_RE.fullmatch(value) is None:
            raise ValueError("reconciliation reason must be a lowercase reason identifier")
        return value


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
            "allOf": [
                {
                    "if": {"properties": {"extraction_state": {"const": "extracted"}}},
                    "then": {
                        "properties": {
                            "criteria": {"minItems": 1},
                            "extraction_unknown_reason": {"type": "null"},
                            "empty_confirmation_record_id": {"type": "null"},
                        }
                    },
                },
                {
                    "if": {"properties": {"extraction_state": {"const": "unknown"}}},
                    "then": {
                        "required": ["extraction_unknown_reason"],
                        "properties": {
                            "criteria": {"maxItems": 0},
                            "extraction_unknown_reason": {
                                "type": "string",
                                "minLength": 1,
                            },
                            "empty_confirmation_record_id": {"type": "null"},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"extraction_state": {"const": "confirmed_empty"}}
                    },
                    "then": {
                        "required": ["empty_confirmation_record_id"],
                        "properties": {
                            "criteria": {"maxItems": 0},
                            "extraction_unknown_reason": {"type": "null"},
                            "empty_confirmation_record_id": {"not": {"type": "null"}},
                        },
                    },
                },
            ],
        },
    )

    schema_version: Literal["1.0.0"] = CRITERIA_SCHEMA_VERSION
    job_id: JobIdentifier
    input_fingerprint: Sha256Value
    semantic_inputs: tuple[SemanticInputReceiptV1, ...]
    extraction_state: CriteriaExtractionState
    extraction_unknown_reason: ReasonIdentifier | None = None
    empty_confirmation_record_id: CorrectionIdentifier | None = None
    criteria: tuple[CriterionV1, ...]
    unresolved_criterion_ids: tuple[CriterionIdentifier, ...] = ()
    orphaned_corrections: tuple[CorrectionReconciliationV1, ...] = ()
    orphaned_extraction_confirmations: tuple[
        ExtractionConfirmationReconciliationV1, ...
    ] = ()

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

    @field_validator("extraction_unknown_reason")
    @classmethod
    def _valid_extraction_reason(cls, value: str | None) -> str | None:
        if value is not None and _REASON_ID_RE.fullmatch(value) is None:
            raise ValueError("extraction_unknown_reason must be a lowercase reason identifier")
        return value

    @field_validator("empty_confirmation_record_id")
    @classmethod
    def _valid_empty_confirmation_id(cls, value: str | None) -> str | None:
        if value is not None:
            return _semantic_id(
                value,
                pattern=_CORRECTION_ID_RE,
                label="empty_confirmation_record_id",
            )
        return None

    @model_validator(mode="after")
    def _consistent_catalog(self) -> CriteriaCatalogV1:
        criterion_ids = tuple(item.criterion_id for item in self.criteria)
        _require_unique(criterion_ids, label="criterion IDs")
        input_paths = tuple(item.path for item in self.semantic_inputs)
        _require_unique(input_paths, label="criteria input paths")
        expected_unresolved = {
            item.criterion_id
            for item in self.criteria
            if item.source_state == "unknown" or item.confirmation_state == "unconfirmed"
        }
        if set(self.unresolved_criterion_ids) != expected_unresolved:
            raise ValueError("unresolved_criterion_ids must name every unresolved criterion exactly once")
        confirmation_ids = tuple(
            item.confirmation_record_id
            for item in self.criteria
            if item.confirmation_record_id is not None
        )
        applied_confirmation_ids = (
            *confirmation_ids,
            *((self.empty_confirmation_record_id,) if self.empty_confirmation_record_id else ()),
        )
        _require_unique(
            applied_confirmation_ids,
            label="applied confirmation record IDs",
        )
        orphaned_ids = tuple(item.correction_id for item in self.orphaned_corrections)
        _require_unique(orphaned_ids, label="orphaned correction IDs")
        orphaned_extraction_ids = tuple(
            item.correction_id for item in self.orphaned_extraction_confirmations
        )
        _require_unique(
            orphaned_extraction_ids,
            label="orphaned extraction confirmation IDs",
        )
        if set(orphaned_ids) & set(orphaned_extraction_ids):
            raise ValueError("an orphaned record must appear in exactly one reconciliation list")
        if set(applied_confirmation_ids) & (
            set(orphaned_ids) | set(orphaned_extraction_ids)
        ):
            raise ValueError("an applied confirmation record cannot also be orphaned")
        if self.extraction_state == "extracted":
            if not self.criteria or self.extraction_unknown_reason is not None:
                raise ValueError("extracted criteria require at least one record and no unknown reason")
            if self.empty_confirmation_record_id is not None:
                raise ValueError("extracted criteria must not include an empty confirmation")
        elif self.extraction_state == "unknown":
            if self.criteria or self.extraction_unknown_reason is None:
                raise ValueError("unknown extraction requires no criteria and an explicit reason")
            if self.empty_confirmation_record_id is not None:
                raise ValueError("unknown extraction must not include an empty confirmation")
        else:
            if self.criteria or self.extraction_unknown_reason is not None:
                raise ValueError("confirmed-empty extraction requires no criteria and no unknown reason")
            if self.empty_confirmation_record_id is None:
                raise ValueError("confirmed-empty extraction requires a confirmation record")
        return self

    @property
    def orphaned_correction_ids(self) -> tuple[str, ...]:
        return tuple(item.correction_id for item in self.orphaned_corrections)


class CriterionCorrectionV1(DecisionContractModel):
    model_config = ConfigDict(
        json_schema_extra={
            "allOf": [
                {
                    "if": {"properties": {"confirmation": {"const": "corrected"}}},
                    "then": {
                        "required": ["corrected_text"],
                        "properties": {
                            "corrected_text": {"type": "string", "minLength": 1}
                        },
                    },
                    "else": {"properties": {"corrected_text": {"type": "null"}}},
                },
                {
                    "if": {
                        "properties": {"source_occurrence": {"type": "integer"}},
                        "required": ["source_occurrence"],
                    },
                    "then": {
                        "required": ["source_anchor_sha256"],
                        "properties": {
                            "source_anchor_sha256": {"not": {"type": "null"}}
                        },
                    },
                    "else": {"properties": {"source_anchor_sha256": {"type": "null"}}},
                },
                {
                    "if": {
                        "properties": {"record_state": {"const": "superseded"}},
                        "required": ["record_state"],
                    },
                    "then": {
                        "required": ["superseded_by"],
                        "properties": {"superseded_by": {"not": {"type": "null"}}},
                    },
                    "else": {"properties": {"superseded_by": {"type": "null"}}},
                },
            ]
        }
    )

    correction_id: CorrectionIdentifier
    criterion_id: CriterionIdentifier
    target_source_sha256: Sha256Value
    target_criterion_sha256: Sha256Value
    confirmation: Literal["confirmed", "corrected"]
    corrected_text: str | None = None
    source_occurrence: Annotated[StrictInt, Field(ge=1)] | None = None
    source_anchor_sha256: Sha256Value | None = None
    record_state: RecordState = "active"
    superseded_by: CorrectionIdentifier | None = None
    confirmed_at: UserControlTimestamp

    @field_validator("correction_id")
    @classmethod
    def _valid_correction_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CORRECTION_ID_RE, label="correction_id")

    @field_validator("criterion_id")
    @classmethod
    def _valid_criterion_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CRITERION_ID_RE, label="criterion_id")

    @field_validator("target_source_sha256", "target_criterion_sha256")
    @classmethod
    def _valid_target_hashes(cls, value: str, info: object) -> str:
        return _sha256(value, label=getattr(info, "field_name", "sha256"))

    @field_validator("source_anchor_sha256")
    @classmethod
    def _valid_source_anchor(cls, value: str | None) -> str | None:
        return _sha256(value, label="source_anchor_sha256") if value is not None else None

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
        if (self.source_occurrence is None) != (self.source_anchor_sha256 is None):
            raise ValueError("source occurrence and anchor hash must appear together")
        if self.record_state == "superseded":
            if self.superseded_by is None or self.superseded_by == self.correction_id:
                raise ValueError("a superseded correction requires a different superseded_by ID")
        elif self.superseded_by is not None:
            raise ValueError("only a superseded correction may include superseded_by")
        return self


class CriteriaExtractionConfirmationV1(DecisionContractModel):
    model_config = ConfigDict(
        json_schema_extra={
            "allOf": [
                {
                    "if": {
                        "properties": {"record_state": {"const": "superseded"}},
                        "required": ["record_state"],
                    },
                    "then": {
                        "required": ["superseded_by"],
                        "properties": {"superseded_by": {"not": {"type": "null"}}},
                    },
                    "else": {"properties": {"superseded_by": {"type": "null"}}},
                }
            ]
        }
    )

    correction_id: CorrectionIdentifier
    target_extraction_sha256: Sha256Value
    confirmation: Literal["confirmed_empty"]
    record_state: RecordState = "active"
    superseded_by: CorrectionIdentifier | None = None
    confirmed_at: UserControlTimestamp

    @field_validator("correction_id")
    @classmethod
    def _valid_correction_id(cls, value: str) -> str:
        return _semantic_id(value, pattern=_CORRECTION_ID_RE, label="correction_id")

    @field_validator("target_extraction_sha256")
    @classmethod
    def _valid_target_hash(cls, value: str) -> str:
        return _sha256(value, label="target_extraction_sha256")

    @field_validator("superseded_by")
    @classmethod
    def _valid_superseded_by(cls, value: str | None) -> str | None:
        if value is not None:
            return _semantic_id(value, pattern=_CORRECTION_ID_RE, label="superseded_by")
        return None

    @model_validator(mode="after")
    def _consistent_confirmation(self) -> CriteriaExtractionConfirmationV1:
        if self.record_state == "superseded":
            if self.superseded_by is None or self.superseded_by == self.correction_id:
                raise ValueError("a superseded extraction confirmation requires a different replacement")
        elif self.superseded_by is not None:
            raise ValueError("only a superseded extraction confirmation may include superseded_by")
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
    job_id: JobIdentifier
    revision: UserRevision
    updated_at: UserControlTimestamp
    criteria: tuple[CriterionCorrectionV1, ...] = ()
    criteria_extraction_confirmations: tuple[CriteriaExtractionConfirmationV1, ...] = ()

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @model_validator(mode="after")
    def _consistent_overlay(self) -> ConfirmedCorrectionsV1:
        criterion_correction_ids = tuple(item.correction_id for item in self.criteria)
        extraction_confirmation_ids = tuple(
            item.correction_id for item in self.criteria_extraction_confirmations
        )
        correction_ids = (*criterion_correction_ids, *extraction_confirmation_ids)
        _require_unique(correction_ids, label="correction IDs")
        active_targets = tuple(
            item.criterion_id for item in self.criteria if item.record_state == "active"
        )
        _require_unique(active_targets, label="active correction targets")
        known_ids = set(criterion_correction_ids)
        by_id = {item.correction_id: item for item in self.criteria}
        for item in self.criteria:
            if item.superseded_by is not None and item.superseded_by not in known_ids:
                raise ValueError("superseded_by must resolve within the correction overlay")
            if item.superseded_by is not None:
                replacement = by_id[item.superseded_by]
                if replacement.criterion_id != item.criterion_id:
                    raise ValueError("a superseding correction must target the same criterion")
                seen = {item.correction_id}
                current = replacement
                while current.superseded_by is not None:
                    if current.correction_id in seen:
                        raise ValueError("correction supersession must not contain a cycle")
                    seen.add(current.correction_id)
                    current = by_id[current.superseded_by]
                if current.correction_id in seen or current.record_state not in {
                    "active",
                    "withdrawn",
                }:
                    raise ValueError(
                        "a correction supersession chain must terminate at an active or withdrawn record"
                    )
        active_extractions = tuple(
            item
            for item in self.criteria_extraction_confirmations
            if item.record_state == "active"
        )
        if len(active_extractions) > 1:
            raise ValueError("only one criteria extraction confirmation may be active")
        extraction_by_id = {
            item.correction_id: item for item in self.criteria_extraction_confirmations
        }
        for item in self.criteria_extraction_confirmations:
            if item.superseded_by is not None and item.superseded_by not in extraction_by_id:
                raise ValueError(
                    "extraction superseded_by must resolve within extraction history"
                )
        for item in self.criteria_extraction_confirmations:
            if item.superseded_by is None:
                continue
            replacement = extraction_by_id[item.superseded_by]
            seen = {item.correction_id}
            current = replacement
            while current.superseded_by is not None:
                if current.correction_id in seen:
                    raise ValueError("extraction confirmation supersession must not contain a cycle")
                seen.add(current.correction_id)
                current = extraction_by_id[current.superseded_by]
            if current.correction_id in seen or current.record_state != "active":
                raise ValueError("an extraction supersession chain must terminate at an active record")
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


class EvidenceSourceReceiptV1(DecisionContractModel):
    path: str
    source_type: EvidenceSourceType
    content_sha256: str
    size_bytes: int = Field(ge=0)
    item_count: int = Field(ge=0)

    @field_validator("path")
    @classmethod
    def _valid_path(cls, value: str) -> str:
        return _relative_path(value)

    @field_validator("content_sha256")
    @classmethod
    def _valid_content_hash(cls, value: str) -> str:
        return _sha256(value, label="content_sha256")

    @model_validator(mode="after")
    def _consistent_source_receipt(self) -> EvidenceSourceReceiptV1:
        if self.source_type in {"manifest", "profile_source"} and self.item_count != 0:
            raise ValueError("an evidence manifest or profile source receipt must have item_count 0")
        return self


class EvidenceCatalogItemV1(DecisionContractModel):
    evidence_id: str
    path: str
    section: str = Field(min_length=1)
    item_locator: str | None = None
    kind: str = Field(min_length=1)
    text: str = Field(min_length=1)
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

    @property
    def reference(self) -> EvidenceRefV1:
        return EvidenceRefV1(
            evidence_id=self.evidence_id,
            path=self.path,
            section=self.section,
            item_locator=self.item_locator,
            kind=self.kind,
            content_sha256=self.content_sha256,
        )


class EvidenceCatalogV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendEvidenceCatalogV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/evidence-catalog.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = EVIDENCE_CATALOG_SCHEMA_VERSION
    job_id: str
    input_fingerprint: str
    state: EvidenceCatalogState
    unavailable_reason: str | None = None
    source_receipts: tuple[EvidenceSourceReceiptV1, ...] = ()
    items: tuple[EvidenceCatalogItemV1, ...] = ()

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("input_fingerprint")
    @classmethod
    def _valid_input_fingerprint(cls, value: str) -> str:
        return _sha256(value, label="input_fingerprint")

    @field_validator("unavailable_reason")
    @classmethod
    def _valid_unavailable_reason(cls, value: str | None) -> str | None:
        if value is not None and _REASON_ID_RE.fullmatch(value) is None:
            raise ValueError("unavailable_reason must be a lowercase reason identifier")
        return value

    @model_validator(mode="after")
    def _consistent_catalog(self) -> EvidenceCatalogV1:
        receipt_paths = tuple(receipt.path for receipt in self.source_receipts)
        _require_unique(receipt_paths, label="evidence source receipt paths")
        receipt_order = tuple(
            sorted(
                self.source_receipts,
                key=lambda receipt: (
                    {
                        "manifest": 0,
                        "profile_source": 1,
                        "generated_evidence": 2,
                    }[receipt.source_type],
                    receipt.path,
                ),
            )
        )
        if self.source_receipts != receipt_order:
            raise ValueError("evidence source receipts must use deterministic ordering")

        manifest_count = sum(
            receipt.source_type == "manifest" for receipt in self.source_receipts
        )
        if manifest_count > 1:
            raise ValueError("an evidence catalog may contain at most one manifest receipt")
        generated_receipts = tuple(
            receipt
            for receipt in self.source_receipts
            if receipt.source_type == "generated_evidence"
        )
        generated_paths = {receipt.path for receipt in generated_receipts}

        evidence_ids = tuple(item.evidence_id for item in self.items)
        _require_unique(evidence_ids, label="evidence catalog item IDs")
        if evidence_ids != tuple(sorted(evidence_ids)):
            raise ValueError("evidence catalog items must use deterministic evidence ID ordering")
        if any(item.path not in generated_paths for item in self.items):
            raise ValueError("evidence catalog item paths must resolve in generated source receipts")

        if self.state == "available":
            if not self.items or not generated_receipts:
                raise ValueError("an available evidence catalog requires generated items")
            if self.unavailable_reason is not None:
                raise ValueError("an available evidence catalog must not include unavailable_reason")
        elif self.state == "empty":
            if self.items or not generated_receipts:
                raise ValueError("an empty evidence catalog requires generated sources and no items")
            if any(receipt.item_count != 0 for receipt in generated_receipts):
                raise ValueError("an empty evidence catalog requires zero source items")
            if self.unavailable_reason is not None:
                raise ValueError("an empty evidence catalog must not include unavailable_reason")
        else:
            if self.items:
                raise ValueError("an unavailable evidence catalog must not include evidence items")
            if self.unavailable_reason is None:
                raise ValueError("an unavailable evidence catalog requires unavailable_reason")
        return self


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
    criteria_sha256: Sha256Value
    matches_sha256: Sha256Value
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
            "allOf": [
                {
                    "if": {"properties": {"decision": {"const": "undecided"}}},
                    "then": {
                        "properties": {
                            "confirmation_state": {"const": "unconfirmed"},
                            "confirmed_at": {"type": "null"},
                            "rationale": {"type": "null"},
                            "basis": {"type": "null"},
                        }
                    },
                    "else": {
                        "required": ["confirmation_state", "confirmed_at", "basis"],
                        "properties": {
                            "confirmation_state": {"const": "confirmed"},
                            "confirmed_at": {"not": {"type": "null"}},
                            "basis": {"not": {"type": "null"}},
                        },
                    },
                }
            ],
        },
    )

    schema_version: Literal["1.0.0"] = APPLICATION_DECISION_SCHEMA_VERSION
    job_id: JobIdentifier
    revision: UserRevision
    updated_at: UserControlTimestamp
    decision: DecisionValue = "undecided"
    confirmation_state: ConfirmationState = "unconfirmed"
    confirmed_at: UserControlTimestamp | None = None
    rationale: str | None = None
    basis: DecisionBasisV1 | None = None

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @model_validator(mode="after")
    def _consistent_decision(self) -> ApplicationDecisionV1:
        if self.decision == "undecided":
            if (
                self.confirmation_state != "unconfirmed"
                or self.confirmed_at is not None
                or self.rationale is not None
                or self.basis is not None
            ):
                raise ValueError(
                    "an undecided application must remain unconfirmed without rationale or a basis"
                )
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
