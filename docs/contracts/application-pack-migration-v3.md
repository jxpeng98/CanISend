# CanISend Application Pack migration v3 contract

**Preview format:** `canisend.application-pack-migration-preview/v3`

**Database schema:** 17

**Authority:** exact verified source/target Pack bundles and the current immutable Application
revision

## Boundary

Pack migration changes the version and content digest of the Pack already bound to an Application.
It cannot replace the Pack ID, choose `latest`, install a Pack, fetch network content, reinterpret
an unverified bundle, or bypass kernel invariants. A cross-Pack conversion requires a future
explicit import/clone protocol.

The caller supplies already verified source and target bundles. The source ID, semantic version,
and content digest must exactly match the current Application head. The target must retain that ID,
have a strictly greater semantic version and different verified content digest, and declare a
migration from the exact source version.

## Reviewed preview

Preview validates both manifests, all declared source/target mappings, target metadata
requirements, field types/options, and current Application compatibility. Mappings may not point
outside either catalog or collapse multiple source IDs onto one target ID.

The deterministic preview binds:

- current Application revision and snapshot digest;
- exact source and target Pack bindings and manifest digests;
- mapped Plan/Deliverable impact and superseded projection paths; and
- a digest over the complete review payload.

Commit reloads the head and current projection set inside an immediate transaction, recomputes the
preview, and requires the exact reviewed digest. A head, Pack, semantic-impact, or projection-set
race therefore fails before mutation.

## Dependency-scoped invalidation

Every persisted entity advances exactly one revision because its Pack binding changes. Labels,
descriptions, publisher metadata, and other non-semantic vocabulary changes rebind current outputs
without making them stale.

A current Plan becomes stale only when it consumes a Requirement whose mapped category contract
changed, or when the relevant workflow/Deliverable planning contract changed. A materialized
Deliverable becomes stale only when its Plan became stale or its dependency-reached template,
renderer, validator, Evidence contract, or workflow output changed. Stale records retain the exact
historical input revisions and content identities that produced them. Unaffected current outputs
are rebound to the new current upstream revisions.

## Atomicity, history, and projections

One transaction inserts the immutable target snapshot, dependency edges, conditional head
advance, audit event, and one immutable `application_pack_v3_migrations` row. Any validation,
digest, concurrency, or SQLite failure rolls back the entire migration. The ledger records both
exact bindings/manifests, reviewed preview digest, invalidation result, actor, reason, and time.

Projection rows from the source revision are reported as superseded derived state until current
publication rebinds the same managed paths. Repair does not recreate them, and direct replacement
fails stale. An edited superseded projection may be preserved only through explicit copy-as-new;
immutable Application revisions and the Pack-migration ledger retain history.
