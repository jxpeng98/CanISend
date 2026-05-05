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
        "prompts/job_parser.md",
        "prompts/profile_matcher.md",
        "prompts/cover_letter_writer.md",
        "prompts/cv_tailor.md",
        "prompts/criteria_checker.md",
        "prompts/package_builder.md",
        "agent-skills/academic-application-prep/SKILL.md",
        "agent-skills/academic-application-prep/agents/openai.yaml",
        "agent-skills/academic-application-prep/references/workflow.md",
        "agent-skills/academic-application-prep/references/file-contracts.md",
        "agent-skills/academic-application-prep/references/typst-profile.md",
        "agent-skills/academic-application-prep/references/agent-orchestration.md",
        "agent-skills/academic-application-prep/references/job-lifecycle.md",
        "agent-skills/academic-application-prep/references/platforms.md",
        "agent-skills/academic-application-prep/references/privacy.md",
        "agent-skills/academic-application-prep/references/provider-config.md",
        "agent-skills/academic-application-prep/references/quality-gates.md",
        "platform-bridges/AGENTS.md",
        "platform-bridges/CLAUDE.md",
        "platform-bridges/GEMINI.md",
        "schemas/parsed_job.schema.json",
        "schemas/fit_report.schema.json",
        "schemas/criteria_check.schema.json",
    ]

    for path in expected_files:
        assert (root / path).exists()


def test_typst_templates_use_modernpro_packages():
    root = Path(__file__).resolve().parents[1]

    cover_template = (root / "templates/typst/cover_letter.typ").read_text()
    cv_template = (root / "templates/typst/cv_notes.typ").read_text()

    assert '@preview/modernpro-coverletter:0.0.8' in cover_template
    assert '@preview/modernpro-cv:1.3.0' in cv_template


def test_docs_record_rss_and_privacy_contracts():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()
    proposal = (root / "academic_application_prep_copilot_proposal.md").read_text()

    assert "jobs.ac.uk RSS" in readme
    assert "new-job-from-lead" in readme
    assert "`profile/` is ignored by git" in readme
    assert "jobs.ac.uk RSS" in proposal
    assert "new-job-from-lead" in proposal
    assert "RSS import." not in proposal


def test_agent_skill_has_standard_frontmatter_and_references():
    root = Path(__file__).resolve().parents[1]
    skill = (root / "agent-skills/academic-application-prep/SKILL.md").read_text()
    metadata = (root / "agent-skills/academic-application-prep/agents/openai.yaml").read_text()

    assert skill.startswith("---\n")
    assert "name: academic-application-prep" in skill
    assert "description: Use when" in skill
    assert "Codex, Claude Code, Gemini" in skill
    assert "jobs.ac.uk RSS" in skill
    assert "references/workflow.md" in skill
    assert "references/typst-profile.md" in skill
    assert "references/agent-orchestration.md" in skill
    assert "references/provider-config.md" in skill
    assert "references/quality-gates.md" in skill
    assert "references/job-lifecycle.md" in skill
    assert "references/platforms.md" in skill
    assert "$academic-application-prep" in metadata
    assert "display_name: \"Academic Application Prep\"" in metadata
    assert len(skill.splitlines()) < 120


def test_agent_skill_references_capture_operational_gates():
    root = Path(__file__).resolve().parents[1]
    references = root / "agent-skills/academic-application-prep" / "references"
    quality = (references / "quality-gates.md").read_text()
    provider = (references / "provider-config.md").read_text()
    lifecycle = (references / "job-lifecycle.md").read_text()
    platforms = (references / "platforms.md").read_text()

    assert "profile/generated/" in quality
    assert "unknown citations fail validation" in quality
    assert "ACADEMIC_PREP_LLM_PROVIDER=command" in provider
    assert "OPENAI_BASE_URL" in provider
    assert "status: lead_imported" in lifecycle
    assert "status: packaged" in lifecycle
    assert "AGENTS.md" in platforms
    assert "CLAUDE.md" in platforms
    assert "GEMINI.md" in platforms
    assert "IDE" in platforms


def test_platform_bridges_point_to_project_skill():
    root = Path(__file__).resolve().parents[1]
    bridges = root / "platform-bridges"

    for filename in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"]:
        bridge = (bridges / filename).read_text()
        assert "agent-skills/academic-application-prep/SKILL.md" in bridge
        assert "academic-prep doctor --workspace" in bridge
        assert "profile/" in bridge


def test_readme_documents_complete_workflow_and_round_two_tasks():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()

    expected_sections = [
        "## Complete Workflow",
        "### 1. Install and verify the CLI",
        "### 2. Initialize a private workspace",
        "### 3. Prepare local private profile data",
        "### 4. Generate normalized profile evidence",
        "### 5. Fetch jobs.ac.uk RSS leads",
        "### 6. Select one advert and create a job workspace",
        "### 7. Run the application preparation pipeline",
        "### 8. Review and edit generated materials",
        "### 9. Render Typst outputs when needed",
        "### 10. Submit manually outside the tool",
        "## Round 2 Task Queue",
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
    assert "cover_letter_content.json" in readme
    assert "examples/end_to_end" in readme
    assert "Codex, Claude Code, Gemini" in readme
    assert "ACADEMIC_PREP_LLM_PROVIDER" in readme


def test_proposal_documents_prompt_skill_split_and_typst_profile():
    root = Path(__file__).resolve().parents[1]
    proposal = (root / "academic_application_prep_copilot_proposal.md").read_text()

    assert "prompts/" in proposal
    assert "agent-skills/" in proposal
    assert "profile/profile.yaml" in proposal
    assert "profile/generated/" in proposal
    assert "extract-profile-evidence" in proposal
    assert "--llm-parser" in proposal
    assert "--llm-drafts" in proposal
    assert "cover_letter_content.json" in proposal
    assert "Codex, Claude Code, Gemini" in proposal
    assert "Prompt files should live in `skills/`" not in proposal
