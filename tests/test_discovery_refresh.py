from __future__ import annotations

from datetime import UTC, datetime
from hashlib import sha256
import json
from pathlib import Path
from urllib.parse import urlsplit

from pydantic import ValidationError
import pytest
import yaml
from jsonschema import Draft202012Validator
from typer.testing import CliRunner

from canisend.cli import app
from canisend.discovery.agent import discovery_refresh_agent_response
from canisend.discovery.catalog import DiscoveryWriteError
from canisend.discovery.catalog_models import RankingPolicyV1
from canisend.discovery.refresh import (
    DiscoveryRefreshInputError,
    load_discovery_cache,
    load_discovery_sources,
    load_lead_batch,
    refresh_discovery_sources,
)
from canisend.discovery.refresh_models import (
    DiscoveryCacheV1,
    DiscoveryRefreshReportV1,
    DiscoverySourceV1,
    DiscoverySourcesV1,
    LeadBatchV1,
)
from canisend.discovery.store import DiscoveryStoreError
from canisend.discovery.transport import PublicTransport


OBSERVED_AT = datetime(2026, 7, 15, 10, 30, tzinfo=UTC)
LATER_AT = datetime(2026, 7, 15, 11, 0, tzinfo=UTC)
PUBLIC_ADDRESS = ("93.184.216.34",)


class FakeResponse:
    def __init__(
        self,
        body: str = "",
        *,
        status: int = 200,
        content_type: str = "application/rss+xml; charset=utf-8",
        final_url: str = "",
        headers: dict[str, str] | None = None,
    ) -> None:
        self.body = body.encode("utf-8")
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


def _rss(*items: tuple[str, str, str, str]) -> str:
    rendered = "".join(
        "<item>"
        f"<title>{title}</title>"
        f"<guid>{record_id}</guid>"
        f"<link>{link}</link>"
        f"<description>{description}</description>"
        "</item>"
        for title, record_id, link, description in items
    )
    return (
        "<?xml version='1.0' encoding='UTF-8'?>"
        "<rss version='2.0'><channel><title>Jobs</title>"
        f"{rendered}</channel></rss>"
    )


def _source(
    source_id: str,
    name: str,
    url: str,
    **overrides,
) -> DiscoverySourceV1:
    return DiscoverySourceV1(
        source_id=source_id,
        name=name,
        url=url,
        backoff_seconds=0,
        min_interval_seconds=0,
        **overrides,
    )


def _config(
    *sources: DiscoverySourceV1,
    policy: RankingPolicyV1 | None = None,
) -> DiscoverySourcesV1:
    return DiscoverySourcesV1(
        policy=policy or RankingPolicyV1(),
        sources=sources,
    )


def _transport(opener) -> PublicTransport:
    return PublicTransport(
        opener=opener,
        resolver=lambda hostname: PUBLIC_ADDRESS,
        sleep=lambda seconds: None,
        now=lambda: OBSERVED_AT,
    )


def _success_response(request, *, title: str | None = None) -> FakeResponse:
    host = urlsplit(request.full_url).hostname or "example.edu"
    source_key = host.split(".")[0]
    resolved_title = title or f"Lecturer from {source_key}"
    return FakeResponse(
        _rss(
            (
                resolved_title,
                f"{source_key}-1",
                f"https://{host}/jobs/1",
                f"Public description for {source_key}.",
            )
        ),
        final_url=request.full_url,
        headers={
            "ETag": f'"{source_key}-v1"',
            "Last-Modified": "Wed, 15 Jul 2026 10:00:00 GMT",
        },
    )


def test_complete_refresh_writes_strict_batches_cache_catalog_and_report(tmp_path: Path) -> None:
    sources = _config(
        _source(
            "board-a",
            "Board A",
            "https://a.example/feed.xml?department=economics",
        ),
        _source("board-b", "Board B", "https://b.example/feed.xml"),
    )

    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.catalog is not None
    assert execution.catalog.stats.input_records == 2
    assert execution.catalog.stats.retained_records == 2
    assert execution.report.status == "complete"
    assert execution.report.successful_sources == 2
    assert execution.report.stale_sources == 0
    assert execution.report.failed_sources == 0
    assert execution.report.catalog_path == "job_leads/catalog.json"
    assert execution.report_path == tmp_path / "job_leads" / "refresh-report.json"
    assert execution.catalog_path == tmp_path / "job_leads" / "catalog.json"

    for result in execution.report.sources:
        assert result.status == "refreshed"
        assert result.batch_path is not None
        assert result.cache_path is not None
        assert result.batch_path.startswith("job_leads/batches/")
        assert result.cache_path.startswith("job_leads/cache/")
        batch = load_lead_batch(tmp_path / result.batch_path)
        cache = load_discovery_cache(tmp_path / result.cache_path)
        assert batch.record_count == 1
        assert batch.leads[0].rank == 0
        assert cache.content_sha256 == batch.content_sha256

    serialized_report = execution.report_path.read_text(encoding="utf-8")
    assert str(tmp_path) not in serialized_report
    assert "department=economics" not in serialized_report
    assert "Public description" not in serialized_report
    board_a_batch = load_lead_batch(
        tmp_path / execution.report.sources[0].batch_path  # type: ignore[arg-type]
    )
    assert board_a_batch.source_url.endswith("?redacted")
    assert board_a_batch.leads[0].source_feed.endswith("?redacted")


def test_304_reuses_complete_batch_and_sends_validators(tmp_path: Path) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )
    first = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )
    first_result = first.report.sources[0]
    batch_path = tmp_path / first_result.batch_path  # type: ignore[arg-type]
    cache_path = tmp_path / first_result.cache_path  # type: ignore[arg-type]
    original_batch = batch_path.read_bytes()
    original_catalog_id = first.catalog.catalog_id  # type: ignore[union-attr]

    def not_modified(request, timeout):
        headers = dict(request.header_items())
        assert headers["If-none-match"] == '"a-v1"'
        assert headers["If-modified-since"] == "Wed, 15 Jul 2026 10:00:00 GMT"
        return FakeResponse(
            status=304,
            final_url=request.full_url,
            headers={"ETag": '"a-v1"'},
        )

    second = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(not_modified),
        clock=lambda: LATER_AT,
    )

    assert second.report.status == "complete"
    assert second.report.sources[0].status == "not_modified"
    assert second.report.sources[0].http_status == 304
    assert batch_path.read_bytes() == original_batch
    assert second.catalog is not None
    assert second.catalog.catalog_id == original_catalog_id
    assert load_discovery_cache(cache_path).validated_at == LATER_AT


def test_one_source_failure_reuses_stale_batch_while_other_source_advances(
    tmp_path: Path,
) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml"),
        _source("board-b", "Board B", "https://b.example/feed.xml"),
    )
    initial = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )
    board_b_result = next(
        item for item in initial.report.sources if item.source_id == "board-b"
    )
    board_b_path = tmp_path / board_b_result.batch_path  # type: ignore[arg-type]
    board_b_bytes = board_b_path.read_bytes()
    calls = {"b": 0}

    def partial_opener(request, timeout):
        host = urlsplit(request.full_url).hostname
        if host == "b.example":
            calls["b"] += 1
            raise RuntimeError("PRIVATE RESPONSE BODY token=secret")
        return _success_response(request, title="Reader from a")

    refreshed = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(partial_opener),
        clock=lambda: LATER_AT,
    )

    assert calls["b"] == 3
    assert refreshed.report.status == "partial"
    assert refreshed.report.successful_sources == 1
    assert refreshed.report.stale_sources == 1
    assert refreshed.report.failed_sources == 0
    board_b_refresh = next(
        item for item in refreshed.report.sources if item.source_id == "board-b"
    )
    assert board_b_refresh.status == "stale_reused"
    assert board_b_refresh.error_code == "transport.network_failed"
    assert board_b_path.read_bytes() == board_b_bytes
    assert refreshed.catalog is not None
    assert {lead.title for lead in refreshed.catalog.leads} == {
        "Reader from a",
        "Lecturer from b",
    }
    rendered = refreshed.report.model_dump_json()
    assert "PRIVATE RESPONSE BODY" not in rendered
    assert "token=secret" not in rendered


def test_failed_source_without_prior_batch_does_not_discard_successful_source(
    tmp_path: Path,
) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml"),
        _source("board-b", "Board B", "https://b.example/feed.xml"),
    )

    def opener(request, timeout):
        if urlsplit(request.full_url).hostname == "b.example":
            raise TimeoutError("PRIVATE TIMEOUT DETAIL")
        return _success_response(request)

    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(opener),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "partial"
    assert execution.report.successful_sources == 1
    assert execution.report.failed_sources == 1
    failed = next(
        item for item in execution.report.sources if item.source_id == "board-b"
    )
    assert failed.status == "failed"
    assert failed.batch_path is None
    assert execution.catalog is not None
    assert execution.catalog.stats.retained_records == 1


def test_all_sources_failed_preserves_existing_catalog_and_writes_failed_report(
    tmp_path: Path,
) -> None:
    catalog_path = tmp_path / "job_leads" / "catalog.json"
    catalog_path.parent.mkdir(parents=True)
    catalog_path.write_bytes(b"existing catalog sentinel\n")
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )

    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(
            lambda request, timeout: (_ for _ in ()).throw(
                RuntimeError("PRIVATE BODY")
            )
        ),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "failed"
    assert execution.report.catalog_promoted is False
    assert execution.report.catalog_error_code == "catalog.no_usable_sources"
    assert execution.catalog is None
    assert execution.catalog_path is None
    assert catalog_path.read_bytes() == b"existing catalog sentinel\n"
    assert execution.report_path.is_file()


def test_catalog_promotion_failure_preserves_existing_catalog(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    catalog_path = tmp_path / "job_leads" / "catalog.json"
    catalog_path.parent.mkdir(parents=True)
    catalog_path.write_bytes(b"existing catalog sentinel\n")
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )

    def fail_catalog(path, catalog):
        raise DiscoveryWriteError("PRIVATE CATALOG STORE DETAIL")

    monkeypatch.setattr(
        "canisend.discovery.refresh.write_lead_catalog",
        fail_catalog,
    )

    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "failed"
    assert execution.report.catalog_promoted is False
    assert execution.report.catalog_error_code == "catalog.promotion_failed"
    assert execution.report.successful_sources == 1
    assert catalog_path.read_bytes() == b"existing catalog sentinel\n"
    assert "PRIVATE CATALOG STORE DETAIL" not in execution.report.model_dump_json()


def test_empty_valid_source_promotes_complete_empty_catalog(tmp_path: Path) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )

    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(
            lambda request, timeout: FakeResponse(
                _rss(),
                final_url=request.full_url,
            )
        ),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "complete"
    assert execution.report.input_records == 0
    assert execution.report.retained_records == 0
    assert execution.catalog is not None
    assert execution.catalog.leads == ()


def test_invalid_new_feed_preserves_previous_complete_batch(tmp_path: Path) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )
    first = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )
    batch_path = tmp_path / first.report.sources[0].batch_path  # type: ignore[arg-type]
    original = batch_path.read_bytes()

    second = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(
            lambda request, timeout: FakeResponse(
                "<rss><PRIVATE RESPONSE BODY>",
                final_url=request.full_url,
            )
        ),
        clock=lambda: LATER_AT,
    )

    assert second.report.status == "partial"
    assert second.report.sources[0].status == "stale_reused"
    assert second.report.sources[0].error_code == "source.parse_invalid"
    assert batch_path.read_bytes() == original
    assert "PRIVATE RESPONSE BODY" not in second.report.model_dump_json()


def test_filter_policy_applies_after_complete_batch_storage(tmp_path: Path) -> None:
    policy = RankingPolicyV1(include_keywords=("economics",))
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml"),
        policy=policy,
    )
    body = _rss(
        (
            "Lecturer in Economics",
            "economics",
            "https://a.example/jobs/economics",
            "Economics role.",
        ),
        (
            "PhD Studentship",
            "phd",
            "https://a.example/jobs/phd",
            "Doctoral role.",
        ),
    )

    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(
            lambda request, timeout: FakeResponse(body, final_url=request.full_url)
        ),
        clock=lambda: OBSERVED_AT,
    )

    batch = load_lead_batch(
        tmp_path / execution.report.sources[0].batch_path  # type: ignore[arg-type]
    )
    assert batch.record_count == 2
    assert execution.catalog is not None
    assert execution.catalog.stats.retained_records == 1
    assert execution.catalog.stats.excluded_records == 1


def test_source_order_does_not_change_catalog_and_report_results_are_sorted(
    tmp_path: Path,
) -> None:
    board_a = _source("board-a", "Board A", "https://a.example/feed.xml")
    board_b = _source("board-b", "Board B", "https://b.example/feed.xml")

    forward = refresh_discovery_sources(
        tmp_path / "forward",
        _config(board_a, board_b),
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )
    reverse = refresh_discovery_sources(
        tmp_path / "reverse",
        _config(board_b, board_a),
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )

    assert forward.catalog is not None and reverse.catalog is not None
    assert forward.catalog.model_dump(mode="json") == reverse.catalog.model_dump(mode="json")
    assert [item.source_id for item in reverse.report.sources] == ["board-a", "board-b"]


def test_source_config_loader_is_strict_and_rejects_credential_queries(
    tmp_path: Path,
) -> None:
    config_path = tmp_path / "sources.yaml"
    config_path.write_text(
        yaml.safe_dump(
            {
                "protocol": "canisend.discovery-sources/v1",
                "schema_version": "1.0.0",
                "sources": [
                    {
                        "source_id": "board-a",
                        "name": "Board A",
                        "url": "https://a.example/feed.xml",
                    }
                ],
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )

    loaded = load_discovery_sources(config_path)
    assert loaded.sources[0].source_id == "board-a"

    invalid = yaml.safe_load(config_path.read_text(encoding="utf-8"))
    invalid["sources"][0]["url"] = "https://a.example/feed.xml?token=PRIVATE"
    invalid["vendor_private"] = "PRIVATE BODY"
    config_path.write_text(yaml.safe_dump(invalid), encoding="utf-8")
    with pytest.raises(DiscoveryRefreshInputError) as failure:
        load_discovery_sources(config_path)

    assert str(tmp_path) not in str(failure.value)
    assert "PRIVATE" not in str(failure.value)


def test_refresh_rejects_invalid_injected_clock_without_exposing_detail(
    tmp_path: Path,
) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )

    with pytest.raises(DiscoveryRefreshInputError) as failure:
        refresh_discovery_sources(
            tmp_path,
            sources,
            transport=_transport(lambda request, timeout: _success_response(request)),
            clock=lambda: (_ for _ in ()).throw(
                RuntimeError("PRIVATE CLOCK DETAIL")
            ),
        )

    assert "PRIVATE" not in str(failure.value)


def test_refresh_rejects_batch_directory_symlink_before_transport(
    tmp_path: Path,
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()
    lead_root = tmp_path / "workspace" / "job_leads"
    lead_root.mkdir(parents=True)
    try:
        (lead_root / "batches").symlink_to(outside, target_is_directory=True)
    except OSError:
        pytest.skip("directory symlinks are unavailable")
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )
    opened = False

    def opener(request, timeout):
        nonlocal opened
        opened = True
        return _success_response(request)

    with pytest.raises(DiscoveryRefreshInputError, match="symbolic links"):
        refresh_discovery_sources(
            tmp_path / "workspace",
            sources,
            lead_root=lead_root,
            transport=_transport(opener),
            clock=lambda: OBSERVED_AT,
        )

    assert opened is False
    assert list(outside.iterdir()) == []


def test_invalid_cached_contracts_are_not_reused_on_304(tmp_path: Path) -> None:
    catalog_path = tmp_path / "job_leads" / "catalog.json"
    catalog_path.parent.mkdir(parents=True)
    catalog_path.write_bytes(b"existing catalog\n")
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )
    batches = tmp_path / "job_leads" / "batches"
    cache = tmp_path / "job_leads" / "cache"
    batches.mkdir()
    cache.mkdir()
    artifact_hash = sha256(
        "https://a.example/feed.xml".encode("utf-8")
    ).hexdigest()[:12]
    (batches / f"board-a-{artifact_hash}.json").write_text(
        json.dumps({"protocol": "vendor.batch/v1", "private": "PRIVATE BODY"}),
        encoding="utf-8",
    )

    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(
            lambda request, timeout: FakeResponse(
                status=304,
                final_url=request.full_url,
            )
        ),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "failed"
    assert execution.report.sources[0].status == "failed"
    assert execution.report.sources[0].error_code == "cache.not_modified_without_batch"
    assert catalog_path.read_bytes() == b"existing catalog\n"


def test_batch_promotion_failure_preserves_old_batch_and_marks_source_stale(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )
    first = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )
    batch_path = tmp_path / first.report.sources[0].batch_path  # type: ignore[arg-type]
    original = batch_path.read_bytes()

    import canisend.discovery.refresh as refresh_module

    original_write = refresh_module.atomic_write_json

    def fail_batch(path, value):
        if Path(path).parent.name == "batches":
            raise DiscoveryStoreError("PRIVATE STORE DETAIL")
        return original_write(path, value)

    monkeypatch.setattr(refresh_module, "atomic_write_json", fail_batch)
    second = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(
            lambda request, timeout: _success_response(request, title="New title")
        ),
        clock=lambda: LATER_AT,
    )

    assert second.report.status == "partial"
    assert second.report.sources[0].status == "stale_reused"
    assert second.report.sources[0].error_code == "store.source_promotion_failed"
    assert second.report.sources[0].cache_path is None
    assert batch_path.read_bytes() == original
    assert "PRIVATE STORE DETAIL" not in second.report.model_dump_json()


def test_batch_cache_and_report_contracts_reject_tampering(tmp_path: Path) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml")
    )
    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(lambda request, timeout: _success_response(request)),
        clock=lambda: OBSERVED_AT,
    )
    result = execution.report.sources[0]
    batch = load_lead_batch(tmp_path / result.batch_path)  # type: ignore[arg-type]
    cache = load_discovery_cache(tmp_path / result.cache_path)  # type: ignore[arg-type]

    with pytest.raises(ValidationError):
        LeadBatchV1.model_validate(
            {**batch.model_dump(mode="json"), "record_count": 99}
        )
    with pytest.raises(ValidationError):
        DiscoveryCacheV1.model_validate(
            {
                **cache.model_dump(mode="json"),
                "source_url": "https://a.example/feed.xml?private=value",
            }
        )
    with pytest.raises(ValidationError):
        DiscoveryRefreshReportV1.model_validate(
            {**execution.report.model_dump(mode="json"), "successful_sources": 99}
        )
    with pytest.raises(ValidationError):
        DiscoveryRefreshReportV1.model_validate(
            {**execution.report.model_dump(mode="json"), "refresh_id": "refresh_" + "0" * 32}
        )


def test_refresh_agent_response_keeps_partial_failure_body_free(tmp_path: Path) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml", max_attempts=1),
        _source("board-b", "Board B", "https://b.example/feed.xml", max_attempts=1),
    )

    def opener(request, timeout):
        if urlsplit(request.full_url).hostname == "b.example":
            raise RuntimeError("PRIVATE RESPONSE BODY token=secret")
        return _success_response(request, title="PRIVATE TITLE A")

    execution = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(opener),
        clock=lambda: OBSERVED_AT,
    )
    response = discovery_refresh_agent_response(tmp_path, execution)
    rendered = response.model_dump_json()

    assert response.ok is True
    assert response.operation == "discovery.refresh"
    assert response.extensions["canisend.discovery.refresh_status"] == "partial"
    assert "discovery.partial_refresh" in response.warnings
    assert "discovery.sources_unavailable" in response.warnings
    assert {artifact.path for artifact in response.artifacts} == {
        "job_leads/catalog.json",
        "job_leads/refresh-report.json",
    }
    assert "PRIVATE TITLE" not in rendered
    assert "PRIVATE RESPONSE BODY" not in rendered
    assert str(tmp_path) not in rendered


def test_discovery_refresh_cli_json_is_one_line_and_body_free(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sources = _config(
        _source(
            "board-a",
            "Board A",
            "https://a.example/feed.xml?department=economics",
            max_attempts=1,
        )
    )
    source_path = tmp_path / "discovery-sources.yaml"
    source_path.write_text(
        yaml.safe_dump(sources.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )
    fake_transport = _transport(
        lambda request, timeout: _success_response(
            request,
            title="PRIVATE TITLE SENTINEL",
        )
    )
    monkeypatch.setattr(
        "canisend.discovery.refresh.PublicTransport",
        lambda **kwargs: fake_transport,
    )

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "refresh",
            "--workspace",
            str(tmp_path),
            "--sources",
            "discovery-sources.yaml",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 0, result.output
    assert result.stdout.count("\n") == 1
    assert payload["operation"] == "discovery.refresh"
    assert payload["ok"] is True
    assert payload["extensions"]["canisend.discovery.refresh_status"] == "complete"
    assert {artifact["path"] for artifact in payload["artifacts"]} == {
        "job_leads/catalog.json",
        "job_leads/refresh-report.json",
    }
    assert "PRIVATE TITLE SENTINEL" not in result.stdout
    assert "department=economics" not in result.stdout
    assert str(tmp_path) not in result.stdout


def test_discovery_refresh_cli_failed_run_returns_body_free_agent_error(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sources = _config(
        _source(
            "board-a",
            "Board A",
            "https://a.example/feed.xml",
            max_attempts=1,
        )
    )
    (tmp_path / "discovery-sources.yaml").write_text(
        yaml.safe_dump(sources.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )
    fake_transport = _transport(
        lambda request, timeout: (_ for _ in ()).throw(
            RuntimeError("PRIVATE FAILURE DETAIL")
        )
    )
    monkeypatch.setattr(
        "canisend.discovery.refresh.PublicTransport",
        lambda **kwargs: fake_transport,
    )

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "refresh",
            "--workspace",
            str(tmp_path),
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 1
    assert result.stdout.count("\n") == 1
    assert payload["ok"] is False
    assert payload["error"]["code"] == "source.import_failed"
    assert payload["artifacts"][0]["path"] == "job_leads/refresh-report.json"
    assert "PRIVATE FAILURE DETAIL" not in result.stdout
    assert str(tmp_path) not in result.stdout


def test_discovery_refresh_cli_invalid_config_preserves_existing_catalog(
    tmp_path: Path,
) -> None:
    catalog_path = tmp_path / "job_leads" / "catalog.json"
    catalog_path.parent.mkdir(parents=True)
    catalog_path.write_bytes(b"existing catalog sentinel\n")
    (tmp_path / "discovery-sources.yaml").write_text(
        "protocol: canisend.discovery-sources/v1\n"
        "schema_version: 1.0.0\n"
        "vendor_private: PRIVATE CONFIG BODY\n"
        "sources: []\n",
        encoding="utf-8",
    )

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "refresh",
            "--workspace",
            str(tmp_path),
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 1
    assert result.stdout.count("\n") == 1
    assert payload["error"]["code"] == "input.invalid"
    assert "PRIVATE CONFIG BODY" not in result.stdout
    assert str(tmp_path) not in result.stdout
    assert catalog_path.read_bytes() == b"existing catalog sentinel\n"


def test_discovery_refresh_cli_text_hides_unexpected_failure_detail(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml", max_attempts=1)
    )
    (tmp_path / "discovery-sources.yaml").write_text(
        yaml.safe_dump(sources.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )

    def fail_refresh(*args, **kwargs):
        raise RuntimeError("PRIVATE UNEXPECTED FAILURE DETAIL")

    monkeypatch.setattr("canisend.cli.refresh_discovery_sources", fail_refresh)

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "refresh",
            "--workspace",
            str(tmp_path),
        ],
    )

    assert result.exit_code == 2
    assert "The discovery refresh operation failed unexpectedly." in result.output
    assert "PRIVATE UNEXPECTED FAILURE DETAIL" not in result.output
    assert str(tmp_path) not in result.output


def test_discovery_refresh_cli_text_uses_relative_private_safe_artifacts(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sources = _config(
        _source("board-a", "Board A", "https://a.example/feed.xml", max_attempts=1)
    )
    (tmp_path / "discovery-sources.yaml").write_text(
        yaml.safe_dump(sources.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )
    fake_transport = _transport(
        lambda request, timeout: _success_response(
            request,
            title="PRIVATE TITLE SENTINEL",
        )
    )
    monkeypatch.setattr(
        "canisend.discovery.refresh.PublicTransport",
        lambda **kwargs: fake_transport,
    )

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "refresh",
            "--workspace",
            str(tmp_path),
        ],
    )

    assert result.exit_code == 0, result.output
    assert "Operation: discovery.refresh" in result.output
    assert "job_leads/catalog.json" in result.output
    assert "job_leads/refresh-report.json" in result.output
    assert "Refresh: complete, 1 current, 0 stale, 0 failed, 1 retained" in result.output
    assert "PRIVATE TITLE SENTINEL" not in result.output
    assert str(tmp_path) not in result.output


@pytest.mark.parametrize(
    ("filename", "model"),
    [
        ("discovery-sources-v1.schema.json", DiscoverySourcesV1),
        ("lead-batch-v1.schema.json", LeadBatchV1),
        ("discovery-cache-v1.schema.json", DiscoveryCacheV1),
        ("discovery-refresh-report-v1.schema.json", DiscoveryRefreshReportV1),
    ],
)
def test_refresh_schemas_are_generated_from_runtime_models(filename, model) -> None:
    stored = json.loads(Path("schemas", filename).read_text(encoding="utf-8"))

    assert stored == model.model_json_schema(mode="validation")
    Draft202012Validator.check_schema(stored)
