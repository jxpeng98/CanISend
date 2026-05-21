from __future__ import annotations

from pathlib import Path
import tomllib
import zipfile

import yaml
from typer.testing import CliRunner

from canisend import __version__
from canisend.cli import app
from canisend.package_check import missing_wheel_resources, required_wheel_resources


def test_doctor_reports_installed_package_version(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])

    result = runner.invoke(app, ["doctor", "--workspace", str(workspace)])

    assert result.exit_code == 0
    assert f"canisend: {__version__}" in result.output


def test_package_version_matches_project_metadata():
    metadata = tomllib.loads(Path("pyproject.toml").read_text())

    assert __version__ == metadata["project"]["version"]


def test_project_metadata_is_ready_for_public_package_index():
    metadata = tomllib.loads(Path("pyproject.toml").read_text())["project"]

    assert metadata["name"] == "canisend"
    assert metadata["description"] == "Evidence-backed application prep for academic and professional jobs."
    assert metadata["license"] == "MIT"
    assert {"name": "Peng Jiaxin"} in metadata["authors"]
    assert "academic jobs" in metadata["keywords"]
    assert "professional jobs" in metadata["keywords"]
    assert "Development Status :: 3 - Alpha" in metadata["classifiers"]
    assert "License :: OSI Approved :: MIT License" in metadata["classifiers"]
    assert "Programming Language :: Python :: 3.12" in metadata["classifiers"]
    assert metadata["urls"]["Repository"] == "https://github.com/jxpeng98/CanISend"
    assert metadata["urls"]["Issues"] == "https://github.com/jxpeng98/CanISend/issues"


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
    wheel_path = tmp_path / "canisend-0.1.0-py3-none-any.whl"
    with zipfile.ZipFile(wheel_path, "w") as wheel:
        for resource in required_wheel_resources():
            wheel.writestr(resource, "ok")

    assert missing_wheel_resources(wheel_path) == []


def test_package_check_requires_all_run_example_resources():
    resources = set(required_wheel_resources())

    expected = {
        "canisend/resources/prompts/cv_tailor.md",
        "canisend/resources/prompts/criteria_checker.md",
        "canisend/resources/prompts/package_builder.md",
        "canisend/resources/examples/end_to_end/jobs_ac_uk_sample.xml",
        "canisend/resources/examples/end_to_end/full_job_advert.md",
        "canisend/resources/examples/end_to_end/fake_llm_provider.py",
        "canisend/resources/examples/end_to_end/profile/profile.yaml",
        "canisend/resources/examples/end_to_end/profile/typst/cv.typ",
        "canisend/resources/examples/end_to_end/profile/typst/cover_letter_base.typ",
        "canisend/resources/examples/end_to_end/profile/typst/research_statement.typ",
        "canisend/resources/examples/end_to_end/profile/typst/teaching_statement.typ",
    }

    assert expected <= resources


def test_package_check_reports_missing_wheel_resources(tmp_path):
    wheel_path = tmp_path / "canisend-0.1.0-py3-none-any.whl"
    with zipfile.ZipFile(wheel_path, "w") as wheel:
        wheel.writestr("canisend/resources/prompts/job_parser.md", "ok")

    missing = missing_wheel_resources(wheel_path)

    assert "canisend/resources/templates/typst/cover_letter.typ" in missing
    assert "canisend/resources/agent-skills/canisend/SKILL.md" in missing
    assert "canisend/resources/agent-skills/canisend/agents/openai.yaml" in missing
    assert "canisend/resources/agent-skills/canisend/references/platforms.md" in missing
    assert "canisend/resources/platform-bridges/AGENTS.md" in missing


def test_ci_workflow_runs_tests_build_and_package_resource_check():
    workflow_path = Path(".github/workflows/ci.yml")

    workflow = yaml.safe_load(workflow_path.read_text())
    rendered = workflow_path.read_text()

    assert workflow["name"] == "ci"
    assert "uv run pytest -v" in rendered
    assert "uv build" in rendered
    assert "python -m canisend.package_check dist/*.whl" in rendered
    assert "uvx twine check dist/*" in rendered
    assert "python -m venv /tmp/canisend-smoke" in rendered
    assert "/tmp/canisend-smoke/bin/canisend doctor --workspace /tmp/canisend-workspace" in rendered


def test_release_workflow_publishes_with_trusted_publishing():
    workflow_path = Path(".github/workflows/release.yml")
    rendered = workflow_path.read_text()

    assert "publish_target:" in rendered
    assert "github.event.inputs.publish_target == 'testpypi'" in rendered
    assert "id-token: write" in rendered
    assert "pypa/gh-action-pypi-publish@release/v1" in rendered
    assert "repository-url: https://test.pypi.org/legacy/" in rendered
    assert "environment:" in rendered
    assert "dist/*.whl" in rendered
    assert "uvx twine check dist/*" in rendered
    assert "python -m venv /tmp/canisend-smoke" in rendered


def test_release_playbook_documents_testpypi_dry_run():
    playbook = Path("RELEASE.md").read_text()

    assert "## TestPyPI Dry Run" in playbook
    assert "uv run pytest" in playbook
    assert "uvx twine check dist/*" in playbook
    assert "uv run python -m canisend.package_check dist/*.whl" in playbook
    assert "gh workflow run release.yml -f publish_target=testpypi" in playbook
    assert "https://test.pypi.org/legacy/" in playbook
    assert "Repository: `CanISend`" in playbook
    assert "`repository`: `jxpeng98/CanISend`" in playbook
    assert "--index-url https://test.pypi.org/simple/" in playbook
    assert "--extra-index-url https://pypi.org/simple/" in playbook
    assert "canisend doctor --workspace" in playbook


def test_readme_documents_release_and_update_workflow():
    readme = Path("README.md").read_text()

    assert "## Maintainer Release" in readme
    assert "Trusted Publishing" in readme
    assert "TestPyPI" in readme
    assert "PyPI" in readme
    assert "RELEASE.md" in readme
    assert "jxpeng98/CanISend" in readme
    assert "Trusted Publisher" in readme
    assert "gh workflow run release.yml -f publish_target=testpypi" in readme
    assert "uv tool upgrade canisend" in readme
    assert "canisend doctor --workspace ~/CanISendWorkspace" in readme
