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
- Prefer privacy-safe `criteria.json`, `criterion_matches.json`, AgentResponse counts/reasons, and generated evidence
  under `profile/generated/`; cite gaps instead of inventing support.
- Treat every Match classification as `review_state=proposed`, not a user-owned Decision, claim confirmation, or
  readiness result.

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
4. Prefer `criteria.json`, `criterion_matches.json`, `parsed_job.json`, `05_criteria_checklist.md`, and generated
   evidence. Read the body-bearing Evidence catalog only with approval and when opaque IDs/gaps are insufficient.
5. Review each criterion's proposed `strong`, `partial`, `weak`, `missing`, or `unknown` classification. Do not
   silently upgrade a proposal.
6. Distinguish an unavailable or valid-empty catalog (`unknown`) from an available catalog with no relevant support
   (`missing`), and ensure weak/partial/strong support resolves a current opaque Evidence ID.
7. Flag unknown citations, weak support, and claims that exceed the evidence.
8. Recommend concrete next evidence or edits for gaps.
9. Before saying criteria coverage is ready, check `../canisend/references/quality-gates.md`; Match alone never makes
   coverage ready.
