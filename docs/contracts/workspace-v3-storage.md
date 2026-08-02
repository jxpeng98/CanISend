# CanISend Workspace v3 storage contract

**Authority format:** `canisend.workspace/v3`

**Database schema:** 17

**Model format:** `canisend.application-model/v3`

**Runtime status:** Repository, neutral app services, and an explicit dry-run-first,
verified-backup v2→v3 migration service, neutral Application projections, and exact Pack migration
with dependency-scoped invalidation are implemented. CLI, MCP, desktop migration/projection/Pack
controls and canonical Agent v3 operations remain unavailable.

## Boundary

Migration 14 is a contiguous append-only SQLite migration. It adds dormant Workspace v3 authority,
application head/revision, and dependency tables without editing migrations 1–13 or transforming
existing Job/Agent v2 records.

Opening a Workspace with this binary upgrades its database schema through 17 as an ordinary
structural migration. Migration 15 adds the migration ledger, one-to-one Job/Application links,
and immutable legacy-row bindings. Migration 16 adds the neutral Application projection manifest.
Migration 17 adds the immutable Application Pack-migration ledger. None of these structural
migrations activates Workspace v3 or reinterprets v2 data. A binary that supports an older schema
rejects that database as newer; downgrade uses a verified pre-migration backup restored to a new
path.

The repository refuses create, read, list, history, and commit operations unless the singleton
authority row contains exactly `canisend.workspace/v3`. Only the migration service can activate
that row for an existing v2 Workspace. The status read model reports v3 after activation while the
outer v2 configuration remains a transitional storage locator for current adapters.

The complete migration protocol is defined in the
[Workspace v2→v3 migration contract](workspace-v2-to-v3-migration.md).

## Tables

### `workspace_v3_authority`

Stores one exact authority format, activation timestamp, and body-free reason code. Application heads carry a
foreign key to this row, so no v3 model can be committed before authority activation.

### `application_model_v3_heads`

Stores one current head per Application with:

- Application and Opportunity identities;
- exact Pack ID, semantic version, and content digest;
- positive head revision; and
- immutable creation plus current update timestamps.

Pack identity is denormalized deliberately and rechecked against every loaded snapshot. A normal
commit cannot change it; Pack change requires the dedicated migration/invalidation boundary.

### `application_model_v3_revisions`

Stores immutable compact JSON snapshots, SHA-256 over the exact stored bytes, actor, bounded
lowercase kebab-case reason code, and commit timestamp under `(application_id, revision)`. Reads recalculate the digest,
run the v3 structural/semantic contract, and compare head identity, Pack, Opportunity, and
revision metadata before returning data. Every materialized Deliverable content identity is also
registered in the shared Blob-reference ledger in the same transaction, so check, backup, and
restore cover v3 content even before a projection is requested.

### `application_model_v3_dependencies`

Stores revision-specific Plan→Requirement and Deliverable→Plan/Evidence edges. Edges are attached
to an immutable Application revision. Stale records continue to point at the historical upstream
revisions that produced them.

### Migration ledger and compatibility bindings

`workspace_v3_migrations` records exact source/target versions, Pack identity, reviewed plan
digest, source-inventory digest, Blob counts/bytes, verified-backup manifest digest, and bounded
times. `workspace_v3_application_links` records the deterministic one-to-one legacy Job,
Opportunity, and Application mapping. `workspace_v3_legacy_bindings` binds every source row from
the frozen v2 table inventory to its row digest, optional Application, and exact academic Pack
digest without copying private bodies or changing legacy rows.

### `application_projection_v3_manifests`

Stores managed `applications/APPLICATION_ID/` paths bound to an immutable Application revision,
snapshot digest, exact Pack identity, optional Deliverable revision, source/generated/observed
digests, and edit status. It is independent of v2 Artifact foreign keys and fixed document kinds.
The complete publication, reconciliation, legacy-recognition, and recovery behavior is defined in
the [Application projection v3 contract](application-projections-v3.md).

### `application_pack_v3_migrations`

Records one failure-atomic Pack migration between immutable source and target Application
revisions. Each row binds the same Pack ID, exact source/target versions and verified content
digests, both manifest digests, reviewed preview digest, Plan invalidation result, stale
Deliverable count, actor, reason, and commit time. The complete protocol is defined in the
[Application Pack migration v3 contract](application-pack-migration-v3.md).

## Commit protocol

Create and commit use SQLite `BEGIN IMMEDIATE` transactions. A create requires revision one for
every initial entity. A commit requires:

- the exact expected current Application revision;
- the next positive Application revision and a strictly later `updated_at`;
- immutable Application, Opportunity, creation-time, and Pack identities;
- unchanged entity content retaining its revision;
- changed existing entity content advancing exactly one revision;
- new Requirement, Plan, or Deliverable identities beginning at revision one; and
- no deletion of persisted Requirements, Plans, or Deliverables.

Removal is represented through neutral lifecycle, exclusion, omission, or stale states so history
is not erased. Pack migration is rejected from the ordinary commit path.

The transaction writes the immutable snapshot revision, dependency edges, conditional head
advance, and one audit event. Any validation, conflict, digest, SQL, or head-update failure rolls
back all of them.

## Stale propagation

When a Requirement revision changes and an unchanged current Plan consumed the earlier revision,
the repository advances the Plan once and marks it stale while preserving its historical
Requirement references. Any unchanged materialized Deliverable affected by that Plan change or by
a referenced changed Requirement is likewise advanced once and marked stale while preserving its
historical Plan/content bindings.

A caller may provide a genuinely replanned or regenerated next revision bound to current inputs;
the repository does not overwrite it with stale state. Already stale records are not repeatedly
advanced merely because another upstream revision changes.

Ordinary commits still reject Pack ID/version/digest changes. The dedicated migration service
accepts only an exact current source binding and a verified, higher target version with the same
Pack ID and an explicit predecessor migration. It advances all Pack-bound entities, rebinds
unaffected current dependencies, and marks only dependency-reached Plan/Deliverable outputs stale.

## Concurrency and audit

SQLite serializes writers. The repository reloads the current head inside the immediate
transaction and compares it with the caller's expected revision. Two writers using the same
expected revision cannot both commit; exactly one may advance the head.

Authority activation, Application creation, and every successful revision commit append audit
events. Failed commits append neither history nor dependencies nor audit. Revision-history
metadata is body-free and exposes only digest, actor, reason code, revision, and timestamp.

## App-service boundary

`canisend-app` exposes typed create, show, list, history, commit, and authority methods over the
repository. User-facing writes record `ActorKind::User`; Agent v3 actor routing is deferred until
the canonical v3 operation registry is implemented. On a current v2 Workspace every method fails
closed at the authority gate before model mutation.

These methods do not install Packs, migrate Workspaces, render, export, call a provider, use the
network, or submit an application. Projection operations are a separate Store service and are not
yet exposed by the neutral app-service surface.
