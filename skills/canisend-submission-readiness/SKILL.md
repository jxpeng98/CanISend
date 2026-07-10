---
name: canisend-submission-readiness
description: Use when performing a strict final readiness review of a CanISend application package, checking blockers, required documents, evidence, privacy, Typst outputs, and manual-submission boundaries.
---

# CanISend Submission Readiness

Review the complete package as a strict final gate. Report blockers and unchecked gates; do not silently repair or submit the application.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, citations, required-document coverage, review results, or readiness.
- Ask before reading full private CVs, statements, references, adverts, source URLs, PDFs, or generated packages.
- Prefer existing checklists, structured metadata, and narrow evidence receipts before raw private sources.
- Never describe an application as submitted. Claim readiness only for gates actually checked.

## Required References

Read these as needed for the current review:

- `../canisend/references/privacy.md`: private-source, quoting, and staging rules.
- `../canisend/references/quality-gates.md`: authoritative readiness and manual-submission gates.
- `../canisend/references/file-contracts.md`: expected package files and editable source contracts.
- `../canisend/references/job-lifecycle.md`: expected state before final review.

## Workflow

1. Identify the current workspace and job, then run or request `canisend doctor --workspace <private-workspace>`.
2. Run or inspect `canisend check-package --workspace <private-workspace> --job jobs/<job-slug>` without treating it as sufficient by itself.
3. Follow the review order in `quality-gates.md`: metadata, full advert, parsed job, preparation questions, criteria, fit, drafts, review checklist, Typst sources, and final package.
4. Verify every essential criterion is visible and evidence-backed, every required document is present or explicitly outstanding, and no placeholder or unsupported claim remains.
5. Check language/style decisions, item-level citations, source consistency, rendering status when relevant, and private-file staging risk.
6. Classify every finding as `blocker`, `warning`, or `pass`. Any missing essential criterion, incomplete advert, unknown citation, unresolved placeholder, missing required document, or unchecked privacy gate is a blocker.
7. If blockers exist, route corrections to `$canisend-application-package` or a material-specific skill and require another readiness pass.
8. If all relevant gates were checked and passed, say the package was reviewed for manual submission. The user still handles every portal action and sensitive declaration.

## Output

Report the verdict, blockers, warnings, passed gates, unchecked items, files reviewed, private sources read, provider-backed flags used, and the next manual action.
