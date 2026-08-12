# Current Store/IO boundary research

Date: 2026-08-12

Scope: pre-implementation baseline; the implemented boundary is recorded in `design.md` and
ADR-RN-0019.

## Authority

- `docs/architecture/rust-native/decisions/0019-current-product-graph.md` records the sole
  production target-graph delta and a hard expiry of 2026-08-17.
- `docs/architecture/rust-native/workspace-dependency-policy-v1.json` lists the normal Store→IO
  edge as a temporary exception absent from the target graph.
- The Master Roadmap open-gap table classifies removal or a newly reviewed exception as P1 before
  Beta freeze. It forbids a date-only renewal.

## Concrete production coupling

- `crates/canisend-store/src/render.rs` owns a concrete `EmbeddedRenderExecutor`, document Typst
  projection, PDF compilation, and PDF validation in addition to transactional render commits.
- `crates/canisend-store/src/projection.rs` directly generates Typst during package export,
  replace/copy, repair, backup rebuild, and migration recovery.
- `crates/canisend-store/src/application_flow_v3.rs` directly projects verified Pack templates and
  compiles PDFs during Deliverable export.
- `crates/canisend-store/src/lib.rs` embeds the concrete IO render error in `StoreError`.
- `crates/canisend-store/Cargo.toml` therefore needs a normal `canisend-io` dependency.

The existing `RenderExecutor` seam covers only initial legacy document compilation. It does not
cover persisted-PDF validation, managed projection repair, generic Deliverable export, or restore.
Moving only the default executor would leave the production edge and removal condition unsatisfied.

## Existing composition and callers

- `canisend-app` already depends on Core, IO, Resources, and Store and is the correct composition
  root for the concrete embedded adapter.
- App `render`, `package`, `application_mutations_v4`, and `workspace` modules own the public
  use-case entrypoints.
- Store-internal restore, artifact repair, and migration tests require explicit propagation of the
  port so no hidden concrete fallback remains.

## Minimal selected mechanism

Move the existing Store `RenderExecutor` to Core, retain its real failure/stale test
implementations, implement it directly on IO's existing `EmbeddedTypstCompiler`, and use explicit
App injection. This satisfies the Core-port guideline without creating a second port or App wrapper.
Keep all DB/CAS/path/revision behavior where it is. Do not introduce a new crate, async runtime,
plugin registry, alternate renderer, or rewritten persistence workflow.

## Verification owners

- Core/IO: port contract, real projection/compile/validate behavior, and neutral error mapping.
- Store: transaction atomicity, revision recheck, Blob ledger, path defense, repair state, and
  convergence with fake failure ports.
- App: concrete wiring for both Packs, restore, and unchanged cross-surface classification.
- `xtask`: Cargo metadata, actual/target graph equality, exception removal, ADR/Roadmap sync, and
  final source gate.
