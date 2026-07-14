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
from canisend.draft_models import FindingIdentifier
from canisend.package_review_models import (
    PackageDocumentReviewV1,
    PackageReviewFindingsV1,
)
from canisend.review_readiness import FindingDispositionV1


PACKAGE_REVIEW_DISPOSITIONS_SCHEMA_VERSION = "1.0.0"
APPLICATION_PACKAGE_READINESS_SCHEMA_VERSION = "1.0.0"

ApplicationPackageReadinessState = Literal[
    "blocked",
    "review_required",
    "revision_required",
    "reviewed",
]


class PackageReviewDispositionsV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendPackageReviewDispositionsV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/package-review-dispositions.schema.json",
        },
    )

    schema_version: Literal["1.0.0"] = PACKAGE_REVIEW_DISPOSITIONS_SCHEMA_VERSION
    job_id: JobIdentifier
    revision: UserRevision
    updated_at: UserControlTimestamp
    package_review_findings_sha256: Sha256Value
    dispositions: tuple[FindingDispositionV1, ...] = Field(
        default=(),
        max_length=4_096,
        json_schema_extra={"uniqueItems": True},
    )

    @model_validator(mode="after")
    def _consistent_dispositions(self) -> PackageReviewDispositionsV1:
        finding_ids = tuple(item.finding_id for item in self.dispositions)
        _require_ordered_unique(finding_ids, label="package Review dispositions")
        if any(item.decided_at > self.updated_at for item in self.dispositions):
            raise ValueError("a package finding disposition cannot postdate the update")
        return self


class ApplicationPackageReadinessV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendApplicationPackageReadinessV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/application-package-readiness.schema.json",
            "allOf": [
                {
                    "if": {
                        "properties": {"state": {"const": "blocked"}},
                        "required": ["state"],
                    },
                    "then": {
                        "anyOf": [
                            {"properties": {"blocker_finding_ids": {"minItems": 1}}},
                            {
                                "properties": {
                                    "reason_codes": {
                                        "contains": {
                                            "const": "package.required_document_not_reviewed"
                                        }
                                    }
                                }
                            },
                        ]
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
                            "package_review_dispositions_sha256": {
                                "not": {"type": "null"}
                            },
                            "revision_required_finding_ids": {"maxItems": 0},
                            "unresolved_finding_ids": {"maxItems": 0},
                            "blocker_finding_ids": {"maxItems": 0},
                            "reason_codes": {"maxItems": 0},
                        }
                    },
                },
            ],
        },
    )

    schema_version: Literal["1.0.0"] = APPLICATION_PACKAGE_READINESS_SCHEMA_VERSION
    job_id: JobIdentifier
    state: ApplicationPackageReadinessState
    required_document_plan_sha256: Sha256Value
    document_execution_plan_sha256: Sha256Value
    package_review_findings_sha256: Sha256Value
    package_review_dispositions_sha256: Sha256Value | None = None
    required_documents: tuple[PackageDocumentReviewV1, ...] = Field(
        default=(),
        max_length=4_096,
        json_schema_extra={"uniqueItems": True},
    )
    required_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    reviewed_required_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    optional_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
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
        "required_document_ids",
        "reviewed_required_document_ids",
        "optional_document_ids",
        "accepted_finding_ids",
        "revision_required_finding_ids",
        "unresolved_finding_ids",
        "blocker_finding_ids",
        "reason_codes",
    )
    @classmethod
    def _ordered_unique(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        _require_ordered_unique(values, label=getattr(info, "field_name", "values"))
        return values

    @model_validator(mode="after")
    def _consistent_readiness(self) -> ApplicationPackageReadinessV1:
        document_ids = tuple(item.document_id for item in self.required_documents)
        _require_ordered_unique(document_ids, label="required package documents")
        if any(item.requirement != "required" for item in self.required_documents):
            raise ValueError("required package document receipts must be required")
        if self.required_document_ids != document_ids:
            raise ValueError("required document IDs must exactly project receipts")
        expected_reviewed = tuple(
            item.document_id
            for item in self.required_documents
            if item.state == "reviewed"
        )
        if self.reviewed_required_document_ids != expected_reviewed:
            raise ValueError("reviewed required IDs must exactly project receipts")
        if set(self.optional_document_ids) & set(self.required_document_ids):
            raise ValueError("optional and required document IDs must be disjoint")

        groups = (
            set(self.accepted_finding_ids),
            set(self.revision_required_finding_ids),
            set(self.unresolved_finding_ids),
            set(self.blocker_finding_ids),
        )
        for index, values in enumerate(groups):
            if any(values & other for other in groups[index + 1 :]):
                raise ValueError("package readiness finding groups must be disjoint")

        required_incomplete = (
            self.required_document_ids != self.reviewed_required_document_ids
        )
        if self.state == "blocked" and not (
            self.blocker_finding_ids or required_incomplete
        ):
            raise ValueError("blocked package readiness requires a proven blocker")
        if self.state == "revision_required" and (
            self.blocker_finding_ids or not self.revision_required_finding_ids
        ):
            raise ValueError(
                "revision-required package readiness needs revisions and no blocker"
            )
        if self.state == "reviewed" and (
            self.package_review_dispositions_sha256 is None
            or required_incomplete
            or self.revision_required_finding_ids
            or self.unresolved_finding_ids
            or self.blocker_finding_ids
            or self.reason_codes
        ):
            raise ValueError(
                "reviewed package readiness requires current complete exact receipts"
            )
        return self


def derive_application_package_readiness(
    review: PackageReviewFindingsV1,
    *,
    package_review_findings_sha256: str,
    dispositions: PackageReviewDispositionsV1 | None,
    package_review_dispositions_sha256: str | None,
) -> ApplicationPackageReadinessV1:
    """Derive one fail-closed aggregate readiness projection from exact receipts."""

    finding_ids = tuple(item.finding_id for item in review.findings)
    finding_id_set = set(finding_ids)
    blockers = tuple(sorted(review.blocker_finding_ids))
    accepted: tuple[str, ...] = ()
    revisions: tuple[str, ...] = ()
    unresolved: tuple[str, ...] = tuple(sorted(finding_id_set - set(blockers)))
    reasons: set[str] = set()

    basis_current = bool(
        dispositions is not None
        and package_review_dispositions_sha256 is not None
        and dispositions.job_id == review.job_id
        and dispositions.package_review_findings_sha256
        == package_review_findings_sha256
    )
    if dispositions is None:
        reasons.add("package.dispositions_missing")
    elif not basis_current:
        reasons.add("package.dispositions_stale")
    else:
        disposition_by_id = {
            item.finding_id: item.disposition for item in dispositions.dispositions
        }
        if set(disposition_by_id) - finding_id_set:
            reasons.add("package.disposition_orphaned")
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
                if finding_id in finding_id_set
                and value == "revision_required"
            )
        )
        unresolved = tuple(
            sorted(
                finding_id_set
                - set(accepted)
                - set(revisions)
                - set(blockers)
            )
        )
        if unresolved:
            reasons.add("package.finding_unresolved")

    accepted_blockers = set(accepted) & set(blockers)
    if accepted_blockers:
        reasons.add("package.blocker_nonwaivable")
        accepted = tuple(item for item in accepted if item not in accepted_blockers)
    revisions = tuple(item for item in revisions if item not in set(blockers))

    required_documents = tuple(
        item for item in review.documents if item.requirement == "required"
    )
    required_document_ids = tuple(item.document_id for item in required_documents)
    reviewed_required_document_ids = tuple(
        item.document_id for item in required_documents if item.state == "reviewed"
    )
    required_incomplete = required_document_ids != reviewed_required_document_ids
    if required_incomplete:
        reasons.add("package.required_document_not_reviewed")
    if blockers:
        reasons.add("package.blocker_open")

    if required_incomplete or blockers:
        state: ApplicationPackageReadinessState = "blocked"
    elif revisions:
        reasons.add("package.revision_required")
        state = "revision_required"
    elif reasons or unresolved:
        state = "review_required"
    else:
        state = "reviewed"

    return ApplicationPackageReadinessV1(
        job_id=review.job_id,
        state=state,
        required_document_plan_sha256=review.required_document_plan_sha256,
        document_execution_plan_sha256=review.document_execution_plan_sha256,
        package_review_findings_sha256=package_review_findings_sha256,
        package_review_dispositions_sha256=(
            package_review_dispositions_sha256 if basis_current else None
        ),
        required_documents=required_documents,
        required_document_ids=required_document_ids,
        reviewed_required_document_ids=reviewed_required_document_ids,
        optional_document_ids=tuple(
            item.document_id
            for item in review.documents
            if item.requirement == "optional"
        ),
        accepted_finding_ids=accepted,
        revision_required_finding_ids=revisions,
        unresolved_finding_ids=unresolved,
        blocker_finding_ids=blockers,
        reason_codes=tuple(sorted(reasons)),
    )


def _require_ordered_unique(values: tuple[str, ...], *, label: str) -> None:
    if len(values) != len(set(values)):
        raise ValueError(f"{label} must be unique")
    if values != tuple(sorted(values)):
        raise ValueError(f"{label} must use stable lexical order")
