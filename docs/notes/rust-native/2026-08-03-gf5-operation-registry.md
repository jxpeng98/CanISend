# GF5-OP-001 — Typed cross-surface operation registry

Date: 2026-08-03

## Outcome

CanISend now has one typed leaf-level registry for the complete compiled CLI, Tauri, and MCP
surfaces. Canonical generic v3 leaves and academic v2 compatibility aliases have explicit Pack
scope. Every other real adapter leaf is retained and classified as adapter-only instead of being
hidden behind an earlier operation-family wildcard.

The source contract owns 86 Clap leaves, 111 Tauri handlers, and 22 MCP tools. It separately
classifies the composite `application.dossier` presentation entry and every wildcard family still
used by the legacy UI parity ledgers.

## Enforcement

- `OperationId`, `OperationStatus`, class, surface, and Pack-scope values are closed typed Rust
  contracts.
- Status policy rejects missing, duplicate, or class-incompatible registry states.
- A shared leaf must appear on at least two surfaces; a canonical leaf may not be silently shared.
- One surface cannot assign the same semantic ID to two leaves.
- v2 aliases are deprecated, exact academic-Pack mappings and must target a declared canonical v3
  leaf.
- Adapter source sets and registry sets must be exactly equal.
- Tauri registrations must also resolve to declared command functions.

Agent v3 capability reporting now obtains its MCP tool binding from the typed registry. The old
Rust compatibility mapping is contract-tested against all 19 registry aliases and canonical
targets, removing two independently drifting operation authorities.

The legacy CLI binary fixtures now select `--pack academic-job` explicitly. This preserves their
v2 compatibility purpose after the framework transition made the generic Pack the ordinary CLI
default, and prevents a test from relying on an implicit Pack before a compatibility mutation.

## Focused verification

- `cargo test -p canisend-contracts --locked operation -- --nocapture`
- `cargo test -p canisend-cli --locked
  compiled_clap_leaves_match_the_typed_operation_registry_exactly -- --nocapture`
- `cargo run -p xtask --locked -- operations check`

The contract regression mutates missing, duplicate, falsely shared, Pack-incompatible, and missing
status states and requires each to fail.

## Remaining boundary

This closes the GF5-OP-001 and M1-OP-001/002 source implementation scope. It does not claim
semantic equivalence for adapter-only leaves. GF5-PARITY-001 / M1-OP-003 must execute shared
outcomes for both Packs, machine-list uncovered leaves, and prove success, stale/replay, wrong
Pack/context, no-mutation, and recovery behavior before the M1A operation exit gate is qualified.
