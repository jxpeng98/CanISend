# CanISend Workspace v3 storage contract

**Authority format:** `canisend.workspace/v3`

**Database schema:** 14

**Model format:** `canisend.application-model/v3`

**Runtime status:** Repository and neutral app-service foundation implemented. Authority
activation and semantic v2-to-v3 migration are intentionally unavailable to ordinary runtime
callers.

## Boundary

Migration 14 is a contiguous append-only SQLite migration. It adds dormant Workspace v3 authority,
application head/revision, and dependency tables without editing migrations 1–13 or transforming
existing Job/Agent v2 records.

Opening a Workspace with this binary upgrades its database schema to 14 as an ordinary structural
migration. This does not activate Workspace v3 or reinterpret v2 data. A binary that supports
only schema 13 will reject that database as newer; downgrade requires the project's documented
verified-backup/restore path.

The repository refuses create, read, list, history, and commit operations unless the singleton
authority row contains exactly `canisend.workspace/v3`. The activation function is crate-private
and reserved for the future failure-atomic GF2 migration. Current v2 app services cannot create a
mixed-authority Workspace.

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
revision metadata before returning data.

### `application_model_v3_dependencies`

Stores revision-specific Plan→Requirement and Deliverable→Plan/Evidence edges. Edges are attached
to an immutable Application revision. Stale records continue to point at the historical upstream
revisions that produced them.

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

Pack ID/version/digest changes remain rejected here and are owned by GF2-INVALID-001 after the
explicit Pack-migration workflow exists.

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

These methods do not install Packs, migrate Workspaces, create projections, render, export, call a
provider, use the network, or submit an application.
