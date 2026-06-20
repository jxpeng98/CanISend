import yaml
from typer.testing import CliRunner

from canisend.cli import app


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
    assert "created_at" in metadata
    assert "updated_at" in metadata
    assert (job_dir / "job_advert.md").read_text() == ""


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
    assert "RSS lead only" in advert
    assert "Paste the full advert manually" in advert


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
