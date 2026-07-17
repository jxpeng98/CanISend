from __future__ import annotations

from typing import Literal

from pydantic import AwareDatetime, BaseModel, ConfigDict, Field, field_validator, model_validator

from canisend.stage_models import ArtifactFingerprint


RECOVERY_SCHEMA_VERSION = "1.0.0"
STAGE5_MIGRATION_ID = "stage5-v1"

MigrationSourceShape = Literal[
    "pre_workflow",
    "prior_schema",
    "current_unmigrated",
]
MigrationChangeAction = Literal["created", "replaced"]
RollbackOutcome = Literal[
    "removed",
    "restored",
    "already_rolled_back",
    "conflict",
]
RepairKind = Literal["projection", "state"]
RepairOutcome = Literal[
    "created",
    "replaced",
    "unchanged",
    "candidate_created",
    "candidate_replaced",
]


class RecoveryModel(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
    )


class MigrationChangeV1(RecoveryModel):
    path: str
    action: MigrationChangeAction
    before_sha256: str | None = None
    after_sha256: str
    backup_path: str | None = None
    backup_sha256: str | None = None

    @field_validator("path", "backup_path")
    @classmethod
    def _paths(cls, value: str | None) -> str | None:
        return _job_relative_path(value) if value is not None else None

    @field_validator("before_sha256", "after_sha256", "backup_sha256")
    @classmethod
    def _hashes(cls, value: str | None) -> str | None:
        return _sha256(value) if value is not None else None

    @model_validator(mode="after")
    def _consistent_action(self) -> MigrationChangeV1:
        if self.action == "created" and any(
            value is not None
            for value in (self.before_sha256, self.backup_path, self.backup_sha256)
        ):
            raise ValueError("created migration metadata cannot have a backup")
        if self.action == "replaced" and any(
            value is None
            for value in (self.before_sha256, self.backup_path, self.backup_sha256)
        ):
            raise ValueError("replaced migration metadata requires an exact backup")
        return self


class MigrationPlanV1(RecoveryModel):
    schema_version: Literal["1.0.0"] = RECOVERY_SCHEMA_VERSION
    migration_id: Literal["stage5-v1"] = STAGE5_MIGRATION_ID
    job_id: str
    source_shape: MigrationSourceShape
    planned_at: AwareDatetime
    observed_metadata: tuple[ArtifactFingerprint, ...]
    changes: tuple[MigrationChangeV1, ...]

    @field_validator("job_id")
    @classmethod
    def _job_id(cls, value: str) -> str:
        return _slug(value, label="job_id")

    @model_validator(mode="after")
    def _unique_paths(self) -> MigrationPlanV1:
        observed = tuple(item.path for item in self.observed_metadata)
        changes = tuple(item.path for item in self.changes)
        if observed != tuple(sorted(observed)) or len(observed) != len(set(observed)):
            raise ValueError("observed migration metadata must use sorted unique paths")
        if changes != tuple(sorted(changes)) or len(changes) != len(set(changes)):
            raise ValueError("migration changes must use sorted unique paths")
        return self


class MigrationReceiptV1(RecoveryModel):
    schema_version: Literal["1.0.0"] = RECOVERY_SCHEMA_VERSION
    migration_id: Literal["stage5-v1"] = STAGE5_MIGRATION_ID
    job_id: str
    source_shape: MigrationSourceShape
    applied_at: AwareDatetime
    plan_sha256: str
    observed_metadata: tuple[ArtifactFingerprint, ...]
    changes: tuple[MigrationChangeV1, ...]

    @field_validator("job_id")
    @classmethod
    def _job_id(cls, value: str) -> str:
        return _slug(value, label="job_id")

    @field_validator("plan_sha256")
    @classmethod
    def _plan_hash(cls, value: str) -> str:
        return _sha256(value)

    @model_validator(mode="after")
    def _unique_paths(self) -> MigrationReceiptV1:
        MigrationPlanV1(
            source_shape=self.source_shape,
            job_id=self.job_id,
            planned_at=self.applied_at,
            observed_metadata=self.observed_metadata,
            changes=self.changes,
        )
        return self


class RollbackEntryV1(RecoveryModel):
    path: str
    outcome: RollbackOutcome
    expected_after_sha256: str
    observed_sha256: str | None = None
    restored_sha256: str | None = None

    @field_validator("path")
    @classmethod
    def _path(cls, value: str) -> str:
        return _job_relative_path(value)

    @field_validator(
        "expected_after_sha256",
        "observed_sha256",
        "restored_sha256",
    )
    @classmethod
    def _hashes(cls, value: str | None) -> str | None:
        return _sha256(value) if value is not None else None


class MigrationRollbackReceiptV1(RecoveryModel):
    schema_version: Literal["1.0.0"] = RECOVERY_SCHEMA_VERSION
    rollback_id: str
    migration_id: Literal["stage5-v1"] = STAGE5_MIGRATION_ID
    job_id: str
    completed_at: AwareDatetime
    status: Literal["complete", "conflict"]
    migration_receipt_sha256: str
    entries: tuple[RollbackEntryV1, ...]

    @field_validator("rollback_id")
    @classmethod
    def _rollback_id(cls, value: str) -> str:
        if not value.startswith("rollback_") or len(value) != 41:
            raise ValueError("rollback_id must contain one UUID hex identifier")
        _hex(value.removeprefix("rollback_"), length=32, label="rollback_id")
        return value

    @field_validator("job_id")
    @classmethod
    def _job_id(cls, value: str) -> str:
        return _slug(value, label="job_id")

    @field_validator("migration_receipt_sha256")
    @classmethod
    def _receipt_hash(cls, value: str) -> str:
        return _sha256(value)

    @model_validator(mode="after")
    def _consistent_status(self) -> MigrationRollbackReceiptV1:
        paths = tuple(entry.path for entry in self.entries)
        if paths != tuple(sorted(paths)) or len(paths) != len(set(paths)):
            raise ValueError("rollback entries must use sorted unique paths")
        expected = "conflict" if any(entry.outcome == "conflict" for entry in self.entries) else "complete"
        if self.status != expected:
            raise ValueError("rollback status must match its entry outcomes")
        return self


class RepairEntryV1(RecoveryModel):
    path: str
    outcome: RepairOutcome
    before_sha256: str | None = None
    after_sha256: str

    @field_validator("path")
    @classmethod
    def _path(cls, value: str) -> str:
        return _job_relative_path(value)

    @field_validator("before_sha256", "after_sha256")
    @classmethod
    def _hashes(cls, value: str | None) -> str | None:
        return _sha256(value) if value is not None else None


class RepairReceiptV1(RecoveryModel):
    schema_version: Literal["1.0.0"] = RECOVERY_SCHEMA_VERSION
    repair_id: str
    job_id: str
    kind: RepairKind
    stage: Literal["package", "render"] | None = None
    completed_at: AwareDatetime
    source_sha256: str | None = None
    entries: tuple[RepairEntryV1, ...]

    @field_validator("repair_id")
    @classmethod
    def _repair_id(cls, value: str) -> str:
        if not value.startswith("repair_") or len(value) != 39:
            raise ValueError("repair_id must contain one UUID hex identifier")
        _hex(value.removeprefix("repair_"), length=32, label="repair_id")
        return value

    @field_validator("job_id")
    @classmethod
    def _job_id(cls, value: str) -> str:
        return _slug(value, label="job_id")

    @field_validator("source_sha256")
    @classmethod
    def _source_hash(cls, value: str | None) -> str | None:
        return _sha256(value) if value is not None else None

    @model_validator(mode="after")
    def _consistent_kind(self) -> RepairReceiptV1:
        if (self.kind == "projection") != (self.stage is not None):
            raise ValueError("only projection repair names a bundle stage")
        paths = tuple(entry.path for entry in self.entries)
        if paths != tuple(sorted(paths)) or len(paths) != len(set(paths)):
            raise ValueError("repair entries must use sorted unique paths")
        return self


def _job_relative_path(value: str) -> str:
    from pathlib import PurePosixPath, PureWindowsPath

    if "\\" in value or PureWindowsPath(value).drive:
        raise ValueError("path must use job-relative POSIX syntax")
    path = PurePosixPath(value)
    if path.is_absolute() or value in {"", "."} or any(
        part in {"", ".", ".."} for part in path.parts
    ):
        raise ValueError("path must be a normalized job-relative path")
    return path.as_posix()


def _sha256(value: str) -> str:
    return _hex(value, length=64, label="sha256")


def _hex(value: str, *, length: int, label: str) -> str:
    if len(value) != length or any(character not in "0123456789abcdef" for character in value):
        raise ValueError(f"{label} must be lowercase hexadecimal")
    return value


def _slug(value: str, *, label: str) -> str:
    import re

    if re.fullmatch(r"[a-z0-9][a-z0-9_.-]*", value) is None:
        raise ValueError(f"{label} must be a lowercase slug")
    return value
