from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
from datetime import datetime
from html import unescape
from html.parser import HTMLParser
import json
import re
from typing import ClassVar, Protocol
from urllib.parse import urlencode, urlsplit

from canisend.discovery.identity import normalize_job_lead
from canisend.discovery.models import JobLeadV2
from canisend.discovery.refresh_models import DiscoverySourceV1
from canisend.discovery.transport import redact_public_url
from canisend.rss import (
    ALLOWED_FEED_CONTENT_TYPES,
    JobFeedError,
    decode_job_feed_bytes,
    parse_job_feed,
)


class DiscoveryAdapterError(ValueError):
    """A stable, body-free adapter failure."""


@dataclass(frozen=True)
class AdapterTransportSpec:
    allowed_media_types: tuple[str, ...]
    media_description: str
    accept: str
    allow_application_xml_suffix: bool = False


class DiscoveryAdapter(Protocol):
    kind: ClassVar[str]
    adapter_id: ClassVar[str]

    def request_url(self, source: DiscoverySourceV1) -> str: ...

    def source_locator(self, source: DiscoverySourceV1) -> str: ...

    def transport_spec(self) -> AdapterTransportSpec: ...

    def validate_final_url(
        self,
        source: DiscoverySourceV1,
        final_url: str,
    ) -> None: ...

    def parse(
        self,
        source: DiscoverySourceV1,
        body: bytes,
        *,
        content_type: str,
        observed_at: datetime,
    ) -> tuple[JobLeadV2, ...]: ...


@dataclass(frozen=True)
class RSSAtomAdapter:
    kind: ClassVar[str] = "rss_atom"
    adapter_id: ClassVar[str] = "feed.rss_atom"

    def request_url(self, source: DiscoverySourceV1) -> str:
        return source.url

    def source_locator(self, source: DiscoverySourceV1) -> str:
        return redact_public_url(self.request_url(source))

    def transport_spec(self) -> AdapterTransportSpec:
        return AdapterTransportSpec(
            allowed_media_types=tuple(sorted(ALLOWED_FEED_CONTENT_TYPES)),
            allow_application_xml_suffix=True,
            media_description="XML, RSS, or Atom content",
            accept="application/atom+xml, application/rss+xml, application/xml, text/xml",
        )

    def validate_final_url(
        self,
        source: DiscoverySourceV1,
        final_url: str,
    ) -> None:
        return None

    def parse(
        self,
        source: DiscoverySourceV1,
        body: bytes,
        *,
        content_type: str,
        observed_at: datetime,
    ) -> tuple[JobLeadV2, ...]:
        try:
            text = decode_job_feed_bytes(body, content_type)
            raw_leads = parse_job_feed(
                text,
                feed_url=source.url,
                source_name=source.name,
            )
            return tuple(
                normalize_job_lead(
                    lead,
                    fetched_at=observed_at,
                    source_type=lead.source_type,  # type: ignore[arg-type]
                )
                for lead in raw_leads
            )
        except JobFeedError as exc:
            raise DiscoveryAdapterError("RSS/Atom source response is invalid.") from exc


@dataclass(frozen=True)
class GreenhouseAdapter:
    kind: ClassVar[str] = "greenhouse"
    adapter_id: ClassVar[str] = "public_api.greenhouse"

    def request_url(self, source: DiscoverySourceV1) -> str:
        if source.board_token is None:
            raise DiscoveryAdapterError("Greenhouse board token is unavailable.")
        return (
            "https://boards-api.greenhouse.io/v1/boards/"
            f"{source.board_token}/jobs?content=true"
        )

    def source_locator(self, source: DiscoverySourceV1) -> str:
        return redact_public_url(self.request_url(source))

    def transport_spec(self) -> AdapterTransportSpec:
        return AdapterTransportSpec(
            allowed_media_types=("application/json",),
            media_description="JSON job-board content",
            accept="application/json",
        )

    def validate_final_url(
        self,
        source: DiscoverySourceV1,
        final_url: str,
    ) -> None:
        _require_exact_api_url(final_url, self.request_url(source))

    def parse(
        self,
        source: DiscoverySourceV1,
        body: bytes,
        *,
        content_type: str,
        observed_at: datetime,
    ) -> tuple[JobLeadV2, ...]:
        document = _json_document(body)
        if not isinstance(document, Mapping):
            raise DiscoveryAdapterError(
                "Greenhouse response root must be an object."
            )
        records = document.get("jobs")
        if not isinstance(records, list):
            raise DiscoveryAdapterError(
                "Greenhouse response requires a jobs list."
            )
        leads: list[JobLeadV2] = []
        for record in records:
            if not isinstance(record, Mapping):
                raise DiscoveryAdapterError("Greenhouse job must be an object.")
            source_record_id = _required_identifier(record, "id", vendor="Greenhouse")
            title = _required_string(record, "title", vendor="Greenhouse")
            source_url = _required_string(
                record,
                "absolute_url",
                vendor="Greenhouse",
            )
            location = _nested_string(record.get("location"), "name")
            institution = _optional_string(record.get("company_name")) or source.name
            leads.append(
                normalize_job_lead(
                    {
                        "title": title,
                        "source_url": source_url,
                        "description": _html_text(
                            _optional_string(record.get("content"))
                        ),
                        "published_at": _optional_string(
                            record.get("first_published")
                        )
                        or _optional_string(record.get("updated_at")),
                        "source": source.name,
                        "source_feed": self.source_locator(source),
                        "source_record_id": source_record_id,
                        "institution": institution,
                        "location": location,
                        "deadline": _optional_string(
                            record.get("application_deadline")
                        ),
                        "source_type": "public_api",
                    },
                    fetched_at=observed_at,
                    source_type="public_api",
                    adapter=self.adapter_id,
                )
            )
        return tuple(leads)


@dataclass(frozen=True)
class LeverAdapter:
    kind: ClassVar[str] = "lever"
    adapter_id: ClassVar[str] = "public_api.lever"

    def request_url(self, source: DiscoverySourceV1) -> str:
        if source.site_id is None:
            raise DiscoveryAdapterError("Lever site identifier is unavailable.")
        hostname = "api.eu.lever.co" if source.region == "eu" else "api.lever.co"
        query = urlencode((("limit", source.max_leads), ("mode", "json")))
        return f"https://{hostname}/v0/postings/{source.site_id}?{query}"

    def source_locator(self, source: DiscoverySourceV1) -> str:
        return redact_public_url(self.request_url(source))

    def transport_spec(self) -> AdapterTransportSpec:
        return AdapterTransportSpec(
            allowed_media_types=("application/json",),
            media_description="JSON published-posting content",
            accept="application/json",
        )

    def validate_final_url(
        self,
        source: DiscoverySourceV1,
        final_url: str,
    ) -> None:
        _require_exact_api_url(final_url, self.request_url(source))

    def parse(
        self,
        source: DiscoverySourceV1,
        body: bytes,
        *,
        content_type: str,
        observed_at: datetime,
    ) -> tuple[JobLeadV2, ...]:
        document = _json_document(body)
        if not isinstance(document, list):
            raise DiscoveryAdapterError("Lever response root must be a list.")
        leads: list[JobLeadV2] = []
        for record in document:
            if not isinstance(record, Mapping):
                raise DiscoveryAdapterError("Lever posting must be an object.")
            source_record_id = _required_identifier(record, "id", vendor="Lever")
            title = _required_string(record, "text", vendor="Lever")
            source_url = _required_string(record, "hostedUrl", vendor="Lever")
            description = (
                _optional_string(record.get("descriptionPlain"))
                or _optional_string(record.get("openingPlain"))
                or _html_text(_optional_string(record.get("description")))
            )
            categories = record.get("categories")
            leads.append(
                normalize_job_lead(
                    {
                        "title": title,
                        "source_url": source_url,
                        "description": description,
                        "published_at": "",
                        "source": source.name,
                        "source_feed": self.source_locator(source),
                        "source_record_id": source_record_id,
                        "institution": source.name,
                        "location": _nested_string(categories, "location"),
                        "deadline": "",
                        "source_type": "public_api",
                    },
                    fetched_at=observed_at,
                    source_type="public_api",
                    adapter=self.adapter_id,
                )
            )
        return tuple(leads)


_ADAPTERS: dict[str, DiscoveryAdapter] = {
    adapter.kind: adapter
    for adapter in (RSSAtomAdapter(), GreenhouseAdapter(), LeverAdapter())
}


def discovery_adapter(source: DiscoverySourceV1) -> DiscoveryAdapter:
    try:
        return _ADAPTERS[source.kind]
    except KeyError as exc:
        raise DiscoveryAdapterError("Discovery source adapter is unsupported.") from exc


def registered_discovery_adapters() -> tuple[DiscoveryAdapter, ...]:
    return tuple(_ADAPTERS[kind] for kind in sorted(_ADAPTERS))


def _json_document(body: bytes):
    try:
        return json.loads(body.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise DiscoveryAdapterError("Discovery API response is not valid UTF-8 JSON.") from exc


def _required_identifier(record: Mapping, field: str, *, vendor: str) -> str:
    value = record.get(field)
    if isinstance(value, bool) or not isinstance(value, (str, int)):
        raise DiscoveryAdapterError(f"{vendor} job requires a published {field}.")
    normalized = str(value).strip()
    if not normalized:
        raise DiscoveryAdapterError(f"{vendor} job requires a published {field}.")
    return normalized


def _required_string(record: Mapping, field: str, *, vendor: str) -> str:
    value = record.get(field)
    if not isinstance(value, str) or not value.strip():
        raise DiscoveryAdapterError(f"{vendor} job requires {field}.")
    return value.strip()


def _optional_string(value: object) -> str:
    return value.strip() if isinstance(value, str) else ""


def _nested_string(value: object, field: str) -> str:
    return _optional_string(value.get(field)) if isinstance(value, Mapping) else ""


def _require_exact_api_url(actual: str, expected: str) -> None:
    if actual != expected:
        raise DiscoveryAdapterError(
            "Public API adapter refused an undocumented redirect."
        )
    parsed = urlsplit(actual)
    if parsed.scheme != "https" or parsed.fragment:
        raise DiscoveryAdapterError("Public API adapter URL is outside its contract.")


class _TextCollector(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []

    def handle_data(self, data: str) -> None:
        self.parts.append(data)


_SPACE_RE = re.compile(r"\s+")


def _html_text(value: str) -> str:
    if not value:
        return ""
    collector = _TextCollector()
    try:
        collector.feed(unescape(unescape(value)))
        collector.close()
    except Exception as exc:
        raise DiscoveryAdapterError("Published job description HTML is invalid.") from exc
    return _SPACE_RE.sub(" ", " ".join(collector.parts)).strip()
