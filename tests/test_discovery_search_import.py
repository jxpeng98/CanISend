from __future__ import annotations

from datetime import UTC, datetime
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import pytest
from typer.testing import CliRunner

from canisend.cli import app
from canisend.discovery.agent import discovery_search_import_agent_response
from canisend.discovery.catalog_models import normalized_ranking_policy
from canisend.discovery.refresh import load_lead_batch, refresh_discovery_sources
from canisend.discovery.refresh_models import DiscoverySourceV1, DiscoverySourcesV1
from canisend.discovery.search_import import (
    DiscoverySearchImportInputError,
    import_host_search_file,
)
from canisend.discovery.search_models import DiscoverySearchV1
from canisend.discovery.transport import PublicTransport


OBSERVED_AT = datetime(2026, 7, 15, 13, 0, tzinfo=UTC)
LATER_AT = datetime(2026, 7, 15, 14, 0, tzinfo=UTC)
FIXTURES = Path("tests/fixtures/discovery_search")
PUBLIC_ADDRESS = ("93.184.216.34",)


class FakeResponse:
    def __init__(self, body: bytes, *, final_url: str) -> None:
        self.body = body
        self.status = 200
        self.final_url = final_url
        self.headers = {"Content-Type": "application/rss+xml; charset=utf-8"}

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        return None

    def geturl(self) -> str:
        return self.final_url

    def read(self, size: int = -1) -> bytes:
        return self.body if size < 0 else self.body[:size]


def _rss() -> bytes:
    return (
        "<?xml version='1.0' encoding='UTF-8'?>"
        "<rss version='2.0'><channel><title>Jobs</title><item>"
        "<title>Network Reader</title><guid>network-1</guid>"
        "<link>https://network.example.edu/jobs/1</link>"
        "<description>Public network role.</description>"
        "</item></channel></rss>"
    ).encode("utf-8")


def _transport() -> PublicTransport:
    return PublicTransport(
        opener=lambda request, timeout: FakeResponse(
            _rss(),
            final_url=request.full_url,
        ),
        resolver=lambda hostname: PUBLIC_ADDRESS,
        sleep=lambda seconds: None,
        now=lambda: LATER_AT,
    )


def _envelope(**updates) -> dict:
    payload = json.loads((FIXTURES / "generic-host.json").read_text(encoding="utf-8"))
    payload.update(updates)
    return payload


def _write_envelope(path: Path, payload: dict | None = None) -> Path:
    path.write_text(
        json.dumps(payload or _envelope()),
        encoding="utf-8",
    )
    return path


def test_codex_claude_and_generic_fixtures_produce_identical_artifacts(
    tmp_path: Path,
) -> None:
    executions = []
    for host in ("codex", "claude", "generic-host"):
        workspace = tmp_path / host
        executions.append(
            import_host_search_file(
                workspace,
                FIXTURES / f"{host}.json",
                clock=lambda: LATER_AT,
            )
        )

    batches = [item.batch.model_dump(mode="json") for item in executions]
    catalogs = [item.catalog.model_dump(mode="json") for item in executions]
    assert batches[0] == batches[1] == batches[2]
    assert catalogs[0] == catalogs[1] == catalogs[2]
    assert executions[0].batch.adapter == "host.search"
    assert executions[0].batch.source_url == "host-search"
    assert executions[0].batch.record_count == 2
    assert {lead.provenance[0].source_type for lead in executions[0].batch.leads} == {
        "host_agent"
    }
    rendered = json.dumps(batches[0], sort_keys=True)
    assert "utm_source" not in rendered
    assert "codex" not in rendered
    assert "claude" not in rendered


def test_search_import_uses_shared_dedupe_filter_and_ranking(
    tmp_path: Path,
) -> None:
    policy = normalized_ranking_policy(
        include_keywords=["economics"],
        exclude_keywords=["fixed-term"],
        source_preference=["Academic Host Search"],
    )

    execution = import_host_search_file(
        tmp_path,
        FIXTURES / "generic-host.json",
        policy=policy,
        clock=lambda: LATER_AT,
    )

    assert [lead.title for lead in execution.catalog.leads] == [
        "Lecturer in Economics"
    ]
    assert [item.lead.title for item in execution.catalog.excluded] == [
        "Research Fellow in Political Economy"
    ]
    assert execution.catalog.leads[0].rank == 1
    assert execution.catalog.leads[0].score > 0


def test_repeated_search_import_reuses_batch_without_duplicate_catalog_leads(
    tmp_path: Path,
) -> None:
    first = import_host_search_file(
        tmp_path,
        FIXTURES / "generic-host.json",
        clock=lambda: LATER_AT,
    )
    second = import_host_search_file(
        tmp_path,
        FIXTURES / "generic-host.json",
        clock=lambda: LATER_AT,
    )

    assert second.batch.batch_id == first.batch.batch_id
    assert second.batch_path.read_bytes() == first.batch_path.read_bytes()
    assert second.catalog.stats.retained_records == 2
    assert len(second.catalog.leads) == 2


def test_network_refresh_retains_current_host_search_batch(tmp_path: Path) -> None:
    imported = import_host_search_file(
        tmp_path,
        FIXTURES / "generic-host.json",
        clock=lambda: OBSERVED_AT,
    )
    sources = DiscoverySourcesV1(
        sources=(
            DiscoverySourceV1(
                source_id="network-board",
                name="Network Board",
                url="https://network.example.edu/feed.xml",
                max_attempts=1,
                backoff_seconds=0,
            ),
        )
    )

    refreshed = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(),
        clock=lambda: LATER_AT,
    )

    assert refreshed.catalog is not None
    assert {lead.title for lead in refreshed.catalog.leads} == {
        "Lecturer in Economics",
        "Research Fellow in Political Economy",
        "Network Reader",
    }
    assert refreshed.report.input_records == imported.batch.record_count + 1


@pytest.mark.parametrize(
    "private_field",
    [
        {"provider": "codex"},
        {"session_id": "PRIVATE SESSION"},
        {"cursor": "PRIVATE CURSOR"},
        {"headers": {"Authorization": "PRIVATE TOKEN"}},
        {"query": "private user search"},
    ],
)
def test_vendor_and_session_fields_fail_closed_before_persistence(
    tmp_path: Path,
    private_field: dict,
) -> None:
    path = _write_envelope(tmp_path / "search.json", _envelope(**private_field))

    with pytest.raises(DiscoverySearchImportInputError):
        import_host_search_file(tmp_path, path, clock=lambda: LATER_AT)

    assert not (tmp_path / "job_leads").exists()


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("source_id", "host-session-private"),
        ("source_name", "private.sender@example.com"),
        ("source_name", "/Users/private/search.json"),
        ("source_name", "token=PRIVATE"),
    ],
)
def test_private_source_identifiers_are_rejected(
    tmp_path: Path,
    field: str,
    value: str,
) -> None:
    path = _write_envelope(tmp_path / "search.json", _envelope(**{field: value}))

    with pytest.raises(DiscoverySearchImportInputError):
        import_host_search_file(tmp_path, path, clock=lambda: LATER_AT)

    assert not (tmp_path / "job_leads").exists()


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("source_url", "https://jobs.example.edu/jobs/token=PRIVATE/1"),
        ("source_url", "https://private.sender@example.com/jobs/1"),
        ("source_record_id", "token=PRIVATE"),
        ("source_record_id", "/Users/private/session"),
    ],
)
def test_private_result_locators_are_rejected_without_persistence(
    tmp_path: Path,
    field: str,
    value: str,
) -> None:
    payload = _envelope()
    payload["results"][0][field] = value
    path = _write_envelope(tmp_path / "search.json", payload)

    with pytest.raises(DiscoverySearchImportInputError):
        import_host_search_file(tmp_path, path, clock=lambda: LATER_AT)

    assert not (tmp_path / "job_leads").exists()


def test_result_count_mismatch_and_future_observation_fail_closed(
    tmp_path: Path,
) -> None:
    mismatch = _write_envelope(
        tmp_path / "mismatch.json",
        _envelope(result_count=3),
    )
    future = _write_envelope(
        tmp_path / "future.json",
        _envelope(observed_at="2026-07-16T13:00:00Z"),
    )

    with pytest.raises(DiscoverySearchImportInputError):
        import_host_search_file(tmp_path, mismatch, clock=lambda: LATER_AT)
    with pytest.raises(DiscoverySearchImportInputError, match="future"):
        import_host_search_file(tmp_path, future, clock=lambda: LATER_AT)

    assert not (tmp_path / "job_leads").exists()


def test_search_directory_symlink_is_rejected_before_writing(tmp_path: Path) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()
    lead_root = tmp_path / "workspace" / "job_leads"
    lead_root.mkdir(parents=True)
    try:
        (lead_root / "searches").symlink_to(outside, target_is_directory=True)
    except OSError:
        pytest.skip("directory symlinks are unavailable")

    with pytest.raises(DiscoverySearchImportInputError, match="symbolic link"):
        import_host_search_file(
            tmp_path / "workspace",
            FIXTURES / "generic-host.json",
            clock=lambda: LATER_AT,
        )

    assert list(outside.iterdir()) == []


def test_agent_and_cli_responses_are_body_free_and_relative(tmp_path: Path) -> None:
    payload = _envelope()
    payload["results"][0]["title"] = "PRIVATE HOST TITLE"
    payload["results"][0]["snippet"] = "PRIVATE HOST SEARCH BODY"
    input_path = _write_envelope(tmp_path / "host-search.json", payload)

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "import-search",
            "--workspace",
            str(tmp_path),
            "--input",
            input_path.name,
            "--format",
            "json",
        ],
    )
    response_payload = json.loads(result.stdout)

    assert result.exit_code == 0, result.output
    assert result.stdout.count("\n") == 1
    assert response_payload["operation"] == "discovery.search_import"
    assert response_payload["ok"] is True
    assert {artifact["path"] for artifact in response_payload["artifacts"]} == {
        "job_leads/catalog.json",
        "job_leads/searches/host-academic-search.batch.json",
    }
    assert response_payload["extensions"]["canisend.discovery.input_records"] == 2
    assert "PRIVATE HOST TITLE" not in result.stdout
    assert "PRIVATE HOST SEARCH BODY" not in result.stdout
    assert str(tmp_path) not in result.stdout

    direct = import_host_search_file(
        tmp_path / "direct",
        input_path,
        clock=lambda: LATER_AT,
    )
    agent_response = discovery_search_import_agent_response(
        tmp_path / "direct",
        direct,
    )
    assert "PRIVATE HOST TITLE" not in agent_response.model_dump_json()
    assert "PRIVATE HOST SEARCH BODY" not in agent_response.model_dump_json()


def test_invalid_and_unexpected_cli_errors_are_private_safe(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    invalid_path = tmp_path / "vendor.json"
    invalid_path.write_text(
        json.dumps({"results": [], "private": "PRIVATE VENDOR BODY"}),
        encoding="utf-8",
    )
    invalid = CliRunner().invoke(
        app,
        [
            "discovery",
            "import-search",
            "--workspace",
            str(tmp_path),
            "--input",
            invalid_path.name,
            "--format",
            "json",
        ],
    )

    assert invalid.exit_code == 1
    assert json.loads(invalid.stdout)["error"]["code"] == "input.invalid"
    assert "PRIVATE VENDOR BODY" not in invalid.stdout
    assert str(tmp_path) not in invalid.stdout

    valid_path = _write_envelope(tmp_path / "valid.json")

    def fail_import(*args, **kwargs):
        raise RuntimeError("PRIVATE UNEXPECTED HOST DETAIL")

    monkeypatch.setattr("canisend.cli.import_host_search_file", fail_import)
    unexpected = CliRunner().invoke(
        app,
        [
            "discovery",
            "import-search",
            "--workspace",
            str(tmp_path),
            "--input",
            valid_path.name,
            "--format",
            "json",
        ],
    )

    assert unexpected.exit_code == 1
    assert json.loads(unexpected.stdout)["error"]["code"] == "operation.failed"
    assert "PRIVATE UNEXPECTED HOST DETAIL" not in unexpected.stdout
    assert str(tmp_path) not in unexpected.stdout


def test_invalid_host_batch_is_skipped_during_refresh(tmp_path: Path) -> None:
    searches = tmp_path / "job_leads" / "searches"
    searches.mkdir(parents=True)
    (searches / "invalid.batch.json").write_text(
        json.dumps({"protocol": "canisend.discovery-batch/v1", "private": "PRIVATE"}),
        encoding="utf-8",
    )
    sources = DiscoverySourcesV1(
        sources=(
            DiscoverySourceV1(
                source_id="network-board",
                name="Network Board",
                url="https://network.example.edu/feed.xml",
                max_attempts=1,
                backoff_seconds=0,
            ),
        )
    )

    refreshed = refresh_discovery_sources(
        tmp_path,
        sources,
        transport=_transport(),
        clock=lambda: LATER_AT,
    )

    assert refreshed.catalog is not None
    assert [lead.title for lead in refreshed.catalog.leads] == ["Network Reader"]
    assert "PRIVATE" not in refreshed.report.model_dump_json()


def test_search_schema_is_generated_and_rejects_vendor_envelopes() -> None:
    stored = json.loads(
        Path("schemas/discovery-search-v1.schema.json").read_text(encoding="utf-8")
    )

    assert stored == DiscoverySearchV1.model_json_schema(mode="validation")
    Draft202012Validator.check_schema(stored)
    with pytest.raises(ValidationError):
        DiscoverySearchV1.model_validate(
            {
                **_envelope(),
                "provider": "codex",
                "session_id": "PRIVATE",
            }
        )


def test_persisted_batch_loads_through_strict_contract(tmp_path: Path) -> None:
    execution = import_host_search_file(
        tmp_path,
        FIXTURES / "generic-host.json",
        clock=lambda: LATER_AT,
    )

    loaded = load_lead_batch(execution.batch_path)
    assert loaded == execution.batch
