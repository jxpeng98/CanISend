# Design: move and reuse the render seam

## Boundary

Move the existing `RenderExecutor` trait to `canisend-core` instead of adding another abstraction,
then expand it only for current projection and PDF-validation callers. The trait carries existing
contract records, verified Pack inputs, content bytes, adapter-neutral render failures, and bounded
output metadata. The existing IO `EmbeddedTypstCompiler` implements it directly; `canisend-app`
constructs that compiler and injects it into Store persistence services.

The trait already has real failure/stale test implementations, so this is reuse rather than a
single-implementation interface. No dynamic discovery, registry, new crate, or alternate renderer
is introduced.

## Ownership after the change

- `canisend-contracts`: unchanged.
- `canisend-core`: owns the moved `RenderExecutor`, neutral render errors, and output values, as
  required by the project directory guideline for port traits.
- `canisend-io`: unchanged ownership of Typst projection, embedded compilation, PDF parsing, and
  the existing `EmbeddedTypstCompiler`, which directly implements the Core trait.
- `canisend-store`: authoritative reads, prepared Blob writes, revision-bound commit, managed
  filesystem projection, observation/repair state, and audit records. Methods that render receive
  the executor explicitly; Store has no default concrete adapter.
- `canisend-app`: composition root. It constructs the existing IO compiler and passes it to legacy
  render, projection/export, Application Deliverable export, Workspace repair, and restore flows.

## Data flows

### Legacy document render

1. Store loads the current package and immutable structured Documents.
2. The injected executor projects and compiles each Document outside the commit transaction.
3. Store asks the executor to validate returned or persisted PDF bytes and verifies reported
   metadata.
4. Store prepares immutable Typst/PDF/manifest Blobs.
5. One immediate transaction rechecks package, Document, and render-stage revisions, then commits
   artifact rows, Blob references, head, stage state, and audit event.

Renderer failure occurs before prepared authority. A stale stage at step 5 rolls back every
authoritative row; any immutable unreferenced digest remains auditable and recoverable exactly as
today.

### Managed projection and repair

Store continues to derive Markdown/JSON/package projections itself. Only Typst generation crosses
the injected executor. The existing path preflight, edit preservation, generated-digest comparison,
`RepairRequired` recording, and idempotent convergence stay unchanged.

### Application Deliverable export

Store validates the Pack-bound Application snapshot, expected revision, destination, approved
Deliverables, and content references. The injected executor projects the verified Pack template and
compiles each PDF. Store writes the new export directory and commits the existing receipt/audit
state only after all outputs are ready and the revision remains current.

### Workspace restore and repair

The App supplies the executor to restore/repair entrypoints. Restore keeps the staging directory
private, opens the staged Workspace, rebuilds managed projections through the executor, and renames
the staging directory only after convergence. An executor or path failure removes the staging
directory and does not replace the destination.

## Error compatibility

The IO implementation maps its projection failures to Core-owned neutral categories without
stringifying them. `StoreError` wraps the Core render error, while App preserves the current
`ErrorCode`, retryability, and remediation. No `canisend-io` type remains in `StoreError`.

## Dependency graph

`canisend-store` removes its normal IO dependency. If existing exact-render Store integration tests
still require IO, move that dependency to `[dev-dependencies]` and record the dev-only edge in both
actual and target machine policy; production target diagrams contain no Store→IO edge. The
temporary exception is deleted.

## Compatibility and release boundary

This is an internal composition change. It does not change Workspace, Agent, Pack, operation,
approval, schema, or filesystem contracts. Published Alpha.8 remains immutable and cannot be used
as evidence for these changed bytes. A later, separately authorized sequential Alpha checkpoint
must bind the refactor before affected cohort evidence advances.

## Rollback

Before merge, revert the branch. If focused atomicity/recovery tests reveal a higher integrity risk,
stop and retain the current exception without moving its date; a new explicit review is a separate
Roadmap decision. After merge, never rewrite Alpha.8 or its evidence—fix forward on the next
sequential Alpha source.
