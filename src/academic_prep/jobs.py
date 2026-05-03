from datetime import UTC, datetime
from pathlib import Path
import re

import yaml


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
) -> Path:
    job_id = build_job_slug(deadline, institution, title)
    job_dir = jobs_dir / job_id
    job_dir.mkdir(parents=True, exist_ok=False)

    advert_text = ""
    status = "new"
    if advert_file is not None:
        if advert_file.suffix.lower() not in {".md", ".txt"}:
            raise ValueError("V1 only imports local .md or .txt job advert files.")
        advert_text = advert_file.read_text(encoding="utf-8")
        status = "advert_imported"

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
        "created_at": now,
        "updated_at": now,
        "notes": "",
    }
    (job_dir / "job.yaml").write_text(yaml.safe_dump(metadata, sort_keys=False), encoding="utf-8")

    return job_dir
