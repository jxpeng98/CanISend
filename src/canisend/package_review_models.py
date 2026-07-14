from __future__ import annotations

import hashlib
import json
import re
import unicodedata
from typing import Annotated, Literal

from pydantic import ConfigDict, Field, field_validator, model_validator

from canisend.decision_models import (
    DecisionContractModel,
    DocumentAction,
    DocumentIdentifier,
    DocumentRequirement,
    DottedIdentifier,
    EvidenceIdentifier,
    JobIdentifier,
    JSON_SCHEMA_DIALECT,
    SCHEMA_BASE_ID,
    Sha256Value,
    SlugIdentifier,
)
from canisend.draft_models import (
    BoundedFindingText,
    ClaimIdentifier,
    FindingCategory,
    FindingIdentifier,
    FindingSeverity,
    FindingStatus,
    SemanticVersion,
)
from canisend.review_readiness import DocumentReadinessState


PACKAGE_REVIEW_FINDINGS_SCHEMA_VERSION = "1.0.0"

PackageDocumentState = Literal[
    "omitted",
    "plan_blocked",
    "executor_unavailable",
    "draft_missing",
    "draft_not_current",
    "review_missing",
    "review_not_current",
    "blocked",
    "review_required",
    "revision_required",
    "reviewed",
]
PackageExecutorAvailability = Literal["available", "planned", "unregistered"]
PackageReviewState = Literal["proposed"]
CorrectionApplicationRoute = Literal["guarded_draft_candidate"]

_PROPOSAL_ID_RE = re.compile(r"^proposal_[0-9a-f]{32}$")
ProposalIdentifier = Annotated[str, Field(pattern=_PROPOSAL_ID_RE.pattern)]


_NULL_RECEIPTS = {
    "draft_sha256": {"type": "null"},
    "review_findings_sha256": {"type": "null"},
    "review_dispositions_sha256": {"type": "null"},
    "document_readiness_sha256": {"type": "null"},
    "readiness_state": {"type": "null"},
}
_DRAFT_ONLY_RECEIPTS = {
    "draft_sha256": {"not": {"type": "null"}},
    "review_findings_sha256": {"type": "null"},
    "review_dispositions_sha256": {"type": "null"},
    "document_readiness_sha256": {"type": "null"},
    "readiness_state": {"type": "null"},
}


class PackageDocumentReviewV1(DecisionContractModel):
    model_config = ConfigDict(
        json_schema_extra={
            "allOf": [
                {
                    "if": {
                        "properties": {"action": {"const": "omit"}},
                        "required": ["action"],
                    },
                    "then": {"properties": {"state": {"const": "omitted"}}},
                },
                {
                    "if": {
                        "properties": {"state": {"const": "omitted"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "action": {"const": "omit"},
                            **_NULL_RECEIPTS,
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"state": {"const": "plan_blocked"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "action": {"const": "prepare"},
                            **_NULL_RECEIPTS,
                        }
                    },
                },
                {
                    "if": {
                        "properties": {
                            "state": {"const": "executor_unavailable"}
                        },
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "action": {"const": "prepare"},
                            "executor_availability": {
                                "enum": ["planned", "unregistered"]
                            },
                            **_NULL_RECEIPTS,
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"state": {"const": "draft_missing"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "action": {"const": "prepare"},
                            "executor_availability": {"const": "available"},
                            **_NULL_RECEIPTS,
                        }
                    },
                },
                {
                    "if": {
                        "properties": {
                            "state": {
                                "enum": ["draft_not_current", "review_missing"]
                            }
                        },
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "action": {"const": "prepare"},
                            "executor_availability": {"const": "available"},
                            **_DRAFT_ONLY_RECEIPTS,
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"state": {"const": "review_not_current"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "action": {"const": "prepare"},
                            "executor_availability": {"const": "available"},
                            "draft_sha256": {"not": {"type": "null"}},
                            "review_findings_sha256": {
                                "not": {"type": "null"}
                            },
                            "review_dispositions_sha256": {"type": "null"},
                            "document_readiness_sha256": {"type": "null"},
                            "readiness_state": {"type": "null"},
                        }
                    },
                },
                *[
                    {
                        "if": {
                            "properties": {"state": {"const": state}},
                            "required": ["state"],
                        },
                        "then": {
                            "properties": {
                                "action": {"const": "prepare"},
                                "executor_availability": {"const": "available"},
                                "draft_sha256": {"not": {"type": "null"}},
                                "review_findings_sha256": {
                                    "not": {"type": "null"}
                                },
                                "document_readiness_sha256": {
                                    "not": {"type": "null"}
                                },
                                "readiness_state": {"const": state},
                            }
                        },
                    }
                    for state in (
                        "blocked",
                        "review_required",
                        "revision_required",
                        "reviewed",
                    )
                ],
                {
                    "if": {
                        "properties": {"state": {"const": "reviewed"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "review_dispositions_sha256": {
                                "not": {"type": "null"}
                            },
                            "reason_codes": {"maxItems": 0},
                        }
                    },
                    "else": {"properties": {"reason_codes": {"minItems": 1}}},
                },
            ]
        }
    )

    document_id: DocumentIdentifier
    normalized_kind: SlugIdentifier
    requirement: DocumentRequirement
    action: DocumentAction
    executor_availability: PackageExecutorAvailability
    state: PackageDocumentState
    draft_sha256: Sha256Value | None = None
    review_findings_sha256: Sha256Value | None = None
    review_dispositions_sha256: Sha256Value | None = None
    document_readiness_sha256: Sha256Value | None = None
    readiness_state: DocumentReadinessState | None = None
    claim_ids: tuple[ClaimIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    reason_codes: tuple[DottedIdentifier, ...] = Field(
        default=(), max_length=64, json_schema_extra={"uniqueItems": True}
    )

    @field_validator("claim_ids", "reason_codes")
    @classmethod
    def _ordered_unique(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        _require_ordered_unique(values, label=getattr(info, "field_name", "values"))
        return values

    @model_validator(mode="after")
    def _consistent_state(self) -> PackageDocumentReviewV1:
        downstream = (
            self.review_findings_sha256,
            self.review_dispositions_sha256,
            self.document_readiness_sha256,
            self.readiness_state,
        )
        if self.action == "omit":
            if self.state != "omitted" or any(
                value is not None for value in (self.draft_sha256, *downstream)
            ):
                raise ValueError("an omitted package document must not claim review receipts")
        elif self.state == "plan_blocked":
            if any(value is not None for value in (self.draft_sha256, *downstream)):
                raise ValueError("a plan-blocked document must not claim review receipts")
        elif self.executor_availability != "available":
            if self.state != "executor_unavailable" or any(
                value is not None for value in (self.draft_sha256, *downstream)
            ):
                raise ValueError("an unavailable executor must not claim document receipts")
        elif self.state == "draft_missing":
            if self.draft_sha256 is not None or any(value is not None for value in downstream):
                raise ValueError("a missing Draft must not claim downstream receipts")
        elif self.state == "draft_not_current":
            if self.draft_sha256 is None or any(value is not None for value in downstream):
                raise ValueError("a non-current Draft binds only its observed Draft hash")
        elif self.state == "review_missing":
            if self.draft_sha256 is None or any(value is not None for value in downstream):
                raise ValueError("a missing Review binds only the current Draft hash")
        elif self.state == "review_not_current":
            if (
                self.draft_sha256 is None
                or self.review_findings_sha256 is None
                or any(value is not None for value in downstream[1:])
            ):
                raise ValueError("a non-current Review binds the observed Draft and Review hashes")
        else:
            if (
                self.draft_sha256 is None
                or self.review_findings_sha256 is None
                or self.document_readiness_sha256 is None
                or self.readiness_state != self.state
            ):
                raise ValueError("a reviewed package document requires exact readiness receipts")
            if self.state == "reviewed" and (
                self.review_dispositions_sha256 is None or self.reason_codes
            ):
                raise ValueError("reviewed package documents require dispositions and no reasons")
        if self.state != "reviewed" and not self.reason_codes:
            raise ValueError("a non-reviewed package document requires a reason code")
        return self


class PackageCorrectionProposalV1(DecisionContractModel):
    proposal_id: ProposalIdentifier
    document_id: DocumentIdentifier
    claim_ids: tuple[ClaimIdentifier, ...] = Field(
        min_length=1, max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    reason_code: DottedIdentifier
    instruction: BoundedFindingText
    application_route: CorrectionApplicationRoute = "guarded_draft_candidate"

    @field_validator("claim_ids")
    @classmethod
    def _ordered_unique_claims(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_ordered_unique(values, label="correction proposal Claim IDs")
        return values


class PackageReviewFindingV1(DecisionContractModel):
    finding_id: FindingIdentifier
    code: DottedIdentifier
    severity: FindingSeverity
    category: FindingCategory
    message: BoundedFindingText
    next_action: BoundedFindingText
    document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    claim_ids: tuple[ClaimIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    evidence_ref_ids: tuple[EvidenceIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    correction_proposal_ids: tuple[ProposalIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    status: FindingStatus = "open"

    @field_validator(
        "document_ids",
        "claim_ids",
        "evidence_ref_ids",
        "correction_proposal_ids",
    )
    @classmethod
    def _ordered_unique_refs(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        _require_ordered_unique(values, label=getattr(info, "field_name", "references"))
        return values


class PackageReviewFindingsV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendPackageReviewFindingsV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/package-review-findings.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = PACKAGE_REVIEW_FINDINGS_SCHEMA_VERSION
    job_id: JobIdentifier
    input_fingerprint: Sha256Value
    parsed_job_sha256: Sha256Value
    application_brief_sha256: Sha256Value
    required_document_plan_sha256: Sha256Value
    document_execution_plan_sha256: Sha256Value
    reviewer_strategy: DottedIdentifier
    reviewer_version: SemanticVersion
    review_state: PackageReviewState = "proposed"
    documents: tuple[PackageDocumentReviewV1, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    required_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    selected_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    reviewed_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    findings: tuple[PackageReviewFindingV1, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    correction_proposals: tuple[PackageCorrectionProposalV1, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    blocker_finding_ids: tuple[FindingIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )

    @field_validator(
        "required_document_ids",
        "selected_document_ids",
        "reviewed_document_ids",
        "blocker_finding_ids",
    )
    @classmethod
    def _ordered_unique_ids(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        _require_ordered_unique(values, label=getattr(info, "field_name", "identifiers"))
        return values

    @model_validator(mode="after")
    def _consistent_aggregate(self) -> PackageReviewFindingsV1:
        document_ids = tuple(item.document_id for item in self.documents)
        _require_ordered_unique(document_ids, label="package Review documents")
        expected_required = tuple(
            item.document_id for item in self.documents if item.requirement == "required"
        )
        expected_selected = tuple(
            item.document_id for item in self.documents if item.action == "prepare"
        )
        expected_reviewed = tuple(
            item.document_id for item in self.documents if item.state == "reviewed"
        )
        if self.required_document_ids != expected_required:
            raise ValueError("required_document_ids must exactly project document requirements")
        if self.selected_document_ids != expected_selected:
            raise ValueError("selected_document_ids must exactly project prepare actions")
        if self.reviewed_document_ids != expected_reviewed:
            raise ValueError("reviewed_document_ids must exactly project reviewed documents")

        known_documents = set(document_ids)
        known_claims = {claim_id for item in self.documents for claim_id in item.claim_ids}
        proposal_ids = tuple(item.proposal_id for item in self.correction_proposals)
        _require_ordered_unique(proposal_ids, label="package correction proposals")
        for proposal in self.correction_proposals:
            if proposal.document_id not in known_documents:
                raise ValueError("a correction proposal must target a bound document")
            if not set(proposal.claim_ids).issubset(known_claims):
                raise ValueError("a correction proposal must target bound Claims")
            expected_id = stable_package_proposal_id(
                job_id=self.job_id,
                document_id=proposal.document_id,
                claim_ids=proposal.claim_ids,
                reason_code=proposal.reason_code,
                instruction=proposal.instruction,
            )
            if proposal.proposal_id != expected_id:
                raise ValueError("proposal_id does not match normalized proposal content")

        finding_ids = tuple(item.finding_id for item in self.findings)
        _require_ordered_unique(finding_ids, label="package Review findings")
        known_proposals = set(proposal_ids)
        for finding in self.findings:
            if not set(finding.document_ids).issubset(known_documents):
                raise ValueError("package finding document references must resolve")
            if not set(finding.claim_ids).issubset(known_claims):
                raise ValueError("package finding Claim references must resolve")
            if not set(finding.correction_proposal_ids).issubset(known_proposals):
                raise ValueError("package finding correction proposals must resolve")
            expected_id = stable_package_finding_id(
                job_id=self.job_id,
                code=finding.code,
                message=finding.message,
                document_ids=finding.document_ids,
                claim_ids=finding.claim_ids,
                evidence_ref_ids=finding.evidence_ref_ids,
                correction_proposal_ids=finding.correction_proposal_ids,
            )
            if finding.finding_id != expected_id:
                raise ValueError("finding_id does not match normalized package finding content")

        expected_blockers = tuple(
            item.finding_id for item in self.findings if item.severity == "blocker"
        )
        if self.blocker_finding_ids != expected_blockers:
            raise ValueError("blocker_finding_ids must name exactly the package blockers")
        return self


def stable_package_finding_id(
    *,
    job_id: str,
    code: str,
    message: str,
    document_ids: tuple[str, ...] = (),
    claim_ids: tuple[str, ...] = (),
    evidence_ref_ids: tuple[str, ...] = (),
    correction_proposal_ids: tuple[str, ...] = (),
) -> str:
    return "finding_" + _stable_digest(
        {
            "claim_ids": sorted(claim_ids),
            "code": code,
            "correction_proposal_ids": sorted(correction_proposal_ids),
            "document_ids": sorted(document_ids),
            "evidence_ref_ids": sorted(evidence_ref_ids),
            "job_id": job_id,
            "message": _normalize_semantic_text(message),
        }
    )[:32]


def stable_package_proposal_id(
    *,
    job_id: str,
    document_id: str,
    claim_ids: tuple[str, ...],
    reason_code: str,
    instruction: str,
) -> str:
    return "proposal_" + _stable_digest(
        {
            "claim_ids": sorted(claim_ids),
            "document_id": document_id,
            "instruction": _normalize_semantic_text(instruction),
            "job_id": job_id,
            "reason_code": reason_code,
        }
    )[:32]


def canonical_sha256(value: object) -> str:
    return _stable_digest(value)


def normalize_claim_text(value: str) -> str:
    return _normalize_semantic_text(value)


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


def _require_ordered_unique(values: tuple[str, ...], *, label: str) -> None:
    if len(values) != len(set(values)):
        raise ValueError(f"{label} must be unique")
    if values != tuple(sorted(values)):
        raise ValueError(f"{label} must use stable lexical order")
