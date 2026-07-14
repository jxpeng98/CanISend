from __future__ import annotations

from pathlib import Path
import json
import tomllib
import zipfile

import pytest
import yaml
from typer.testing import CliRunner

from canisend import __version__
from canisend.cli import app
from canisend.package_check import missing_wheel_resources, required_wheel_resources
from scripts import smoke_decision_spine


def _assert_complete_decision_spine_smoke(command: str) -> None:
    assert "scripts/smoke_decision_spine.py" in command
    assert "--canisend" in command
    assert "--workspace" in command


def test_shared_decision_spine_smoke_owns_the_full_body_free_contract():
    rendered = Path("scripts/smoke_decision_spine.py").read_text()
    ordered_steps = [
        'for stage in ("evidence", "parse", "confirm")',
        '["corrections", "status"',
        '["corrections", "init"',
        '"--stage", "match"',
        '["decision", "status"',
        '["decision", "init"',
        '"decision",\n                "update"',
        '["brief", "status"',
        '["brief", "init"',
        '"--stage", "brief"',
        '"operation": "confirm_document_requirements"',
        '"--stage",\n            "draft"',
        '"--mode",\n            "configured-provider"',
        '"--allow-provider-backed"',
        '"--stage", "review"',
        '["review-dispositions", "status"',
        '"review-dispositions",\n            "init"',
        '"operation": "set_finding_disposition"',
        '"run",\n            "--workspace"',
        '"check-package"',
    ]

    positions = [rendered.index(step) for step in ordered_steps]
    assert positions == sorted(positions)
    assert '"operation": "set_decision", "decision": "apply"' in rendered
    assert '"--expected-revision"' in rendered
    assert '"--expected-sha256"' in rendered
    assert "application_brief.yaml" in rendered
    assert "required_document_plan.json" in rendered
    assert "cover_letter_draft.json" in rendered
    assert "stage.provider_consent_required" in rendered
    assert "configured_provider" in rendered
    assert "review_findings.json" in rendered
    assert "Deterministic proposal" in rendered
    assert "Criterion is unresolved" in rendered
    assert "not document readiness" in rendered
    assert "structured-draft projection" in rendered
    assert "application_package_content.json" in rendered
    assert "EXPECTED_USER_MUTATION_RECEIPTS = 14" in rendered
    for stage in ("evidence", "parse", "confirm", "match", "brief", "draft", "review"):
        assert f'"{stage}":' in rendered
    assert "preparation.json" in rendered
    assert "submission.json" in rendered
    assert "completed.stdout" not in rendered[rendered.index("def main(") :]
    assert "completed.stderr" not in rendered[rendered.index("def main(") :]


def test_shared_decision_spine_smoke_does_not_echo_failed_command_bodies(monkeypatch):
    private_body = "PRIVATE-SMOKE-BODY-DO-NOT-ECHO"
    monkeypatch.setattr(
        smoke_decision_spine.subprocess,
        "run",
        lambda *args, **kwargs: smoke_decision_spine.subprocess.CompletedProcess(
            args=args,
            returncode=9,
            stdout=private_body,
            stderr=private_body,
        ),
    )

    with pytest.raises(smoke_decision_spine.SmokeFailure) as failure:
        smoke_decision_spine._run("canisend", ["decision", "status"], expect_json=True)

    assert private_body not in str(failure.value)


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


def test_readme_testpypi_version_matches_package_version():
    readme = Path("README.md").read_text()

    assert f"canisend=={__version__}" in readme
    assert f"TestPyPI-{__version__}-blue" in readme


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
        "canisend/resources/schemas/agent-response.schema.json",
        "canisend/resources/schemas/workflow-state.schema.json",
        "canisend/resources/schemas/task-spec.schema.json",
        "canisend/resources/schemas/task-result.schema.json",
        "canisend/resources/schemas/run-manifest.schema.json",
        "canisend/resources/schemas/criteria.schema.json",
        "canisend/resources/schemas/evidence-catalog.schema.json",
        "canisend/resources/schemas/criterion-matches.schema.json",
        "canisend/resources/schemas/confirmed-corrections.schema.json",
        "canisend/resources/schemas/application-decision.schema.json",
        "canisend/resources/schemas/application-brief.schema.json",
        "canisend/resources/schemas/required-document-plan.schema.json",
        "canisend/resources/schemas/cover-letter-draft.schema.json",
        "canisend/resources/schemas/review-findings.schema.json",
        "canisend/resources/schemas/review-dispositions.schema.json",
        "canisend/resources/schemas/document-readiness.schema.json",
        "canisend/resources/schemas/user-mutation-receipt.schema.json",
        "canisend/resources/prompts/cv_tailor.md",
        "canisend/resources/prompts/criteria_checker.md",
        "canisend/resources/prompts/package_builder.md",
        "canisend/resources/prompts/profile_evidence_augmenter.md",
        "canisend/resources/prompts/structured_cover_letter_draft.md",
        "canisend/resources/examples/end_to_end/jobs_ac_uk_sample.xml",
        "canisend/resources/examples/end_to_end/full_job_advert.md",
        "canisend/resources/examples/end_to_end/fake_llm_provider.py",
        "canisend/resources/examples/end_to_end/profile/profile.yaml",
        "canisend/resources/examples/end_to_end/profile/typst/cv.typ",
        "canisend/resources/examples/end_to_end/profile/typst/cover_letter_base.typ",
        "canisend/resources/examples/end_to_end/profile/typst/research_statement.typ",
        "canisend/resources/examples/end_to_end/profile/typst/teaching_statement.typ",
        "canisend/resources/examples/agent_handoff/README.md",
        "canisend/resources/examples/agent_handoff/expected_capabilities.json",
        "canisend/resources/examples/agent_handoff/expected_context_shape.json",
    }

    assert expected <= resources
    assert "canisend/resources/platform-bridges/GEMINI.md" not in resources


def test_package_check_requires_self_contained_workspace_skill_pack():
    resources = set(required_wheel_resources())

    for skill_name in [
        "canisend",
        "canisend-job-intake",
        "canisend-application-package",
        "canisend-submission-readiness",
    ]:
        assert f"canisend/resources/skills/{skill_name}/SKILL.md" in resources
        assert f"canisend/resources/skills/{skill_name}/agents/openai.yaml" in resources


def test_package_check_reports_missing_wheel_resources(tmp_path):
    wheel_path = tmp_path / "canisend-0.1.0-py3-none-any.whl"
    with zipfile.ZipFile(wheel_path, "w") as wheel:
        wheel.writestr("canisend/resources/prompts/job_parser.md", "ok")

    missing = missing_wheel_resources(wheel_path)

    assert "canisend/resources/templates/typst/cover_letter.typ" in missing
    assert "canisend/resources/schemas/evidence-catalog.schema.json" in missing
    assert "canisend/resources/agent-skills/canisend/SKILL.md" in missing
    assert "canisend/resources/agent-skills/canisend/agents/openai.yaml" in missing
    assert "canisend/resources/agent-skills/canisend/references/platforms.md" in missing
    assert "canisend/resources/platform-bridges/AGENTS.md" in missing


def test_ci_workflow_runs_tests_build_and_package_resource_check():
    workflow_path = Path(".github/workflows/ci.yml")

    workflow = yaml.safe_load(workflow_path.read_text())
    rendered = workflow_path.read_text()

    assert workflow["name"] == "ci"
    assert "uv run python -m pytest -v" in rendered
    assert "uv build" in rendered
    assert "python -m canisend.package_check dist/*.whl" in rendered
    assert "uvx twine check dist/*" in rendered
    assert "python -m venv /tmp/canisend-smoke" in rendered
    assert rendered.count("scripts/smoke_decision_spine.py") == 2
    built_wheel_smoke = next(
        step["run"]
        for step in workflow["jobs"]["build-package"]["steps"]
        if step.get("name") == "Smoke test built wheel"
    )
    _assert_complete_decision_spine_smoke(built_wheel_smoke)


def test_ci_covers_supported_python_versions_and_cross_os_cli_smoke():
    workflow = yaml.safe_load(Path(".github/workflows/ci.yml").read_text())
    jobs = workflow["jobs"]

    assert jobs["test"]["strategy"]["matrix"]["python-version"] == ["3.11", "3.12", "3.13"]
    assert jobs["build-package"]["needs"] == "test"
    assert "strategy" not in jobs["build-package"]
    assert jobs["smoke"]["strategy"]["matrix"]["os"] == [
        "ubuntu-latest",
        "macos-latest",
        "windows-latest",
    ]
    smoke_job = jobs["smoke"]
    smoke = json.dumps(smoke_job)
    assert 'python-version: "3.12"' in Path(".github/workflows/ci.yml").read_text()
    assert "scripts/smoke_decision_spine.py" in smoke
    assert "--canisend canisend" in smoke
    assert "--workspace .ci-smoke-stage2" in smoke
    cross_os_smoke = next(
        step["run"]
        for step in smoke_job["steps"]
        if step.get("name") == "Smoke test CLI contract"
    )
    _assert_complete_decision_spine_smoke(cross_os_smoke)


def test_release_workflow_publishes_with_trusted_publishing():
    workflow_path = Path(".github/workflows/release.yml")
    workflow = yaml.safe_load(workflow_path.read_text())
    rendered = workflow_path.read_text()

    assert "push:" in rendered
    assert "tags:" in rendered
    assert '"v*"' in rendered
    assert '"test/v*"' in rendered
    assert "workflow_dispatch:" not in rendered
    assert "github.event.inputs.publish_target" not in rendered
    assert "github.event_name == 'release'" not in rendered
    assert "publish_pypi=true" in rendered
    assert "id-token: write" in rendered
    assert "pypa/gh-action-pypi-publish@release/v1" in rendered
    assert "repository-url: https://test.pypi.org/legacy/" in rendered
    assert "needs: [build, publish-testpypi, smoke-test-testpypi]" in rendered
    assert "Create GitHub Release" in rendered
    assert "gh release create \"$GITHUB_REF_NAME\"" in rendered
    assert "--prerelease" in rendered
    create_release_job = rendered[rendered.index("  create-github-release:") :]
    assert "uses: actions/checkout@v4" in create_release_job
    assert create_release_job.index("uses: actions/checkout@v4") < create_release_job.index("gh release create")
    assert "environment:" in rendered
    assert "dist/*.whl" in rendered
    assert "uvx twine check dist/*" in rendered
    assert "python -m venv /tmp/canisend-smoke" in rendered
    assert rendered.count("scripts/smoke_decision_spine.py") == 2
    assert "--workspace /tmp/canisend-stage2-example" in rendered
    assert "--workspace /tmp/canisend-testpypi-stage2-example" in rendered
    testpypi_steps = workflow["jobs"]["smoke-test-testpypi"]["steps"]
    assert testpypi_steps[0]["uses"] == "actions/checkout@v4"
    for job_name, step_name in (
        ("build", "Smoke test built wheel"),
        ("smoke-test-testpypi", "Smoke test TestPyPI package"),
    ):
        smoke_command = next(
            step["run"]
            for step in workflow["jobs"][job_name]["steps"]
            if step.get("name") == step_name
        )
        _assert_complete_decision_spine_smoke(smoke_command)


def test_local_release_checks_use_shared_smoke_through_brief():
    rendered = Path("scripts/release.sh").read_text()

    assert "uv pip install --python" in rendered
    assert rendered.count("scripts/smoke_decision_spine.py") == 1
    assert '--canisend "$smoke_root/venv/bin/canisend"' in rendered
    assert '--workspace "$smoke_root/workspace"' in rendered


def test_release_workflow_guards_stable_tag_source_provenance():
    rendered = Path(".github/workflows/release.yml").read_text()

    assert "fetch-depth: 0" in rendered
    assert "Verify release source provenance" in rendered
    assert 'if [[ "$release_channel" == "stable" ]]' in rendered
    assert "git fetch origin main" in rendered
    assert 'git merge-base --is-ancestor "$GITHUB_SHA" "origin/main"' in rendered
    assert "Prerelease tags may originate from a reviewed non-main branch" in rendered


def test_release_playbook_documents_testpypi_dry_run():
    playbook = Path("RELEASE.md").read_text()

    assert "## Release Channels" in playbook
    assert "scripts/release.sh test" in playbook
    assert "scripts/release.sh beta --version 0.3.0a1" in playbook
    assert "scripts/release.sh stable --version 0.3.0" in playbook
    assert "test/v0.3.0.dev1" in playbook
    assert "v0.3.0a1" in playbook
    assert "v0.3.0" in playbook
    assert "Do not reuse `v0.2.0`" in playbook
    assert "stable" in playbook and "origin/main" in playbook
    assert "prerelease" in playbook and "non-main branch" in playbook
    assert "candidate commit" in playbook and "pushed" in playbook
    assert "commits the version bump" in playbook
    assert "publishes to PyPI only after TestPyPI publish and smoke testing succeed" in playbook
    assert "## TestPyPI Dry Run" in playbook
    assert "uv run python -m pytest" in playbook
    assert "uvx twine check dist/*" in playbook
    assert "uv run python -m canisend.package_check dist/*.whl" in playbook
    assert "gh workflow run release.yml" not in playbook
    assert "https://test.pypi.org/legacy/" in playbook
    assert "Repository: `CanISend`" in playbook
    assert "`repository`: `jxpeng98/CanISend`" in playbook
    assert "--index-url https://test.pypi.org/simple/" in playbook
    assert "--extra-index-url https://pypi.org/simple/" in playbook
    assert "canisend doctor --workspace" in playbook


def test_changelog_records_published_020_before_unreleased_phase_one_work():
    changelog = Path("CHANGELOG.md").read_text()

    assert "## Unreleased" in changelog
    assert "## 0.2.0 - 2026-06-22" in changelog
    assert changelog.index("## Unreleased") < changelog.index("## 0.2.0 - 2026-06-22")
    assert "agent protocol" in changelog.lower()


def test_readme_documents_release_and_update_workflow():
    readme = Path("README.md").read_text()

    assert "## Maintainer Release" in readme
    assert "Trusted Publishing" in readme
    assert "TestPyPI" in readme
    assert "PyPI" in readme
    assert "RELEASE.md" in readme
    assert "jxpeng98/CanISend" in readme
    assert "Trusted Publisher" in readme
    assert "scripts/release.sh test" in readme
    assert "scripts/release.sh beta" in readme
    assert "scripts/release.sh stable" in readme
    assert "git tag" in readme
    assert "commits the version bump" in readme
    assert "gh workflow run release.yml" not in readme
    assert "uv tool upgrade canisend" in readme
    assert "canisend doctor --workspace ~/CanISendWorkspace" in readme
