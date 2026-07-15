from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import pytest
from typer.testing import CliRunner

from canisend.cli import app
from canisend.discovery.catalog import (
    DiscoveryInputError,
    build_catalog_from_files,
    load_lead_document,
    merge_lead_catalog,
    write_lead_catalog,
)
from canisend.discovery.catalog_models import (
    LeadCatalogV1,
    RankingPolicyV1,
    normalized_ranking_policy,
)
from canisend.discovery.identity import normalize_job_lead
from canisend.discovery.models import JobLeadV2


OBSERVED_AT = "2026-07-15T10:30:00Z"


def _lead(
    *,
    title: str,
    source: str,
    url: str = "",
    record_id: str = "",
    description: str = "",
    institution: str = "",
    location: str = "",
    deadline: str = "",
    observed_at: str = OBSERVED_AT,
) -> JobLeadV2:
    return normalize_job_lead(
        {
            "title": title,
            "source_url": url,
            "description": description,
            "published_at": "2026-07-14T09:00:00Z",
            "source": source,
            "source_feed": f"{source.casefold().replace(' ', '-')}-fixture",
            "source_record_id": record_id,
            "institution": institution,
            "location": location,
            "deadline": deadline,
            "source_type": "json",
        },
        fetched_at=observed_at,
        source_type="json",
        adapter="local.json",
    )


def test_merge_deduplicates_canonical_url_and_preserves_aliases_and_provenance():
    first = _lead(
        title="Lecturer in Economics",
        source="Board A",
        url="https://example.edu/jobs/42",
        record_id="a-42",
        institution="Example University",
        deadline="2026-08-31",
    )
    second = _lead(
        title="Lecturer in Economics and Finance",
        source="Board B",
        url="https://example.edu/jobs/42?utm_source=alert",
        record_id="b-900",
        description="A fuller public description.",
        institution="Example University",
        location="London",
        deadline="2026-08-31",
    )
    policy = normalized_ranking_policy(source_preference=["Board B"])

    catalog = merge_lead_catalog([first, second], policy=policy)

    assert catalog.stats.input_records == 2
    assert catalog.stats.unique_records == 1
    assert catalog.stats.merged_records == 1
    merged = catalog.leads[0]
    assert {merged.lead_id, *merged.alternate_lead_ids} == {
        first.lead_id,
        second.lead_id,
    }
    assert len(merged.provenance) == 2
    assert merged.canonical_url == "https://example.edu/jobs/42"
    assert merged.description == "A fuller public description."
    assert merged.location == "London"
    assert "rank.multi_source" in {reason.code for reason in merged.match_reasons}


def test_merge_is_input_order_independent_and_repeat_refresh_keeps_primary_id():
    first = _lead(
        title="Reader in Finance",
        source="Board A",
        url="https://example.edu/jobs/reader",
        record_id="reader-a",
        institution="Example University",
        deadline="2026-09-01",
    )
    second = _lead(
        title="Reader in Finance",
        source="Board B",
        url="https://example.edu/jobs/reader",
        record_id="reader-b",
        institution="Example University",
        deadline="2026-09-01",
    )

    forward = merge_lead_catalog([first, second])
    reverse = merge_lead_catalog([second, first])
    refreshed = merge_lead_catalog(
        [forward.leads[0], second, first],
        input_record_count=3,
    )

    assert forward.model_dump(mode="json") == reverse.model_dump(mode="json")
    assert refreshed.leads[0].lead_id == forward.leads[0].lead_id
    assert refreshed.leads[0].alternate_lead_ids == forward.leads[0].alternate_lead_ids
    assert refreshed.stats.unique_records == 1
    assert refreshed.stats.merged_records == 2
    assert refreshed.catalog_id == forward.catalog_id


def test_explicit_aliases_form_transitive_merge_groups():
    first = _lead(
        title="Research Fellow",
        source="Board A",
        url="https://example.edu/jobs/research",
        record_id="a-1",
    )
    second = _lead(
        title="Research Fellow",
        source="Board B",
        url="https://example.edu/jobs/research",
        record_id="b-1",
    )
    third = _lead(
        title="Research Fellow in Econometrics",
        source="Board C",
        url="https://other.example/jobs/999",
        record_id="c-1",
    )
    second_with_alias = JobLeadV2.model_validate(
        {
            **second.model_dump(mode="json"),
            "alternate_lead_ids": [third.lead_id],
        }
    )

    catalog = merge_lead_catalog([third, first, second_with_alias])

    assert catalog.stats.unique_records == 1
    assert len({catalog.leads[0].lead_id, *catalog.leads[0].alternate_lead_ids}) == 3
    assert len(catalog.leads[0].provenance) == 3


def test_fallback_fingerprint_merges_one_weak_record_but_not_generic_titles():
    strong = _lead(
        title="Lecturer in Accounting",
        source="Board A",
        url="https://example.edu/jobs/accounting",
        record_id="accounting-1",
        institution="Example University",
        deadline="2026-08-20",
    )
    weak = _lead(
        title="  LECTURER   in Accounting ",
        source="Email Export",
        institution="example university",
        deadline="2026-08-20",
    )
    generic_a = _lead(title="Lecturer", source="Board A", record_id="generic-a")
    generic_b = _lead(title="Lecturer", source="Board B", record_id="generic-b")

    catalog = merge_lead_catalog([generic_b, weak, strong, generic_a])

    assert catalog.stats.unique_records == 3
    merged_group = next(
        lead for lead in catalog.leads if strong.lead_id in {lead.lead_id, *lead.alternate_lead_ids}
    )
    assert weak.lead_id in {merged_group.lead_id, *merged_group.alternate_lead_ids}
    generic_groups = [lead for lead in catalog.leads if lead.title.casefold() == "lecturer"]
    assert len(generic_groups) == 2


def test_conflicting_strong_identities_are_not_merged_by_fingerprint_alone():
    first = _lead(
        title="Lecturer in Economics",
        source="Board A",
        url="https://a.example/jobs/1",
        record_id="a-1",
        institution="Example University",
        deadline="2026-08-31",
    )
    second = _lead(
        title="Lecturer in Economics",
        source="Board B",
        url="https://b.example/jobs/2",
        record_id="b-2",
        institution="Example University",
        deadline="2026-08-31",
    )

    catalog = merge_lead_catalog([first, second])

    assert catalog.stats.unique_records == 2
    assert {lead.lead_id for lead in catalog.leads} == {first.lead_id, second.lead_id}


def test_ranking_and_exclusions_are_deterministic_and_fully_explained():
    preferred = _lead(
        title="Lecturer in Economics",
        source="Preferred Board",
        url="https://example.edu/jobs/economics",
        record_id="economics-1",
        institution="Example University",
        location="London",
        deadline="2026-08-31",
    )
    description_match = _lead(
        title="Research Fellow",
        source="Other Board",
        url="https://other.example/jobs/research",
        record_id="research-1",
        description="Applied economics and public policy.",
    )
    excluded_keyword = _lead(
        title="PhD Studentship in Economics",
        source="Other Board",
        url="https://other.example/jobs/phd",
        record_id="phd-1",
    )
    missing_include = _lead(
        title="Laboratory Manager in Biology",
        source="Other Board",
        url="https://other.example/jobs/biology",
        record_id="biology-1",
    )
    policy = normalized_ranking_policy(
        include_keywords=["ECONOMICS", " economics "],
        exclude_keywords=["PhD"],
        source_preference=["Preferred Board", "Other Board"],
    )

    catalog = merge_lead_catalog(
        [missing_include, excluded_keyword, description_match, preferred],
        policy=policy,
    )

    assert [lead.lead_id for lead in catalog.leads] == [
        preferred.lead_id,
        description_match.lead_id,
    ]
    assert [lead.rank for lead in catalog.leads] == [1, 2]
    assert all(
        lead.score == sum(reason.score_delta for reason in lead.match_reasons)
        for lead in catalog.leads
    )
    preferred_codes = {reason.code for reason in catalog.leads[0].match_reasons}
    assert {
        "match.title_keyword",
        "rank.source_preference",
        "rank.canonical_url",
        "rank.deadline_metadata",
    } <= preferred_codes
    excluded = {item.lead.lead_id: item for item in catalog.excluded}
    assert {reason.code for reason in excluded[excluded_keyword.lead_id].reasons} == {
        "filter.exclude_keyword"
    }
    assert {reason.code for reason in excluded[missing_include.lead_id].reasons} == {
        "filter.include_missing"
    }
    assert catalog.stats.retained_records == 2
    assert catalog.stats.excluded_records == 2


def test_default_policy_retains_leads_with_zero_delta_explanation():
    lead = _lead(title="Lecturer", source="Board A", record_id="lecturer-1")

    catalog = merge_lead_catalog([lead])

    assert catalog.leads[0].rank == 1
    assert catalog.leads[0].score == 0
    assert [reason.code for reason in catalog.leads[0].match_reasons] == [
        "filter.default_include"
    ]


def test_normalized_policy_is_order_independent_except_source_precedence():
    policy = normalized_ranking_policy(
        include_keywords=["Finance", " economics ", "finance"],
        exclude_keywords=["PHD", "studentship"],
        source_preference=["Board B", "board a", "BOARD B"],
    )

    assert policy.include_keywords == ("economics", "finance")
    assert policy.exclude_keywords == ("phd", "studentship")
    assert policy.source_preference == ("board b", "board a")
    with pytest.raises(ValidationError):
        RankingPolicyV1(include_keywords=("Finance",))
    with pytest.raises(ValidationError):
        RankingPolicyV1(include_keywords=("finance", "economics"))


def test_catalog_contract_rejects_rank_stats_identity_and_extra_field_tampering():
    first = _lead(
        title="Lecturer in Economics",
        source="Board A",
        url="https://example.edu/jobs/1",
        record_id="1",
    )
    second = _lead(
        title="Research Fellow",
        source="Board B",
        url="https://example.edu/jobs/2",
        record_id="2",
    )
    payload = merge_lead_catalog([first, second]).model_dump(mode="json")

    with pytest.raises(ValidationError):
        LeadCatalogV1.model_validate({**payload, "vendor_payload": {}})
    with pytest.raises(ValidationError):
        LeadCatalogV1.model_validate({**payload, "catalog_id": "catalog_" + "0" * 32})
    with pytest.raises(ValidationError):
        LeadCatalogV1.model_validate(
            {**payload, "stats": {**payload["stats"], "merged_records": 99}}
        )
    changed_rank = json.loads(json.dumps(payload))
    changed_rank["leads"][0]["rank"] = 2
    with pytest.raises(ValidationError):
        LeadCatalogV1.model_validate(changed_rank)


def test_catalog_schema_is_generated_from_runtime_model():
    stored = json.loads(
        Path("schemas/lead-catalog-v1.schema.json").read_text(encoding="utf-8")
    )

    assert stored == LeadCatalogV1.model_json_schema(mode="validation")
    Draft202012Validator.check_schema(stored)


def test_load_lead_document_accepts_legacy_list_v2_list_and_catalog(tmp_path):
    legacy = {
        "title": "Lecturer in Finance",
        "source_url": "https://example.edu/jobs/finance",
        "description": "Finance role.",
        "published_at": "",
        "source": "Example",
        "source_feed": "",
    }
    legacy_path = tmp_path / "legacy.json"
    legacy_path.write_text(json.dumps([legacy]), encoding="utf-8")
    loaded_legacy = load_lead_document(legacy_path, observed_at=OBSERVED_AT)
    v2_path = tmp_path / "v2.json"
    v2_path.write_text(
        json.dumps([loaded_legacy[0].model_dump(mode="json")]), encoding="utf-8"
    )
    catalog = merge_lead_catalog(loaded_legacy)
    catalog_path = tmp_path / "catalog.json"
    write_lead_catalog(catalog_path, catalog)

    assert load_lead_document(v2_path) == loaded_legacy
    assert load_lead_document(catalog_path) == list(catalog.leads)


def test_catalog_input_restores_excluded_leads_for_filter_changes(tmp_path):
    retained = _lead(
        title="Lecturer in Finance",
        source="Example",
        url="https://example.edu/jobs/finance",
        record_id="finance",
    )
    initially_excluded = _lead(
        title="Lecturer in Economics",
        source="Example",
        url="https://example.edu/jobs/economics",
        record_id="economics",
    )
    filtered = merge_lead_catalog(
        [retained, initially_excluded],
        policy=normalized_ranking_policy(include_keywords=["finance"]),
    )
    catalog_path = tmp_path / "catalog.json"
    write_lead_catalog(catalog_path, filtered)

    restored = load_lead_document(catalog_path)
    reranked = merge_lead_catalog(restored)

    assert {lead.lead_id for lead in restored} == {
        retained.lead_id,
        initially_excluded.lead_id,
    }
    assert reranked.stats.retained_records == 2
    assert reranked.stats.excluded_records == 0


@pytest.mark.parametrize(
    "payload",
    [
        {"results": []},
        {"leads": []},
        {"protocol": "vendor.catalog/v1", "leads": []},
    ],
)
def test_load_lead_document_rejects_unversioned_vendor_envelopes(tmp_path, payload):
    path = tmp_path / "vendor.json"
    path.write_text(json.dumps(payload), encoding="utf-8")

    with pytest.raises(DiscoveryInputError, match="lead JSON list or a versioned"):
        load_lead_document(path)


def test_build_catalog_from_files_is_byte_deterministic_for_v2_inputs(tmp_path):
    first = _lead(
        title="Lecturer in Economics",
        source="Board A",
        url="https://example.edu/jobs/1",
        record_id="a-1",
    )
    second = _lead(
        title="Research Fellow",
        source="Board B",
        url="https://example.edu/jobs/2",
        record_id="b-2",
    )
    path_a = tmp_path / "a.json"
    path_b = tmp_path / "b.json"
    path_a.write_text(json.dumps([first.model_dump(mode="json")]), encoding="utf-8")
    path_b.write_text(json.dumps([second.model_dump(mode="json")]), encoding="utf-8")

    forward = build_catalog_from_files([path_a, path_b])
    reverse = build_catalog_from_files([path_b, path_a])

    assert forward.model_dump_json() == reverse.model_dump_json()


def test_discovery_merge_cli_writes_catalog_and_returns_body_free_agent_json(tmp_path):
    workspace = tmp_path / "workspace"
    leads_dir = workspace / "job_leads"
    leads_dir.mkdir(parents=True)
    first = _lead(
        title="PRIVATE TITLE A",
        source="Board A",
        url="https://example.edu/jobs/1",
        record_id="a-1",
        description="PRIVATE DESCRIPTION A economics",
    )
    second = _lead(
        title="PRIVATE TITLE B economics",
        source="Board B",
        url="https://example.edu/jobs/2",
        record_id="b-2",
        description="PRIVATE DESCRIPTION B",
    )
    (leads_dir / "a.json").write_text(
        json.dumps([first.model_dump(mode="json")]), encoding="utf-8"
    )
    (leads_dir / "b.json").write_text(
        json.dumps([second.model_dump(mode="json")]), encoding="utf-8"
    )

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "merge",
            "--workspace",
            str(workspace),
            "--input",
            "job_leads/b.json",
            "--input",
            "job_leads/a.json",
            "--include",
            "economics",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)
    catalog_path = leads_dir / "catalog.json"

    assert result.exit_code == 0, result.output
    assert result.stdout.count("\n") == 1
    assert payload["operation"] == "discovery.merge"
    assert payload["artifacts"][0]["path"] == "job_leads/catalog.json"
    assert payload["artifacts"][0]["sha256"]
    assert payload["extensions"]["canisend.discovery.retained_records"] == 2
    assert "PRIVATE TITLE" not in result.stdout
    assert "PRIVATE DESCRIPTION" not in result.stdout
    assert str(workspace) not in result.stdout
    stored = LeadCatalogV1.model_validate_json(catalog_path.read_text(encoding="utf-8"))
    assert stored.stats.retained_records == 2


def test_discovery_merge_cli_text_output_uses_relative_artifact_and_safe_counts(tmp_path):
    workspace = tmp_path / "workspace"
    leads = workspace / "job_leads" / "source.json"
    leads.parent.mkdir(parents=True)
    lead = _lead(
        title="PRIVATE ROLE",
        source="Example",
        url="https://example.edu/jobs/1",
        record_id="1",
        description="PRIVATE BODY",
    )
    leads.write_text(json.dumps([lead.model_dump(mode="json")]), encoding="utf-8")

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "merge",
            "--workspace",
            str(workspace),
            "--input",
            "job_leads/source.json",
        ],
    )

    assert result.exit_code == 0, result.output
    assert "Operation: discovery.merge" in result.output
    assert "job_leads/catalog.json" in result.output
    assert "Catalog: 1 input, 0 merged, 1 retained, 0 excluded" in result.output
    assert "PRIVATE ROLE" not in result.output
    assert "PRIVATE BODY" not in result.output
    assert str(workspace) not in result.output


def test_discovery_merge_invalid_input_does_not_replace_existing_catalog(tmp_path):
    workspace = tmp_path / "workspace"
    leads_dir = workspace / "job_leads"
    leads_dir.mkdir(parents=True)
    catalog = leads_dir / "catalog.json"
    catalog.write_text('{"keep":"existing"}\n', encoding="utf-8")
    invalid = leads_dir / "invalid.json"
    invalid.write_text('{"vendor_private":"PRIVATE BODY"}', encoding="utf-8")

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "merge",
            "--workspace",
            str(workspace),
            "--input",
            "job_leads/invalid.json",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 1
    assert payload["operation"] == "discovery.merge"
    assert payload["error"]["code"] == "input.invalid"
    assert "PRIVATE BODY" not in result.stdout
    assert catalog.read_text(encoding="utf-8") == '{"keep":"existing"}\n'


def test_discovery_merge_requires_at_least_one_input_with_one_line_json_error(tmp_path):
    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "merge",
            "--workspace",
            str(tmp_path),
            "--format",
            "json",
        ],
    )

    assert result.exit_code == 1
    assert result.stdout.count("\n") == 1
    assert json.loads(result.stdout)["error"]["code"] == "input.invalid"
