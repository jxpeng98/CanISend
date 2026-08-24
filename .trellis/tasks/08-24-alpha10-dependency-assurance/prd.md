# Resolve the expired dependency-assurance gate

## Goal

Resolve the expired, lock-bound dependency exceptions with a fresh maintainer decision so the
Alpha.10 headless capability can pass its source gate without weakening CanISend's defensive
boundaries.

## Background

All 23 exceptions reached their review and expiry date on 2026-08-17. The third-party lock
fingerprint is unchanged at 751 packages, and the pinned `cargo-deny 0.19.5` check currently finds
no new unaccepted advisory. Current direct roots (`typst 0.15.1`, `hayagriva 0.10.1`,
`tauri 2.11.5`) are already at their latest published versions; replacing their transitive paths
is not a bounded Alpha.10 fix.

The maintainer accepted the recommended bounded reauthorization on 2026-08-24: unchanged entries
may be reviewed through 2026-09-07, with the review and expiry dates intentionally identical.

## Requirements

- Re-review every existing exception against the exact `Cargo.lock` fingerprint and a fresh pinned
  advisory, ban, license, and source check.
- Preserve the existing product boundaries: fixed bounded Typst projection, no bibliography/CSL/XML
  input, embedded verified fonts only, checked-in Tauri patterns only, and no public Linux GUI.
- Reauthorize only entries whose advisory identity, dependency path, reachability, removal condition,
  and upstream tracking remain proven. Any new or changed entry blocks the task for separate review.
- Record one explicit maintainer risk-acceptance window; do not silently roll expired dates forward.
- Keep the exception authority, dependency-assurance documentation, and machine-enforced facts in
  sync.
- Unblock the existing Alpha.10 headless task only after both dependency assurance and the complete
  source gate pass on the same worktree head.

## Acceptance Criteria

- [x] The 23 policy entries remain bound to the unchanged 751-package fingerprint and match
      `deny.toml` exactly.
- [x] A fresh exact `cargo-deny 0.19.5 check advisories bans licenses sources` passes with no new
      unaccepted advisory.
- [x] Source checks still reject bibliography/helper reachability, user/system font input,
      user-authored Tauri patterns, or public Linux GUI drift.
- [x] `cargo run -p xtask --locked -- dependencies check` passes.
- [x] `cargo run -p xtask --locked -- release check` passes on the final combined Alpha.10 head.
- [x] The review evidence, dates, decision, and permanent removal conditions are documented.

## Constraints

- This task does not reinterpret an expired exception as valid without maintainer authorization.
- Trust-boundary assertions and fail-closed checks must not be removed or relaxed to pass CI.
- The normal policy maximums remain 14 days between reviews and 30 days total exception lifetime.

## Out of Scope

- Replacing Typst, Tauri, GTK, or their transitive dependency graphs.
- Publishing a Linux GUI or accepting user-authored Typst, bibliography, XML, fonts, or Tauri
  patterns.
- Alpha.10 tagging, packaging, or release qualification.

## Key Decisions

- Reauthorize only the currently proven 23 entries for 14 days:
  `reviewed_on=2026-08-24`, `review_by=expires_on=2026-09-07`.
- Use one date for review and hard expiry so a missed next review cannot leave an additional grace
  period.
- Keep dependency replacement outside this blocker; it remains each exception's permanent removal
  condition.

## Notes

- Research evidence: `research/2026-08-24-expired-exception-audit.md`.
