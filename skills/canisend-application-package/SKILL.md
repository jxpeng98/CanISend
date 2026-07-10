---
name: canisend-application-package
description: Use when building, regenerating, integrating, or reviewing a complete CanISend application package across the cover letter, CV tailoring, statements, criteria, and Typst sources.
---

# CanISend Application Package

Focus on constructing the whole evidence-backed package for one job. Use material-specific skills for deep work on a single document.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, citations, qualifications, motivation, fit, required documents, or review results.
- Ask before reading full private CVs, statements, references, adverts, source URLs, PDFs, or generated packages.
- Prefer `profile/generated/` evidence and current structured job artifacts over raw private sources.
- Do not edit original profile inputs during ordinary package work; record profile-improvement suggestions in the job folder.

## Required References

Read only what the current package requires:

- `../canisend/references/privacy.md`: private-material, quoting, and git-safety rules.
- `../canisend/references/quality-gates.md`: advert, evidence, draft, Typst, and package gates.
- `../canisend/references/file-contracts.md`: required package artifacts and editable Typst sources.
- `../canisend/references/workflow.md`: deterministic and explicitly approved provider-backed flows.

## Workflow

1. Identify the workspace and job, then run or request `canisend doctor --workspace <private-workspace>`.
2. Confirm the job contains a complete advert. Route lead-only jobs to `$canisend-job-intake`.
3. Refresh normalized evidence with `extract-profile-evidence` when profile sources are newer.
4. Generate the deterministic baseline with `canisend run`. Enable LLM-backed parsing or drafting only after explicit user approval.
5. Verify `parsed_job.json` against the advert and map every essential criterion to evidence or a clear gap.
6. Resolve language, writing style, motivation, emphasis, risk, and exclusion questions before final prose edits.
7. Integrate the fit report, cover letter, CV tailoring notes, criteria checklist, required statements, remaining actions, and editable Typst sources. Keep every strong claim proportionate and traceable.
8. Review `07_material_review_checklist.md` and the relevant quality gates. Render only when the user asks.
9. Report generated or changed artifacts, unresolved evidence gaps, missing documents, and unchecked gates. Hand the package to `$canisend-submission-readiness` for a strict final review.
