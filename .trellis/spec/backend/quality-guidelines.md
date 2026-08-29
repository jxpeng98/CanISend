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

## Scenario: Codex-first provider qualification

### 1. Scope / Trigger

- Trigger: changing `release/provider-dogfood.json`, its validator, or the external-host policy
  used to select the next release checkpoint.

### 2. Signatures

- `validate_provider_dogfood_file(path, root)` validates the complete evidence record.
- `check_provider_dogfood()` also binds its candidate tag to the Roadmap's current public
  checkpoint.

### 3. Contracts

`canisend.provider-dogfood/v2` retains the exact candidate, consent, Agent/Workspace/resource,
evidence-note, Pack, and Skill bindings. It requires exactly these passed scenarios, in order:

1. `codex-cli-academic-requirement-preview-cancel`;
2. `codex-cli-generic-requirement-preview-cancel`.

Both preserve a positive Application revision and `proposed` state, return `previewed`, and set
mutation/submission to false. `excluded_attempts` is empty; historical Claude and stale-host
outcomes belong in immutable body-free notes.

### 4. Validation & Error Matrix

- Missing or reordered Pack scenario -> reject the record.
- Failed, mutating, submitting, or state-changing scenario -> reject the record.
- Unknown/private field, stale note/contract/Pack/Skill digest, or non-empty exclusion -> reject
  the record.
- Candidate tag differs from the Roadmap checkpoint -> `release check` fails.

### 5. Good / Base / Bad Cases

- Good: exact public Alpha candidate plus both safe Codex Pack scenarios.
- Base: historical v1 records and Claude observations remain unchanged in dated notes.
- Bad: count a skipped Claude session as passed or retain a transcript/token in active evidence.

### 6. Tests Required

- Focused regression:
  `cargo test -p xtask --locked provider_dogfood_rejects_missing_stale_failed_or_private_records`.
- Static checks: Rust format and `xtask` Clippy.
- Source gate: one final `cargo run -p xtask --locked -- release check`.

### 7. Wrong vs Correct

```json
// Wrong: optional host outcome is promoted into required passed evidence.
{"host":"claude-code","status":"passed"}

// Correct: each required Pack is proven by Codex without mutation.
{"host":"codex-cli","pack_id":"org.canisend.academic-job","mutation_performed":false}
```

## Scenario: lean Beta readiness authority

### 1. Scope / Trigger

- Trigger: changing `release/beta-readiness.json`, its refresh path, or the Alpha-to-Beta
  readiness validator.

### 2. Signatures

- `./scripts/refresh_beta_readiness.sh OWNER/REPO MAINTAINER_JSON [--write]` builds the preview and
  writes only from a clean worktree.
- `validate_qualified_beta_readiness(path, root, now)` validates the complete authority.
- `xtask release verify-beta-readiness FILE` exposes that validator to operators and CI.

### 3. Contracts

`canisend.beta-readiness/v2` binds exact public Alpha.10 identity, the checked-in
`canisend.provider-dogfood/v2` digest and its two Codex scenario IDs, Agent/Workspace/resource,
Pack, and Skill digests, a body-free maintainer note, canonical zero cohort counts, and nine clear
blocker classes. Public Issue input is limited to number, state, and labels; only an open Issue with
both `priority:P0` and `state:blocked` is an applicable Issue blocker.

### 4. Validation & Error Matrix

- Unknown/private field, stale note, false user count, or mismatched provider/contract digest ->
  reject the record.
- Missing or uncleared blocker class, unresolved blocker, or applicable public P0 blocker -> reject
  qualification.
- Planned or ready public Issues without both blocker labels -> retain their counts but do not
  block readiness.
- Record older than 24 hours or more than five minutes in the future -> reject it.

### 5. Good / Base / Bad Cases

- Good: exact Alpha.10 plus both non-mutating Codex Pack scenarios, reviewed limitations, zero
  users, and nine clear blocker classes.
- Base: historical readiness v1 stays immutable under `release/history/` and cannot authorize the
  active Beta transition.
- Bad: infer readiness from an empty Issue result or claim cohort users before public Beta.1.

### 6. Tests Required

- Focused regression:
  `cargo test -p xtask --locked beta_readiness_v2_rejects_stale_private_false_or_blocked_records`.
- Operator check: `xtask release verify-beta-readiness release/beta-readiness.json`.
- Source gate: one final `cargo run -p xtask --locked -- release check`.
- Protected Fast CI owns the complete workspace suite; do not repeat native or host matrices when
  product bytes are unchanged.

### 7. Wrong vs Correct

```json
// Wrong: an empty Issue list is treated as sufficient readiness.
{"open_p0_blocker_issue_numbers":[],"status":"qualified"}

// Correct: readiness also binds exact provider, contract, maintainer, cohort, and blocker evidence.
{"schema":"canisend.beta-readiness/v2","status":"qualified","unresolved_blockers":[]}
```

## Scenario: Beta v4 contract freeze

### 1. Scope / Trigger

- Trigger: changing `release/beta-contract-freeze.json`, its builder, or an Alpha-to-Beta
  transition validator.

### 2. Signatures

- `build_beta_contract_freeze_at(root, version) -> Result<Value, String>` derives the full record.
- `validate_qualified_beta_contract_freeze(freeze, root, version)` requires exact derived equality.
- `validate_beta_transition_authorities(root, version, readiness, freeze)` validates both records
  and their shared Alpha source.

### 3. Contracts

`canisend.beta-contract-freeze/v2` binds the exact Alpha tag/source, Agent/Workspace/resource/Pack/
Skill contracts, complete migration inventory through 20, four schema families, every stable
error-to-exit mapping, and the validated Alpha package contract plus its three layout sections.

### 4. Validation & Error Matrix

- Incomplete or unqualified readiness -> do not build or accept the freeze.
- Legacy protocol, stale digest, changed exit mapping, unknown field, or baseline-only record ->
  reject the freeze.
- Readiness and freeze Alpha identities differ -> reject the Beta transition.

### 5. Good / Base / Bad Cases

- Good: checked-in v2 record exactly equals the value derived from current qualified authorities.
- Base: historical v1 records remain unchanged and cannot authorize the active transition.
- Bad: validate only `baseline.release` and `baseline.source_commit`.

### 6. Tests Required

- Focused regression:
  `cargo test -p xtask --locked beta_contract_freeze_v2_rejects_legacy_or_unbound_records`.
- Static checks: Rust format and `xtask` Clippy.
- Source gate: one final `cargo run -p xtask --locked -- release check`.
- Protected Fast CI owns the complete workspace suite; do not repeat native or host matrices.

### 7. Wrong vs Correct

```rust
// Wrong: a matching baseline can hide stale or missing contracts.
freeze["baseline"] == readiness["alpha_release"]

// Correct: both release check and stage preparation require the exact derived record.
validate_qualified_beta_contract_freeze(freeze, root, version)?;
```

## Scenario: transactional release-stage source projection

### 1. Scope / Trigger

- Trigger: changing `render_stage_transition`, `release prepare-stage`, or a checked-in source
  version projection.

### 2. Signatures

- `xtask release prepare-stage <TAG> [--write]` previews or applies one stage transition.
- `insert_active_source_version_updates(root, files, from, to)` renders the shared projections.
- `insert_sequential_alpha_evidence_resets(files, to)` runs only for Alpha-to-Alpha iteration.

### 3. Contracts

Every supported transition updates the native-preview package, desktop fallback, parity manifest,
performance baseline, README, RELEASE, bug template, release-workflow default, known limitations,
and package contract. Alpha-to-Alpha additionally resets readiness, freeze, and feedback; a
cross-stage transition preserves those authorities byte-for-byte. Dry run is non-mutating and
write mode requires a clean worktree.

### 4. Validation & Error Matrix

- Missing, duplicated, or stale source projection -> reject before mutation.
- Wrong target stage/iteration, stale readiness, or incomplete freeze -> reject before mutation.
- Dirty worktree in write mode -> reject before mutation.
- Preview/write file or preserved-history mismatch -> stop; do not commit the transition.

### 5. Good / Base / Bad Cases

- Good: Alpha-to-Beta updates all current projections and preserves qualified Alpha evidence.
- Base: Alpha-to-Alpha updates the same projections and resets the three candidate authorities.
- Bad: update Cargo and the ledger while leaving README, workflow, or package metadata on Alpha.

### 6. Tests Required

- Focused regressions: `stage_source_projections_are_shared_and_only_alpha_resets_evidence` and
  `stage_transition_changes_only_controlled_current_state`.
- Stage-aware truth: `active_release_truth_rejects_stale_current_surfaces_and_ignores_history`.
- Source gate: one final `cargo run -p xtask --locked -- release check` on the final PR head.

### 7. Wrong vs Correct

```rust
// Wrong: common projections update only during sequential Alpha work.
if from_stage == Alpha && to_stage == Alpha { update_source_projections()?; }

// Correct: every transition updates source projections; only evidence reset is Alpha-specific.
update_source_projections()?;
if from_stage == Alpha && to_stage == Alpha { reset_candidate_evidence()?; }
```

## Code Review Checklist

- Correct authority/layer and smallest root-cause change.
- No weakened evidence, consent, path, privacy, recovery, or release invariant.
- Stable cross-surface error and operation semantics.
- Focused runnable regression for non-trivial logic.
- Roadmap/ADR/machine authority updated only when the fact they own changed.
