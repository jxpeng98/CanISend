# ADR-RN-0019: Consolidate the current product and dependency graph

**Status:** Accepted

**Date:** 2026-08-03

**Decision owner:** CanISend maintainer

## Context

ADR-RN-0002 described the initial six-crate workspace. ADR-RN-0013 later added an egui desktop,
ADR-RN-0015 replaced it with Tauri/Svelte, ADR-RN-0016 unified the packaged macOS GUI and CLI
host, and ADR-RN-0018 changed the product from an academic-only workflow to a generic
evidence-application framework. All of those changes are implemented in source, but no accepted
ADR consolidated their resulting graph.

Cargo compilation alone is insufficient architecture enforcement. It accepts a new normal, dev,
build, target-specific, optional, or feature-enabled workspace edge even when that edge bypasses
the application facade or reverses an intended dependency direction. The former normal
`canisend-store -> canisend-io` rendering/projection edge contradicted the original inward graph
and therefore remained explicit and time-bounded until its closure under M3-ARCH-001.

## Decision

CanISend has nine product crates and one repository-automation crate:

| Crate | Owned boundary |
|---|---|
| `canisend-contracts` | Versioned public/domain types, schemas, operation identity, validation primitives |
| `canisend-core` | Pack compiler, stage graph, capability and domain rules without concrete storage or adapters |
| `canisend-resources` | Build-time embedded, digest-verified Packs, schemas, templates, fonts, examples, and Agent assets |
| `canisend-io` | Bounded local/network parsing, discovery adapters, PDF extraction, and in-process rendering |
| `canisend-store` | SQLite, immutable Blobs, revisions, transactions, audit, projections, migration, recovery |
| `canisend-app` | The only shared product use-case facade and adapter-neutral error/receipt boundary |
| `canisend-mcp` | Guarded MCP transport over `canisend-app`; no independent workflow engine |
| `canisend-cli` | Clap/process/output adapter plus the MCP stdio host entrypoint |
| `canisend-gui` | Tauri 2 command boundary and packaged unified host for the Svelte 5 desktop |
| `xtask` | Non-product repository, source-gate, and release automation |

The checked-in
[`workspace-dependency-policy-v1.json`](../workspace-dependency-policy-v1.json) is the
machine-readable authority for every internal workspace edge and its exact Cargo classification.
The source gate derives locked Cargo metadata and rejects additions, removals, kind changes,
target changes, optional/default-feature changes, feature-set changes, and dependency renames.

## Actual graph

The actual source graph includes normal edges as solid arrows and dev-only fixture edges as dotted
arrows. External crates and the Svelte build graph are not internal Cargo edges and are governed by
their lockfiles and dependency policies.

```mermaid
flowchart LR
    contracts["canisend-contracts"]
    core["canisend-core"]
    resources["canisend-resources"]
    io["canisend-io"]
    store["canisend-store"]
    app["canisend-app"]
    mcp["canisend-mcp"]
    cli["canisend-cli"]
    gui["canisend-gui"]
    xtask["xtask"]

    core --> contracts
    resources --> contracts
    io --> contracts
    io --> core
    io --> resources
    store --> contracts
    store --> core
    store -. dev .-> io
    store -. dev .-> resources
    app --> contracts
    app --> core
    app --> io
    app --> resources
    app --> store
    mcp --> app
    mcp --> contracts
    mcp -. dev .-> store
    cli --> app
    cli --> contracts
    cli --> mcp
    gui --> app
    gui --> cli
    gui --> contracts
    xtask --> cli
    xtask --> contracts
    xtask --> resources
```

`canisend-gui -> canisend-cli` is intentional: ADR-RN-0016's unified host dispatches explicit CLI
and MCP invocations in-process and opens Tauri only for GUI launch modes. It does not shell out for
product behavior. `canisend-cli -> canisend-mcp` owns the stdio server entrypoint. Tauri commands
and MCP tools continue to call `canisend-app` for shared business operations.

## Target graph

The pre-Beta target removes concrete rendering/projection ownership from Store. The CLI's former
facade-bypassing direct Store/IO/Resources edges were removed by M1-ARCH-003; dev-only Store and MCP
fixture edges may remain because they do not become production adapter dependencies.

```mermaid
flowchart LR
    contracts["canisend-contracts"]
    core["canisend-core"]
    resources["canisend-resources"]
    io["canisend-io"]
    store["canisend-store"]
    app["canisend-app"]
    mcp["canisend-mcp"]
    cli["canisend-cli"]
    gui["canisend-gui"]

    core --> contracts
    resources --> contracts
    io --> contracts
    io --> core
    io --> resources
    store --> contracts
    store --> core
    store -. dev .-> io
    store -. dev .-> resources
    app --> contracts
    app --> core
    app --> io
    app --> resources
    app --> store
    mcp --> app
    mcp --> contracts
    mcp -. dev .-> store
    cli --> app
    cli --> contracts
    cli --> mcp
    gui --> app
    gui --> cli
    gui --> contracts
```

M1-ARCH-003 moved structured candidate parsing and private candidate-file projection behind
`canisend-app`, re-exported Agent skill presentation states through that facade, and rebuilt the CLI
performance fixture through application operations. The CLI now depends only on `canisend-app`,
`canisend-contracts`, and the ADR-approved `canisend-mcp` host boundary. M3-ARCH-001 subsequently
removed the final actual/target production-graph delta, so the graphs above now agree exactly.

## Store→IO exception closure

M3-ARCH-001 / Issue #182 removed the normal Store→IO edge on 2026-08-12 without changing public
Workspace, Pack, operation, approval, or filesystem contracts:

- the existing `RenderExecutor` seam, neutral error categories, and output values moved to
  `canisend-core`;
- the existing IO `EmbeddedTypstCompiler` implements that seam directly;
- `canisend-app` constructs and injects the implementation for legacy render, managed projection,
  Deliverable export, Workspace repair, and atomic restore;
- Store retains SQLite, immutable Blob, path, revision, audit, recovery, and commit ownership; and
- Store retains only a dev dependency on IO for exact real-render integration fixtures. That edge
  appears identically in the actual and target graphs and is not a production exception.

The machine policy therefore records 28 actual edges, 28 target edges, and zero temporary
exceptions. Named failure tests continue to prove that renderer failure creates no Blob or
authoritative write, stale-at-commit rolls back artifact/head/reference/audit writes, prepared CAS
leftovers remain digest-valid and auditable, and projection/path failure converges through
`repair-required` without deleting pre-existing or shared content.

The closed exception remains historical evidence: it was accepted on 2026-08-03 for Alpha.6,
initially reviewed by 2026-08-10, re-reviewed on 2026-08-11 for published Alpha.8 source
`35e7c822ea2f469ab726a31b5d08e622f6810c55`, and had a hard expiry of 2026-08-17. Its original
tracking was M1-ARCH-001, M1-ARCH-002, and M1-ARCH-004; the closure is M3-ARCH-001. No date-only
renewal occurred. The original limits remain recorded in the
[M1 exception review](../../../notes/rust-native/2026-08-03-m1-store-render-exception.md).

## Tauri, Svelte, and platform boundaries

- Tauri 2 owns the native window, capabilities, dialogs, and IPC command boundary.
- Svelte 5, TypeScript, Vite, and pnpm are build-time presentation dependencies. End-user packages
  embed static assets and require no Node.js runtime.
- The Svelte frontend never reads SQLite, `.canisend`, immutable Blobs, or private source files.
- The unified host chooses GUI, CLI, or MCP mode from explicit launch context; it does not create a
  second engine or parse shell commands.
- Five CLI targets remain the portable release unit. Desktop packaging is platform-qualified under
  the release matrix; Linux musl remains CLI-only unless a separately accepted window-system
  boundary is added.
- Target-specific and optional Cargo edges are first-class policy fields even when the current
  internal graph has none. Feature activation cannot silently introduce an internal edge.

## Supersession

- ADR-RN-0002 is superseded because its six-crate list is no longer the actual workspace.
- ADR-RN-0013 is superseded by ADR-RN-0015 because egui/eframe is no longer a supported or present
  desktop implementation.
- ADR-RN-0015, ADR-RN-0016, and ADR-RN-0018 remain accepted; this ADR consolidates their current
  dependency consequences without weakening their UI, unified-host, or generic-framework rules.

## Consequences

- A Cargo manifest change cannot silently alter the internal architecture.
- Dev, build, target, optional, feature, default-feature, and renamed dependency distinctions are
  reviewable rather than flattened into a crate-pair diagram.
- Closed graph debt remains visible in historical review evidence instead of being rewritten as if
  the exception never existed.
- Updating a legitimate edge requires an intentional policy and ADR review in the same change.
