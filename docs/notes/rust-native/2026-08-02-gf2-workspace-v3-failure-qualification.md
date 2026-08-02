# GF2-MIG-002 — Workspace v3 migration failure qualification

**Date:** 2026-08-02

**Status:** Implemented locally; exact release-binary lifecycle qualification, work-item linkage,
and independent review remain roadmap governance steps.

## Delivered

- Added a production-path observer over every logical migration write boundary. The production
  observer is a no-op; tests inject an error after each recorded boundary without replacing the
  transaction or write implementation.
- Proved every injected interruption leaves zero v3 authority, Application, dependency,
  migration-ledger, link, binding, and migration-audit rows.
- Proved the pre-migration backup remains verified, the v2 plan digest repeats exactly, and a retry
  with a new backup destination reaches valid v3 authority.
- Added a real external-writer fixture producing bounded `SQLITE_BUSY`/`SQLITE_LOCKED`, followed by
  a successful retry after lock release.
- Added a bounded local database-capacity fixture that reaches real `SQLITE_FULL`, proves complete
  transaction rollback, restores capacity, and retries successfully.
- Added an edited legacy-projection fixture proving preview conflict counting and byte/digest
  preservation.
- Replaced generic future-schema invariant text with a typed, stable refusal before connection
  configuration. The app failure is `upgrade-required` and carries the restore-to-new-path action.
- Proved the schema compatibility check is non-mutating and that the migration backup restores v2
  semantic authority for a compatible binary.

## Recovery distinction

Two backups have different purposes:

1. The migration-created backup is made after the new binary has opened the Workspace. It contains
   that binary's database schema and recovers v2 semantic authority with a compatible binary.
2. Executable rollback to an older schema requires the verified backup created before the newer
   binary first opened the Workspace.

Neither path performs an in-place downgrade. Both restore into a new destination and keep the
upgraded Workspace unchanged for diagnosis.

## Focused evidence

- `migration_v3::tests::every_logical_write_boundary_rolls_back_and_retries_cleanly`
- `migration_v3::tests::database_busy_leaves_verified_backup_and_retryable_v2_authority`
- `migration_v3::tests::sqlite_full_rolls_back_without_mixed_authority`
- `migration_v3::tests::edited_legacy_projection_is_counted_and_preserved_byte_for_byte`
- `migration_v3::tests::older_schema_gate_refuses_v3_without_mutation_and_backup_restores_v2`
- `database::tests::future_schema_and_incomplete_history_are_rejected_without_mutation`
- `error::tests::application_failures_use_stable_adapter_neutral_classes`

## Remaining boundary

The source-level and focused fault matrix is complete. M2 lifecycle qualification must still run
the exact old/new release binaries and candidate archives. GF2-PROJ-001 owns the new
`applications/APPLICATION_ID/` projection family, legacy recognition, symlink, unmanaged-file,
copy/replace, and repair behavior.
