from __future__ import annotations

from dataclasses import asdict, is_dataclass
from datetime import UTC, datetime
from hashlib import sha256
import re
from typing import Any, Mapping
from urllib.parse import parse_qsl, quote, urlencode, urlsplit, urlunsplit

from canisend.discovery.models import (
    JobLeadV2,
    LeadIdentityMethod,
    LeadProvenanceV1,
    LeadSourceType,
)


_TRACKING_QUERY_NAMES = frozenset(
    {
        "fbclid",
        "gclid",
        "mc_cid",
        "mc_eid",
        "ref",
        "source",
    }
)
_SECRET_QUERY_NAMES = frozenset(
    {
        "access_token",
        "api-key",
        "api_key",
        "apikey",
        "auth",
        "authorization",
        "credential",
        "credentials",
        "id_token",
        "key",
        "password",
        "passwd",
        "secret",
        "session",
        "session_id",
        "sessionid",
        "sig",
        "signature",
        "token",
    }
)
_CONTROL_RE = re.compile(r"[\x00-\x1f\x7f]")
_WHITESPACE_RE = re.compile(r"\s+")
_EMAIL_RE = re.compile(r"(?i)(?<![A-Z0-9._%+-])[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}(?![A-Z0-9.-])")
_INLINE_SECRET_RE = re.compile(
    r"(?i)(?:access[_-]?token|api[_-]?key|auth|credential|password|secret|session|signature|token)\s*[:=]"
)
_WINDOWS_PATH_RE = re.compile(r"^[A-Za-z]:[\\/]")


class LeadNormalizationError(ValueError):
    pass


def canonicalize_job_url(value: str) -> str:
    """Return a deterministic, credential-free HTTP(S) URL or an empty string."""

    raw = value.strip()
    if not raw:
        return ""
    if _CONTROL_RE.search(raw):
        raise LeadNormalizationError("job URL must not contain control characters")
    try:
        parsed = urlsplit(raw)
    except ValueError as exc:
        raise LeadNormalizationError("job URL is invalid") from exc
    scheme = parsed.scheme.lower()
    if scheme not in {"http", "https"}:
        raise LeadNormalizationError("job URL must use http or https")
    if not parsed.hostname:
        raise LeadNormalizationError("job URL must include a host")
    if parsed.username is not None or parsed.password is not None:
        raise LeadNormalizationError("job URL must not include credentials")
    try:
        port = parsed.port
    except ValueError as exc:
        raise LeadNormalizationError("job URL port is invalid") from exc

    try:
        hostname = parsed.hostname.rstrip(".").encode("idna").decode("ascii").lower()
    except UnicodeError as exc:
        raise LeadNormalizationError("job URL host is invalid") from exc
    if not hostname:
        raise LeadNormalizationError("job URL must include a host")

    default_port = (scheme == "http" and port == 80) or (scheme == "https" and port == 443)
    host_literal = f"[{hostname}]" if ":" in hostname else hostname
    netloc = host_literal if port is None or default_port else f"{host_literal}:{port}"
    path = quote(parsed.path or "/", safe="/%:@!$&'()*+,;=-._~")

    kept_query: list[tuple[str, str]] = []
    for name, query_value in parse_qsl(parsed.query, keep_blank_values=True):
        normalized_name = name.casefold()
        if normalized_name.startswith("utm_") or normalized_name in _TRACKING_QUERY_NAMES:
            continue
        if _is_sensitive_query_name(normalized_name):
            continue
        kept_query.append((name, query_value))
    kept_query.sort(key=lambda item: (item[0].casefold(), item[0], item[1]))
    query = urlencode(kept_query, doseq=True)
    return urlunsplit((scheme, netloc, path, query, ""))


def redact_feed_url(value: str) -> str:
    raw = value.strip()
    if not raw:
        return ""
    if _CONTROL_RE.search(raw):
        raise LeadNormalizationError("source locator must not contain control characters")
    parsed_raw = urlsplit(raw)
    if not parsed_raw.scheme and not parsed_raw.netloc:
        if raw.startswith(("/", "\\", ".")) or "/" in raw or "\\" in raw:
            return "local-import"
        return raw
    canonical = canonicalize_job_url(raw)
    parsed = urlsplit(canonical)
    return urlunsplit(
        (
            parsed.scheme,
            parsed.netloc,
            parsed.path,
            "redacted" if parsed_raw.query else "",
            "",
        )
    )


def sanitize_source_record_id(value: str) -> str:
    record_id = _CONTROL_RE.sub("", value).strip()
    if not record_id:
        return ""
    if len(record_id) > 1_024:
        raise LeadNormalizationError("source record ID is too long")
    if (
        _EMAIL_RE.search(record_id)
        or _INLINE_SECRET_RE.search(record_id)
        or record_id.startswith(("/", "\\"))
        or _WINDOWS_PATH_RE.match(record_id)
    ):
        return f"opaque_{sha256(record_id.encode('utf-8')).hexdigest()[:32]}"
    parsed = urlsplit(record_id)
    if parsed.scheme.lower() in {"http", "https"} and parsed.hostname:
        canonical = canonicalize_job_url(record_id)
        safe = urlsplit(canonical)
        return urlunsplit((safe.scheme, safe.netloc, safe.path, "", ""))
    return record_id


def _is_sensitive_query_name(value: str) -> bool:
    if value in _SECRET_QUERY_NAMES:
        return True
    parts = {part for part in re.split(r"[^a-z0-9]+", value) if part}
    return bool(parts & {"auth", "credential", "password", "secret", "session", "signature", "token"})


def stable_lead_id(*, source: str, source_record_id: str, canonical_url: str, title: str,
                   institution: str, deadline: str) -> tuple[str, LeadIdentityMethod]:
    source_key = _normalized_text(source)
    safe_record_id = sanitize_source_record_id(source_record_id)
    if safe_record_id:
        identity_method: LeadIdentityMethod = "source_record_id"
        identity = f"source-record\n{source_key}\n{safe_record_id}"
    elif canonical_url:
        identity_method = "canonical_url"
        identity = f"canonical-url\n{canonical_url}"
    else:
        identity_method = "fingerprint"
        identity = "fingerprint\n" + "\n".join(
            (
                _normalized_text(title),
                _normalized_text(institution),
                _normalized_text(deadline),
            )
        )
    digest = sha256(identity.encode("utf-8")).hexdigest()[:32]
    return f"lead_{digest}", identity_method


def normalize_job_lead(
    lead: Mapping[str, Any] | object,
    *,
    fetched_at: datetime | str | None = None,
    source_type: LeadSourceType | None = None,
    adapter: str | None = None,
) -> JobLeadV2:
    """Normalize a legacy/dataclass/dict lead into the strict additive v2 contract."""

    payload = _lead_mapping(lead)
    if payload.get("schema_version") == "2.0.0":
        return JobLeadV2.model_validate(payload)

    source = _clean_text(payload.get("source")) or "unknown"
    raw_source_url = _clean_text(payload.get("source_url"))
    canonical_url = canonicalize_job_url(raw_source_url) if raw_source_url else ""
    source_feed = _clean_text(payload.get("source_feed"))
    safe_source_feed = redact_feed_url(source_feed) if source_feed else ""
    source_record_id = sanitize_source_record_id(_clean_text(payload.get("source_record_id")))
    title = _clean_text(payload.get("title"))
    institution = _clean_text(payload.get("institution"))
    deadline = _clean_text(payload.get("deadline"))
    lead_id, identity_method = stable_lead_id(
        source=source,
        source_record_id=source_record_id,
        canonical_url=canonical_url,
        title=title,
        institution=institution,
        deadline=deadline,
    )

    observed_at = _aware_utc_datetime(fetched_at)
    resolved_source_type = source_type or _source_type(payload.get("source_type"))
    resolved_adapter = adapter or _default_adapter(resolved_source_type)
    provenance = LeadProvenanceV1(
        source=source,
        source_type=resolved_source_type,
        adapter=resolved_adapter,
        source_record_id=source_record_id,
        source_url=canonical_url,
        source_feed=safe_source_feed,
        fetched_at=observed_at,
    )
    return JobLeadV2(
        lead_id=lead_id,
        identity_method=identity_method,
        title=title,
        source_url=canonical_url,
        description=_clean_text(payload.get("description")),
        published_at=_clean_text(payload.get("published_at")),
        source=source,
        source_feed=safe_source_feed,
        source_record_id=source_record_id,
        canonical_url=canonical_url,
        institution=institution,
        location=_clean_text(payload.get("location")),
        deadline=deadline,
        fetched_at=observed_at,
        first_seen_at=observed_at,
        last_seen_at=observed_at,
        provenance=(provenance,),
    )


def _lead_mapping(lead: Mapping[str, Any] | object) -> Mapping[str, Any]:
    if isinstance(lead, Mapping):
        return lead
    if is_dataclass(lead) and not isinstance(lead, type):
        return asdict(lead)
    raise LeadNormalizationError("lead must be a mapping or dataclass instance")


def _clean_text(value: object) -> str:
    if value is None:
        return ""
    if not isinstance(value, str):
        raise LeadNormalizationError("lead text fields must be strings")
    return _CONTROL_RE.sub("", value).strip()


def _normalized_text(value: str) -> str:
    return _WHITESPACE_RE.sub(" ", value.casefold()).strip()


def _aware_utc_datetime(value: datetime | str | None) -> datetime:
    if value is None:
        return datetime.now(UTC).replace(microsecond=0)
    if isinstance(value, str):
        normalized = value.strip().replace("Z", "+00:00").replace("z", "+00:00")
        try:
            value = datetime.fromisoformat(normalized)
        except ValueError as exc:
            raise LeadNormalizationError("fetched_at must be an ISO 8601 date-time") from exc
    if not isinstance(value, datetime) or value.tzinfo is None or value.utcoffset() is None:
        raise LeadNormalizationError("fetched_at must include a timezone")
    return value.astimezone(UTC)


def _source_type(value: object) -> LeadSourceType:
    if isinstance(value, str) and value in {
        "rss",
        "atom",
        "public_api",
        "csv",
        "json",
        "email_alert",
        "host_agent",
        "legacy",
    }:
        return value  # type: ignore[return-value]
    return "legacy"


def _default_adapter(source_type: LeadSourceType) -> str:
    return {
        "rss": "feed.rss",
        "atom": "feed.atom",
        "public_api": "public_api.unknown",
        "csv": "local.csv",
        "json": "local.json",
        "email_alert": "local.email_alert",
        "host_agent": "host_agent.search",
        "legacy": "legacy.lead",
    }[source_type]
