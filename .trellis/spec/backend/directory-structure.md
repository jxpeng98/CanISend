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

## Scenario: compose rendering without a Store-to-IO production edge

### 1. Scope / Trigger

Use this boundary whenever Store-owned render, projection, export, repair, or restore code needs
Typst projection, compilation, or PDF validation.

### 2. Signatures

- `canisend_core::RenderExecutor` owns `project_document`, `render_pdf`, `validate_pdf`,
  `project_deliverable`, and the default `render_document` composition.
- Store entrypoints accept `&mut impl RenderExecutor` explicitly.
- `canisend_io::EmbeddedTypstCompiler` implements the Core trait; `canisend-app` constructs it.

### 3. Contracts

- Core carries verified records, content bytes, `RenderError`, and bounded output metadata.
- IO owns templates, projection rules, compilation, PDF parsing, and concrete limits.
- Store owns SQLite, immutable Blobs, paths, revision rechecks, audit, and recovery.
- Store owns create-new export publication: a failed batch removes only files and directories
  created by that attempt before any export audit is written.
- Public operations, receipts, schemas, and Workspace formats do not expose the executor.

### 4. Validation & Error Matrix

| Condition | Owning result |
|---|---|
| Unresolved document fields | `StoreError::TemplateFieldsUnresolved` |
| Invalid projection input/invariant | Existing Store invalid-input or projection-invariant class |
| Compile, malformed/encrypted PDF, size, or time failure | `StoreError::EmbeddedRender(RenderError)` |
| Export file write fails | Typed Store IO error; no partial files or newly created directory chain remains |
| Revision changes after rendering | Existing stale error; authoritative transaction rolls back |
| Restore projection fails | Staging directory is discarded; destination is not replaced |

### 5. Good / Base / Bad Cases

- Good: App injects one compiler and Store commits only after projection/render validation.
- Base: Store tests inject a fake executor to prove failure and stale boundaries.
- Bad: Store imports IO in production or constructs a fallback compiler internally.

### 6. Tests Required

- IO: project, compile, and validate through `RenderExecutor`.
- Store: success plus renderer/projector failure, invalid PDF, stale commit, Blob leftovers, and
  repair convergence without partial authority; inject a later export-file failure and assert
  earlier files plus the new directory chain are removed before audit.
- App: both built-in Packs export, and Workspace repair/restore uses the concrete adapter.
- Architecture: locked actual and target graphs match with no temporary Store/IO exception.

### 7. Wrong vs Correct

```rust
// Wrong: persistence selects a concrete adapter.
let compiler = canisend_io::EmbeddedTypstCompiler::new();

// Correct: the App composition root supplies the Core port.
store_operation(..., &mut executor)?;
```

## Naming Conventions

Rust modules and files use `snake_case`; public types use Rust's `UpperCamelCase`. Versioned
contracts use explicit suffixes such as `V4` when the version is part of the public boundary.
Migrations are append-only numbered SQL files under `crates/canisend-store/migrations/`.

## Examples

- `crates/canisend-app/src/error.rs` centralizes adapter-neutral failure classification.
- `crates/canisend-store/src/database.rs` owns schema configuration and append-only migrations.
- `crates/canisend-mcp/src/lib.rs` exposes guarded tools through the shared application facade.
- `apps/canisend-desktop/src/lib/bridge.ts` keeps the Svelte side at the typed IPC boundary.
