# R11.2 Beta contract and migration freeze

**Date:** 2026-07-18

**Roadmap item:** R11.2

## Frozen agent surface

`release/beta-contract-freeze.json` records the Alpha baseline and deterministic hashes for all 40 generated public
schemas plus normalized `agent capabilities` and `agent context` snapshots. The snapshot normalization excludes only
`product_version`; protocol identifiers, public schema version, capability IDs/status, adapters, stable error codes,
stages, execution modes, blockers, next actions, and envelope structure remain part of the digest.

The freeze is recomputed from authoritative generated files by `xtask release check`. A schema or agent-surface edit
therefore fails release verification even when the developer forgets to update a snapshot. The
`release freeze-candidate` command prints a candidate for explicit compatibility review, but the accepted v2 digest
is not silently regenerated.

## Frozen workspace migration baseline

Workspace format `canisend.workspace/v2` remains current at database schema 13. Migration files 1–13 have one
frozen tree digest and cannot be changed or removed. The inventory parser rejects non-contiguous numbers, duplicate
versions, unsafe/symlink files, and disagreement between the highest migration and `DATABASE_SCHEMA_VERSION`.

The policy permits only reviewed, contiguous appends after migration 13 so R11.3 can prove beta-to-RC upgrades. On
every database open, CanISend already rejects a future `user_version`; it now additionally requires the
`schema_migrations` rows to equal every version from 1 through the supported current version. It does not mutate a
future workspace or silently repair incomplete migration history.

## Verification

- Five `xtask` tests pass, including exact freeze comparison.
- Four database-focused tests pass, including transactional v1-to-v13 upgrade, future-version rejection, incomplete
  history rejection, failed-migration rollback, corruption rejection, and writer concurrency.
- `cargo clippy -p xtask -p canisend-store --all-targets -- -D warnings` passes.
- `xtask release check` reports 40 schemas and migrations frozen through 13.

## Transition

The R11.2 agent protocol and workspace migration freeze items are complete. Package-manager candidate generation is
next; signing/notarization remains a separate credential-backed release boundary.
