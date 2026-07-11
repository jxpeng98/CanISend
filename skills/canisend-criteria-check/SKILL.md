---
name: canisend-criteria-check
description: Use when checking job criteria, evidence coverage, fit gaps, selection criteria, or claim support for a CanISend academic or professional job application.
---

# CanISend Criteria Check

Focus only on criteria matching and evidence coverage. Do not prepare a full application package unless the user explicitly switches to the main `canisend` workflow.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, eligibility, qualifications, experience, or criterion matches.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, or enabling LLM-backed commands.
- Ask before reading a run Evidence snapshot, Evidence candidate, or `evidence_catalog.json`; these may duplicate
  normalized private profile bodies.
- Prefer AgentResponse counts/reasons and generated evidence under `profile/generated/`. `criteria.json` and
  `criterion_matches.json` are Tier 2; ask before reading either body. Criteria can contain user-corrected wording,
  while Match is body-minimized but still job-specific.
- Treat every Match classification as `review_state=proposed`, not a user-owned Decision, claim confirmation, or
  readiness result.
- Never write `confirmed_corrections.yaml` directly. Agent-assisted changes use `corrections status|init|update`, one
  strict scoped patch, current revision/hash, and explicit consent.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence and citation gates.
- `../canisend/references/file-contracts.md`: parsed job and criteria checklist paths.
- `../canisend/references/job-lifecycle.md`: job status and next actions.

## Workflow

1. Run `canisend agent context --workspace <private-workspace> --job jobs/<job-slug> --format json`, then inspect
   `stage status`.
2. Make Parse, Confirm, and Evidence current. If Typst-generated evidence lacks a current source-hash receipt, rerun
   `extract-profile-evidence`; resumable Evidence does not accept a workspace-external profile root.
3. Run deterministic Match only after Confirm and Evidence are current. Unknown Criteria extraction must return to
   confirmation review.
4. Prefer `parsed_job.json`, AgentResponse, `05_criteria_checklist.md`, and generated evidence. Read the body-bearing
   Evidence catalog, `criteria.json`, or `criterion_matches.json` only with approval and when privacy-safe
   counts/IDs/reasons are insufficient.
5. When the user confirms/corrects criteria, apply exactly one scoped patch against current Parse+Confirm, then rerun
   Confirm before another correction or Match. An empty initialized overlay is not `confirmed_empty`.
6. Review each criterion's proposed `strong`, `partial`, `weak`, `missing`, or `unknown` classification. Do not
   silently upgrade a proposal.
7. Distinguish an unavailable or valid-empty catalog (`unknown`) from an available catalog with no relevant support
   (`missing`), and ensure weak/partial/strong support resolves a current opaque Evidence ID.
8. Flag unknown citations, weak support, and claims that exceed the evidence.
9. Recommend concrete next evidence or edits for gaps.
10. Before saying criteria coverage is ready, check `../canisend/references/quality-gates.md`; Match alone never makes
   coverage ready.
