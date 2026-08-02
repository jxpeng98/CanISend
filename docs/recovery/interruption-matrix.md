# Recovery and interruption matrix

**Applies to:** CanISend workspace format v2 and the v2→v3 authority transition

**Reviewed:** 2026-08-02

CanISend treats SQLite rows and immutable blobs as authoritative state. Files under `applications/`,
`jobs/`, `profile/`, and `agent/` are projections or scoped exports and are not backup authority. A command
may publish an immutable blob before its SQLite transaction commits; such a blob is deliberately
retained as an auditable, unreferenced object. A committed database reference must always resolve
to a verified blob.

## Interruption boundaries

| Boundary | Injected failure or race | Required postcondition | Automated evidence |
| --- | --- | --- | --- |
| Blob stream before publication | Reader returns an error after a partial write | No destination blob and no temporary file remain | `blobs_are_bounded_immutable_verified_and_auditable` |
| Blob publication before SQLite commit | Artifact dependency validation rejects the transaction after bytes are published | No partial artifact/event rows; immutable bytes remain visible as an unreferenced blob | `artifact_commit_stales_dependents_and_projection_repairs` |
| SQLite migration before commit | Invalid migration statement | Migration version and schema remain unchanged | `database::tests::migration_failure_rolls_back_and_corrupt_database_fails_closed` |
| Competing SQLite writers | A second immediate transaction starts while the first is held | The second writer fails within the bounded busy timeout; the first can roll back cleanly | `database::tests::readers_coexist_and_second_writer_conflicts` |
| Authoritative commit before projection publication | A regular file blocks projection directory creation | Authoritative artifact remains readable; manifest records repair-required; retry rebuilds the file | `artifact_commit_stales_dependents_and_projection_repairs` |
| Backup while a referenced blob is missing | Backup verifies the staged snapshot after copying | Backup fails, final destination is absent, and the partial staging directory is removed | `recovery_interrupted_backup_removes_partial_destination` |
| Restore before derived projections exist | Verified backup contains SQLite and blobs but omits derived files | Restore regenerates raw, Markdown, JSON, and Typst projections before atomic destination publication | `recovery_verified_backup_restores_into_new_workspace`; `projection::tests::recovery_restore_rebuilds_managed_projections_from_authoritative_blobs` |
| Concurrent completion of one host-agent task | Two independent processes submit the same lease and candidate together | Exactly one non-idempotent commit, one idempotent replay, and one output artifact | `recovery_concurrent_host_agents_commit_one_idempotent_result` |
| Completion after input revision changes | A source is imported after task preparation | Completion fails as stale and cannot commit the candidate | `agent_tasks_validate_commit_idempotently_and_detect_changed_jobs` |
| Intake confirmation after job revision changes | A second source commit lands after the first source preview | The old preview fails with a dependency conflict; current assistance is rebuilt from revision 2 | `stale_intake_preview_fails_and_assistance_rebuilds_from_current_revisions` |
| Catalog reopen and concurrent readers | Four readers rebuild the Catalog and metadata index from the same workspace | Every Catalog is identical; readers do not contend; no private-body index survives a subsequent metadata-only search | `catalog_rebuild_is_deterministic_ephemeral_and_concurrent` |
| Malformed local search or navigation memory | A control character, unbounded limit, oversized memory value, invalid route, or invalid date is supplied | Input is rejected or the malformed optional field is discarded without losing valid workspace/application continuity | `catalog_rebuild_is_deterministic_ephemeral_and_concurrent`; `workflow-navigation.test.ts` |
| Workspace check after blob loss or replacement | One referenced blob is removed and another is replaced with different bytes | Check fails closed and identifies both reference digests as invalid | `recovery_check_detects_missing_and_corrupted_referenced_blobs` |
| Workspace open after database corruption | SQLite content is replaced with invalid bytes | Open fails; no repair is attempted over the damaged authority | `database::tests::migration_failure_rolls_back_and_corrupt_database_fails_closed` |
| Workspace v2→v3 logical write boundary | Deterministic interruption after each authority, Application, ledger, link, binding, audit, and pre-commit verification boundary | The entire transaction rolls back, the verified backup remains, the v2 plan digest repeats, and retry reaches valid v3 authority | `migration_v3::tests::every_logical_write_boundary_rolls_back_and_retries_cleanly` |
| Workspace v2→v3 database capacity | A bounded local fixture reaches real `SQLITE_FULL` before the migration write | No v3 row or migration audit remains; the backup verifies; restoring capacity allows exact-plan retry | `migration_v3::tests::sqlite_full_rolls_back_without_mixed_authority` |
| Workspace v2→v3 competing writer | A second connection holds an immediate transaction through the migration attempt | Bounded `SQLITE_BUSY`/`SQLITE_LOCKED`, verified backup, no mixed authority, and successful retry after lock release | `migration_v3::tests::database_busy_leaves_verified_backup_and_retryable_v2_authority` |
| Workspace v2→v3 edited projection | A legacy projection manifest is `edited` and its file contains user-owned bytes | Preview counts the conflict; migration changes neither the file bytes nor projection-state digest | `migration_v3::tests::edited_legacy_projection_is_counted_and_preserved_byte_for_byte` |
| Older schema gate opens a newer Workspace | Compatibility check supports schema N−1 while the Workspace is schema N | Stable upgrade/restore refusal occurs before configuration or migration; v3 rows and authority remain unchanged | `database::tests::future_schema_and_incomplete_history_are_rejected_without_mutation`; `migration_v3::tests::older_schema_gate_refuses_v3_without_mutation_and_backup_restores_v2` |
| Application v3 projection preflight | An unmanaged target, symbolic-link parent, or missing authoritative content Blob is present | Publication fails before v3 manifest ownership or writes outside the managed root | `application_projection_v3::tests::unmanaged_missing_blob_and_symlink_paths_fail_before_projection_ownership` |
| Application v3 projection edit and repair | One managed file is edited and another is missing | Repair rebuilds only the missing file; replace/copy require explicit user action and never change Application authority | `application_projection_v3::tests::generic_projections_preserve_edits_copy_replace_and_repair` |
| Application v3 backup restore | Derived files are omitted while materialized Deliverable content is referenced | Backup includes the verified content Blob; restore rebuilds every v3 projection in staging and repeated repair is idempotent | `application_projection_v3::tests::backup_restore_rebuilds_generic_projections_from_authoritative_content` |
| Migrated academic projection recognition | A linked Job has one managed and one unmanaged legacy file | Only the manifest-backed path is reported read-only; neither file is re-owned or changed | `application_projection_v3::tests::migrated_academic_legacy_projection_is_recognized_but_never_reowned` |

The complete workspace suite in `fast-ci` runs these contracts on Apple Silicon macOS during
development. Release candidates run the same complete suite in the Linux source gate and the
bounded `recovery_` subset in `windows-release-tests`. Windows and Linux native validation is
therefore release-only, while the macOS development loop keeps immediate recovery coverage.

## Restore behavior

`workspace backup` writes to a unique sibling staging directory, verifies the complete manifest,
and only then renames it to the requested destination. `workspace restore` follows the same rule:

1. verify every manifest entry and referenced blob in the source backup;
2. copy the backup into a private staging directory;
3. remove the backup-only manifest and create empty derived directories;
4. open the staged workspace and rebuild missing legacy and Application projections from
   authoritative records and blobs;
5. atomically rename the staged workspace to the requested destination;
6. remove staging automatically on any failure before the rename.

Restore never overwrites an existing destination. Edited projection files are preserved by normal
`workspace repair`, but cannot be recovered from a backup because projections are intentionally not
authoritative. The regenerated file is the deterministic projection of the recorded artifact
revision.

The automatic backup created by v2→v3 semantic migration already uses the database schema of the
migrating binary. It restores v2 semantic authority for that binary or a later compatible binary.
Executable rollback to an older schema requires the separately retained backup created before the
new binary first opened the Workspace.

## Read-model and search recovery

Application Dossiers, the Content Catalog, Agent assistance, Agent handoff context, and metadata
search are read models. They are rebuilt from the current SQLite rows and immutable blob identities
each time the application facade is called; none is a second state authority.

The optional private full-text index is even narrower: after explicit private-read consent,
CanISend verifies eligible blobs, builds a bounded index in process memory, returns bounded
snippets, and drops the entire index when that call ends. It is never written to SQLite,
projections, the workspace registry, navigation memory, diagnostics, handoff files, or backups.

Consequently:

- no migration, backup copy, projection repair, or downgrade transform exists for these read
  models;
- reopening the workspace or refreshing the current application is the recovery action;
- a mutation invalidates UI guidance, and the next Dossier/Catalog/assistance call observes the
  new authoritative revision;
- a malformed optional registry or navigation field is rejected or discarded without modifying
  workspace authority; and
- rollback follows the executable/schema procedure in
  [Upgrade, Roll Back, and Uninstall](../guides/upgrade-and-rollback.md), not a read-model rollback.

## Operator procedure

Before creating a backup:

```console
canisend --workspace ./my-workspace workspace check
canisend --workspace ./my-workspace workspace backup ./canisend-backup
```

Restore into a new path and verify it before use:

```console
canisend workspace restore ./canisend-backup ./restored-workspace
canisend --workspace ./restored-workspace workspace check
```

If `workspace check` reports `blob.reference_invalid`, stop writing to the workspace and restore
from a verified backup. `workspace repair` repairs only deterministic projections; it does not
invent or replace missing authoritative content.
