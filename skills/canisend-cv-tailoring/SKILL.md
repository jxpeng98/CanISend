---
name: canisend-cv-tailoring
description: Use when tailoring, reviewing, or prioritizing CV content for an academic or professional job application with CanISend evidence, job criteria, or Typst profile sources.
---

# CanISend CV Tailoring

Focus only on CV tailoring. Do not prepare a full application package unless the user explicitly switches to the main `canisend` workflow.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, publications, awards, teaching, grants, service, or employment history.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, or enabling LLM-backed commands.
- Prefer generated evidence under `profile/generated/` and cite gaps instead of inventing support.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence and readiness gates.
- `../canisend/references/file-contracts.md`: CV notes and profile evidence paths.
- `../canisend/references/typst-profile.md`: modernpro Typst profile conventions.

## Workflow

1. Identify the workspace and run or request `canisend doctor --workspace <private-workspace>`.
2. Prefer `profile/generated/` evidence and generated CV tailoring notes over raw private files.
3. Compare job criteria against profile evidence to prioritize sections, bullets, and ordering.
4. Recommend edits as targeted notes unless the user explicitly asks for full CV text.
5. Mark unsupported claims as gaps and suggest evidence the user can add.
6. Before saying CV tailoring is ready, check `../canisend/references/quality-gates.md`.
