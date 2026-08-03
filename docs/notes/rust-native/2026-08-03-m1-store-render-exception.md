# M1-ARCH-001/002/004 — Store/render exception and failure atomicity

Date: 2026-08-03

## Decision

Retain the single `canisend-store -> canisend-io` normal dependency for the Alpha.6 source
checkpoint. It remains an actual-to-target graph exception, must be reviewed again by 2026-08-10,
and expires on 2026-08-17. The 25-edge target graph still excludes it.

The review rejected an immediate mechanical split because the edge crosses four coupled integrity
surfaces:

- legacy package rendering and revision-bound render-head commit;
- legacy managed projection generation, observation, and repair;
- generic v3 deliverable projection/render export; and
- backup restore and v2/v3 rebuild paths that reuse the same projection rules.

Moving only one call would leave split transaction ownership. Moving all calls in the Alpha.6
window would simultaneously change rendering, projection repair, migration, and recovery behavior.
That creates more data-integrity risk than the visible, source-gated exception.

## Compensating boundary

`RenderService::build_with_executor` separates deterministic rendering from Store's authoritative
prepare/commit transaction. Production uses `EmbeddedRenderExecutor`; tests can inject an exact
failure or change the render stage after compilation and before the immediate transaction. This is
a failure-test seam, not a claim that Store and IO are decoupled. Store independently parses every
executor-produced PDF and verifies its page count before publishing any of its bytes to the CAS.

The authoritative commit remains one immediate SQLite transaction. Typst, PDF, and manifest bytes
may enter the immutable CAS before that transaction. If the commit becomes stale, CanISend does not
delete those digests: a later successful build may reuse them, and otherwise `workspace check`
reports them as verified unreferenced blobs. Automatic deletion is intentionally disabled because
a digest may predate the failed attempt or be shared by another owner.

## Failure matrix

| Boundary | Injected condition | Required result |
|---|---|---|
| Renderer | Executor fails before artifact preparation | No new CAS content; artifacts, revisions, Blob references, render heads, and audit events unchanged |
| Renderer validation | Executor returns bytes that are not a valid PDF | Store rejects the output before CAS publication or authoritative writes |
| Commit freshness | Render stage changes after real embedded compilation | `TaskStale`; transaction rolls back all authoritative writes; each prepared digest remains valid and explicitly auditable |
| Retry | Normal build follows the stale attempt | Existing/shared digests are never deleted; prepared content is reused or remains classified as unreferenced |
| Projector generation | Recipe closure fails | Destination remains absent; manifest becomes `repair-required`; authoritative artifacts, Blob references, and audit events unchanged |
| Projector destination | Managed path is a directory/non-regular target | Write is rejected; no partial temporary projection remains; failure is recorded |
| Projector recovery | Unsafe target is removed and repair repeats | Exact bytes are restored, status becomes `current`, and a second repair is a no-op |

The named regressions are:

- `evidence_and_match_tasks_enforce_stable_revision_bound_identities` in `store_contract.rs`; and
- `repair_failure_marks_projection_and_converges_without_authority_writes` in `projection.rs`.

## Closure and remaining evidence

This supplies the M1-ARCH-001 accepted-exception decision, M1-ARCH-002 compensating render tests,
and the local source portion of M1-ARCH-004. It does not remove the edge, renew it past expiry, or
claim remote/native qualification. M1B still depends on M1A remaining green on the committed change;
the exact GitHub and native candidate evidence is owned by the later release gates.

Focused verification on this source state:

```text
cargo test -p canisend-store --locked
  38 unit + 13 integration tests passed
cargo clippy -p canisend-store -p canisend-app -p xtask --all-targets --locked -- -D warnings
cargo test -p xtask --locked
  79 tests passed
cargo run -p xtask --locked -- release check
cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.6
  27 controlled files; writes_performed: false
```
