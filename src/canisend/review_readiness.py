from __future__ import annotations

from typing import Literal

from pydantic import ConfigDict, Field, field_validator, model_validator

from canisend.decision_models import (
    DecisionContractModel,
    DocumentIdentifier,
    DottedIdentifier,
    JSON_SCHEMA_DIALECT,
    JobIdentifier,
    SCHEMA_BASE_ID,
    Sha256Value,
    UserControlTimestamp,
    UserRevision,
)
from canisend.draft_models import (
    FindingIdentifier,
    ReviewFindingsV1,
)


REVIEW_DISPOSITIONS_SCHEMA_VERSION = "1.0.0"
DOCUMENT_READINESS_SCHEMA_VERSION = "1.0.0"

FindingDispositionValue = Literal["accepted", "revision_required"]
DocumentReadinessState = Literal[
    "blocked",
    "review_required",
    "revision_required",
    "reviewed",
]


class FindingDispositionV1(DecisionContractModel):
    finding_id: FindingIdentifier
    disposition: FindingDispositionValue
    decided_at: UserControlTimestamp


class ReviewDispositionsV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendReviewDispositionsV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/review-dispositions.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = REVIEW_DISPOSITIONS_SCHEMA_VERSION
    job_id: JobIdentifier
    document_id: DocumentIdentifier
    revision: UserRevision
    updated_at: UserControlTimestamp
    draft_sha256: Sha256Value
    review_findings_sha256: Sha256Value
    dispositions: tuple[FindingDispositionV1, ...] = Field(
        default=(),
        max_length=4_096,
        json_schema_extra={"uniqueItems": True},
    )

    @model_validator(mode="after")
    def _consistent_dispositions(self) -> ReviewDispositionsV1:
        finding_ids = tuple(item.finding_id for item in self.dispositions)
        if len(finding_ids) != len(set(finding_ids)):
            raise ValueError("review disposition finding IDs must be unique")
        if finding_ids != tuple(sorted(finding_ids)):
            raise ValueError("review dispositions must use stable finding-ID order")
        if any(item.decided_at > self.updated_at for item in self.dispositions):
            raise ValueError("a finding disposition cannot postdate the artifact update")
        return self


class DocumentReadinessV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendDocumentReadinessV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/document-readiness.schema.json",
            "allOf": [
                {
                    "if": {
                        "properties": {"state": {"const": "blocked"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {"blocker_finding_ids": {"minItems": 1}}
                    },
                },
                {
                    "if": {
                        "properties": {"state": {"const": "revision_required"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "blocker_finding_ids": {"maxItems": 0},
                            "revision_required_finding_ids": {"minItems": 1},
                        }
                    },
                },
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
                            "blocker_finding_ids": {"maxItems": 0},
                            "revision_required_finding_ids": {"maxItems": 0},
                            "unresolved_finding_ids": {"maxItems": 0},
                            "reason_codes": {"maxItems": 0},
                        }
                    },
                },
            ],
        },
    )

    schema_version: Literal["1.0.0"] = DOCUMENT_READINESS_SCHEMA_VERSION
    job_id: JobIdentifier
    document_id: DocumentIdentifier
    document_kind: Literal["cover_letter"] = "cover_letter"
    state: DocumentReadinessState
    draft_sha256: Sha256Value
    review_findings_sha256: Sha256Value
    review_dispositions_sha256: Sha256Value | None = None
    accepted_finding_ids: tuple[FindingIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    revision_required_finding_ids: tuple[FindingIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    unresolved_finding_ids: tuple[FindingIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    blocker_finding_ids: tuple[FindingIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    reason_codes: tuple[DottedIdentifier, ...] = Field(
        default=(), max_length=64, json_schema_extra={"uniqueItems": True}
    )

    @field_validator(
        "accepted_finding_ids",
        "revision_required_finding_ids",
        "unresolved_finding_ids",
        "blocker_finding_ids",
        "reason_codes",
    )
    @classmethod
    def _ordered_unique(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        if len(values) != len(set(values)):
            raise ValueError(f"{getattr(info, 'field_name', 'values')} must be unique")
        if values != tuple(sorted(values)):
            raise ValueError(f"{getattr(info, 'field_name', 'values')} must be ordered")
        return values

    @model_validator(mode="after")
    def _consistent_state(self) -> DocumentReadinessV1:
        groups = (
            set(self.accepted_finding_ids),
            set(self.revision_required_finding_ids),
            set(self.unresolved_finding_ids),
            set(self.blocker_finding_ids),
        )
        for index, values in enumerate(groups):
            if any(values & other for other in groups[index + 1 :]):
                raise ValueError("document readiness finding groups must be disjoint")

        if self.state == "blocked" and not self.blocker_finding_ids:
            raise ValueError("blocked document readiness requires blocker findings")
        if self.state == "revision_required":
            if self.blocker_finding_ids or not self.revision_required_finding_ids:
                raise ValueError(
                    "revision-required readiness needs a revision finding and no blocker"
                )
        if self.state == "reviewed":
            if (
                self.review_dispositions_sha256 is None
                or self.blocker_finding_ids
                or self.revision_required_finding_ids
                or self.unresolved_finding_ids
                or self.reason_codes
            ):
                raise ValueError(
                    "reviewed readiness requires current dispositions and no open reasons"
                )
        return self


def derive_document_readiness(
    review: ReviewFindingsV1,
    *,
    draft_sha256: str,
    review_findings_sha256: str,
    dispositions: ReviewDispositionsV1 | None,
    review_dispositions_sha256: str | None,
) -> DocumentReadinessV1:
    """Derive one fail-closed Cover Letter readiness projection."""

    finding_ids = tuple(item.finding_id for item in review.findings)
    finding_id_set = set(finding_ids)
    blockers = tuple(sorted(review.blocker_finding_ids))
    accepted: tuple[str, ...] = ()
    revisions: tuple[str, ...] = ()
    unresolved: tuple[str, ...] = tuple(sorted(finding_ids))
    reasons: set[str] = set()
    review_binds_draft = review.draft_sha256 == draft_sha256
    if not review_binds_draft:
        reasons.add("review.draft_receipt_mismatch")

    basis_current = bool(
        dispositions is not None
        and review_dispositions_sha256 is not None
        and review_binds_draft
        and dispositions.job_id == review.job_id
        and dispositions.document_id == review.document_id
        and dispositions.draft_sha256 == draft_sha256
        and dispositions.review_findings_sha256 == review_findings_sha256
    )

    if dispositions is None:
        reasons.add("review.dispositions_missing")
    elif not basis_current:
        reasons.add("review.dispositions_stale")
    else:
        disposition_by_id = {
            item.finding_id: item.disposition for item in dispositions.dispositions
        }
        orphaned = set(disposition_by_id) - finding_id_set
        if orphaned:
            reasons.add("review.disposition_orphaned")
        accepted = tuple(
            sorted(
                finding_id
                for finding_id, value in disposition_by_id.items()
                if finding_id in finding_id_set and value == "accepted"
            )
        )
        revisions = tuple(
            sorted(
                finding_id
                for finding_id, value in disposition_by_id.items()
                if finding_id in finding_id_set and value == "revision_required"
            )
        )
        unresolved = tuple(
            sorted(finding_id_set - set(accepted) - set(revisions))
        )
        if unresolved:
            reasons.add("review.finding_unresolved")

    accepted_blockers = tuple(sorted(set(accepted) & set(blockers)))
    if blockers:
        reasons.add("review.blocker_open")
        if accepted_blockers:
            reasons.add("review.blocker_nonwaivable")
        state: DocumentReadinessState = "blocked"
        accepted = tuple(item for item in accepted if item not in set(blockers))
        revisions = tuple(item for item in revisions if item not in set(blockers))
        unresolved = tuple(item for item in unresolved if item not in set(blockers))
    elif revisions:
        reasons.add("review.revision_required")
        state = "revision_required"
    elif reasons or unresolved:
        state = "review_required"
    else:
        state = "reviewed"

    return DocumentReadinessV1(
        job_id=review.job_id,
        document_id=review.document_id,
        state=state,
        draft_sha256=draft_sha256,
        review_findings_sha256=review_findings_sha256,
        review_dispositions_sha256=(
            review_dispositions_sha256 if basis_current else None
        ),
        accepted_finding_ids=accepted,
        revision_required_finding_ids=revisions,
        unresolved_finding_ids=unresolved,
        blocker_finding_ids=blockers,
        reason_codes=tuple(sorted(reasons)),
    )
