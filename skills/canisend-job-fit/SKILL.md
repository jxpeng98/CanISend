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
- Ask before reading a run Evidence snapshot, Evidence candidate, or `evidence_catalog.json`; these may duplicate
  normalized private profile bodies.
- Prefer privacy-safe `agent context`, `criteria.json`, `criterion_matches.json`, generated evidence under
  `profile/generated/`, `parsed_job.json`, and criteria checklists over raw private files.
- Treat missing essential criteria as strategy risks, not wording problems.
- Treat every durable Match classification as `review_state=proposed`. Do not turn it into apply, hold, or skip; the
  user-owned Decision slice is not implemented yet.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence and readiness gates.
- `../canisend/references/file-contracts.md`: job, criteria, and evidence artifact paths.
- `../canisend/references/workflow.md`: local-first generation sequence.

## Workflow

1. Run `canisend agent context --workspace <private-workspace> --job jobs/<job-slug> --format json`, then inspect
   `stage status`.
2. If Evidence is missing, stale, or receiptless, rerun `extract-profile-evidence` and deterministic Evidence. Do not
   bypass the workspace-only resumable boundary for an external profile.
3. Make Parse, Confirm, Evidence, and Match current through deterministic stage commands. Unknown Criteria routes
   back to confirmation review rather than guessed matching.
4. Prefer `criteria.json`, `criterion_matches.json`, `parsed_job.json`, `05_criteria_checklist.md`,
   `02_fit_report.md`, and `profile/generated/` evidence. Read catalog bodies only when approved and necessary.
5. Separate essential criteria, desirable criteria, role priorities, and implicit fit signals.
6. Review each proposed `strong`, `partial`, `weak`, `missing`, or `unknown` result and its explicit gap. Distinguish
   unavailable/empty Evidence from an available catalog with no relevant support.
7. Offer a provisional application angle—strongest fit, credible stretch, or high risk—without recording a Decision.
8. Suggest which materials need targeted work: cover letter, CV, research statement, teaching statement, criteria response, or evidence extraction.
9. Before saying the application is strategically ready, check `../canisend/references/quality-gates.md`; Match alone
   never satisfies that gate.

## Output

Use a compact decision format:

- Provisional fit assessment (not an application Decision)
- Strongest evidence-backed selling points
- Material risks or unsupported claims
- Recommended application angle
- Next CanISend skill or CLI step
