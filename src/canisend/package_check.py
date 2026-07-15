from __future__ import annotations

from pathlib import Path
import sys
import zipfile


REQUIRED_WHEEL_RESOURCES = [
    "canisend/resources/.env.example",
    "canisend/resources/prompts/job_parser.md",
    "canisend/resources/prompts/profile_matcher.md",
    "canisend/resources/prompts/cover_letter_writer.md",
    "canisend/resources/prompts/cv_tailor.md",
    "canisend/resources/prompts/criteria_checker.md",
    "canisend/resources/prompts/package_builder.md",
    "canisend/resources/prompts/profile_evidence_augmenter.md",
    "canisend/resources/prompts/structured_cover_letter_draft.md",
    "canisend/resources/templates/typst/cover_letter.typ",
    "canisend/resources/templates/typst/application_package.typ",
    "canisend/resources/schemas/parsed_job.schema.json",
    "canisend/resources/schemas/agent-response.schema.json",
    "canisend/resources/schemas/job-lead-v2.schema.json",
    "canisend/resources/schemas/lead-catalog-v1.schema.json",
    "canisend/resources/schemas/discovery-sources-v1.schema.json",
    "canisend/resources/schemas/lead-batch-v1.schema.json",
    "canisend/resources/schemas/discovery-cache-v1.schema.json",
    "canisend/resources/schemas/discovery-refresh-report-v1.schema.json",
    "canisend/resources/schemas/discovery-import-report-v1.schema.json",
    "canisend/resources/schemas/discovery-search-v1.schema.json",
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
    "canisend/resources/schemas/research-statement-draft.schema.json",
    "canisend/resources/schemas/review-findings.schema.json",
    "canisend/resources/schemas/review-dispositions.schema.json",
    "canisend/resources/schemas/document-readiness.schema.json",
    "canisend/resources/schemas/document-execution-plan.schema.json",
    "canisend/resources/schemas/package-review-findings.schema.json",
    "canisend/resources/schemas/package-review-dispositions.schema.json",
    "canisend/resources/schemas/application-package-readiness.schema.json",
    "canisend/resources/schemas/user-mutation-receipt.schema.json",
    "canisend/resources/.codex-plugin/plugin.json",
    "canisend/resources/skills/canisend/SKILL.md",
    "canisend/resources/skills/canisend/agents/openai.yaml",
    "canisend/resources/skills/canisend/references/privacy.md",
    "canisend/resources/skills/canisend/references/quality-gates.md",
    "canisend/resources/skills/canisend/references/file-contracts.md",
    "canisend/resources/skills/canisend/references/job-lifecycle.md",
    "canisend/resources/skills/canisend/references/workflow.md",
    "canisend/resources/skills/canisend/references/agent-orchestration.md",
    "canisend/resources/skills/canisend/references/platforms.md",
    "canisend/resources/skills/canisend/references/provider-config.md",
    "canisend/resources/skills/canisend/references/typst-profile.md",
    "canisend/resources/skills/canisend-job-intake/SKILL.md",
    "canisend/resources/skills/canisend-job-intake/agents/openai.yaml",
    "canisend/resources/skills/canisend-application-package/SKILL.md",
    "canisend/resources/skills/canisend-application-package/agents/openai.yaml",
    "canisend/resources/skills/canisend-job-fit/SKILL.md",
    "canisend/resources/skills/canisend-job-fit/agents/openai.yaml",
    "canisend/resources/skills/canisend-research-statement/SKILL.md",
    "canisend/resources/skills/canisend-research-statement/agents/openai.yaml",
    "canisend/resources/skills/canisend-teaching-statement/SKILL.md",
    "canisend/resources/skills/canisend-teaching-statement/agents/openai.yaml",
    "canisend/resources/skills/canisend-cover-letter/SKILL.md",
    "canisend/resources/skills/canisend-cover-letter/agents/openai.yaml",
    "canisend/resources/skills/canisend-cv-tailoring/SKILL.md",
    "canisend/resources/skills/canisend-cv-tailoring/agents/openai.yaml",
    "canisend/resources/skills/canisend-humanizer/SKILL.md",
    "canisend/resources/skills/canisend-humanizer/agents/openai.yaml",
    "canisend/resources/skills/canisend-application-email/SKILL.md",
    "canisend/resources/skills/canisend-application-email/agents/openai.yaml",
    "canisend/resources/skills/canisend-interview-prep/SKILL.md",
    "canisend/resources/skills/canisend-interview-prep/agents/openai.yaml",
    "canisend/resources/skills/canisend-criteria-check/SKILL.md",
    "canisend/resources/skills/canisend-criteria-check/agents/openai.yaml",
    "canisend/resources/skills/canisend-material-review/SKILL.md",
    "canisend/resources/skills/canisend-material-review/agents/openai.yaml",
    "canisend/resources/skills/canisend-submission-readiness/SKILL.md",
    "canisend/resources/skills/canisend-submission-readiness/agents/openai.yaml",
    "canisend/resources/agent-skills/canisend/SKILL.md",
    "canisend/resources/agent-skills/canisend/agents/openai.yaml",
    "canisend/resources/agent-skills/canisend/references/provider-config.md",
    "canisend/resources/agent-skills/canisend/references/quality-gates.md",
    "canisend/resources/agent-skills/canisend/references/file-contracts.md",
    "canisend/resources/agent-skills/canisend/references/agent-orchestration.md",
    "canisend/resources/agent-skills/canisend/references/job-lifecycle.md",
    "canisend/resources/agent-skills/canisend/references/platforms.md",
    "canisend/resources/agent-skills/canisend/references/privacy.md",
    "canisend/resources/agent-skills/canisend/references/typst-profile.md",
    "canisend/resources/agent-skills/canisend/references/workflow.md",
    "canisend/resources/platform-bridges/AGENTS.md",
    "canisend/resources/platform-bridges/CLAUDE.md",
    "canisend/resources/examples/end_to_end/README.md",
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
]


def required_wheel_resources() -> list[str]:
    return list(REQUIRED_WHEEL_RESOURCES)


def missing_wheel_resources(wheel_path: Path) -> list[str]:
    with zipfile.ZipFile(wheel_path) as wheel:
        names = set(wheel.namelist())
    return [resource for resource in REQUIRED_WHEEL_RESOURCES if resource not in names]


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if not args:
        print("Usage: python -m canisend.package_check dist/*.whl", file=sys.stderr)
        return 2

    exit_code = 0
    for raw_path in args:
        wheel_path = Path(raw_path)
        missing = missing_wheel_resources(wheel_path)
        if missing:
            exit_code = 1
            print(f"{wheel_path}: missing packaged resources:", file=sys.stderr)
            for resource in missing:
                print(f"- {resource}", file=sys.stderr)
        else:
            print(f"{wheel_path}: packaged resources ok")
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
