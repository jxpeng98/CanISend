# Directory Structure and Ownership

> How product code and authority are organized.

---

## Overview

CanISend is a Rust workspace with inward-facing product boundaries and a Tauri/Svelte presentation
adapter. ADR-RN-0019 and `docs/architecture/rust-native/workspace-dependency-policy-v1.json` own
the exact dependency graph.

## Directory Layout

```
crates/
├── canisend-contracts/   versioned public types and schemas
├── canisend-core/        storage-independent Pack and domain rules
├── canisend-resources/   verified embedded Packs, schemas, templates, and Agent assets
├── canisend-io/          bounded parsers, network adapters, PDF, and rendering
├── canisend-store/       SQLite, immutable Blobs, revisions, recovery, and projections
├── canisend-app/         shared product use-case facade
├── canisend-mcp/         guarded MCP adapter over canisend-app
├── canisend-cli/         Clap/process adapter and MCP stdio entrypoint
└── canisend-desktop/     Tauri command boundary and unified native host
apps/canisend-desktop/    Svelte 5 desktop presentation
xtask/                    repository and release automation, not product runtime
docs/                     ADRs, contracts, guides, evidence notes, and the Master Roadmap
release/                  machine-readable release and qualification authority
```

## Module Organization

- Put versioned public JSON types and operation identity in `canisend-contracts`.
- Put domain rules and port traits in `canisend-core`.
- Put concrete SQLite/blob behavior in `canisend-store`; adapters must not bypass `canisend-app`.
- Put bounded external input and rendering behavior in `canisend-io`.
- Keep MCP, CLI, Tauri, and Svelte as adapters; do not create a host-specific workflow engine.
- Add repository/release automation to `xtask`, not the product crates.

## Naming Conventions

Rust modules and files use `snake_case`; public types use Rust's `UpperCamelCase`. Versioned
contracts use explicit suffixes such as `V4` when the version is part of the public boundary.
Migrations are append-only numbered SQL files under `crates/canisend-store/migrations/`.

## Examples

- `crates/canisend-app/src/error.rs` centralizes adapter-neutral failure classification.
- `crates/canisend-store/src/database.rs` owns schema configuration and append-only migrations.
- `crates/canisend-mcp/src/lib.rs` exposes guarded tools through the shared application facade.
- `apps/canisend-desktop/src/lib/bridge.ts` keeps the Svelte side at the typed IPC boundary.
