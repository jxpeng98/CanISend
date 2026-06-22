---
name: canisend-teaching-statement
description: Use when drafting, revising, tailoring, or reviewing a teaching statement for an academic job application with CanISend evidence, teaching criteria, or profile materials.
---

# CanISend Teaching Statement

Focus only on teaching statements and teaching-focused sections. Do not prepare a full application package unless the user explicitly switches to the main `canisend` workflow.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, teaching roles, course names, pedagogy, student outcomes, awards, supervision, or teaching philosophy.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, PDFs, or enabling LLM-backed commands.
- Prefer generated evidence under `profile/generated/`, teaching-related profile sources, criteria checklists, and existing drafts over raw private files.
- Mark missing teaching evidence as a gap instead of turning generic pedagogy into unsupported claims.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence and readiness gates.
- `../canisend/references/file-contracts.md`: statement and evidence artifact paths.
- `../canisend/references/typst-profile.md`: profile and Typst conventions when statement sources are involved.
- `../canisend/references/workflow.md`: local-first generation sequence.

## Workflow

1. Identify the workspace and run or request `canisend doctor --workspace <private-workspace>`.
2. Prefer `profile/generated/` evidence, teaching-related profile files, job criteria, and existing statement drafts.
3. Map the statement to the role's teaching needs: subject areas, student groups, assessment, supervision, inclusivity, employability, or curriculum design.
4. Build paragraphs around concrete teaching evidence, not generic teaching values.
5. Connect philosophy to practice: principle, example, result, and relevance to the target role.
6. Keep claims proportionate and cite gaps where evidence is thin.
7. Before saying the statement is ready, check `../canisend/references/quality-gates.md`.

## Output

For drafting, provide statement text plus evidence notes when claims are material.

For review, return:

- Teaching criteria covered
- Evidence-backed strengths
- Unsupported or generic claims
- Suggested revisions by paragraph
