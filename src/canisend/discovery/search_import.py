from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
from hashlib import sha256
import json
from pathlib import Path

from pydantic import ValidationError

from canisend.discovery.catalog import (
    DiscoveryInputError,
    DiscoveryWriteError,
    load_lead_document,
    merge_lead_catalog,
    write_lead_catalog,
)
from canisend.discovery.catalog_models import LeadCatalogV1, RankingPolicyV1
from canisend.discovery.identity import LeadNormalizationError, normalize_job_lead
from canisend.discovery.models import JobLeadV2
from canisend.discovery.refresh import (
    DiscoveryRefreshInputError,
    DiscoveryRefreshWriteError,
    load_lead_batch,
    write_lead_batch,
)
from canisend.discovery.refresh_models import (
    LeadBatchV1,
    batch_identifier,
    lead_batch_sort_key,
)
from canisend.discovery.search_models import DiscoverySearchV1


_MAX_INPUT_BYTES = 25_000_000
_HOST_SEARCH_ADAPTER = "host.search"
_HOST_SEARCH_LOCATOR = "host-search"


class DiscoverySearchImportError(ValueError):
    pass


class DiscoverySearchImportInputError(DiscoverySearchImportError):
    pass


class DiscoverySearchImportWriteError(DiscoverySearchImportError):
    pass


@dataclass(frozen=True)
class DiscoverySearchImportExecution:
    envelope: DiscoverySearchV1
    batch: LeadBatchV1
    batch_path: Path
    catalog: LeadCatalogV1
    catalog_path: Path


def load_discovery_search(path: Path) -> DiscoverySearchV1:
    """Load one strict host-neutral search envelope without retaining its path."""

    try:
        target = Path(path)
        if not target.is_file():
            raise DiscoverySearchImportInputError(
                "Discovery search input must be a readable JSON file."
            )
        if target.stat().st_size > _MAX_INPUT_BYTES:
            raise DiscoverySearchImportInputError(
                "Discovery search input exceeds the supported size limit."
            )
        raw = target.read_bytes()
    except DiscoverySearchImportInputError:
        raise
    except OSError as exc:
        raise DiscoverySearchImportInputError(
            "Discovery search input could not be read."
        ) from exc
    try:
        document = json.loads(raw.decode("utf-8"))
        return DiscoverySearchV1.model_validate(document)
    except (UnicodeError, json.JSONDecodeError, ValidationError) as exc:
        raise DiscoverySearchImportInputError(
            "Discovery search input must match canisend.discovery-search/v1."
        ) from exc


def import_host_search_file(
    workspace: Path,
    input_path: Path,
    *,
    policy: RankingPolicyV1 | None = None,
    lead_root: Path | None = None,
    clock=None,
) -> DiscoverySearchImportExecution:
    envelope = load_discovery_search(input_path)
    return import_host_search(
        workspace,
        envelope,
        policy=policy,
        lead_root=lead_root,
        clock=clock,
    )


def import_host_search(
    workspace: Path,
    envelope: DiscoverySearchV1,
    *,
    policy: RankingPolicyV1 | None = None,
    lead_root: Path | None = None,
    clock=None,
) -> DiscoverySearchImportExecution:
    """Promote a validated host search through the normal Lead/catalog pipeline."""

    workspace_root = Path(workspace).expanduser().resolve()
    discovery_root = _safe_discovery_root(
        workspace_root,
        lead_root or workspace_root / "job_leads",
    )
    searches_dir = _safe_artifact_directory(
        workspace_root,
        discovery_root / "searches",
    )
    imported_at = _read_clock(clock or (lambda: datetime.now(UTC).replace(microsecond=0)))
    search = DiscoverySearchV1.model_validate(envelope.model_dump(mode="json"))
    if search.observed_at > imported_at:
        raise DiscoverySearchImportInputError(
            "Discovery search observed_at cannot be in the future."
        )

    leads = _normalized_search_leads(search)
    content_sha256 = _normalized_search_digest(search, leads)
    candidate_batch = LeadBatchV1(
        batch_id=batch_identifier(
            source_id=search.source_id,
            content_sha256=content_sha256,
        ),
        source_id=search.source_id,
        source_name=search.source_name,
        adapter=_HOST_SEARCH_ADAPTER,
        source_url=_HOST_SEARCH_LOCATOR,
        fetched_at=search.observed_at,
        content_sha256=content_sha256,
        record_count=len(leads),
        leads=leads,
    )
    batch_path = searches_dir / f"{search.source_id}.batch.json"
    previous_batch = _compatible_previous_batch(batch_path, candidate_batch)
    batch = previous_batch or candidate_batch
    try:
        if previous_batch is None:
            write_lead_batch(batch_path, batch)
    except DiscoveryRefreshWriteError as exc:
        raise DiscoverySearchImportWriteError(
            "Discovery search batch could not be written atomically."
        ) from exc

    catalog_path = discovery_root / "catalog.json"
    try:
        existing_leads = _existing_catalog_leads(catalog_path)
        all_leads = [*existing_leads, *batch.leads]
        catalog = merge_lead_catalog(
            all_leads,
            policy=policy or RankingPolicyV1(),
            input_record_count=len(all_leads),
            generated_at=imported_at,
        )
        write_lead_catalog(catalog_path, catalog)
    except DiscoveryInputError as exc:
        raise DiscoverySearchImportInputError(
            "Existing discovery catalog is invalid."
        ) from exc
    except DiscoveryWriteError as exc:
        raise DiscoverySearchImportWriteError(
            "Discovery search catalog could not be promoted atomically."
        ) from exc
    except (ValidationError, ValueError) as exc:
        raise DiscoverySearchImportInputError(
            "Discovery search results could not be normalized."
        ) from exc

    return DiscoverySearchImportExecution(
        envelope=search,
        batch=batch,
        batch_path=batch_path,
        catalog=catalog,
        catalog_path=catalog_path,
    )


def _normalized_search_leads(search: DiscoverySearchV1) -> tuple[JobLeadV2, ...]:
    leads: list[JobLeadV2] = []
    try:
        for result in search.results:
            leads.append(
                normalize_job_lead(
                    {
                        "title": result.title,
                        "source_url": result.source_url,
                        "description": result.snippet,
                        "published_at": result.published_at,
                        "source": search.source_name,
                        "source_feed": _HOST_SEARCH_LOCATOR,
                        "source_record_id": result.source_record_id,
                        "institution": result.institution,
                        "location": result.location,
                        "deadline": result.deadline,
                        "source_type": "host_agent",
                    },
                    fetched_at=search.observed_at,
                    source_type="host_agent",
                    adapter=_HOST_SEARCH_ADAPTER,
                )
            )
    except (LeadNormalizationError, ValidationError, ValueError) as exc:
        raise DiscoverySearchImportInputError(
            "Discovery search results could not be normalized."
        ) from exc
    return tuple(sorted(leads, key=lead_batch_sort_key))


def _normalized_search_digest(
    search: DiscoverySearchV1,
    leads: tuple[JobLeadV2, ...],
) -> str:
    normalized = {
        "protocol": search.protocol,
        "schema_version": search.schema_version,
        "source_id": search.source_id,
        "source_name": search.source_name,
        "observed_at": search.observed_at.isoformat(),
        "leads": [lead.model_dump(mode="json") for lead in leads],
    }
    payload = json.dumps(
        normalized,
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(payload).hexdigest()


def _existing_catalog_leads(path: Path) -> list[JobLeadV2]:
    if not path.exists():
        return []
    if not path.is_file() or path.is_symlink():
        raise DiscoveryInputError("Existing discovery catalog is unavailable.")
    return load_lead_document(path)


def _compatible_previous_batch(
    path: Path,
    candidate: LeadBatchV1,
) -> LeadBatchV1 | None:
    try:
        previous = load_lead_batch(path)
    except DiscoveryRefreshInputError:
        return None
    if (
        previous.batch_id == candidate.batch_id
        and previous.source_id == candidate.source_id
        and previous.source_name == candidate.source_name
        and previous.adapter == candidate.adapter
        and previous.source_url == candidate.source_url
    ):
        return previous
    return None


def _safe_discovery_root(workspace: Path, lead_root: Path) -> Path:
    candidate = Path(lead_root).expanduser()
    if not candidate.is_absolute():
        candidate = workspace / candidate
    resolved = candidate.resolve()
    try:
        resolved.relative_to(workspace)
    except ValueError as exc:
        raise DiscoverySearchImportInputError(
            "Discovery output directory must remain inside the workspace."
        ) from exc
    if resolved == workspace:
        raise DiscoverySearchImportInputError(
            "Discovery output directory must be a workspace subdirectory."
        )
    return resolved


def _safe_artifact_directory(workspace: Path, path: Path) -> Path:
    candidate = Path(path)
    if candidate.is_symlink():
        raise DiscoverySearchImportInputError(
            "Discovery search directory must not be a symbolic link."
        )
    if candidate.exists() and not candidate.is_dir():
        raise DiscoverySearchImportInputError(
            "Discovery search directory must be a directory."
        )
    resolved = candidate.resolve()
    try:
        resolved.relative_to(workspace)
    except ValueError as exc:
        raise DiscoverySearchImportInputError(
            "Discovery search directory escaped the workspace."
        ) from exc
    return resolved


def _read_clock(clock) -> datetime:
    try:
        value = clock()
    except Exception as exc:
        raise DiscoverySearchImportInputError(
            "Discovery search import clock could not be read."
        ) from exc
    if (
        not isinstance(value, datetime)
        or value.tzinfo is None
        or value.utcoffset() is None
    ):
        raise DiscoverySearchImportInputError(
            "Discovery search import clock must be timezone-aware."
        )
    return value.astimezone(UTC)
