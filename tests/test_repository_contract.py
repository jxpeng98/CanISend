from pathlib import Path


def test_v1_contract_files_exist():
    root = Path(__file__).resolve().parents[1]
    expected_files = [
        "templates/typst/cover_letter.typ",
        "templates/typst/cv_notes.typ",
        "templates/typst/application_package.typ",
        "prompts/job_parser.md",
        "prompts/profile_matcher.md",
        "prompts/cover_letter_writer.md",
        "prompts/cv_tailor.md",
        "prompts/criteria_checker.md",
        "prompts/package_builder.md",
        "agent-skills/academic-application-prep/SKILL.md",
        "agent-skills/academic-application-prep/references/workflow.md",
        "agent-skills/academic-application-prep/references/file-contracts.md",
        "agent-skills/academic-application-prep/references/typst-profile.md",
        "agent-skills/academic-application-prep/references/privacy.md",
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
    assert "profile/ is ignored by git" in readme
    assert "jobs.ac.uk RSS" in proposal
    assert "RSS import." not in proposal


def test_agent_skill_has_standard_frontmatter_and_references():
    root = Path(__file__).resolve().parents[1]
    skill = (root / "agent-skills/academic-application-prep/SKILL.md").read_text()

    assert skill.startswith("---\n")
    assert "name: academic-application-prep" in skill
    assert "description: Use when" in skill
    assert "references/workflow.md" in skill
    assert "references/typst-profile.md" in skill


def test_readme_documents_complete_workflow_and_round_two_tasks():
    root = Path(__file__).resolve().parents[1]
    readme = (root / "README.md").read_text()

    expected_sections = [
        "## Complete Workflow",
        "### 1. Install and verify the CLI",
        "### 2. Prepare local private profile data",
        "### 3. Generate normalized profile evidence",
        "### 4. Fetch jobs.ac.uk RSS leads",
        "### 5. Select one advert and create a job workspace",
        "### 6. Run the application preparation pipeline",
        "### 7. Review and edit generated materials",
        "### 8. Render Typst outputs when needed",
        "### 9. Submit manually outside the tool",
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
    assert "Prompt files should live in `skills/`" not in proposal
