from __future__ import annotations

from datetime import UTC, datetime
from email.message import EmailMessage
import json
import mailbox
from pathlib import Path

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import pytest
from typer.testing import CliRunner

from canisend.cli import app
from canisend.discovery.agent import discovery_local_import_agent_response
from canisend.discovery.catalog import load_lead_document
from canisend.discovery.catalog_models import normalized_ranking_policy
from canisend.discovery.import_models import DiscoveryImportReportV1
from canisend.discovery.local_import import (
    DiscoveryLocalImportInputError,
    import_local_discovery_file,
)
from canisend.discovery.identity import normalize_job_lead
from canisend.discovery.refresh import (
    load_lead_batch,
    refresh_discovery_sources,
)
from canisend.discovery.refresh_models import DiscoverySourceV1, DiscoverySourcesV1
from canisend.discovery.transport import PublicTransport


OBSERVED_AT = datetime(2026, 7, 15, 13, 0, tzinfo=UTC)
LATER_AT = datetime(2026, 7, 15, 14, 0, tzinfo=UTC)
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


def _rss(title: str = "Network Lecturer") -> bytes:
    return (
        "<?xml version='1.0' encoding='UTF-8'?>"
        "<rss version='2.0'><channel><title>Jobs</title><item>"
        f"<title>{title}</title><guid>network-1</guid>"
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
        now=lambda: OBSERVED_AT,
    )


def _write_csv(path: Path) -> None:
    path.write_text(
        "Job Title,Employer,Job Link,Closing Date,Vendor Private\n"
        "Lecturer in Economics,Example University,"
        "https://jobs.example.edu/jobs/1?utm_source=mail&token=PRIVATE,"
        "2026-08-31,PRIVATE CSV BODY\n"
        ",Example University,,,PRIVATE INVALID ROW\n"
        ",,,,\n",
        encoding="utf-8",
    )


def test_csv_aliases_row_report_and_private_safe_artifacts(tmp_path: Path) -> None:
    input_path = tmp_path / "exports" / "jobs.csv"
    input_path.parent.mkdir()
    _write_csv(input_path)

    execution = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Saved University Search",
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "partial"
    assert execution.report.input_records == 3
    assert execution.report.imported_records == 1
    assert execution.report.rejected_records == 1
    assert execution.report.ignored_records == 1
    assert execution.report.issues[0].record_number == 2
    assert execution.report.issues[0].code == "import.row_invalid"
    assert execution.batch_path is not None
    assert execution.catalog is not None
    assert execution.catalog.stats.retained_records == 1

    batch = load_lead_batch(execution.batch_path)
    assert batch.adapter == "local.csv"
    assert batch.source_url == "local-csv"
    assert batch.leads[0].canonical_url == "https://jobs.example.edu/jobs/1"
    assert batch.leads[0].institution == "Example University"
    rendered = "\n".join(
        path.read_text(encoding="utf-8")
        for path in (execution.batch_path, execution.report_path, execution.catalog_path)
        if path is not None
    )
    assert "PRIVATE" not in rendered
    assert "Vendor Private" not in rendered
    assert str(input_path) not in rendered


def test_json_list_preserves_v2_identity_and_rejects_unknown_row_fields(
    tmp_path: Path,
) -> None:
    existing_v2 = normalize_job_lead(
        {
            "title": "Research Fellow",
            "source_url": "https://research.example.edu/jobs/2",
            "source": "Original Board",
            "source_feed": "original-json",
            "source_record_id": "rf-2",
            "source_type": "json",
        },
        fetched_at="2026-07-14T10:00:00Z",
        source_type="json",
        adapter="local.json",
    )
    input_path = tmp_path / "leads.json"
    input_path.write_text(
        json.dumps(
            [
                {
                    "title": "Lecturer in Finance",
                    "source_url": "https://jobs.example.edu/jobs/3",
                    "institution": "Example University",
                },
                existing_v2.model_dump(mode="json"),
                {
                    "title": "PRIVATE VENDOR RECORD",
                    "vendor_payload": {"secret": "PRIVATE JSON BODY"},
                },
            ],
            default=str,
        ),
        encoding="utf-8",
    )

    execution = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Local JSON Export",
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "partial"
    assert execution.report.imported_records == 2
    assert execution.report.rejected_records == 1
    assert execution.report.issues[0].code == "import.row_unknown_field"
    assert execution.batch is not None
    imported_v2 = next(
        lead for lead in execution.batch.leads if lead.lead_id == existing_v2.lead_id
    )
    assert imported_v2.source == "Original Board"
    assert any(
        item.source == "Local JSON Export"
        and item.adapter == "local.json"
        and item.source_feed == "local-json"
        for item in imported_v2.provenance
    )
    rendered = execution.batch.model_dump_json()
    assert "PRIVATE VENDOR RECORD" not in rendered
    assert "PRIVATE JSON BODY" not in rendered


@pytest.mark.parametrize(
    "payload",
    [
        {"results": []},
        {"leads": []},
        {"protocol": "vendor.search/v1", "results": []},
    ],
)
def test_json_vendor_envelopes_fail_closed_and_preserve_catalog(
    tmp_path: Path,
    payload,
) -> None:
    catalog_path = tmp_path / "job_leads" / "catalog.json"
    catalog_path.parent.mkdir(parents=True)
    catalog_path.write_bytes(b"existing catalog sentinel\n")
    input_path = tmp_path / "vendor.json"
    input_path.write_text(json.dumps(payload), encoding="utf-8")

    with pytest.raises(DiscoveryLocalImportInputError) as failure:
        import_local_discovery_file(
            tmp_path,
            input_path,
            source_name="Vendor Export",
            clock=lambda: OBSERVED_AT,
        )

    assert "PRIVATE" not in str(failure.value)
    assert str(tmp_path) not in str(failure.value)
    assert catalog_path.read_bytes() == b"existing catalog sentinel\n"


def test_versioned_lead_batch_json_is_accepted_without_source_override(
    tmp_path: Path,
) -> None:
    csv_path = tmp_path / "source.csv"
    csv_path.write_text(
        "title,url\nReader in Accounting,https://jobs.example.edu/jobs/reader\n",
        encoding="utf-8",
    )
    first = import_local_discovery_file(
        tmp_path / "first",
        csv_path,
        source_name="Accounting Export",
        source_id="accounting-export",
        clock=lambda: OBSERVED_AT,
    )
    assert first.batch is not None
    batch_json = tmp_path / "batch.json"
    batch_json.write_text(
        json.dumps(first.batch.model_dump(mode="json"), default=str),
        encoding="utf-8",
    )

    imported = import_local_discovery_file(
        tmp_path / "second",
        batch_json,
        clock=lambda: LATER_AT,
    )

    assert imported.report.status == "complete"
    assert imported.batch == first.batch
    assert imported.catalog is not None
    assert load_lead_document(imported.batch_path) == list(first.batch.leads)


def test_eml_extracts_only_job_links_and_never_persists_message_metadata(
    tmp_path: Path,
) -> None:
    message = EmailMessage()
    message["From"] = "private.sender@example.com"
    message["To"] = "private.recipient@example.net"
    message["Message-ID"] = "<PRIVATE-MESSAGE-ID@example.com>"
    message["Subject"] = "PRIVATE ALERT SUBJECT"
    message.set_content(
        "Lecturer role https://jobs.example.edu/jobs/lecturer?utm_source=email\n"
        "Privacy https://jobs.example.edu/privacy\n"
        "PRIVATE PLAIN MESSAGE BODY"
    )
    message.add_alternative(
        "<html><body><p>PRIVATE HTML MESSAGE BODY</p>"
        "<a href='https://jobs.example.edu/jobs/lecturer?token=PRIVATE'>"
        "Lecturer in Economics</a>"
        "<a href='https://jobs.example.edu/unsubscribe'>Unsubscribe</a>"
        "<a href='mailto:private.sender@example.com'>Job support</a>"
        "</body></html>",
        subtype="html",
    )
    input_path = tmp_path / "alert.eml"
    input_path.write_bytes(message.as_bytes())

    execution = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="University Email Alerts",
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "complete"
    assert execution.report.imported_records == 1
    assert execution.report.ignored_records >= 2
    assert execution.batch is not None
    lead = execution.batch.leads[0]
    assert lead.title in {"Lecturer role", "Lecturer in Economics"}
    assert lead.canonical_url == "https://jobs.example.edu/jobs/lecturer"
    assert lead.description == ""
    rendered = "\n".join(
        path.read_text(encoding="utf-8")
        for path in (execution.batch_path, execution.report_path, execution.catalog_path)
        if path is not None
    )
    assert "PRIVATE" not in rendered
    assert "private.sender" not in rendered
    assert "private.recipient" not in rendered
    assert "unsubscribe" not in rendered.casefold()
    assert str(input_path) not in rendered


def test_mbox_extracts_job_links_from_multiple_messages(tmp_path: Path) -> None:
    input_path = tmp_path / "alerts.mbox"
    box = mailbox.mbox(input_path)
    try:
        for title, slug in (
            ("Lecturer in Economics", "economics"),
            ("Research Fellow", "research-fellow"),
        ):
            message = EmailMessage()
            message["From"] = "private@example.com"
            message["To"] = "recipient@example.net"
            message.set_content(
                f"{title}: https://jobs.example.edu/jobs/{slug}\nPRIVATE BODY"
            )
            box.add(message)
        box.flush()
    finally:
        box.close()

    execution = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Saved Job Alerts",
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "complete"
    assert execution.report.imported_records == 2
    assert execution.catalog is not None
    assert {lead.title for lead in execution.catalog.leads} == {
        "Lecturer in Economics",
        "Research Fellow",
    }
    assert execution.batch is not None
    assert "private@example.com" not in execution.batch.model_dump_json()
    assert "PRIVATE BODY" not in execution.batch.model_dump_json()


def test_repeated_identical_import_reuses_batch_and_does_not_duplicate_catalog(
    tmp_path: Path,
) -> None:
    input_path = tmp_path / "jobs.csv"
    input_path.write_text(
        "title,url\nLecturer,https://jobs.example.edu/jobs/1\n",
        encoding="utf-8",
    )
    first = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Repeatable Export",
        clock=lambda: OBSERVED_AT,
    )
    batch_bytes = first.batch_path.read_bytes()  # type: ignore[union-attr]
    second = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Repeatable Export",
        clock=lambda: LATER_AT,
    )

    assert second.batch_path.read_bytes() == batch_bytes  # type: ignore[union-attr]
    assert first.catalog is not None and second.catalog is not None
    assert second.catalog.stats.unique_records == 1
    assert second.catalog.catalog_id == first.catalog.catalog_id


def test_local_import_uses_shared_dedupe_filter_and_ranking_pipeline(
    tmp_path: Path,
) -> None:
    input_path = tmp_path / "ranked.csv"
    input_path.write_text(
        "title,url,description\n"
        "Lecturer in Economics,https://jobs.example.edu/jobs/1,Economics role\n"
        "Economics Lecturer,https://jobs.example.edu/jobs/1,Duplicate listing\n"
        "PhD Studentship,https://jobs.example.edu/jobs/2,Economics PhD\n",
        encoding="utf-8",
    )

    execution = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Ranked Export",
        policy=normalized_ranking_policy(
            include_keywords=["economics"],
            exclude_keywords=["phd"],
        ),
        clock=lambda: OBSERVED_AT,
    )

    assert execution.batch is not None
    assert execution.batch.record_count == 3
    assert execution.catalog is not None
    assert execution.catalog.stats.input_records == 3
    assert execution.catalog.stats.merged_records == 1
    assert execution.catalog.stats.retained_records == 1
    assert execution.catalog.stats.excluded_records == 1
    retained = execution.catalog.leads[0]
    assert retained.rank == 1
    assert retained.score == sum(
        reason.score_delta for reason in retained.match_reasons
    )
    assert execution.catalog.excluded[0].reasons[0].code == "filter.exclude_keyword"


def test_network_refresh_retains_valid_local_import_batch(tmp_path: Path) -> None:
    input_path = tmp_path / "jobs.csv"
    input_path.write_text(
        "title,url\nLocal Reader,https://local.example.edu/jobs/reader\n",
        encoding="utf-8",
    )
    imported = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Local Export",
        clock=lambda: OBSERVED_AT,
    )
    assert imported.catalog is not None
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

    assert refreshed.report.status == "complete"
    assert refreshed.catalog is not None
    assert {lead.title for lead in refreshed.catalog.leads} == {
        "Local Reader",
        "Network Lecturer",
    }
    assert refreshed.report.input_records == 2


def test_import_agent_and_cli_json_are_body_free_and_relative(
    tmp_path: Path,
) -> None:
    input_path = tmp_path / "private-export.csv"
    input_path.write_text(
        "title,url,private\nPRIVATE LEAD TITLE,"
        "https://jobs.example.edu/jobs/private,PRIVATE CSV BODY\n",
        encoding="utf-8",
    )

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "import",
            "--workspace",
            str(tmp_path),
            "--input",
            "private-export.csv",
            "--source-name",
            "Saved Search",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 0, result.output
    assert result.stdout.count("\n") == 1
    assert payload["operation"] == "discovery.import"
    assert payload["ok"] is True
    assert {artifact["kind"] for artifact in payload["artifacts"]} == {
        "discovery-catalog",
        "discovery-lead-batch",
        "discovery-import-report",
    }
    assert "PRIVATE LEAD TITLE" not in result.stdout
    assert "PRIVATE CSV BODY" not in result.stdout
    assert str(tmp_path) not in result.stdout

    execution = import_local_discovery_file(
        tmp_path / "agent",
        input_path,
        source_name="Saved Search",
        clock=lambda: OBSERVED_AT,
    )
    response = discovery_local_import_agent_response(tmp_path / "agent", execution)
    assert response.ok is True
    assert "PRIVATE LEAD TITLE" not in response.model_dump_json()


def test_all_invalid_rows_write_failed_report_without_replacing_catalog(
    tmp_path: Path,
) -> None:
    catalog_path = tmp_path / "job_leads" / "catalog.json"
    catalog_path.parent.mkdir(parents=True)
    catalog_path.write_bytes(b"existing catalog sentinel\n")
    input_path = tmp_path / "invalid.csv"
    input_path.write_text(
        "title,url,institution\n,,Example University\n",
        encoding="utf-8",
    )

    execution = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Invalid Export",
        clock=lambda: OBSERVED_AT,
    )
    response = discovery_local_import_agent_response(tmp_path, execution)

    assert execution.report.status == "failed"
    assert execution.report.error_code == "import.no_valid_records"
    assert execution.batch is None
    assert execution.catalog is None
    assert response.ok is False
    assert response.error.code == "source.import_failed"  # type: ignore[union-attr]
    assert catalog_path.read_bytes() == b"existing catalog sentinel\n"
    assert execution.report_path.is_file()


def test_all_invalid_rows_cli_returns_body_free_report_error(tmp_path: Path) -> None:
    input_path = tmp_path / "invalid.csv"
    input_path.write_text(
        "title,url,institution,private\n,,Example University,PRIVATE ROW BODY\n",
        encoding="utf-8",
    )

    result = CliRunner().invoke(
        app,
        [
            "discovery",
            "import",
            "--workspace",
            str(tmp_path),
            "--input",
            "invalid.csv",
            "--source-name",
            "Invalid Export",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 1
    assert result.stdout.count("\n") == 1
    assert payload["error"]["code"] == "source.import_failed"
    assert [artifact["kind"] for artifact in payload["artifacts"]] == [
        "discovery-import-report"
    ]
    assert "PRIVATE ROW BODY" not in result.stdout
    assert str(tmp_path) not in result.stdout


def test_private_or_path_like_source_names_are_rejected(tmp_path: Path) -> None:
    input_path = tmp_path / "jobs.csv"
    input_path.write_text(
        "title,url\nLecturer,https://jobs.example.edu/jobs/1\n",
        encoding="utf-8",
    )

    for source_name in (
        "private@example.com",
        "/Users/private/jobs.csv",
        "token=PRIVATE",
    ):
        with pytest.raises(DiscoveryLocalImportInputError):
            import_local_discovery_file(
                tmp_path,
                input_path,
                source_name=source_name,
                clock=lambda: OBSERVED_AT,
            )

    with pytest.raises(DiscoveryLocalImportInputError):
        import_local_discovery_file(
            tmp_path,
            input_path,
            source_name="Safe Label",
            source_id="token-private",
            clock=lambda: OBSERVED_AT,
        )


def test_credential_like_url_paths_are_rejected_without_persistence(
    tmp_path: Path,
) -> None:
    input_path = tmp_path / "jobs.csv"
    input_path.write_text(
        "title,url\nLecturer,"
        "https://jobs.example.edu/jobs/token=PRIVATE/lecturer\n",
        encoding="utf-8",
    )

    execution = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Safe Export",
        clock=lambda: OBSERVED_AT,
    )

    assert execution.report.status == "failed"
    assert execution.report.rejected_records == 1
    assert execution.batch is None
    assert "PRIVATE" not in execution.report.model_dump_json()


def test_ambiguous_csv_aliases_fail_before_writing_artifacts(tmp_path: Path) -> None:
    input_path = tmp_path / "jobs.csv"
    input_path.write_text(
        "title,job_title,url\nLecturer,Reader,https://jobs.example.edu/jobs/1\n",
        encoding="utf-8",
    )

    with pytest.raises(DiscoveryLocalImportInputError, match="ambiguous"):
        import_local_discovery_file(
            tmp_path,
            input_path,
            source_name="Ambiguous Export",
            clock=lambda: OBSERVED_AT,
        )

    assert not (tmp_path / "job_leads").exists()


def test_imports_directory_symlink_is_rejected_before_writing(
    tmp_path: Path,
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()
    lead_root = tmp_path / "workspace" / "job_leads"
    lead_root.mkdir(parents=True)
    try:
        (lead_root / "imports").symlink_to(outside, target_is_directory=True)
    except OSError:
        pytest.skip("directory symlinks are unavailable")
    input_path = tmp_path / "jobs.csv"
    input_path.write_text(
        "title,url\nLecturer,https://jobs.example.edu/jobs/1\n",
        encoding="utf-8",
    )

    with pytest.raises(DiscoveryLocalImportInputError, match="symbolic links"):
        import_local_discovery_file(
            tmp_path / "workspace",
            input_path,
            source_name="Safe Export",
            clock=lambda: OBSERVED_AT,
        )

    assert list(outside.iterdir()) == []


def test_cli_invalid_envelope_and_unexpected_failure_are_private_safe(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    input_path = tmp_path / "vendor.json"
    input_path.write_text(
        json.dumps({"results": [], "private": "PRIVATE VENDOR BODY"}),
        encoding="utf-8",
    )
    invalid = CliRunner().invoke(
        app,
        [
            "discovery",
            "import",
            "--workspace",
            str(tmp_path),
            "--input",
            "vendor.json",
            "--source-name",
            "Vendor Export",
            "--format",
            "json",
        ],
    )
    payload = json.loads(invalid.stdout)

    assert invalid.exit_code == 1
    assert invalid.stdout.count("\n") == 1
    assert payload["error"]["code"] == "input.invalid"
    assert "PRIVATE VENDOR BODY" not in invalid.stdout
    assert str(tmp_path) not in invalid.stdout

    def fail_import(*args, **kwargs):
        raise RuntimeError("PRIVATE UNEXPECTED IMPORT DETAIL")

    monkeypatch.setattr("canisend.cli.import_local_discovery_file", fail_import)
    unexpected = CliRunner().invoke(
        app,
        [
            "discovery",
            "import",
            "--workspace",
            str(tmp_path),
            "--input",
            "vendor.json",
            "--source-name",
            "Vendor Export",
        ],
    )

    assert unexpected.exit_code == 2
    assert "The discovery import operation failed unexpectedly." in unexpected.output
    assert "PRIVATE UNEXPECTED IMPORT DETAIL" not in unexpected.output
    assert str(tmp_path) not in unexpected.output


def test_import_report_contract_and_schema_reject_tampering(tmp_path: Path) -> None:
    input_path = tmp_path / "jobs.csv"
    input_path.write_text(
        "title,url\nLecturer,https://jobs.example.edu/jobs/1\n",
        encoding="utf-8",
    )
    execution = import_local_discovery_file(
        tmp_path,
        input_path,
        source_name="Schema Export",
        clock=lambda: OBSERVED_AT,
    )

    with pytest.raises(ValidationError):
        DiscoveryImportReportV1.model_validate(
            {
                **execution.report.model_dump(mode="json"),
                "imported_records": 99,
            }
        )
    stored = json.loads(
        Path("schemas/discovery-import-report-v1.schema.json").read_text(
            encoding="utf-8"
        )
    )
    assert stored == DiscoveryImportReportV1.model_json_schema(mode="validation")
    Draft202012Validator.check_schema(stored)
