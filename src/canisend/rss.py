from __future__ import annotations

from dataclasses import asdict, dataclass
from html import unescape
import json
from pathlib import Path
import re
from urllib.request import urlopen
import xml.etree.ElementTree as ET


@dataclass(frozen=True)
class JobLead:
    title: str
    source_url: str
    description: str
    published_at: str
    source: str
    source_feed: str


def parse_jobs_ac_uk_rss(xml_text: str, feed_url: str = "") -> list[JobLead]:
    root = ET.fromstring(xml_text)
    leads: list[JobLead] = []

    for item in root.findall("./channel/item"):
        title = _child_text(item, "title")
        link = _child_text(item, "link")
        description = _clean_description(_child_text(item, "description"))
        published_at = _child_text(item, "pubDate")
        if not title and not link:
            continue
        leads.append(
            JobLead(
                title=title,
                source_url=link,
                description=description,
                published_at=published_at,
                source="jobs.ac.uk",
                source_feed=feed_url,
            )
        )

    return leads


def filter_job_leads(
    leads: list[JobLead],
    *,
    include_keywords: list[str] | None = None,
    exclude_keywords: list[str] | None = None,
) -> list[JobLead]:
    includes = [keyword.lower() for keyword in include_keywords or [] if keyword.strip()]
    excludes = [keyword.lower() for keyword in exclude_keywords or [] if keyword.strip()]
    filtered: list[JobLead] = []

    for lead in leads:
        haystack = f"{lead.title}\n{lead.description}".lower()
        if includes and not any(keyword in haystack for keyword in includes):
            continue
        if excludes and any(keyword in haystack for keyword in excludes):
            continue
        filtered.append(lead)

    return filtered


def fetch_rss_text(feed_url: str, timeout_seconds: int = 30) -> str:
    with urlopen(feed_url, timeout=timeout_seconds) as response:
        return response.read().decode("utf-8")


def write_job_leads(path: Path, leads: list[JobLead]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps([asdict(lead) for lead in leads], indent=2) + "\n", encoding="utf-8")
    return path


def _child_text(item: ET.Element, tag: str) -> str:
    child = item.find(tag)
    if child is None or child.text is None:
        return ""
    return child.text.strip()


def _clean_description(value: str) -> str:
    without_tags = re.sub(r"<[^>]+>", " ", value)
    compact = re.sub(r"\s+", " ", without_tags)
    return unescape(compact).strip()
