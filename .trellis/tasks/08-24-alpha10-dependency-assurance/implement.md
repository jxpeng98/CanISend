# Alpha.10 dependency-assurance execution plan

## 1. Activate and project the blocker

- [x] Leave `alpha10-headless-capability` in progress but clear its session pointer.
- [x] Start this planned P0 task and record its bounded release-governance scope.
- [x] Create one `M3-DEPS-001` GitHub Issue in the existing Alpha.10 milestone using existing P0,
      release, and in-progress labels.
- [x] Add the linked blocker to the Master Roadmap and Trellis project-control projection without
      changing the order `M3-HEADLESS-001 -> M3-ALPHA10-001`.

## 2. Renew only the unchanged exception set

- [x] Confirm the lock fingerprint and exact `cargo-deny 0.19.5` advisory result once more.
- [x] Change only `reviewed_on`, `review_by`, and `expires_on` for all 23 existing policy entries to
      `2026-08-24`, `2026-09-07`, and `2026-09-07` respectively.
- [x] Stop if any advisory ID, lock fact, dependency path, reachability statement, or guarded
      product boundary differs from the planning evidence.

## 3. Record the review

- [x] Update the dependency-assurance runbook's current review facts.
- [x] Add one public dated note containing the lock/tool identities, grouped reachability review,
      maintainer decision, permanent removal conditions, and body-free verification results.
- [x] Reconcile the GitHub Issue and task metadata with the final evidence; do not claim dependency
      remediation or Alpha.10 qualification.

## 4. Verify and return to the headless gate

- [x] Run the exact pinned dependency scan and the owning policy validator.
- [x] Run the complete source gate once on the combined worktree head.
- [x] Run `git diff --check`; no Rust formatting, Clippy, or duplicate workspace test is needed for
      a JSON/documentation/governance-only change.
- [x] Mark the locally passing blocker ready for protected CI and restore the headless task as
      current.
- [ ] Reconcile Issues #193 and #195 after the protected integration result.

## Validation commands

```text
cargo run -p xtask --locked -- dependencies fingerprint
/private/tmp/canisend-cargo-deny-019fc1ec/cargo-deny-0.19.5-aarch64-apple-darwin/cargo-deny check advisories bans licenses sources
cargo run -p xtask --locked -- dependencies check
cargo run -p xtask --locked -- release check
git diff --check
```

## Stop and rollback conditions

- A new advisory, lock drift, changed reachability, or expanded input/platform boundary stops the
  task before policy renewal.
- A failed policy or source gate leaves Alpha.10 blocked; revert only the renewal/governance files.
- No version transition, tag, candidate build, release, or dependency replacement belongs here.
