from __future__ import annotations

import hashlib
import json
import re
import unicodedata
from typing import Annotated, Literal

from pydantic import ConfigDict, Field, field_validator, model_validator

from canisend.decision_models import (
    BriefFieldName,
    CriterionIdentifier,
    DecisionContractModel,
    DocumentIdentifier,
    DottedIdentifier,
    EvidenceIdentifier,
    JobIdentifier,
    JSON_SCHEMA_DIALECT,
    SCHEMA_BASE_ID,
    Sha256Value,
    SlugIdentifier,
)


COVER_LETTER_DRAFT_SCHEMA_VERSION = "1.0.0"
REVIEW_FINDINGS_SCHEMA_VERSION = "1.0.0"

ClaimKind = Literal[
    "factual",
    "motivation",
    "future_intent",
    "role_context",
    "administrative",
]
ClaimSupportStrength = Literal["strong", "partial", "unsupported", "not_applicable"]
DraftGenerationMode = Literal["host_agent", "configured_provider"]
DraftReviewState = Literal["proposed"]
JobFieldName = Literal[
    "title",
    "institution",
    "department",
    "location",
    "deadline",
    "application_url",
]
FindingSeverity = Literal["blocker", "review", "warning"]
FindingCategory = Literal[
    "support",
    "contradiction",
    "consistency",
    "completeness",
    "compliance",
    "style",
]
FindingStatus = Literal["open"]

_CLAIM_ID_RE = re.compile(r"^claim_[0-9a-f]{32}$")
_FINDING_ID_RE = re.compile(r"^finding_[0-9a-f]{32}$")
_SEMVER_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")

ClaimIdentifier = Annotated[str, Field(pattern=_CLAIM_ID_RE.pattern)]
FindingIdentifier = Annotated[str, Field(pattern=_FINDING_ID_RE.pattern)]
SemanticVersion = Annotated[str, Field(pattern=_SEMVER_RE.pattern)]
BoundedBody = Annotated[
    str,
    Field(min_length=1, max_length=20_000, pattern=r"[\s\S]*\S[\s\S]*"),
]
BoundedFindingText = Annotated[
    str,
    Field(min_length=1, max_length=10_000, pattern=r"[\s\S]*\S[\s\S]*"),
]


class DraftBasisV1(DecisionContractModel):
    parsed_job_sha256: Sha256Value
    criteria_sha256: Sha256Value
    evidence_catalog_sha256: Sha256Value
    criterion_matches_sha256: Sha256Value
    application_decision_sha256: Sha256Value
    application_brief_sha256: Sha256Value
    required_document_plan_sha256: Sha256Value


class ClaimV1(DecisionContractModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "allOf": [
                {
                    "if": {
                        "properties": {"kind": {"const": "factual"}},
                        "required": ["kind"],
                    },
                    "then": {
                        "properties": {
                            "support_strength": {
                                "enum": ["strong", "partial", "unsupported"]
                            },
                            "brief_field_refs": {"maxItems": 0},
                            "job_field_refs": {"maxItems": 0},
                        }
                    },
                    "else": {
                        "properties": {
                            "support_strength": {"const": "not_applicable"},
                            "evidence_ref_ids": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {
                            "kind": {"const": "factual"},
                            "support_strength": {"const": "strong"},
                        },
                        "required": ["kind", "support_strength"],
                    },
                    "then": {
                        "properties": {
                            "evidence_ref_ids": {"minItems": 1},
                            "blockers": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {
                            "kind": {"const": "factual"},
                            "support_strength": {"const": "partial"},
                        },
                        "required": ["kind", "support_strength"],
                    },
                    "then": {
                        "properties": {
                            "evidence_ref_ids": {"minItems": 1},
                            "blockers": {
                                "minItems": 1,
                                "maxItems": 1,
                                "items": {"const": "claim.partial_support"},
                            },
                        }
                    },
                },
                {
                    "if": {
                        "properties": {
                            "kind": {"const": "factual"},
                            "support_strength": {"const": "unsupported"},
                        },
                        "required": ["kind", "support_strength"],
                    },
                    "then": {
                        "properties": {
                            "evidence_ref_ids": {"maxItems": 0},
                            "blockers": {
                                "minItems": 1,
                                "maxItems": 1,
                                "items": {"const": "claim.unsupported"},
                            },
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"kind": {"const": "motivation"}},
                        "required": ["kind"],
                    },
                    "then": {
                        "properties": {
                            "criterion_ids": {"maxItems": 0},
                            "brief_field_refs": {
                                "minItems": 1,
                                "maxItems": 1,
                                "items": {"const": "motivation"},
                            },
                            "job_field_refs": {"maxItems": 0},
                            "blockers": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"kind": {"const": "future_intent"}},
                        "required": ["kind"],
                    },
                    "then": {
                        "properties": {
                            "brief_field_refs": {"items": {"const": "emphasis"}},
                            "job_field_refs": {"maxItems": 0},
                            "blockers": {"maxItems": 0},
                        },
                        "anyOf": [
                            {
                                "required": ["criterion_ids"],
                                "properties": {"criterion_ids": {"minItems": 1}},
                            },
                            {
                                "required": ["brief_field_refs"],
                                "properties": {"brief_field_refs": {"minItems": 1}},
                            },
                        ],
                    },
                },
                {
                    "if": {
                        "properties": {"kind": {"const": "role_context"}},
                        "required": ["kind"],
                    },
                    "then": {
                        "properties": {
                            "brief_field_refs": {"maxItems": 0},
                            "blockers": {"maxItems": 0},
                        },
                        "anyOf": [
                            {
                                "required": ["criterion_ids"],
                                "properties": {"criterion_ids": {"minItems": 1}},
                            },
                            {
                                "required": ["job_field_refs"],
                                "properties": {"job_field_refs": {"minItems": 1}},
                            },
                        ],
                    },
                },
                {
                    "if": {
                        "properties": {"kind": {"const": "administrative"}},
                        "required": ["kind"],
                    },
                    "then": {
                        "properties": {
                            "criterion_ids": {"maxItems": 0},
                            "evidence_ref_ids": {"maxItems": 0},
                            "brief_field_refs": {"maxItems": 0},
                            "job_field_refs": {"maxItems": 0},
                            "blockers": {"maxItems": 0},
                        }
                    },
                },
            ]
        },
    )

    claim_id: ClaimIdentifier
    text: BoundedBody
    kind: ClaimKind
    support_strength: ClaimSupportStrength
    criterion_ids: tuple[CriterionIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    evidence_ref_ids: tuple[EvidenceIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    brief_field_refs: tuple[BriefFieldName, ...] = Field(
        default=(), max_length=6, json_schema_extra={"uniqueItems": True}
    )
    job_field_refs: tuple[JobFieldName, ...] = Field(
        default=(), max_length=6, json_schema_extra={"uniqueItems": True}
    )
    blockers: tuple[DottedIdentifier, ...] = Field(
        default=(), max_length=64, json_schema_extra={"uniqueItems": True}
    )
    review_state: DraftReviewState = "proposed"

    @field_validator("claim_id")
    @classmethod
    def _valid_claim_id(cls, value: str) -> str:
        if _CLAIM_ID_RE.fullmatch(value) is None:
            raise ValueError("claim_id is invalid")
        return value

    @field_validator(
        "criterion_ids",
        "evidence_ref_ids",
        "brief_field_refs",
        "job_field_refs",
        "blockers",
    )
    @classmethod
    def _ordered_unique_refs(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        _require_ordered_unique(values, label=getattr(info, "field_name", "references"))
        return values

    @model_validator(mode="after")
    def _consistent_support(self) -> ClaimV1:
        if self.kind == "factual":
            if self.support_strength == "not_applicable":
                raise ValueError("factual claims require an evidence support classification")
            if self.brief_field_refs or self.job_field_refs:
                raise ValueError(
                    "factual claims must not substitute Brief or job fields for Evidence"
                )
            if self.support_strength == "strong":
                if not self.evidence_ref_ids or self.blockers:
                    raise ValueError("strong factual claims require Evidence and no blockers")
            elif self.support_strength == "partial":
                if not self.evidence_ref_ids or self.blockers != ("claim.partial_support",):
                    raise ValueError(
                        "partial factual claims require Evidence and claim.partial_support"
                    )
            elif self.evidence_ref_ids or self.blockers != ("claim.unsupported",):
                raise ValueError(
                    "unsupported factual claims require no Evidence and claim.unsupported"
                )
            return self

        if self.support_strength != "not_applicable" or self.evidence_ref_ids:
            raise ValueError("non-factual claims use not_applicable and no applicant Evidence")
        if self.blockers:
            raise ValueError("non-factual claim basis errors must reject the candidate")

        if self.kind == "motivation":
            if (
                self.criterion_ids
                or self.brief_field_refs != ("motivation",)
                or self.job_field_refs
            ):
                raise ValueError("motivation claims require only the Brief motivation basis")
        elif self.kind == "future_intent":
            if self.job_field_refs:
                raise ValueError("future-intent claims must not use job fields as applicant proof")
            if any(value != "emphasis" for value in self.brief_field_refs):
                raise ValueError("future-intent Brief references may name only emphasis")
            if not self.criterion_ids and not self.brief_field_refs:
                raise ValueError("future-intent claims require a Criterion or Brief emphasis basis")
        elif self.kind == "role_context":
            if self.brief_field_refs or (not self.criterion_ids and not self.job_field_refs):
                raise ValueError("role-context claims require a Criterion or current job field")
        elif any(
            (
                self.criterion_ids,
                self.evidence_ref_ids,
                self.brief_field_refs,
                self.job_field_refs,
                self.blockers,
            )
        ):
            raise ValueError("administrative claims must not carry semantic references")
        return self


class DraftSectionV1(DecisionContractModel):
    section_id: SlugIdentifier
    heading: None = Field(
        default=None,
        description=(
            "Reserved for a future controlled projection; applicant-facing text must be a Claim."
        ),
    )
    claims: tuple[ClaimV1, ...] = Field(min_length=1, max_length=4_096)


class CoverLetterDraftV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendCoverLetterDraftV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/cover-letter-draft.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = COVER_LETTER_DRAFT_SCHEMA_VERSION
    job_id: JobIdentifier
    document_id: DocumentIdentifier
    input_fingerprint: Sha256Value
    basis: DraftBasisV1
    generation_mode: DraftGenerationMode
    generator_strategy: DottedIdentifier
    generator_version: SemanticVersion
    review_state: DraftReviewState = "proposed"
    sections: tuple[DraftSectionV1, ...] = Field(min_length=1, max_length=256)
    blockers: tuple[DottedIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )

    @field_validator("blockers")
    @classmethod
    def _ordered_unique_blockers(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_ordered_unique(values, label="draft blockers")
        return values

    @field_validator("generator_version")
    @classmethod
    def _valid_generator_version(cls, value: str) -> str:
        if _SEMVER_RE.fullmatch(value) is None:
            raise ValueError("generator_version must be semantic version text")
        return value

    @model_validator(mode="after")
    def _consistent_claim_graph(self) -> CoverLetterDraftV1:
        section_ids = tuple(section.section_id for section in self.sections)
        _require_unique(section_ids, label="draft section IDs")

        claims = tuple(claim for section in self.sections for claim in section.claims)
        claim_ids = tuple(claim.claim_id for claim in claims)
        _require_unique(claim_ids, label="draft claim IDs")
        if not any(claim.kind != "administrative" for claim in claims):
            raise ValueError("a Cover Letter Draft requires at least one substantive claim")

        for claim in claims:
            expected_id = stable_claim_id(
                job_id=self.job_id,
                document_id=self.document_id,
                kind=claim.kind,
                text=claim.text,
            )
            if claim.claim_id != expected_id:
                raise ValueError("claim_id does not match the normalized claim content")

        expected_blockers = tuple(sorted({code for claim in claims for code in claim.blockers}))
        if self.blockers != expected_blockers:
            raise ValueError("draft blockers must equal the sorted union of Claim blockers")
        return self


class ReviewFindingV1(DecisionContractModel):
    finding_id: FindingIdentifier
    code: DottedIdentifier
    severity: FindingSeverity
    category: FindingCategory
    message: BoundedFindingText
    next_action: BoundedFindingText
    claim_ids: tuple[ClaimIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    criterion_ids: tuple[CriterionIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    evidence_ref_ids: tuple[EvidenceIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    status: FindingStatus = "open"

    @field_validator("finding_id")
    @classmethod
    def _valid_finding_id(cls, value: str) -> str:
        if _FINDING_ID_RE.fullmatch(value) is None:
            raise ValueError("finding_id is invalid")
        return value

    @field_validator("claim_ids", "criterion_ids", "evidence_ref_ids")
    @classmethod
    def _ordered_unique_refs(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        _require_ordered_unique(values, label=getattr(info, "field_name", "references"))
        return values


class ReviewFindingsV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendReviewFindingsV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/review-findings.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = REVIEW_FINDINGS_SCHEMA_VERSION
    job_id: JobIdentifier
    document_id: DocumentIdentifier
    input_fingerprint: Sha256Value
    draft_sha256: Sha256Value
    reviewer_strategy: DottedIdentifier
    reviewer_version: SemanticVersion
    review_state: DraftReviewState = "proposed"
    findings: tuple[ReviewFindingV1, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    blocker_finding_ids: tuple[FindingIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )

    @field_validator("reviewer_version")
    @classmethod
    def _valid_reviewer_version(cls, value: str) -> str:
        if _SEMVER_RE.fullmatch(value) is None:
            raise ValueError("reviewer_version must be semantic version text")
        return value

    @field_validator("blocker_finding_ids")
    @classmethod
    def _ordered_unique_blocker_ids(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_ordered_unique(values, label="blocker finding IDs")
        return values

    @model_validator(mode="after")
    def _consistent_findings(self) -> ReviewFindingsV1:
        finding_ids = tuple(finding.finding_id for finding in self.findings)
        _require_ordered_unique(finding_ids, label="review findings")

        for finding in self.findings:
            expected_id = stable_finding_id(
                job_id=self.job_id,
                document_id=self.document_id,
                code=finding.code,
                message=finding.message,
                claim_ids=finding.claim_ids,
                criterion_ids=finding.criterion_ids,
                evidence_ref_ids=finding.evidence_ref_ids,
            )
            if finding.finding_id != expected_id:
                raise ValueError("finding_id does not match the normalized finding content")

        expected_blockers = tuple(
            finding.finding_id for finding in self.findings if finding.severity == "blocker"
        )
        if self.blocker_finding_ids != expected_blockers:
            raise ValueError("blocker_finding_ids must name exactly the blocker findings")
        return self


def stable_claim_id(
    *,
    job_id: str,
    document_id: str,
    kind: ClaimKind,
    text: str,
) -> str:
    return "claim_" + _stable_digest(
        {
            "document_id": document_id,
            "job_id": job_id,
            "kind": kind,
            "text": _normalize_semantic_text(text),
        }
    )[:32]


def stable_finding_id(
    *,
    job_id: str,
    document_id: str,
    code: str,
    message: str,
    claim_ids: tuple[str, ...] = (),
    criterion_ids: tuple[str, ...] = (),
    evidence_ref_ids: tuple[str, ...] = (),
) -> str:
    return "finding_" + _stable_digest(
        {
            "claim_ids": sorted(claim_ids),
            "code": code,
            "criterion_ids": sorted(criterion_ids),
            "document_id": document_id,
            "evidence_ref_ids": sorted(evidence_ref_ids),
            "job_id": job_id,
            "message": _normalize_semantic_text(message),
        }
    )[:32]


def _stable_digest(value: object) -> str:
    encoded = json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _normalize_semantic_text(value: str) -> str:
    normalized = unicodedata.normalize("NFKC", value)
    return " ".join(normalized.split()).casefold()


def _require_unique(values: tuple[str, ...], *, label: str) -> None:
    if len(values) != len(set(values)):
        raise ValueError(f"{label} must be unique")


def _require_ordered_unique(values: tuple[str, ...], *, label: str) -> None:
    _require_unique(values, label=label)
    if values != tuple(sorted(values)):
        raise ValueError(f"{label} must use stable lexical order")
