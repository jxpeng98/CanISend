from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path

import yaml

from canisend.discovery.adapters import discovery_adapter
from canisend.discovery.refresh_models import DiscoverySourcesV1
from canisend.discovery.search_models import DiscoverySearchV1


EXAMPLES = Path("examples/discovery")
OBSERVED_AT = datetime(2026, 7, 15, 13, 0, tzinfo=UTC)


def test_discovery_source_and_host_search_examples_match_public_contracts() -> None:
    sources = DiscoverySourcesV1.model_validate(
        yaml.safe_load(
            (EXAMPLES / "discovery-sources.example.yaml").read_text(
                encoding="utf-8"
            )
        )
    )
    search = DiscoverySearchV1.model_validate_json(
        (EXAMPLES / "normalized-search.example.json").read_text(encoding="utf-8")
    )

    assert {source.kind for source in sources.sources} == {
        "rss_atom",
        "greenhouse",
        "lever",
    }
    assert search.result_count == 2
    assert all(result.source_url.startswith("https://") for result in search.results)


def test_packaged_public_adapter_fixtures_map_to_lead_v2() -> None:
    sources = DiscoverySourcesV1.model_validate(
        yaml.safe_load(
            (EXAMPLES / "discovery-sources.example.yaml").read_text(
                encoding="utf-8"
            )
        )
    )
    api_sources = {source.kind: source for source in sources.sources if source.kind != "rss_atom"}

    greenhouse = discovery_adapter(api_sources["greenhouse"]).parse(
        api_sources["greenhouse"],
        (EXAMPLES / "greenhouse-list.fixture.json").read_bytes(),
        content_type="application/json",
        observed_at=OBSERVED_AT,
    )
    lever = discovery_adapter(api_sources["lever"]).parse(
        api_sources["lever"],
        (EXAMPLES / "lever-list.fixture.json").read_bytes(),
        content_type="application/json",
        observed_at=OBSERVED_AT,
    )

    assert [lead.title for lead in greenhouse] == ["Lecturer in Economics"]
    assert [lead.title for lead in lever] == ["Assistant Professor of Economics"]
    assert all(lead.schema_version == "2.0.0" for lead in (*greenhouse, *lever))


def test_public_discovery_examples_exclude_private_transport_and_host_fields() -> None:
    rendered = "\n".join(
        path.read_text(encoding="utf-8")
        for path in EXAMPLES.iterdir()
        if path.is_file()
    ).casefold()

    for forbidden in (
        "api_key",
        "authorization",
        "access_token",
        "session_id",
        "connector_id",
        "applyurl",
        "applicationform",
        "/users/",
    ):
        assert forbidden not in rendered
