from datetime import UTC, datetime
import json
from pathlib import Path
import re
from typing import Any

import yaml

from canisend.job_import import JobImportError, fetch_advert_from_url, import_advert_file
from canisend.discovery.identity import LeadNormalizationError, normalize_job_lead


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
    lead_index: int | None = None,
    lead_id: str | None = None,
    jobs_dir: Path,
    institution: str,
    deadline: str = "unknown",
    title: str | None = None,
    english_variant: str = "",
    writing_style: str = "",
) -> Path:
    lead = _load_lead(leads_file, lead_index=lead_index, lead_id=lead_id)
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
        notes="Created from a discovery lead only; paste or import and review the full advert.",
        english_variant=english_variant,
        writing_style=writing_style,
        source_lead_id=str(lead.get("lead_id", "")),
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
    source_lead_id: str = "",
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
    if source_lead_id:
        metadata["source_lead_id"] = source_lead_id
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
    if (
        "rss lead only" not in lowered
        and "feed lead only" not in lowered
        and "discovery lead only" not in lowered
    ):
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


def _load_lead(
    leads_file: Path,
    *,
    lead_index: int | None,
    lead_id: str | None,
) -> dict[str, Any]:
    selected_id = (lead_id or "").strip()
    if (lead_index is None) == (not selected_id):
        raise ValueError("Provide exactly one of --lead-id or --lead-index.")
    if lead_index is not None and lead_index < 0:
        raise ValueError("Lead index must be zero or greater.")

    try:
        document = json.loads(leads_file.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ValueError("Lead file could not be read.") from exc
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise ValueError("Lead file must contain valid UTF-8 JSON.") from exc

    if isinstance(document, list):
        leads = document
    elif isinstance(document, dict) and isinstance(document.get("leads"), list):
        leads = document["leads"]
    else:
        raise ValueError("Lead file must contain a JSON list or a catalog with a leads list.")

    if lead_index is not None:
        try:
            lead = leads[lead_index]
        except IndexError as exc:
            raise ValueError(f"Lead index {lead_index} is out of range.") from exc
        if not isinstance(lead, dict):
            raise ValueError(f"Lead index {lead_index} is not an object.")
        return _validated_or_legacy_lead(lead)

    matches: list[dict[str, Any]] = []
    for position, candidate in enumerate(leads):
        if not isinstance(candidate, dict):
            raise ValueError(f"Lead index {position} is not an object.")
        normalized = _validated_or_legacy_lead(candidate)
        candidate_ids = {
            str(normalized.get("lead_id", "")),
            *(
                str(value)
                for value in normalized.get("alternate_lead_ids", [])
                if isinstance(value, str)
            ),
        }
        if selected_id in candidate_ids:
            matches.append(normalized)
    if not matches:
        raise ValueError(f"Lead ID {selected_id} was not found.")
    if len(matches) > 1:
        raise ValueError(f"Lead ID {selected_id} is ambiguous in the selected file.")
    return matches[0]


def _validated_or_legacy_lead(lead: dict[str, Any]) -> dict[str, Any]:
    if lead.get("schema_version") == "2.0.0":
        try:
            return normalize_job_lead(lead).model_dump(mode="json")
        except (LeadNormalizationError, ValueError) as exc:
            raise ValueError("Selected Lead v2 record is invalid.") from exc
    try:
        normalized = normalize_job_lead(lead)
    except (LeadNormalizationError, ValueError) as exc:
        raise ValueError("Selected legacy lead record is invalid.") from exc
    return {**lead, "lead_id": normalized.lead_id, "canonical_url": normalized.canonical_url}


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
        "> Discovery lead only (Feed lead only for RSS/Atom; untrusted imported metadata). Paste or import the full advert below before final generation.",
        "",
        "## Feed Lead Metadata",
        "",
        f"- Source: {lead.get('source', 'unknown') or 'unknown'}",
        f"- Lead ID: {lead.get('lead_id', '')}",
        f"- Source URL: {lead.get('source_url', '')}",
        f"- Canonical URL: {lead.get('canonical_url', '')}",
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
