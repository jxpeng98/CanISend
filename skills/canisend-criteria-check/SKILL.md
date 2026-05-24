---
name: canisend-criteria-check
description: Use when checking job criteria, evidence coverage, fit gaps, selection criteria, or claim support for a CanISend academic or professional job application.
---

# CanISend Criteria Check

Focus only on criteria matching and evidence coverage. Do not prepare a full application package unless the user explicitly switches to the main `canisend` workflow.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, eligibility, qualifications, experience, or criterion matches.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, or enabling LLM-backed commands.
- Prefer generated evidence under `profile/generated/` and cite gaps instead of inventing support.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence and citation gates.
- `../canisend/references/file-contracts.md`: parsed job and criteria checklist paths.
- `../canisend/references/job-lifecycle.md`: job status and next actions.

## Workflow

1. Identify the workspace and run or request `canisend doctor --workspace <private-workspace>`.
2. Prefer `parsed_job.json`, `05_criteria_checklist.md`, and `profile/generated/` evidence.
3. Classify each criterion as strong, partial, missing, or unclear based on cited evidence.
4. Flag unknown citations, weak support, and claims that exceed the evidence.
5. Recommend concrete next evidence or edits for gaps.
6. Before saying criteria coverage is ready, check `../canisend/references/quality-gates.md`.
