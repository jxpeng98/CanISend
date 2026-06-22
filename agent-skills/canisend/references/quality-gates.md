# Quality Gates

Use this reference before saying generated application materials are ready for user review, Typst rendering, or manual submission.

## Ready Claim Gate

Do not use ready, final, complete, or submission-ready for generated materials until the relevant gates below have been checked against the current job folder. If a gate is not checked, say what remains unchecked instead.

## Profile Evidence Gate

- `profile/profile.yaml` exists.
- `profile/generated/` contains current evidence files.
- Strong claims cite normalized evidence, not private Typst prose directly.
- Every citation points to an existing generated evidence file, section, and item when an item ID exists.
- New agent output should use item-level citations such as `profile/generated/file.evidence.md#Section/item-id`.
- Item-level citations are preferred; section-level citations are compatibility only for older outputs or manually reviewed fallbacks.
- Missing evidence is reported as a gap.

## Job Advert Gate

- RSS-created jobs no longer rely only on RSS description.
- `job_advert.md` contains the full advert or the user has explicitly accepted partial processing.
- `parsed_job.json` title, institution, deadline, required documents, essential criteria, and source URL match the advert.
- Missing advert fields are left empty or unknown; do not invent.

## Draft Gate

- `02_fit_report.md` separates strong fit, partial fit, and gaps.
- `03_cover_letter_draft.md` is application-facing English and does not include unsupported claims.
- `04_cv_tailoring_notes.md` tells the user what to adjust in the private CV, but does not rewrite the CV unless asked.
- `05_criteria_checklist.md` covers all extracted essential criteria.
- `07_material_review_checklist.md` tracks cover letter draft and CV tailoring notes review actions before Typst rendering.
- Each non-missing criterion row cites item-level evidence when evidence exists.
- LLM-backed drafts with unknown citations fail validation; unknown citations fail validation by design.

## Typst Gate

- `typst/cover_letter.typ` directly contains the cover letter text and stable section markers.
- `typst/application_package.typ` directly contains the package text, criteria checklist, and remaining actions.
- Generated `.typ` files use `modernpro-coverletter` or `modernpro-cv` templates rather than Markdown-to-Typst conversion.
- PDF rendering is optional and requires local Typst.

## Privacy Gate

- Real `profile/`, `jobs/`, `job_leads/`, PDFs, `.env`, and private source URLs are not staged.
- Sensitive declarations are left to the user.
- The final package is a preparation dossier, not proof of submission.
- Agent-assisted reviews report any full private sources read directly.
- LLM-backed CLI runs report which flags/providers were used.

## Profile Input Edit Gate

- Profile improvement ideas are recorded as suggestions before source files are changed.
- Original `profile/` inputs outside `profile/generated/` are edited only through `canisend orchestrate`.
- Any profile-input edit task declares `edits_profile_input: true`, uses privacy tier 2+, depends on a prior review task, and writes only the intended profile source file.
- The run includes both profile edit confirmations: `--confirm-profile-input-edit` and `--confirm-profile-input-edit-again`.

## Manual Submission Gate

- The tool stops at preparation; the user submits manually outside CanISend.
- Portal uploads, account creation, equality monitoring, visa, right-to-work, disability, health, criminal record, conflict, and other sensitive declarations remain user-only.
- Generated material can be described as reviewed only for the checks actually performed.

## Review Order

Review files in this order:

1. `job.yaml`
2. `job_advert.md`
3. `parsed_job.json`
4. `00_preparation_questions.md`
5. `05_criteria_checklist.md`
6. `02_fit_report.md`
7. `03_cover_letter_draft.md`
8. `04_cv_tailoring_notes.md`
9. `07_material_review_checklist.md`
10. `typst/cover_letter.typ`
11. `typst/application_package.typ`
12. `06_final_application_package.md`

Before editing prose, confirm `00_preparation_questions.md` has resolved US English vs UK English, the target writing style, specific motivation, emphasis, risk areas, and details to exclude.
