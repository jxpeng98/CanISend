from __future__ import annotations

from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from datetime import UTC, datetime
from email.utils import parsedate_to_datetime
from hashlib import sha256
import ipaddress
import math
import re
import time
from typing import Any
from urllib.error import HTTPError
from urllib.parse import urlsplit, urlunsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener

from canisend.network_safety import (
    AddressResolver,
    ResolvedAddressError,
    require_public_resolved_addresses,
    resolve_host_addresses,
)


_CONTROL_RE = re.compile(r"[\x00-\x1f\x7f]")
_HOST_RE = re.compile(r"^[A-Za-z0-9.-]+$")
_RETRYABLE_STATUS_CODES = frozenset({408, 425, 429, 500, 502, 503, 504})

TransportOpener = Callable[..., Any]
Sleep = Callable[[float], None]
MonotonicClock = Callable[[], float]
WallClock = Callable[[], datetime]


class PublicTransportError(ValueError):
    """A stable body-free transport failure suitable for refresh reports."""

    def __init__(
        self,
        code: str,
        message: str,
        *,
        retryable: bool = False,
        attempts: int = 0,
        status_code: int = 0,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.retryable = retryable
        self.attempts = attempts
        self.status_code = status_code


@dataclass(frozen=True)
class TransportPolicy:
    allowed_media_types: tuple[str, ...]
    media_subject: str
    media_description: str
    size_subject: str
    user_agent: str
    accept: str
    timeout_seconds: int = 30
    max_bytes: int = 2_000_000
    max_attempts: int = 1
    backoff_seconds: float = 1.0
    max_retry_delay_seconds: float = 300.0
    min_interval_seconds: float = 0.0
    allow_application_xml_suffix: bool = False
    retry_status_codes: frozenset[int] = field(
        default_factory=lambda: _RETRYABLE_STATUS_CODES
    )

    def __post_init__(self) -> None:
        if not self.allowed_media_types:
            raise ValueError("transport policy requires at least one media type")
        if any(
            not value or value != value.strip().lower()
            for value in self.allowed_media_types
        ):
            raise ValueError("transport media types must be normalized lowercase values")
        if not (1 <= self.timeout_seconds <= 300):
            raise ValueError("transport timeout must be between 1 and 300 seconds")
        if not (1 <= self.max_bytes <= 100_000_000):
            raise ValueError("transport response limit is outside the supported range")
        if not (1 <= self.max_attempts <= 5):
            raise ValueError("transport attempts must be between 1 and 5")
        if not (0 <= self.backoff_seconds <= 300):
            raise ValueError("transport backoff is outside the supported range")
        if not (0 <= self.max_retry_delay_seconds <= 3_600):
            raise ValueError("transport retry delay is outside the supported range")
        if not (0 <= self.min_interval_seconds <= 300):
            raise ValueError("transport host interval is outside the supported range")
        for value in (
            self.media_subject,
            self.media_description,
            self.size_subject,
            self.user_agent,
            self.accept,
        ):
            if not value or _CONTROL_RE.search(value):
                raise ValueError("transport policy text must be non-empty control-free text")


@dataclass(frozen=True)
class TransportResponse:
    status_code: int
    body: bytes
    content_type: str
    media_type: str
    final_url: str
    etag: str
    last_modified: str
    content_sha256: str
    attempts: int
    not_modified: bool = False


class _PublicRedirectHandler(HTTPRedirectHandler):
    def __init__(self, resolver: AddressResolver) -> None:
        super().__init__()
        self._resolver = resolver

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: ANN001
        validate_public_http_url(str(newurl), resolver=self._resolver)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


class PublicTransport:
    """Bounded public GET transport with shared host throttling and safe retries."""

    def __init__(
        self,
        *,
        opener: TransportOpener | None = None,
        resolver: AddressResolver | None = None,
        sleep: Sleep = time.sleep,
        monotonic: MonotonicClock = time.monotonic,
        now: WallClock | None = None,
    ) -> None:
        self._resolver = resolver or resolve_host_addresses
        self._opener = opener
        self._sleep = sleep
        self._monotonic = monotonic
        self._now = now or (lambda: datetime.now(UTC))
        self._last_request_at: dict[str, float] = {}

    def fetch(
        self,
        url: str,
        *,
        policy: TransportPolicy,
        etag: str = "",
        last_modified: str = "",
    ) -> TransportResponse:
        validated_url = validate_public_http_url(url, resolver=self._resolver)
        hostname = _hostname(validated_url)
        conditional_headers = _conditional_headers(etag, last_modified)
        headers = {
            "User-Agent": policy.user_agent,
            "Accept": policy.accept,
            **conditional_headers,
        }
        active_opener = self._opener
        if active_opener is None:
            active_opener = build_opener(
                _PublicRedirectHandler(self._resolver)
            ).open

        for attempt in range(1, policy.max_attempts + 1):
            self._throttle(hostname, policy.min_interval_seconds)
            request = Request(validated_url, headers=headers, method="GET")
            try:
                response = active_opener(request, timeout=policy.timeout_seconds)
            except PublicTransportError as exc:
                if exc.attempts == 0:
                    exc.attempts = attempt
                raise
            except HTTPError as exc:
                result = self._handle_http_error(
                    exc,
                    request_url=validated_url,
                    attempt=attempt,
                    policy=policy,
                )
                if result is not None:
                    return result
                continue
            except Exception as exc:
                if attempt < policy.max_attempts:
                    self._wait_for_retry({}, attempt=attempt, policy=policy)
                    continue
                raise PublicTransportError(
                    "transport.network_failed",
                    "The public resource request failed before a valid response was received.",
                    retryable=True,
                    attempts=attempt,
                ) from exc

            retry = False
            try:
                with response:
                    final_url = validate_public_http_url(
                        str(_response_url(response) or request.full_url),
                        resolver=self._resolver,
                    )
                    status_code = _response_status(response)
                    response_headers = _response_headers(response)
                    if status_code == 304:
                        return _not_modified_response(
                            response_headers,
                            final_url=final_url,
                            attempts=attempt,
                        )
                    if (
                        status_code in policy.retry_status_codes
                        and attempt < policy.max_attempts
                    ):
                        retry = True
                    elif status_code < 200 or status_code >= 300:
                        raise PublicTransportError(
                            "transport.http_status",
                            "The public resource returned a non-success HTTP status.",
                            retryable=status_code in policy.retry_status_codes,
                            attempts=attempt,
                            status_code=status_code,
                        )
                    else:
                        content_type = _safe_header(response_headers, "Content-Type")
                        media_type = content_type.partition(";")[0].strip().lower()
                        if not _media_type_allowed(media_type, policy):
                            raise PublicTransportError(
                                "transport.media_type",
                                f"{policy.media_subject} did not return "
                                f"{policy.media_description}.",
                                attempts=attempt,
                                status_code=status_code,
                            )
                        body = _read_limited_response(
                            response,
                            headers=response_headers,
                            max_bytes=policy.max_bytes,
                            size_subject=policy.size_subject,
                            attempts=attempt,
                            status_code=status_code,
                        )
                        return TransportResponse(
                            status_code=status_code,
                            body=body,
                            content_type=content_type,
                            media_type=media_type,
                            final_url=final_url,
                            etag=_safe_header(response_headers, "ETag"),
                            last_modified=_safe_header(
                                response_headers, "Last-Modified"
                            ),
                            content_sha256=sha256(body).hexdigest(),
                            attempts=attempt,
                        )
            except PublicTransportError:
                raise
            except Exception as exc:
                if attempt < policy.max_attempts:
                    self._wait_for_retry({}, attempt=attempt, policy=policy)
                    continue
                raise PublicTransportError(
                    "transport.network_failed",
                    "The public resource response could not be read safely.",
                    retryable=True,
                    attempts=attempt,
                ) from exc
            if retry:
                self._wait_for_retry(
                    response_headers,
                    attempt=attempt,
                    policy=policy,
                )

        raise PublicTransportError(
            "transport.network_failed",
            "The public resource request did not complete.",
            retryable=True,
            attempts=policy.max_attempts,
        )

    def _handle_http_error(
        self,
        error: HTTPError,
        *,
        request_url: str,
        attempt: int,
        policy: TransportPolicy,
    ) -> TransportResponse | None:
        final_url = validate_public_http_url(
            str(error.geturl() or request_url),
            resolver=self._resolver,
        )
        status_code = int(error.code)
        headers = _headers_mapping(error.headers)
        if status_code == 304:
            return _not_modified_response(
                headers,
                final_url=final_url,
                attempts=attempt,
            )
        if status_code in policy.retry_status_codes and attempt < policy.max_attempts:
            self._wait_for_retry(headers, attempt=attempt, policy=policy)
            return None
        raise PublicTransportError(
            "transport.http_status",
            "The public resource returned a non-success HTTP status.",
            retryable=status_code in policy.retry_status_codes,
            attempts=attempt,
            status_code=status_code,
        ) from error

    def _throttle(self, hostname: str, minimum_interval: float) -> None:
        now = self._monotonic_time()
        previous = self._last_request_at.get(hostname)
        if previous is not None and minimum_interval > 0:
            delay = minimum_interval - (now - previous)
            if delay > 0:
                self._sleep_for(
                    delay,
                    code="transport.throttle_wait_failed",
                    attempts=0,
                )
                now = self._monotonic_time()
        self._last_request_at[hostname] = now

    def _wait_for_retry(
        self,
        headers: Mapping[str, str],
        *,
        attempt: int,
        policy: TransportPolicy,
    ) -> None:
        try:
            current = self._now()
        except Exception as exc:
            raise PublicTransportError(
                "transport.clock_failed",
                "The transport wall clock could not be read.",
                attempts=attempt,
            ) from exc
        if not isinstance(current, datetime):
            raise PublicTransportError(
                "transport.clock_failed",
                "The transport wall clock returned an invalid value.",
                attempts=attempt,
            )
        delay = _retry_delay(
            headers,
            attempt=attempt,
            backoff_seconds=policy.backoff_seconds,
            maximum=policy.max_retry_delay_seconds,
            now=current,
        )
        if delay > 0:
            self._sleep_for(
                delay,
                code="transport.retry_wait_failed",
                attempts=attempt,
            )

    def _monotonic_time(self) -> float:
        try:
            value = float(self._monotonic())
        except Exception as exc:
            raise PublicTransportError(
                "transport.clock_failed",
                "The transport monotonic clock could not be read.",
            ) from exc
        if not math.isfinite(value):
            raise PublicTransportError(
                "transport.clock_failed",
                "The transport monotonic clock returned an invalid value.",
            )
        return value

    def _sleep_for(self, delay: float, *, code: str, attempts: int) -> None:
        try:
            self._sleep(delay)
        except Exception as exc:
            raise PublicTransportError(
                code,
                "The transport wait operation failed.",
                retryable=True,
                attempts=attempts,
            ) from exc


def validate_public_http_url(
    value: str,
    *,
    resolver: AddressResolver | None = None,
    label: str = "URL",
) -> str:
    normalized, hostname = _normalize_http_url(value, label=label)
    try:
        require_public_resolved_addresses(
            hostname,
            resolver=resolver or resolve_host_addresses,
        )
    except ResolvedAddressError as exc:
        raise PublicTransportError("transport.url_not_public", str(exc)) from exc
    return normalized


def redact_public_url(value: str) -> str:
    normalized, _ = _normalize_http_url(value, label="Source URL")
    from canisend.discovery.identity import redact_feed_url

    return redact_feed_url(normalized)


def _normalize_http_url(value: str, *, label: str) -> tuple[str, str]:
    raw = value.strip()
    if not raw:
        raise PublicTransportError("transport.url_empty", f"{label} is required.")
    if len(raw) > 8_192 or _CONTROL_RE.search(raw):
        raise PublicTransportError(
            "transport.url_invalid", f"{label} contains unsafe characters."
        )
    try:
        parsed = urlsplit(raw)
    except ValueError as exc:
        raise PublicTransportError(
            "transport.url_invalid", f"{label} is invalid."
        ) from exc
    if parsed.scheme.lower() not in {"http", "https"}:
        raise PublicTransportError(
            "transport.url_scheme",
            "Only http:// and https:// URLs can be fetched.",
        )
    if not parsed.hostname:
        raise PublicTransportError(
            "transport.url_host", f"{label} must include a host."
        )
    if parsed.username is not None or parsed.password is not None:
        raise PublicTransportError(
            "transport.url_credentials", f"{label} must not include credentials."
        )
    try:
        port = parsed.port
    except ValueError as exc:
        raise PublicTransportError(
            "transport.url_port", f"{label} port is invalid."
        ) from exc
    try:
        hostname = parsed.hostname.rstrip(".").encode("idna").decode("ascii").lower()
    except UnicodeError as exc:
        raise PublicTransportError(
            "transport.url_host", f"{label} host contains unsafe characters."
        ) from exc
    try:
        ipaddress.ip_address(hostname)
    except ValueError:
        if not _HOST_RE.fullmatch(hostname):
            raise PublicTransportError(
                "transport.url_host", f"{label} host contains unsafe characters."
            )
    if (
        hostname in {
            "localhost",
            "localhost.localdomain",
            "ip6-localhost",
            "ip6-loopback",
        }
        or hostname.endswith(".localhost")
        or hostname.endswith(".local")
    ):
        raise PublicTransportError(
            "transport.url_localhost", f"{label} must not target localhost."
        )
    host_literal = f"[{hostname}]" if ":" in hostname else hostname
    netloc = host_literal if port is None else f"{host_literal}:{port}"
    return (
        urlunsplit(
            (
                parsed.scheme.lower(),
                netloc,
                parsed.path,
                parsed.query,
                "",
            )
        ),
        hostname,
    )


def _conditional_headers(etag: str, last_modified: str) -> dict[str, str]:
    headers: dict[str, str] = {}
    if etag:
        headers["If-None-Match"] = _validated_request_header(etag, "ETag")
    if last_modified:
        headers["If-Modified-Since"] = _validated_request_header(
            last_modified, "Last-Modified"
        )
    return headers


def _validated_request_header(value: str, label: str) -> str:
    normalized = value.strip()
    if not normalized or len(normalized) > 1_024 or _CONTROL_RE.search(normalized):
        raise PublicTransportError(
            "transport.validator_invalid",
            f"Cached {label} validator is invalid.",
        )
    return normalized


def _response_url(response: Any) -> str:
    if hasattr(response, "geturl"):
        return str(response.geturl() or "")
    return ""


def _response_status(response: Any) -> int:
    value = getattr(response, "status", None)
    if value is None and hasattr(response, "getcode"):
        value = response.getcode()
    if value is None:
        return 200
    try:
        return int(value)
    except (TypeError, ValueError) as exc:
        raise PublicTransportError(
            "transport.status_invalid",
            "The public resource returned an invalid HTTP status.",
        ) from exc


def _response_headers(response: Any) -> Mapping[str, str]:
    return _headers_mapping(getattr(response, "headers", {}))


def _headers_mapping(headers: Any) -> Mapping[str, str]:
    if headers is None:
        return {}
    if isinstance(headers, Mapping):
        return {str(key): str(value) for key, value in headers.items()}
    if hasattr(headers, "items"):
        return {str(key): str(value) for key, value in headers.items()}
    return {}


def _safe_header(headers: Mapping[str, str], name: str) -> str:
    value = next(
        (str(value) for key, value in headers.items() if str(key).casefold() == name.casefold()),
        "",
    ).strip()
    if len(value) > 4_096 or _CONTROL_RE.search(value):
        raise PublicTransportError(
            "transport.header_invalid",
            "The public resource returned an invalid response header.",
        )
    return value


def _media_type_allowed(media_type: str, policy: TransportPolicy) -> bool:
    if media_type in policy.allowed_media_types:
        return True
    return bool(
        policy.allow_application_xml_suffix
        and media_type.startswith("application/")
        and media_type.endswith("+xml")
    )


def _read_limited_response(
    response: Any,
    *,
    headers: Mapping[str, str],
    max_bytes: int,
    size_subject: str,
    attempts: int,
    status_code: int,
) -> bytes:
    content_length = _safe_header(headers, "Content-Length")
    if content_length:
        try:
            declared_length = int(content_length)
        except ValueError:
            declared_length = -1
        if declared_length > max_bytes:
            raise PublicTransportError(
                "transport.response_too_large",
                f"{size_subject} response exceeds the configured limit of "
                f"{max_bytes} bytes.",
                attempts=attempts,
                status_code=status_code,
            )
    body = response.read(max_bytes + 1)
    if not isinstance(body, bytes):
        raise PublicTransportError(
            "transport.body_invalid",
            "The public resource returned a non-binary response body.",
            attempts=attempts,
            status_code=status_code,
        )
    if len(body) > max_bytes:
        raise PublicTransportError(
            "transport.response_too_large",
            f"{size_subject} response exceeds the configured limit of "
            f"{max_bytes} bytes.",
            attempts=attempts,
            status_code=status_code,
        )
    return body


def _not_modified_response(
    headers: Mapping[str, str],
    *,
    final_url: str,
    attempts: int,
) -> TransportResponse:
    return TransportResponse(
        status_code=304,
        body=b"",
        content_type="",
        media_type="",
        final_url=final_url,
        etag=_safe_header(headers, "ETag"),
        last_modified=_safe_header(headers, "Last-Modified"),
        content_sha256="",
        attempts=attempts,
        not_modified=True,
    )


def _retry_delay(
    headers: Mapping[str, str],
    *,
    attempt: int,
    backoff_seconds: float,
    maximum: float,
    now: datetime,
) -> float:
    retry_after = _safe_header(headers, "Retry-After")
    parsed = _parse_retry_after(retry_after, now=now) if retry_after else None
    delay = parsed if parsed is not None else backoff_seconds * (2 ** (attempt - 1))
    return max(0.0, min(float(delay), maximum))


def _parse_retry_after(value: str, *, now: datetime) -> float | None:
    try:
        return max(0.0, float(int(value)))
    except ValueError:
        pass
    try:
        retry_at = parsedate_to_datetime(value)
    except (TypeError, ValueError, OverflowError):
        return None
    if retry_at.tzinfo is None or retry_at.utcoffset() is None:
        retry_at = retry_at.replace(tzinfo=UTC)
    current = now if now.tzinfo is not None else now.replace(tzinfo=UTC)
    return max(0.0, (retry_at.astimezone(UTC) - current.astimezone(UTC)).total_seconds())


def _hostname(url: str) -> str:
    hostname = urlsplit(url).hostname
    if not hostname:
        raise PublicTransportError(
            "transport.url_host", "URL must include a host."
        )
    return hostname.rstrip(".").lower()
