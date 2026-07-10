import json
import re

import yaml
import pytest
from typer.testing import CliRunner

from canisend.cli import app
from canisend.jobs import JobMetadataError, job_advert_is_stub, load_job_metadata


ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")


def strip_ansi(text: str) -> str:
    return ANSI_ESCAPE_RE.sub("", text)


def test_new_job_creates_slugged_job_folder_and_metadata(tmp_path):
    jobs_dir = tmp_path / "jobs"
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Economics",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--source-url",
            "https://example.edu/jobs/123",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert result.exit_code == 0
    job_dir = jobs_dir / "2026-06-15_university-x_lecturer-in-economics"
    assert job_dir.exists()
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text())
    assert metadata["id"] == "2026-06-15_university-x_lecturer-in-economics"
    assert metadata["title"] == "Lecturer in Economics"
    assert metadata["institution"] == "University X"
    assert metadata["deadline"] == "2026-06-15"
    assert metadata["source_url"] == "https://example.edu/jobs/123"
    assert metadata["status"] == "new"
    assert metadata["english_variant"] == "needs_confirmation"
    assert metadata["writing_style"] == "needs_confirmation"
    assert "created_at" in metadata
    assert "updated_at" in metadata
    advert = (job_dir / "job_advert.md").read_text()
    assert "Source URL saved" in advert
    assert "https://example.edu/jobs/123" in advert
    assert "full advert still needs manual paste, PDF import, or explicit fetch" in advert


def test_new_job_accepts_language_and_style_preferences(tmp_path):
    jobs_dir = tmp_path / "jobs"
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Economics",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--english-variant",
            "US English",
            "--writing-style",
            "direct, warm, evidence-led",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert result.exit_code == 0
    metadata = yaml.safe_load(
        (jobs_dir / "2026-06-15_university-x_lecturer-in-economics" / "job.yaml").read_text()
    )
    assert metadata["english_variant"] == "us"
    assert metadata["writing_style"] == "direct, warm, evidence-led"


def test_new_job_imports_local_advert_file_unchanged(tmp_path):
    jobs_dir = tmp_path / "jobs"
    advert_file = tmp_path / "advert.md"
    advert_file.write_text("# Lecturer role\n\nEssential: PhD.\n")
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Economics",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--jobs-dir",
            str(jobs_dir),
            "--advert-file",
            str(advert_file),
        ],
    )

    assert result.exit_code == 0
    job_dir = jobs_dir / "2026-06-15_university-x_lecturer-in-economics"
    assert (job_dir / "job_advert.md").read_text() == "# Lecturer role\n\nEssential: PhD.\n"
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text())
    assert metadata["status"] == "advert_imported"


def test_new_job_with_source_url_writes_metadata_stub(tmp_path):
    jobs_dir = tmp_path / "jobs"
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Economics",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--source-url",
            "https://example.edu/jobs/123",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert result.exit_code == 0
    job_dir = jobs_dir / "2026-06-15_university-x_lecturer-in-economics"
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text())
    assert metadata["status"] == "new"
    advert = (job_dir / "job_advert.md").read_text()
    assert "Source URL saved" in advert
    assert "https://example.edu/jobs/123" in advert
    assert "full advert still needs manual paste, PDF import, or explicit fetch" in advert


def test_new_job_fetch_url_requires_source_url(tmp_path):
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer",
            "--institution",
            "University X",
            "--fetch-url",
            "--jobs-dir",
            str(tmp_path / "jobs"),
        ],
    )

    assert result.exit_code != 0
    assert "--fetch-url requires --source-url" in strip_ansi(result.output)
    assert not (tmp_path / "jobs").exists()


def test_new_job_fetch_url_imports_fetched_advert(tmp_path, monkeypatch):
    jobs_dir = tmp_path / "jobs"

    def fake_fetch(source_url: str):
        from canisend.job_import import ImportedAdvert

        return ImportedAdvert(
            text="# Lecturer\n\nEssential criteria:\n- PhD\n",
            status="advert_imported",
            notes=f"Fetched from {source_url}; review extracted text.",
            metadata_source_url="https://example.edu/jobs/123?redacted",
        )

    monkeypatch.setattr("canisend.jobs.fetch_advert_from_url", fake_fetch)
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--source-url",
            "https://example.edu/jobs/123",
            "--fetch-url",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert result.exit_code == 0
    job_dir = jobs_dir / "2026-06-15_university-x_lecturer"
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text())
    assert metadata["status"] == "advert_imported"
    assert "Fetched from https://example.edu/jobs/123" in metadata["notes"]
    assert metadata["source_url"] == "https://example.edu/jobs/123?redacted"
    assert "Essential criteria" in (job_dir / "job_advert.md").read_text()

    pipeline_result = runner.invoke(
        app,
        ["run", "--job", str(job_dir), "--no-git-add-materials"],
    )

    assert pipeline_result.exit_code == 0
    parsed_job = json.loads((job_dir / "parsed_job.json").read_text(encoding="utf-8"))
    assert parsed_job["title"] == "Lecturer"
    assert parsed_job["essential_criteria"][0]["criterion"] == "PhD"


def test_new_job_pdf_import_flows_into_job_parser(tmp_path, monkeypatch):
    jobs_dir = tmp_path / "jobs"
    pdf_file = tmp_path / "lecturer.pdf"
    pdf_file.write_bytes(b"%PDF fake")
    monkeypatch.setattr(
        "canisend.job_import.extract_pdf_text",
        lambda path: (
            "# Lecturer in Economics\n\n"
            "Department: Economics\n"
            "Required documents: CV, Cover letter\n\n"
            "Essential criteria:\n"
            "- PhD in Economics\n"
        ),
    )
    runner = CliRunner()

    intake_result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Economics",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--advert-file",
            str(pdf_file),
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    job_dir = jobs_dir / "2026-06-15_university-x_lecturer-in-economics"
    pipeline_result = runner.invoke(
        app,
        ["run", "--job", str(job_dir), "--no-git-add-materials"],
    )

    assert intake_result.exit_code == 0
    assert pipeline_result.exit_code == 0
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text(encoding="utf-8"))
    parsed_job = json.loads((job_dir / "parsed_job.json").read_text(encoding="utf-8"))
    assert metadata["notes"].startswith("Imported from local PDF lecturer.pdf")
    assert parsed_job["title"] == "Lecturer in Economics"
    assert parsed_job["required_documents"] == ["CV", "Cover letter"]
    assert parsed_job["essential_criteria"][0]["criterion"] == "PhD in Economics"


def test_new_job_from_lead_creates_workspace_without_scraping(tmp_path):
    jobs_dir = tmp_path / "jobs"
    leads_file = tmp_path / "jobs_ac_uk.json"
    leads_file.write_text(
        """[
  {
    "title": "Lecturer in Economics",
    "source_url": "https://www.jobs.ac.uk/job/ABC123/lecturer-in-economics",
    "description": "Teach econometrics and finance. Permanent academic role.",
    "published_at": "Mon, 04 May 2026 09:00:00 GMT",
    "source": "jobs.ac.uk",
    "source_feed": "https://www.jobs.ac.uk/jobs/rss"
  }
]
"""
    )
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job-from-lead",
            "--leads-file",
            str(leads_file),
            "--lead-index",
            "0",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert result.exit_code == 0
    job_dir = jobs_dir / "2026-06-15_university-x_lecturer-in-economics"
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text())
    assert metadata["title"] == "Lecturer in Economics"
    assert metadata["institution"] == "University X"
    assert metadata["source_url"] == "https://www.jobs.ac.uk/job/ABC123/lecturer-in-economics"
    assert metadata["status"] == "lead_imported"
    advert = (job_dir / "job_advert.md").read_text()
    assert "Teach econometrics and finance" in advert
    assert "Feed lead only" in advert
    assert "Paste or import the full advert" in advert


def test_new_job_from_feed_lead_uses_source_neutral_empty_description(tmp_path):
    jobs_dir = tmp_path / "jobs"
    leads_file = tmp_path / "atom.json"
    leads_file.write_text(
        '[{"title":"Research Fellow","source_url":"https://example.edu/jobs/1",'
        '"description":"","published_at":"","source":"Example",'
        '"source_feed":"https://example.edu/jobs.atom"}]\n',
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job-from-lead",
            "--leads-file",
            str(leads_file),
            "--lead-index",
            "0",
            "--institution",
            "Example University",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert result.exit_code == 0
    advert = next(jobs_dir.iterdir()).joinpath("job_advert.md").read_text(encoding="utf-8")
    assert "No feed description available" in advert
    assert "No RSS description" not in advert


def test_new_job_from_lead_rejects_negative_index(tmp_path):
    leads_file = tmp_path / "jobs_ac_uk.json"
    leads_file.write_text(
        """[
  {
    "title": "Lecturer in Economics",
    "source_url": "https://www.jobs.ac.uk/job/ABC123/lecturer-in-economics",
    "description": "Teach econometrics.",
    "published_at": "Mon, 04 May 2026 09:00:00 GMT",
    "source": "jobs.ac.uk",
    "source_feed": "https://www.jobs.ac.uk/jobs/rss"
  }
]
"""
    )
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job-from-lead",
            "--leads-file",
            str(leads_file),
            "--lead-index",
            "-1",
            "--institution",
            "University X",
        ],
    )

    assert result.exit_code != 0
    assert "Lead index must be zero or greater" in result.output


def test_list_jobs_shows_next_action_for_each_lifecycle_state(tmp_path):
    workspace = tmp_path / "workspace"
    jobs_dir = workspace / "jobs"
    jobs_dir.mkdir(parents=True)
    _write_job(jobs_dir / "new-role", status="new", title="New Role")
    _write_job(jobs_dir / "lead-role", status="lead_imported", title="Lead Role")
    packaged = jobs_dir / "packaged-role"
    _write_job(packaged, status="packaged", title="Packaged Role")
    (packaged / "07_material_review_checklist.md").write_text("# Checklist\n", encoding="utf-8")
    runner = CliRunner()

    result = runner.invoke(app, ["list-jobs", "--workspace", str(workspace)])

    assert result.exit_code == 0
    assert "Next action" in result.output
    assert "New Role" in result.output
    assert "add advert" in result.output
    assert "Lead Role" in result.output
    assert "paste full advert" in result.output
    assert "Packaged Role" in result.output
    assert "run check-package" in result.output


def test_load_job_metadata_returns_normalized_legacy_preferences(tmp_path):
    job_dir = tmp_path / "jobs" / "legacy-role"
    _write_job(job_dir, status="advert_imported", title="Legacy Role")

    metadata = load_job_metadata(job_dir)

    assert metadata["id"] == "legacy-role"
    assert metadata["english_variant"] == "needs_confirmation"
    assert metadata["writing_style"] == "needs_confirmation"


def test_load_job_metadata_rejects_missing_and_invalid_yaml(tmp_path):
    missing = tmp_path / "missing"
    missing.mkdir()
    invalid = tmp_path / "invalid"
    invalid.mkdir()
    (invalid / "job.yaml").write_text("- invalid\n- metadata\n", encoding="utf-8")

    with pytest.raises(JobMetadataError, match="job.yaml is missing"):
        load_job_metadata(missing)
    with pytest.raises(JobMetadataError, match="must contain a mapping"):
        load_job_metadata(invalid)


def test_job_advert_stub_detection_is_source_neutral():
    assert job_advert_is_stub("") is True
    assert job_advert_is_stub("# Job Advert Pending Import\n") is True
    assert job_advert_is_stub(
        "# Role\n\n> Feed lead only (RSS/Atom).\n\n"
        "## Full Advert\n\nPaste the full advert manually here.\n"
    ) is True
    assert job_advert_is_stub("# Full role\n\nEssential: PhD.\n") is False


def _write_job(job_dir, *, status: str, title: str) -> None:
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": job_dir.name,
                "title": title,
                "institution": "University X",
                "deadline": "2026-06-15",
                "status": status,
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job_dir / "job_advert.md").write_text("", encoding="utf-8")
