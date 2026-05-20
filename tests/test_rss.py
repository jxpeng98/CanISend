import json

from typer.testing import CliRunner

from canisend.cli import app
from canisend.rss import filter_job_leads, parse_jobs_ac_uk_rss


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


def test_parse_jobs_ac_uk_rss_extracts_job_leads():
    leads = parse_jobs_ac_uk_rss(SAMPLE_RSS, feed_url="https://www.jobs.ac.uk/jobs/rss")

    assert len(leads) == 2
    assert leads[0].title == "Lecturer in Economics"
    assert leads[0].source_url == "https://www.jobs.ac.uk/job/ABC123/lecturer-in-economics"
    assert leads[0].source == "jobs.ac.uk"
    assert leads[0].source_feed == "https://www.jobs.ac.uk/jobs/rss"
    assert "econometrics" in leads[0].description


def test_filter_job_leads_applies_include_and_exclude_keywords():
    leads = parse_jobs_ac_uk_rss(SAMPLE_RSS)

    filtered = filter_job_leads(leads, include_keywords=["finance", "economics"], exclude_keywords=["phd"])

    assert [lead.title for lead in filtered] == ["Lecturer in Economics"]


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
