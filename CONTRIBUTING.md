# Contributing to Rust-Native CanISend

## Toolchain

Use the toolchain pinned in `rust-toolchain.toml`. CanISend's product, build, tests, and release
artifacts do not depend on Python. Trellis uses Python 3.9+ only for repository-local development
hooks and task bookkeeping.

## Minimum sufficient checks

Choose the smallest row that owns the change. Do not run every row for every edit.

| Change | Local check before commit | Higher owner |
|---|---|---|
| Prose or historical note only | `git diff --check` | Documentation/source gate when the file is active release truth |
| Rust leaf behavior | affected test or test filter, `cargo fmt --all -- --check`, affected-package Clippy | Fast CI runs the workspace suite |
| Shared contract, schema, resource, CI, or release metadata | smallest affected test plus `cargo run -p xtask --locked -- release check` once on the final PR head | Fast CI |
| Desktop behavior | affected pnpm test/check plus production build only when bundling changed | Fast CI accessibility and macOS lanes |
| Package/runtime behavior | exact affected package smoke | Native candidate workflow |
| Release candidate | no ad hoc local matrix | Build-once native and public-verification workflows |

One invariant has one primary test owner. Other adapters receive a wiring/parity smoke, not copies of
the same business-rule test. A trust-boundary, consent, data-loss, recovery, or release-integrity
change keeps one positive and one negative regression at the lowest owning layer.

## Architecture

- Keep dependencies pointed inward according to ADR-RN-0002.
- Put public versioned JSON types in `canisend-contracts`.
- Keep domain rules and port traits in `canisend-core`.
- Keep SQLite/blob details in `canisend-store`.
- Keep HTTP, parsers, providers, and rendering in `canisend-io`.
- Do not let agent hosts write `.canisend/` internal state.
- Do not add a Python runtime or test dependency.

New dependencies require a documented purpose, compatible license, and evidence that they do not introduce an
unplanned end-user runtime.

## Changes and tracking

Update the Rust-native roadmap when a tracked task is completed. Add a dated note for phase transitions, dependency
decisions, material risks, and release evidence. Commits use Conventional Commits and should represent one auditable
milestone.

## Trellis project control

Run `trellis init -u <name>` once after cloning, then let Codex or Claude Code load the tracked
`.trellis/` workflow. Use a Trellis task for durable multi-file work; keep a contained one-turn edit
inline when task history would add no value. Trellis task status is execution bookkeeping, not
release authority: every Roadmap item still needs its exact GitHub Issue/milestone and committed
evidence. See `.trellis/spec/guides/project-control.md` before creating roadmap-linked tasks.
