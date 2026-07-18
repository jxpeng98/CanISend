# Backup and recovery

The authoritative workspace is `canisend.toml`, SQLite state, and referenced immutable SHA-256 blobs. Files under
`jobs/`, `profile/`, and `agent/` are projections or scoped exports; they can be rebuilt where deterministic and are
not backup authority.

## Create a verified backup

Stop other writers, check the workspace, and choose a new or empty destination:

```console
canisend --workspace ./applications workspace check
canisend --workspace ./applications workspace backup ./applications-backup
```

The command takes a consistent SQLite snapshot, copies only referenced and verified blobs plus configuration, writes
a hash manifest, verifies the staged backup, and then atomically publishes the destination. It refuses to overwrite
a non-empty directory.

Store the backup separately from the workspace. It contains private adverts, evidence, drafts, review state, and
rendered artifacts even though derived projection files are omitted.

## Restore

Never restore over an existing workspace:

```console
canisend workspace restore ./applications-backup ./applications-restored
canisend --workspace ./applications-restored workspace check
```

Restore verifies the source, copies it to a unique staging directory, rebuilds missing raw/Markdown/JSON/Typst
projections from authoritative blobs, and only then publishes the new destination. Failure removes staging.

## Repair versus restore

Use `workspace repair` when authoritative SQLite/blob state passes `workspace check` but deterministic projection
files are missing or marked repair-required:

```console
canisend --workspace ./applications workspace repair
```

Repair preserves user-edited projections and does not invent a missing authoritative blob. If `workspace check`
reports `blob.reference_invalid`, stop writing and restore a verified backup. Do not replace a content-addressed blob
manually.

For the complete failure model and test evidence, see the
[recovery and interruption matrix](../recovery/interruption-matrix.md).
