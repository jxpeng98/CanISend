# R1 implementation plan

Status: Complete; Steps 1-5 verified

## Step 1 — Session metadata

- Upgrade `crates/canisend-app/src/agent_session.rs` to registry v2 with bounded body-free fields,
  v1 read migration, canonical v2 writes, and focused migration/privacy tests.
- Keep the current atomic private-file write and existing Claude/session APIs working.

## Step 2 — App Server connection

- Extend `crates/canisend-desktop/src/agent_runtime.rs` with bounded JSONL encode/decode, request
  correlation, normalized events, readiness, persistent child lifecycle, start/resume, streamed
  turn, fail-closed server requests, interrupt, and cleanup.
- Reuse existing discovery/version/error helpers. Do not add Tokio, an SDK, or a protocol module.
- Add a feature-gated fake App Server binary plus the smallest lifecycle test entry needed to drive
  it on macOS, Linux, and Windows.

## Step 3 — Native contract

- Register `start_agent_session` in `crates/canisend-desktop/src/lib.rs`.
- Add that leaf to `crates/canisend-contracts/operation-registry-v1.json`; retain the existing
  catalog, turn, and cancel leaves.
- Add focused command regressions for clean Workspace v4 scope, consent, and exact cancellation.

## Step 4 — Stream into the existing panel

- Extend `apps/canisend-desktop/src/lib/bridge.ts` with session status/event types, Tauri `Channel`,
  start/load, and streamed-turn wiring.
- Update `apps/canisend-desktop/src/lib/agent-state.svelte.ts`, `App.svelte`, and
  `lib/views/AgentView.svelte` to append deltas, show finite status, cancel, restart, and resume.
- Add only the English/Chinese labels required by the new finite states and update the existing
  bridge/view tests. Do not redesign navigation or add a second conversation component.

## Step 5 — Contract and verification

- Add the implemented embedded-session contract to
  `.trellis/spec/backend/directory-structure.md` after behavior is proven.
- Run focused registry and fake-server Rust tests, Rust formatting and affected-package Clippy,
  desktop check/tests, and the clean Workspace v4 regression.
- Perform one bounded local signed-in Codex smoke without recording transcript content.
- Run `cargo run -p xtask --locked -- release check` once on the final implementation head.
- Follow the active feature-freeze exact-commit exception protocol for product/spec changes; keep
  `.trellis/tasks/` planning and evidence separate from unrelated working-tree edits.

## Expected changed files

- `crates/canisend-app/src/agent_session.rs`
- `crates/canisend-desktop/Cargo.toml`
- `crates/canisend-desktop/src/agent_runtime.rs`
- `crates/canisend-desktop/src/lib.rs`
- one feature-gated fake-server source and one focused lifecycle test if the in-module test cannot
  execute the fixture directly
- `crates/canisend-contracts/operation-registry-v1.json`
- `apps/canisend-desktop/src/lib/bridge.ts`
- `apps/canisend-desktop/src/lib/bridge.commands.test.ts`
- `apps/canisend-desktop/src/lib/agent-state.svelte.ts`
- `apps/canisend-desktop/src/App.svelte`
- `apps/canisend-desktop/src/lib/views/AgentView.svelte`
- existing Agent view test and `apps/canisend-desktop/src/lib/i18n.ts` only where new states require it
- `.trellis/spec/backend/directory-structure.md`

## Completion evidence

- Exact implementation/source/spec commit IDs and their feature-freeze exception records.
- Focused command/test output and final source-gate output.
- Body-free manual smoke note: Codex version, scenario outcomes, and registry field names only.

Recorded in `research/app-server-evidence.md`. The source commit is
`8240fae7da0eb5ecafc2e3dce7bf7609d05f9cf3`, its feature-freeze exception commit is
`dd0ee4ce64bcf3dc714844ddd4e8e728a9ea76a5`, and the exact source gate passed on that exception head.
