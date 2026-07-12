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
- Ask before reading Tier 2 `application_brief.yaml` or `required_document_plan.json`; prefer their body-free status
  first.
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
3. Follow the review order in `quality-gates.md`: metadata, advert, parsed job, Criteria/Match/Decision, Brief and
   required-document plan, preparation questions, drafts, review checklist, Typst sources, and final package.
4. Require a current confirmed apply Decision and current deterministic Brief plan. Empty requirements are not
   `confirmed_empty` unless explicitly confirmed against the current basis.
5. Verify every essential criterion is evidence-backed and every required document has a prepared/reviewed artifact.
   An unconfirmed set/field, unresolved choice, `required + omit`, missing preparation action, or orphaned choice is a
   blocker even when the omission itself was explicitly recorded.
6. Check language/style decisions, item-level citations, source consistency, rendering status when relevant, and private-file staging risk.
7. Classify every finding as `blocker`, `warning`, or `pass`. Any missing essential criterion, incomplete advert,
   unknown citation, unresolved placeholder/document-plan blocker, missing required document, or unchecked privacy
   gate is a blocker.
8. If blockers exist, route corrections to `$canisend-application-package` or a material-specific skill and require another readiness pass.
9. If all relevant gates were checked and passed, say the package was reviewed for manual submission. The user still handles every portal action and sensitive declaration. Do not treat in-progress Task 6 implementation as readiness by itself.

## Output

Report the verdict, blockers, warnings, passed gates, unchecked items, files reviewed, private sources read, provider-backed flags used, and the next manual action.
