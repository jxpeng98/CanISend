from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
from typing import Literal

from pydantic import ConfigDict, Field, ValidationError, field_validator, model_validator

from canisend.agent_protocol import (
    AgentResponse,
    NextAction,
    WorkflowSnapshotReference,
    artifact_reference_from_path,
    success_response,
)
from canisend.decision_models import (
    ConfirmationState,
    DecisionContractModel,
    DocumentAction,
    DocumentIdentifier,
    DocumentRequirement,
    DottedIdentifier,
    JSON_SCHEMA_DIALECT,
    JobIdentifier,
    RequiredDocumentPlanV1,
    SCHEMA_BASE_ID,
    Sha256Value,
    SlugIdentifier,
)
from canisend.stage_store import StageStoreError, read_json_object, sha256_file


DOCUMENT_EXECUTION_PLAN_SCHEMA_VERSION = "1.0.0"
REQUIRED_DOCUMENT_PLAN_PATH = "required_document_plan.json"

DocumentExecutorAvailability = Literal["available", "planned", "unregistered"]
DocumentExecutorScope = Literal["submission_document", "workflow_support"]
DocumentExecutionMode = Literal["host_agent", "configured_provider"]
DocumentWorkItemState = Literal[
    "blocked",
    "omitted",
    "ready_to_prepare",
    "executor_unavailable",
]
DocumentExecutionPlanState = Literal[
    "blocked",
    "partially_dispatchable",
    "ready",
    "no_work",
]
DocumentExecutionSourceState = Literal["current", "missing", "not_current", "invalid"]

_OUTPUT_SCHEMA_RE = re.compile(r"^canisend\.[a-z0-9][a-z0-9_.-]*/v[1-9][0-9]*$")


class DocumentExecutorCapabilityV1(DecisionContractModel):
    normalized_kind: SlugIdentifier
    scope: DocumentExecutorScope
    route_id: DottedIdentifier
    availability: Literal["available", "planned"]
    executor_id: DottedIdentifier | None = None
    stage: Literal["draft"] | None = None
    authoritative_target: str | None = None
    output_schema: str | None = None
    execution_modes: tuple[DocumentExecutionMode, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    max_instances: int = Field(default=1, ge=1, le=4_096)

    @field_validator("authoritative_target")
    @classmethod
    def _valid_target(cls, value: str | None) -> str | None:
        return _safe_relative_path(value) if value is not None else None

    @field_validator("output_schema")
    @classmethod
    def _valid_output_schema(cls, value: str | None) -> str | None:
        if value is not None and _OUTPUT_SCHEMA_RE.fullmatch(value) is None:
            raise ValueError("output_schema must be a versioned CanISend schema identifier")
        return value

    @field_validator("execution_modes")
    @classmethod
    def _unique_modes(
        cls, values: tuple[DocumentExecutionMode, ...]
    ) -> tuple[DocumentExecutionMode, ...]:
        if len(values) != len(set(values)):
            raise ValueError("document execution modes must be unique")
        return values

    @model_validator(mode="after")
    def _consistent_availability(self) -> DocumentExecutorCapabilityV1:
        execution_fields = (
            self.executor_id,
            self.stage,
            self.authoritative_target,
            self.output_schema,
        )
        if self.availability == "available":
            if any(value is None for value in execution_fields) or not self.execution_modes:
                raise ValueError("an available document executor requires a complete execution contract")
        elif any(value is not None for value in execution_fields) or self.execution_modes:
            raise ValueError("a planned document route must not claim an execution contract")
        return self


class DocumentWorkItemV1(DecisionContractModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "allOf": [
                {
                    "if": {"properties": {"state": {"const": "blocked"}}, "required": ["state"]},
                    "then": {"properties": {"reason_codes": {"minItems": 1}}},
                },
                {
                    "if": {"properties": {"state": {"const": "omitted"}}, "required": ["state"]},
                    "then": {
                        "properties": {
                            "action": {"const": "omit"},
                            "confirmation_state": {"const": "confirmed"},
                            "reason_codes": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"state": {"const": "ready_to_prepare"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "action": {"const": "prepare"},
                            "confirmation_state": {"const": "confirmed"},
                            "executor_availability": {"const": "available"},
                            "reason_codes": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"state": {"const": "executor_unavailable"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "action": {"const": "prepare"},
                            "confirmation_state": {"const": "confirmed"},
                            "executor_availability": {"enum": ["planned", "unregistered"]},
                            "reason_codes": {"minItems": 1},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"executor_availability": {"const": "available"}},
                        "required": ["executor_availability"],
                    },
                    "then": {
                        "properties": {
                            "route_id": {"not": {"type": "null"}},
                            "executor_id": {"not": {"type": "null"}},
                            "stage": {"not": {"type": "null"}},
                            "authoritative_target": {"not": {"type": "null"}},
                            "output_schema": {"not": {"type": "null"}},
                            "execution_modes": {"minItems": 1},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"executor_availability": {"const": "planned"}},
                        "required": ["executor_availability"],
                    },
                    "then": {
                        "properties": {
                            "route_id": {"not": {"type": "null"}},
                            "executor_id": {"type": "null"},
                            "stage": {"type": "null"},
                            "authoritative_target": {"type": "null"},
                            "output_schema": {"type": "null"},
                            "execution_modes": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"executor_availability": {"const": "unregistered"}},
                        "required": ["executor_availability"],
                    },
                    "then": {
                        "properties": {
                            "route_id": {"type": "null"},
                            "executor_id": {"type": "null"},
                            "stage": {"type": "null"},
                            "authoritative_target": {"type": "null"},
                            "output_schema": {"type": "null"},
                            "execution_modes": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"action": {"const": "needs_confirmation"}},
                        "required": ["action"],
                    },
                    "then": {
                        "properties": {
                            "confirmation_state": {"const": "unconfirmed"},
                            "state": {"const": "blocked"},
                        }
                    },
                    "else": {
                        "properties": {"confirmation_state": {"const": "confirmed"}}
                    },
                },
            ]
        },
    )

    document_id: DocumentIdentifier
    normalized_kind: SlugIdentifier
    requirement: DocumentRequirement
    action: DocumentAction
    confirmation_state: ConfirmationState
    state: DocumentWorkItemState
    executor_scope: DocumentExecutorScope = "submission_document"
    executor_availability: DocumentExecutorAvailability
    route_id: DottedIdentifier | None = None
    executor_id: DottedIdentifier | None = None
    stage: Literal["draft"] | None = None
    authoritative_target: str | None = None
    output_schema: str | None = None
    execution_modes: tuple[DocumentExecutionMode, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    reason_codes: tuple[DottedIdentifier, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )

    @field_validator("authoritative_target")
    @classmethod
    def _valid_target(cls, value: str | None) -> str | None:
        return _safe_relative_path(value) if value is not None else None

    @field_validator("output_schema")
    @classmethod
    def _valid_output_schema(cls, value: str | None) -> str | None:
        if value is not None and _OUTPUT_SCHEMA_RE.fullmatch(value) is None:
            raise ValueError("output_schema must be a versioned CanISend schema identifier")
        return value

    @field_validator("execution_modes", "reason_codes")
    @classmethod
    def _ordered_unique(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        if len(values) != len(set(values)):
            raise ValueError(f"{getattr(info, 'field_name', 'values')} must be unique")
        if values != tuple(sorted(values)):
            raise ValueError(f"{getattr(info, 'field_name', 'values')} must be ordered")
        return values

    @model_validator(mode="after")
    def _consistent_state(self) -> DocumentWorkItemV1:
        execution_fields = (
            self.executor_id,
            self.stage,
            self.authoritative_target,
            self.output_schema,
        )
        if self.executor_availability == "available":
            if (
                self.route_id is None
                or any(value is None for value in execution_fields)
                or not self.execution_modes
            ):
                raise ValueError("an available work item requires a complete execution contract")
        elif self.executor_availability == "planned":
            if (
                self.route_id is None
                or any(value is not None for value in execution_fields)
                or self.execution_modes
            ):
                raise ValueError("a planned work item must not claim an execution contract")
        elif (
            self.route_id is not None
            or any(value is not None for value in execution_fields)
            or self.execution_modes
        ):
            raise ValueError("an unregistered work item must not claim a route or execution contract")

        if self.state == "blocked":
            if not self.reason_codes:
                raise ValueError("a blocked work item requires a reason code")
        elif self.state == "omitted":
            if (
                self.action != "omit"
                or self.confirmation_state != "confirmed"
                or self.reason_codes
            ):
                raise ValueError("an omitted work item requires one confirmed omit action")
        elif self.state == "ready_to_prepare":
            if (
                self.action != "prepare"
                or self.confirmation_state != "confirmed"
                or self.executor_availability != "available"
                or self.reason_codes
            ):
                raise ValueError("a ready work item requires one available confirmed executor")
        elif (
            self.action != "prepare"
            or self.confirmation_state != "confirmed"
            or self.executor_availability not in {"planned", "unregistered"}
            or not self.reason_codes
        ):
            raise ValueError("an unavailable work item requires one confirmed prepare action")

        if self.action == "needs_confirmation" and (
            self.confirmation_state != "unconfirmed" or self.state != "blocked"
        ):
            raise ValueError("an unresolved document action must remain blocked and unconfirmed")
        if self.action != "needs_confirmation" and self.confirmation_state != "confirmed":
            raise ValueError("a resolved document action must remain confirmed")
        return self


class DocumentExecutionPlanV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendDocumentExecutionPlanV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/document-execution-plan.schema.json",
            "allOf": [
                {
                    "if": {"properties": {"state": {"const": "ready"}}, "required": ["state"]},
                    "then": {
                        "properties": {
                            "ready_document_ids": {"minItems": 1},
                            "blocked_document_ids": {"maxItems": 0},
                            "executor_unavailable_document_ids": {"maxItems": 0},
                            "source_blockers": {"maxItems": 0},
                            "blockers": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {"state": {"const": "partially_dispatchable"}},
                        "required": ["state"],
                    },
                    "then": {
                        "properties": {
                            "ready_document_ids": {"minItems": 1},
                            "blocked_document_ids": {"maxItems": 0},
                            "executor_unavailable_document_ids": {"minItems": 1},
                            "source_blockers": {"maxItems": 0},
                        }
                    },
                },
                {
                    "if": {"properties": {"state": {"const": "blocked"}}, "required": ["state"]},
                    "then": {"properties": {"blockers": {"minItems": 1}}},
                },
                {
                    "if": {"properties": {"state": {"const": "no_work"}}, "required": ["state"]},
                    "then": {
                        "properties": {
                            "ready_document_ids": {"maxItems": 0},
                            "blocked_document_ids": {"maxItems": 0},
                            "executor_unavailable_document_ids": {"maxItems": 0},
                            "blocking_document_ids": {"maxItems": 0},
                            "source_blockers": {"maxItems": 0},
                            "blockers": {"maxItems": 0},
                        }
                    },
                },
            ],
        },
    )

    schema_version: Literal["1.0.0"] = DOCUMENT_EXECUTION_PLAN_SCHEMA_VERSION
    job_id: JobIdentifier
    source_plan_sha256: Sha256Value
    source_input_fingerprint: Sha256Value
    state: DocumentExecutionPlanState
    items: tuple[DocumentWorkItemV1, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    ready_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    blocked_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    executor_unavailable_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    omitted_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    blocking_document_ids: tuple[DocumentIdentifier, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    source_blockers: tuple[DottedIdentifier, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )
    blockers: tuple[DottedIdentifier, ...] = Field(
        default=(), json_schema_extra={"uniqueItems": True}
    )

    @field_validator(
        "ready_document_ids",
        "blocked_document_ids",
        "executor_unavailable_document_ids",
        "omitted_document_ids",
        "blocking_document_ids",
        "source_blockers",
        "blockers",
    )
    @classmethod
    def _ordered_unique(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        if len(values) != len(set(values)):
            raise ValueError(f"{getattr(info, 'field_name', 'values')} must be unique")
        if values != tuple(sorted(values)):
            raise ValueError(f"{getattr(info, 'field_name', 'values')} must be ordered")
        return values

    @model_validator(mode="after")
    def _consistent_plan(self) -> DocumentExecutionPlanV1:
        item_ids = tuple(item.document_id for item in self.items)
        if len(item_ids) != len(set(item_ids)) or item_ids != tuple(sorted(item_ids)):
            raise ValueError("document work items must use unique deterministic ID order")

        expected = {
            "ready_document_ids": tuple(
                item.document_id for item in self.items if item.state == "ready_to_prepare"
            ),
            "blocked_document_ids": tuple(
                item.document_id for item in self.items if item.state == "blocked"
            ),
            "executor_unavailable_document_ids": tuple(
                item.document_id for item in self.items if item.state == "executor_unavailable"
            ),
            "omitted_document_ids": tuple(
                item.document_id for item in self.items if item.state == "omitted"
            ),
        }
        for field_name, values in expected.items():
            if getattr(self, field_name) != values:
                raise ValueError(f"{field_name} must exactly project work-item state")

        expected_blocking = tuple(
            sorted(
                set(self.blocked_document_ids)
                | set(self.executor_unavailable_document_ids)
            )
        )
        if self.blocking_document_ids != expected_blocking:
            raise ValueError("blocking document IDs must include blocked and unavailable work")

        item_reasons = {reason for item in self.items for reason in item.reason_codes}
        expected_blockers = tuple(sorted(set(self.source_blockers) | item_reasons))
        if self.blockers != expected_blockers:
            raise ValueError("execution-plan blockers must exactly project source and item reasons")

        if self.source_blockers or self.blocked_document_ids:
            expected_state: DocumentExecutionPlanState = "blocked"
        elif self.ready_document_ids and self.executor_unavailable_document_ids:
            expected_state = "partially_dispatchable"
        elif self.executor_unavailable_document_ids:
            expected_state = "blocked"
        elif self.ready_document_ids:
            expected_state = "ready"
        else:
            expected_state = "no_work"
        if self.state != expected_state:
            raise ValueError("document execution state must match the exact work-item projection")
        return self


@dataclass(frozen=True)
class DocumentExecutionInspection:
    source_path: Path
    source_state: DocumentExecutionSourceState
    plan: DocumentExecutionPlanV1 | None = None
    reason_codes: tuple[str, ...] = ()


def inspect_stage_status(*args: object, **kwargs: object) -> object:
    """Late-bound runtime seam used by read-only document inspection.

    Keeping this wrapper at module scope preserves the existing test/plugin
    monkeypatch boundary without recreating the package-review import cycle.
    """

    from canisend.stage_runtime import inspect_stage_status as runtime_inspect

    return runtime_inspect(*args, **kwargs)


def document_executor_capabilities() -> tuple[DocumentExecutorCapabilityV1, ...]:
    """Return the immutable normalized-kind capability registry."""

    return _DOCUMENT_EXECUTOR_CAPABILITIES


def derive_document_execution_plan(
    source: RequiredDocumentPlanV1,
    *,
    source_plan_sha256: str,
) -> DocumentExecutionPlanV1:
    """Derive a body-free fan-out plan from one validated Required Document Plan."""

    capability_by_kind = {
        item.normalized_kind: item for item in _DOCUMENT_EXECUTOR_CAPABILITIES
    }
    kind_counts = Counter(item.normalized_kind for item in source.requirements)
    task_by_id = {item.document_id: item for item in source.tasks}
    items: list[DocumentWorkItemV1] = []

    for requirement in source.requirements:
        task = task_by_id[requirement.document_id]
        capability = capability_by_kind.get(requirement.normalized_kind)
        if capability is not None and capability.scope != "submission_document":
            capability = None
        availability: DocumentExecutorAvailability = (
            capability.availability if capability is not None else "unregistered"
        )
        state: DocumentWorkItemState
        reasons: tuple[str, ...]

        if task.blockers or task.action == "needs_confirmation":
            state = "blocked"
            reasons = _ordered_unique((*task.blockers, "documents.task_blocked"))
        elif task.action == "omit":
            state = "omitted"
            reasons = ()
        elif source.blockers:
            state = "blocked"
            reasons = ("documents.plan_blocked",)
        elif (
            capability is not None
            and capability.availability == "available"
            and kind_counts[requirement.normalized_kind] > capability.max_instances
        ):
            state = "blocked"
            reasons = ("documents.executor_cardinality_unsupported",)
        elif availability == "available":
            state = "ready_to_prepare"
            reasons = ()
        else:
            state = "executor_unavailable"
            reasons = (
                "documents.executor_planned"
                if availability == "planned"
                else "documents.executor_unregistered",
            )

        items.append(
            DocumentWorkItemV1(
                document_id=requirement.document_id,
                normalized_kind=requirement.normalized_kind,
                requirement=requirement.requirement,
                action=task.action,
                confirmation_state=task.confirmation_state,
                state=state,
                executor_scope=(
                    capability.scope if capability is not None else "submission_document"
                ),
                executor_availability=availability,
                route_id=capability.route_id if capability is not None else None,
                executor_id=capability.executor_id if capability is not None else None,
                stage=capability.stage if capability is not None else None,
                authoritative_target=(
                    capability.authoritative_target if capability is not None else None
                ),
                output_schema=capability.output_schema if capability is not None else None,
                execution_modes=(
                    tuple(sorted(capability.execution_modes)) if capability is not None else ()
                ),
                reason_codes=tuple(sorted(reasons)),
            )
        )

    items.sort(key=lambda item: item.document_id)
    ready_ids = tuple(item.document_id for item in items if item.state == "ready_to_prepare")
    blocked_ids = tuple(item.document_id for item in items if item.state == "blocked")
    unavailable_ids = tuple(
        item.document_id for item in items if item.state == "executor_unavailable"
    )
    omitted_ids = tuple(item.document_id for item in items if item.state == "omitted")
    source_blockers = tuple(sorted(source.blockers))
    item_reasons = {reason for item in items for reason in item.reason_codes}
    blockers = tuple(sorted(set(source_blockers) | item_reasons))

    if source_blockers or blocked_ids:
        state: DocumentExecutionPlanState = "blocked"
    elif ready_ids and unavailable_ids:
        state = "partially_dispatchable"
    elif unavailable_ids:
        state = "blocked"
    elif ready_ids:
        state = "ready"
    else:
        state = "no_work"

    return DocumentExecutionPlanV1(
        job_id=source.job_id,
        source_plan_sha256=source_plan_sha256,
        source_input_fingerprint=source.input_fingerprint,
        state=state,
        items=tuple(items),
        ready_document_ids=ready_ids,
        blocked_document_ids=blocked_ids,
        executor_unavailable_document_ids=unavailable_ids,
        omitted_document_ids=omitted_ids,
        blocking_document_ids=tuple(sorted(set(blocked_ids) | set(unavailable_ids))),
        source_blockers=source_blockers,
        blockers=blockers,
    )


def inspect_document_execution(workspace: Path, job_dir: Path) -> DocumentExecutionInspection:
    """Inspect current fan-out without writing a derived status artifact."""

    # Keep the pure execution-plan contract importable by aggregate stage adapters.
    # The runtime imports this module through CLI/agent surfaces, so importing it
    # lazily here avoids a stage_adapters -> package_review -> document_execution
    # -> stage_runtime cycle.
    from canisend.stage_runtime import StageRuntimeError

    source_path = job_dir / REQUIRED_DOCUMENT_PLAN_PATH
    try:
        stage = inspect_stage_status(workspace, job_dir, stage="brief")
    except StageRuntimeError:
        source_state: DocumentExecutionSourceState = (
            "missing" if not source_path.is_file() else "not_current"
        )
        return DocumentExecutionInspection(
            source_path=source_path,
            source_state=source_state,
            reason_codes=(
                "documents.plan_missing"
                if source_state == "missing"
                else "documents.plan_not_current",
            ),
        )
    if (
        stage.stage.status != "succeeded"
        or stage.reasons
        or stage.output_drift
    ):
        source_state = "missing" if not source_path.is_file() else "not_current"
        return DocumentExecutionInspection(
            source_path=source_path,
            source_state=source_state,
            reason_codes=(
                "documents.plan_missing"
                if source_state == "missing"
                else "documents.plan_not_current",
            ),
        )
    try:
        source = RequiredDocumentPlanV1.model_validate(read_json_object(source_path))
        if source.job_id != job_dir.name:
            raise ValueError("required document plan belongs to another job")
        plan = derive_document_execution_plan(
            source,
            source_plan_sha256=sha256_file(source_path),
        )
    except (OSError, StageStoreError, ValidationError, ValueError):
        return DocumentExecutionInspection(
            source_path=source_path,
            source_state="invalid",
            reason_codes=("documents.plan_invalid",),
        )
    return DocumentExecutionInspection(
        source_path=source_path,
        source_state="current",
        plan=plan,
    )


def document_execution_status_agent_response(
    workspace: Path,
    job_dir: Path,
    inspection: DocumentExecutionInspection,
) -> AgentResponse:
    """Project aggregate document execution state without private document bodies."""

    artifacts = []
    if inspection.source_path.exists():
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=inspection.source_path,
                kind="required_document_plan",
                privacy_tier=2,
                trust_level=(
                    "validated" if inspection.source_state == "current" else "trusted_local"
                ),
                media_type="application/json",
                include_hash=inspection.source_state == "current",
            )
        )

    if inspection.plan is None:
        primary_reason = inspection.reason_codes[0] if inspection.reason_codes else None
        return success_response(
            operation="documents.status",
            workflow=WorkflowSnapshotReference(phase="unknown", readiness="blocked"),
            artifacts=artifacts,
            blockers=["A current required-document plan is required before document fan-out."],
            next_actions=[
                NextAction(
                    id="stage.run_brief",
                    label="Generate or refresh the required-document plan",
                )
            ],
            extensions=_status_extensions(
                source_state=inspection.source_state,
                plan=None,
                primary_reason=primary_reason,
            ),
        )

    plan = inspection.plan
    blockers: list[str] = []
    actions: list[NextAction] = []
    if plan.source_blockers:
        blockers.append("The current required-document plan contains unresolved blockers.")
        actions.append(
            NextAction(
                id="brief.status",
                label="Resolve Brief and required-document blockers",
            )
        )
    else:
        if plan.ready_document_ids:
            actions.append(
                NextAction(
                    id="stage.run_draft",
                    label="Prepare the currently dispatchable structured document",
                )
            )
        if plan.executor_unavailable_document_ids:
            count = len(plan.executor_unavailable_document_ids)
            blockers.append(
                f"{count} confirmed document task(s) have no guarded executor."
            )
            actions.append(
                NextAction(
                    id="documents.review_capabilities",
                    label="Review document tasks without guarded executors",
                )
            )
        if plan.blocked_document_ids:
            blockers.append("At least one document task cannot be dispatched safely.")
            if not actions:
                actions.append(
                    NextAction(
                        id="documents.review_capabilities",
                        label="Review blocked document execution capabilities",
                    )
                )

    readiness = {
        "ready": "ready_for_next_stage",
        "partially_dispatchable": "action_required",
        "blocked": "blocked",
        "no_work": "ready_for_next_stage",
    }[plan.state]
    return success_response(
        operation="documents.status",
        workflow=WorkflowSnapshotReference(phase="unknown", readiness=readiness),
        artifacts=artifacts,
        blockers=blockers,
        next_actions=actions,
        extensions=_status_extensions(
            source_state=inspection.source_state,
            plan=plan,
            primary_reason=plan.blockers[0] if plan.blockers else None,
        ),
    )


def _status_extensions(
    *,
    source_state: DocumentExecutionSourceState,
    plan: DocumentExecutionPlanV1 | None,
    primary_reason: str | None,
) -> dict[str, str | int | None]:
    return {
        "canisend.document_execution_source_state": source_state,
        "canisend.document_execution_state": plan.state if plan is not None else "unavailable",
        "canisend.document_execution_source_sha256": (
            plan.source_plan_sha256 if plan is not None else None
        ),
        "canisend.document_execution_item_count": len(plan.items) if plan is not None else 0,
        "canisend.document_ready_to_prepare_count": (
            len(plan.ready_document_ids) if plan is not None else 0
        ),
        "canisend.document_blocked_count": (
            len(plan.blocked_document_ids) if plan is not None else 0
        ),
        "canisend.document_executor_unavailable_count": (
            len(plan.executor_unavailable_document_ids) if plan is not None else 0
        ),
        "canisend.document_omitted_count": (
            len(plan.omitted_document_ids) if plan is not None else 0
        ),
        "canisend.document_execution_blocker_count": (
            len(plan.blockers)
            if plan is not None
            else int(primary_reason is not None)
        ),
        "canisend.document_execution_primary_blocker": primary_reason,
    }


def _safe_relative_path(value: str) -> str:
    if not value or "\\" in value or PureWindowsPath(value).is_absolute():
        raise ValueError("document target must be a safe relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError("document target must be a safe relative path")
    return path.as_posix()


def _ordered_unique(values: tuple[str, ...]) -> tuple[str, ...]:
    return tuple(dict.fromkeys(values))


def _build_capability_registry() -> tuple[DocumentExecutorCapabilityV1, ...]:
    planned_submission_kinds = (
        "application_form",
        "cv",
        "diversity_statement",
        "personal_statement",
        "publication_list",
        "references",
        "research_proposal",
        "supporting_statement",
        "teaching_philosophy",
        "teaching_statement",
        "writing_sample",
    )
    capabilities = [
        DocumentExecutorCapabilityV1(
            normalized_kind="cover_letter",
            scope="submission_document",
            route_id="documents.cover_letter",
            availability="available",
            executor_id="draft.cover_letter",
            stage="draft",
            authoritative_target="cover_letter_draft.json",
            output_schema="canisend.cover-letter-draft/v1",
            execution_modes=("configured_provider", "host_agent"),
            max_instances=1,
        ),
        DocumentExecutorCapabilityV1(
            normalized_kind="research_statement",
            scope="submission_document",
            route_id="documents.research_statement",
            availability="available",
            executor_id="draft.research_statement",
            stage="draft",
            authoritative_target="research_statement_draft.json",
            output_schema="canisend.research-statement-draft/v1",
            execution_modes=("host_agent",),
            max_instances=1,
        ),
        *(
            DocumentExecutorCapabilityV1(
                normalized_kind=kind,
                scope="submission_document",
                route_id=f"documents.{kind}",
                availability="planned",
            )
            for kind in planned_submission_kinds
        ),
        DocumentExecutorCapabilityV1(
            normalized_kind="application_email",
            scope="workflow_support",
            route_id="documents.application_email",
            availability="planned",
        ),
        DocumentExecutorCapabilityV1(
            normalized_kind="interview_preparation",
            scope="workflow_support",
            route_id="documents.interview_preparation",
            availability="planned",
        ),
    ]
    capabilities.sort(key=lambda item: item.normalized_kind)
    kinds = tuple(item.normalized_kind for item in capabilities)
    routes = tuple(item.route_id for item in capabilities)
    if len(kinds) != len(set(kinds)) or len(routes) != len(set(routes)):
        raise ValueError("document executor capability registry contains duplicate identifiers")
    return tuple(capabilities)


_DOCUMENT_EXECUTOR_CAPABILITIES = _build_capability_registry()
