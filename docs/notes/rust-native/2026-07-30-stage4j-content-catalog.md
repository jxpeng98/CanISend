# Stage 4J Content Catalog and intake convergence evidence

**Date:** 2026-07-30

**Source state:** Working `1.0.0-alpha.5` source. This record does not authorize or claim a commit,
tag, package, push, or public release.

## Outcome

CanISend now projects current user-visible content into one local Content Catalog. It covers job
and profile sources, parsed jobs, criteria, evidence, matches, plans, application materials,
review findings, packages, exports, Typst, PDFs, and render manifests.

Catalog entries are derived from current authoritative artifact heads and existing job, source,
workflow, task, document, export, render, and dependency relationships. Each entry includes its
exact artifact revision/hash, category, workflow stage, lifecycle status, stale state, recursively
conservative privacy classification, provenance, related applications, and direct dependencies.
There is no schema migration, catalog table, copied artifact body, or new mutable authority.

The same content operations are connected through:

- the Rust store and application services;
- `canisend content list` and `canisend content search`;
- Tauri commands and the typed TypeScript bridge;
- the bilingual Content Library embedded below Application Overview; and
- route-aware links back to the exact Profile, Workflow, Documents, Review, Package, or Render
  surface.

Metadata listing and search never read artifact bodies. Filters cover application, category,
stage, status, privacy, and UTC date bounds. A private full-text search requires a non-empty query
and explicit `read-private-inputs` consent before workspace discovery. It excludes Secret
artifacts, caps each eligible body at 4 MiB and the operation at 32 MiB, verifies immutable blobs,
and returns only matching snippets. Its deterministic inverted index is rebuilt in memory and
discarded after the operation.

## Intake convergence

Direct file, link, and PDF previews plus CSV, JSON, host-agent, and bounded network discovery
previews now project into one typed intake review model and one Svelte review component. It shows:

- source identity and detected type;
- extraction sizes/counts;
- explicit duplicate state with automatic merge disabled;
- target application or opportunity library;
- intended mutations;
- the exact confirmed consent scope; and
- the commit boundary.

Direct job intake commits the exact prepared bytes retained in bounded Rust preview state and
rejects a changed job revision. Discovery intake commits the exact reviewed normalized report and
never rereads the file or endpoint during confirmation.

## Contract and privacy boundary

The Content Catalog is an additive CLI/GUI read surface. It does not alter the frozen Agent v2
capability registry, the thirteen-tool MCP surface, the forty public schemas, or database
migrations frozen through version 13.

Secret classification is propagated from source artifacts through current dependency heads, so a
derived artifact cannot be made searchable merely because its own table lacks a privacy column.
Body-free catalog and metadata-search tests include private sentinels and prove those values are
absent. Consent, query, ID, limit, and date validation run before workspace access.

The desktop retains only returned search results in its current in-memory App session. Workspace
changes and authoritative artifact mutations clear them. Search indexes are never retained.

## Experience and performance

The Content Library is part of the selected application journey instead of a new disconnected
tab. It supports current-application or whole-workspace scope, explicit search, useful empty
states, semantic result articles, visible provenance, text-plus-icon status, labelled controls,
44-pixel interaction targets, live result counts, loading feedback, and reduced-motion handling.
English and Simplified Chinese use the same typed labels and privacy explanation.

The panel is loaded only when Applications is opened. The production frontend build emits a
separate 16.02 kB Content Library chunk and keeps the main minified chunk at 498.75 kB, eliminating
the prior build-size warning introduced during the first integrated build.

## Verification

- Focused Content Catalog store test: 1 passed, including dependency privacy propagation.
- Focused application content tests: 2 passed.
- Focused shared intake application tests: 3 passed under the intake filter.
- CLI binary contracts: 22 passed, including body-free metadata and consent-gated full text.
- Desktop Rust library tests: 34 passed, including consent ordering and exact intake boundaries.
- Frontend tests: 6 files and 30 tests passed.
- Svelte check: 0 errors and 0 warnings.
- Production Svelte build passed without chunk-size warnings.
- Complete locked Rust workspace suite: 263 passed and 4 explicitly external/native tests ignored.
- Strict all-workspace, all-target Clippy passed with warnings denied.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- `cargo run -p xtask --locked -- release check` passed with 40 schemas, migrations frozen through
  13, 37 implemented CLI/GUI operations, and 37/37 Svelte parity.

The macOS linker emitted its existing debug-only compact-unwind size warning for the large CLI
test binary. It did not fail tests or strict Clippy.

## Next slice

Stage 4K can now organize the selected Application Workspace around the Dossier, Content Catalog,
workflow stage, and contextual actions without inventing another state model. Contextual Agent
assistance should continue to reference catalog identities and use existing consent-gated task
inputs rather than embedding private bodies into navigation state.
