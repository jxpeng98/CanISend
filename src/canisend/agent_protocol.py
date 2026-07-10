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


class AgentResponse(ProtocolModel):
    protocol: Literal["canisend.agent/v1"] = AGENT_PROTOCOL
    schema_version: Literal["1.0.0"] = AGENT_SCHEMA_VERSION
    request_id: str = Field(default_factory=lambda: f"req_{uuid4().hex}")
    operation: str
    ok: bool
    job: JobReference | None = None
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
    job: JobReference | None = None,
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
        job=job,
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
    warnings: list[str] | None = None,
    extensions: dict[str, JsonScalar] | None = None,
) -> AgentResponse:
    return AgentResponse(
        operation=operation,
        ok=False,
        warnings=warnings or [],
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

