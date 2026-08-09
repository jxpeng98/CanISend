# M3 neutral Application-resource reads

Date: 2026-08-09

CanISend now exposes the first exact Agent v4 Requirement, Plan, and Deliverable resource slice
without depending on the App or on an academic/generic Workspace mode.

## Implemented boundary

- `requirement.list` and `requirement.show` return Pack-bound Requirement records.
- `plan.show` returns the current Plan or an explicit `not-created` state.
- `deliverable.list` and `deliverable.show` return Deliverable metadata and content references,
  never the referenced private body.
- Every receipt includes the exact Application ID, Pack binding, Application revision, and snapshot
  SHA-256 digest.
- A Requirement or Deliverable ID from another Application fails closed.
- CLI, MCP, Tauri, and the TypeScript bridge delegate to the same application facade.

The operation registry contains 28 CLI, 111 Tauri, and 16 MCP leaves. Five new shared operations
raise the semantic matrix to 18 shared operations and 60 qualified Pack/surface bindings. The MCP
surface contains 14 read/preview tools and two guarded association writes.

## Verification

- `cargo test -p canisend-app application_resources_v4 --locked`
- `cargo test -p canisend-cli -p canisend-mcp --locked`
- `cargo test -p canisend-gui --locked`
- `svelte-check --tsconfig ./tsconfig.json`
- `vitest run` — 13 files, 78 tests
- `cargo run -p xtask --locked -- operations check`
- `cargo run -p xtask --locked -- semantics check`
- `scripts/smoke_agent_v4_mcp.sh target/debug/canisend <new-temp-path>`
- `cargo run -p xtask --locked -- release check`

## Deliberately deferred

`deliverable.audit` is not classified as a routine body-free read because it needs Deliverable
content and Evidence support. Requirement extraction/confirmation, Plan propose/confirm, and
Deliverable draft/revise operations also remain deferred until their exact preview, approval,
consent, stale, replay, and no-mutation contracts are implemented across all required adapters.
