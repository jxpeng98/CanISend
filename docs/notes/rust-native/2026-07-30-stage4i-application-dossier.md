# Stage 4I Application Dossier evidence

**Date:** 2026-07-30

**Source state:** Working `1.0.0-alpha.5` source. This record does not authorize or claim a commit,
tag, package, push, or public release.

## Outcome

CanISend now has one body-free Application Dossier read model for an academic application. It
composes the canonical Job with its promoted Discovery lead, source readiness, reusable Profile
readiness, Workflow progress, current-stage blocker, and exact next actions.

Discovery location, deadline, URL, freshness, and last-seen metadata are projected through the
existing `job_leads.promoted_job_id` relationship. No database migration or copied mutable
application state was added. Direct and discovered jobs therefore remain owned by their existing
services.

The same Dossier is available through:

- the Rust application facade;
- `canisend application list --json`;
- `canisend application show --job JOB_ID --json`;
- Tauri and the TypeScript bridge;
- Today deadline and next-action cards;
- the Applications Overview; and
- selected-job Agent Context guidance.

Applications Overview shows localized state and stage labels, progress, deadline, location,
relevant blocker, and a route-aware Continue action. The progress indicator has an accessible
name/value, blockers use an icon and text instead of colour alone, async list loading retains
skeleton feedback, and motion respects the existing reduced-motion convention.

## Contract boundary

The new application commands are an additive body-free CLI/GUI read surface. They do not change
the frozen Agent v2 capability snapshot or public schema inventory. Agent hosts continue to use
the existing `agent context` contract, whose selected-job blockers and next actions now come from
the same Dossier. The Beta contract digest therefore remains unchanged and verified.

Tests serialize a private sentinel into an imported advert and prove it is absent from Dossier and
CLI responses. Archive/list behavior, Discovery metadata projection, workflow next-action
identity, and body-free desktop command envelopes are also covered.

## Verification

- `cargo test -p canisend-app --lib dossier --locked`: 2 passed.
- Focused Discovery origin regression: 1 passed.
- CLI binary contracts: 21 passed, including the new Dossier contract.
- Desktop Rust library tests: 32 passed.
- Svelte check: 0 errors and 0 warnings.
- Frontend tests: 6 files and 29 tests passed.
- Production Svelte build passed.
- Complete locked Rust workspace suite passed; only the pre-existing explicitly external/native
  qualification tests remained ignored.
- Strict all-workspace, all-target, all-feature Clippy passed with warnings denied.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- `cargo run -p xtask --locked -- release check` passed with 40 schemas, the existing Agent v2 and
  workspace freeze, 36 implemented CLI/GUI operations, and 36/36 Svelte parity.

The macOS linker emitted its existing debug-only compact-unwind size warning for the large CLI
test binary. It did not fail tests or strict Clippy.

## Next slice

Stage 4J should add the Content Catalog and repairable local index before the information
architecture is expanded again. That keeps future search, relationships, deep links, and
contextual Agent proposals grounded in one visible content model instead of introducing another
isolated screen.
