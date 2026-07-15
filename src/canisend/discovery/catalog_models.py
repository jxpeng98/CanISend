from __future__ import annotations

from hashlib import sha256
import json
import re
from typing import Annotated, Literal

from pydantic import ConfigDict, Field, StrictInt, field_validator, model_validator

from canisend.discovery.models import (
    DiscoveryContractModel,
    DiscoveryTimestamp,
    JobLeadV2,
    LeadMatchReasonV1,
    JSON_SCHEMA_DIALECT,
    SCHEMA_BASE_ID,
)


LEAD_CATALOG_PROTOCOL = "canisend.discovery-catalog/v1"
LEAD_CATALOG_SCHEMA_VERSION = "1.0.0"
RANKING_POLICY_VERSION = "1.0.0"

_CATALOG_ID_RE = re.compile(r"^catalog_[0-9a-f]{32}$")
_NORMALIZED_SPACE_RE = re.compile(r"\s+")
_CONTROL_RE = re.compile(r"[\x00-\x1f\x7f]")

CatalogIdentifier = Annotated[str, Field(pattern=_CATALOG_ID_RE.pattern)]
PolicyTerm = Annotated[str, Field(min_length=1, max_length=256)]


class RankingPolicyV1(DiscoveryContractModel):
    version: Literal["1.0.0"] = RANKING_POLICY_VERSION
    include_keywords: tuple[PolicyTerm, ...] = Field(
        default=(), max_length=256, json_schema_extra={"uniqueItems": True}
    )
    exclude_keywords: tuple[PolicyTerm, ...] = Field(
        default=(), max_length=256, json_schema_extra={"uniqueItems": True}
    )
    source_preference: tuple[PolicyTerm, ...] = Field(
        default=(), max_length=256, json_schema_extra={"uniqueItems": True}
    )

    @field_validator("include_keywords", "exclude_keywords")
    @classmethod
    def _sorted_normalized_terms(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_normalized_terms(values)
        if values != tuple(sorted(values)):
            raise ValueError("include and exclude keywords must be sorted")
        return values

    @field_validator("source_preference")
    @classmethod
    def _ordered_normalized_sources(cls, values: tuple[str, ...]) -> tuple[str, ...]:
        _require_normalized_terms(values)
        return values


class LeadCatalogStatsV1(DiscoveryContractModel):
    input_records: StrictInt = Field(ge=0, le=10_000_000)
    unique_records: StrictInt = Field(ge=0, le=10_000_000)
    merged_records: StrictInt = Field(ge=0, le=10_000_000)
    retained_records: StrictInt = Field(ge=0, le=10_000_000)
    excluded_records: StrictInt = Field(ge=0, le=10_000_000)
    source_count: StrictInt = Field(ge=0, le=1_000_000)


class ExcludedLeadV1(DiscoveryContractModel):
    lead: JobLeadV2
    reasons: tuple[LeadMatchReasonV1, ...] = Field(min_length=1, max_length=4_096)

    @field_validator("reasons")
    @classmethod
    def _ordered_unique_reasons(
        cls, values: tuple[LeadMatchReasonV1, ...]
    ) -> tuple[LeadMatchReasonV1, ...]:
        keys = tuple(
            (item.code, item.field, item.term, item.score_delta) for item in values
        )
        if len(keys) != len(set(keys)) or keys != tuple(sorted(keys)):
            raise ValueError("exclusion reasons must be sorted and unique")
        if any(not reason.code.startswith("filter.") for reason in values):
            raise ValueError("exclusion reasons must use filter.* codes")
        return values

    @model_validator(mode="after")
    def _unranked_excluded_lead(self) -> ExcludedLeadV1:
        if self.lead.rank != 0 or self.lead.score != 0 or self.lead.match_reasons:
            raise ValueError("excluded leads must remain unranked with no rank reasons")
        return self


class LeadCatalogV1(DiscoveryContractModel):
    model_config = ConfigDict(
        title="CanISendLeadCatalogV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/lead-catalog-v1.schema.json",
        },
    )

    protocol: Literal["canisend.discovery-catalog/v1"] = LEAD_CATALOG_PROTOCOL
    schema_version: Literal["1.0.0"] = LEAD_CATALOG_SCHEMA_VERSION
    catalog_id: CatalogIdentifier
    generated_at: DiscoveryTimestamp
    policy: RankingPolicyV1
    stats: LeadCatalogStatsV1
    leads: tuple[JobLeadV2, ...] = Field(default=(), max_length=1_000_000)
    excluded: tuple[ExcludedLeadV1, ...] = Field(default=(), max_length=1_000_000)

    @model_validator(mode="after")
    def _consistent_catalog(self) -> LeadCatalogV1:
        expected_ranks = tuple(range(1, len(self.leads) + 1))
        if tuple(lead.rank for lead in self.leads) != expected_ranks:
            raise ValueError("retained lead ranks must be contiguous and one-based")
        if self.leads != tuple(sorted(self.leads, key=catalog_lead_sort_key)):
            raise ValueError("retained leads must use deterministic rank ordering")
        if self.excluded != tuple(
            sorted(self.excluded, key=lambda item: item.lead.lead_id)
        ):
            raise ValueError("excluded leads must be sorted by lead_id")

        all_records = (*self.leads, *(item.lead for item in self.excluded))
        identity_owners: dict[str, str] = {}
        for lead in all_records:
            for identity in (lead.lead_id, *lead.alternate_lead_ids):
                owner = identity_owners.setdefault(identity, lead.lead_id)
                if owner != lead.lead_id:
                    raise ValueError("catalog lead identities must belong to one merge group")

        unique_records = len(all_records)
        source_keys = {
            (
                item.source.casefold(),
                item.source_type,
                item.adapter,
                item.source_feed,
            )
            for lead in all_records
            for item in lead.provenance
        }
        expected_stats = LeadCatalogStatsV1(
            input_records=self.stats.input_records,
            unique_records=unique_records,
            merged_records=self.stats.input_records - unique_records,
            retained_records=len(self.leads),
            excluded_records=len(self.excluded),
            source_count=len(source_keys),
        )
        if self.stats.input_records < unique_records or self.stats != expected_stats:
            raise ValueError("catalog statistics do not match the catalog records")

        if all_records:
            newest = max(lead.last_seen_at for lead in all_records)
            if self.generated_at != newest:
                raise ValueError("catalog generated_at must equal its newest lead observation")
        if self.catalog_id != catalog_identifier(
            policy=self.policy,
            leads=self.leads,
            excluded=self.excluded,
        ):
            raise ValueError("catalog_id does not match deterministic catalog content")
        return self


def catalog_lead_sort_key(lead: JobLeadV2) -> tuple[object, ...]:
    return (
        -lead.score,
        _normalized_term(lead.title),
        _normalized_term(lead.institution),
        _normalized_term(lead.deadline),
        lead.lead_id,
    )


def catalog_identifier(
    *,
    policy: RankingPolicyV1,
    leads: tuple[JobLeadV2, ...],
    excluded: tuple[ExcludedLeadV1, ...],
) -> str:
    payload = {
        "policy": policy.model_dump(mode="json"),
        "leads": [lead.model_dump(mode="json") for lead in leads],
        "excluded": [item.model_dump(mode="json") for item in excluded],
    }
    serialized = json.dumps(
        payload,
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )
    return f"catalog_{sha256(serialized.encode('utf-8')).hexdigest()[:32]}"


def normalized_ranking_policy(
    *,
    include_keywords: tuple[str, ...] | list[str] = (),
    exclude_keywords: tuple[str, ...] | list[str] = (),
    source_preference: tuple[str, ...] | list[str] = (),
) -> RankingPolicyV1:
    return RankingPolicyV1(
        include_keywords=tuple(sorted({_normalized_term(value) for value in include_keywords if value.strip()})),
        exclude_keywords=tuple(sorted({_normalized_term(value) for value in exclude_keywords if value.strip()})),
        source_preference=_ordered_unique(
            _normalized_term(value) for value in source_preference if value.strip()
        ),
    )


def _require_normalized_terms(values: tuple[str, ...]) -> None:
    if len(values) != len(set(values)):
        raise ValueError("ranking policy terms must be unique")
    for value in values:
        if _CONTROL_RE.search(value) or value != _normalized_term(value):
            raise ValueError("ranking policy terms must be normalized lowercase text")


def _normalized_term(value: str) -> str:
    return _NORMALIZED_SPACE_RE.sub(" ", value.casefold()).strip()


def _ordered_unique(values) -> tuple[str, ...]:
    seen: set[str] = set()
    ordered: list[str] = []
    for value in values:
        if value not in seen:
            seen.add(value)
            ordered.append(value)
    return tuple(ordered)
