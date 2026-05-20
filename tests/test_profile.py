from typer.testing import CliRunner
import yaml

from canisend.cli import app


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
    assert (profile_dir / "profile.yaml").exists()
    manifest = yaml.safe_load((profile_dir / "profile.yaml").read_text())
    assert manifest["profile_mode"] == "hybrid"
    assert (profile_dir / "typst" / "cv.typ").exists()
    assert (profile_dir / "generated" / ".gitkeep").exists()


def test_init_profile_does_not_overwrite_existing_files(tmp_path):
    profile_dir = tmp_path / "profile"
    profile_dir.mkdir()
    cv_file = profile_dir / "cv.md"
    cv_file.write_text("existing cv content\n")
    runner = CliRunner()

    result = runner.invoke(app, ["init-profile", "--profile-dir", str(profile_dir)])

    assert result.exit_code == 0
    assert cv_file.read_text() == "existing cv content\n"


def test_init_profile_typst_mode_creates_only_typst_manifest_and_sources(tmp_path):
    profile_dir = tmp_path / "profile"
    runner = CliRunner()

    result = runner.invoke(app, ["init-profile", "--profile-dir", str(profile_dir), "--mode", "typst"])

    assert result.exit_code == 0
    assert (profile_dir / "profile.yaml").exists()
    manifest = yaml.safe_load((profile_dir / "profile.yaml").read_text())
    assert manifest["profile_mode"] == "typst"
    assert (profile_dir / "typst" / "cv.typ").exists()
    assert (profile_dir / "typst" / "research_statement.typ").exists()
    assert (profile_dir / "generated" / ".gitkeep").exists()
    assert not (profile_dir / "cv.md").exists()
