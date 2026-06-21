from __future__ import annotations

from dataclasses import dataclass
from html.parser import HTMLParser
from pathlib import Path
import re
from typing import Any, Callable
from urllib.parse import urlparse
from urllib.request import Request, urlopen


class JobImportError(ValueError):
    pass


@dataclass(frozen=True)
class ImportedAdvert:
    text: str
    status: str
    notes: str = ""


class _ReadableHTMLParser(HTMLParser):
    _BLOCK_TAGS = {
        "article",
        "br",
        "div",
        "h1",
        "h2",
        "h3",
        "li",
        "p",
        "section",
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
        return ImportedAdvert(
            text=path.read_text(encoding="utf-8"),
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
        reader = reader_factory(path)
        pages = [page.extract_text() or "" for page in reader.pages]
    except Exception as exc:
        raise JobImportError(f"Could not read PDF advert: {exc}") from exc

    text = "\n\n".join(page.strip() for page in pages if page.strip()).strip()
    if not text:
        raise JobImportError("No text could be extracted from the PDF advert.")
    return text


def validate_fetch_url(source_url: str) -> str:
    url = source_url.strip()
    if not url:
        raise JobImportError("--fetch-url requires --source-url.")
    parsed = urlparse(url)
    if parsed.scheme not in {"http", "https"}:
        raise JobImportError("Only http:// and https:// URLs can be fetched.")
    if not parsed.hostname:
        raise JobImportError("Fetch URL must include a host.")
    return url


def fetch_advert_from_url(
    source_url: str,
    *,
    opener: Callable[..., Any] = urlopen,
    timeout: int = 30,
) -> ImportedAdvert:
    url = validate_fetch_url(source_url)
    request = Request(url, headers={"User-Agent": "CanISend/0.2 job-advert-import"})
    try:
        with opener(request, timeout=timeout) as response:
            content_type = response.headers.get("Content-Type", "")
            if "html" not in content_type.lower():
                detail = content_type or "unknown content type"
                raise JobImportError(f"Fetched URL did not return HTML: {detail}")
            raw = response.read()
    except JobImportError:
        raise
    except Exception as exc:
        raise JobImportError(f"Could not fetch job URL: {exc}") from exc

    text = extract_html_text(raw.decode("utf-8", errors="replace"))
    if not text:
        raise JobImportError(
            "No readable text could be extracted from the fetched job page."
        )

    header = (
        f"<!-- Fetched from {url}. "
        "Review extracted text before relying on parsed criteria. -->\n\n"
    )
    return ImportedAdvert(
        text=header + text.rstrip() + "\n",
        status="advert_imported",
        notes=(
            f"Fetched from {url}; "
            "review extracted text before relying on parsed criteria."
        ),
    )


def extract_html_text(html: str) -> str:
    parser = _ReadableHTMLParser()
    parser.feed(html)
    raw_text = "".join(parser.parts)
    lines = [re.sub(r"[ \t]+", " ", line).strip() for line in raw_text.splitlines()]
    meaningful_lines = [line for line in lines if line]
    return "\n".join(meaningful_lines).strip()
