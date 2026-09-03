# SQLite and Blob Storage Guidelines

> Database patterns and invariants for CanISend.

---

## Overview

SQLite plus immutable content-addressed Blobs is authoritative. Markdown, Typst, PDF, and Agent
files are projections, verified resources, or exports. The product uses `rusqlite` directly; do
not introduce an ORM or let an adapter write `.canisend/`.

## Query Patterns

- Use bound `rusqlite::params!`; never interpolate user-controlled values into SQL.
- Use `Database::immediate_transaction()` for multi-row mutations, verify expected revisions
  inside the transaction, and commit once at the end.
- Keep reads deterministic with explicit ordering when results cross a public boundary.
- Store content by verified SHA-256 digest through `BlobStore`; never overwrite an existing Blob.

## Migrations

- Add the next numbered file under `crates/canisend-store/migrations/`.
- Include it and advance `DATABASE_SCHEMA_VERSION` in `crates/canisend-store/src/database.rs`.
- Apply it transactionally and retain the full `schema_migrations` sequence check.
- Never edit an applied migration or silently upgrade an unsupported Workspace format.

## Scenario: atomic pending migration sequence

### 1. Scope / Trigger

Use this contract whenever `Database::open` has one or more schema versions to apply.

### 2. Signatures

- `Database::migrate()` resolves the current and supported schema versions.
- `Database::apply_migration_sequence(current, target, migrations, applied_at)` applies the
  ordered pending SQL files.
- `Database::verify_migration_history(expected_version)` verifies exact history through a version.

### 3. Contracts

- Reject a newer schema before configuration or migration.
- For a nonzero older schema, verify exact history `1..=current` before running pending SQL.
- Require pending versions to equal `current + 1..=target` in order.
- Execute every pending SQL file, its `user_version` check, and its history row in one immediate
  transaction, then commit once.

### 4. Validation & Error Matrix

| Condition | Result |
|---|---|
| Schema is newer than supported | Typed unsupported-version error; no migration |
| Existing history is missing, duplicated, or out of order | Invariant error before pending SQL |
| Pending versions are not contiguous | Invariant error before the transaction |
| SQL fails or sets the wrong `user_version` | Entire pending sequence rolls back |

### 5. Good / Base / Bad Cases

- Good: versions 18–20 apply together and publish history only after version 20 succeeds.
- Base: a current schema performs no migration and verifies full history.
- Bad: commit version 19, then attempt version 20 in a separate transaction.

### 6. Tests Required

- Inject a valid pending migration followed by an invalid one; assert the original schema objects,
  history, and `user_version`, then correct it and prove retry succeeds.
- Reject incomplete history on both current and older schemas without applying pending columns.
- Retain fresh-version, version-one upgrade, and newer-schema refusal coverage.

### 7. Wrong vs Correct

```rust
// Wrong: an early version remains committed if a later version fails.
for migration in pending {
    apply_and_commit(migration)?;
}

// Correct: the pending chain shares one immediate transaction and one final commit.
apply_migration_sequence(current, target, migrations, applied_at)?;
```

## Naming and Connection Rules

SQL identifiers use `snake_case`. Head tables identify the authoritative revision; immutable or
audit records are append-only unless an accepted contract says otherwise. Foreign keys remain
enabled and the connection keeps WAL, `synchronous=FULL`, and the bounded busy timeout.

## Common Mistakes

- Opening unsupported Workspace versions before format validation can mutate them through the
  migration runner; `Workspace::open_v4_from` checks first.
- Publishing filesystem projections before a transaction commits can expose rejected state.
- Deleting an unreferenced Blob automatically can remove content needed for audit or recovery.

Examples: `crates/canisend-store/src/database.rs`, `blob.rs`, `workspace.rs`, and
`application_v3.rs`.
