from __future__ import annotations

import re
from typing import Annotated, Literal

from pydantic import (
    AwareDatetime,
    BaseModel,
    BeforeValidator,
    ConfigDict,
    Field,
    field_validator,
    model_validator,
)


JOB_LEAD_SCHEMA_VERSION = "2.0.0"
JSON_SCHEMA_DIALECT = "https://json-schema.org/draft/2020-12/schema"
SCHEMA_BASE_ID = "https://github.com/jxpeng98/CanISend/schemas"

LeadIdentityMethod = Literal["source_record_id", "canonical_url", "fingerprint"]
LeadSourceType = Literal[
    "rss",
    "atom",
    "public_api",
    "csv",
    "json",
    "email_alert",
    "host_agent",
    "legacy",
]
LeadMatchField = Literal[
    "title",
    "description",
    "institution",
    "location",
    "deadline",
    "source",
    "record",
]

_LEAD_ID_RE = re.compile(r"^lead_[0-9a-f]{32}$")
_REASON_CODE_RE = re.compile(r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)+$")
_RFC3339_DATETIME_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:[Zz]|[+-]\d{2}:\d{2})$"
)


def _reject_numeric_timestamp(value: object) -> object:
    if isinstance(value, (bool, int, float)):
        raise ValueError("discovery timestamps must be aware date-time strings")
    if isinstance(value, str):
        normalized = value.strip()
        if _RFC3339_DATETIME_RE.fullmatch(normalized) is None:
            raise ValueError("discovery timestamp strings must use RFC 3339 date-time syntax")
        return normalized
    return value


DiscoveryTimestamp = Annotated[AwareDatetime, BeforeValidator(_reject_numeric_timestamp)]
LeadIdentifier = Annotated[str, Field(pattern=_LEAD_ID_RE.pattern)]
DottedIdentifier = Annotated[str, Field(pattern=_REASON_CODE_RE.pattern)]


class DiscoveryContractModel(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
    )


class LeadMatchReasonV1(DiscoveryContractModel):
    """One machine-readable reason contributing to retention, exclusion, or rank."""

    code: DottedIdentifier
    field: LeadMatchField
    term: str = Field(default="", max_length=256)
    score_delta: int = Field(default=0, ge=-100_000, le=100_000)

    @field_validator("code")
    @classmethod
    def _valid_code(cls, value: str) -> str:
        if _REASON_CODE_RE.fullmatch(value) is None:
            raise ValueError("match reason code must be a lowercase dotted identifier")
        return value


class LeadProvenanceV1(DiscoveryContractModel):
    """Source-neutral origin receipt with no response body or credentials."""

    source: str = Field(min_length=1, max_length=256)
    source_type: LeadSourceType
    adapter: DottedIdentifier = Field(min_length=1, max_length=128)
    source_record_id: str = Field(default="", max_length=1_024)
    source_url: str = Field(default="", max_length=8_192)
    source_feed: str = Field(default="", max_length=8_192)
    fetched_at: DiscoveryTimestamp

    @field_validator("adapter")
    @classmethod
    def _valid_adapter(cls, value: str) -> str:
        if _REASON_CODE_RE.fullmatch(value) is None:
            raise ValueError("adapter must be a lowercase dotted identifier")
        return value

    @field_validator("source_url")
    @classmethod
    def _canonical_source_url(cls, value: str) -> str:
        return _require_canonical_url(value, label="provenance source_url")

    @field_validator("source_feed")
    @classmethod
    def _redacted_source_feed(cls, value: str) -> str:
        return _require_redacted_locator(value)

    @field_validator("source_record_id")
    @classmethod
    def _safe_record_id(cls, value: str) -> str:
        return _require_safe_record_id(value)


class JobLeadV2(DiscoveryContractModel):
    """Additive Lead v2 contract retaining the original six lead fields."""

    model_config = ConfigDict(
        title="CanISendJobLeadV2",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/job-lead-v2.schema.json",
        },
    )

    schema_version: Literal["2.0.0"] = JOB_LEAD_SCHEMA_VERSION
    lead_id: LeadIdentifier
    identity_method: LeadIdentityMethod

    # Original JobLead fields remain required and keep their names.
    title: str = Field(max_length=2_048)
    source_url: str = Field(default="", max_length=8_192)
    description: str = Field(default="", max_length=1_000_000)
    published_at: str = Field(default="", max_length=256)
    source: str = Field(min_length=1, max_length=256)
    source_feed: str = Field(default="", max_length=8_192)

    source_record_id: str = Field(default="", max_length=1_024)
    canonical_url: str = Field(default="", max_length=8_192)
    institution: str = Field(default="", max_length=1_024)
    location: str = Field(default="", max_length=1_024)
    deadline: str = Field(default="", max_length=256)
    fetched_at: DiscoveryTimestamp
    first_seen_at: DiscoveryTimestamp
    last_seen_at: DiscoveryTimestamp
    provenance: tuple[LeadProvenanceV1, ...] = Field(min_length=1, max_length=4_096)
    alternate_lead_ids: tuple[LeadIdentifier, ...] = Field(
        default=(), max_length=4_096, json_schema_extra={"uniqueItems": True}
    )
    match_reasons: tuple[LeadMatchReasonV1, ...] = Field(default=(), max_length=4_096)
    score: int = Field(default=0, ge=-1_000_000, le=1_000_000)

    @field_validator("lead_id")
    @classmethod
    def _valid_lead_id(cls, value: str) -> str:
        if _LEAD_ID_RE.fullmatch(value) is None:
            raise ValueError("lead_id must use the lead_<32 lowercase hex> format")
        return value

    @field_validator("source_url", "canonical_url")
    @classmethod
    def _canonical_urls(cls, value: str) -> str:
        return _require_canonical_url(value, label="lead URL")

    @field_validator("source_feed")
    @classmethod
    def _safe_source_feed(cls, value: str) -> str:
        return _require_redacted_locator(value)

    @field_validator("source_record_id")
    @classmethod
    def _safe_source_record_id(cls, value: str) -> str:
        return _require_safe_record_id(value)

    @field_validator("alternate_lead_ids")
    @classmethod
    def _valid_alternate_ids(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        if any(_LEAD_ID_RE.fullmatch(value) is None for value in values):
            raise ValueError("alternate lead IDs must use the lead_<32 lowercase hex> format")
        if len(values) != len(set(values)):
            raise ValueError("alternate lead IDs must be unique")
        if values != tuple(sorted(values)):
            raise ValueError("alternate lead IDs must be sorted")
        return values

    @field_validator("provenance")
    @classmethod
    def _unique_provenance(
        cls, values: tuple[LeadProvenanceV1, ...]
    ) -> tuple[LeadProvenanceV1, ...]:
        keys = tuple(
            (
                item.source.casefold(),
                item.source_type,
                item.adapter,
                item.source_record_id,
                item.source_url,
                item.source_feed,
                item.fetched_at.isoformat(),
            )
            for item in values
        )
        if len(keys) != len(set(keys)):
            raise ValueError("lead provenance records must be unique")
        return values

    @model_validator(mode="after")
    def _consistent_lead(self) -> JobLeadV2:
        if not self.title and not self.canonical_url:
            raise ValueError("a lead requires a title or canonical URL")
        if self.identity_method == "source_record_id" and not self.source_record_id:
            raise ValueError("source-record identity requires source_record_id")
        if self.identity_method == "canonical_url" and not self.canonical_url:
            raise ValueError("canonical-URL identity requires canonical_url")
        if self.lead_id in self.alternate_lead_ids:
            raise ValueError("lead_id must not also appear in alternate_lead_ids")
        if self.first_seen_at > self.last_seen_at:
            raise ValueError("first_seen_at must not be after last_seen_at")
        if self.fetched_at > self.last_seen_at:
            raise ValueError("fetched_at must not be after last_seen_at")
        newest_provenance = max(item.fetched_at for item in self.provenance)
        if self.fetched_at != newest_provenance or self.last_seen_at != newest_provenance:
            raise ValueError(
                "fetched_at and last_seen_at must equal the newest provenance timestamp"
            )
        if not any(
            item.source == self.source
            and item.source_record_id == self.source_record_id
            and item.source_url == self.canonical_url
            for item in self.provenance
        ):
            raise ValueError("lead source fields must resolve to one provenance record")
        return self


def _require_canonical_url(value: str, *, label: str) -> str:
    if not value:
        return ""
    from canisend.discovery.identity import canonicalize_job_url

    if canonicalize_job_url(value) != value:
        raise ValueError(f"{label} must be canonical and credential-free")
    return value


def _require_redacted_locator(value: str) -> str:
    if not value:
        return ""
    from canisend.discovery.identity import redact_feed_url

    if redact_feed_url(value) != value:
        raise ValueError("source_feed must be a redacted URL or non-path source label")
    return value


def _require_safe_record_id(value: str) -> str:
    if not value:
        return ""
    from canisend.discovery.identity import sanitize_source_record_id

    if sanitize_source_record_id(value) != value:
        raise ValueError("source_record_id must not contain credentials or private locators")
    return value
