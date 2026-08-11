# Quality and Verification Guidelines

> Minimum-sufficient engineering checks for CanISend.

---

## Overview

Use the smallest verification tier that proves the changed invariant. `AGENTS.md` and
`CONTRIBUTING.md` own the full matrix; Trellis checks must not automatically escalate to the full
workspace, native matrix, or extended assurance suite.

## Forbidden Patterns

- No direct adapter writes to SQLite, `.canisend/`, Blobs, or managed projections.
- No unsafe Rust (`unsafe_code = "forbid"`), unreviewed internal crate edge, or arbitrary
  executable workflow Pack code.
- No duplicated business-rule test at every adapter; one owner plus wiring/parity smokes.
- No compatibility, abstraction, dependency, or test matrix added for speculative future use.

## Required Patterns

- Reuse the application facade, typed contracts, existing approval broker, and repository helpers.
- Keep trust-boundary, consent, data-loss, recovery, and release-integrity checks positive and
  negative at the lowest owning layer.
- Keep public schemas, generated resources, machine contracts, and projections synchronized.
- Use Conventional Commits and one auditable change scope.

## Testing Requirements

1. Documentation-only: `git diff --check`; no Rust tests.
2. Rust leaf: focused test, `cargo fmt --all -- --check`, affected-package Clippy.
3. Shared contract/resource/CI/release metadata: smallest affected test plus one final
   `cargo run -p xtask --locked -- release check`.
4. Desktop: affected pnpm check/test; build only when bundling changed.
5. Native/extended assurance: only the owning scheduled or release workflow.

## Date-Bound Release Authority

- Recheck UTC `review_by` and `expires_on` values immediately before push or qualification; a
  passing local gate can become stale after a date rollover.
- An overdue gate requires an explicit owner review of the current lock/graph and reachability
  evidence. Update the machine policy and owning ADR together; never unblock CI by changing only a
  date or silently moving the hard expiry.
- Use the owning policy check, the relevant fresh audit, any named compensating regression, and one
  final `release check`. Do not repeat unrelated suites when product source is unchanged.

## Code Review Checklist

- Correct authority/layer and smallest root-cause change.
- No weakened evidence, consent, path, privacy, recovery, or release invariant.
- Stable cross-surface error and operation semantics.
- Focused runnable regression for non-trivial logic.
- Roadmap/ADR/machine authority updated only when the fact they own changed.
