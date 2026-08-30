# Implementation plan: public Beta.1 cohort qualification

## Phase A — planning and consent boundary

- [x] Rebase the stable Trellis task ID from stale Alpha.9/Beta-entry wording to public Beta.1 and
      pre-RC qualification.
- [x] Bind the plan to Issue #70, the qualified Beta.1 identity, and the active feature-freeze
      baseline.
- [x] Keep `release/beta-readiness.json` immutable and define a separate post-Beta aggregate.
- [ ] Obtain explicit participant consent and a bounded owner schedule before any invitation,
      provider send, private-data access, or cohort execution.

## Phase B — minimum execution packet

- [ ] Derive one body-free flow/coverage checklist directly from Issue #70; do not create a new
      project-management system or participant database.
- [ ] Verify the exact public Beta.1 tag/source before each cohort window.
- [ ] Run 5–8 invited users per bounded window until at least 8 cumulative users and 20 completed
      supported flows satisfy the coverage contract.
- [ ] Cover the exact input, Deliverable, recovery, legacy-refusal, and accessibility tokens from
      the PRD rather than creating a broader host or platform matrix.
- [ ] Use Codex as the required Agent host; record Claude only as optional truthful observation.
- [ ] Keep product failures in denominators and create minimum-safe Issue links for each failure.

## Phase C — evidence and RC gate

- [ ] Add the smallest aggregate `release/cohort-evidence.json` contract and owning xtask
      validator once real consented observations exist.
- [ ] Add one focused positive/negative validator regression covering stale/private/incomplete
      records; do not duplicate native or workspace suites.
- [ ] Commit one reviewed body-free cohort note and its SHA-256 binding.
- [ ] Reconcile Issue #70 only after the checked-in aggregate passes and no P0/P1 blocker remains.
- [ ] Hand off to RC.1 planning without publishing, tagging, or building RC bytes in this task.

## Validation

Planning-only reconciliation:

```bash
git diff --check
jq empty .trellis/tasks/08-10-alpha8-cohort-beta-evidence/task.json
python3 .trellis/scripts/task.py validate .trellis/tasks/08-10-alpha8-cohort-beta-evidence
```

Future cohort-record implementation:

```bash
cargo test -p xtask --locked cohort_evidence_rejects_stale_private_or_incomplete_records
cargo fmt --all -- --check
cargo clippy -p xtask --locked --all-targets -- -D warnings
cargo run -p xtask --locked -- release check
```

Fast CI owns the complete workspace suite. Native release matrices are not repeated unless a
changed-byte P0 fix creates a new prerelease candidate.

## Rollback points

- Before consent: revert the planning PR; no participant or release state exists.
- During collection: stop the affected window and retain only approved body-free aggregates.
- After a changed-byte blocker: qualify the next prerelease and resume affected coverage there;
  never relabel observations as Beta.1.
