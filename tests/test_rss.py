import hashlib
import json
import re

import pytest
from typer.testing import CliRunner

from canisend.cli import app
from canisend.rss import (
    JobFeedError,
    fetch_rss_text,
    filter_job_leads,
    parse_job_feed,
    parse_jobs_ac_uk_rss,
    write_job_leads,
)


ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")


def _strip_ansi(value: str) -> str:
    return ANSI_ESCAPE_RE.sub("", value)


@pytest.fixture(autouse=True)
def _public_dns(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        "canisend.rss.resolve_host_addresses",
        lambda hostname: ("93.184.216.34",),
    )


SAMPLE_RSS = """<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>jobs.ac.uk sample</title>
    <item>
      <title>Lecturer in Economics</title>
      <link>https://www.jobs.ac.uk/job/ABC123/lecturer-in-economics</link>
      <description><![CDATA[Teach econometrics and finance. Permanent academic role.]]></description>
      <pubDate>Mon, 04 May 2026 09:00:00 GMT</pubDate>
    </item>
    <item>
      <title>PhD Studentship in Finance</title>
      <link>https://www.jobs.ac.uk/job/DEF456/phd-studentship-in-finance</link>
      <description><![CDATA[Funded PhD position.]]></description>
      <pubDate>Mon, 04 May 2026 10:00:00 GMT</pubDate>
    </item>
  </channel>
</rss>
"""


SAMPLE_ATOM = """<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Academic jobs sample</title>
  <entry>
    <title>Research Fellow in Econometrics</title>
    <link rel="alternate" href="https://example.edu/jobs/research-fellow" />
    <summary type="html">&lt;p&gt;Research on applied econometrics.&lt;/p&gt;</summary>
    <published>2026-05-01T08:00:00Z</published>
    <updated>2026-05-04T09:00:00Z</updated>
  </entry>
  <entry>
    <title>Lecturer in Finance</title>
    <link href="https://example.edu/jobs/lecturer-finance" />
    <content type="html">&lt;p&gt;Teach finance and supervise students.&lt;/p&gt;</content>
    <published>2026-05-03T10:00:00Z</published>
  </entry>
</feed>
"""


SAMPLE_RSS1 = """<?xml version="1.0" encoding="UTF-8"?>
<rdf:RDF
    xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
    xmlns="http://purl.org/rss/1.0/"
    xmlns:dc="http://purl.org/dc/elements/1.1/">
  <channel rdf:about="https://example.edu/jobs.rdf">
    <title>Example academic jobs</title>
    <link>https://example.edu/jobs</link>
    <description>Academic vacancies</description>
  </channel>
  <item rdf:about="https://example.edu/jobs/reader">
    <title>Reader in Economics</title>
    <link>https://example.edu/jobs/reader</link>
    <description><![CDATA[Research and teach economics.]]></description>
    <dc:date>2026-05-02T12:00:00Z</dc:date>
  </item>
</rdf:RDF>
"""


class FakeFeedResponse:
    def __init__(
        self,
        body: bytes,
        *,
        content_type: str = "application/rss+xml",
        content_length: str | None = None,
        final_url: str = "https://example.edu/jobs.xml",
    ) -> None:
        self.body = body
        self.headers = {"Content-Type": content_type}
        if content_length is not None:
            self.headers["Content-Length"] = content_length
        self.final_url = final_url
        self.read_sizes: list[int] = []

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        return None

    def geturl(self) -> str:
        return self.final_url

    def read(self, size: int = -1) -> bytes:
        self.read_sizes.append(size)
        return self.body if size < 0 else self.body[:size]


def test_parse_jobs_ac_uk_rss_extracts_job_leads():
    leads = parse_jobs_ac_uk_rss(SAMPLE_RSS, feed_url="https://www.jobs.ac.uk/jobs/rss")

    assert len(leads) == 2
    assert leads[0].title == "Lecturer in Economics"
    assert leads[0].source_url == "https://www.jobs.ac.uk/job/ABC123/lecturer-in-economics"
    assert leads[0].source == "jobs.ac.uk"
    assert leads[0].source_feed == "https://www.jobs.ac.uk/jobs/rss"
    assert "econometrics" in leads[0].description


def test_parse_job_feed_maps_generic_rss_source_metadata():
    leads = parse_job_feed(
        SAMPLE_RSS,
        feed_url="https://example.edu/jobs.xml",
        source_name="Example University",
    )

    assert len(leads) == 2
    assert leads[0].title == "Lecturer in Economics"
    assert leads[0].source == "Example University"
    assert leads[0].source_feed == "https://example.edu/jobs.xml"


def test_parse_job_feed_extracts_namespaced_atom_entries_with_fallback_fields():
    leads = parse_job_feed(
        SAMPLE_ATOM,
        feed_url="https://example.edu/jobs.atom",
        source_name="Example Academic Jobs",
    )

    assert len(leads) == 2
    assert leads[0].source_url == "https://example.edu/jobs/research-fellow"
    assert leads[0].description == "Research on applied econometrics."
    assert leads[0].published_at == "2026-05-01T08:00:00Z"
    assert leads[1].source_url == "https://example.edu/jobs/lecturer-finance"
    assert leads[1].description == "Teach finance and supervise students."
    assert leads[1].published_at == "2026-05-03T10:00:00Z"


def test_parse_job_feed_extracts_rss_guid_and_atom_id_as_source_record_ids():
    rss = """<rss version="2.0"><channel><item><title>Role</title>
    <link>https://example.edu/jobs/1</link><guid isPermaLink="false">role-1</guid>
    </item></channel></rss>"""
    atom = """<feed xmlns="http://www.w3.org/2005/Atom"><entry><title>Role</title>
    <id>tag:example.edu,2026:role-1</id><link href="https://example.edu/jobs/1" />
    </entry></feed>"""

    assert parse_job_feed(rss)[0].source_record_id == "role-1"
    assert parse_job_feed(atom)[0].source_record_id == "tag:example.edu,2026:role-1"
    assert parse_job_feed(rss)[0].source_type == "rss"
    assert parse_job_feed(atom)[0].source_type == "atom"


def test_parse_job_feed_extracts_rss1_rdf_items():
    leads = parse_job_feed(
        SAMPLE_RSS1,
        feed_url="https://example.edu/jobs.rdf",
        source_name="Example University",
    )

    assert len(leads) == 1
    assert leads[0].title == "Reader in Economics"
    assert leads[0].source_url == "https://example.edu/jobs/reader"
    assert leads[0].description == "Research and teach economics."
    assert leads[0].published_at == "2026-05-02T12:00:00Z"


@pytest.mark.parametrize(
    ("xml_text", "message"),
    [
        ("<rss>", "Malformed job feed XML"),
        ("<html><body>Not a feed</body></html>", "Unsupported job feed format"),
        ("<rss version='1.0'><channel /></rss>", "Unsupported job feed format"),
        ("<rss version='2evil'><channel /></rss>", "Unsupported job feed format"),
        ("<rss version='2.0'><item /></rss>", "missing channel element"),
    ],
)
def test_parse_job_feed_rejects_malformed_or_unsupported_xml(xml_text, message):
    with pytest.raises(JobFeedError, match=message):
        parse_job_feed(xml_text)


def test_parse_job_feed_redacts_query_from_source_provenance():
    leads = parse_job_feed(
        SAMPLE_RSS,
        feed_url="https://example.edu/jobs.xml?token=secret&session=abc#latest",
        source_name="Example University",
    )

    assert leads[0].source_feed == "https://example.edu/jobs.xml?redacted"
    assert "secret" not in leads[0].source_feed


def test_filter_job_leads_applies_include_and_exclude_keywords():
    leads = parse_jobs_ac_uk_rss(SAMPLE_RSS)

    filtered = filter_job_leads(leads, include_keywords=["finance", "economics"], exclude_keywords=["phd"])

    assert [lead.title for lead in filtered] == ["Lecturer in Economics"]


def test_fetch_rss_text_uses_request_user_agent_and_injected_opener():
    response = FakeFeedResponse(SAMPLE_RSS.encode("utf-8"))

    def opener(request, timeout):
        assert request.full_url == "https://example.edu/jobs.xml"
        assert dict(request.header_items())["User-agent"] == "CanISend/0.2 job-feed-fetch"
        assert timeout == 7
        return response

    result = fetch_rss_text(
        "https://example.edu/jobs.xml",
        7,
        opener=opener,
    )

    assert "Lecturer in Economics" in result
    assert response.read_sizes == [2_000_001]


@pytest.mark.parametrize(
    "feed_url",
    [
        "file:///tmp/jobs.xml",
        "https://user:secret@example.edu/jobs.xml",
        "http://localhost/jobs.xml",
        "http://jobs.local/jobs.xml",
        "http://127.0.0.1/jobs.xml",
        "http://10.0.0.8/jobs.xml",
        "http://[::1]/jobs.xml",
        "http://[::1/jobs.xml",
    ],
)
def test_fetch_rss_text_rejects_unsafe_source_urls_before_opening(feed_url):
    def opener(*args, **kwargs):
        raise AssertionError("unsafe URL reached the opener")

    with pytest.raises(JobFeedError):
        fetch_rss_text(feed_url, opener=opener)


def test_fetch_rss_text_revalidates_redirect_destination_before_reading():
    response = FakeFeedResponse(
        SAMPLE_RSS.encode("utf-8"),
        final_url="http://127.0.0.1/internal/jobs.xml",
    )

    with pytest.raises(JobFeedError, match="publicly routable"):
        fetch_rss_text(
            "https://example.edu/jobs.xml",
            opener=lambda request, timeout: response,
        )

    assert response.read_sizes == []


@pytest.mark.parametrize(
    "addresses",
    [
        ("127.0.0.1",),
        ("169.254.10.20",),
        ("172.16.0.5",),
        ("93.184.216.34", "192.168.1.20"),
        ("::1",),
    ],
)
def test_fetch_rss_text_rejects_non_public_resolved_addresses_before_opening(addresses):
    def opener(*args, **kwargs):
        raise AssertionError("unsafe resolved address reached the opener")

    with pytest.raises(JobFeedError, match="resolved.*publicly routable"):
        fetch_rss_text(
            "https://example.edu/jobs.xml",
            opener=opener,
            resolver=lambda hostname: addresses,
        )


def test_fetch_rss_text_revalidates_redirect_hostname_resolution_before_reading():
    response = FakeFeedResponse(
        SAMPLE_RSS.encode("utf-8"),
        final_url="https://redirect.example/jobs.xml",
    )

    def resolver(hostname: str):
        return ("10.0.0.9",) if hostname == "redirect.example" else ("93.184.216.34",)

    with pytest.raises(JobFeedError, match="resolved.*publicly routable"):
        fetch_rss_text(
            "https://example.edu/jobs.xml",
            opener=lambda request, timeout: response,
            resolver=resolver,
        )

    assert response.read_sizes == []


@pytest.mark.parametrize(
    "response",
    [
        FakeFeedResponse(b"<rss />", content_length="101"),
        FakeFeedResponse(b"x" * 101, content_length="10"),
    ],
)
def test_fetch_rss_text_rejects_declared_or_actual_oversized_response(response):
    with pytest.raises(JobFeedError, match="exceeds the configured limit"):
        fetch_rss_text(
            "https://example.edu/jobs.xml",
            opener=lambda request, timeout: response,
            max_bytes=100,
        )


def test_fetch_rss_text_rejects_non_feed_content_type():
    response = FakeFeedResponse(
        SAMPLE_RSS.encode("utf-8"),
        content_type="text/html; charset=utf-8",
    )

    with pytest.raises(JobFeedError, match="did not return XML, RSS, or Atom"):
        fetch_rss_text(
            "https://example.edu/jobs.xml",
            opener=lambda request, timeout: response,
        )


def test_fetch_rss_text_decodes_charset_from_content_type_header():
    body = "<rss version='2.0'><channel><title>Café jobs</title></channel></rss>".encode(
        "iso-8859-1"
    )
    response = FakeFeedResponse(
        body,
        content_type="application/rss+xml; charset=iso-8859-1",
    )

    result = fetch_rss_text(
        "https://example.edu/jobs.xml",
        opener=lambda request, timeout: response,
    )

    assert "Café jobs" in result


def test_fetch_rss_text_decodes_charset_from_xml_declaration():
    body = (
        "<?xml version='1.0' encoding='windows-1252'?>"
        "<rss version='2.0'><channel><title>Research – café</title></channel></rss>"
    ).encode("windows-1252")
    response = FakeFeedResponse(body, content_type="application/xml")

    result = fetch_rss_text(
        "https://example.edu/jobs.xml",
        opener=lambda request, timeout: response,
    )

    assert "Research – café" in result


def test_fetch_jobs_ac_uk_cli_reads_local_rss_and_writes_filtered_json(tmp_path):
    rss_file = tmp_path / "jobs.xml"
    rss_file.write_text(SAMPLE_RSS)
    output = tmp_path / "leads.json"
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "fetch-jobs-ac-uk",
            "--rss-file",
            str(rss_file),
            "--output",
            str(output),
            "--include",
            "economics",
            "--exclude",
            "phd",
        ],
    )

    assert result.exit_code == 0
    leads = json.loads(output.read_text())
    assert len(leads) == 1
    assert leads[0]["title"] == "Lecturer in Economics"
    assert leads[0]["source"] == "jobs.ac.uk"
    assert leads[0]["schema_version"] == "2.0.0"
    assert leads[0]["lead_id"].startswith("lead_")
    assert leads[0]["canonical_url"] == leads[0]["source_url"]
    assert leads[0]["provenance"][0]["source_type"] == "rss"


def test_write_job_leads_is_stable_for_one_observation_timestamp(tmp_path):
    leads = parse_jobs_ac_uk_rss(SAMPLE_RSS, feed_url="https://example.edu/feed.xml")
    first = tmp_path / "first.json"
    second = tmp_path / "second.json"

    write_job_leads(first, leads, fetched_at="2026-07-15T10:30:00Z")
    write_job_leads(second, list(reversed(leads)), fetched_at="2026-07-15T10:30:00Z")
    first_payload = json.loads(first.read_text(encoding="utf-8"))
    second_payload = json.loads(second.read_text(encoding="utf-8"))

    assert {lead["lead_id"] for lead in first_payload} == {
        lead["lead_id"] for lead in second_payload
    }
    assert {lead["fetched_at"] for lead in first_payload} == {"2026-07-15T10:30:00Z"}


def test_fetch_job_feed_cli_reads_atom_and_uses_source_slug_for_default_output(tmp_path):
    workspace = tmp_path / "workspace"
    atom_file = tmp_path / "jobs.atom"
    atom_file.write_text(SAMPLE_ATOM)
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "fetch-job-feed",
            "--workspace",
            str(workspace),
            "--source-name",
            "Example Academic Jobs",
            "--rss-file",
            str(atom_file),
            "--include",
            "econometrics",
            "--limit",
            "1",
        ],
    )

    output = workspace / "job_leads" / "example-academic-jobs.json"
    assert result.exit_code == 0
    assert output.exists()
    leads = json.loads(output.read_text())
    assert [lead["title"] for lead in leads] == ["Research Fellow in Econometrics"]
    assert leads[0]["source"] == "Example Academic Jobs"


@pytest.mark.parametrize(
    ("source_name", "message"),
    [
        ("   ", "--source-name must not be empty"),
        ("---", "--source-name must contain a letter or number"),
    ],
)
def test_fetch_job_feed_cli_rejects_invalid_source_name(tmp_path, source_name, message):
    rss_file = tmp_path / "jobs.xml"
    rss_file.write_text(SAMPLE_RSS)
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "fetch-job-feed",
            "--source-name",
            source_name,
            "--rss-file",
            str(rss_file),
        ],
    )

    assert result.exit_code != 0
    assert message in _strip_ansi(result.output)


@pytest.mark.parametrize(
    "command",
    [
        ["fetch-jobs-ac-uk"],
        ["fetch-job-feed", "--source-name", "Example Jobs"],
    ],
)
def test_feed_cli_rejects_negative_limit(tmp_path, command):
    rss_file = tmp_path / "jobs.xml"
    rss_file.write_text(SAMPLE_RSS)
    runner = CliRunner()

    result = runner.invoke(
        app,
        [*command, "--rss-file", str(rss_file), "--limit", "-1"],
    )

    assert result.exit_code != 0
    assert "--limit must be zero or greater" in _strip_ansi(result.output)


@pytest.mark.parametrize(
    "command",
    [
        ["fetch-jobs-ac-uk"],
        ["fetch-job-feed", "--source-name", "Example Jobs"],
    ],
)
def test_feed_cli_rejects_local_and_remote_inputs_together(tmp_path, command):
    rss_file = tmp_path / "jobs.xml"
    rss_file.write_text(SAMPLE_RSS, encoding="utf-8")
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            *command,
            "--rss-file",
            str(rss_file),
            "--feed-url",
            "https://example.edu/jobs.xml",
        ],
    )

    assert result.exit_code != 0
    assert "either --feed-url or --rss-file" in _strip_ansi(result.output)


def test_fetch_job_feed_cli_supports_non_ascii_source_names(tmp_path):
    workspace = tmp_path / "workspace"
    rss_file = tmp_path / "jobs.xml"
    rss_file.write_text(SAMPLE_RSS, encoding="utf-8")
    source_name = "高校职位"
    digest = hashlib.sha256(source_name.encode("utf-8")).hexdigest()[:10]
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "fetch-job-feed",
            "--workspace",
            str(workspace),
            "--source-name",
            source_name,
            "--rss-file",
            str(rss_file),
        ],
    )

    output = workspace / "job_leads" / f"source-{digest}.json"
    assert result.exit_code == 0
    assert output.exists()
    assert json.loads(output.read_text(encoding="utf-8"))[0]["source"] == source_name


def test_fetch_job_feed_cli_refuses_default_slug_collision(tmp_path):
    workspace = tmp_path / "workspace"
    rss_file = tmp_path / "jobs.xml"
    rss_file.write_text(SAMPLE_RSS, encoding="utf-8")
    runner = CliRunner()

    first = runner.invoke(
        app,
        [
            "fetch-job-feed",
            "--workspace",
            str(workspace),
            "--source-name",
            "A/B",
            "--rss-file",
            str(rss_file),
        ],
    )
    second = runner.invoke(
        app,
        [
            "fetch-job-feed",
            "--workspace",
            str(workspace),
            "--source-name",
            "A B",
            "--rss-file",
            str(rss_file),
        ],
    )

    assert first.exit_code == 0
    assert second.exit_code != 0
    assert "already belongs to source A/B" in second.output


def test_fetch_job_feed_cli_does_not_overwrite_output_on_parse_failure(tmp_path):
    invalid_feed = tmp_path / "invalid.xml"
    invalid_feed.write_text("<jobs><job /></jobs>", encoding="utf-8")
    output = tmp_path / "leads.json"
    output.write_text('[{"title":"keep"}]\n', encoding="utf-8")
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "fetch-job-feed",
            "--source-name",
            "Example Jobs",
            "--rss-file",
            str(invalid_feed),
            "--output",
            str(output),
        ],
    )

    assert result.exit_code != 0
    assert "Unsupported job feed format" in result.output
    assert json.loads(output.read_text(encoding="utf-8")) == [{"title": "keep"}]
