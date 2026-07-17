from pathlib import Path


def test_v1_contract_files_exist():
    root = Path(__file__).resolve().parents[1]
    expected_files = [
        "templates/typst/cover_letter.typ",
        "templates/typst/cv_notes.typ",
        "templates/typst/application_package.typ",
        "examples/end_to_end/README.md",
        "examples/end_to_end/jobs_ac_uk_sample.xml",
        "examples/end_to_end/full_job_advert.md",
        "examples/end_to_end/fake_llm_provider.py",
        "examples/discovery/README.md",
        "examples/discovery/discovery-sources.example.yaml",
        "examples/discovery/normalized-search.example.json",
        "examples/discovery/local-leads.example.csv",
        "examples/discovery/greenhouse-list.fixture.json",
        "examples/discovery/lever-list.fixture.json",
        "prompts/job_parser.md",
        "prompts/profile_matcher.md",
        "prompts/cover_letter_writer.md",
        "prompts/cv_tailor.md",
        "prompts/criteria_checker.md",
        "prompts/package_builder.md",
        "prompts/profile_evidence_augmenter.md",
        "prompts/structured_cover_letter_draft.md",
        "agent-skills/canisend/SKILL.md",
        "agent-skills/canisend/agents/openai.yaml",
        "agent-skills/canisend/references/workflow.md",
        "agent-skills/canisend/references/file-contracts.md",
        "agent-skills/canisend/references/typst-profile.md",
        "agent-skills/canisend/references/agent-orchestration.md",
        "agent-skills/canisend/references/job-lifecycle.md",
        "agent-skills/canisend/references/platforms.md",
        "agent-skills/canisend/references/privacy.md",
        "agent-skills/canisend/references/provider-config.md",
        "agent-skills/canisend/references/quality-gates.md",
        "platform-bridges/AGENTS.md",
        "platform-bridges/CLAUDE.md",
        "scripts/release.sh",
        "scripts/smoke_decision_spine.py",
        "scripts/smoke_discovery.py",
        "scripts/sync_workspace_skill_mirror.py",
        "docs/stage3-migration.md",
        "docs/stage4-migration.md",
        "schemas/parsed_job.schema.json",
        "schemas/fit_report.schema.json",
        "schemas/criteria_check.schema.json",
        "schemas/agent-response.schema.json",
        "schemas/job-lead-v2.schema.json",
        "schemas/lead-catalog-v1.schema.json",
        "schemas/discovery-sources-v1.schema.json",
        "schemas/lead-batch-v1.schema.json",
        "schemas/discovery-cache-v1.schema.json",
        "schemas/discovery-refresh-report-v1.schema.json",
        "schemas/discovery-import-report-v1.schema.json",
        "schemas/discovery-search-v1.schema.json",
        "schemas/criteria.schema.json",
        "schemas/criterion-matches.schema.json",
        "schemas/confirmed-corrections.schema.json",
        "schemas/application-decision.schema.json",
        "schemas/application-brief.schema.json",
        "schemas/required-document-plan.schema.json",
        "schemas/cover-letter-draft.schema.json",
        "schemas/research-statement-draft.schema.json",
        "schemas/review-findings.schema.json",
        "schemas/review-dispositions.schema.json",
        "schemas/document-readiness.schema.json",
        "schemas/document-execution-plan.schema.json",
        "schemas/package-review-findings.schema.json",
        "schemas/package-review-dispositions.schema.json",
        "schemas/application-package-readiness.schema.json",
        "schemas/user-mutation-receipt.schema.json",
        "schemas/artifact-bundle.schema.json",
        "schemas/projection-journal.schema.json",
        "schemas/application-gate-report.schema.json",
    ]

    for path in expected_files:
        assert (root / path).exists()


def test_repository_native_skill_mirror_gate_blocks_ci_and_local_release():
    root = Path(__file__).resolve().parents[1]
    script = (root / "scripts/sync_workspace_skill_mirror.py").read_text(encoding="utf-8")
    ci = (root / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    release = (root / "scripts/release.sh").read_text(encoding="utf-8")
    readme = (root / "README.md").read_text(encoding="utf-8")

    check_command = "python scripts/sync_workspace_skill_mirror.py --check"
    assert check_command in ci
    assert check_command in release
    assert "missing" in script
    assert "extra" in script
    assert "content/type drift" in script
    assert "sync_hermes_tap" not in script
    assert "repository-native substitute for an external `sync_hermes_tap` contract" in readme


def test_stage3_migration_guide_is_fail_closed_and_non_destructive():
    root = Path(__file__).resolve().parents[1]
    guide = (root / "docs" / "stage3-migration.md").read_text(encoding="utf-8")

    for contract in (
        "not a user-data migration",
        "APP-Q5 fails closed",
        "reset_for_current_review",
        "reset_for_current_package_review",
        "user-mutation recover",
        "does not roll back private data",
        "is not rendering approval",
    ):
        assert contract in guide


def test_typst_templates_use_modernpro_packages():
    root = Path(__file__).resolve().parents[1]

    cover_template = (root / "templates/typst/cover_letter.typ").read_text()
    cv_template = (root / "templates/typst/cv_notes.typ").read_text()

    assert '@preview/modernpro-coverletter:0.0.8' in cover_template
    assert '@preview/modernpro-cv:1.3.0' in cv_template


def test_docs_record_rss_and_privacy_contracts():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()
    proposal = (root / "canisend_v1_proposal.md").read_text()

    assert "jobs.ac.uk RSS" in readme
    assert "new-job-from-lead" in readme
    assert "`profile/` is ignored by git" in readme
    assert "jobs.ac.uk RSS" in proposal
    assert "new-job-from-lead" in proposal
    assert "RSS import." not in proposal


def test_agent_skill_has_standard_frontmatter_and_references():
    root = Path(__file__).resolve().parents[1]
    skill = (root / "agent-skills/canisend/SKILL.md").read_text()
    metadata = (root / "agent-skills/canisend/agents/openai.yaml").read_text()

    assert skill.startswith("---\n")
    assert "name: canisend" in skill
    assert "CanISend" in skill
    assert "这也能投" in skill
    assert "No claims without receipts" in skill
    assert "description: Use when" in skill
    assert "Codex, Claude Code, and IDE agents" in skill
    assert "Gemini" not in skill
    assert "jobs.ac.uk RSS" in skill
    assert "references/workflow.md" in skill
    assert "references/typst-profile.md" in skill
    assert "references/agent-orchestration.md" in skill
    assert "references/provider-config.md" in skill
    assert "references/quality-gates.md" in skill
    assert "references/job-lifecycle.md" in skill
    assert "references/platforms.md" in skill
    assert "$canisend" in metadata
    assert "display_name: \"CanISend\"" in metadata
    assert len(skill.splitlines()) < 140


def test_agent_skill_references_capture_operational_gates():
    root = Path(__file__).resolve().parents[1]
    references = root / "agent-skills/canisend" / "references"
    quality = (references / "quality-gates.md").read_text()
    provider = (references / "provider-config.md").read_text()
    lifecycle = (references / "job-lifecycle.md").read_text()
    platforms = (references / "platforms.md").read_text()

    assert "profile/generated/" in quality
    assert "profile/generated/file.evidence.md#Section/item-id" in quality
    assert "item-level citations are preferred" in quality.lower()
    assert "section-level citations" in quality
    assert "unknown citations fail validation" in quality
    assert "07_material_review_checklist.md" in quality
    assert "ACADEMIC_PREP_LLM_PROVIDER=command" in provider
    assert "OPENAI_BASE_URL" in provider
    assert "Gemini" not in provider
    assert "gemini" not in provider
    assert "status: lead_imported" in lifecycle
    assert "status: packaged" in lifecycle
    assert "AGENTS.md" in platforms
    assert "CLAUDE.md" in platforms
    assert "GEMINI.md" not in platforms
    assert "Gemini" not in platforms
    assert "IDE" in platforms


def test_agent_skill_defines_release_ready_agent_boundaries():
    root = Path(__file__).resolve().parents[1]
    skill = (root / "agent-skills/canisend/SKILL.md").read_text()
    privacy = (root / "agent-skills/canisend/references/privacy.md").read_text()
    quality = (root / "agent-skills/canisend/references/quality-gates.md").read_text()
    provider = (root / "agent-skills/canisend/references/provider-config.md").read_text()
    workflow = (root / "agent-skills/canisend/references/workflow.md").read_text()

    assert "Agent Contract" in skill
    assert "Allowed by default" in skill
    assert "Requires explicit user approval" in skill
    assert "Always forbidden" in skill
    assert "Do not claim materials are ready" in skill
    assert "do not stage private files" in skill.lower()
    assert "edits_profile_input" in skill

    assert "Do Not Read Unless Needed" in privacy
    assert "Original Profile Input Edits" in privacy
    assert "--confirm-profile-input-edit-again" in privacy
    assert "Do Not Quote In Chat" in privacy
    assert "Do Not Stage Or Commit" in privacy
    assert "Agent-assisted mode" in privacy
    assert "not a promise that agent-assisted or LLM-backed workflows keep all content away from models" in privacy
    assert "full job adverts" in privacy
    assert "source URLs" in privacy

    assert "Ready Claim Gate" in quality
    assert "Profile Input Edit Gate" in quality
    assert "Do not use ready, final, complete, or submission-ready" in quality
    assert "Manual Submission Gate" in quality
    assert "explicit opt-in" in provider
    assert "transmits private advert and evidence context" in provider
    assert "ask before enabling" in workflow


def test_platform_bridges_expose_agent_boundaries_immediately():
    root = Path(__file__).resolve().parents[1]
    bridges = root / "platform-bridges"

    assert not (bridges / "GEMINI.md").exists()
    for filename in ["AGENTS.md", "CLAUDE.md"]:
        bridge = (bridges / filename).read_text()
        assert "Allowed by default" in bridge
        assert "Ask first" in bridge
        assert "Never do" in bridge
        assert "Do not quote private materials" in bridge
        assert "Do not stage private files" in bridge


def test_agent_skill_documents_typst_first_item_level_evidence_contract():
    root = Path(__file__).resolve().parents[1]
    references = root / "agent-skills/canisend" / "references"
    contracts = (references / "file-contracts.md").read_text()
    typst_profile = (references / "typst-profile.md").read_text()
    workflow = (references / "workflow.md").read_text()
    orchestration = (references / "agent-orchestration.md").read_text()

    assert "profile/generated/file.evidence.md#Section/item-id" in contracts
    assert "cv-001" in contracts
    assert "#dated-entry(...)" in typst_profile
    assert "#entry(...)" in typst_profile
    assert "statement paragraphs" in typst_profile
    assert "item-level citations" in workflow
    assert "quality-gates.md" in workflow
    assert "canisend orchestrate" in orchestration
    assert "max_parallel_tasks" in orchestration
    assert "agent_count" in orchestration
    assert "supports_native_subagents" in orchestration


def test_readme_has_release_page_quick_start():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()

    assert "github/actions/workflow/status/jxpeng98/CanISend/ci.yml" in readme
    assert "TestPyPI" in readme
    assert "python-3.11%2B" in readme
    assert "license-MIT" in readme
    assert "## Quick Start" in readme
    assert "canisend run-example" in readme
    assert "/tmp/canisend-example" in readme
    assert "uv tool install" in readme
    assert "Put your real modernpro CV and statements" in readme
    assert "Fetch jobs.ac.uk RSS leads" in readme
    assert "Paste the full advert" in readme
    assert "Review item-level evidence citations" in readme
    assert "Render Typst only when needed" in readme
    assert "Submit manually" in readme


def test_platform_bridges_point_to_project_skill():
    root = Path(__file__).resolve().parents[1]
    bridges = root / "platform-bridges"

    assert not (bridges / "GEMINI.md").exists()
    for filename in ["AGENTS.md", "CLAUDE.md"]:
        bridge = (bridges / filename).read_text()
        assert "agent-skills/canisend/SKILL.md" in bridge
        assert "canisend doctor --workspace" in bridge
        assert "profile/" in bridge


def test_readme_documents_core_workflow_and_agent_usage():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()

    expected_sections = [
        "## What It Does",
        "## Quick Start",
        "## Core Workflow",
        "### 1. Initialize a private workspace",
        "### 2. Prepare profile evidence",
        "### 3. Import leads and create one job folder",
        "### 4. Generate draft materials",
        "### 5. Review, render, and submit manually",
        "## Agent Usage",
        "## Privacy Boundaries",
        "## Maintainer Release",
        "## Repository Layout",
    ]

    for section in expected_sections:
        assert section in readme
    assert "LLM-backed parser" in readme
    assert "evidence citation" in readme
    assert "prompts/" in readme
    assert "agent-skills/" in readme
    assert "profile/profile.yaml" in readme
    assert "profile/generated/" in readme
    assert "extract-profile-evidence" in readme
    assert "--llm-parser" in readme
    assert "--llm-drafts" in readme
    assert "--llm-augment" in readme
    assert "canisend orchestrate" in readme
    assert "edits_profile_input" in readme
    assert "--confirm-profile-input-edit-again" in readme
    assert "typst/cover_letter.typ" in readme
    assert "typst/application_package.typ" in readme
    assert "directly edit `typst/cover_letter.typ`" in readme
    assert "cover_letter_content.json" not in readme
    assert "07_material_review_checklist.md" in readme
    assert "examples/end_to_end" in readme
    assert "Codex, Claude Code, and IDE agents" in readme
    assert "GEMINI.md" not in readme
    assert "Gemini" not in readme
    assert "ACADEMIC_PREP_LLM_PROVIDER" in readme
    assert "AGENTS.md" in readme
    assert "CLAUDE.md" in readme


def test_readme_documents_skill_distribution_pack():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()

    assert "## Skill Distribution" in readme
    assert "canisend export-skills" in readme
    assert "git submodule add https://github.com/jxpeng98/CanISend plugins/canisend" in readme
    assert "canisend-research-statement" in readme
    assert "Codex plugin manifest" in readme


def test_docs_record_material_review_management_artifact():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()
    proposal = (root / "canisend_v1_proposal.md").read_text()
    contracts = (root / "agent-skills/canisend/references/file-contracts.md").read_text()
    workflow = (root / "agent-skills/canisend/references/workflow.md").read_text()

    assert "07_material_review_checklist.md" in readme
    assert "07_material_review_checklist.md" in proposal
    assert "07_material_review_checklist.md" in contracts
    assert "07_material_review_checklist.md" in workflow
    assert "cover letter draft, CV tailoring notes" in readme


def test_proposal_documents_prompt_skill_split_and_typst_profile():
    root = Path(__file__).resolve().parents[1]
    proposal = (root / "canisend_v1_proposal.md").read_text()

    assert "prompts/" in proposal
    assert "agent-skills/" in proposal
    assert "profile/profile.yaml" in proposal
    assert "profile/generated/" in proposal
    assert "extract-profile-evidence" in proposal
    assert "--llm-parser" in proposal
    assert "--llm-drafts" in proposal
    assert "typst/cover_letter.typ" in proposal
    assert "cover_letter_content.json" not in proposal
    assert "Codex, Claude Code, and IDE agents" in proposal
    assert "Gemini" not in proposal
    assert "Prompt files should live in `skills/`" not in proposal
