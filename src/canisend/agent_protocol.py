from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
from typing import Annotated, Literal
from uuid import uuid4

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator


AGENT_PROTOCOL = "canisend.agent/v1"
AGENT_SCHEMA_VERSION = "1.0.0"

JsonScalar = str | int | float | bool | None
PrivacyTier = Annotated[int, Field(ge=0, le=3)]
TrustLevel = Literal[
    "trusted_local",
    "untrusted_import",
    "validated",
    "generated_candidate",
]
Readiness = Literal[
    "blocked",
    "action_required",
    "review_required",
    "ready_for_next_stage",
    "unknown",
]
WorkflowPhase = Literal[
    "intake",
    "evidence",
    "parse",
    "package",
    "verify",
    "render",
    "unknown",
]
IntakeType = Literal[
    "manual_metadata",
    "local_text",
    "local_pdf",
    "explicit_url",
    "feed_lead",
]
ExecutionMode = Literal["local_service", "host_agent", "configured_provider"]
SupportedHost = Literal["codex_cli", "codex_app_shell", "claude_code", "ide_shell"]

SUPPORTED_AGENT_OPERATIONS = (
    "agent.capabilities",
    "agent.context",
    "workspace.inspect",
    "job.intake",
    "job.intake_from_lead",
    "job.list",
    "package.check",
    "criteria.corrections_status",
    "criteria.corrections_initialize",
    "criteria.corrections_update",
    "decision.status",
    "decision.initialize",
    "decision.update",
    "brief.status",
    "brief.initialize",
    "brief.update",
    "documents.status",
    "review.dispositions_status",
    "review.dispositions_initialize",
    "review.dispositions_update",
    "user_mutation.recover",
    "workflow.stage_status",
    "workflow.stage_prepare",
    "workflow.stage_submit",
    "workflow.stage_apply",
    "workflow.stage_cancel",
    "workflow.stage_run",
)
KNOWN_AGENT_ERROR_CODES = frozenset(
    {
        "workspace.invalid",
        "workspace.not_initialized",
        "job.not_found",
        "job.invalid_metadata",
        "input.invalid",
        "source.import_failed",
        "operation.failed",
        "user_input.not_initialized",
        "user_input.invalid",
        "user_input.unsafe_path",
        "user_input.consent_required",
        "user_input.conflict",
        "user_input.dependency_not_current",
        "user_input.store_failed",
        "user_input.recovery_required",
        "stage.unknown",
        "stage.unsupported",
        "stage.unsupported_mode",
        "stage.document_ambiguous",
        "stage.document_id_invalid",
        "stage.document_not_found",
        "stage.document_not_resolved",
        "stage.document_scope_invalid",
        "stage.job_outside_workspace",
        "stage.invalid_input",
        "stage.dependency_not_current",
        "stage.concurrent_run",
        "stage.task_contract_mismatch",
        "stage.task_integrity_mismatch",
        "stage.task_not_active",
        "stage.submission_missing",
        "stage.submission_conflict",
        "stage.transition_conflict",
        "stage.no_active_run",
        "stage.stale_input",
        "stage.output_conflict",
        "stage.already_current",
        "stage.task_identity_mismatch",
        "stage.result_identity_mismatch",
        "stage.result_scope_mismatch",
        "stage.invalid_candidate",
        "stage.candidate_hash_mismatch",
        "stage.candidate_missing",
        "stage.unsafe_path",
        "stage.execution_failed",
        "stage.invalid_result",
        "stage.store_failed",
        "stage.state_write_failed",
        "stage.output_unreadable",
        "stage.artifact_unreadable",
        "stage.recovery_failed",
        "stage.provider_consent_required",
        "stage.provider_not_configured",
        "stage.provider_failed",
        "stage.provider_invalid_response",
        "stage.provider_input_too_large",
    }
)

_DOTTED_ID_RE = re.compile(r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)+$")
_SLUG_ID_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]*$")
_REQUEST_ID_RE = re.compile(r"^req_[0-9a-f]{32}$")
_OPAQUE_ID_RE = re.compile(r"^external-[0-9a-f]{16}$")
_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_EXTENSION_KEY_RE = re.compile(r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)+$")


class ProtocolModel(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True, str_strip_whitespace=True)


class AgentError(ProtocolModel):
    code: str
    message: str = Field(min_length=1)
    retryable: bool = False
    hint: str | None = None

    @field_validator("code")
    @classmethod
    def _valid_code(cls, value: str) -> str:
        if _DOTTED_ID_RE.fullmatch(value) is None:
            raise ValueError("error code must be a lowercase dotted identifier")
        return value


class ArtifactReference(ProtocolModel):
    kind: str
    path: str | None = None
    opaque_id: str | None = None
    exists: bool
    sha256: str | None = None
    privacy_tier: PrivacyTier
    trust_level: TrustLevel
    media_type: str | None = None

    @field_validator("kind")
    @classmethod
    def _valid_kind(cls, value: str) -> str:
        if _SLUG_ID_RE.fullmatch(value) is None:
            raise ValueError("artifact kind must be a lowercase identifier")
        return value

    @field_validator("path")
    @classmethod
    def _valid_path(cls, value: str | None) -> str | None:
        return _workspace_relative_path(value) if value is not None else None

    @field_validator("opaque_id")
    @classmethod
    def _valid_opaque_id(cls, value: str | None) -> str | None:
        if value is not None and _OPAQUE_ID_RE.fullmatch(value) is None:
            raise ValueError("external artifact identifier is invalid")
        return value

    @field_validator("sha256")
    @classmethod
    def _valid_hash(cls, value: str | None) -> str | None:
        if value is not None and _SHA256_RE.fullmatch(value) is None:
            raise ValueError("sha256 must be a lowercase 64-character digest")
        return value

    @model_validator(mode="after")
    def _one_locator(self) -> ArtifactReference:
        if (self.path is None) == (self.opaque_id is None):
            raise ValueError("artifact reference requires exactly one path or opaque_id")
        if self.opaque_id is not None and self.sha256 is not None:
            raise ValueError("external artifact references must not include a content hash")
        return self


class ConsentRequirement(ProtocolModel):
    id: str
    purpose: str = Field(min_length=1)
    privacy_tier: PrivacyTier
    artifact_kinds: list[str] = Field(default_factory=list)
    required: bool = True

    @field_validator("id")
    @classmethod
    def _valid_id(cls, value: str) -> str:
        if _SLUG_ID_RE.fullmatch(value) is None:
            raise ValueError("consent identifier is invalid")
        return value

    @field_validator("artifact_kinds")
    @classmethod
    def _valid_artifact_kinds(cls, values: list[str]) -> list[str]:
        for value in values:
            if _SLUG_ID_RE.fullmatch(value) is None:
                raise ValueError("consent artifact kind is invalid")
        return values


class NextAction(ProtocolModel):
    id: str
    label: str = Field(min_length=1)
    requires_consent: bool = False
    consent_ids: list[str] = Field(default_factory=list)

    @field_validator("id")
    @classmethod
    def _valid_id(cls, value: str) -> str:
        if _DOTTED_ID_RE.fullmatch(value) is None:
            raise ValueError("next-action identifier must be a lowercase dotted identifier")
        return value

    @field_validator("consent_ids")
    @classmethod
    def _valid_consent_ids(cls, values: list[str]) -> list[str]:
        for value in values:
            if _SLUG_ID_RE.fullmatch(value) is None:
                raise ValueError("consent identifier is invalid")
        return values

    @model_validator(mode="after")
    def _consistent_consent(self) -> NextAction:
        if self.requires_consent and not self.consent_ids:
            raise ValueError("consent-requiring action must name a consent id")
        if not self.requires_consent and self.consent_ids:
            raise ValueError("action with consent ids must require consent")
        return self


class JobReference(ProtocolModel):
    id: str = Field(min_length=1)
    path: str
    title: str = Field(min_length=1)
    institution: str = Field(min_length=1)
    deadline: str
    status: str = Field(min_length=1)
    next_action: NextAction | None = None

    @field_validator("path")
    @classmethod
    def _valid_path(cls, value: str) -> str:
        return _workspace_relative_path(value)


class WorkflowSnapshotReference(ProtocolModel):
    phase: WorkflowPhase
    readiness: Readiness
    derived: bool = True


class GateOutcome(ProtocolModel):
    status: Literal["PASS", "FAIL", "STALE", "NOT_RUN"]
    issue_count: int = Field(default=0, ge=0)
    report_path: str | None = None

    @field_validator("report_path")
    @classmethod
    def _valid_report_path(cls, value: str | None) -> str | None:
        return _workspace_relative_path(value) if value is not None else None


class AgentCapabilities(ProtocolModel):
    package_version: str = Field(min_length=1)
    protocol_versions: list[str] = Field(default_factory=lambda: [AGENT_PROTOCOL])
    schema_versions: list[str] = Field(default_factory=lambda: [AGENT_SCHEMA_VERSION])
    operations: list[str] = Field(default_factory=lambda: list(SUPPORTED_AGENT_OPERATIONS))
    intake_types: list[IntakeType] = Field(
        default_factory=lambda: [
            "manual_metadata",
            "local_text",
            "local_pdf",
            "explicit_url",
            "feed_lead",
        ]
    )
    execution_modes: list[ExecutionMode] = Field(
        default_factory=lambda: ["local_service", "host_agent", "configured_provider"]
    )
    supported_hosts: list[SupportedHost] = Field(
        default_factory=lambda: ["codex_cli", "codex_app_shell", "claude_code", "ide_shell"]
    )

    @field_validator("operations")
    @classmethod
    def _valid_operations(cls, values: list[str]) -> list[str]:
        for value in values:
            if _DOTTED_ID_RE.fullmatch(value) is None:
                raise ValueError("capability operation must be a lowercase dotted identifier")
        return values


class AgentResponse(ProtocolModel):
    protocol: Literal["canisend.agent/v1"] = AGENT_PROTOCOL
    schema_version: Literal["1.0.0"] = AGENT_SCHEMA_VERSION
    request_id: str = Field(default_factory=lambda: f"req_{uuid4().hex}")
    operation: str
    ok: bool
    capabilities: AgentCapabilities | None = None
    job: JobReference | None = None
    jobs: list[JobReference] = Field(default_factory=list)
    workflow: WorkflowSnapshotReference | None = None
    artifacts: list[ArtifactReference] = Field(default_factory=list)
    missing_fields: list[str] = Field(default_factory=list)
    required_consents: list[ConsentRequirement] = Field(default_factory=list)
    warnings: list[str] = Field(default_factory=list)
    blockers: list[str] = Field(default_factory=list)
    next_actions: list[NextAction] = Field(default_factory=list)
    gate: GateOutcome | None = None
    error: AgentError | None = None
    extensions: dict[str, JsonScalar] = Field(default_factory=dict)

    @field_validator("request_id")
    @classmethod
    def _valid_request_id(cls, value: str) -> str:
        if _REQUEST_ID_RE.fullmatch(value) is None:
            raise ValueError("request_id is invalid")
        return value

    @field_validator("operation")
    @classmethod
    def _valid_operation(cls, value: str) -> str:
        if _DOTTED_ID_RE.fullmatch(value) is None:
            raise ValueError("operation must be a lowercase dotted identifier")
        return value

    @field_validator("extensions")
    @classmethod
    def _valid_extensions(cls, values: dict[str, JsonScalar]) -> dict[str, JsonScalar]:
        for key in values:
            if _EXTENSION_KEY_RE.fullmatch(key) is None:
                raise ValueError("extension keys must be namespaced lowercase identifiers")
        return values

    @model_validator(mode="after")
    def _consistent_error(self) -> AgentResponse:
        if self.ok and self.error is not None:
            raise ValueError("successful response must not include an error")
        if not self.ok and self.error is None:
            raise ValueError("failed response must include an error")
        return self


def success_response(
    *,
    operation: str,
    capabilities: AgentCapabilities | None = None,
    job: JobReference | None = None,
    jobs: list[JobReference] | None = None,
    workflow: WorkflowSnapshotReference | None = None,
    artifacts: list[ArtifactReference] | None = None,
    missing_fields: list[str] | None = None,
    required_consents: list[ConsentRequirement] | None = None,
    warnings: list[str] | None = None,
    blockers: list[str] | None = None,
    next_actions: list[NextAction] | None = None,
    gate: GateOutcome | None = None,
    extensions: dict[str, JsonScalar] | None = None,
) -> AgentResponse:
    return AgentResponse(
        operation=operation,
        ok=True,
        capabilities=capabilities,
        job=job,
        jobs=jobs or [],
        workflow=workflow,
        artifacts=artifacts or [],
        missing_fields=missing_fields or [],
        required_consents=required_consents or [],
        warnings=warnings or [],
        blockers=blockers or [],
        next_actions=next_actions or [],
        gate=gate,
        extensions=extensions or {},
    )


def error_response(
    *,
    operation: str,
    code: str,
    message: str,
    retryable: bool = False,
    hint: str | None = None,
    job: JobReference | None = None,
    jobs: list[JobReference] | None = None,
    workflow: WorkflowSnapshotReference | None = None,
    artifacts: list[ArtifactReference] | None = None,
    missing_fields: list[str] | None = None,
    required_consents: list[ConsentRequirement] | None = None,
    warnings: list[str] | None = None,
    blockers: list[str] | None = None,
    next_actions: list[NextAction] | None = None,
    gate: GateOutcome | None = None,
    extensions: dict[str, JsonScalar] | None = None,
) -> AgentResponse:
    return AgentResponse(
        operation=operation,
        ok=False,
        job=job,
        jobs=jobs or [],
        workflow=workflow,
        artifacts=artifacts or [],
        missing_fields=missing_fields or [],
        required_consents=required_consents or [],
        warnings=warnings or [],
        blockers=blockers or [],
        next_actions=next_actions or [],
        gate=gate,
        error=AgentError(
            code=code,
            message=message,
            retryable=retryable,
            hint=hint,
        ),
        extensions=extensions or {},
    )


def dumps_agent_response(response: AgentResponse) -> str:
    payload = response.model_dump(mode="json")
    return json.dumps(payload, ensure_ascii=False, sort_keys=True) + "\n"


def default_agent_capabilities(package_version: str) -> AgentCapabilities:
    return AgentCapabilities(package_version=package_version)


def agent_response_lines(response: AgentResponse) -> list[str]:
    lines = [
        f"Operation: {response.operation}",
        f"Protocol: {response.protocol} (schema {response.schema_version})",
        f"Result: {'ok' if response.ok else 'error'}",
    ]
    if response.capabilities is not None:
        capabilities = response.capabilities
        lines.extend(
            [
                f"Package: {capabilities.package_version}",
                f"Operations: {', '.join(capabilities.operations)}",
                f"Intake types: {', '.join(capabilities.intake_types)}",
                f"Execution modes: {', '.join(capabilities.execution_modes)}",
                f"Supported hosts: {', '.join(capabilities.supported_hosts)}",
            ]
        )
    if response.job is not None:
        lines.append(f"Job: {response.job.id} — {response.job.title} at {response.job.institution}")
    for job in response.jobs:
        suffix = f"; next: {job.next_action.id}" if job.next_action is not None else ""
        lines.append(f"Job: {job.id} — {job.title} at {job.institution} ({job.status}{suffix})")
    if response.workflow is not None:
        lines.append(f"Workflow: {response.workflow.phase} ({response.workflow.readiness})")
    for artifact in response.artifacts:
        locator = artifact.path or artifact.opaque_id
        lines.append(f"Artifact: {artifact.kind} — {locator} ({'present' if artifact.exists else 'missing'})")
    lines.extend(f"Missing: {field}" for field in response.missing_fields)
    lines.extend(f"Consent required: {consent.id} — {consent.purpose}" for consent in response.required_consents)
    lines.extend(f"Warning: {warning}" for warning in response.warnings)
    lines.extend(f"Blocker: {blocker}" for blocker in response.blockers)
    lines.extend(f"Next action: {action.id} — {action.label}" for action in response.next_actions)
    if response.gate is not None:
        lines.append(f"Gate: {response.gate.status} ({response.gate.issue_count} issue(s))")
    if response.error is not None:
        lines.append(f"Error: {response.error.code} — {response.error.message}")
        if response.error.hint:
            lines.append(f"Hint: {response.error.hint}")
    return lines


def artifact_reference_from_path(
    *,
    workspace: Path,
    path: Path,
    kind: str,
    privacy_tier: PrivacyTier,
    trust_level: TrustLevel,
    media_type: str | None = None,
    include_hash: bool = False,
) -> ArtifactReference:
    workspace_root = workspace.expanduser().resolve()
    expanded = path.expanduser()
    candidate = expanded if expanded.is_absolute() else workspace_root / expanded
    resolved = candidate.resolve()
    exists = resolved.exists()

    try:
        relative = resolved.relative_to(workspace_root)
    except ValueError:
        opaque_id = f"external-{sha256(str(resolved).encode('utf-8')).hexdigest()[:16]}"
        return ArtifactReference(
            kind=kind,
            opaque_id=opaque_id,
            exists=exists,
            privacy_tier=privacy_tier,
            trust_level=trust_level,
            media_type=media_type,
        )

    digest = _file_sha256(resolved) if include_hash and resolved.is_file() else None
    return ArtifactReference(
        kind=kind,
        path=relative.as_posix(),
        exists=exists,
        sha256=digest,
        privacy_tier=privacy_tier,
        trust_level=trust_level,
        media_type=media_type,
    )


def _workspace_relative_path(value: str) -> str:
    if not value or "\\" in value:
        raise ValueError("path must be a non-empty POSIX path")
    windows_path = PureWindowsPath(value)
    path = PurePosixPath(value)
    if path.is_absolute() or windows_path.is_absolute() or windows_path.drive:
        raise ValueError("path must be workspace-relative")
    if any(part in {".", ".."} for part in value.split("/")):
        raise ValueError("path must not contain dot or parent segments")
    normalized = path.as_posix()
    if normalized != value:
        raise ValueError("path must be normalized")
    return value


def _file_sha256(path: Path) -> str:
    return sha256(path.read_bytes()).hexdigest()
