from __future__ import annotations

from collections import defaultdict
from datetime import UTC, datetime
import json
from pathlib import Path
import re
from typing import Iterable, Mapping, Sequence

from pydantic import ValidationError

from canisend.discovery.catalog_models import (
    LEAD_CATALOG_PROTOCOL,
    ExcludedLeadV1,
    LeadCatalogStatsV1,
    LeadCatalogV1,
    RankingPolicyV1,
    catalog_identifier,
    catalog_lead_sort_key,
    normalized_ranking_policy,
)
from canisend.discovery.identity import LeadNormalizationError, normalize_job_lead
from canisend.discovery.models import JobLeadV2, LeadMatchReasonV1, LeadProvenanceV1
from canisend.discovery.store import DiscoveryStoreError, atomic_write_json


_SPACE_RE = re.compile(r"\s+")
_IDENTITY_PRIORITY = {
    "source_record_id": 0,
    "canonical_url": 1,
    "fingerprint": 2,
}
_MATCH_FIELD_SCORES = {
    "title": 100,
    "institution": 70,
    "location": 40,
    "description": 25,
    "source": 15,
    "deadline": 10,
}


class DiscoveryCatalogError(ValueError):
    pass


class DiscoveryInputError(DiscoveryCatalogError):
    pass


class DiscoveryWriteError(DiscoveryCatalogError):
    pass


def build_catalog_from_files(
    paths: Sequence[Path],
    *,
    policy: RankingPolicyV1 | None = None,
    observed_at: datetime | str | None = None,
) -> LeadCatalogV1:
    if not paths:
        raise DiscoveryInputError("Provide at least one --input lead file.")
    observation = _aware_utc_datetime(observed_at)
    leads: list[JobLeadV2] = []
    for path in paths:
        leads.extend(load_lead_document(path, observed_at=observation))
    return merge_lead_catalog(
        leads,
        policy=policy,
        input_record_count=len(leads),
        generated_at=observation,
    )


def load_lead_document(
    path: Path,
    *,
    observed_at: datetime | str | None = None,
) -> list[JobLeadV2]:
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except OSError as exc:
        raise DiscoveryInputError("Discovery input could not be read.") from exc
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise DiscoveryInputError("Discovery input must contain valid UTF-8 JSON.") from exc

    if isinstance(document, list):
        raw_leads = document
    elif isinstance(document, dict) and document.get("protocol") == LEAD_CATALOG_PROTOCOL:
        try:
            catalog = LeadCatalogV1.model_validate(document)
            return [
                *catalog.leads,
                *(item.lead for item in catalog.excluded),
            ]
        except ValidationError as exc:
            raise DiscoveryInputError("Discovery catalog input is invalid.") from exc
    else:
        raise DiscoveryInputError(
            "Discovery input must be a lead JSON list or a versioned CanISend catalog."
        )

    observation = _aware_utc_datetime(observed_at)
    normalized: list[JobLeadV2] = []
    for position, raw_lead in enumerate(raw_leads):
        if not isinstance(raw_lead, Mapping):
            raise DiscoveryInputError(
                f"Discovery input record {position} must be an object."
            )
        try:
            normalized.append(normalize_job_lead(raw_lead, fetched_at=observation))
        except (LeadNormalizationError, ValidationError, ValueError) as exc:
            raise DiscoveryInputError(
                f"Discovery input record {position} is not a valid lead."
            ) from exc
    return normalized


def merge_lead_catalog(
    leads: Iterable[JobLeadV2],
    *,
    policy: RankingPolicyV1 | None = None,
    input_record_count: int | None = None,
    generated_at: datetime | str | None = None,
) -> LeadCatalogV1:
    ranking_policy = policy or normalized_ranking_policy()
    normalized = sorted(
        (JobLeadV2.model_validate(lead.model_dump(mode="json")) for lead in leads),
        key=_lead_content_key,
    )
    count = len(normalized) if input_record_count is None else input_record_count
    if count < len(normalized):
        raise DiscoveryInputError("Input record count cannot be smaller than supplied leads.")

    groups = _deduplicate_groups(normalized)
    merged = tuple(
        _merge_group(group, policy=ranking_policy)
        for group in groups
    )
    retained, excluded = _rank_and_filter(merged, policy=ranking_policy)
    all_records = (*retained, *(item.lead for item in excluded))
    if all_records:
        catalog_time = max(lead.last_seen_at for lead in all_records)
    else:
        catalog_time = _aware_utc_datetime(generated_at)
    source_count = len(
        {
            (
                item.source.casefold(),
                item.source_type,
                item.adapter,
                item.source_feed,
            )
            for lead in all_records
            for item in lead.provenance
        }
    )
    stats = LeadCatalogStatsV1(
        input_records=count,
        unique_records=len(all_records),
        merged_records=count - len(all_records),
        retained_records=len(retained),
        excluded_records=len(excluded),
        source_count=source_count,
    )
    catalog_id = catalog_identifier(
        policy=ranking_policy,
        leads=retained,
        excluded=excluded,
    )
    return LeadCatalogV1(
        catalog_id=catalog_id,
        generated_at=catalog_time,
        policy=ranking_policy,
        stats=stats,
        leads=retained,
        excluded=excluded,
    )


def write_lead_catalog(path: Path, catalog: LeadCatalogV1) -> Path:
    try:
        return atomic_write_json(path, catalog.model_dump(mode="json"))
    except DiscoveryStoreError as exc:
        raise DiscoveryWriteError("Discovery catalog could not be written atomically.") from exc


def _deduplicate_groups(leads: Sequence[JobLeadV2]) -> tuple[tuple[JobLeadV2, ...], ...]:
    if not leads:
        return ()
    parents = list(range(len(leads)))

    def find(position: int) -> int:
        while parents[position] != position:
            parents[position] = parents[parents[position]]
            position = parents[position]
        return position

    def union(left: int, right: int) -> None:
        left_root = find(left)
        right_root = find(right)
        if left_root == right_root:
            return
        lower, higher = sorted((left_root, right_root))
        parents[higher] = lower

    owners: dict[tuple[str, ...], int] = {}
    for position, lead in enumerate(leads):
        for key in _strong_identity_keys(lead):
            owner = owners.setdefault(key, position)
            union(owner, position)

    fingerprint_positions: dict[str, list[int]] = defaultdict(list)
    for position, lead in enumerate(leads):
        fingerprint = _safe_fingerprint(lead)
        if fingerprint:
            fingerprint_positions[fingerprint].append(position)

    for fingerprint in sorted(fingerprint_positions):
        positions = fingerprint_positions[fingerprint]
        roots = sorted({find(position) for position in positions})
        strong_roots = [
            root
            for root in roots
            if any(
                _has_native_or_url_identity(leads[position])
                for position in positions
                if find(position) == root
            )
        ]
        if len(strong_roots) <= 1:
            anchor = strong_roots[0] if strong_roots else roots[0]
            for root in roots:
                union(anchor, root)

    grouped: dict[int, list[JobLeadV2]] = defaultdict(list)
    for position, lead in enumerate(leads):
        grouped[find(position)].append(lead)
    groups = [tuple(sorted(group, key=_lead_content_key)) for group in grouped.values()]
    groups.sort(key=lambda group: tuple(lead.lead_id for lead in group))
    return tuple(groups)


def _strong_identity_keys(lead: JobLeadV2) -> set[tuple[str, ...]]:
    keys = {("lead-id", lead.lead_id)}
    keys.update(("lead-id", alias) for alias in lead.alternate_lead_ids)
    for item in lead.provenance:
        if item.source_record_id:
            keys.add(
                (
                    "source-record",
                    _normalized_text(item.source),
                    item.source_record_id,
                )
            )
        if item.source_url:
            keys.add(("canonical-url", item.source_url))
    if lead.source_record_id:
        keys.add(
            (
                "source-record",
                _normalized_text(lead.source),
                lead.source_record_id,
            )
        )
    if lead.canonical_url:
        keys.add(("canonical-url", lead.canonical_url))
    return keys


def _safe_fingerprint(lead: JobLeadV2) -> str:
    title = _normalized_text(lead.title)
    institution = _normalized_text(lead.institution)
    deadline = _normalized_text(lead.deadline)
    if not title or not (institution or deadline):
        return ""
    return "\n".join((title, institution, deadline))


def _has_native_or_url_identity(lead: JobLeadV2) -> bool:
    return bool(
        lead.source_record_id
        or lead.canonical_url
        or any(item.source_record_id or item.source_url for item in lead.provenance)
    )


def _merge_group(
    group: tuple[JobLeadV2, ...],
    *,
    policy: RankingPolicyV1,
) -> JobLeadV2:
    survivor = min(group, key=_survivor_key)
    preferred = sorted(group, key=lambda lead: _record_preference_key(lead, policy))
    provenance = _merged_provenance(group)
    newest = max(item.fetched_at for item in provenance)
    all_ids = {
        identity
        for lead in group
        for identity in (lead.lead_id, *lead.alternate_lead_ids)
    }
    canonical_url = (
        survivor.canonical_url
        if survivor.identity_method == "canonical_url"
        else _first_value(preferred, "canonical_url")
    )
    return JobLeadV2(
        lead_id=survivor.lead_id,
        identity_method=survivor.identity_method,
        title=_first_value(preferred, "title"),
        source_url=canonical_url,
        description=_first_value(preferred, "description"),
        published_at=_first_value(preferred, "published_at"),
        source=survivor.source,
        source_feed=survivor.source_feed,
        source_record_id=survivor.source_record_id,
        canonical_url=canonical_url,
        institution=_first_value(preferred, "institution"),
        location=_first_value(preferred, "location"),
        deadline=_first_value(preferred, "deadline"),
        fetched_at=newest,
        first_seen_at=min(lead.first_seen_at for lead in group),
        last_seen_at=newest,
        provenance=provenance,
        alternate_lead_ids=tuple(sorted(all_ids - {survivor.lead_id})),
        match_reasons=(),
        score=0,
        rank=0,
    )


def _survivor_key(lead: JobLeadV2) -> tuple[object, ...]:
    history_size = len(lead.alternate_lead_ids) + len(lead.provenance)
    return (
        -history_size,
        _IDENTITY_PRIORITY[lead.identity_method],
        lead.lead_id,
        -int(lead.last_seen_at.timestamp() * 1_000_000),
        _lead_content_key(lead),
    )


def _record_preference_key(
    lead: JobLeadV2,
    policy: RankingPolicyV1,
) -> tuple[object, ...]:
    source_names = {_normalized_text(lead.source)} | {
        _normalized_text(item.source) for item in lead.provenance
    }
    source_rank = min(
        (
            position
            for position, source in enumerate(policy.source_preference)
            if source in source_names
        ),
        default=len(policy.source_preference),
    )
    completeness = sum(
        bool(getattr(lead, field))
        for field in (
            "title",
            "canonical_url",
            "description",
            "published_at",
            "institution",
            "location",
            "deadline",
        )
    )
    return (
        source_rank,
        -completeness,
        -int(lead.last_seen_at.timestamp() * 1_000_000),
        _normalized_text(lead.source),
        lead.lead_id,
        _lead_content_key(lead),
    )


def _first_value(leads: Sequence[JobLeadV2], field: str) -> str:
    return next(
        (str(getattr(lead, field)) for lead in leads if str(getattr(lead, field))),
        "",
    )


def _merged_provenance(
    group: Sequence[JobLeadV2],
) -> tuple[LeadProvenanceV1, ...]:
    latest: dict[tuple[str, ...], LeadProvenanceV1] = {}
    for lead in group:
        for item in lead.provenance:
            key = (
                _normalized_text(item.source),
                item.source_type,
                item.adapter,
                item.source_record_id,
                item.source_url,
                item.source_feed,
            )
            current = latest.get(key)
            if current is None or item.fetched_at > current.fetched_at:
                latest[key] = item
    return tuple(
        sorted(
            latest.values(),
            key=lambda item: (
                _normalized_text(item.source),
                item.source_type,
                item.adapter,
                item.source_record_id,
                item.source_url,
                item.source_feed,
                item.fetched_at.isoformat(),
            ),
        )
    )


def _rank_and_filter(
    leads: Sequence[JobLeadV2],
    *,
    policy: RankingPolicyV1,
) -> tuple[tuple[JobLeadV2, ...], tuple[ExcludedLeadV1, ...]]:
    retained_unranked: list[JobLeadV2] = []
    excluded: list[ExcludedLeadV1] = []
    for lead in leads:
        fields = _search_fields(lead)
        exclusion_reasons = _exclude_reasons(fields, policy)
        include_reasons = _include_reasons(fields, policy)
        if exclusion_reasons or (policy.include_keywords and not include_reasons):
            reasons = exclusion_reasons or tuple(
                LeadMatchReasonV1(
                    code="filter.include_missing",
                    field="record",
                    term=term,
                    score_delta=0,
                )
                for term in policy.include_keywords
            )
            excluded.append(
                ExcludedLeadV1(
                    lead=_reset_ranking(lead),
                    reasons=_sorted_reasons(reasons),
                )
            )
            continue

        reasons = list(include_reasons)
        if not policy.include_keywords:
            reasons.append(
                LeadMatchReasonV1(
                    code="filter.default_include",
                    field="record",
                    score_delta=0,
                )
            )
        reasons.extend(_source_preference_reasons(lead, policy))
        reasons.extend(_metadata_reasons(lead))
        ordered_reasons = _sorted_reasons(reasons)
        score = sum(reason.score_delta for reason in ordered_reasons)
        retained_unranked.append(
            JobLeadV2.model_validate(
                {
                    **_reset_ranking(lead).model_dump(mode="json"),
                    "match_reasons": [
                        reason.model_dump(mode="json") for reason in ordered_reasons
                    ],
                    "score": score,
                }
            )
        )

    ordered = sorted(retained_unranked, key=catalog_lead_sort_key)
    retained = tuple(
        JobLeadV2.model_validate(
            {**lead.model_dump(mode="json"), "rank": position}
        )
        for position, lead in enumerate(ordered, start=1)
    )
    return retained, tuple(sorted(excluded, key=lambda item: item.lead.lead_id))


def _search_fields(lead: JobLeadV2) -> dict[str, str]:
    return {
        "title": _normalized_text(lead.title),
        "description": _normalized_text(lead.description),
        "institution": _normalized_text(lead.institution),
        "location": _normalized_text(lead.location),
        "deadline": _normalized_text(lead.deadline),
        "source": _normalized_text(
            " ".join(
                sorted({lead.source, *(item.source for item in lead.provenance)})
            )
        ),
    }


def _exclude_reasons(
    fields: Mapping[str, str],
    policy: RankingPolicyV1,
) -> tuple[LeadMatchReasonV1, ...]:
    return _sorted_reasons(
        LeadMatchReasonV1(
            code="filter.exclude_keyword",
            field=field,  # type: ignore[arg-type]
            term=term,
            score_delta=0,
        )
        for term in policy.exclude_keywords
        for field, value in fields.items()
        if term in value
    )


def _include_reasons(
    fields: Mapping[str, str],
    policy: RankingPolicyV1,
) -> tuple[LeadMatchReasonV1, ...]:
    return _sorted_reasons(
        LeadMatchReasonV1(
            code=f"match.{field}_keyword",
            field=field,  # type: ignore[arg-type]
            term=term,
            score_delta=_MATCH_FIELD_SCORES[field],
        )
        for term in policy.include_keywords
        for field, value in fields.items()
        if term in value
    )


def _source_preference_reasons(
    lead: JobLeadV2,
    policy: RankingPolicyV1,
) -> tuple[LeadMatchReasonV1, ...]:
    sources = {_normalized_text(lead.source)} | {
        _normalized_text(item.source) for item in lead.provenance
    }
    for position, source in enumerate(policy.source_preference):
        if source in sources:
            return (
                LeadMatchReasonV1(
                    code="rank.source_preference",
                    field="source",
                    term=source,
                    score_delta=(len(policy.source_preference) - position) * 25,
                ),
            )
    return ()


def _metadata_reasons(lead: JobLeadV2) -> tuple[LeadMatchReasonV1, ...]:
    reasons: list[LeadMatchReasonV1] = []
    if lead.alternate_lead_ids:
        reasons.append(
            LeadMatchReasonV1(
                code="merge.identity_aliases",
                field="record",
                term=str(len(lead.alternate_lead_ids)),
                score_delta=0,
            )
        )
    for code, field, score in (
        ("rank.canonical_url", "record", 5),
        ("rank.institution_metadata", "institution", 5),
        ("rank.deadline_metadata", "deadline", 5),
        ("rank.location_metadata", "location", 2),
    ):
        value = lead.canonical_url if code == "rank.canonical_url" else getattr(lead, field)
        if value:
            reasons.append(
                LeadMatchReasonV1(
                    code=code,
                    field=field,  # type: ignore[arg-type]
                    score_delta=score,
                )
            )
    source_count = len(
        {
            (
                _normalized_text(item.source),
                item.source_type,
                item.source_record_id,
                item.source_url,
            )
            for item in lead.provenance
        }
    )
    if source_count > 1:
        reasons.append(
            LeadMatchReasonV1(
                code="rank.multi_source",
                field="record",
                term=str(source_count),
                score_delta=min(15, (source_count - 1) * 3),
            )
        )
    return tuple(reasons)


def _reset_ranking(lead: JobLeadV2) -> JobLeadV2:
    return JobLeadV2.model_validate(
        {
            **lead.model_dump(mode="json"),
            "match_reasons": [],
            "score": 0,
            "rank": 0,
        }
    )


def _sorted_reasons(
    reasons: Iterable[LeadMatchReasonV1],
) -> tuple[LeadMatchReasonV1, ...]:
    by_key = {
        (reason.code, reason.field, reason.term, reason.score_delta): reason
        for reason in reasons
    }
    return tuple(by_key[key] for key in sorted(by_key))


def _lead_content_key(lead: JobLeadV2) -> str:
    return json.dumps(
        lead.model_dump(mode="json"),
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )


def _normalized_text(value: str) -> str:
    return _SPACE_RE.sub(" ", value.casefold()).strip()


def _aware_utc_datetime(value: datetime | str | None) -> datetime:
    if value is None:
        return datetime.now(UTC).replace(microsecond=0)
    if isinstance(value, str):
        try:
            value = datetime.fromisoformat(
                value.strip().replace("Z", "+00:00").replace("z", "+00:00")
            )
        except ValueError as exc:
            raise DiscoveryInputError(
                "Catalog observation time must use ISO 8601 date-time syntax."
            ) from exc
    if not isinstance(value, datetime) or value.tzinfo is None or value.utcoffset() is None:
        raise DiscoveryInputError("Catalog observation time must include a timezone.")
    return value.astimezone(UTC)
