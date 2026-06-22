from __future__ import annotations

import json
import re
from pathlib import Path

import pytest
import yaml
from typer.testing import CliRunner

from canisend import __version__
from canisend.cli import app
from canisend.package_check import required_wheel_resources


EXPECTED_SKILLS = [
    "canisend",
    "canisend-job-fit",
    "canisend-research-statement",
    "canisend-teaching-statement",
    "canisend-cover-letter",
    "canisend-cv-tailoring",
    "canisend-humanizer",
    "canisend-application-email",
    "canisend-interview-prep",
    "canisend-criteria-check",
    "canisend-material-review",
]


def _strip_ansi(value: str) -> str:
    return re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", value)


def test_top_level_codex_plugin_manifest_exposes_skill_pack():
    manifest_path = Path(".codex-plugin/plugin.json")

    manifest = json.loads(manifest_path.read_text())

    assert manifest["name"] == "canisend"
    assert manifest["version"] == __version__
    assert manifest["skills"] == "./skills/"
    assert manifest["author"]["name"] == "Peng Jiaxin"
    assert manifest["interface"]["displayName"] == "CanISend"
    assert manifest["interface"]["category"] == "Productivity"
    assert manifest["interface"]["capabilities"] == ["Write"]
    assert manifest["interface"]["defaultPrompt"]
    assert "mcpServers" not in manifest
    assert "apps" not in manifest
    assert "hooks" not in manifest


@pytest.mark.parametrize("skill_name", EXPECTED_SKILLS)
def test_distributed_skills_have_standard_manifests(skill_name):
    skill_root = Path("skills") / skill_name
    skill_md = skill_root / "SKILL.md"
    agent_yaml = skill_root / "agents" / "openai.yaml"

    assert skill_md.exists()
    assert agent_yaml.exists()

    contents = skill_md.read_text()
    assert contents.startswith("---\n")
    frontmatter = yaml.safe_load(contents.split("---", 2)[1])
    assert frontmatter["name"] == skill_name
    assert frontmatter["description"].startswith("Use when")

    agent = yaml.safe_load(agent_yaml.read_text())
    assert agent["interface"]["display_name"]
    assert agent["interface"]["short_description"]
    assert agent["interface"]["default_prompt"]


def test_distributed_canisend_skill_mirrors_workspace_skill():
    assert Path("skills/canisend/SKILL.md").read_text() == Path("agent-skills/canisend/SKILL.md").read_text()

    reference_names = sorted(path.name for path in Path("agent-skills/canisend/references").glob("*.md"))
    assert reference_names
    for reference_name in reference_names:
        assert (Path("skills/canisend/references") / reference_name).read_text() == (
            Path("agent-skills/canisend/references") / reference_name
        ).read_text()


def test_material_skills_reference_shared_canisend_rules():
    for skill_name in EXPECTED_SKILLS:
        if skill_name == "canisend":
            continue
        contents = (Path("skills") / skill_name / "SKILL.md").read_text()
        assert "../canisend/references/privacy.md" in contents
        assert "../canisend/references/quality-gates.md" in contents
        assert "Do not submit applications" in contents
        assert "Do not fabricate" in contents


def test_package_check_requires_distributed_skill_pack_resources():
    resources = set(required_wheel_resources())

    expected = {
        "canisend/resources/.codex-plugin/plugin.json",
        "canisend/resources/skills/canisend/SKILL.md",
        "canisend/resources/skills/canisend/references/privacy.md",
        "canisend/resources/skills/canisend-job-fit/SKILL.md",
        "canisend/resources/skills/canisend-research-statement/SKILL.md",
        "canisend/resources/skills/canisend-teaching-statement/SKILL.md",
        "canisend/resources/skills/canisend-cover-letter/SKILL.md",
        "canisend/resources/skills/canisend-cv-tailoring/SKILL.md",
        "canisend/resources/skills/canisend-humanizer/SKILL.md",
        "canisend/resources/skills/canisend-application-email/SKILL.md",
        "canisend/resources/skills/canisend-interview-prep/SKILL.md",
        "canisend/resources/skills/canisend-criteria-check/SKILL.md",
        "canisend/resources/skills/canisend-material-review/SKILL.md",
    }

    assert expected <= resources
    assert "canisend/resources/platform-bridges/GEMINI.md" not in resources


def test_export_skills_writes_codex_plugin_distribution(tmp_path):
    target = tmp_path / "plugins" / "canisend"
    runner = CliRunner()

    result = runner.invoke(app, ["export-skills", "--target", str(target), "--kind", "codex-plugin"])

    assert result.exit_code == 0
    assert (target / ".codex-plugin" / "plugin.json").exists()
    assert (target / "skills" / "canisend" / "SKILL.md").exists()
    assert (target / "skills" / "canisend-research-statement" / "SKILL.md").exists()


def test_export_skills_writes_skills_only_distribution(tmp_path):
    target = tmp_path / "claude-skills"
    runner = CliRunner()

    result = runner.invoke(app, ["export-skills", "--target", str(target), "--kind", "skills-only"])

    assert result.exit_code == 0
    assert not (target / ".codex-plugin").exists()
    assert (target / "canisend" / "SKILL.md").exists()
    assert (target / "canisend-job-fit" / "SKILL.md").exists()
    assert (target / "canisend-cover-letter" / "SKILL.md").exists()
    assert (target / "canisend-humanizer" / "SKILL.md").exists()
    assert (target / "canisend-interview-prep" / "SKILL.md").exists()


def test_export_skills_refuses_non_empty_target_without_overwrite(tmp_path):
    target = tmp_path / "plugins" / "canisend"
    target.mkdir(parents=True)
    (target / "local.txt").write_text("keep me\n")
    runner = CliRunner()

    result = runner.invoke(app, ["export-skills", "--target", str(target), "--kind", "codex-plugin"])

    assert result.exit_code != 0
    assert "Use --overwrite" in _strip_ansi(result.output)
    assert (target / "local.txt").read_text() == "keep me\n"
