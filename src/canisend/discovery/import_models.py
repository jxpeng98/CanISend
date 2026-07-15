from __future__ import annotations

from datetime import datetime
from hashlib import sha256
import json
from pathlib import PurePosixPath, PureWindowsPath
import re
from typing import Literal

from pydantic import ConfigDict, Field, StrictBool, StrictInt, field_validator, model_validator

from canisend.discovery.catalog_models import CatalogIdentifier
from canisend.discovery.models import (
    DiscoveryContractModel,
    DiscoveryTimestamp,
    DottedIdentifier,
    JSON_SCHEMA_DIALECT,
    SCHEMA_BASE_ID,
)
from canisend.discovery.refresh_models import (
    BatchIdentifier,
    Sha256Digest,
    SourceIdentifier,
)


DISCOVERY_IMPORT_REPORT_PROTOCOL = "canisend.discovery-import-report/v1"
DISCOVERY_IMPORT_REPORT_SCHEMA_VERSION = "1.0.0"

_IMPORT_ID_RE = re.compile(r"^import_[0-9a-f]{32}$")
_CONTROL_RE = re.compile(r"[\x00-\x1f\x7f]")
_EMAIL_RE = re.compile(
    r"(?i)(?<![A-Z0-9._%+-])[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}"
)
_INLINE_SECRET_RE = re.compile(
    r"(?i)(?:access[_-]?token|api[_-]?key|auth|credential|password|secret|"
    r"session|signature|token)\s*[:=]"
)

ImportIdentifier = str
ImportFormat = Literal["csv", "json", "eml", "mbox"]
ImportStatus = Literal["complete", "partial", "failed"]
ImportIssueField = Literal[
    "record",
    "title",
    "source_url",
    "description",
    "published_at",
    "source_record_id",
    "institution",
    "location",
    "deadline",
    "link",
]


class DiscoveryImportIssueV1(DiscoveryContractModel):
    record_number: StrictInt = Field(ge=1, le=100_000)
    code: DottedIdentifier
    field: ImportIssueField = "record"

    @field_validator("code")
    @classmethod
    def _import_issue_code(cls, value: str) -> str:
        if not value.startswith("import."):
            raise ValueError("import issue codes must use the import.* namespace")
        return value


class DiscoveryImportReportV1(DiscoveryContractModel):
    model_config = ConfigDict(
        title="CanISendDiscoveryImportReportV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/discovery-import-report-v1.schema.json",
        },
    )

    protocol: Literal["canisend.discovery-import-report/v1"] = (
        DISCOVERY_IMPORT_REPORT_PROTOCOL
    )
    schema_version: Literal["1.0.0"] = DISCOVERY_IMPORT_REPORT_SCHEMA_VERSION
    import_id: ImportIdentifier = Field(pattern=_IMPORT_ID_RE.pattern)
    imported_at: DiscoveryTimestamp
    format: ImportFormat
    source_id: SourceIdentifier
    source_name: str = Field(min_length=1, max_length=256)
    input_sha256: Sha256Digest
    status: ImportStatus
    catalog_promoted: StrictBool
    error_code: DottedIdentifier | None = None

    input_records: StrictInt = Field(ge=0, le=100_000)
    imported_records: StrictInt = Field(ge=0, le=100_000)
    rejected_records: StrictInt = Field(ge=0, le=100_000)
    ignored_records: StrictInt = Field(ge=0, le=100_000)
    issue_count: StrictInt = Field(ge=0, le=100_000)
    issues_truncated: StrictBool = False
    issues: tuple[DiscoveryImportIssueV1, ...] = Field(
        default=(), max_length=1_000
    )
    skipped_batches: StrictInt = Field(default=0, ge=0, le=100_000)

    batch_id: BatchIdentifier | None = None
    batch_path: str | None = Field(default=None, max_length=1_024)
    catalog_id: CatalogIdentifier | None = None
    catalog_path: str | None = Field(default=None, max_length=1_024)
    catalog_input_records: StrictInt = Field(default=0, ge=0, le=10_000_000)
    merged_records: StrictInt = Field(default=0, ge=0, le=10_000_000)
    retained_records: StrictInt = Field(default=0, ge=0, le=10_000_000)
    excluded_records: StrictInt = Field(default=0, ge=0, le=10_000_000)

    @field_validator("import_id")
    @classmethod
    def _valid_import_id(cls, value: str) -> str:
        if _IMPORT_ID_RE.fullmatch(value) is None:
            raise ValueError("import_id must use the import_<32 lowercase hex> format")
        return value

    @field_validator("source_name")
    @classmethod
    def _private_safe_source_name(cls, value: str) -> str:
        if (
            _CONTROL_RE.search(value)
            or _EMAIL_RE.search(value)
            or _INLINE_SECRET_RE.search(value)
            or value.startswith(("/", "\\", "file:"))
            or PureWindowsPath(value).is_absolute()
        ):
            raise ValueError("import source name must be a private-safe label")
        return value

    @field_validator("batch_path", "catalog_path")
    @classmethod
    def _relative_artifact_path(cls, value: str | None) -> str | None:
        return _safe_relative_path(value) if value is not None else None

    @field_validator("error_code")
    @classmethod
    def _body_free_error_code(cls, value: str | None) -> str | None:
        if value is not None and not value.startswith(("import.", "store.", "catalog.")):
            raise ValueError("import error code is outside the stable namespace")
        return value

    @field_validator("issues")
    @classmethod
    def _ordered_unique_issues(
        cls, values: tuple[DiscoveryImportIssueV1, ...]
    ) -> tuple[DiscoveryImportIssueV1, ...]:
        keys = tuple((item.record_number, item.code, item.field) for item in values)
        if len(keys) != len(set(keys)) or keys != tuple(sorted(keys)):
            raise ValueError("import issues must be sorted and unique")
        return values

    @model_validator(mode="after")
    def _consistent_report(self) -> DiscoveryImportReportV1:
        if (self.batch_id is None) != (self.batch_path is None):
            raise ValueError("import batch identity and path must appear together")
        if self.input_records != (
            self.imported_records + self.rejected_records + self.ignored_records
        ):
            raise ValueError("import record counts do not match")
        if self.issue_count != self.rejected_records:
            raise ValueError("import issue count must match rejected records")
        if self.issue_count < len(self.issues):
            raise ValueError("import issue count cannot be smaller than stored issues")
        if self.issues_truncated != (self.issue_count > len(self.issues)):
            raise ValueError("import issue truncation flag does not match issue counts")

        expected_status = (
            "failed"
            if not self.catalog_promoted
            else "partial"
            if self.rejected_records
            else "complete"
        )
        if self.status != expected_status:
            raise ValueError("import status does not match promotion and row outcomes")
        if self.catalog_promoted:
            if (
                self.imported_records < 1
                or not self.batch_id
                or not self.batch_path
                or not self.catalog_id
                or not self.catalog_path
                or self.error_code is not None
            ):
                raise ValueError("promoted imports require batch and catalog artifacts")
            if self.catalog_input_records < self.imported_records:
                raise ValueError("catalog input count cannot be smaller than imported records")
            if (
                self.merged_records
                + self.retained_records
                + self.excluded_records
                != self.catalog_input_records
            ):
                raise ValueError("catalog counts do not reconcile with catalog inputs")
        else:
            if self.catalog_id or self.catalog_path or self.error_code is None:
                raise ValueError("failed imports require a body-free error code")
            if any(
                (
                    self.catalog_input_records,
                    self.merged_records,
                    self.retained_records,
                    self.excluded_records,
                )
            ):
                raise ValueError("failed imports must not claim promoted catalog counts")

        expected_id = import_identifier(
            imported_at=self.imported_at,
            format=self.format,
            source_id=self.source_id,
            input_sha256=self.input_sha256,
            status=self.status,
            batch_id=self.batch_id,
            catalog_id=self.catalog_id,
            error_code=self.error_code,
            input_records=self.input_records,
            imported_records=self.imported_records,
            rejected_records=self.rejected_records,
            ignored_records=self.ignored_records,
            issue_count=self.issue_count,
            skipped_batches=self.skipped_batches,
        )
        if self.import_id != expected_id:
            raise ValueError("import_id does not match deterministic report content")
        return self


def import_identifier(
    *,
    imported_at: datetime,
    format: str,
    source_id: str,
    input_sha256: str,
    status: str,
    batch_id: str | None,
    catalog_id: str | None,
    error_code: str | None,
    input_records: int,
    imported_records: int,
    rejected_records: int,
    ignored_records: int,
    issue_count: int,
    skipped_batches: int,
) -> str:
    payload = {
        "imported_at": imported_at.isoformat(),
        "format": format,
        "source_id": source_id,
        "input_sha256": input_sha256,
        "status": status,
        "batch_id": batch_id,
        "catalog_id": catalog_id,
        "error_code": error_code,
        "input_records": input_records,
        "imported_records": imported_records,
        "rejected_records": rejected_records,
        "ignored_records": ignored_records,
        "issue_count": issue_count,
        "skipped_batches": skipped_batches,
    }
    serialized = json.dumps(
        payload,
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )
    return f"import_{sha256(serialized.encode('utf-8')).hexdigest()[:32]}"


def _safe_relative_path(value: str) -> str:
    if not value or _CONTROL_RE.search(value) or "\\" in value:
        raise ValueError("artifact path must be a safe workspace-relative path")
    posix = PurePosixPath(value)
    windows = PureWindowsPath(value)
    if (
        posix.is_absolute()
        or windows.is_absolute()
        or windows.drive
        or any(part in {"", ".", ".."} for part in posix.parts)
    ):
        raise ValueError("artifact path must be a safe workspace-relative path")
    return posix.as_posix()
