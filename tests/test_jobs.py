import yaml
from typer.testing import CliRunner

from academic_prep.cli import app


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
