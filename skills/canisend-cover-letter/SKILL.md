---
name: canisend-cover-letter
description: Use when drafting, revising, tailoring, or reviewing a cover letter for an academic or professional job application with CanISend evidence, job criteria, or workspace materials.
---

# CanISend Cover Letter

Focus only on cover letters. Do not prepare a full application package unless the user explicitly switches to the main `canisend` workflow.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, achievements, relationships, teaching experience, research claims, or fit.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, or enabling LLM-backed commands.
- Prefer generated evidence under `profile/generated/` and cite gaps instead of inventing support.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence and readiness gates.
- `../canisend/references/file-contracts.md`: cover letter and job artifact paths.
- `../canisend/references/workflow.md`: local-first generation sequence.

## Workflow

1. Identify the workspace and run or request `canisend doctor --workspace <private-workspace>`.
2. Prefer `profile/generated/` evidence, `parsed_job.json`, and criteria checklists over raw private files.
3. Map the letter opening, fit paragraphs, and closing to job criteria and evidence citations.
4. Keep claims specific, verifiable, and proportionate to the source evidence.
5. Mark unsupported claims as gaps and suggest evidence the user can add.
6. Before saying the cover letter is ready, check `../canisend/references/quality-gates.md`.
