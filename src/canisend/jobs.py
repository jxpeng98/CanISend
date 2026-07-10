from datetime import UTC, datetime
import json
from pathlib import Path
import re
from typing import Any

import yaml

from canisend.job_import import JobImportError, fetch_advert_from_url, import_advert_file


class JobMetadataError(ValueError):
    """Raised when a job workspace has no usable metadata document."""


def slugify(value: str) -> str:
    normalized = value.strip().lower()
    normalized = re.sub(r"[^a-z0-9]+", "-", normalized)
    return normalized.strip("-")


def build_job_slug(deadline: str, institution: str, title: str) -> str:
    return f"{deadline}_{slugify(institution)}_{slugify(title)}"


def create_job(
    *,
    jobs_dir: Path,
    title: str,
    institution: str,
    deadline: str,
    source_url: str = "",
    advert_file: Path | None = None,
    fetch_url: bool = False,
    english_variant: str = "",
    writing_style: str = "",
) -> Path:
    advert_text = ""
    status = "new"
    notes = ""
    if advert_file is not None and fetch_url:
        raise ValueError("Use either --advert-file or --fetch-url, not both.")
    if advert_file is not None:
        try:
            imported = import_advert_file(advert_file)
        except JobImportError as exc:
            raise ValueError(str(exc)) from exc
        advert_text = imported.text
        status = imported.status
        notes = imported.notes
        source_url = imported.metadata_source_url or source_url
    elif fetch_url:
        try:
            imported = fetch_advert_from_url(source_url)
        except JobImportError as exc:
            raise ValueError(str(exc)) from exc
        advert_text = imported.text
        status = imported.status
        notes = imported.notes
        source_url = imported.metadata_source_url or source_url
    elif source_url.strip():
        advert_text = _source_url_stub(source_url)

    return _write_job_workspace(
        jobs_dir=jobs_dir,
        title=title,
        institution=institution,
        deadline=deadline,
        source_url=source_url,
        advert_text=advert_text,
        status=status,
        notes=notes,
        english_variant=english_variant,
        writing_style=writing_style,
    )


def create_job_from_lead(
    *,
    leads_file: Path,
    lead_index: int,
    jobs_dir: Path,
    institution: str,
    deadline: str = "unknown",
    title: str | None = None,
    english_variant: str = "",
    writing_style: str = "",
) -> Path:
    lead = _load_lead(leads_file, lead_index)
    lead_title = (title or str(lead.get("title", ""))).strip()
    if not lead_title:
        raise ValueError("Selected lead does not contain a title; provide --title.")
    if not institution.strip():
        raise ValueError("Provide --institution when creating a job from a feed lead.")

    return _write_job_workspace(
        jobs_dir=jobs_dir,
        title=lead_title,
        institution=institution,
        deadline=deadline,
        source_url=str(lead.get("source_url", "")),
        advert_text=_lead_advert_markdown(lead, lead_title),
        status="lead_imported",
        notes="Created from a feed lead only; paste or import and review the full advert.",
        english_variant=english_variant,
        writing_style=writing_style,
    )


def _write_job_workspace(
    *,
    jobs_dir: Path,
    title: str,
    institution: str,
    deadline: str,
    source_url: str,
    advert_text: str,
    status: str,
    notes: str,
    english_variant: str,
    writing_style: str,
) -> Path:
    job_id = build_job_slug(deadline, institution, title)
    job_dir = jobs_dir / job_id
    job_dir.mkdir(parents=True, exist_ok=False)
    (job_dir / "job_advert.md").write_text(advert_text, encoding="utf-8")

    now = datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    metadata = {
        "id": job_id,
        "title": title,
        "institution": institution,
        "department": "",
        "location": "",
        "deadline": deadline,
        "source_url": source_url,
        "status": status,
        "english_variant": normalize_english_variant(english_variant),
        "writing_style": normalize_writing_style(writing_style),
        "created_at": now,
        "updated_at": now,
        "notes": notes,
    }
    (job_dir / "job.yaml").write_text(yaml.safe_dump(metadata, sort_keys=False), encoding="utf-8")

    return job_dir


def normalize_english_variant(value: str) -> str:
    normalized = value.strip().lower().replace(".", "")
    normalized = re.sub(r"[\s-]+", "_", normalized)
    if not normalized:
        return "needs_confirmation"
    aliases = {
        "british": "uk",
        "british_english": "uk",
        "en_gb": "uk",
        "gb": "uk",
        "uk_english": "uk",
        "american": "us",
        "american_english": "us",
        "en_us": "us",
        "usa": "us",
        "us_english": "us",
        "needs_confirmation": "needs_confirmation",
        "need_confirmation": "needs_confirmation",
        "confirm": "needs_confirmation",
    }
    normalized = aliases.get(normalized, normalized)
    if normalized not in {"uk", "us", "needs_confirmation"}:
        raise ValueError("English variant must be uk, us, or needs_confirmation.")
    return normalized


def normalize_writing_style(value: str) -> str:
    return value.strip() or "needs_confirmation"


def load_job_metadata(job_dir: Path) -> dict[str, Any]:
    metadata_path = job_dir / "job.yaml"
    if not metadata_path.is_file():
        raise JobMetadataError("job.yaml is missing")
    try:
        loaded = yaml.safe_load(metadata_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, yaml.YAMLError) as exc:
        raise JobMetadataError("job.yaml is not valid YAML") from exc
    if not isinstance(loaded, dict):
        raise JobMetadataError("job.yaml must contain a mapping")

    metadata = dict(loaded)
    metadata.setdefault("id", job_dir.name)
    metadata.setdefault("source_url", "")
    metadata.setdefault("english_variant", "needs_confirmation")
    metadata.setdefault("writing_style", "needs_confirmation")
    for field in ("id", "title", "institution", "deadline", "status"):
        value = metadata.get(field)
        if value is None or isinstance(value, (dict, list)) or not str(value).strip():
            raise JobMetadataError(f"job.yaml field is missing or invalid: {field}")
        metadata[field] = str(value).strip()
    metadata["english_variant"] = str(metadata.get("english_variant") or "needs_confirmation").strip()
    metadata["writing_style"] = str(metadata.get("writing_style") or "needs_confirmation").strip()
    return metadata


def job_advert_is_stub(text: str) -> bool:
    stripped = text.strip()
    if not stripped:
        return True
    lowered = stripped.lower()
    if "# job advert pending import" in lowered:
        return True
    if "the full advert still needs manual paste" in lowered:
        return True
    if "rss lead only" not in lowered and "feed lead only" not in lowered:
        return False

    full_advert_match = re.search(
        r"^##\s+Full Advert\s*$\n(?P<body>.*?)(?=^##\s+|\Z)",
        text,
        flags=re.IGNORECASE | re.MULTILINE | re.DOTALL,
    )
    if full_advert_match is None:
        return True
    full_advert = full_advert_match.group("body").strip().lower()
    return not full_advert or "paste the full advert manually here" in full_advert


def _source_url_stub(source_url: str) -> str:
    return "\n".join(
        [
            "# Job Advert Pending Import",
            "",
            f"Source URL saved: {source_url}",
            "",
            "The full advert still needs manual paste, PDF import, or explicit fetch before final parsing.",
            "",
        ]
    )


def _load_lead(leads_file: Path, lead_index: int) -> dict[str, Any]:
    if lead_index < 0:
        raise ValueError("Lead index must be zero or greater.")
    leads = json.loads(leads_file.read_text(encoding="utf-8"))
    if not isinstance(leads, list):
        raise ValueError("Lead file must contain a JSON list.")
    try:
        lead = leads[lead_index]
    except IndexError as exc:
        raise ValueError(f"Lead index {lead_index} is out of range.") from exc
    if not isinstance(lead, dict):
        raise ValueError(f"Lead index {lead_index} is not an object.")
    return lead


def list_jobs(jobs_dir: Path) -> list[dict[str, Any]]:
    if not jobs_dir.exists():
        return []
    entries: list[dict[str, Any]] = []
    for job_dir in sorted(jobs_dir.iterdir()):
        if not job_dir.is_dir():
            continue
        yaml_path = job_dir / "job.yaml"
        if not yaml_path.exists():
            continue
        try:
            metadata = yaml.safe_load(yaml_path.read_text(encoding="utf-8"))
        except yaml.YAMLError:
            continue
        if not isinstance(metadata, dict):
            continue
        entries.append({
            "id": metadata.get("id", job_dir.name),
            "title": metadata.get("title", "unknown"),
            "institution": metadata.get("institution", "unknown"),
            "deadline": metadata.get("deadline", "unknown"),
            "status": metadata.get("status", "unknown"),
            "next_action": next_job_action(job_dir, str(metadata.get("status", "unknown"))),
            "path": str(job_dir),
        })
    entries.sort(key=lambda e: (e["deadline"], e["institution"]))
    return entries


def next_job_action(job_dir: Path, status: str) -> str:
    if status == "lead_imported":
        return "paste full advert"
    if status == "new":
        return "add advert"
    if status == "advert_imported":
        return "run extract-profile-evidence, then run"
    if status == "packaged":
        if (job_dir / "07_material_review_checklist.md").exists():
            return "run check-package"
        return "rerun package generation"
    return "inspect job.yaml"


def _lead_advert_markdown(lead: dict[str, Any], title: str) -> str:
    lines = [
        f"# {title}",
        "",
        "> Feed lead only (RSS/Atom). Paste or import the full advert below before final generation.",
        "",
        "## Feed Lead Metadata",
        "",
        f"- Source: {lead.get('source', 'unknown') or 'unknown'}",
        f"- Source URL: {lead.get('source_url', '')}",
        f"- Published: {lead.get('published_at', '')}",
        f"- Source feed: {lead.get('source_feed', '')}",
        "",
        "## Feed Description",
        "",
        str(lead.get("description", "")).strip() or "_No feed description available._",
        "",
        "## Full Advert",
        "",
        "Paste the full advert manually here before relying on parsed criteria or generated drafts.",
        "",
    ]
    return "\n".join(lines)
