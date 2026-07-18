# ADR-RN-0009: Freeze Agent v2 and Make Workspace v2 Migrations Append-Only

**Status:** Accepted

**Date:** 2026-07-18

## Context

The native Alpha proved the full agent and workspace flows. Beta users and host integrations now need a concrete
compatibility boundary: ordinary implementation changes must not silently alter accepted JSON, advertised
capabilities, stable errors, workflow stages, or the meaning of an existing SQLite migration.

The product version must still advance, and a later Rust beta/RC may append a migration. An exact digest of every
byte in a versioned snapshot would therefore be too strict if it included `product_version`, while a policy stated
only in prose would be too weak to detect accidental drift.

## Decision

The Beta line freezes these agent surfaces under `canisend.agent/v2` and public schema version `2.0.0`:

- all generated public JSON Schema files and their complete file set;
- the capability registry, discovery adapters, stable error codes, execution stages, modes, and envelope shape in
  the normalized `agent capabilities` snapshot; and
- the body-free context shape, blockers, next actions, stages, modes, and envelope shape in the normalized
  `agent context` snapshot.

Only the `product_version` value is replaced with a marker before snapshot hashing. A semantic contract change
requires a deliberate new protocol/schema version rather than regeneration of the v2 freeze.

Workspace format `canisend.workspace/v2` adopts an append-only migration policy. Migration files 1 through 13 are
immutable and bound by a deterministic tree digest. A future migration must have the next contiguous number; the
current database schema constant must equal the highest migration. Every database open rejects a future
`user_version` and verifies that `schema_migrations` contains exactly every version through the supported schema.
Downgrade and silent history repair are not supported.

`release/beta-contract-freeze.json` is the machine authority. `xtask release check` recomputes the normalized agent
and migration evidence and fails on drift. `release freeze-candidate` exists to show reviewable candidate values; it
does not authorize changing the frozen contract.

## Consequences

- Product-version bumps do not change the normalized protocol digest.
- Adding/removing a schema, error, stage, adapter, mode, or public field fails the release gate.
- Existing migration SQL cannot be edited, renamed, reordered, removed, or duplicated.
- A later migration is possible only as a contiguous append and requires explicit current-version evidence plus
  beta-to-RC upgrade testing.
- A workspace created by a newer unsupported binary fails closed without mutation.
- A workspace with a missing or unexpected migration record fails closed and must be restored from verified state.

## Rejected alternatives

- Freeze only the protocol string: rejected because incompatible payload changes could retain the same string.
- Hash raw snapshots including the product version: rejected because every release bump would create false drift.
- Freeze the entire migration directory forever: rejected because R11.3 explicitly verifies Rust beta-to-RC
  upgrades; existing migrations must be immutable while reviewed append-only evolution remains possible.
- Trust only SQLite `user_version`: rejected because it cannot prove that every declared migration was applied.
