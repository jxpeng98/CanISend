# Remove the Store to IO production edge

## Goal

Remove the expiring normal `canisend-store -> canisend-io` dependency before Beta freeze while
preserving render, projection, export, backup/restore, recovery, error, and audit behavior. This
turns the existing Store render seam plus App composition into the actual production boundary
instead of renewing the 2026-08-17 exception.

## Background

- ADR-RN-0019 and the machine dependency policy retain exactly one actual/target production-graph
  delta: `canisend-store -> canisend-io`.
- The reviewed exception hard-expires on 2026-08-17 and explicitly forbids a date-only renewal.
- Store currently invokes IO from legacy document rendering, managed Typst projection/repair, and
  v3 Deliverable PDF export; `RenderExecutor` covers only part of that surface.
- `M3-EVID-005` / Issue #70 requires real invited users and cannot be advanced with synthetic
  evidence. One architecture/safety task may proceed beside that evidence task.

## Requirements

1. Move the existing `RenderExecutor` seam to `canisend-core` and expand it to cover the required
   projection, compilation, and PDF-validation calls. Do not add a second trait, new crate,
   service registry, factory, runtime plugin system, or render engine.
2. Keep Typst templates, compilation, PDF parsing, and concrete render limits in `canisend-io`;
   construct the concrete adapter in `canisend-app` and pass it into Store-owned persistence flows.
3. Remove every production import and normal Cargo dependency from `canisend-store` to
   `canisend-io`. A test-only edge may remain only where an exact real-render integration fixture
   proves behavior that a fake port cannot.
4. Preserve Store ownership of SQLite transactions, immutable Blob writes, projection manifests,
   path defenses, revision rechecks, stale detection, audit events, and recovery state.
5. Preserve observable App, CLI, MCP, and Tauri operation IDs, receipts, error codes,
   retryability, consent boundaries, and no-submission behavior. No public schema or Workspace
   migration changes are allowed.
6. Cover legacy document build/preview/export, package projection export/reconcile/repair,
   Application Deliverable export, Workspace repair, and restore-to-new-path through the same
   concrete adapter wiring.
7. Retain positive and negative evidence for renderer/projector failure, invalid or encrypted
   PDF, stale-at-commit, partial filesystem write, `RepairRequired`, CAS leftovers, and repair
   convergence. Failures must not create authoritative artifact/head/reference/audit writes.
8. Update ADR-RN-0019, the machine dependency policy, the Master Roadmap, and Trellis guidance to
   describe the resulting graph. Remove the temporary exception rather than changing its date.
9. Preserve immutable Alpha.8 history. Because this task changes product source, the Roadmap must
   require a new exact sequential Alpha checkpoint before affected cohort evidence is resumed or
   used for Beta readiness.

## Acceptance Criteria

- [x] Locked Cargo metadata contains no normal `canisend-store -> canisend-io` edge, and the
      machine actual graph equals its reviewed target with no Store/IO temporary exception.
- [x] Both built-in Packs still complete their existing render/export paths through
      `canisend-app`; App-closed CLI/MCP surfaces retain the same application-facade behavior.
- [x] Focused Store tests prove success, renderer failure, invalid PDF, stale-at-commit,
      projection failure, filesystem conflict, and idempotent repair without partial authority.
- [x] Focused App/IO tests prove the concrete adapter projects, compiles, validates, exports, repairs,
      and restores using the existing verified resources and render limits.
- [x] Existing stable error classifications remain unchanged for encrypted/malformed PDF,
      invalid input, stale work, workspace conflict, external IO, and internal render failure.
- [ ] `cargo fmt`, affected-package Clippy/tests, dependency-policy verification, and one final
      `cargo run -p xtask --locked -- release check` pass.
- [x] Roadmap task `M3-ARCH-001` is linked to a public Issue before implementation becomes Ready;
      release/publication work remains separately authorized.

## Out of Scope

- Collecting or manufacturing cohort evidence for Issue #70.
- Publishing, tagging, promoting, signing, or qualifying the next Alpha checkpoint.
- Changing templates, PDF appearance, Pack schemas, operation schemas, Workspace format, or
  legacy compatibility policy.
- Resolving unrelated dependency-advisory exceptions that share the 2026-08-17 review date.

## Execution State

This task is In progress. The reviewed plan, linked Roadmap Issue, and user approval are complete;
implementation still requires the final source gate, protected PR, and CI evidence.
