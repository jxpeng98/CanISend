# Recovery and interruption matrix

**Applies to:** CanISend workspace format v2

**Reviewed:** 2026-07-18

CanISend treats SQLite rows and immutable blobs as authoritative state. Files under `jobs/`,
`profile/`, and `agent/` are projections or scoped exports and are not backup authority. A command
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
| Workspace check after blob loss or replacement | One referenced blob is removed and another is replaced with different bytes | Check fails closed and identifies both reference digests as invalid | `recovery_check_detects_missing_and_corrupted_referenced_blobs` |
| Workspace open after database corruption | SQLite content is replaced with invalid bytes | Open fails; no repair is attempted over the damaged authority | `database::tests::migration_failure_rolls_back_and_corrupt_database_fails_closed` |

The `recovery-native` CI matrix runs the `recovery_` contract subset on Linux x86_64, macOS
arm64, and Windows x86_64. The full quality job separately runs every store and protocol test.

## Restore behavior

`workspace backup` writes to a unique sibling staging directory, verifies the complete manifest,
and only then renames it to the requested destination. `workspace restore` follows the same rule:

1. verify every manifest entry and referenced blob in the source backup;
2. copy the backup into a private staging directory;
3. remove the backup-only manifest and create empty derived directories;
4. open the staged workspace and rebuild missing projections from authoritative blobs;
5. atomically rename the staged workspace to the requested destination;
6. remove staging automatically on any failure before the rename.

Restore never overwrites an existing destination. Edited projection files are preserved by normal
`workspace repair`, but cannot be recovered from a backup because projections are intentionally not
authoritative. The regenerated file is the deterministic projection of the recorded artifact
revision.

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
