from __future__ import annotations

from datetime import UTC, datetime
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import pytest

from canisend.discovery.adapters import (
    discovery_adapter,
    registered_discovery_adapters,
)
from canisend.discovery.refresh import load_lead_batch, refresh_discovery_sources
from canisend.discovery.refresh_models import DiscoverySourceV1, DiscoverySourcesV1
from canisend.discovery.transport import PublicTransport


OBSERVED_AT = datetime(2026, 7, 15, 15, 0, tzinfo=UTC)
FIXTURES = Path("tests/fixtures/discovery_adapters")
PUBLIC_ADDRESS = ("93.184.216.34",)


class FakeResponse:
    def __init__(
        self,
        body: bytes,
        *,
        final_url: str,
        content_type: str = "application/json; charset=utf-8",
        status: int = 200,
    ) -> None:
        self.body = body
        self.status = status
        self.final_url = final_url
        self.headers = {"Content-Type": content_type}

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        return None

    def geturl(self) -> str:
        return self.final_url

    def read(self, size: int = -1) -> bytes:
        return self.body if size < 0 else self.body[:size]


def _fixture(name: str) -> bytes:
    return (FIXTURES / name).read_bytes()


def _greenhouse(**updates) -> DiscoverySourceV1:
    values = {
        "source_id": "greenhouse-example",
        "name": "Example University",
        "kind": "greenhouse",
        "board_token": "example_university",
        "max_leads": 100,
        "max_attempts": 1,
        "backoff_seconds": 0,
    }
    values.update(updates)
    return DiscoverySourceV1(**values)


def _lever(**updates) -> DiscoverySourceV1:
    values = {
        "source_id": "lever-example",
        "name": "Example Institute",
        "kind": "lever",
        "site_id": "example",
        "max_leads": 100,
        "max_attempts": 1,
        "backoff_seconds": 0,
    }
    values.update(updates)
    return DiscoverySourceV1(**values)


def _transport(opener) -> PublicTransport:
    return PublicTransport(
        opener=opener,
        resolver=lambda hostname: PUBLIC_ADDRESS,
        sleep=lambda seconds: None,
        now=lambda: OBSERVED_AT,
    )


def test_registered_adapters_freeze_read_only_endpoint_contracts() -> None:
    adapters = registered_discovery_adapters()
    assert {(adapter.kind, adapter.adapter_id) for adapter in adapters} == {
        ("greenhouse", "public_api.greenhouse"),
        ("lever", "public_api.lever"),
        ("rss_atom", "feed.rss_atom"),
    }

    greenhouse = discovery_adapter(_greenhouse())
    assert greenhouse.request_url(_greenhouse()) == (
        "https://boards-api.greenhouse.io/v1/boards/"
        "example_university/jobs?content=true"
    )
    lever = discovery_adapter(_lever())
    assert lever.request_url(_lever()) == (
        "https://api.lever.co/v0/postings/example?limit=100&mode=json"
    )
    eu_source = _lever(region="eu")
    assert discovery_adapter(eu_source).request_url(eu_source) == (
        "https://api.eu.lever.co/v0/postings/example?limit=100&mode=json"
    )
    for adapter, source in ((greenhouse, _greenhouse()), (lever, _lever())):
        request_url = adapter.request_url(source)
        assert request_url.startswith("https://")
        assert "/apply" not in request_url
        assert "key=" not in request_url
        assert adapter.transport_spec().allowed_media_types == ("application/json",)


@pytest.mark.parametrize(
    "payload",
    [
        {"source_id": "gh", "name": "GH", "kind": "greenhouse"},
        {
            "source_id": "gh",
            "name": "GH",
            "kind": "greenhouse",
            "board_token": "example",
            "url": "https://boards-api.greenhouse.io/v1/boards/example/jobs",
        },
        {
            "source_id": "lever",
            "name": "Lever",
            "kind": "lever",
            "site_id": "example/apply",
        },
        {
            "source_id": "lever",
            "name": "Lever",
            "kind": "lever",
            "site_id": "example",
            "api_key": "PRIVATE",
        },
        {
            "source_id": "feed",
            "name": "Feed",
            "kind": "rss_atom",
            "url": "https://jobs.example.edu/feed.xml",
            "site_id": "example",
        },
        {
            "source_id": "gh",
            "name": "GH",
            "kind": "greenhouse",
            "board_token": "UPPERCASE",
        },
    ],
)
def test_source_configuration_rejects_auth_urls_and_malformed_identifiers(
    payload: dict,
) -> None:
    with pytest.raises(ValidationError):
        DiscoverySourceV1.model_validate(payload)


def test_greenhouse_and_lever_refresh_map_only_published_job_fields(
    tmp_path: Path,
) -> None:
    requests = []

    def opener(request, timeout):
        requests.append(request)
        if request.full_url.startswith("https://boards-api.greenhouse.io/"):
            body = _fixture("greenhouse-list.json")
        else:
            body = _fixture("lever-list.json")
        return FakeResponse(body, final_url=request.full_url)

    execution = refresh_discovery_sources(
        tmp_path,
        DiscoverySourcesV1(sources=(_greenhouse(), _lever())),
        transport=_transport(opener),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "complete"
    assert execution.report.input_records == 4
    assert execution.catalog is not None
    assert {lead.title for lead in execution.catalog.leads} == {
        "Lecturer in Economics",
        "Research Fellow",
        "Assistant Professor of Economics",
        "Postdoctoral Researcher",
    }
    assert all(request.get_method() == "GET" for request in requests)
    assert all(request.get_header("Authorization") is None for request in requests)
    assert len(requests) == 2

    batches = [
        load_lead_batch(path)
        for path in sorted((tmp_path / "job_leads" / "batches").glob("*.json"))
    ]
    assert {batch.adapter for batch in batches} == {
        "public_api.greenhouse",
        "public_api.lever",
    }
    greenhouse_batch = next(
        batch for batch in batches if batch.adapter == "public_api.greenhouse"
    )
    assert greenhouse_batch.leads[0].description
    assert all(
        lead.provenance[0].source_type == "public_api"
        for batch in batches
        for lead in batch.leads
    )
    persisted = "\n".join(
        path.read_text(encoding="utf-8")
        for path in (tmp_path / "job_leads").rglob("*.json")
    )
    assert "utm_source" not in persisted
    assert "applyUrl" not in persisted
    assert "/apply" not in persisted
    assert "PRIVATE APPLICATION FORM SENTINEL" not in persisted
    assert "token=PRIVATE" not in persisted


@pytest.mark.parametrize(
    ("source", "body"),
    [
        (_greenhouse(), b"[]"),
        (_greenhouse(), b'{"data": []}'),
        (_lever(), b'{"postings": []}'),
        (_lever(), b'{"group": "location", "London": []}'),
    ],
)
def test_undocumented_response_roots_fail_closed(
    tmp_path: Path,
    source: DiscoverySourceV1,
    body: bytes,
) -> None:
    execution = refresh_discovery_sources(
        tmp_path,
        DiscoverySourcesV1(sources=(source,)),
        transport=_transport(
            lambda request, timeout: FakeResponse(body, final_url=request.full_url)
        ),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "failed"
    assert execution.catalog is None
    assert execution.report.sources[0].error_code == "source.parse_invalid"
    assert not (tmp_path / "job_leads" / "batches").exists()


def test_public_api_adapter_rejects_undocumented_redirect(tmp_path: Path) -> None:
    execution = refresh_discovery_sources(
        tmp_path,
        DiscoverySourcesV1(sources=(_greenhouse(),)),
        transport=_transport(
            lambda request, timeout: FakeResponse(
                _fixture("greenhouse-list.json"),
                final_url="https://redirect.example.edu/jobs",
            )
        ),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "failed"
    assert execution.report.sources[0].error_code == "source.endpoint_invalid"
    assert execution.catalog is None


def test_public_api_record_limit_fails_before_batch_promotion(tmp_path: Path) -> None:
    source = _greenhouse(max_leads=1)
    execution = refresh_discovery_sources(
        tmp_path,
        DiscoverySourcesV1(sources=(source,)),
        transport=_transport(
            lambda request, timeout: FakeResponse(
                _fixture("greenhouse-list.json"),
                final_url=request.full_url,
            )
        ),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "failed"
    assert execution.report.sources[0].error_code == "source.record_limit"
    assert not (tmp_path / "job_leads" / "batches").exists()


def test_invalid_api_refresh_reuses_previous_complete_batch(tmp_path: Path) -> None:
    source = _greenhouse()
    config = DiscoverySourcesV1(sources=(source,))
    first = refresh_discovery_sources(
        tmp_path,
        config,
        transport=_transport(
            lambda request, timeout: FakeResponse(
                _fixture("greenhouse-list.json"),
                final_url=request.full_url,
            )
        ),
        clock=lambda: OBSERVED_AT,
    )
    second = refresh_discovery_sources(
        tmp_path,
        config,
        transport=_transport(
            lambda request, timeout: FakeResponse(
                b'{"jobs": "PRIVATE INVALID BODY"}',
                final_url=request.full_url,
            )
        ),
        clock=lambda: OBSERVED_AT,
    )

    assert first.catalog is not None
    assert second.report.status == "partial"
    assert second.report.sources[0].status == "stale_reused"
    assert second.report.sources[0].error_code == "source.parse_invalid"
    assert second.catalog == first.catalog
    assert "PRIVATE INVALID BODY" not in second.report.model_dump_json()


def test_lever_adapter_makes_one_request_and_ignores_pagination_like_fields(
    tmp_path: Path,
) -> None:
    records = json.loads(_fixture("lever-list.json"))
    records[0]["next"] = "https://api.lever.co/v0/postings/example?skip=PRIVATE"
    calls = 0

    def opener(request, timeout):
        nonlocal calls
        calls += 1
        return FakeResponse(
            json.dumps(records).encode("utf-8"),
            final_url=request.full_url,
        )

    execution = refresh_discovery_sources(
        tmp_path,
        DiscoverySourcesV1(sources=(_lever(),)),
        transport=_transport(opener),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "complete"
    assert calls == 1
    persisted = (tmp_path / "job_leads" / "catalog.json").read_text(
        encoding="utf-8"
    )
    assert "skip=PRIVATE" not in persisted


def test_discovery_sources_schema_includes_only_frozen_adapter_kinds() -> None:
    stored = json.loads(
        Path("schemas/discovery-sources-v1.schema.json").read_text(encoding="utf-8")
    )
    runtime = DiscoverySourcesV1.model_json_schema(mode="validation")

    assert stored == runtime
    Draft202012Validator.check_schema(stored)
    kind_schema = stored["$defs"]["DiscoverySourceV1"]["properties"]["kind"]
    assert kind_schema["enum"] == ["rss_atom", "greenhouse", "lever"]
