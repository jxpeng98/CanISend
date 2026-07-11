from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml

from canisend.stages.parse_stage import (
    ParseStageValidationError,
    build_deterministic_parse_candidate,
    parse_input_fingerprint,
    parse_input_projection,
    validate_parse_candidate,
)


def _write_job(root: Path) -> Path:
    job_dir = root / "jobs" / "example-role"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "example-role",
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/jobs/role?token=private",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": "direct",
                "created_at": "2026-07-11T10:00:00Z",
                "updated_at": "2026-07-11T10:00:00Z",
                "notes": "private downstream note",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job_dir / "job_advert.md").write_text(
        """# Lecturer in Economics

Department: Economics
Location: London
Required documents: CV, Cover letter

Essential criteria:
- PhD in Economics

Desirable criteria:
- Experience teaching econometrics
""",
        encoding="utf-8",
    )
    return job_dir


def _metadata(job_dir: Path) -> dict[str, object]:
    return yaml.safe_load((job_dir / "job.yaml").read_text(encoding="utf-8"))


def _write_metadata(job_dir: Path, metadata: dict[str, object]) -> None:
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(metadata, sort_keys=False),
        encoding="utf-8",
    )


def test_parse_projection_contains_only_declared_metadata_and_hashes(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)

    projection = parse_input_projection(job_dir)
    rendered = json.dumps(projection, sort_keys=True)

    assert projection["stage"] == "parse"
    assert projection["contract_version"] == "1.0.0"
    assert set(projection["metadata"]) == {
        "title",
        "institution",
        "department",
        "location",
        "deadline",
        "source_url",
    }
    assert len(projection["advert_sha256"]) == 64
    assert len(projection["schema_sha256"]) == 64
    assert "private downstream note" not in rendered
    assert "writing_style" not in rendered
    assert "updated_at" not in rendered


def test_parse_fingerprint_ignores_downstream_metadata_and_profile_changes(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    first = parse_input_fingerprint(job_dir)
    metadata = _metadata(job_dir)
    metadata.update(
        {
            "status": "packaged",
            "updated_at": "2026-07-11T11:00:00Z",
            "notes": "changed private note",
            "english_variant": "us",
            "writing_style": "warm",
        }
    )
    _write_metadata(job_dir, metadata)
    profile = tmp_path / "profile" / "generated" / "cv.evidence.md"
    profile.parent.mkdir(parents=True)
    profile.write_text("changed profile evidence\n", encoding="utf-8")

    assert parse_input_fingerprint(job_dir) == first


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("title", "Senior Lecturer in Economics"),
        ("institution", "Another University"),
        ("department", "Business School"),
        ("location", "Manchester"),
        ("deadline", "2026-09-01"),
        ("source_url", "https://example.edu/jobs/new"),
    ],
)
def test_parse_fingerprint_changes_for_relevant_metadata(
    tmp_path: Path,
    field: str,
    value: str,
) -> None:
    job_dir = _write_job(tmp_path)
    first = parse_input_fingerprint(job_dir)
    metadata = _metadata(job_dir)
    metadata[field] = value
    _write_metadata(job_dir, metadata)

    assert parse_input_fingerprint(job_dir) != first


def test_parse_fingerprint_changes_for_advert_or_schema(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    schema_path = tmp_path / "parsed_job.schema.json"
    schema_path.write_text(Path("schemas/parsed_job.schema.json").read_text(), encoding="utf-8")
    first = parse_input_fingerprint(job_dir, schema_path=schema_path)

    (job_dir / "job_advert.md").write_text("# Changed advert\n", encoding="utf-8")
    assert parse_input_fingerprint(job_dir, schema_path=schema_path) != first

    (job_dir / "job_advert.md").write_text("# Lecturer in Economics\n", encoding="utf-8")
    second = parse_input_fingerprint(job_dir, schema_path=schema_path)
    schema_path.write_text(schema_path.read_text() + "\n", encoding="utf-8")
    assert parse_input_fingerprint(job_dir, schema_path=schema_path) != second


def test_deterministic_parse_candidate_passes_semantic_and_source_validation(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)

    candidate = build_deterministic_parse_candidate(job_dir)
    validated = validate_parse_candidate(
        candidate,
        advert_text=(job_dir / "job_advert.md").read_text(encoding="utf-8"),
    )

    assert validated["title"] == "Lecturer in Economics"
    assert validated["essential_criteria"][0]["source_text"] == "PhD in Economics"


def test_parse_candidate_rejects_missing_fields_and_unresolved_source_text(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    advert = (job_dir / "job_advert.md").read_text(encoding="utf-8")
    candidate = build_deterministic_parse_candidate(job_dir)
    candidate.pop("deadline")

    with pytest.raises(ParseStageValidationError, match="schema or semantic validation"):
        validate_parse_candidate(candidate, advert_text=advert)

    candidate = build_deterministic_parse_candidate(job_dir)
    candidate["essential_criteria"][0]["source_text"] = "Fabricated requirement"

    with pytest.raises(ParseStageValidationError, match="source receipt"):
        validate_parse_candidate(candidate, advert_text=advert)


def test_parse_candidate_rejects_undeclared_nested_private_fields(tmp_path: Path) -> None:
    job_dir = _write_job(tmp_path)
    candidate = build_deterministic_parse_candidate(job_dir)
    candidate["essential_criteria"][0]["private_body"] = "must not be promoted"

    with pytest.raises(ParseStageValidationError, match="schema or semantic validation"):
        validate_parse_candidate(
            candidate,
            advert_text=(job_dir / "job_advert.md").read_text(encoding="utf-8"),
        )
