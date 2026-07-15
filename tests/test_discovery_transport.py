from __future__ import annotations

from datetime import UTC, datetime
from email.message import Message
from urllib.error import HTTPError

import pytest

from canisend.discovery.transport import (
    PublicTransport,
    PublicTransportError,
    TransportPolicy,
    redact_public_url,
    validate_public_http_url,
)


PUBLIC_ADDRESS = ("93.184.216.34",)


class FakeResponse:
    def __init__(
        self,
        body: bytes = b"<rss version='2.0'><channel /></rss>",
        *,
        status: int = 200,
        content_type: str = "application/rss+xml",
        final_url: str = "https://example.edu/feed.xml",
        headers: dict[str, str] | None = None,
    ) -> None:
        self.body = body
        self.status = status
        self.final_url = final_url
        self.headers = {"Content-Type": content_type, **(headers or {})}
        self.read_sizes: list[int] = []

    def __enter__(self) -> FakeResponse:
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        return None

    def geturl(self) -> str:
        return self.final_url

    def read(self, size: int = -1) -> bytes:
        self.read_sizes.append(size)
        return self.body if size < 0 else self.body[:size]


def _policy(**overrides) -> TransportPolicy:
    values = {
        "allowed_media_types": ("application/rss+xml",),
        "media_subject": "Discovery response",
        "media_description": "RSS content",
        "size_subject": "Discovery response",
        "user_agent": "CanISend/Test",
        "accept": "application/rss+xml",
        "max_bytes": 1_024,
    }
    values.update(overrides)
    return TransportPolicy(**values)


def test_public_url_validation_and_redaction_are_credential_free() -> None:
    validated = validate_public_http_url(
        "HTTPS://Example.EDU:443/jobs.xml?query=economics#fragment",
        resolver=lambda hostname: PUBLIC_ADDRESS,
    )

    assert validated == "https://example.edu:443/jobs.xml?query=economics"
    assert (
        redact_public_url(validated)
        == "https://example.edu/jobs.xml?redacted"
    )
    with pytest.raises(PublicTransportError, match="credentials"):
        validate_public_http_url(
            "https://user:secret@example.edu/jobs.xml",
            resolver=lambda hostname: PUBLIC_ADDRESS,
        )
    with pytest.raises(PublicTransportError, match="publicly routable"):
        validate_public_http_url(
            "https://example.edu/jobs.xml",
            resolver=lambda hostname: ("10.0.0.8",),
        )


def test_transport_sends_conditional_headers_and_handles_response_304() -> None:
    response = FakeResponse(
        status=304,
        headers={"ETag": '"v2"', "Last-Modified": "Wed, 15 Jul 2026 10:00:00 GMT"},
    )

    def opener(request, timeout):
        headers = dict(request.header_items())
        assert request.method == "GET"
        assert headers["If-none-match"] == '"v1"'
        assert headers["If-modified-since"] == "Tue, 14 Jul 2026 10:00:00 GMT"
        assert headers["User-agent"] == "CanISend/Test"
        assert timeout == 30
        return response

    result = PublicTransport(
        opener=opener,
        resolver=lambda hostname: PUBLIC_ADDRESS,
    ).fetch(
        "https://example.edu/feed.xml",
        policy=_policy(),
        etag='"v1"',
        last_modified="Tue, 14 Jul 2026 10:00:00 GMT",
    )

    assert result.not_modified is True
    assert result.status_code == 304
    assert result.etag == '"v2"'
    assert result.body == b""
    assert response.read_sizes == []


def test_transport_handles_urllib_304_without_reading_error_body() -> None:
    headers = Message()
    headers["ETag"] = '"cached"'

    def opener(request, timeout):
        raise HTTPError(request.full_url, 304, "PRIVATE BODY", headers, None)

    result = PublicTransport(
        opener=opener,
        resolver=lambda hostname: PUBLIC_ADDRESS,
    ).fetch("https://example.edu/feed.xml", policy=_policy())

    assert result.not_modified is True
    assert result.etag == '"cached"'
    assert result.attempts == 1


def test_transport_honors_retry_after_then_succeeds_without_reading_error_body() -> None:
    responses = [
        FakeResponse(status=503, headers={"Retry-After": "2"}),
        FakeResponse(body=b"<rss version='2.0'><channel /></rss>"),
    ]
    sleeps: list[float] = []

    result = PublicTransport(
        opener=lambda request, timeout: responses.pop(0),
        resolver=lambda hostname: PUBLIC_ADDRESS,
        sleep=sleeps.append,
    ).fetch(
        "https://example.edu/feed.xml",
        policy=_policy(max_attempts=2, backoff_seconds=0.5),
    )

    assert result.status_code == 200
    assert result.attempts == 2
    assert sleeps == [2.0]


def test_transport_parses_http_date_retry_after_with_injected_clock() -> None:
    responses = [
        FakeResponse(
            status=429,
            headers={"Retry-After": "Wed, 15 Jul 2026 10:00:10 GMT"},
        ),
        FakeResponse(),
    ]
    sleeps: list[float] = []
    transport = PublicTransport(
        opener=lambda request, timeout: responses.pop(0),
        resolver=lambda hostname: PUBLIC_ADDRESS,
        sleep=sleeps.append,
        now=lambda: datetime(2026, 7, 15, 10, 0, tzinfo=UTC),
    )

    transport.fetch(
        "https://example.edu/feed.xml",
        policy=_policy(max_attempts=2),
    )

    assert sleeps == [10.0]


def test_transport_retries_are_bounded_and_exception_bodies_are_not_exposed() -> None:
    calls = 0

    def opener(request, timeout):
        nonlocal calls
        calls += 1
        raise RuntimeError("PRIVATE RESPONSE BODY token=secret")

    with pytest.raises(PublicTransportError) as failure:
        PublicTransport(
            opener=opener,
            resolver=lambda hostname: PUBLIC_ADDRESS,
            sleep=lambda seconds: None,
        ).fetch(
            "https://example.edu/feed.xml?token=secret",
            policy=_policy(max_attempts=3, backoff_seconds=0),
        )

    assert calls == 3
    assert failure.value.attempts == 3
    assert failure.value.retryable is True
    assert "PRIVATE" not in str(failure.value)
    assert "token=secret" not in str(failure.value)


def test_transport_retries_body_read_failures_without_exposing_details() -> None:
    broken = FakeResponse()

    def fail_read(size=-1):
        raise OSError("PRIVATE READ DETAIL token=secret")

    broken.read = fail_read  # type: ignore[method-assign]
    responses = [broken, FakeResponse()]

    result = PublicTransport(
        opener=lambda request, timeout: responses.pop(0),
        resolver=lambda hostname: PUBLIC_ADDRESS,
        sleep=lambda seconds: None,
    ).fetch(
        "https://example.edu/feed.xml",
        policy=_policy(max_attempts=2, backoff_seconds=0),
    )

    assert result.attempts == 2


def test_transport_throttles_repeated_requests_per_host() -> None:
    current = [0.0]
    sleeps: list[float] = []

    def sleep(seconds: float) -> None:
        sleeps.append(seconds)
        current[0] += seconds

    transport = PublicTransport(
        opener=lambda request, timeout: FakeResponse(),
        resolver=lambda hostname: PUBLIC_ADDRESS,
        sleep=sleep,
        monotonic=lambda: current[0],
    )
    policy = _policy(min_interval_seconds=5.0)

    transport.fetch("https://example.edu/a.xml", policy=policy)
    transport.fetch("https://example.edu/b.xml", policy=policy)

    assert sleeps == [5.0]


def test_transport_revalidates_final_redirect_before_body_read() -> None:
    response = FakeResponse(final_url="https://internal.example/feed.xml")

    def resolver(hostname: str):
        return ("10.0.0.9",) if hostname == "internal.example" else PUBLIC_ADDRESS

    with pytest.raises(PublicTransportError, match="publicly routable"):
        PublicTransport(
            opener=lambda request, timeout: response,
            resolver=resolver,
        ).fetch("https://example.edu/feed.xml", policy=_policy())

    assert response.read_sizes == []


def test_transport_rejects_media_type_and_size_without_echoing_body() -> None:
    private_body = b"PRIVATE RESPONSE BODY token=secret"
    wrong_media = FakeResponse(private_body, content_type="text/private-token")
    with pytest.raises(PublicTransportError) as media_failure:
        PublicTransport(
            opener=lambda request, timeout: wrong_media,
            resolver=lambda hostname: PUBLIC_ADDRESS,
        ).fetch("https://example.edu/feed.xml", policy=_policy())

    oversized = FakeResponse(private_body, headers={"Content-Length": "9999"})
    with pytest.raises(PublicTransportError) as size_failure:
        PublicTransport(
            opener=lambda request, timeout: oversized,
            resolver=lambda hostname: PUBLIC_ADDRESS,
        ).fetch(
            "https://example.edu/feed.xml",
            policy=_policy(max_bytes=10),
        )

    assert wrong_media.read_sizes == []
    assert oversized.read_sizes == []
    for failure in (media_failure.value, size_failure.value):
        assert "PRIVATE" not in str(failure)
        assert "token=secret" not in str(failure)
        assert "private-token" not in str(failure)


def test_injected_wait_and_clock_failures_become_stable_transport_errors() -> None:
    retrying = PublicTransport(
        opener=lambda request, timeout: FakeResponse(status=503),
        resolver=lambda hostname: PUBLIC_ADDRESS,
        sleep=lambda seconds: (_ for _ in ()).throw(
            RuntimeError("PRIVATE WAIT DETAIL")
        ),
    )
    with pytest.raises(PublicTransportError) as wait_failure:
        retrying.fetch(
            "https://example.edu/feed.xml",
            policy=_policy(max_attempts=2),
        )

    broken_clock = PublicTransport(
        opener=lambda request, timeout: FakeResponse(),
        resolver=lambda hostname: PUBLIC_ADDRESS,
        monotonic=lambda: (_ for _ in ()).throw(
            RuntimeError("PRIVATE CLOCK DETAIL")
        ),
    )
    with pytest.raises(PublicTransportError) as clock_failure:
        broken_clock.fetch("https://example.edu/feed.xml", policy=_policy())

    assert wait_failure.value.code == "transport.retry_wait_failed"
    assert clock_failure.value.code == "transport.clock_failed"
    assert "PRIVATE" not in str(wait_failure.value)
    assert "PRIVATE" not in str(clock_failure.value)
