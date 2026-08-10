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
