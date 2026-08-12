# Implementation plan: remove Store to IO production edge

## 1. Establish Roadmap authority

- [x] Add `M3-ARCH-001` and its sequential-checkpoint consequence to the Master Roadmap.
- [x] Create/link the canonical P1 architecture Issue and milestone without changing Alpha.8
      history or claiming a new release.
- [x] Record exact owner, dependencies, expected evidence, verification tier, branch, and Issue in
      `task.json`.

## 2. Extend the existing seam

- [x] Move the existing `RenderExecutor` to Core and extend it with only the projection and
      validation operations required by current production callers.
- [x] Replace Store's IO-backed error payload with the smallest Core-owned neutral render failure
      category that preserves current App classification.
- [x] Implement the Core trait directly on the existing IO `EmbeddedTypstCompiler`; do not add an
      App wrapper type.
- [x] Add the smallest Store/App checks proving projection, compilation, PDF validation, and
      structured error preservation.

## 3. Remove concrete IO from Store

- [x] Inject the executor into legacy RenderService build/preview/export paths.
- [x] Inject the executor into package projection export/replace/copy/repair paths.
- [x] Inject the executor into Application Deliverable export and Workspace restore/repair paths.
- [x] Remove Store's concrete embedded executor and direct projection/compiler/validator imports.
- [x] Move `canisend-io` out of Store production dependencies; retain a dev-only edge only if the
      existing exact-render integration owner still needs it.

## 4. Compose through the App

- [x] Construct the existing embedded compiler at each shared application-facade entrypoint.
- [x] Update internal Store callers and tests without adding adapter-specific workflow logic.
- [x] Verify CLI, MCP, and Tauri continue to call the same App operations with unchanged receipts.

## 5. Preserve failure and recovery invariants

- [x] Run/add focused positive render/export/restore coverage for both built-in Packs.
- [x] Run/add focused renderer/projector failure, invalid/encrypted PDF, stale-at-commit,
      filesystem conflict, CAS leftover, `RepairRequired`, and convergence regressions.
- [x] Confirm no failure changes artifact/head/reference/audit authority or deletes a shared Blob.

## 6. Reconcile graph and documentation

- [x] Regenerate/update locked actual/target edge policy and remove the temporary exception.
- [x] Update ADR-RN-0019, dependency-assurance guidance, Roadmap status, and Trellis architecture
      guidance to the exact implemented boundary.
- [x] Search for obsolete Store-owned concrete-render wording and date-only exception references.

## 7. Validate and prepare protected integration

- [x] Run `git diff --check` and `cargo fmt --all -- --check`.
- [x] Run focused Core, IO, Store, and App owner tests plus affected-package Clippy.
- [x] Run the workspace dependency-policy check and named atomicity/recovery regressions.
- [x] Run one final `cargo run -p xtask --locked -- release check` on the complete PR head.
- [x] Review with `trellis-check`, update specs with `trellis-update-spec`, commit, push, and open a
      protected PR. Do not merge or publish without separate authorization.

## Rollback points

- Executor/error types only: revert before Store signatures change.
- Store injection: keep commits buildable per layer so a failed path can be reverted without
  weakening tests.
- Graph authority: update only after Cargo metadata proves the production edge is absent.
