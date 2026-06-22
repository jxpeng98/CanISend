---
name: canisend-job-fit
description: Use when assessing job fit, prioritizing application strategy, mapping role criteria to CanISend evidence, or deciding whether and how to apply for a specific job.
---

# CanISend Job Fit

Focus only on fit analysis and application strategy. Do not draft the full application package unless the user explicitly switches to a material-specific skill or the main `canisend` workflow.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, motivation, eligibility, experience, publications, teaching, service, or institutional fit.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, PDFs, or enabling LLM-backed commands.
- Prefer generated evidence under `profile/generated/`, `parsed_job.json`, and criteria checklists over raw private files.
- Treat missing essential criteria as strategy risks, not wording problems.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence and readiness gates.
- `../canisend/references/file-contracts.md`: job, criteria, and evidence artifact paths.
- `../canisend/references/workflow.md`: local-first generation sequence.

## Workflow

1. Identify the workspace and run or request `canisend doctor --workspace <private-workspace>`.
2. Prefer `parsed_job.json`, `05_criteria_checklist.md`, `02_fit_report.md`, and `profile/generated/` evidence.
3. Separate essential criteria, desirable criteria, role priorities, and implicit fit signals.
4. Map each criterion to evidence, weaker support, or a clear gap.
5. Recommend an application angle: strongest fit, credible stretch, high-risk application, or not worth prioritizing.
6. Suggest which materials need targeted work: cover letter, CV, research statement, teaching statement, criteria response, or evidence extraction.
7. Before saying the application is strategically ready, check `../canisend/references/quality-gates.md`.

## Output

Use a compact decision format:

- Fit verdict
- Strongest evidence-backed selling points
- Material risks or unsupported claims
- Recommended application angle
- Next CanISend skill or CLI step
