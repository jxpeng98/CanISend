# Backup and recovery

The authoritative workspace is `canisend.toml`, SQLite state, and referenced immutable SHA-256 blobs. Files under
`applications/`, `jobs/`, `profile/`, and `agent/` are projections or scoped exports; they can be rebuilt where
deterministic and are not backup authority.

The configuration also freezes the exact workflow Pack identity and digest. A backup of
`org.canisend.generic-application` restores a Generic Workspace; a backup of
`org.canisend.academic-job` restores the Academic compatibility authority. Restore never changes
Packs or converts Workspace v2 content into a different ontology.

Application Dossiers, the Content Catalog, Agent guidance, and search indexes are not additional
workspace files. Dossiers, Catalog entries, metadata search, and body-free Agent guidance are
rebuilt from current authoritative records whenever they are requested. A consented private
full-text index exists only in bounded process memory for one search and is discarded afterwards.
Backups therefore include the authoritative source artifacts but never copy a Catalog database,
Agent transcript, private search index, returned snippet, or GUI navigation cache.

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

A Workspace v2→v3 semantic migration creates its own verified pre-migration backup at the exact
destination approved with `workspace migrate`. Keep that backup separate from routine backups and
record the binary version that created it. The migration backup preserves the Academic Pack; it is
not a Generic Workspace seed.

## Restore

Never restore over an existing workspace:

```console
canisend workspace restore ./applications-backup ./applications-restored
canisend --workspace ./applications-restored workspace check
```

Restore verifies the source, copies it to a unique staging directory, rebuilds missing neutral Application and
legacy raw/Markdown/JSON/Typst projections from authoritative records and blobs, and only then publishes the new
destination. Failure removes staging.

After restore, inspect `workspace status` before any mutation and confirm that its Pack ID and
Workspace authority generation match the backup you intended to restore. The application services
continue to validate the exact embedded Pack digest before Pack-bound operations.

The macOS GUI exposes the same operation under **Workspaces → Restore backup**. It shows the backup
and destination before confirmation, and adds the restored workspace to the GUI registry only
after the verified restore succeeds.

## Repair versus restore

Use `workspace repair` when authoritative SQLite/blob state passes `workspace check` but deterministic projection
files are missing or marked repair-required:

```console
canisend --workspace ./applications workspace repair
```

Repair preserves user-edited projections and does not invent a missing authoritative blob. If `workspace check`
reports `blob.reference_invalid`, stop writing and restore a verified backup. Do not replace a content-addressed blob
manually.

The same bounded repair is available as **Workspaces → Repair active** in the macOS GUI.

If a Dossier, Catalog result, or Agent recommendation appears stale after an interrupted UI
operation, refresh or reopen the selected application. Do not run `workspace repair` merely to
refresh a read model. If the underlying workspace passes `workspace check`, deterministic
read-model rebuild is sufficient; if the check fails, follow the restore procedure above.

For the complete failure model and test evidence, see the
[recovery and interruption matrix](../recovery/interruption-matrix.md).
