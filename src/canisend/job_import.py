from __future__ import annotations

from dataclasses import dataclass
from html.parser import HTMLParser
from io import BytesIO
from pathlib import Path
import re
from typing import Any, Callable
from urllib.parse import quote, unquote, urlsplit, urlunsplit
from urllib.request import urlopen  # retained for compatibility/offline sentinels

from canisend.network_safety import (
    AddressResolver,
    resolve_host_addresses,
)
from canisend.discovery.transport import (
    PublicTransport,
    PublicTransportError,
    TransportOpener,
    TransportPolicy,
    validate_public_http_url,
)


_DEFAULT_MAX_BYTES = 2_000_000


class JobImportError(ValueError):
    pass


@dataclass(frozen=True)
class ImportedAdvert:
    text: str
    status: str
    notes: str = ""
    metadata_source_url: str = ""


class _ReadableHTMLParser(HTMLParser):
    _BLOCK_TAGS = {
        "article",
        "blockquote",
        "br",
        "div",
        "h1",
        "h2",
        "h3",
        "li",
        "main",
        "ol",
        "p",
        "section",
        "table",
        "td",
        "th",
        "tr",
        "ul",
    }
    _SKIP_TAGS = {"noscript", "script", "style"}

    def __init__(self) -> None:
        super().__init__()
        self._skip_depth = 0
        self.parts: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        tag_name = tag.lower()
        if tag_name in self._SKIP_TAGS:
            self._skip_depth += 1
            return
        if tag_name in self._BLOCK_TAGS:
            self.parts.append("\n")

    def handle_endtag(self, tag: str) -> None:
        tag_name = tag.lower()
        if tag_name in self._SKIP_TAGS and self._skip_depth:
            self._skip_depth -= 1
            return
        if tag_name in self._BLOCK_TAGS - {"br"}:
            self.parts.append("\n")

    def handle_data(self, data: str) -> None:
        if not self._skip_depth:
            self.parts.append(data)


def import_advert_file(path: Path) -> ImportedAdvert:
    suffix = path.suffix.lower()
    if suffix in {".md", ".txt"}:
        if not path.is_file():
            raise JobImportError(f"Could not read job advert file {path}: not a file.")
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as exc:
            raise JobImportError(f"Could not read job advert file {path}: {exc}") from exc
        return ImportedAdvert(
            text=text,
            status="advert_imported",
        )
    if suffix == ".pdf":
        text = extract_pdf_text(path)
        header = (
            f"<!-- Imported from local PDF: {path.name}. "
            "Review extracted text before relying on parsed criteria. -->\n\n"
        )
        return ImportedAdvert(
            text=header + text.rstrip() + "\n",
            status="advert_imported",
            notes=(
                f"Imported from local PDF {path.name}; "
                "review extracted text before relying on parsed criteria."
            ),
        )
    raise JobImportError("CanISend imports local .md, .txt, or .pdf job advert files.")


def extract_pdf_text(
    path: Path, *, reader_factory: Callable[[Path], Any] | None = None
) -> str:
    if reader_factory is None:
        try:
            from pypdf import PdfReader
        except ImportError as exc:
            raise JobImportError("PDF import requires the pypdf package.") from exc
        reader_factory = PdfReader

    try:
        return _extract_pdf_reader_text(reader_factory(path))
    except JobImportError:
        raise
    except Exception as exc:
        raise JobImportError(f"Could not read PDF advert: {exc}") from exc


def extract_pdf_bytes(
    pdf_bytes: bytes,
    *,
    reader_factory: Callable[[Any], Any] | None = None,
) -> str:
    if reader_factory is None:
        try:
            from pypdf import PdfReader
        except ImportError as exc:
            raise JobImportError("PDF import requires the pypdf package.") from exc
        reader_factory = PdfReader

    try:
        return _extract_pdf_reader_text(reader_factory(BytesIO(pdf_bytes)))
    except JobImportError:
        raise
    except Exception as exc:
        raise JobImportError(f"Could not read PDF advert: {exc}") from exc


def _extract_pdf_reader_text(reader: Any) -> str:
    pages = [page.extract_text() or "" for page in reader.pages]
    text = "\n\n".join(page.strip() for page in pages if page.strip()).strip()
    if not text:
        raise JobImportError("No text could be extracted from the PDF advert.")
    return text


def validate_fetch_url(
    source_url: str,
    *,
    resolver: AddressResolver | None = None,
) -> str:
    if not source_url.strip():
        raise JobImportError("--fetch-url requires --source-url.")
    try:
        return validate_public_http_url(
            source_url,
            resolver=resolver or resolve_host_addresses,
            label="Fetch URL",
        )
    except PublicTransportError as exc:
        raise JobImportError(str(exc)) from exc


def _provenance_url(fetch_url: str) -> str:
    parsed = urlsplit(fetch_url)
    safe_path = quote(unquote(parsed.path), safe="/:@!$&'()*+,;=%")
    safe_query = "redacted" if parsed.query else ""
    return urlunsplit((parsed.scheme, parsed.netloc, safe_path, safe_query, ""))


def fetch_advert_from_url(
    source_url: str,
    *,
    opener: TransportOpener | None = None,
    timeout: int = 30,
    max_bytes: int = _DEFAULT_MAX_BYTES,
    resolver: AddressResolver | None = None,
) -> ImportedAdvert:
    validated_url = validate_fetch_url(source_url, resolver=resolver)
    try:
        response = PublicTransport(
            opener=opener,
            resolver=resolver or resolve_host_addresses,
        ).fetch(
            validated_url,
            policy=TransportPolicy(
                allowed_media_types=(
                    "application/pdf",
                    "application/xhtml+xml",
                    "text/html",
                ),
                media_subject="Fetched URL",
                media_description="HTML or PDF",
                size_subject="Fetched URL",
                user_agent="CanISend/0.2 job-advert-import",
                accept="text/html, application/xhtml+xml, application/pdf",
                timeout_seconds=timeout,
                max_bytes=max_bytes,
            ),
        )
    except PublicTransportError as exc:
        if exc.code == "transport.response_too_large":
            raise JobImportError(
                "Fetched URL response is larger than the configured limit "
                f"of {max_bytes} bytes."
            ) from exc
        raise JobImportError(str(exc)) from exc
    except ValueError as exc:
        raise JobImportError(str(exc)) from exc

    raw = response.body
    content_type = response.content_type
    media_type = response.media_type
    provenance_url = _provenance_url(response.final_url)
    if media_type == "application/pdf":
        text = extract_pdf_bytes(raw)
        source_label = "PDF from"
    else:
        charset_match = re.search(
            r"(?:^|;)\s*charset\s*=\s*['\"]?([^;'\"\s]+)",
            content_type,
            flags=re.IGNORECASE,
        )
        charset = charset_match.group(1) if charset_match else "utf-8"
        try:
            html = raw.decode(charset, errors="replace")
        except LookupError as exc:
            raise JobImportError(f"Fetched URL declares unsupported charset: {charset}") from exc
        text = extract_html_text(html)
        source_label = "from"
    if not text:
        raise JobImportError(
            "No readable text could be extracted from the fetched job page."
        )

    header = (
        f"Fetched {source_label} {provenance_url}\n"
        "Review extracted text before relying on parsed criteria.\n\n"
    )
    return ImportedAdvert(
        text=header + text.rstrip() + "\n",
        status="advert_imported",
        notes=(
            f"Fetched {source_label} {provenance_url}; "
            "review extracted text before relying on parsed criteria."
        ),
        metadata_source_url=provenance_url,
    )


def extract_html_text(html: str) -> str:
    parser = _ReadableHTMLParser()
    parser.feed(html)
    raw_text = "".join(parser.parts)
    lines = [re.sub(r"[ \t]+", " ", line).strip() for line in raw_text.splitlines()]
    meaningful_lines = [line for line in lines if line]
    return "\n".join(meaningful_lines).strip()
