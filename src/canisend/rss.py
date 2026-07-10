from __future__ import annotations

from dataclasses import asdict, dataclass
from html import unescape
import json
from pathlib import Path
import re
from typing import Any, Callable
from urllib.parse import urlsplit, urlunsplit
from urllib.request import Request, urlopen
import xml.etree.ElementTree as ET

from canisend.network_safety import (
    AddressResolver,
    ResolvedAddressError,
    require_public_resolved_addresses,
    resolve_host_addresses,
)


ATOM_NAMESPACE = "http://www.w3.org/2005/Atom"
RDF_NAMESPACE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
DEFAULT_MAX_FEED_BYTES = 2_000_000
ALLOWED_FEED_CONTENT_TYPES = {
    "application/atom+xml",
    "application/rss+xml",
    "application/xml",
    "text/atom+xml",
    "text/rss+xml",
    "text/xml",
}
XML_ENCODING_RE = re.compile(
    br"<\?xml[^>]*\bencoding\s*=\s*['\"]([A-Za-z0-9._:-]+)['\"]",
    flags=re.IGNORECASE,
)


class JobFeedError(ValueError):
    pass


@dataclass(frozen=True)
class JobLead:
    title: str
    source_url: str
    description: str
    published_at: str
    source: str
    source_feed: str


def parse_jobs_ac_uk_rss(xml_text: str, feed_url: str = "") -> list[JobLead]:
    return parse_job_feed(
        xml_text,
        feed_url=feed_url,
        source_name="jobs.ac.uk",
    )


def parse_job_feed(
    xml_text: str,
    *,
    feed_url: str = "",
    source_name: str = "unknown",
) -> list[JobLead]:
    try:
        root = ET.fromstring(xml_text)
    except ET.ParseError as exc:
        raise JobFeedError(f"Malformed job feed XML: {exc}") from exc

    root_name = _local_name(root.tag)
    root_namespace = _namespace(root.tag)
    provenance_url = _feed_provenance_url(feed_url)

    if root_name == "feed" and root_namespace == ATOM_NAMESPACE:
        return _parse_atom_feed(
            root,
            feed_url=provenance_url,
            source_name=source_name,
        )
    rss_version = root.attrib.get("version", "").strip()
    if root_name == "rss" and re.fullmatch(r"2(?:\.0+)?", rss_version):
        return _parse_rss2_feed(
            root,
            feed_url=provenance_url,
            source_name=source_name,
        )
    if root_name == "RDF" and root_namespace == RDF_NAMESPACE:
        return _parse_rss1_feed(
            root,
            feed_url=provenance_url,
            source_name=source_name,
        )

    raise JobFeedError(
        "Unsupported job feed format; expected Atom, RSS 2.0, or RSS 1.0 RDF."
    )


def _parse_rss2_feed(
    root: ET.Element,
    *,
    feed_url: str,
    source_name: str,
) -> list[JobLead]:
    channel = _first_child(root, "channel")
    if channel is None:
        raise JobFeedError("Unsupported RSS 2.0 feed: missing channel element.")
    return [
        lead
        for item in _children(channel, "item")
        if (lead := _rss_item_lead(item, feed_url=feed_url, source_name=source_name)) is not None
    ]


def _parse_rss1_feed(
    root: ET.Element,
    *,
    feed_url: str,
    source_name: str,
) -> list[JobLead]:
    if _first_child(root, "channel") is None:
        raise JobFeedError("Unsupported RSS 1.0 feed: missing channel element.")
    return [
        lead
        for item in _children(root, "item")
        if (lead := _rss_item_lead(item, feed_url=feed_url, source_name=source_name)) is not None
    ]


def _rss_item_lead(
    item: ET.Element,
    *,
    feed_url: str,
    source_name: str,
) -> JobLead | None:
    title = _child_text(item, "title")
    link = _child_text(item, "link")
    if not title and not link:
        return None
    return JobLead(
        title=title,
        source_url=link,
        description=_clean_description(_child_text(item, "description")),
        published_at=_child_text(item, "pubDate") or _child_text(item, "date"),
        source=source_name,
        source_feed=feed_url,
    )


def _parse_atom_feed(
    root: ET.Element,
    *,
    feed_url: str,
    source_name: str,
) -> list[JobLead]:
    leads: list[JobLead] = []

    for entry in _children(root, "entry"):
        title = _child_text(entry, "title")
        link = _atom_link(entry)
        if not title and not link:
            continue
        leads.append(
            JobLead(
                title=title,
                source_url=link,
                description=_clean_description(
                    _child_text(entry, "summary") or _child_text(entry, "content")
                ),
                published_at=(
                    _child_text(entry, "published") or _child_text(entry, "updated")
                ),
                source=source_name,
                source_feed=feed_url,
            )
        )

    return leads


def filter_job_leads(
    leads: list[JobLead],
    *,
    include_keywords: list[str] | None = None,
    exclude_keywords: list[str] | None = None,
) -> list[JobLead]:
    includes = [keyword.lower() for keyword in include_keywords or [] if keyword.strip()]
    excludes = [keyword.lower() for keyword in exclude_keywords or [] if keyword.strip()]
    filtered: list[JobLead] = []

    for lead in leads:
        haystack = f"{lead.title}\n{lead.description}".lower()
        if includes and not any(keyword in haystack for keyword in includes):
            continue
        if excludes and any(keyword in haystack for keyword in excludes):
            continue
        filtered.append(lead)

    return filtered


def fetch_rss_text(
    feed_url: str,
    timeout_seconds: int = 30,
    *,
    opener: Callable[..., Any] = urlopen,
    max_bytes: int = DEFAULT_MAX_FEED_BYTES,
    resolver: AddressResolver | None = None,
) -> str:
    url = _validate_feed_url(feed_url, resolver=resolver)
    if max_bytes <= 0:
        raise JobFeedError("Feed response byte limit must be greater than zero.")

    request = Request(url, headers={"User-Agent": "CanISend/0.2 job-feed-fetch"})
    try:
        with opener(request, timeout=timeout_seconds) as response:
            final_url = response.geturl() if hasattr(response, "geturl") else request.full_url
            _validate_feed_url(str(final_url or request.full_url), resolver=resolver)
            content_type = response.headers.get("Content-Type", "")
            _validate_feed_content_type(content_type)
            raw = _read_limited_response(response, max_bytes=max_bytes)
    except JobFeedError:
        raise
    except Exception as exc:
        raise JobFeedError(f"Could not fetch job feed: {exc}") from exc

    return _decode_feed_xml(raw, content_type)


def write_job_leads(path: Path, leads: list[JobLead]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps([asdict(lead) for lead in leads], indent=2) + "\n",
        encoding="utf-8",
    )
    return path


def _validate_feed_url(
    feed_url: str,
    *,
    resolver: AddressResolver | None = None,
) -> str:
    url = feed_url.strip()
    try:
        parsed = urlsplit(url)
    except ValueError as exc:
        raise JobFeedError("Job feed URL is invalid.") from exc
    if parsed.scheme.lower() not in {"http", "https"}:
        raise JobFeedError("Only http:// and https:// job feed URLs can be fetched.")
    if not parsed.hostname:
        raise JobFeedError("Job feed URL must include a host.")
    if parsed.username is not None or parsed.password is not None:
        raise JobFeedError("Job feed URL must not include credentials.")
    try:
        parsed.port
    except ValueError as exc:
        raise JobFeedError("Job feed URL port is invalid.") from exc

    hostname = parsed.hostname.rstrip(".").lower()
    if (
        hostname in {"localhost", "localhost.localdomain", "ip6-localhost", "ip6-loopback"}
        or hostname.endswith(".localhost")
        or hostname.endswith(".local")
    ):
        raise JobFeedError("Job feed URL must not target localhost.")

    try:
        require_public_resolved_addresses(
            hostname,
            resolver=resolver or resolve_host_addresses,
        )
    except ResolvedAddressError as exc:
        raise JobFeedError(str(exc)) from exc
    return url


def _validate_feed_content_type(content_type: str) -> None:
    media_type = content_type.partition(";")[0].strip().lower()
    if media_type in ALLOWED_FEED_CONTENT_TYPES or (
        media_type.startswith("application/") and media_type.endswith("+xml")
    ):
        return
    detail = media_type or "missing content type"
    raise JobFeedError(f"Job feed response did not return XML, RSS, or Atom content: {detail}")


def _read_limited_response(response: Any, *, max_bytes: int) -> bytes:
    content_length = response.headers.get("Content-Length")
    if content_length:
        try:
            length = int(content_length)
        except (TypeError, ValueError):
            length = None
        if length is not None and length > max_bytes:
            raise JobFeedError(
                f"Job feed response exceeds the configured limit of {max_bytes} bytes."
            )

    raw = response.read(max_bytes + 1)
    if len(raw) > max_bytes:
        raise JobFeedError(
            f"Job feed response exceeds the configured limit of {max_bytes} bytes."
        )
    return raw


def _decode_feed_xml(raw: bytes, content_type: str) -> str:
    charset = _content_type_charset(content_type) or _bom_charset(raw) or _xml_charset(raw)
    encoding = charset or "utf-8-sig"
    if encoding.lower().replace("_", "-") in {"utf8", "utf-8"}:
        encoding = "utf-8-sig"
    try:
        return raw.decode(encoding)
    except LookupError as exc:
        raise JobFeedError(f"Job feed declares an unsupported charset: {encoding}") from exc
    except UnicodeDecodeError as exc:
        raise JobFeedError(f"Could not decode job feed using charset {encoding}: {exc}") from exc


def _content_type_charset(content_type: str) -> str:
    match = re.search(
        r"(?:^|;)\s*charset\s*=\s*['\"]?([^;'\"\s]+)",
        content_type,
        flags=re.IGNORECASE,
    )
    return match.group(1) if match else ""


def _xml_charset(raw: bytes) -> str:
    match = XML_ENCODING_RE.search(raw[:256])
    return match.group(1).decode("ascii") if match else ""


def _bom_charset(raw: bytes) -> str:
    if raw.startswith((b"\xff\xfe\x00\x00", b"\x00\x00\xfe\xff")):
        return "utf-32"
    if raw.startswith((b"\xff\xfe", b"\xfe\xff")):
        return "utf-16"
    if raw.startswith(b"\xef\xbb\xbf"):
        return "utf-8-sig"
    return ""


def _feed_provenance_url(feed_url: str) -> str:
    if not feed_url:
        return ""
    parsed = urlsplit(feed_url)
    netloc = parsed.netloc.rsplit("@", 1)[-1]
    query = "redacted" if parsed.query else ""
    return urlunsplit((parsed.scheme, netloc, parsed.path, query, ""))


def _child_text(item: ET.Element, tag: str) -> str:
    child = _first_child(item, tag)
    if child is None:
        return ""
    return "".join(child.itertext()).strip()


def _atom_link(entry: ET.Element) -> str:
    links = _children(entry, "link")
    for link in links:
        href = link.attrib.get("href", "").strip()
        if link.attrib.get("rel", "alternate") in {"", "alternate"} and href:
            return href
    for link in links:
        href = link.attrib.get("href", "").strip()
        if href:
            return href
        text = "".join(link.itertext()).strip()
        if text:
            return text
    return ""


def _first_child(item: ET.Element, tag: str) -> ET.Element | None:
    return next((child for child in item if _local_name(child.tag) == tag), None)


def _children(item: ET.Element, tag: str) -> list[ET.Element]:
    return [child for child in item if _local_name(child.tag) == tag]


def _namespace(tag: str) -> str:
    if tag.startswith("{") and "}" in tag:
        return tag[1:].split("}", 1)[0]
    return ""


def _local_name(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _clean_description(value: str) -> str:
    without_tags = re.sub(r"<[^>]+>", " ", value)
    compact = re.sub(r"\s+", " ", without_tags)
    return unescape(compact).strip()
