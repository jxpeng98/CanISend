from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator
import pytest
from pydantic import ValidationError

from canisend.discovery.identity import (
    LeadNormalizationError,
    canonicalize_job_url,
    normalize_job_lead,
    stable_lead_id,
)
from canisend.discovery.models import JobLeadV2
from canisend.discovery.store import DiscoveryStoreError, atomic_write_json


OBSERVED_AT = "2026-07-15T10:30:00Z"


def _legacy_lead(**updates):
    lead = {
        "title": "Lecturer in Economics",
        "source_url": "https://EXAMPLE.edu:443/jobs/42?utm_source=alert&id=42#apply",
        "description": "Teach econometrics.",
        "published_at": "2026-07-14T09:00:00Z",
        "source": "Example University",
        "source_feed": "https://example.edu/feed.xml?token=private",
        "source_record_id": "vacancy-42",
        "institution": "Example University",
        "location": "London",
        "deadline": "2026-08-31",
    }
    lead.update(updates)
    return lead


def test_normalize_job_lead_adds_v2_identity_timestamps_and_redacted_provenance():
    lead = normalize_job_lead(
        _legacy_lead(),
        fetched_at=OBSERVED_AT,
        source_type="rss",
        adapter="feed.rss",
    )

    assert lead.schema_version == "2.0.0"
    assert lead.lead_id.startswith("lead_")
    assert lead.identity_method == "source_record_id"
    assert lead.canonical_url == "https://example.edu/jobs/42?id=42"
    assert lead.source_url == lead.canonical_url
    assert lead.source_feed == "https://example.edu/feed.xml?redacted"
    assert lead.fetched_at.isoformat() == "2026-07-15T10:30:00+00:00"
    assert lead.first_seen_at == lead.last_seen_at == lead.fetched_at
    assert lead.provenance[0].source_type == "rss"
    serialized = json.dumps(lead.model_dump(mode="json"))
    assert "private" not in serialized
    assert "utm_source" not in serialized


def test_stable_lead_id_uses_namespaced_source_record_before_url():
    first, first_method = stable_lead_id(
        source="Example University",
        source_record_id="vacancy-42",
        canonical_url="https://example.edu/jobs/old",
        title="Old title",
        institution="Example University",
        deadline="2026-08-31",
    )
    changed_url, changed_method = stable_lead_id(
        source="Example University",
        source_record_id="vacancy-42",
        canonical_url="https://example.edu/jobs/new",
        title="New title",
        institution="Example University",
        deadline="2026-09-01",
    )
    other_source, _ = stable_lead_id(
        source="Other University",
        source_record_id="vacancy-42",
        canonical_url="https://example.edu/jobs/new",
        title="New title",
        institution="Example University",
        deadline="2026-09-01",
    )

    assert first == changed_url
    assert first != other_source
    assert first_method == changed_method == "source_record_id"


def test_normalization_hashes_private_source_record_locators_without_losing_identity():
    private_id = "message-user@example.edu?token=private"
    first = normalize_job_lead(
        _legacy_lead(source_record_id=private_id),
        fetched_at=OBSERVED_AT,
        source_type="email_alert",
    )
    second = normalize_job_lead(
        _legacy_lead(source_record_id=private_id, source_url="https://example.edu/jobs/new"),
        fetched_at=OBSERVED_AT,
        source_type="email_alert",
    )

    assert first.source_record_id.startswith("opaque_")
    assert first.source_record_id == second.source_record_id
    assert first.lead_id == second.lead_id
    assert "example.edu" not in first.source_record_id
    assert "private" not in json.dumps(first.model_dump(mode="json"))


def test_stable_lead_id_falls_back_to_canonical_url_then_normalized_fingerprint():
    canonical = canonicalize_job_url(
        "HTTPS://Example.edu:443/jobs/7?b=2&utm_medium=email&a=1#details"
    )
    url_id, url_method = stable_lead_id(
        source="A",
        source_record_id="",
        canonical_url=canonical,
        title="Title A",
        institution="A",
        deadline="unknown",
    )
    same_url_id, _ = stable_lead_id(
        source="B",
        source_record_id="",
        canonical_url="https://example.edu/jobs/7?a=1&b=2",
        title="Title B",
        institution="B",
        deadline="different",
    )
    fingerprint_id, fingerprint_method = stable_lead_id(
        source="A",
        source_record_id="",
        canonical_url="",
        title="  Lecturer   IN Economics ",
        institution="Example UNIVERSITY",
        deadline=" 2026-08-31 ",
    )
    same_fingerprint_id, _ = stable_lead_id(
        source="B",
        source_record_id="",
        canonical_url="",
        title="lecturer in economics",
        institution="example university",
        deadline="2026-08-31",
    )

    assert canonical == "https://example.edu/jobs/7?a=1&b=2"
    assert url_id == same_url_id
    assert url_method == "canonical_url"
    assert fingerprint_id == same_fingerprint_id
    assert fingerprint_method == "fingerprint"


def test_canonicalize_job_url_preserves_encoded_path_separators_and_ipv6_brackets():
    assert canonicalize_job_url("https://example.edu/jobs/a%2Fb") == (
        "https://example.edu/jobs/a%2Fb"
    )
    assert canonicalize_job_url("https://[2001:db8::1]:443/jobs/1") == (
        "https://[2001:db8::1]/jobs/1"
    )


@pytest.mark.parametrize(
    "url",
    [
        "file:///tmp/jobs.json",
        "https://user:secret@example.edu/jobs/1",
        "https://example.edu:bad/jobs/1",
        "https://example.edu/jobs/1\nInjected",
    ],
)
def test_canonicalize_job_url_rejects_non_http_credentials_and_malformed_urls(url):
    with pytest.raises(LeadNormalizationError):
        canonicalize_job_url(url)


def test_job_lead_v2_rejects_extra_fields_naive_timestamps_and_unsorted_aliases():
    payload = normalize_job_lead(
        _legacy_lead(), fetched_at=OBSERVED_AT, source_type="rss"
    ).model_dump(mode="json")

    with pytest.raises(ValidationError):
        JobLeadV2.model_validate({**payload, "vendor_private": "not allowed"})
    with pytest.raises(ValidationError):
        JobLeadV2.model_validate({**payload, "fetched_at": "2026-07-15T10:30:00"})
    with pytest.raises(ValidationError):
        JobLeadV2.model_validate({**payload, "fetched_at": "1721040000"})
    with pytest.raises(ValidationError):
        JobLeadV2.model_validate(
            {**payload, "canonical_url": "https://user:secret@example.edu/jobs/42"}
        )
    with pytest.raises(ValidationError):
        JobLeadV2.model_validate(
            {
                **payload,
                "alternate_lead_ids": [
                    "lead_ffffffffffffffffffffffffffffffff",
                    "lead_00000000000000000000000000000000",
                ],
            }
        )


def test_job_lead_v2_schema_is_generated_from_the_runtime_model():
    stored = json.loads(Path("schemas/job-lead-v2.schema.json").read_text(encoding="utf-8"))

    assert stored == JobLeadV2.model_json_schema(mode="validation")
    Draft202012Validator.check_schema(stored)


def test_atomic_discovery_write_preserves_previous_file_when_replace_fails(
    tmp_path, monkeypatch
):
    target = tmp_path / "leads.json"
    target.write_text('[{"title":"keep"}]\n', encoding="utf-8")

    def fail_replace(source, destination):
        raise OSError("simulated replace failure")

    monkeypatch.setattr("canisend.discovery.store.os.replace", fail_replace)
    with pytest.raises(DiscoveryStoreError, match="atomically"):
        atomic_write_json(target, [{"title": "new"}])

    assert json.loads(target.read_text(encoding="utf-8")) == [{"title": "keep"}]
    assert list(tmp_path.glob(".leads.json.*.tmp")) == []
