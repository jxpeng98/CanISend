import subprocess

import pytest

from canisend.git_tracking import (
    APPLICATION_MATERIAL_RELATIVE_PATHS,
    GitTrackingError,
    application_material_paths,
    git_add_application_materials,
)


def test_application_material_paths_only_returns_existing_allowlisted_outputs(tmp_path):
    job_dir = tmp_path / "jobs" / "job"
    for relative_path in [
        *APPLICATION_MATERIAL_RELATIVE_PATHS,
        "job_advert.md",
        "job.yaml",
        "parsed_job.json",
        "typst/cover_letter_content.json",
        "typst/application_package_content.json",
        "pdf/cover_letter.pdf",
    ]:
        path = job_dir / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(relative_path, encoding="utf-8")

    material_paths = application_material_paths(job_dir)

    assert [path.relative_to(job_dir).as_posix() for path in material_paths] == APPLICATION_MATERIAL_RELATIVE_PATHS


def test_git_add_application_materials_force_adds_only_allowlisted_outputs(tmp_path, monkeypatch):
    job_dir = tmp_path / "jobs" / "job"
    for relative_path in [*APPLICATION_MATERIAL_RELATIVE_PATHS, "job_advert.md", "parsed_job.json"]:
        path = job_dir / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(relative_path, encoding="utf-8")
    calls = []

    def fake_run(command, *, cwd, text, capture_output, check):
        calls.append((command, cwd, text, capture_output, check))
        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

    monkeypatch.setattr("canisend.git_tracking.subprocess.run", fake_run)

    result = git_add_application_materials(job_dir, repo_dir=tmp_path)

    assert result.files == application_material_paths(job_dir)
    assert len(calls) == 1
    command, cwd, text, capture_output, check = calls[0]
    assert command[:4] == ["git", "add", "-f", "--"]
    assert cwd == tmp_path
    assert text is True
    assert capture_output is True
    assert check is False
    staged = set(command[4:])
    assert staged == {str(job_dir / relative_path) for relative_path in APPLICATION_MATERIAL_RELATIVE_PATHS}
    assert str(job_dir / "job_advert.md") not in staged
    assert str(job_dir / "parsed_job.json") not in staged


def test_git_add_application_materials_reports_git_failures(tmp_path, monkeypatch):
    job_dir = tmp_path / "jobs" / "job"
    path = job_dir / "03_cover_letter_draft.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("# Cover\n", encoding="utf-8")

    def fake_run(command, *, cwd, text, capture_output, check):
        return subprocess.CompletedProcess(command, 128, stdout="", stderr="not a git repository")

    monkeypatch.setattr("canisend.git_tracking.subprocess.run", fake_run)

    with pytest.raises(GitTrackingError, match="not a git repository"):
        git_add_application_materials(job_dir, repo_dir=tmp_path)
