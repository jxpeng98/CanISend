from __future__ import annotations

from collections.abc import Mapping
import csv
from dataclasses import dataclass
from datetime import UTC, datetime
from email import policy as email_policy
from email.message import Message
from email.parser import BytesParser
from hashlib import sha256
from html.parser import HTMLParser
import io
import ipaddress
import json
from pathlib import Path, PureWindowsPath
import re
from typing import Literal
from urllib.parse import unquote, urlsplit

from pydantic import TypeAdapter, ValidationError

from canisend.discovery.catalog import (
    DiscoveryInputError,
    DiscoveryWriteError,
    load_lead_document,
    merge_lead_catalog,
    write_lead_catalog,
)
from canisend.discovery.catalog_models import LeadCatalogV1, RankingPolicyV1
from canisend.discovery.identity import (
    LeadNormalizationError,
    canonicalize_job_url,
    normalize_job_lead,
)
from canisend.discovery.import_models import (
    DiscoveryImportIssueV1,
    DiscoveryImportReportV1,
    ImportFormat,
    import_identifier,
)
from canisend.discovery.models import JobLeadV2, LeadProvenanceV1
from canisend.discovery.refresh import (
    DiscoveryRefreshInputError,
    DiscoveryRefreshWriteError,
    load_lead_batch,
    write_lead_batch,
)
from canisend.discovery.refresh_models import (
    LeadBatchV1,
    SourceIdentifier,
    batch_identifier,
    lead_batch_sort_key,
)
from canisend.discovery.store import DiscoveryStoreError, atomic_write_json


_MAX_INPUT_BYTES = 25_000_000
_MAX_RECORDS = 100_000
_MAX_STORED_ISSUES = 1_000
_MAX_MESSAGES = 10_000
_SOURCE_ID_ADAPTER = TypeAdapter(SourceIdentifier)
_CONTROL_RE = re.compile(r"[\x00-\x1f\x7f]")
_SPACE_RE = re.compile(r"\s+")
_EMAIL_RE = re.compile(
    r"(?i)(?<![A-Z0-9._%+-])[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}"
)
_INLINE_SECRET_RE = re.compile(
    r"(?i)(?:access[_-]?token|api[_-]?key|auth|credential|password|secret|"
    r"session|signature|token)\s*[:=]"
)
_PLAIN_URL_RE = re.compile(r"https?://[^\s<>\"']+", re.IGNORECASE)
_JOB_SIGNAL_RE = re.compile(
    r"(?i)(?:\bjobs?\b|\bcareers?\b|vacanc|opening|posting|position|opportunit|"
    r"lecturer|professor|faculty|postdoc|research[\s_-]+fellow|teaching[\s_-]+fellow|"
    r"assistant[\s_-]+professor|associate[\s_-]+professor|reader)"
)
_NON_JOB_LINK_RE = re.compile(
    r"(?i)(?:unsubscribe|email[\s_-]*preferences?|privacy|cookie|terms[\s_-]*of|"
    r"view[\s_-]*in[\s_-]*browser|sign[\s_-]*in|log[\s_-]*in|facebook|instagram|"
    r"linkedin|twitter|youtube)"
)

_CSV_ALIASES = {
    "title": {"title", "job_title", "position", "position_title", "role"},
    "source_url": {"source_url", "url", "job_url", "job_link", "link"},
    "description": {"description", "summary", "snippet", "job_description"},
    "published_at": {"published_at", "published", "posted_at", "date_posted"},
    "source_record_id": {
        "source_record_id",
        "id",
        "job_id",
        "reference",
        "reference_number",
    },
    "institution": {
        "institution",
        "employer",
        "company",
        "organisation",
        "organization",
        "university",
    },
    "location": {"location", "city", "place"},
    "deadline": {
        "deadline",
        "closing_date",
        "closes_at",
        "application_deadline",
    },
}
_CSV_ALIAS_TO_FIELD = {
    alias: field
    for field, aliases in _CSV_ALIASES.items()
    for alias in aliases
}
_LEGACY_JSON_FIELDS = frozenset(
    {
        "title",
        "source_url",
        "description",
        "published_at",
        "source",
        "source_feed",
        "source_record_id",
        "institution",
        "location",
        "deadline",
        "source_type",
    }
)


class DiscoveryLocalImportError(ValueError):
    pass


class DiscoveryLocalImportInputError(DiscoveryLocalImportError):
    pass


class DiscoveryLocalImportWriteError(DiscoveryLocalImportError):
    pass


@dataclass(frozen=True)
class DiscoveryLocalImportExecution:
    report: DiscoveryImportReportV1
    report_path: Path
    batch: LeadBatchV1 | None
    batch_path: Path | None
    catalog: LeadCatalogV1 | None
    catalog_path: Path | None


@dataclass(frozen=True)
class _ParsedImport:
    format: ImportFormat
    source_id: str
    source_name: str
    adapter: str
    locator: str
    input_records: int
    imported_records: int
    rejected_records: int
    ignored_records: int
    issues: tuple[DiscoveryImportIssueV1, ...]
    leads: tuple[JobLeadV2, ...]
    supplied_batch: LeadBatchV1 | None = None


class _AnchorCollector(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.links: list[tuple[str, str]] = []
        self._href: str | None = None
        self._text: list[str] = []

    def handle_starttag(self, tag: str, attrs) -> None:
        if tag.casefold() != "a" or self._href is not None:
            return
        values = {str(name).casefold(): str(value or "") for name, value in attrs}
        self._href = values.get("href", "").strip()
        self._text = []

    def handle_data(self, data: str) -> None:
        if self._href is not None:
            self._text.append(data)

    def handle_endtag(self, tag: str) -> None:
        if tag.casefold() == "a" and self._href is not None:
            self.links.append((self._href, "".join(self._text)))
            self._href = None
            self._text = []


def import_local_discovery_file(
    workspace: Path,
    input_path: Path,
    *,
    source_name: str | None = None,
    source_id: str | None = None,
    policy: RankingPolicyV1 | None = None,
    lead_root: Path | None = None,
    clock=None,
) -> DiscoveryLocalImportExecution:
    workspace_root = Path(workspace).expanduser().resolve()
    discovery_root = _safe_discovery_root(
        workspace_root,
        lead_root or workspace_root / "job_leads",
    )
    imports_dir = _safe_artifact_directory(
        workspace_root,
        discovery_root / "imports",
    )
    imported_at = _read_clock(clock or (lambda: datetime.now(UTC).replace(microsecond=0)))
    target = Path(input_path).expanduser()
    raw = _read_input_bytes(target)
    input_sha256 = sha256(raw).hexdigest()
    import_format = _input_format(target)
    parsed = _parse_import(
        raw,
        format=import_format,
        source_name=source_name,
        source_id=source_id,
        observed_at=imported_at,
    )

    batch_path = imports_dir / f"{parsed.source_id}.batch.json"
    report_path = imports_dir / f"{parsed.source_id}.report.json"
    candidate_catalog_path = discovery_root / "catalog.json"
    batch: LeadBatchV1 | None = None
    catalog: LeadCatalogV1 | None = None
    error_code: str | None = None

    if parsed.imported_records < 1:
        error_code = "import.no_valid_records"
    else:
        candidate_batch = parsed.supplied_batch or _build_local_batch(
            parsed,
            imported_at=imported_at,
            content_sha256=input_sha256,
        )
        previous_batch = _load_compatible_previous_batch(batch_path, candidate_batch)
        batch = previous_batch or candidate_batch
        try:
            if previous_batch is None:
                write_lead_batch(batch_path, batch)
        except DiscoveryRefreshWriteError:
            batch = None
            error_code = "store.import_batch_failed"

    if batch is not None and error_code is None:
        try:
            existing_leads = _existing_catalog_leads(candidate_catalog_path)
            all_leads = [*existing_leads, *batch.leads]
            candidate_catalog = merge_lead_catalog(
                all_leads,
                policy=policy or RankingPolicyV1(),
                input_record_count=len(all_leads),
                generated_at=imported_at,
            )
            write_lead_catalog(candidate_catalog_path, candidate_catalog)
            catalog = candidate_catalog
        except DiscoveryInputError:
            error_code = "catalog.existing_invalid"
        except DiscoveryWriteError:
            error_code = "catalog.promotion_failed"
        except (ValidationError, ValueError):
            error_code = "catalog.import_invalid"

    status = (
        "failed"
        if catalog is None
        else "partial"
        if parsed.rejected_records
        else "complete"
    )
    relative_batch_path = (
        _workspace_relative_path(workspace_root, batch_path) if batch else None
    )
    relative_catalog_path = (
        _workspace_relative_path(workspace_root, candidate_catalog_path)
        if catalog
        else None
    )
    stored_issues = tuple(sorted(parsed.issues, key=_issue_key)[:_MAX_STORED_ISSUES])
    report_id = import_identifier(
        imported_at=imported_at,
        format=parsed.format,
        source_id=parsed.source_id,
        input_sha256=input_sha256,
        status=status,
        batch_id=batch.batch_id if batch else None,
        catalog_id=catalog.catalog_id if catalog else None,
        error_code=error_code,
        input_records=parsed.input_records,
        imported_records=parsed.imported_records,
        rejected_records=parsed.rejected_records,
        ignored_records=parsed.ignored_records,
        issue_count=parsed.rejected_records,
        skipped_batches=0,
    )
    report = DiscoveryImportReportV1(
        import_id=report_id,
        imported_at=imported_at,
        format=parsed.format,
        source_id=parsed.source_id,
        source_name=parsed.source_name,
        input_sha256=input_sha256,
        status=status,
        catalog_promoted=catalog is not None,
        error_code=error_code,
        input_records=parsed.input_records,
        imported_records=parsed.imported_records,
        rejected_records=parsed.rejected_records,
        ignored_records=parsed.ignored_records,
        issue_count=parsed.rejected_records,
        issues_truncated=parsed.rejected_records > len(stored_issues),
        issues=stored_issues,
        batch_id=batch.batch_id if batch else None,
        batch_path=relative_batch_path,
        catalog_id=catalog.catalog_id if catalog else None,
        catalog_path=relative_catalog_path,
        catalog_input_records=catalog.stats.input_records if catalog else 0,
        merged_records=catalog.stats.merged_records if catalog else 0,
        retained_records=catalog.stats.retained_records if catalog else 0,
        excluded_records=catalog.stats.excluded_records if catalog else 0,
    )
    _write_import_report(report_path, report)
    return DiscoveryLocalImportExecution(
        report=report,
        report_path=report_path,
        batch=batch,
        batch_path=batch_path if batch else None,
        catalog=catalog,
        catalog_path=candidate_catalog_path if catalog else None,
    )


def load_local_import_batches(path: Path) -> tuple[tuple[LeadBatchV1, ...], int]:
    directory = Path(path)
    if not directory.exists():
        return (), 0
    if not directory.is_dir() or directory.is_symlink():
        return (), 1
    batches: list[LeadBatchV1] = []
    skipped = 0
    for candidate in sorted(directory.glob("*.batch.json"), key=lambda item: item.name):
        try:
            batches.append(load_lead_batch(candidate))
        except DiscoveryRefreshInputError:
            skipped += 1
    return tuple(batches), skipped


def _parse_import(
    raw: bytes,
    *,
    format: ImportFormat,
    source_name: str | None,
    source_id: str | None,
    observed_at: datetime,
) -> _ParsedImport:
    if format == "csv":
        name, identifier = _resolved_local_source(source_name, source_id)
        return _parse_csv(raw, name, identifier, observed_at)
    if format == "json":
        return _parse_json(
            raw,
            source_name=source_name,
            source_id=source_id,
            observed_at=observed_at,
        )
    name, identifier = _resolved_local_source(source_name, source_id)
    return _parse_email_export(
        raw,
        format=format,
        source_name=name,
        source_id=identifier,
        observed_at=observed_at,
    )


def _parse_csv(
    raw: bytes,
    source_name: str,
    source_id: str,
    observed_at: datetime,
) -> _ParsedImport:
    try:
        text = raw.decode("utf-8-sig")
    except UnicodeDecodeError as exc:
        raise DiscoveryLocalImportInputError(
            "CSV discovery imports must contain valid UTF-8 text."
        ) from exc
    try:
        reader = csv.DictReader(io.StringIO(text, newline=""))
        if not reader.fieldnames:
            raise DiscoveryLocalImportInputError(
                "CSV discovery imports require a header row."
            )
        mapped_headers = _mapped_csv_headers(reader.fieldnames)
        if not ({"title", "source_url"} & set(mapped_headers.values())):
            raise DiscoveryLocalImportInputError(
                "CSV discovery imports require a recognized title or job URL column."
            )
        leads: list[JobLeadV2] = []
        issues: list[DiscoveryImportIssueV1] = []
        input_records = rejected = ignored = 0
        for record_number, row in enumerate(reader, start=1):
            input_records += 1
            if input_records > _MAX_RECORDS:
                raise DiscoveryLocalImportInputError(
                    "Discovery import exceeds the supported record limit."
                )
            if None in row:
                rejected += 1
                issues.append(_issue(record_number, "import.row_invalid"))
                continue
            payload = {
                mapped_headers[header]: str(value or "").strip()
                for header, value in row.items()
                if header in mapped_headers
            }
            if not any(payload.values()):
                ignored += 1
                continue
            payload.update(
                {
                    "source": source_name,
                    "source_feed": "local-csv",
                    "source_type": "csv",
                }
            )
            try:
                leads.append(
                    _private_safe_import_lead(
                        normalize_job_lead(
                            payload,
                            fetched_at=observed_at,
                            source_type="csv",
                            adapter="local.csv",
                        )
                    )
                )
            except (LeadNormalizationError, ValidationError, ValueError):
                rejected += 1
                issues.append(_issue(record_number, "import.row_invalid"))
    except csv.Error as exc:
        raise DiscoveryLocalImportInputError(
            "CSV discovery import syntax is invalid."
        ) from exc
    return _ParsedImport(
        format="csv",
        source_id=source_id,
        source_name=source_name,
        adapter="local.csv",
        locator="local-csv",
        input_records=input_records,
        imported_records=len(leads),
        rejected_records=rejected,
        ignored_records=ignored,
        issues=tuple(issues),
        leads=tuple(leads),
    )


def _parse_json(
    raw: bytes,
    *,
    source_name: str | None,
    source_id: str | None,
    observed_at: datetime,
) -> _ParsedImport:
    try:
        document = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise DiscoveryLocalImportInputError(
            "JSON discovery imports must contain valid UTF-8 JSON."
        ) from exc
    if isinstance(document, dict):
        if document.get("protocol") != "canisend.discovery-batch/v1":
            raise DiscoveryLocalImportInputError(
                "JSON discovery imports must be a lead list or strict CanISend Lead Batch."
            )
        try:
            batch = LeadBatchV1.model_validate(document)
        except ValidationError as exc:
            raise DiscoveryLocalImportInputError(
                "The CanISend Lead Batch import is invalid."
            ) from exc
        batch_source_name = _safe_source_name(batch.source_name)
        batch_source_id = _safe_source_id(batch.source_id)
        _private_safe_locator(batch.source_url)
        for lead in batch.leads:
            _private_safe_import_lead(lead)
        if source_name is not None and _safe_source_name(source_name) != batch_source_name:
            raise DiscoveryLocalImportInputError(
                "--source-name must match a versioned Lead Batch source name."
            )
        if source_id is not None and _safe_source_id(source_id) != batch_source_id:
            raise DiscoveryLocalImportInputError(
                "--source-id must match a versioned Lead Batch source ID."
            )
        return _ParsedImport(
            format="json",
            source_id=batch_source_id,
            source_name=batch_source_name,
            adapter=batch.adapter,
            locator=batch.source_url,
            input_records=batch.record_count,
            imported_records=batch.record_count,
            rejected_records=0,
            ignored_records=0,
            issues=(),
            leads=batch.leads,
            supplied_batch=batch,
        )
    if not isinstance(document, list):
        raise DiscoveryLocalImportInputError(
            "JSON discovery imports must be a lead list or strict CanISend Lead Batch."
        )
    name, identifier = _resolved_local_source(source_name, source_id)
    if len(document) > _MAX_RECORDS:
        raise DiscoveryLocalImportInputError(
            "Discovery import exceeds the supported record limit."
        )
    leads: list[JobLeadV2] = []
    issues: list[DiscoveryImportIssueV1] = []
    rejected = ignored = 0
    for record_number, record in enumerate(document, start=1):
        if not isinstance(record, Mapping):
            rejected += 1
            issues.append(_issue(record_number, "import.row_not_object"))
            continue
        if not record:
            ignored += 1
            continue
        try:
            if record.get("schema_version") == "2.0.0":
                lead = JobLeadV2.model_validate(record)
                leads.append(
                    _private_safe_import_lead(
                        _attach_import_receipt(
                            lead,
                            source_name=name,
                            adapter="local.json",
                            locator="local-json",
                            observed_at=observed_at,
                        )
                    )
                )
                continue
            if set(record) - _LEGACY_JSON_FIELDS:
                rejected += 1
                issues.append(_issue(record_number, "import.row_unknown_field"))
                continue
            payload = dict(record)
            payload.update(
                {
                    "source": name,
                    "source_feed": "local-json",
                    "source_type": "json",
                }
            )
            leads.append(
                _private_safe_import_lead(
                    normalize_job_lead(
                        payload,
                        fetched_at=observed_at,
                        source_type="json",
                        adapter="local.json",
                    )
                )
            )
        except (LeadNormalizationError, ValidationError, ValueError):
            rejected += 1
            issues.append(_issue(record_number, "import.row_invalid"))
    return _ParsedImport(
        format="json",
        source_id=identifier,
        source_name=name,
        adapter="local.json",
        locator="local-json",
        input_records=len(document),
        imported_records=len(leads),
        rejected_records=rejected,
        ignored_records=ignored,
        issues=tuple(issues),
        leads=tuple(leads),
    )


def _parse_email_export(
    raw: bytes,
    *,
    format: Literal["eml", "mbox"],
    source_name: str,
    source_id: str,
    observed_at: datetime,
) -> _ParsedImport:
    messages = _email_messages(raw, format=format)
    leads: list[JobLeadV2] = []
    issues: list[DiscoveryImportIssueV1] = []
    seen_urls: set[str] = set()
    input_records = rejected = ignored = 0
    for message in messages:
        for raw_url, visible_text in _message_links(message):
            input_records += 1
            if input_records > _MAX_RECORDS:
                raise DiscoveryLocalImportInputError(
                    "Discovery import exceeds the supported record limit."
                )
            if urlsplit(raw_url.strip()).scheme.casefold() not in {"http", "https"}:
                ignored += 1
                continue
            signal_text = f"{visible_text} {unquote(raw_url)}"
            if _NON_JOB_LINK_RE.search(signal_text) or not _JOB_SIGNAL_RE.search(
                signal_text
            ):
                ignored += 1
                continue
            try:
                canonical_url = canonicalize_job_url(_trim_email_url(raw_url))
                if not _is_public_email_link(canonical_url):
                    raise LeadNormalizationError("email link is not public")
            except (LeadNormalizationError, ValueError):
                rejected += 1
                issues.append(
                    _issue(input_records, "import.link_invalid", field="link")
                )
                continue
            if canonical_url in seen_urls:
                ignored += 1
                continue
            seen_urls.add(canonical_url)
            title = _email_link_title(visible_text, canonical_url)
            try:
                leads.append(
                    _private_safe_import_lead(
                        normalize_job_lead(
                            {
                                "title": title,
                                "source_url": canonical_url,
                                "description": "",
                                "published_at": "",
                                "source": source_name,
                                "source_feed": "email-alert",
                                "source_type": "email_alert",
                            },
                            fetched_at=observed_at,
                            source_type="email_alert",
                            adapter="local.email_alert",
                        )
                    )
                )
            except (LeadNormalizationError, ValidationError, ValueError):
                rejected += 1
                issues.append(
                    _issue(input_records, "import.link_invalid", field="link")
                )
    return _ParsedImport(
        format=format,
        source_id=source_id,
        source_name=source_name,
        adapter="local.email_alert",
        locator="email-alert",
        input_records=input_records,
        imported_records=len(leads),
        rejected_records=rejected,
        ignored_records=ignored,
        issues=tuple(issues),
        leads=tuple(leads),
    )


def _build_local_batch(
    parsed: _ParsedImport,
    *,
    imported_at: datetime,
    content_sha256: str,
) -> LeadBatchV1:
    leads = tuple(sorted(parsed.leads, key=lead_batch_sort_key))
    return LeadBatchV1(
        batch_id=batch_identifier(
            source_id=parsed.source_id,
            content_sha256=content_sha256,
        ),
        source_id=parsed.source_id,
        source_name=parsed.source_name,
        adapter=parsed.adapter,
        source_url=parsed.locator,
        fetched_at=imported_at,
        content_sha256=content_sha256,
        record_count=len(leads),
        leads=leads,
    )


def _attach_import_receipt(
    lead: JobLeadV2,
    *,
    source_name: str,
    adapter: str,
    locator: str,
    observed_at: datetime,
) -> JobLeadV2:
    if lead.last_seen_at > observed_at:
        raise DiscoveryLocalImportInputError(
            "Imported Lead v2 records cannot be observed in the future."
        )
    receipt = LeadProvenanceV1(
        source=source_name,
        source_type="json",
        adapter=adapter,
        source_feed=locator,
        fetched_at=observed_at,
    )
    provenance = {*lead.provenance, receipt}
    ordered = tuple(sorted(provenance, key=_provenance_key))
    return JobLeadV2.model_validate(
        {
            **lead.model_dump(mode="json"),
            "fetched_at": observed_at,
            "last_seen_at": observed_at,
            "provenance": [item.model_dump(mode="json") for item in ordered],
            "match_reasons": [],
            "score": 0,
            "rank": 0,
        }
    )


def _email_messages(
    raw: bytes,
    *,
    format: Literal["eml", "mbox"],
) -> tuple[Message, ...]:
    parser = BytesParser(policy=email_policy.default)
    if format == "eml":
        try:
            return (parser.parsebytes(raw),)
        except Exception as exc:
            raise DiscoveryLocalImportInputError(
                "EML discovery import could not be parsed."
            ) from exc
    messages: list[Message] = []
    try:
        for position, message_bytes in enumerate(_mbox_messages(raw), start=1):
            if position > _MAX_MESSAGES:
                raise DiscoveryLocalImportInputError(
                    "MBOX discovery import exceeds the supported message limit."
                )
            messages.append(parser.parsebytes(message_bytes))
        if raw and not messages:
            raise DiscoveryLocalImportInputError(
                "MBOX discovery import does not contain a message envelope."
            )
    except DiscoveryLocalImportInputError:
        raise
    except Exception as exc:
        raise DiscoveryLocalImportInputError(
            "MBOX discovery import could not be parsed."
        ) from exc
    return tuple(messages)


def _mbox_messages(raw: bytes) -> tuple[bytes, ...]:
    messages: list[bytes] = []
    current: list[bytes] | None = None
    for line in raw.splitlines(keepends=True):
        if line.startswith(b"From "):
            if current is not None and current:
                messages.append(b"".join(current))
            current = []
            continue
        if current is not None:
            current.append(line)
    if current is not None and current:
        messages.append(b"".join(current))
    return tuple(messages)


def _message_links(message: Message) -> tuple[tuple[str, str], ...]:
    links: list[tuple[str, str]] = []
    for part in message.walk():
        if part.is_multipart() or part.get_content_disposition() == "attachment":
            continue
        content_type = part.get_content_type().casefold()
        if content_type not in {"text/html", "text/plain"}:
            continue
        try:
            content = part.get_content()
        except Exception:
            continue
        if not isinstance(content, str):
            continue
        if content_type == "text/html":
            collector = _AnchorCollector()
            try:
                collector.feed(content)
                collector.close()
            except Exception:
                continue
            links.extend(collector.links)
            continue
        for line in content.splitlines():
            for match in _PLAIN_URL_RE.finditer(line):
                visible = f"{line[:match.start()]} {line[match.end():]}"
                links.append((match.group(0), visible))
    return tuple(links)


def _mapped_csv_headers(headers: list[str]) -> dict[str, str]:
    mapped: dict[str, str] = {}
    owners: dict[str, str] = {}
    for header in headers:
        if header is None:
            raise DiscoveryLocalImportInputError(
                "CSV discovery import headers are invalid."
            )
        normalized = _normalized_header(header)
        canonical = _CSV_ALIAS_TO_FIELD.get(normalized)
        if canonical is None:
            continue
        if canonical in owners:
            raise DiscoveryLocalImportInputError(
                "CSV discovery import contains ambiguous field aliases."
            )
        owners[canonical] = header
        mapped[header] = canonical
    return mapped


def _existing_catalog_leads(path: Path) -> list[JobLeadV2]:
    if not path.exists():
        return []
    if not path.is_file() or path.is_symlink():
        raise DiscoveryInputError("Existing discovery catalog is unavailable.")
    return load_lead_document(path)


def _load_compatible_previous_batch(
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


def _write_import_report(path: Path, report: DiscoveryImportReportV1) -> Path:
    try:
        return atomic_write_json(path, report.model_dump(mode="json"))
    except (DiscoveryStoreError, TypeError, ValueError) as exc:
        raise DiscoveryLocalImportWriteError(
            "Discovery import report could not be written atomically."
        ) from exc


def _read_input_bytes(path: Path) -> bytes:
    try:
        target = Path(path)
        if not target.is_file():
            raise DiscoveryLocalImportInputError(
                "Discovery import input must be a readable file."
            )
        size = target.stat().st_size
        if size > _MAX_INPUT_BYTES:
            raise DiscoveryLocalImportInputError(
                "Discovery import input exceeds the supported size limit."
            )
        return target.read_bytes()
    except DiscoveryLocalImportInputError:
        raise
    except OSError as exc:
        raise DiscoveryLocalImportInputError(
            "Discovery import input could not be read."
        ) from exc


def _input_format(path: Path) -> ImportFormat:
    suffix = path.suffix.casefold()
    mapping: dict[str, ImportFormat] = {
        ".csv": "csv",
        ".json": "json",
        ".eml": "eml",
        ".mbox": "mbox",
    }
    if suffix not in mapping:
        raise DiscoveryLocalImportInputError(
            "Discovery import input must use .csv, .json, .eml, or .mbox."
        )
    return mapping[suffix]


def _resolved_local_source(
    source_name: str | None,
    source_id: str | None,
) -> tuple[str, str]:
    if source_name is None:
        raise DiscoveryLocalImportInputError(
            "Local discovery imports require --source-name."
        )
    name = _safe_source_name(source_name)
    identifier = (
        _safe_source_id(source_id)
        if source_id is not None
        else f"import-{sha256(name.casefold().encode('utf-8')).hexdigest()[:16]}"
    )
    return name, identifier


def _safe_source_name(value: str) -> str:
    normalized = value.strip()
    if (
        not normalized
        or len(normalized) > 256
        or _CONTROL_RE.search(normalized)
        or _EMAIL_RE.search(normalized)
        or _INLINE_SECRET_RE.search(normalized)
        or normalized.startswith(("/", "\\", "file:"))
        or PureWindowsPath(normalized).is_absolute()
    ):
        raise DiscoveryLocalImportInputError(
            "Discovery import source name must be a private-safe label."
        )
    return normalized


def _safe_source_id(value: str) -> str:
    try:
        normalized = _SOURCE_ID_ADAPTER.validate_python(value.strip())
    except (ValidationError, ValueError) as exc:
        raise DiscoveryLocalImportInputError(
            "Discovery import source ID must be a safe lowercase identifier."
        ) from exc
    parts = {part for part in re.split(r"[^a-z0-9]+", normalized) if part}
    if parts & {
        "auth",
        "credential",
        "password",
        "secret",
        "session",
        "signature",
        "token",
    }:
        raise DiscoveryLocalImportInputError(
            "Discovery import source ID must not contain credential-like terms."
        )
    return normalized


def _private_safe_import_lead(lead: JobLeadV2) -> JobLeadV2:
    _safe_source_name(lead.source)
    _private_safe_locator(lead.source_feed)
    for item in lead.provenance:
        _safe_source_name(item.source)
        _private_safe_locator(item.source_feed)
    for locator in (
        lead.canonical_url,
        lead.source_url,
        *(item.source_url for item in lead.provenance),
    ):
        decoded = unquote(locator)
        if _EMAIL_RE.search(decoded) or _INLINE_SECRET_RE.search(decoded):
            raise DiscoveryLocalImportInputError(
                "Imported job URLs must not contain email or credential-like path data."
            )
    return lead


def _private_safe_locator(value: str) -> None:
    decoded = unquote(value)
    if (
        _EMAIL_RE.search(decoded)
        or _INLINE_SECRET_RE.search(decoded)
        or decoded.startswith(("/", "\\", "file:"))
        or PureWindowsPath(decoded).is_absolute()
    ):
        raise DiscoveryLocalImportInputError(
            "Imported source locators must not contain private path or credential data."
        )


def _is_public_email_link(value: str) -> bool:
    decoded = unquote(value)
    if _EMAIL_RE.search(decoded) or _INLINE_SECRET_RE.search(decoded):
        return False
    parsed = urlsplit(value)
    hostname = (parsed.hostname or "").rstrip(".").casefold()
    if (
        not hostname
        or hostname in {"localhost", "localhost.localdomain"}
        or hostname.endswith((".localhost", ".local", ".internal"))
    ):
        return False
    try:
        address = ipaddress.ip_address(hostname)
    except ValueError:
        return "." in hostname
    return address.is_global


def _email_link_title(visible_text: str, canonical_url: str) -> str:
    cleaned = _SPACE_RE.sub(" ", _EMAIL_RE.sub("", visible_text)).strip(" -:|\t")
    cleaned = _CONTROL_RE.sub("", cleaned)
    if cleaned and _JOB_SIGNAL_RE.search(cleaned):
        return cleaned[:2_048]
    path = unquote(urlsplit(canonical_url).path)
    slug = path.rstrip("/").rsplit("/", 1)[-1]
    derived = _SPACE_RE.sub(" ", re.sub(r"[-_]+", " ", slug)).strip()
    return (derived or "Job opportunity")[:2_048]


def _trim_email_url(value: str) -> str:
    return value.strip().rstrip(".,;:!?)]}")


def _normalized_header(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", value.casefold()).strip("_")


def _issue(
    record_number: int,
    code: str,
    *,
    field: str = "record",
) -> DiscoveryImportIssueV1:
    return DiscoveryImportIssueV1(
        record_number=record_number,
        code=code,
        field=field,
    )


def _issue_key(issue: DiscoveryImportIssueV1) -> tuple[int, str, str]:
    return (issue.record_number, issue.code, issue.field)


def _provenance_key(item: LeadProvenanceV1) -> tuple[str, ...]:
    return (
        item.source.casefold(),
        item.source_type,
        item.adapter,
        item.source_record_id,
        item.source_url,
        item.source_feed,
        item.fetched_at.isoformat(),
    )


def _safe_discovery_root(workspace: Path, lead_root: Path) -> Path:
    candidate = Path(lead_root).expanduser()
    if not candidate.is_absolute():
        candidate = workspace / candidate
    resolved = candidate.resolve()
    try:
        resolved.relative_to(workspace)
    except ValueError as exc:
        raise DiscoveryLocalImportInputError(
            "Discovery output directory must remain inside the workspace."
        ) from exc
    if resolved == workspace:
        raise DiscoveryLocalImportInputError(
            "Discovery output directory must be a workspace subdirectory."
        )
    return resolved


def _safe_artifact_directory(workspace: Path, path: Path) -> Path:
    candidate = Path(path)
    if candidate.is_symlink():
        raise DiscoveryLocalImportInputError(
            "Discovery artifact directories must not be symbolic links."
        )
    if candidate.exists() and not candidate.is_dir():
        raise DiscoveryLocalImportInputError(
            "Discovery artifact directory must be a directory."
        )
    resolved = candidate.resolve()
    try:
        resolved.relative_to(workspace)
    except ValueError as exc:
        raise DiscoveryLocalImportInputError(
            "Discovery artifact directory escaped the workspace."
        ) from exc
    return resolved


def _workspace_relative_path(workspace: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(workspace).as_posix()
    except ValueError as exc:
        raise DiscoveryLocalImportWriteError(
            "Discovery artifact path escaped the workspace."
        ) from exc


def _read_clock(clock) -> datetime:
    try:
        value = clock()
    except Exception as exc:
        raise DiscoveryLocalImportInputError(
            "Discovery import clock could not be read."
        ) from exc
    if (
        not isinstance(value, datetime)
        or value.tzinfo is None
        or value.utcoffset() is None
    ):
        raise DiscoveryLocalImportInputError(
            "Discovery import clock must be timezone-aware."
        )
    return value.astimezone(UTC)
