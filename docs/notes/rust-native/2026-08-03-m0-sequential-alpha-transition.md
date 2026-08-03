# M0-REL-001 — Sequential Alpha transition planner

Date: 2026-08-03

## Outcome

`release prepare-stage` now accepts only the next sequential Alpha instead of rejecting every
Alpha→Alpha transition. From the current `1.0.0-alpha.5` source, the read-only command

```console
cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.6
```

successfully returns a `canisend.stage-transition-plan/v1` document with
`writes_performed: false`. The plan lists 27 controlled files with before/after SHA-256 digests.
It creates no tag, workflow run, push, release, package-repository change, or product mutation.

## Controlled Alpha iteration

The plan updates one consistent target identity across:

- Workspace Cargo version, all exact internal dependency pins, the main and fuzz lockfiles;
- Svelte and native-preview package versions, Tauri version, Windows MSI product version, and the
  desktop offline fallback;
- the CLI/GUI parity Alpha scope;
- all versioned Alpha CLI/macOS asset names in `release/alpha-package-contract.json`;
- the manual release workflow's future-tag default;
- the release-note heading and active README/known-limitations source-version statements; and
- Beta readiness, Beta contract freeze, and feedback snapshot identities.

The three evidence files are reset to their canonical `pending-alpha-publication` forms bound to
the target Alpha. This prevents Alpha.4/Alpha.5 evidence from authorizing Alpha.6 or a later Beta
baseline. Pre-sequential Alpha.1 feedback remains accepted only while source is earlier than
Alpha.6; from Alpha.6 onward, a stale pending identity fails the source gate.

Alpha and RC iteration both require an increment of exactly one. Beta same-stage iteration,
skipped/backward Alpha numbers, stage skips, release-line changes, patch changes, and build metadata
remain rejected.

## Failure and write boundary

Write mode still requires a clean worktree. It now uses the existing staged, rollback-capable
controlled-file transaction instead of writing the plan file-by-file. A staging, replacement, or
injected controlled-write failure restores replaced files and removes newly staged output.

This implementation intentionally did not run `--write`. Source remains `1.0.0-alpha.5`; no
candidate or publication state changed.

## Verification

- `cargo test -p xtask --locked` — 77 tests
- `cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.6` — 27-file dry run
- `cargo run -p xtask --locked -- release check`
- `cargo fmt --all -- --check`
- `git diff --check`

The regression suite verifies sequential success, skipped-iteration rejection, candidate-authority
updates, stale readiness/freeze/feedback invalidation, the pre-Alpha.6 feedback compatibility
boundary, and exact target rebinding.

## Remaining boundary

This completes the M0-REL-001 source implementation and the dry-run half of M2-VERSION-001. It does
not authorize the Alpha.6 `--write`, candidate workflow, annotated tag, push, promotion, or public
release. Those actions require a reviewed clean committed source, explicit release authority, and
the exact native/source/lifecycle evidence defined by M2.
