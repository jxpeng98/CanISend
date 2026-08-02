# GF2 Workspace v3 store implementation record

**Date:** 2026-08-02

**Roadmap task:** GF2-STORE-001

**State:** Implemented and source-gate verified in this change. Workspace v3 authority activation,
v2-to-v3 semantic migration, Pack-migration invalidation, generic projections, and Agent v3
operation registration remain separate roadmap tasks.

## Implemented boundary

- Added append-only SQLite migration 14 without changing frozen migrations 1–13.
- Added a singleton Workspace v3 authority gate that ordinary app/runtime callers cannot activate.
- Added exact Pack-bound Application heads and immutable JSON/SHA-256 revision history.
- Added revision-specific Plan→Requirement and Deliverable→Plan/Evidence dependency edges.
- Added load-time digest, Schema, semantic, identity, revision, Opportunity, and Pack verification.
- Added optimistic expected-revision commits inside immediate SQLite transactions.
- Enforced one-step entity revision transitions, immutable identities/timestamps/Pack binding,
  revision-one new entities, and no destructive deletion of persisted model records.
- Added automatic Requirement→Plan→Deliverable stale propagation while preserving historical
  upstream revision bindings.
- Added create/commit and authority audit events with body-free bounded reason codes.
- Added neutral typed app-service receipts for authority, create, show, list, history, and commit.
- Kept every v3 model operation unavailable in a current Workspace v2, preventing mixed authority.
- Extended the application-model Schema to represent stale Plans and historical stale references.

## Regression coverage

- a v2 Workspace rejects v3 create/read and leaves the v3 head table empty;
- exact create/load/history digest round trip;
- dependency and audit inventory after create;
- changed Requirement automatically stales the unchanged Plan and materialized Deliverable;
- stale records preserve the Requirement and Plan revisions that produced them;
- invalid deletion rolls back revision, dependencies, head update, and audit;
- concurrent writers using the same expected revision produce one success and one conflict;
- migration 1 upgrades transactionally through schema 14;
- future schema and incomplete migration history still fail without mutation; and
- the neutral app facade fails closed before Workspace v3 authority activation.

## Verification

```console
cargo test -p canisend-contracts -p canisend-store -p canisend-app --locked
cargo clippy -p canisend-contracts -p canisend-store -p canisend-app \
  --all-targets --all-features --locked -- -D warnings
cargo run -p xtask --locked -- schemas check
cargo run -p xtask --locked -- resources check
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- release check
```

## Remaining boundary

GF2-MIG-001 must produce a body-free dry run, verify/create a pre-migration backup, map the complete
v2 semantic inventory, insert the authority row and v3 records in one failure-atomic boundary,
audit the activation, and re-run integrity checks. The crate-private activation function exists
only for that future path and focused repository tests.

GF2-MIG-002 owns failure injection at every migration write boundary plus old-binary and downgrade
qualification. GF2-PROJ-001 owns `applications/APPLICATION_ID/`. GF2-INVALID-001 owns exact Pack
change preview/approval and selective downstream invalidation.

Agent v3 operation IDs, Pack-aware actor routing, CLI/MCP/desktop adapters, and surface parity are
deferred to GF4/GF5. The new app methods are typed service foundations, not claims of a publicly
active v3 protocol.

## Rollback

Revert migration 14, repository/app modules, stale-model contract extension, generated Schema
updates, Beta freeze current-schema metadata, documentation, and Roadmap evidence together.
For any local database already opened at schema 14, use a verified pre-upgrade backup restored to
a new path before running an older binary; do not edit `user_version` or drop tables manually.
