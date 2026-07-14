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
- Ask before reading `application_brief.yaml`, `required_document_plan.json`, `cover_letter_draft.json`,
  `review_findings.json`, or `review_dispositions.yaml`; all are Tier 2 strategy/application artifacts.
- Prefer `profile/generated/` evidence and current structured job artifacts over raw private sources.
- Prefer body-free Brief/plan status first. Never edit either authoritative artifact directly; Brief uses one strict
  scoped patch with fresh revision/hash and explicit consent, while the plan is rebuilt deterministically.
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
6. Require a current confirmed apply Decision, then inspect body-free Brief and Brief-stage status. Use
   `brief status|init|update` only with one strict patch, fresh revision/hash, and explicit consent.
7. Resolve language, writing style, motivation, emphasis, exclusions, and the document-requirement set. Empty parser
   output is not `confirmed_empty` without explicit current-basis confirmation.
8. Run deterministic `stage run --stage brief`; do not draft through an unconfirmed requirement set, unresolved
   choice, `required + omit`, missing required preparation action, or orphaned choice.
9. For the Cover Letter, use guarded host-agent Draft candidate submission/promotion, then deterministic Review.
   Resolve non-waivable blockers, then use guarded Review dispositions for every semantic-support and Claim-kind
   finding; do not edit either authoritative JSON or bypass disposition CAS.
10. Integrate the fit report, cover letter, CV tailoring notes, criteria checklist, required statements, remaining actions, and editable Typst sources. Keep every strong claim proportionate and traceable.
11. Review `07_material_review_checklist.md` and the relevant quality gates. Render only when the user asks.
12. Report generated or changed artifacts, unresolved evidence/Brief/document/Draft/Review gaps, and unchecked gates. Hand the package to `$canisend-submission-readiness` for a strict final review. This workflow alone is not package readiness.
