from __future__ import annotations

from pathlib import PureWindowsPath
import re
from typing import Literal
from urllib.parse import unquote

from pydantic import ConfigDict, Field, StrictInt, field_validator, model_validator

from canisend.discovery.identity import (
    LeadNormalizationError,
    canonicalize_job_url,
    sanitize_source_record_id,
)
from canisend.discovery.models import (
    DiscoveryContractModel,
    DiscoveryTimestamp,
    JSON_SCHEMA_DIALECT,
    SCHEMA_BASE_ID,
)
from canisend.discovery.refresh_models import SourceIdentifier


DISCOVERY_SEARCH_PROTOCOL = "canisend.discovery-search/v1"
DISCOVERY_SEARCH_SCHEMA_VERSION = "1.0.0"

_CONTROL_RE = re.compile(r"[\x00-\x1f\x7f]")
_EMAIL_RE = re.compile(
    r"(?i)(?<![A-Z0-9._%+-])[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}"
)
_INLINE_SECRET_RE = re.compile(
    r"(?i)(?:access[_-]?token|api[_-]?key|auth|credential|password|secret|"
    r"session|signature|token)\s*[:=]"
)
_CREDENTIAL_TERMS = frozenset(
    {
        "auth",
        "credential",
        "password",
        "secret",
        "session",
        "signature",
        "token",
    }
)


class DiscoverySearchResultV1(DiscoveryContractModel):
    """One host-neutral public job search result."""

    title: str = Field(default="", max_length=2_048)
    source_url: str = Field(default="", max_length=8_192)
    snippet: str = Field(default="", max_length=100_000)
    published_at: str = Field(default="", max_length=256)
    source_record_id: str = Field(default="", max_length=1_024)
    institution: str = Field(default="", max_length=1_024)
    location: str = Field(default="", max_length=1_024)
    deadline: str = Field(default="", max_length=256)

    @field_validator("source_url")
    @classmethod
    def _public_job_url(cls, value: str) -> str:
        if not value:
            return ""
        decoded = unquote(value)
        if _EMAIL_RE.search(decoded) or _INLINE_SECRET_RE.search(decoded):
            raise ValueError("search result URL contains private locator data")
        try:
            canonicalize_job_url(value)
        except LeadNormalizationError as exc:
            raise ValueError("search result URL must be a public HTTP(S) URL") from exc
        return value

    @field_validator("source_record_id")
    @classmethod
    def _published_record_id(cls, value: str) -> str:
        if value and sanitize_source_record_id(value) != value:
            raise ValueError(
                "search source_record_id must be a credential-free published job ID"
            )
        return value

    @model_validator(mode="after")
    def _has_identity_signal(self) -> DiscoverySearchResultV1:
        if not self.title and not self.source_url:
            raise ValueError("search result requires a title or source URL")
        return self


class DiscoverySearchV1(DiscoveryContractModel):
    """Normalized handoff contract shared by Codex, Claude, and other hosts."""

    model_config = ConfigDict(
        title="CanISendDiscoverySearchV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/discovery-search-v1.schema.json",
        },
    )

    protocol: Literal["canisend.discovery-search/v1"] = DISCOVERY_SEARCH_PROTOCOL
    schema_version: Literal["1.0.0"] = DISCOVERY_SEARCH_SCHEMA_VERSION
    source_id: SourceIdentifier
    source_name: str = Field(min_length=1, max_length=256)
    observed_at: DiscoveryTimestamp
    result_count: StrictInt = Field(ge=1, le=100_000)
    results: tuple[DiscoverySearchResultV1, ...] = Field(
        min_length=1,
        max_length=100_000,
    )

    @field_validator("source_id")
    @classmethod
    def _private_safe_source_id(cls, value: str) -> str:
        parts = {part for part in re.split(r"[^a-z0-9]+", value) if part}
        if parts & _CREDENTIAL_TERMS:
            raise ValueError("search source_id contains credential-like terms")
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
            raise ValueError("search source_name must be a private-safe label")
        return value

    @model_validator(mode="after")
    def _consistent_result_count(self) -> DiscoverySearchV1:
        if self.result_count != len(self.results):
            raise ValueError("search result_count must match results")
        return self
