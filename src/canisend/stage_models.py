from __future__ import annotations

from datetime import datetime
from pathlib import PurePosixPath, PureWindowsPath
import re
from typing import Annotated, Literal

from pydantic import AwareDatetime, BaseModel, ConfigDict, Field, field_validator, model_validator


WORKFLOW_STATE_SCHEMA_VERSION = "1.1.0"
TASK_SPEC_SCHEMA_VERSION = "1.1.0"
TASK_RESULT_SCHEMA_VERSION = "1.1.0"
RUN_MANIFEST_SCHEMA_VERSION = "1.1.0"
CANDIDATE_SUBMISSION_SCHEMA_VERSION = "1.1.0"

StageContractSchemaVersion = Literal["1.0.0", "1.1.0"]

JSON_SCHEMA_DIALECT = "https://json-schema.org/draft/2020-12/schema"
SCHEMA_BASE_ID = "https://github.com/jxpeng98/CanISend/schemas"

StageName = Literal[
    "intake",
    "evidence",
    "parse",
    "confirm",
    "match",
    "decide",
    "brief",
    "draft",
    "review",
    "package",
    "verify",
    "render",
]
StageStatus = Literal[
    "pending",
    "blocked",
    "ready",
    "running",
    "succeeded",
    "failed",
    "stale",
    "cancelled",
]
ExecutionMode = Literal["deterministic", "host_agent", "configured_provider"]
TaskResultStatus = Literal["succeeded", "failed", "cancelled"]
ValidationStatus = Literal["passed", "failed"]
RunStatus = Literal["prepared", "running", "succeeded", "failed", "cancelled"]
PrivacyTier = Annotated[int, Field(ge=0, le=3)]

_CANONICAL_STAGE_ORDER: tuple[StageName, ...] = (
    "intake",
    "evidence",
    "parse",
    "confirm",
    "match",
    "decide",
    "brief",
    "draft",
    "review",
    "package",
    "verify",
    "render",
)
_STAGE_ORDER_INDEX = {stage: index for index, stage in enumerate(_CANONICAL_STAGE_ORDER)}
_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_JOB_ID_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]*$")
_RUN_ID_RE = re.compile(r"^run_[0-9a-f]{32}$")
_TASK_ID_RE = re.compile(r"^task_[0-9a-f]{32}$")
_DOCUMENT_ID_RE = re.compile(r"^document_[0-9a-f]{32}$")
_DOTTED_ID_RE = re.compile(r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)+$")
_SLUG_ID_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]*$")
_OUTPUT_SCHEMA_RE = re.compile(r"^canisend\.[a-z0-9][a-z0-9_.-]*/v[1-9][0-9]*$")


class StageContractModel(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
    )


class ArtifactFingerprint(StageContractModel):
    path: str
    sha256: str
    size_bytes: int | None = Field(default=None, ge=0)

    @field_validator("path")
    @classmethod
    def _valid_path(cls, value: str) -> str:
        return _job_relative_path(value)

    @field_validator("sha256")
    @classmethod
    def _valid_sha256(cls, value: str) -> str:
        if _SHA256_RE.fullmatch(value) is None:
            raise ValueError("sha256 must be a lowercase 64-character digest")
        return value


class StageRecord(StageContractModel):
    stage: StageName
    document_id: str | None = None
    status: StageStatus = "pending"
    attempt_count: int = Field(default=0, ge=0)
    run_id: str | None = None
    input_fingerprint: str | None = None
    inputs: tuple[ArtifactFingerprint, ...] = ()
    outputs: tuple[ArtifactFingerprint, ...] = ()
    started_at: AwareDatetime | None = None
    completed_at: AwareDatetime | None = None
    error_code: str | None = None

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str | None) -> str | None:
        return _optional_document_id(value)

    @field_validator("run_id")
    @classmethod
    def _valid_run_id(cls, value: str | None) -> str | None:
        return _optional_prefixed_id(value, pattern=_RUN_ID_RE, label="run_id")

    @field_validator("input_fingerprint")
    @classmethod
    def _valid_input_fingerprint(cls, value: str | None) -> str | None:
        return _sha256(value, label="input_fingerprint") if value is not None else None

    @field_validator("inputs", "outputs")
    @classmethod
    def _unique_artifact_paths(
        cls,
        values: tuple[ArtifactFingerprint, ...],
    ) -> tuple[ArtifactFingerprint, ...]:
        _require_unique_paths(values)
        return values

    @field_validator("error_code")
    @classmethod
    def _valid_error_code(cls, value: str | None) -> str | None:
        return _optional_dotted_id(value, label="error_code")

    @model_validator(mode="after")
    def _consistent_lifecycle(self) -> StageRecord:
        _require_document_scope(self.stage, self.document_id)
        inactive = {"pending", "blocked", "ready"}
        terminal = {"succeeded", "failed", "stale", "cancelled"}

        if self.status in inactive:
            if (
                self.run_id is not None
                or self.input_fingerprint is not None
                or self.started_at is not None
                or self.completed_at is not None
            ):
                raise ValueError("an inactive stage must not name a current run or timestamps")
        else:
            if (
                self.run_id is None
                or self.input_fingerprint is None
                or self.started_at is None
                or self.attempt_count < 1
            ):
                raise ValueError(
                    "an attempted stage requires run_id, input_fingerprint, started_at, and attempt_count >= 1"
                )

        if self.status == "running" and self.completed_at is not None:
            raise ValueError("a running stage must not have completed_at")
        if self.status in terminal and self.completed_at is None:
            raise ValueError("a terminal or stale stage requires completed_at")
        if self.completed_at is not None and self.started_at is not None:
            _require_time_order(self.started_at, self.completed_at, "stage timestamps")

        if self.status in {"succeeded", "stale"} and not self.outputs:
            raise ValueError("a succeeded or stale stage requires output fingerprints")
        if self.status == "failed" and self.error_code is None:
            raise ValueError("a failed stage requires error_code")
        if self.status != "failed" and self.error_code is not None:
            raise ValueError("only a failed stage may include error_code")
        return self


class WorkflowStateV1(StageContractModel):
    model_config = ConfigDict(
        title="CanISendWorkflowStateV1",
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/workflow-state.schema.json",
        },
    )

    schema_version: StageContractSchemaVersion = WORKFLOW_STATE_SCHEMA_VERSION
    job_id: str
    revision: int = Field(ge=0)
    created_at: AwareDatetime
    updated_at: AwareDatetime
    active_run_id: str | None = None
    stages: tuple[StageRecord, ...] = ()

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("active_run_id")
    @classmethod
    def _valid_active_run_id(cls, value: str | None) -> str | None:
        return _optional_prefixed_id(value, pattern=_RUN_ID_RE, label="active_run_id")

    @model_validator(mode="after")
    def _consistent_state(self) -> WorkflowStateV1:
        _require_time_order(self.created_at, self.updated_at, "workflow timestamps")

        stage_keys = tuple((record.stage, record.document_id) for record in self.stages)
        if len(stage_keys) != len(set(stage_keys)):
            raise ValueError("workflow stage instances must be unique")
        order = tuple(
            (_STAGE_ORDER_INDEX[record.stage], record.document_id or "")
            for record in self.stages
        )
        if order != tuple(sorted(order)):
            raise ValueError("workflow stage instances must use canonical order")
        if self.schema_version == WORKFLOW_STATE_SCHEMA_VERSION:
            for record in self.stages:
                if record.status not in {"pending", "blocked", "ready"}:
                    _require_current_document_identity(record.stage, record.document_id)

        running = tuple(record for record in self.stages if record.status == "running")
        if len(running) > 1:
            raise ValueError("a workflow may have at most one running stage")
        expected_active = running[0].run_id if running else None
        if self.active_run_id != expected_active:
            raise ValueError("active_run_id must match the single running stage")
        return self


class TaskSpecV1(StageContractModel):
    model_config = ConfigDict(
        title="CanISendTaskSpecV1",
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/task-spec.schema.json",
        },
    )

    schema_version: StageContractSchemaVersion = TASK_SPEC_SCHEMA_VERSION
    task_id: str
    run_id: str
    job_id: str
    stage: StageName
    document_id: str | None = None
    operation: str
    execution_mode: ExecutionMode
    created_at: AwareDatetime
    input_fingerprint: str
    inputs: tuple[ArtifactFingerprint, ...]
    allowed_reads: tuple[str, ...]
    allowed_writes: tuple[str, ...] = Field(
        description=(
            "Core-service output scope. External executors submit scratch JSON "
            "through stage submit and must not write these paths directly."
        )
    )
    write_authority: Literal["core_service"] = "core_service"
    candidate_output: str
    result_output: str
    authoritative_target: str
    expected_output_sha256: str | None
    output_schema: str
    privacy_tier: PrivacyTier
    required_consents: tuple[str, ...] = ()

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str | None) -> str | None:
        return _optional_document_id(value)

    @field_validator("task_id")
    @classmethod
    def _valid_task_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_TASK_ID_RE, label="task_id")

    @field_validator("run_id")
    @classmethod
    def _valid_run_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_RUN_ID_RE, label="run_id")

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("input_fingerprint")
    @classmethod
    def _valid_input_fingerprint(cls, value: str) -> str:
        return _sha256(value, label="input_fingerprint")

    @field_validator("operation")
    @classmethod
    def _valid_operation(cls, value: str) -> str:
        if _DOTTED_ID_RE.fullmatch(value) is None:
            raise ValueError("operation must be a lowercase dotted identifier")
        return value

    @field_validator("inputs")
    @classmethod
    def _unique_input_paths(
        cls,
        values: tuple[ArtifactFingerprint, ...],
    ) -> tuple[ArtifactFingerprint, ...]:
        _require_unique_paths(values)
        return values

    @field_validator("allowed_reads", "allowed_writes")
    @classmethod
    def _valid_scoped_paths(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        normalized = tuple(_job_relative_path(value) for value in values)
        if len(normalized) != len(set(normalized)):
            raise ValueError("declared paths must be unique")
        return normalized

    @field_validator("candidate_output", "result_output", "authoritative_target")
    @classmethod
    def _valid_output_paths(cls, value: str) -> str:
        return _job_relative_path(value)

    @field_validator("expected_output_sha256")
    @classmethod
    def _valid_expected_output_sha256(cls, value: str | None) -> str | None:
        return _sha256(value, label="expected_output_sha256") if value is not None else None

    @field_validator("output_schema")
    @classmethod
    def _valid_output_schema(cls, value: str) -> str:
        if _OUTPUT_SCHEMA_RE.fullmatch(value) is None:
            raise ValueError("output_schema must be a versioned CanISend schema identifier")
        return value

    @field_validator("required_consents")
    @classmethod
    def _valid_consents(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        for value in values:
            if _SLUG_ID_RE.fullmatch(value) is None:
                raise ValueError("consent identifiers must be lowercase slugs")
        if len(values) != len(set(values)):
            raise ValueError("consent identifiers must be unique")
        return values

    @model_validator(mode="after")
    def _consistent_scope(self) -> TaskSpecV1:
        _require_document_scope(self.stage, self.document_id)
        if self.schema_version == TASK_SPEC_SCHEMA_VERSION:
            _require_current_document_identity(self.stage, self.document_id)
        if self.operation != f"stage.{self.stage}":
            raise ValueError("operation must match the task stage")
        if not self.allowed_reads:
            raise ValueError("a task must declare at least one allowed read")
        if not self.allowed_writes:
            raise ValueError("a task must declare at least one allowed write")
        reads = set(self.allowed_reads)
        writes = set(self.allowed_writes)
        if reads & writes:
            raise ValueError("allowed reads and writes must not overlap")
        if not {artifact.path for artifact in self.inputs}.issubset(reads):
            raise ValueError("every input fingerprint must be inside allowed_reads")
        declared_outputs = {self.candidate_output, self.result_output}
        if len(declared_outputs) != 2:
            raise ValueError("candidate_output and result_output must be different paths")
        if writes != declared_outputs:
            raise ValueError("allowed_writes must exactly cover candidate_output and result_output")
        if self.authoritative_target in writes:
            raise ValueError("authoritative_target must not be directly writable by the task")
        return self


class TaskResultV1(StageContractModel):
    model_config = ConfigDict(
        title="CanISendTaskResultV1",
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/task-result.schema.json",
        },
    )

    schema_version: StageContractSchemaVersion = TASK_RESULT_SCHEMA_VERSION
    task_id: str
    run_id: str
    job_id: str
    stage: StageName
    document_id: str | None = None
    status: TaskResultStatus
    input_fingerprint: str
    started_at: AwareDatetime
    completed_at: AwareDatetime
    outputs: tuple[ArtifactFingerprint, ...] = ()
    error_code: str | None = None
    error_message: str | None = None

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str | None) -> str | None:
        return _optional_document_id(value)

    @field_validator("task_id")
    @classmethod
    def _valid_task_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_TASK_ID_RE, label="task_id")

    @field_validator("run_id")
    @classmethod
    def _valid_run_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_RUN_ID_RE, label="run_id")

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("input_fingerprint")
    @classmethod
    def _valid_input_fingerprint(cls, value: str) -> str:
        return _sha256(value, label="input_fingerprint")

    @field_validator("outputs")
    @classmethod
    def _unique_output_paths(
        cls,
        values: tuple[ArtifactFingerprint, ...],
    ) -> tuple[ArtifactFingerprint, ...]:
        _require_unique_paths(values)
        return values

    @field_validator("error_code")
    @classmethod
    def _valid_error_code(cls, value: str | None) -> str | None:
        return _optional_dotted_id(value, label="error_code")

    @model_validator(mode="after")
    def _consistent_result(self) -> TaskResultV1:
        _require_document_scope(self.stage, self.document_id)
        if self.schema_version == TASK_RESULT_SCHEMA_VERSION:
            _require_current_document_identity(self.stage, self.document_id)
        _require_time_order(self.started_at, self.completed_at, "task result timestamps")
        has_error = self.error_code is not None or self.error_message is not None
        if (self.error_code is None) != (self.error_message is None):
            raise ValueError("error_code and error_message must appear together")
        if self.status == "succeeded":
            if not self.outputs:
                raise ValueError("a successful task result requires output fingerprints")
            if has_error:
                raise ValueError("a successful task result must not include an error")
        elif not has_error:
            raise ValueError("a failed or cancelled task result requires an error")
        return self


class CandidateSubmissionV1(StageContractModel):
    schema_version: StageContractSchemaVersion = CANDIDATE_SUBMISSION_SCHEMA_VERSION
    task_id: str
    run_id: str
    job_id: str
    stage: StageName
    document_id: str | None = None
    submitted_at: AwareDatetime
    task_spec_sha256: str
    candidate: ArtifactFingerprint
    result_path: str
    task_result_sha256: str

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str | None) -> str | None:
        return _optional_document_id(value)

    @field_validator("task_id")
    @classmethod
    def _valid_task_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_TASK_ID_RE, label="task_id")

    @field_validator("run_id")
    @classmethod
    def _valid_run_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_RUN_ID_RE, label="run_id")

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("task_spec_sha256", "task_result_sha256")
    @classmethod
    def _valid_hashes(cls, value: str, info: object) -> str:
        return _sha256(value, label=getattr(info, "field_name", "sha256"))

    @field_validator("result_path")
    @classmethod
    def _valid_result_path(cls, value: str) -> str:
        return _job_relative_path(value)

    @model_validator(mode="after")
    def _consistent_document_scope(self) -> CandidateSubmissionV1:
        _require_document_scope(self.stage, self.document_id)
        if self.schema_version == CANDIDATE_SUBMISSION_SCHEMA_VERSION:
            _require_current_document_identity(self.stage, self.document_id)
        return self


class ValidationReportV1(StageContractModel):
    schema_version: StageContractSchemaVersion = TASK_RESULT_SCHEMA_VERSION
    task_id: str
    run_id: str
    job_id: str
    stage: StageName
    document_id: str | None = None
    status: ValidationStatus
    checked_at: AwareDatetime
    input_hashes_match: bool
    schema_valid: bool
    scope_valid: bool
    citations_valid: bool | None = None
    errors: tuple[str, ...] = ()
    warnings: tuple[str, ...] = ()

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str | None) -> str | None:
        return _optional_document_id(value)

    @field_validator("task_id")
    @classmethod
    def _valid_task_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_TASK_ID_RE, label="task_id")

    @field_validator("run_id")
    @classmethod
    def _valid_run_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_RUN_ID_RE, label="run_id")

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("errors", "warnings")
    @classmethod
    def _nonempty_messages(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        if any(not value for value in values):
            raise ValueError("validation messages must not be empty")
        return values

    @model_validator(mode="after")
    def _consistent_status(self) -> ValidationReportV1:
        _require_document_scope(self.stage, self.document_id)
        if self.schema_version == TASK_RESULT_SCHEMA_VERSION:
            _require_current_document_identity(self.stage, self.document_id)
        checks_pass = (
            self.input_hashes_match
            and self.schema_valid
            and self.scope_valid
            and self.citations_valid is not False
        )
        valid = checks_pass and not self.errors
        if self.status == "passed" and not valid:
            raise ValueError("a passed validation report requires all checks to pass and no errors")
        if self.status == "failed" and valid:
            raise ValueError("a failed validation report requires a failed check or error")
        return self


class RunManifestV1(StageContractModel):
    model_config = ConfigDict(
        title="CanISendRunManifestV1",
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/run-manifest.schema.json",
        },
    )

    schema_version: StageContractSchemaVersion = RUN_MANIFEST_SCHEMA_VERSION
    run_id: str
    task_id: str | None = None
    job_id: str
    stage: StageName
    document_id: str | None = None
    attempt: int = Field(ge=1)
    execution_mode: ExecutionMode
    status: RunStatus
    created_at: AwareDatetime
    started_at: AwareDatetime | None = None
    completed_at: AwareDatetime | None = None
    inputs: tuple[ArtifactFingerprint, ...] = ()
    input_fingerprint: str
    task_spec_sha256: str
    candidate_outputs: tuple[ArtifactFingerprint, ...] = ()
    promoted_outputs: tuple[ArtifactFingerprint, ...] = ()
    validation_report_path: str | None = None
    error_code: str | None = None
    error_message: str | None = None

    @field_validator("document_id")
    @classmethod
    def _valid_document_id(cls, value: str | None) -> str | None:
        return _optional_document_id(value)

    @field_validator("run_id")
    @classmethod
    def _valid_run_id(cls, value: str) -> str:
        return _prefixed_id(value, pattern=_RUN_ID_RE, label="run_id")

    @field_validator("task_id")
    @classmethod
    def _valid_task_id(cls, value: str | None) -> str | None:
        return _optional_prefixed_id(value, pattern=_TASK_ID_RE, label="task_id")

    @field_validator("job_id")
    @classmethod
    def _valid_job_id(cls, value: str) -> str:
        return _job_id(value)

    @field_validator("input_fingerprint", "task_spec_sha256")
    @classmethod
    def _valid_manifest_hashes(cls, value: str, info: object) -> str:
        field_name = getattr(info, "field_name", "sha256")
        return _sha256(value, label=field_name)

    @field_validator("inputs", "candidate_outputs", "promoted_outputs")
    @classmethod
    def _unique_artifact_paths(
        cls,
        values: tuple[ArtifactFingerprint, ...],
    ) -> tuple[ArtifactFingerprint, ...]:
        _require_unique_paths(values)
        return values

    @field_validator("validation_report_path")
    @classmethod
    def _valid_validation_path(cls, value: str | None) -> str | None:
        return _job_relative_path(value) if value is not None else None

    @field_validator("error_code")
    @classmethod
    def _valid_error_code(cls, value: str | None) -> str | None:
        return _optional_dotted_id(value, label="error_code")

    @model_validator(mode="after")
    def _consistent_lifecycle(self) -> RunManifestV1:
        _require_document_scope(self.stage, self.document_id)
        if self.schema_version == RUN_MANIFEST_SCHEMA_VERSION:
            _require_current_document_identity(self.stage, self.document_id)
        if self.execution_mode in {"host_agent", "configured_provider"} and self.task_id is None:
            raise ValueError("an externally reasoned run requires task_id")
        if self.started_at is not None:
            _require_time_order(self.created_at, self.started_at, "run start timestamp")
        if self.completed_at is not None:
            if self.started_at is None:
                raise ValueError("completed_at requires started_at")
            _require_time_order(self.started_at, self.completed_at, "run completion timestamp")

        has_error = self.error_code is not None or self.error_message is not None
        if (self.error_code is None) != (self.error_message is None):
            raise ValueError("error_code and error_message must appear together")

        if self.status == "prepared":
            if self.started_at is not None or self.completed_at is not None:
                raise ValueError("a prepared run must not have execution timestamps")
        elif self.status == "running":
            if self.started_at is None or self.completed_at is not None:
                raise ValueError("a running run requires only started_at")
        else:
            if self.started_at is None or self.completed_at is None:
                raise ValueError("a terminal run requires started_at and completed_at")

        if self.status == "succeeded":
            if not self.promoted_outputs or self.validation_report_path is None:
                raise ValueError("a successful run requires promoted outputs and validation report")
            if has_error:
                raise ValueError("a successful run must not include an error")
        elif self.status in {"failed", "cancelled"}:
            if self.promoted_outputs:
                raise ValueError("a failed or cancelled run must not record promoted outputs")
            if not has_error:
                raise ValueError("a failed or cancelled run requires an error")
        elif self.promoted_outputs or self.validation_report_path is not None or has_error:
            raise ValueError("a nonterminal run must not record promotion, validation, or errors")
        return self


def _job_relative_path(value: str) -> str:
    if not value or "\\" in value or "\x00" in value:
        raise ValueError("path must be a non-empty POSIX path")
    windows_path = PureWindowsPath(value)
    path = PurePosixPath(value)
    if path.is_absolute() or windows_path.is_absolute() or windows_path.drive:
        raise ValueError("path must be job-relative")
    if any(part in {"", ".", ".."} for part in value.split("/")):
        raise ValueError("path must not contain empty, dot, or parent segments")
    if path.as_posix() != value:
        raise ValueError("path must be normalized")
    return value


def _job_id(value: str) -> str:
    if _JOB_ID_RE.fullmatch(value) is None:
        raise ValueError("job_id must be a lowercase slug identifier")
    return value


def _sha256(value: str, *, label: str) -> str:
    if _SHA256_RE.fullmatch(value) is None:
        raise ValueError(f"{label} must be a lowercase 64-character digest")
    return value


def _prefixed_id(value: str, *, pattern: re.Pattern[str], label: str) -> str:
    if pattern.fullmatch(value) is None:
        raise ValueError(f"{label} is invalid")
    return value


def _optional_prefixed_id(
    value: str | None,
    *,
    pattern: re.Pattern[str],
    label: str,
) -> str | None:
    if value is None:
        return None
    return _prefixed_id(value, pattern=pattern, label=label)


def _optional_document_id(value: str | None) -> str | None:
    if value is not None and _DOCUMENT_ID_RE.fullmatch(value) is None:
        raise ValueError("document_id must be a stable document identifier")
    return value


def _require_document_scope(stage: StageName, document_id: str | None) -> None:
    if document_id is not None and stage not in {"draft", "review"}:
        raise ValueError("document_id is only valid for a document-scoped stage")


def _require_current_document_identity(
    stage: StageName,
    document_id: str | None,
) -> None:
    if stage in {"draft", "review"} and document_id is None:
        raise ValueError("a current document-scoped stage requires document_id")


def _optional_dotted_id(value: str | None, *, label: str) -> str | None:
    if value is not None and _DOTTED_ID_RE.fullmatch(value) is None:
        raise ValueError(f"{label} must be a lowercase dotted identifier")
    return value


def _require_unique_paths(values: tuple[ArtifactFingerprint, ...]) -> None:
    paths = tuple(value.path for value in values)
    if len(paths) != len(set(paths)):
        raise ValueError("artifact paths must be unique")


def _require_time_order(start: datetime, end: datetime, label: str) -> None:
    if end < start:
        raise ValueError(f"{label} must be monotonic")
