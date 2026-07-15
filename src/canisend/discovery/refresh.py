from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from datetime import UTC, datetime
from hashlib import sha256
import json
from pathlib import Path
from typing import Any

from pydantic import ValidationError
import yaml

from canisend.discovery.catalog import (
    DiscoveryWriteError,
    merge_lead_catalog,
    write_lead_catalog,
)
from canisend.discovery.catalog_models import LeadCatalogV1
from canisend.discovery.identity import LeadNormalizationError, normalize_job_lead
from canisend.discovery.refresh_models import (
    DiscoveryCacheV1,
    DiscoveryRefreshReportV1,
    DiscoverySourceV1,
    DiscoverySourcesV1,
    LeadBatchV1,
    SourceRefreshResultV1,
    batch_identifier,
    config_sha256,
    lead_batch_sort_key,
    refresh_identifier,
)
from canisend.discovery.store import DiscoveryStoreError, atomic_write_json
from canisend.discovery.transport import (
    PublicTransport,
    PublicTransportError,
    TransportPolicy,
    TransportResponse,
    redact_public_url,
)
from canisend.rss import (
    ALLOWED_FEED_CONTENT_TYPES,
    JobFeedError,
    decode_job_feed_bytes,
    parse_job_feed,
)


class DiscoveryRefreshError(ValueError):
    pass


class DiscoveryRefreshInputError(DiscoveryRefreshError):
    pass


class DiscoveryRefreshWriteError(DiscoveryRefreshError):
    pass


@dataclass(frozen=True)
class DiscoveryRefreshExecution:
    report: DiscoveryRefreshReportV1
    report_path: Path
    catalog: LeadCatalogV1 | None
    catalog_path: Path | None


@dataclass
class _SourceState:
    source: DiscoverySourceV1
    batch_path: Path
    cache_path: Path
    previous_batch: LeadBatchV1 | None
    previous_cache: DiscoveryCacheV1 | None
    batch: LeadBatchV1 | None
    cache: DiscoveryCacheV1 | None
    status: str
    attempts: int
    http_status: int
    error_code: str | None = None
    promote_batch: bool = False
    promote_cache: bool = False


@dataclass(frozen=True)
class _SourceFailure(Exception):
    code: str
    attempts: int
    http_status: int = 0


def load_discovery_sources(path: Path) -> DiscoverySourcesV1:
    try:
        target = Path(path)
        if target.stat().st_size > 1_000_000:
            raise DiscoveryRefreshInputError(
                "Discovery source configuration exceeds the supported size limit."
            )
        payload = yaml.safe_load(target.read_text(encoding="utf-8"))
        if not isinstance(payload, dict):
            raise DiscoveryRefreshInputError(
                "Discovery source configuration must contain a versioned mapping."
            )
        return DiscoverySourcesV1.model_validate(payload)
    except DiscoveryRefreshInputError:
        raise
    except OSError as exc:
        raise DiscoveryRefreshInputError(
            "Discovery source configuration could not be read."
        ) from exc
    except (UnicodeError, yaml.YAMLError) as exc:
        raise DiscoveryRefreshInputError(
            "Discovery source configuration must contain valid UTF-8 YAML."
        ) from exc
    except ValidationError as exc:
        raise DiscoveryRefreshInputError(
            "Discovery source configuration does not match its strict contract."
        ) from exc


def load_lead_batch(path: Path) -> LeadBatchV1:
    return _load_json_contract(
        path,
        model=LeadBatchV1,
        label="Discovery lead batch",
    )


def load_discovery_cache(path: Path) -> DiscoveryCacheV1:
    return _load_json_contract(
        path,
        model=DiscoveryCacheV1,
        label="Discovery validator cache",
    )


def write_lead_batch(path: Path, batch: LeadBatchV1) -> Path:
    return _write_contract(path, batch, label="Discovery lead batch")


def write_discovery_cache(path: Path, cache: DiscoveryCacheV1) -> Path:
    return _write_contract(path, cache, label="Discovery validator cache")


def write_refresh_report(path: Path, report: DiscoveryRefreshReportV1) -> Path:
    return _write_contract(path, report, label="Discovery refresh report")


def refresh_discovery_sources(
    workspace: Path,
    sources: DiscoverySourcesV1,
    *,
    lead_root: Path | None = None,
    transport: PublicTransport | None = None,
    clock: Callable[[], datetime] | None = None,
) -> DiscoveryRefreshExecution:
    workspace_root = Path(workspace).expanduser().resolve()
    discovery_root = _safe_discovery_root(
        workspace_root,
        lead_root or workspace_root / "job_leads",
    )
    batches_dir = _safe_artifact_directory(
        workspace_root,
        discovery_root / "batches",
    )
    cache_dir = _safe_artifact_directory(
        workspace_root,
        discovery_root / "cache",
    )
    report_path = discovery_root / "refresh-report.json"
    candidate_catalog_path = discovery_root / "catalog.json"
    active_sources = tuple(
        sorted(
            (source for source in sources.sources if source.enabled),
            key=lambda source: source.source_id,
        )
    )
    now = clock or (lambda: datetime.now(UTC).replace(microsecond=0))
    started_at = _read_clock(now)
    shared_transport = transport or PublicTransport(now=now)

    states: list[_SourceState] = []
    for source in active_sources:
        artifact_stem = _source_artifact_stem(source)
        batch_path = batches_dir / f"{artifact_stem}.json"
        cache_path = cache_dir / f"{artifact_stem}.json"
        previous_batch = _compatible_previous_batch(batch_path, source)
        previous_cache = _compatible_previous_cache(
            cache_path,
            source,
            previous_batch,
        )
        try:
            response = shared_transport.fetch(
                source.url,
                policy=_transport_policy(source),
                etag=previous_cache.etag if previous_cache else "",
                last_modified=(
                    previous_cache.last_modified if previous_cache else ""
                ),
            )
            state = _state_from_response(
                source,
                response=response,
                observed_at=started_at,
                batch_path=batch_path,
                cache_path=cache_path,
                previous_batch=previous_batch,
                previous_cache=previous_cache,
            )
        except PublicTransportError as exc:
            state = _failed_source_state(
                source,
                batch_path=batch_path,
                cache_path=cache_path,
                previous_batch=previous_batch,
                previous_cache=previous_cache,
                failure=_SourceFailure(
                    code=exc.code,
                    attempts=max(1, exc.attempts),
                    http_status=exc.status_code,
                ),
            )
        except _SourceFailure as failure:
            state = _failed_source_state(
                source,
                batch_path=batch_path,
                cache_path=cache_path,
                previous_batch=previous_batch,
                previous_cache=previous_cache,
                failure=failure,
            )
        states.append(state)

    for state in states:
        _promote_source_state(state)

    usable_batches = tuple(
        state.batch
        for state in states
        if state.status in {"refreshed", "not_modified", "stale_reused"}
        and state.batch is not None
    )
    catalog: LeadCatalogV1 | None = None
    catalog_error_code: str | None = None
    if not usable_batches:
        catalog_error_code = "catalog.no_usable_sources"
    else:
        try:
            all_leads = [
                lead
                for batch in usable_batches
                for lead in batch.leads
            ]
            candidate = merge_lead_catalog(
                all_leads,
                policy=sources.policy,
                input_record_count=len(all_leads),
                generated_at=started_at,
            )
            write_lead_catalog(candidate_catalog_path, candidate)
            catalog = candidate
        except (DiscoveryWriteError, DiscoveryStoreError):
            catalog_error_code = "catalog.promotion_failed"
        except (ValidationError, ValueError):
            catalog_error_code = "catalog.invalid_inputs"

    completed_at = max(started_at, _read_clock(now))
    source_results = tuple(
        _source_result(workspace_root, state)
        for state in sorted(states, key=lambda item: item.source.source_id)
    )
    promoted = catalog is not None
    report_catalog_path = (
        _workspace_relative_path(workspace_root, candidate_catalog_path)
        if promoted
        else None
    )
    input_records = sum(
        state.batch.record_count
        for state in states
        if state.batch is not None
        and state.status in {"refreshed", "not_modified", "stale_reused"}
    )
    report_id = refresh_identifier(
        started_at=started_at,
        completed_at=completed_at,
        config_sha256=config_sha256(sources),
        sources=source_results,
        catalog_promoted=promoted,
        catalog_id=catalog.catalog_id if catalog else None,
        catalog_error_code=catalog_error_code,
    )
    report = DiscoveryRefreshReportV1(
        refresh_id=report_id,
        started_at=started_at,
        completed_at=completed_at,
        config_sha256=config_sha256(sources),
        status=(
            "failed"
            if not promoted
            else "partial"
            if any(
                result.status in {"stale_reused", "failed"}
                for result in source_results
            )
            else "complete"
        ),
        catalog_promoted=promoted,
        catalog_id=catalog.catalog_id if catalog else None,
        catalog_path=report_catalog_path,
        catalog_error_code=catalog_error_code,
        source_count=len(source_results),
        successful_sources=sum(
            item.status in {"refreshed", "not_modified"}
            for item in source_results
        ),
        stale_sources=sum(item.status == "stale_reused" for item in source_results),
        failed_sources=sum(item.status == "failed" for item in source_results),
        input_records=input_records,
        retained_records=catalog.stats.retained_records if catalog else 0,
        excluded_records=catalog.stats.excluded_records if catalog else 0,
        sources=source_results,
    )
    write_refresh_report(report_path, report)
    return DiscoveryRefreshExecution(
        report=report,
        report_path=report_path,
        catalog=catalog,
        catalog_path=candidate_catalog_path if catalog else None,
    )


def _state_from_response(
    source: DiscoverySourceV1,
    *,
    response: TransportResponse,
    observed_at: datetime,
    batch_path: Path,
    cache_path: Path,
    previous_batch: LeadBatchV1 | None,
    previous_cache: DiscoveryCacheV1 | None,
) -> _SourceState:
    if response.not_modified:
        if previous_batch is None or previous_cache is None:
            raise _SourceFailure(
                code="cache.not_modified_without_batch",
                attempts=response.attempts,
                http_status=304,
            )
        cache = DiscoveryCacheV1(
            source_id=source.source_id,
            source_url=redact_public_url(source.url),
            etag=response.etag or previous_cache.etag,
            last_modified=response.last_modified or previous_cache.last_modified,
            validated_at=observed_at,
            content_sha256=previous_batch.content_sha256,
        )
        return _SourceState(
            source=source,
            batch_path=batch_path,
            cache_path=cache_path,
            previous_batch=previous_batch,
            previous_cache=previous_cache,
            batch=previous_batch,
            cache=cache,
            status="not_modified",
            attempts=response.attempts,
            http_status=304,
            promote_cache=True,
        )

    try:
        xml_text = decode_job_feed_bytes(response.body, response.content_type)
        raw_leads = parse_job_feed(
            xml_text,
            feed_url=source.url,
            source_name=source.name,
        )
    except JobFeedError as exc:
        raise _SourceFailure(
            code="source.parse_invalid",
            attempts=response.attempts,
            http_status=response.status_code,
        ) from exc
    if len(raw_leads) > source.max_leads:
        raise _SourceFailure(
            code="source.record_limit",
            attempts=response.attempts,
            http_status=response.status_code,
        )
    try:
        leads = tuple(
            sorted(
                (
                    normalize_job_lead(
                        lead,
                        fetched_at=observed_at,
                        source_type=lead.source_type,  # type: ignore[arg-type]
                    )
                    for lead in raw_leads
                ),
                key=lead_batch_sort_key,
            )
        )
        batch = LeadBatchV1(
            batch_id=batch_identifier(
                source_id=source.source_id,
                content_sha256=response.content_sha256,
            ),
            source_id=source.source_id,
            source_name=source.name,
            source_url=redact_public_url(source.url),
            fetched_at=observed_at,
            content_sha256=response.content_sha256,
            record_count=len(leads),
            leads=leads,
        )
        cache = DiscoveryCacheV1(
            source_id=source.source_id,
            source_url=redact_public_url(source.url),
            etag=response.etag,
            last_modified=response.last_modified,
            validated_at=observed_at,
            content_sha256=response.content_sha256,
        )
    except (LeadNormalizationError, ValidationError, ValueError) as exc:
        raise _SourceFailure(
            code="source.batch_invalid",
            attempts=response.attempts,
            http_status=response.status_code,
        ) from exc
    return _SourceState(
        source=source,
        batch_path=batch_path,
        cache_path=cache_path,
        previous_batch=previous_batch,
        previous_cache=previous_cache,
        batch=batch,
        cache=cache,
        status="refreshed",
        attempts=response.attempts,
        http_status=response.status_code,
        promote_batch=True,
        promote_cache=True,
    )


def _failed_source_state(
    source: DiscoverySourceV1,
    *,
    batch_path: Path,
    cache_path: Path,
    previous_batch: LeadBatchV1 | None,
    previous_cache: DiscoveryCacheV1 | None,
    failure: _SourceFailure,
) -> _SourceState:
    return _SourceState(
        source=source,
        batch_path=batch_path,
        cache_path=cache_path,
        previous_batch=previous_batch,
        previous_cache=previous_cache,
        batch=previous_batch,
        cache=previous_cache,
        status="stale_reused" if previous_batch is not None else "failed",
        attempts=max(1, failure.attempts),
        http_status=failure.http_status,
        error_code=failure.code,
    )


def _promote_source_state(state: _SourceState) -> None:
    try:
        if state.promote_cache and state.cache is not None:
            write_discovery_cache(state.cache_path, state.cache)
        if state.promote_batch and state.batch is not None:
            write_lead_batch(state.batch_path, state.batch)
    except DiscoveryRefreshWriteError:
        if state.previous_batch is not None:
            state.batch = state.previous_batch
            state.cache = state.previous_cache
            state.status = "stale_reused"
        else:
            state.batch = None
            state.cache = None
            state.status = "failed"
        state.error_code = "store.source_promotion_failed"
        state.promote_batch = False
        state.promote_cache = False


def _source_result(
    workspace: Path,
    state: _SourceState,
) -> SourceRefreshResultV1:
    usable = state.status in {"refreshed", "not_modified", "stale_reused"}
    cache_exists = _cache_file_matches_batch(state.cache_path, state.batch)
    return SourceRefreshResultV1(
        source_id=state.source.source_id,
        status=state.status,  # type: ignore[arg-type]
        batch_id=state.batch.batch_id if usable and state.batch else None,
        batch_path=(
            _workspace_relative_path(workspace, state.batch_path)
            if usable and state.batch
            else None
        ),
        cache_path=(
            _workspace_relative_path(workspace, state.cache_path)
            if usable and cache_exists
            else None
        ),
        record_count=state.batch.record_count if usable and state.batch else 0,
        attempts=state.attempts,
        http_status=state.http_status,
        error_code=state.error_code,
    )


def _cache_file_matches_batch(
    path: Path,
    batch: LeadBatchV1 | None,
) -> bool:
    if batch is None:
        return False
    try:
        cache = load_discovery_cache(path)
    except DiscoveryRefreshInputError:
        return False
    return (
        cache.source_id == batch.source_id
        and cache.content_sha256 == batch.content_sha256
    )


def _compatible_previous_batch(
    path: Path,
    source: DiscoverySourceV1,
) -> LeadBatchV1 | None:
    try:
        batch = load_lead_batch(path)
    except DiscoveryRefreshInputError:
        return None
    if (
        batch.source_id != source.source_id
        or batch.source_name != source.name
        or batch.source_url != redact_public_url(source.url)
    ):
        return None
    return batch


def _compatible_previous_cache(
    path: Path,
    source: DiscoverySourceV1,
    batch: LeadBatchV1 | None,
) -> DiscoveryCacheV1 | None:
    if batch is None:
        return None
    try:
        cache = load_discovery_cache(path)
    except DiscoveryRefreshInputError:
        return None
    if (
        cache.source_id != source.source_id
        or cache.source_url != redact_public_url(source.url)
        or cache.content_sha256 != batch.content_sha256
    ):
        return None
    return cache


def _transport_policy(source: DiscoverySourceV1) -> TransportPolicy:
    return TransportPolicy(
        allowed_media_types=tuple(sorted(ALLOWED_FEED_CONTENT_TYPES)),
        allow_application_xml_suffix=True,
        media_subject="Discovery source response",
        media_description="XML, RSS, or Atom content",
        size_subject="Discovery source",
        user_agent="CanISend/0.3 discovery-refresh",
        accept="application/atom+xml, application/rss+xml, application/xml, text/xml",
        timeout_seconds=source.timeout_seconds,
        max_bytes=source.max_bytes,
        max_attempts=source.max_attempts,
        backoff_seconds=source.backoff_seconds,
        max_retry_delay_seconds=source.max_retry_delay_seconds,
        min_interval_seconds=source.min_interval_seconds,
    )


def _source_artifact_stem(source: DiscoverySourceV1) -> str:
    locator_hash = sha256(source.url.encode("utf-8")).hexdigest()[:12]
    return f"{source.source_id}-{locator_hash}"


def _safe_discovery_root(workspace: Path, lead_root: Path) -> Path:
    candidate = Path(lead_root).expanduser()
    if not candidate.is_absolute():
        candidate = workspace / candidate
    resolved = candidate.resolve()
    try:
        resolved.relative_to(workspace)
    except ValueError as exc:
        raise DiscoveryRefreshInputError(
            "Discovery output directory must remain inside the workspace."
        ) from exc
    if resolved == workspace:
        raise DiscoveryRefreshInputError(
            "Discovery output directory must be a workspace subdirectory."
        )
    return resolved


def _workspace_relative_path(workspace: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(workspace).as_posix()
    except ValueError as exc:
        raise DiscoveryRefreshWriteError(
            "Discovery artifact path escaped the workspace."
        ) from exc


def _safe_artifact_directory(workspace: Path, path: Path) -> Path:
    candidate = Path(path)
    if candidate.is_symlink():
        raise DiscoveryRefreshInputError(
            "Discovery artifact directories must not be symbolic links."
        )
    resolved = candidate.resolve()
    try:
        resolved.relative_to(workspace)
    except ValueError as exc:
        raise DiscoveryRefreshInputError(
            "Discovery artifact directory escaped the workspace."
        ) from exc
    return resolved


def _load_json_contract(path: Path, *, model, label: str):
    try:
        target = Path(path)
        if not target.is_file() or target.is_symlink():
            raise DiscoveryRefreshInputError(f"{label} is unavailable.")
        if target.stat().st_size > 100_000_000:
            raise DiscoveryRefreshInputError(f"{label} exceeds the supported size limit.")
        payload = json.loads(target.read_text(encoding="utf-8"))
        return model.model_validate(payload)
    except DiscoveryRefreshInputError:
        raise
    except (OSError, UnicodeError, json.JSONDecodeError, ValidationError) as exc:
        raise DiscoveryRefreshInputError(f"{label} is invalid.") from exc


def _write_contract(path: Path, value: Any, *, label: str) -> Path:
    try:
        return atomic_write_json(path, value.model_dump(mode="json"))
    except (DiscoveryStoreError, TypeError, ValueError) as exc:
        raise DiscoveryRefreshWriteError(
            f"{label} could not be written atomically."
        ) from exc


def _utc_time(value: datetime) -> datetime:
    if (
        not isinstance(value, datetime)
        or value.tzinfo is None
        or value.utcoffset() is None
    ):
        raise DiscoveryRefreshInputError("Discovery refresh clock must be timezone-aware.")
    return value.astimezone(UTC)


def _read_clock(clock: Callable[[], datetime]) -> datetime:
    try:
        return _utc_time(clock())
    except DiscoveryRefreshInputError:
        raise
    except Exception as exc:
        raise DiscoveryRefreshInputError(
            "Discovery refresh clock could not be read."
        ) from exc
