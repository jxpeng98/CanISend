from __future__ import annotations

from pathlib import Path
import tomllib
import zipfile

import yaml
from typer.testing import CliRunner

from academic_prep import __version__
from academic_prep.cli import app
from academic_prep.package_check import missing_wheel_resources, required_wheel_resources


def test_doctor_reports_installed_package_version(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    result = runner.invoke(app, ["doctor", "--workspace", str(workspace)])

    assert result.exit_code == 0
    assert f"academic-application-prep: {__version__}" in result.output


def test_package_version_matches_project_metadata():
    metadata = tomllib.loads(Path("pyproject.toml").read_text())

    assert __version__ == metadata["project"]["version"]


def test_project_metadata_is_ready_for_public_package_index():
    metadata = tomllib.loads(Path("pyproject.toml").read_text())["project"]

    assert metadata["license"] == "MIT"
    assert {"name": "Peng Jiaxin"} in metadata["authors"]
    assert "academic jobs" in metadata["keywords"]
    assert "Development Status :: 3 - Alpha" in metadata["classifiers"]
    assert "License :: OSI Approved :: MIT License" in metadata["classifiers"]
    assert "Programming Language :: Python :: 3.12" in metadata["classifiers"]
    assert metadata["urls"]["Repository"]
    assert metadata["urls"]["Issues"]


def test_repository_has_release_notes_and_license():
    license_text = Path("LICENSE").read_text()
    changelog = Path("CHANGELOG.md").read_text()
    gitignore = Path(".gitignore").read_text()

    assert "MIT License" in license_text
    assert "Copyright (c) 2026 Peng Jiaxin" in license_text
    assert "## 0.1.0" in changelog
    assert "Alpha" in changelog
    assert ".claude/" in gitignore


def test_package_check_detects_required_wheel_resources(tmp_path):
    wheel_path = tmp_path / "academic_application_prep-0.1.0-py3-none-any.whl"
    with zipfile.ZipFile(wheel_path, "w") as wheel:
        for resource in required_wheel_resources():
            wheel.writestr(resource, "ok")

    assert missing_wheel_resources(wheel_path) == []


def test_package_check_reports_missing_wheel_resources(tmp_path):
    wheel_path = tmp_path / "academic_application_prep-0.1.0-py3-none-any.whl"
    with zipfile.ZipFile(wheel_path, "w") as wheel:
        wheel.writestr("academic_prep/resources/prompts/job_parser.md", "ok")

    missing = missing_wheel_resources(wheel_path)

    assert "academic_prep/resources/templates/typst/cover_letter.typ" in missing
    assert "academic_prep/resources/agent-skills/academic-application-prep/SKILL.md" in missing
    assert "academic_prep/resources/agent-skills/academic-application-prep/agents/openai.yaml" in missing
    assert "academic_prep/resources/agent-skills/academic-application-prep/references/platforms.md" in missing
    assert "academic_prep/resources/platform-bridges/AGENTS.md" in missing


def test_ci_workflow_runs_tests_build_and_package_resource_check():
    workflow_path = Path(".github/workflows/ci.yml")

    workflow = yaml.safe_load(workflow_path.read_text())
    rendered = workflow_path.read_text()

    assert workflow["name"] == "ci"
    assert "uv run pytest -v" in rendered
    assert "uv build" in rendered
    assert "python -m academic_prep.package_check dist/*.whl" in rendered
    assert "uvx twine check dist/*" in rendered
    assert "python -m venv /tmp/aap-smoke" in rendered
    assert "/tmp/aap-smoke/bin/academic-prep doctor --workspace /tmp/aap-workspace" in rendered


def test_release_workflow_publishes_with_trusted_publishing():
    workflow_path = Path(".github/workflows/release.yml")
    rendered = workflow_path.read_text()

    assert "id-token: write" in rendered
    assert "pypa/gh-action-pypi-publish@release/v1" in rendered
    assert "repository-url: https://test.pypi.org/legacy/" in rendered
    assert "environment:" in rendered
    assert "dist/*.whl" in rendered
    assert "uvx twine check dist/*" in rendered
    assert "python -m venv /tmp/aap-smoke" in rendered


def test_readme_documents_release_and_update_workflow():
    readme = Path("README.md").read_text()

    assert "## Release And Update Workflow" in readme
    assert "Trusted Publishing" in readme
    assert "TestPyPI" in readme
    assert "PyPI" in readme
    assert "uv tool upgrade academic-application-prep" in readme
    assert "academic-prep doctor --workspace ~/AcademicApplications" in readme
