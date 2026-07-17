from __future__ import annotations

import json
import os
import re
from pathlib import Path

import pytest
import yaml
from typer.testing import CliRunner

from canisend import __version__
from canisend.cli import app
from canisend.package_check import required_wheel_resources
from canisend.skill_distribution import CANONICAL_SKILL_RESOURCE
from scripts import sync_workspace_skill_mirror


EXPECTED_SKILLS = [
    "canisend",
    "canisend-job-intake",
    "canisend-application-package",
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
    "canisend-submission-readiness",
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


def _configure_temporary_skill_mirror(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> tuple[Path, Path]:
    canonical = tmp_path / "skills" / "canisend"
    mirror = tmp_path / "agent-skills" / "canisend"
    (canonical / "references").mkdir(parents=True)
    (canonical / "SKILL.md").write_text("canonical skill\n", encoding="utf-8")
    (canonical / "references" / "workflow.md").write_text("canonical workflow\n", encoding="utf-8")
    monkeypatch.setattr(sync_workspace_skill_mirror, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(sync_workspace_skill_mirror, "CANONICAL_MAIN_SKILL", canonical)
    monkeypatch.setattr(sync_workspace_skill_mirror, "COMPATIBILITY_MIRROR", mirror)
    return canonical, mirror


def _copy_temporary_skill(canonical: Path, mirror: Path) -> None:
    import shutil

    shutil.copytree(canonical, mirror)


def test_skill_mirror_check_succeeds_for_an_exact_tree(monkeypatch: pytest.MonkeyPatch, tmp_path: Path):
    canonical, mirror = _configure_temporary_skill_mirror(monkeypatch, tmp_path)
    _copy_temporary_skill(canonical, mirror)

    assert sync_workspace_skill_mirror.main(["--check"]) == 0


@pytest.mark.parametrize("mismatch", ["missing", "extra", "drift"])
def test_skill_mirror_check_rejects_every_tree_mismatch(
    mismatch: str,
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
):
    canonical, mirror = _configure_temporary_skill_mirror(monkeypatch, tmp_path)
    _copy_temporary_skill(canonical, mirror)

    if mismatch == "missing":
        (mirror / "references" / "workflow.md").unlink()
    elif mismatch == "extra":
        (mirror / "references" / "obsolete.md").write_text("obsolete\n", encoding="utf-8")
    else:
        (mirror / "SKILL.md").write_text("drifted skill\n", encoding="utf-8")

    assert sync_workspace_skill_mirror.main(["--check"]) == 1
    assert mismatch in capsys.readouterr().err


def test_skill_mirror_sync_rebuilds_tree_and_removes_extra_entries(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
):
    canonical, mirror = _configure_temporary_skill_mirror(monkeypatch, tmp_path)
    _copy_temporary_skill(canonical, mirror)
    (mirror / "SKILL.md").write_text("stale\n", encoding="utf-8")
    (mirror / "obsolete" / "extra.md").parent.mkdir()
    (mirror / "obsolete" / "extra.md").write_text("extra\n", encoding="utf-8")

    assert sync_workspace_skill_mirror.main([]) == 0
    assert sync_workspace_skill_mirror.main(["--check"]) == 0
    assert not (mirror / "obsolete").exists()


@pytest.mark.skipif(os.name == "nt", reason="creating symlinks is not portable on Windows CI")
def test_skill_mirror_never_follows_a_canonical_symlink(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
):
    canonical, mirror = _configure_temporary_skill_mirror(monkeypatch, tmp_path)
    outside = tmp_path / "outside-private.txt"
    outside.write_text("private\n", encoding="utf-8")
    (canonical / "references" / "escape.md").symlink_to(outside)

    assert sync_workspace_skill_mirror.main([]) == 2
    assert "unsafe symlink" in capsys.readouterr().err
    assert not mirror.exists()
    assert outside.read_text(encoding="utf-8") == "private\n"


@pytest.mark.skipif(os.name == "nt", reason="creating symlinks is not portable on Windows CI")
def test_skill_mirror_sync_unlinks_an_extra_symlink_without_touching_its_target(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
):
    canonical, mirror = _configure_temporary_skill_mirror(monkeypatch, tmp_path)
    _copy_temporary_skill(canonical, mirror)
    outside = tmp_path / "outside-private.txt"
    outside.write_text("private\n", encoding="utf-8")
    (mirror / "obsolete-link").symlink_to(outside)

    assert sync_workspace_skill_mirror.main([]) == 0
    assert not (mirror / "obsolete-link").exists()
    assert outside.read_text(encoding="utf-8") == "private\n"


def test_skill_mirror_rejects_a_destination_outside_the_repository(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
):
    _configure_temporary_skill_mirror(monkeypatch, tmp_path)
    outside = tmp_path.parent / f"{tmp_path.name}-outside-mirror"
    monkeypatch.setattr(sync_workspace_skill_mirror, "COMPATIBILITY_MIRROR", outside)

    assert sync_workspace_skill_mirror.main([]) == 2
    assert "outside the repository" in capsys.readouterr().err
    assert not outside.exists()


def test_skills_directory_is_the_canonical_workspace_pack():
    assert CANONICAL_SKILL_RESOURCE == "skills"


def test_main_skill_focused_routes_exist_in_canonical_pack():
    contents = Path("skills/canisend/SKILL.md").read_text(encoding="utf-8")
    routes = set(re.findall(r"\$(canisend-[a-z0-9-]+)", contents))

    assert routes
    assert all((Path("skills") / route / "SKILL.md").is_file() for route in routes)


def test_material_skills_reference_shared_canisend_rules():
    for skill_name in EXPECTED_SKILLS:
        if skill_name == "canisend":
            continue
        contents = (Path("skills") / skill_name / "SKILL.md").read_text()
        assert "../canisend/references/privacy.md" in contents
        assert "../canisend/references/quality-gates.md" in contents
        assert "Do not submit applications" in contents
        assert "Do not fabricate" in contents


def test_main_skill_routes_focused_intake_package_and_readiness_workflows():
    contents = Path("skills/canisend/SKILL.md").read_text()

    assert "$canisend-job-intake" in contents
    assert "$canisend-application-package" in contents
    assert "$canisend-submission-readiness" in contents


def test_main_skill_preserves_guarded_draft_and_review_contract():
    contents = Path("skills/canisend/SKILL.md").read_text(encoding="utf-8")
    workflow = Path("skills/canisend/references/workflow.md").read_text(encoding="utf-8")

    assert "cover_letter_draft.json" in contents
    assert "review_findings.json" in contents
    assert "stage prepare --stage draft --mode host-agent" in contents
    assert "read-private-draft-inputs" in workflow
    assert "Never write" in workflow
    assert "review_dispositions.yaml" in workflow
    assert "a blocker cannot be accepted" in workflow
    assert "does not infer whole-package" in workflow
    assert "Each Claim is emitted once" in workflow


def test_main_skill_documents_stage5_runtime_recovery_and_projection_boundaries():
    main_skill = Path("skills/canisend/SKILL.md").read_text(encoding="utf-8")
    workflow = Path("skills/canisend/references/workflow.md").read_text(encoding="utf-8")
    files = Path("skills/canisend/references/file-contracts.md").read_text(encoding="utf-8")
    quality = Path("skills/canisend/references/quality-gates.md").read_text(encoding="utf-8")
    lifecycle = Path("skills/canisend/references/job-lifecycle.md").read_text(encoding="utf-8")
    orchestration = Path("skills/canisend/references/agent-orchestration.md").read_text(encoding="utf-8")
    combined = "\n".join((main_skill, workflow, files, quality, lifecycle, orchestration))

    assert "migration inspect" in combined
    assert "repair projection" in combined
    assert "repair state" in combined
    assert "package_bundle.json" in combined
    assert "render_bundle.json" in combined
    assert "workflow/projections/package.json" in combined
    assert "registered_stage" in combined
    assert "do not hand-edit" in combined
    assert "status remains a compatibility summary" in combined


def test_package_check_requires_distributed_skill_pack_resources():
    resources = set(required_wheel_resources())

    expected = {
        "canisend/resources/.codex-plugin/plugin.json",
        "canisend/resources/skills/canisend/SKILL.md",
        "canisend/resources/skills/canisend/references/privacy.md",
        "canisend/resources/skills/canisend/references/job-lifecycle.md",
        "canisend/resources/skills/canisend/references/workflow.md",
        "canisend/resources/skills/canisend-job-intake/SKILL.md",
        "canisend/resources/skills/canisend-job-intake/agents/openai.yaml",
        "canisend/resources/skills/canisend-application-package/SKILL.md",
        "canisend/resources/skills/canisend-application-package/agents/openai.yaml",
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
        "canisend/resources/skills/canisend-submission-readiness/SKILL.md",
        "canisend/resources/skills/canisend-submission-readiness/agents/openai.yaml",
        "canisend/resources/schemas/migration-plan.schema.json",
        "canisend/resources/schemas/migration-receipt.schema.json",
        "canisend/resources/schemas/migration-rollback-receipt.schema.json",
        "canisend/resources/schemas/repair-receipt.schema.json",
        "canisend/resources/examples/orchestration/README.md",
        "canisend/resources/examples/orchestration/registered-parse.example.yaml",
        "canisend/resources/docs/stage5-migration.md",
    }

    assert expected <= resources
    assert "canisend/resources/platform-bridges/GEMINI.md" not in resources


def test_platform_bridges_bootstrap_agent_context_and_privacy_boundary():
    for bridge_path in [Path("platform-bridges/AGENTS.md"), Path("platform-bridges/CLAUDE.md")]:
        bridge = bridge_path.read_text(encoding="utf-8")

        assert "agent-skills/canisend/SKILL.md" in bridge
        assert "canisend agent context --workspace . --format json" in bridge
        assert "Ask first" in bridge
        assert "Do not stage private files" in bridge


def test_agent_guidance_keeps_criteria_and_match_bodies_ask_first_tier_two():
    main_skill = Path("skills/canisend/SKILL.md").read_text(encoding="utf-8")
    privacy = Path("skills/canisend/references/privacy.md").read_text(encoding="utf-8")
    criteria_skill = Path("skills/canisend-criteria-check/SKILL.md").read_text(encoding="utf-8")
    fit_skill = Path("skills/canisend-job-fit/SKILL.md").read_text(encoding="utf-8")

    for body in (main_skill, privacy, criteria_skill, fit_skill):
        assert "criteria.json" in body
        assert "criterion_matches.json" in body
        assert "Tier 2" in body
    for bridge_path in (Path("platform-bridges/AGENTS.md"), Path("platform-bridges/CLAUDE.md")):
        bridge = bridge_path.read_text(encoding="utf-8")
        ask_first = bridge[bridge.index("Ask first") : bridge.index("Never do")]
        assert "criteria.json" in ask_first
        assert "criterion_matches.json" in ask_first


def test_agent_guidance_keeps_brief_and_document_plan_ask_first_tier_two():
    bodies = [
        Path("skills/canisend/SKILL.md").read_text(encoding="utf-8"),
        Path("skills/canisend/references/privacy.md").read_text(encoding="utf-8"),
        Path("skills/canisend/references/file-contracts.md").read_text(encoding="utf-8"),
        Path("skills/canisend-application-package/SKILL.md").read_text(encoding="utf-8"),
        Path("skills/canisend-submission-readiness/SKILL.md").read_text(encoding="utf-8"),
    ]

    for body in bodies:
        assert "application_brief.yaml" in body
        assert "required_document_plan.json" in body
        assert "Tier 2" in body

    main_skill = bodies[0]
    assert "brief status|init|update" in main_skill
    assert "revision/hash CAS" in main_skill
    assert "required + omit" in main_skill
    assert "confirmed_empty" in main_skill
    assert "Stage 2 is locally accepted" in main_skill
    assert "Draft/package readiness does not follow" in main_skill
    assert "Stage 2" in main_skill

    for bridge_path in (Path("platform-bridges/AGENTS.md"), Path("platform-bridges/CLAUDE.md")):
        bridge = bridge_path.read_text(encoding="utf-8")
        ask_first = bridge[bridge.index("Ask first") : bridge.index("Never do")]
        assert "application_brief.yaml" in ask_first
        assert "required_document_plan.json" in ask_first
        assert "Tier 2" in ask_first


def test_brief_product_guidance_keeps_control_plane_body_free_and_blocking():
    privacy = Path("skills/canisend/references/privacy.md").read_text(encoding="utf-8")
    workflow = Path("skills/canisend/references/workflow.md").read_text(encoding="utf-8")
    quality = Path("skills/canisend/references/quality-gates.md").read_text(encoding="utf-8")
    combined = "\n".join((privacy, workflow, quality))

    assert "body-free" in combined
    assert "one bounded strict patch" in combined
    assert "--confirm-user-owned-write" in combined
    assert "empty Parsed Job" in combined
    assert "orphaned" in combined
    assert "Draft/Verify" in combined
    assert "configured provider" in combined
    assert "platform API" in combined


def test_agent_guidance_treats_imported_source_instructions_as_untrusted_data():
    main_skill = Path("skills/canisend/SKILL.md").read_text(encoding="utf-8")
    privacy = Path("skills/canisend/references/privacy.md").read_text(encoding="utf-8")
    orchestration = Path("skills/canisend/references/agent-orchestration.md").read_text(encoding="utf-8")
    combined = "\n".join([main_skill, privacy, orchestration]).lower()

    assert "untrusted data" in combined
    assert "tool instructions" in combined
    assert "cannot change allowed paths" in combined
    assert "deterministic" in combined
    assert "intrinsically immune" not in combined


def test_export_skills_writes_codex_plugin_distribution(tmp_path):
    target = tmp_path / "plugins" / "canisend"
    runner = CliRunner()

    result = runner.invoke(app, ["export-skills", "--target", str(target), "--kind", "codex-plugin"])

    assert result.exit_code == 0
    assert (target / ".codex-plugin" / "plugin.json").exists()
    assert (target / "skills" / "canisend" / "SKILL.md").exists()
    assert (target / "skills" / "canisend-research-statement" / "SKILL.md").exists()
    assert (target / "skills" / "canisend-job-intake" / "SKILL.md").exists()
    assert (target / "skills" / "canisend-application-package" / "SKILL.md").exists()
    assert (target / "skills" / "canisend-submission-readiness" / "SKILL.md").exists()


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
    assert (target / "canisend-job-intake" / "SKILL.md").exists()
    assert (target / "canisend-application-package" / "SKILL.md").exists()
    assert (target / "canisend-submission-readiness" / "SKILL.md").exists()


def test_export_skills_refuses_non_empty_target_without_overwrite(tmp_path):
    target = tmp_path / "plugins" / "canisend"
    target.mkdir(parents=True)
    (target / "local.txt").write_text("keep me\n")
    runner = CliRunner()

    result = runner.invoke(app, ["export-skills", "--target", str(target), "--kind", "codex-plugin"])

    assert result.exit_code != 0
    assert "Use --overwrite" in _strip_ansi(result.output)
    assert (target / "local.txt").read_text() == "keep me\n"
