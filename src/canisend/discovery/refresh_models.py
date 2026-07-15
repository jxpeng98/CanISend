from __future__ import annotations

from datetime import datetime
from hashlib import sha256
import json
from pathlib import PurePosixPath, PureWindowsPath
import re
from typing import Annotated, Literal
from urllib.parse import parse_qsl, urlsplit

from pydantic import (
    AfterValidator,
    BeforeValidator,
    ConfigDict,
    Field,
    StrictBool,
    StrictInt,
    field_validator,
    model_validator,
)

from canisend.discovery.catalog_models import CatalogIdentifier, RankingPolicyV1
from canisend.discovery.identity import redact_feed_url
from canisend.discovery.models import (
    DiscoveryContractModel,
    DiscoveryTimestamp,
    DottedIdentifier,
    JobLeadV2,
    JSON_SCHEMA_DIALECT,
    SCHEMA_BASE_ID,
)
from canisend.discovery.transport import PublicTransportError, redact_public_url


DISCOVERY_SOURCES_PROTOCOL = "canisend.discovery-sources/v1"
DISCOVERY_SOURCES_SCHEMA_VERSION = "1.0.0"
LEAD_BATCH_PROTOCOL = "canisend.discovery-batch/v1"
LEAD_BATCH_SCHEMA_VERSION = "1.0.0"
DISCOVERY_CACHE_PROTOCOL = "canisend.discovery-cache/v1"
DISCOVERY_CACHE_SCHEMA_VERSION = "1.0.0"
DISCOVERY_REFRESH_REPORT_PROTOCOL = "canisend.discovery-refresh-report/v1"
DISCOVERY_REFRESH_REPORT_SCHEMA_VERSION = "1.0.0"

_SOURCE_ID_RE = re.compile(r"^[a-z](?:[a-z0-9._-]{0,62}[a-z0-9_])?$")
_BATCH_ID_RE = re.compile(r"^batch_[0-9a-f]{32}$")
_REFRESH_ID_RE = re.compile(r"^refresh_[0-9a-f]{32}$")
_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_CONTROL_RE = re.compile(r"[\x00-\x1f\x7f]")
_SENSITIVE_QUERY_NAMES = frozenset(
    {
        "access_token",
        "api-key",
        "api_key",
        "apikey",
        "auth",
        "authorization",
        "credential",
        "credentials",
        "id_token",
        "key",
        "password",
        "passwd",
        "secret",
        "session",
        "session_id",
        "sessionid",
        "sig",
        "signature",
        "token",
    }
)


def _validated_source_id(value: str) -> str:
    if ".." in value:
        raise ValueError("source_id must not contain path traversal syntax")
    return value


SourceIdentifier = Annotated[
    str,
    Field(pattern=_SOURCE_ID_RE.pattern),
    AfterValidator(_validated_source_id),
]
BatchIdentifier = Annotated[str, Field(pattern=_BATCH_ID_RE.pattern)]
RefreshIdentifier = Annotated[str, Field(pattern=_REFRESH_ID_RE.pattern)]
Sha256Digest = Annotated[str, Field(pattern=_SHA256_RE.pattern)]
BoundedSeconds = Annotated[
    float,
    BeforeValidator(lambda value: _bounded_number(value, label="seconds")),
    Field(ge=0, le=3_600),
]


class DiscoverySourceV1(DiscoveryContractModel):
    source_id: SourceIdentifier
    name: str = Field(min_length=1, max_length=256)
    kind: Literal["rss_atom"] = "rss_atom"
    url: str = Field(min_length=1, max_length=8_192)
    enabled: StrictBool = True
    timeout_seconds: StrictInt = Field(default=30, ge=1, le=300)
    max_bytes: StrictInt = Field(default=2_000_000, ge=1, le=100_000_000)
    max_leads: StrictInt = Field(default=10_000, ge=1, le=100_000)
    max_attempts: StrictInt = Field(default=3, ge=1, le=5)
    backoff_seconds: BoundedSeconds = 1.0
    max_retry_delay_seconds: BoundedSeconds = 300.0
    min_interval_seconds: BoundedSeconds = 0.0

    @field_validator("source_id")
    @classmethod
    def _safe_source_id(cls, value: str) -> str:
        if _SOURCE_ID_RE.fullmatch(value) is None or value.endswith((".", "-")):
            raise ValueError("source_id must be a safe lowercase path identifier")
        if ".." in value:
            raise ValueError("source_id must not contain path traversal syntax")
        return value

    @field_validator("name")
    @classmethod
    def _safe_name(cls, value: str) -> str:
        if _CONTROL_RE.search(value):
            raise ValueError("source name must not contain control characters")
        return value

    @field_validator("url")
    @classmethod
    def _public_credential_free_url(cls, value: str) -> str:
        raw = value.strip()
        try:
            redact_public_url(raw)
        except PublicTransportError as exc:
            raise ValueError("source URL must be a valid public HTTP(S) URL") from exc
        parsed = urlsplit(raw)
        if parsed.fragment:
            raise ValueError("source URL must not contain a fragment")
        for name, _ in parse_qsl(parsed.query, keep_blank_values=True):
            normalized = name.casefold()
            parts = {
                part for part in re.split(r"[^a-z0-9]+", normalized) if part
            }
            if normalized in _SENSITIVE_QUERY_NAMES or parts & {
                "auth",
                "credential",
                "password",
                "secret",
                "session",
                "signature",
                "token",
            }:
                raise ValueError("source URL must not contain credential-like query fields")
        return raw


class DiscoverySourcesV1(DiscoveryContractModel):
    model_config = ConfigDict(
        title="CanISendDiscoverySourcesV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/discovery-sources-v1.schema.json",
        },
    )

    protocol: Literal["canisend.discovery-sources/v1"] = DISCOVERY_SOURCES_PROTOCOL
    schema_version: Literal["1.0.0"] = DISCOVERY_SOURCES_SCHEMA_VERSION
    policy: RankingPolicyV1 = Field(default_factory=RankingPolicyV1)
    sources: tuple[DiscoverySourceV1, ...] = Field(min_length=1, max_length=1_000)

    @field_validator("sources")
    @classmethod
    def _unique_sources(
        cls, values: tuple[DiscoverySourceV1, ...]
    ) -> tuple[DiscoverySourceV1, ...]:
        ids = tuple(item.source_id for item in values)
        if len(ids) != len(set(ids)):
            raise ValueError("discovery source IDs must be unique")
        return values

    @model_validator(mode="after")
    def _has_enabled_source(self) -> DiscoverySourcesV1:
        if not any(source.enabled for source in self.sources):
            raise ValueError("discovery source configuration requires an enabled source")
        return self


class LeadBatchV1(DiscoveryContractModel):
    model_config = ConfigDict(
        title="CanISendLeadBatchV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/lead-batch-v1.schema.json",
        },
    )

    protocol: Literal["canisend.discovery-batch/v1"] = LEAD_BATCH_PROTOCOL
    schema_version: Literal["1.0.0"] = LEAD_BATCH_SCHEMA_VERSION
    batch_id: BatchIdentifier
    source_id: SourceIdentifier
    source_name: str = Field(min_length=1, max_length=256)
    adapter: DottedIdentifier = Field(default="feed.rss_atom", max_length=128)
    source_url: str = Field(min_length=1, max_length=8_192)
    fetched_at: DiscoveryTimestamp
    content_sha256: Sha256Digest
    record_count: StrictInt = Field(ge=0, le=100_000)
    leads: tuple[JobLeadV2, ...] = Field(default=(), max_length=100_000)

    @field_validator("source_url")
    @classmethod
    def _redacted_source_url(cls, value: str) -> str:
        if redact_feed_url(value) != value:
            raise ValueError("batch source locator must be redacted and path-free")
        return value

    @field_validator("adapter")
    @classmethod
    def _safe_adapter(cls, value: str) -> str:
        if not value.startswith(("feed.", "local.", "public_api.")):
            raise ValueError("batch adapter is outside the discovery adapter boundary")
        return value

    @field_validator("leads")
    @classmethod
    def _ordered_leads(
        cls, values: tuple[JobLeadV2, ...]
    ) -> tuple[JobLeadV2, ...]:
        if values != tuple(sorted(values, key=lead_batch_sort_key)):
            raise ValueError("batch leads must use deterministic ordering")
        return values

    @model_validator(mode="after")
    def _consistent_batch(self) -> LeadBatchV1:
        if self.batch_id != batch_identifier(
            source_id=self.source_id,
            content_sha256=self.content_sha256,
        ):
            raise ValueError("batch_id does not match source content")
        if self.record_count != len(self.leads):
            raise ValueError("batch record_count does not match leads")
        for lead in self.leads:
            if lead.rank != 0 or lead.score != 0 or lead.match_reasons:
                raise ValueError("batch leads must remain unranked")
            if lead.fetched_at != self.fetched_at:
                raise ValueError("batch leads must share the batch fetch timestamp")
            if self.adapter == "feed.rss_atom":
                has_receipt = any(
                    item.source == self.source_name
                    and item.source_type in {"rss", "atom"}
                    and item.adapter in {"feed.rss", "feed.atom"}
                    and item.source_feed == self.source_url
                    and item.fetched_at == self.fetched_at
                    for item in lead.provenance
                )
                if (
                    lead.source != self.source_name
                    or lead.source_feed != self.source_url
                    or not has_receipt
                ):
                    raise ValueError(
                        "feed batches must resolve to their configured feed source"
                    )
            elif not any(
                item.source == self.source_name
                and item.adapter == self.adapter
                and item.source_feed == self.source_url
                and item.fetched_at == self.fetched_at
                for item in lead.provenance
            ):
                raise ValueError("batch leads must resolve to the batch provenance receipt")
        return self


class DiscoveryCacheV1(DiscoveryContractModel):
    model_config = ConfigDict(
        title="CanISendDiscoveryCacheV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/discovery-cache-v1.schema.json",
        },
    )

    protocol: Literal["canisend.discovery-cache/v1"] = DISCOVERY_CACHE_PROTOCOL
    schema_version: Literal["1.0.0"] = DISCOVERY_CACHE_SCHEMA_VERSION
    source_id: SourceIdentifier
    source_url: str = Field(min_length=1, max_length=8_192)
    etag: str = Field(default="", max_length=1_024)
    last_modified: str = Field(default="", max_length=1_024)
    validated_at: DiscoveryTimestamp
    content_sha256: Sha256Digest

    @field_validator("source_url")
    @classmethod
    def _redacted_source_url(cls, value: str) -> str:
        if redact_public_url(value) != value:
            raise ValueError("cache source URL must be redacted")
        return value

    @field_validator("etag", "last_modified")
    @classmethod
    def _safe_validator(cls, value: str) -> str:
        if _CONTROL_RE.search(value):
            raise ValueError("cache validators must not contain control characters")
        return value


RefreshSourceStatus = Literal[
    "refreshed",
    "not_modified",
    "stale_reused",
    "failed",
]


class SourceRefreshResultV1(DiscoveryContractModel):
    source_id: SourceIdentifier
    status: RefreshSourceStatus
    batch_id: BatchIdentifier | None = None
    batch_path: str | None = Field(default=None, max_length=1_024)
    cache_path: str | None = Field(default=None, max_length=1_024)
    record_count: StrictInt = Field(default=0, ge=0, le=100_000)
    attempts: StrictInt = Field(default=0, ge=0, le=5)
    http_status: StrictInt = Field(default=0, ge=0, le=599)
    error_code: DottedIdentifier | None = None

    @field_validator("batch_path", "cache_path")
    @classmethod
    def _relative_artifact_path(cls, value: str | None) -> str | None:
        return _safe_relative_path(value) if value is not None else None

    @model_validator(mode="after")
    def _consistent_result(self) -> SourceRefreshResultV1:
        has_batch = self.batch_id is not None and self.batch_path is not None
        if self.status in {"refreshed", "not_modified"}:
            if not has_batch or self.cache_path is None or self.error_code is not None:
                raise ValueError("successful refresh results require batch/cache and no error")
            if self.attempts < 1:
                raise ValueError("successful refresh results require a transport attempt")
            if self.status == "not_modified" and self.http_status != 304:
                raise ValueError("not-modified refresh results require HTTP 304")
            if self.status == "refreshed" and not (200 <= self.http_status < 300):
                raise ValueError("refreshed results require a successful HTTP status")
        elif self.status == "stale_reused":
            if not has_batch or self.error_code is None or self.attempts < 1:
                raise ValueError("stale refresh results require a batch and error code")
        elif self.status == "failed":
            if (
                self.batch_id is not None
                or self.batch_path is not None
                or self.record_count != 0
            ):
                raise ValueError("failed refresh results must not claim a usable batch")
            if self.error_code is None or self.attempts < 1:
                raise ValueError("failed refresh results require an error and attempt")
        return self


class DiscoveryRefreshReportV1(DiscoveryContractModel):
    model_config = ConfigDict(
        title="CanISendDiscoveryRefreshReportV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/discovery-refresh-report-v1.schema.json",
        },
    )

    protocol: Literal["canisend.discovery-refresh-report/v1"] = (
        DISCOVERY_REFRESH_REPORT_PROTOCOL
    )
    schema_version: Literal["1.0.0"] = DISCOVERY_REFRESH_REPORT_SCHEMA_VERSION
    refresh_id: RefreshIdentifier
    started_at: DiscoveryTimestamp
    completed_at: DiscoveryTimestamp
    config_sha256: Sha256Digest
    status: Literal["complete", "partial", "failed"]
    catalog_promoted: StrictBool
    catalog_id: CatalogIdentifier | None = None
    catalog_path: str | None = Field(default=None, max_length=1_024)
    catalog_error_code: DottedIdentifier | None = None
    source_count: StrictInt = Field(ge=1, le=1_000)
    successful_sources: StrictInt = Field(ge=0, le=1_000)
    stale_sources: StrictInt = Field(ge=0, le=1_000)
    failed_sources: StrictInt = Field(ge=0, le=1_000)
    input_records: StrictInt = Field(ge=0, le=10_000_000)
    retained_records: StrictInt = Field(ge=0, le=10_000_000)
    excluded_records: StrictInt = Field(ge=0, le=10_000_000)
    sources: tuple[SourceRefreshResultV1, ...] = Field(min_length=1, max_length=1_000)

    @field_validator("catalog_path")
    @classmethod
    def _relative_catalog_path(cls, value: str | None) -> str | None:
        return _safe_relative_path(value) if value is not None else None

    @field_validator("sources")
    @classmethod
    def _ordered_unique_sources(
        cls, values: tuple[SourceRefreshResultV1, ...]
    ) -> tuple[SourceRefreshResultV1, ...]:
        ids = tuple(item.source_id for item in values)
        if len(ids) != len(set(ids)) or ids != tuple(sorted(ids)):
            raise ValueError("refresh source results must be sorted and unique")
        return values

    @model_validator(mode="after")
    def _consistent_report(self) -> DiscoveryRefreshReportV1:
        if self.started_at > self.completed_at:
            raise ValueError("refresh completion must not precede its start")
        successful = sum(
            item.status in {"refreshed", "not_modified"} for item in self.sources
        )
        stale = sum(item.status == "stale_reused" for item in self.sources)
        failed = sum(item.status == "failed" for item in self.sources)
        if (
            self.source_count != len(self.sources)
            or self.successful_sources != successful
            or self.stale_sources != stale
            or self.failed_sources != failed
        ):
            raise ValueError("refresh source counts do not match source results")
        expected_status = (
            "failed"
            if not self.catalog_promoted
            else "partial"
            if stale or failed
            else "complete"
        )
        if self.status != expected_status:
            raise ValueError("refresh status does not match promotion/source outcomes")
        if self.catalog_promoted:
            if not self.catalog_id or not self.catalog_path or self.catalog_error_code:
                raise ValueError("promoted refresh reports require catalog identity/path")
        elif self.catalog_id or self.catalog_path or self.catalog_error_code is None:
            raise ValueError("failed catalog promotion requires a body-free error code")
        if self.refresh_id != refresh_identifier(
            started_at=self.started_at,
            completed_at=self.completed_at,
            config_sha256=self.config_sha256,
            sources=self.sources,
            catalog_promoted=self.catalog_promoted,
            catalog_id=self.catalog_id,
            catalog_error_code=self.catalog_error_code,
        ):
            raise ValueError("refresh_id does not match deterministic report content")
        return self


def batch_identifier(*, source_id: str, content_sha256: str) -> str:
    digest = sha256(f"{source_id}\n{content_sha256}".encode("utf-8")).hexdigest()
    return f"batch_{digest[:32]}"


def config_sha256(config: DiscoverySourcesV1) -> str:
    serialized = json.dumps(
        config.model_dump(mode="json"),
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )
    return sha256(serialized.encode("utf-8")).hexdigest()


def refresh_identifier(
    *,
    started_at: datetime,
    completed_at: datetime,
    config_sha256: str,
    sources: tuple[SourceRefreshResultV1, ...],
    catalog_promoted: bool,
    catalog_id: str | None,
    catalog_error_code: str | None,
) -> str:
    payload = {
        "started_at": started_at.isoformat(),
        "completed_at": completed_at.isoformat(),
        "config_sha256": config_sha256,
        "sources": [item.model_dump(mode="json") for item in sources],
        "catalog_promoted": catalog_promoted,
        "catalog_id": catalog_id,
        "catalog_error_code": catalog_error_code,
    }
    serialized = json.dumps(
        payload,
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )
    return f"refresh_{sha256(serialized.encode('utf-8')).hexdigest()[:32]}"


def lead_batch_sort_key(lead: JobLeadV2) -> tuple[str, str]:
    return (
        lead.lead_id,
        json.dumps(
            lead.model_dump(mode="json"),
            allow_nan=False,
            ensure_ascii=False,
            separators=(",", ":"),
            sort_keys=True,
        ),
    )


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


def _bounded_number(value: object, *, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{label} must be a number")
    return float(value)
