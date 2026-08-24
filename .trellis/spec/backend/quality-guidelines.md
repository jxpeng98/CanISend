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

## Scenario: release artifact contract metadata

### 1. Scope / Trigger

- Trigger: changing the supported Agent, Workspace, schema, or host-resource contract, or changing
  `xtask release assemble` / `verify`.

### 2. Signatures

- `xtask release assemble <tag> <commit> <archive-dir> <output-dir>` writes the manifest and SBOM.
- `xtask release verify <tag> <asset-dir>` rejects metadata that is not the active supported tuple.

### 3. Contracts

Active release assets must bind `canisend.agent/v4`, schema `4.0.0`,
`canisend.agent-host-resources/v4`, and `canisend.workspace/v4`. The same shared tuple feeds
`release/support-policy.json`, the release manifest, the SBOM, and the verifier.

### 4. Validation & Error Matrix

- Active manifest differs from the supported tuple -> `release verify` fails before artifact checks.
- Active support policy differs from the shared tuple -> `release check` fails.
- A candidate contains legacy metadata -> reject the candidate; do not tag or promote it.

### 5. Good / Base / Bad Cases

- Good: current v4 metadata agrees across support policy, manifest, and SBOM.
- Base: historical v2 metadata remains unchanged under `release/history/`.
- Bad: active manifest and verifier both use legacy constants and self-validate stale metadata.

### 6. Tests Required

- Unit regression: legacy v2 manifest metadata fails before artifact inspection.
- Source gate: one final `cargo run -p xtask --locked -- release check`.
- Native gate: rebuild the exact candidate only after the repair reaches protected `main`.

### 7. Wrong vs Correct

```rust
// Wrong: compatibility constants can make stale output self-validate.
"agent_protocol": AGENT_PROTOCOL

// Correct: active release projections use the shared supported tuple.
"contracts": supported_release_contract_metadata()
```

## Date-Bound Release Authority

- `reviewed_on` must not be later than the current UTC date. If the local calendar has advanced
  before UTC, wait for UTC rollover or record the actual UTC review date; never fake the clock or
  weaken the validator.
- Recheck UTC `review_by` and `expires_on` values immediately before push or qualification; a
  passing local gate can become stale after a date rollover.
- An overdue gate requires an explicit owner review of the current lock/graph and reachability
  evidence. Update the machine policy and owning ADR together; never unblock CI by changing only a
  date or silently moving the hard expiry.
- Release-policy fixtures read current tags, evidence paths, and review dates from their machine
  authorities. Hard-coded release iterations or calendar dates are only for explicit historical or
  negative cases.
- Use the owning policy check, the relevant fresh audit, any named compensating regression, and one
  final `release check`. Do not repeat unrelated suites when product source is unchanged.

## Code Review Checklist

- Correct authority/layer and smallest root-cause change.
- No weakened evidence, consent, path, privacy, recovery, or release invariant.
- Stable cross-surface error and operation semantics.
- Focused runnable regression for non-trivial logic.
- Roadmap/ADR/machine authority updated only when the fact they own changed.
