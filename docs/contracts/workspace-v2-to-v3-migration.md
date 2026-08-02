# CanISend Workspace v2→v3 migration contract

**Preview format:** `canisend.workspace-migration-preview/v3`

**Result format:** `canisend.workspace-migration-result/v3`

**Source authority:** `canisend.workspace/v2`

**Target authority:** `canisend.workspace/v3`

## Admission boundary

Migration accepts only a fully verified `org.canisend.academic-job` workflow-pack bundle. The Pack
must declare all eight legacy Requirement categories and all four legacy Deliverable kinds. Pack
ID, version, and content digest are copied to every new Application snapshot and every immutable
legacy binding.

The migration refuses to preview or execute when v3 authority is already active, the source
format is not v2, SQLite integrity fails, a referenced Blob is absent or digest-invalid, a legacy
typed artifact is structurally or semantically invalid, or the verified Pack cannot represent the
legacy academic vocabulary.

## Dry run

Dry run performs no migration write. Its body-free report contains:

- source, target, database, and frozen legacy schema versions;
- exact Pack ID, version, and content digest;
- Application and per-table legacy inventory counts;
- the canonical legacy-inventory digest;
- referenced Blob count and verified byte total;
- conservative required backup bytes;
- managed-projection conflict count;
- the verified-backup rollback boundary; and
- a deterministic migration-plan digest.

Job titles, institutions, source text, criteria, evidence, document bodies, review messages, and
projection paths are never serialized into the report. They contribute only through typed v3
snapshots and canonical digests inside the plan hash.

Every v2 Job deterministically maps to one Opportunity retaining the Job ID and one Application ID
derived from the Workspace/Job identities. Repeating a dry run over unchanged authority produces
the same mapping and plan digest.

## Backup and commit

Execution requires the exact reviewed plan digest. It recomputes the complete plan before any
backup; a stale digest fails without creating a backup. A matching request then creates and
verifies a `canisend.backup/v2` backup containing the pre-migration database, configuration, and
every referenced Blob.

One SQLite immediate transaction then:

1. rechecks source inventory, referenced Blob identities, and projection state;
2. inserts v3 authority;
3. inserts one validated neutral Application snapshot per Job;
4. records the migration ledger and one-to-one Job/Application links;
5. records a digest-only binding for every frozen v2 table row;
6. appends body-free Application and Workspace migration audit events; and
7. rechecks source inventory, Blob references, projection state, authority, and Application count.

Any error before commit rolls back every SQLite migration write. The verified backup remains
available. Existing v2 tables, artifact hashes, Blob bytes, `jobs/` projections, user edits, and
unmanaged files are not rewritten, moved, copied, or deleted.

The production transaction reports these logical write boundaries to an internal no-op observer:
authority activation, each Application aggregate, migration ledger, each legacy/Application link,
each legacy-row binding, migration audit, and the verified pre-commit boundary. Tests replace only
that observer with a deterministic interruption; the transaction and write code are otherwise the
production path. Every recorded boundary is interrupted in turn, checked for zero v3 rows/audits,
previewed again with the same digest, and retried to valid v3 authority.

## Semantic mapping

- Job metadata becomes Opportunity/Application metadata and lifecycle.
- Current confirmed Criteria become confirmed Requirements; parse-only Criteria remain proposed.
- Criterion source spans retain exact artifact ID, revision, digest, and byte range.
- Current Application Plans become confirmed or stale Plans with exact historical Requirement
  revisions.
- Planned documents become Pack-qualified Deliverable declarations.
- Materialized document heads become Deliverables with exact content artifact bindings and
  evidence citations.
- All stage, Artifact/revision/dependency, document, review, package, export/render, task,
  consent, audit, discovery, profile, and evidence rows remain authoritative legacy records and
  receive immutable row-digest/Pack bindings.

The legacy inventory excludes only audit rows created by the v3 migration itself. Its digest and
per-table counts must be identical before and after the transaction, and the referenced Blob set
must be byte-for-byte unchanged.

## Recovery boundary

Rollback never attempts an in-place downgrade. Restore the verified pre-migration backup to a new
path and verify it before use. A migration-created backup captures the database schema already
opened by the migrating binary, but retains v2 semantic authority; use it to recover with that
binary or a later compatible binary. Rolling back the executable itself requires the separate,
verified pre-upgrade backup created before the newer binary first opened the Workspace.

A binary that encounters a newer database schema rejects it before setting journal, foreign-key,
or synchronization pragmas and returns the stable diagnostic:

```text
workspace database schema FOUND is newer than supported SUPPORTED; upgrade CanISend or restore a verified pre-upgrade backup to a new path
```

The application failure is non-retryable `upgrade-required` with an explicit instruction not to
modify the newer Workspace or attempt an in-place downgrade. The compatibility probe is read-only,
and qualification verifies that v3 authority and Application counts remain unchanged.

The failure matrix additionally holds a real competing immediate transaction and fills a bounded
local SQLite fixture to `SQLITE_FULL`. Both failures leave no mixed v3 authority and succeed after
the lock or capacity condition is removed. An edited legacy projection is included in the plan's
conflict count and remains byte-for-byte and manifest-digest identical across migration.

Application projections and legacy path recognition remain owned by GF2-PROJ-001.
