from typer.testing import CliRunner

from academic_prep.cli import app


def test_init_profile_creates_starter_evidence_files(tmp_path):
    profile_dir = tmp_path / "profile"
    runner = CliRunner()

    result = runner.invoke(app, ["init-profile", "--profile-dir", str(profile_dir)])

    assert result.exit_code == 0
    expected_files = [
        "cv.md",
        "publications.md",
        "teaching_experience.md",
        "research_statement.md",
        "teaching_statement.md",
        "service_leadership.md",
        "grants_awards.md",
        "references.md",
        "personal_profile.yaml",
    ]
    for filename in expected_files:
        assert (profile_dir / filename).exists()
    assert "# CV" in (profile_dir / "cv.md").read_text()
    assert "name:" in (profile_dir / "personal_profile.yaml").read_text()


def test_init_profile_does_not_overwrite_existing_files(tmp_path):
    profile_dir = tmp_path / "profile"
    profile_dir.mkdir()
    cv_file = profile_dir / "cv.md"
    cv_file.write_text("existing cv content\n")
    runner = CliRunner()

    result = runner.invoke(app, ["init-profile", "--profile-dir", str(profile_dir)])

    assert result.exit_code == 0
    assert cv_file.read_text() == "existing cv content\n"
