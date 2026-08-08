# Upgrade, Roll Back, and Uninstall CanISend

CanISend is a native executable, but its workspaces contain versioned SQLite migrations. Treat the executable and
each workspace as separate upgrade surfaces. Replacing a binary is reversible; an opened workspace may have advanced
to a schema that an older binary must reject.

This guide applies to verified release-archive installations. Homebrew, Scoop, and WinGet files remain unpublished
candidates until their signed RC/Stable lifecycle matrices pass.

## Before every upgrade

1. Download the new archive, `SHA256SUMS`, manifest, and signing evidence when the stage requires it. Complete the
   [release verification procedure](release-verification.md) before extracting.
2. Record the currently installed binary identity:

   ```console
   canisend version --json
   canisend doctor --json
   ```

3. Stop CanISend commands and pause every Codex, Claude, provider, or other host task that could write to the
   workspace. Do not upgrade during an active task lease or concurrent writer.
4. Check and back up every important workspace to a new destination on separate storage:

   ```console
   canisend --workspace ./applications workspace check --json
   canisend --workspace ./applications workspace backup \
     ./backups/applications-before-VERSION --json
   ```

5. Retain the previous verified executable archive and its notices until the new version and all workspaces have
   passed acceptance. A copy of the old binary is not a workspace backup.

Never copy an executable, database, or release file into `.canisend/`. Never edit `schema_migrations` manually.

## Discover Pack and Workspace authority before mutation

An executable upgrade and the semantic Workspace v2→v3 transition are separate operations. With
the new binary, inspect each Workspace before running a job, Application, task, or Agent write:

```console
canisend --workspace ./applications workspace status
```

Confirm the reported authority and exact Pack:

- `org.canisend.generic-application` is a canonical Workspace v3 and uses Generic CLI/Agent v3.
- `org.canisend.academic-job` retains the Academic compatibility journey. A v2 Workspace can be
  previewed and migrated, but its Pack remains academic.

Do not interpret v2→v3 as academic-to-generic conversion. To use the Generic Pack, create a new
Workspace with `workspace init --pack generic-application`.

For an eligible existing Workspace v2, stop all writers and create the body-free plan:

```console
canisend --workspace ./academic-applications workspace check --json
canisend --workspace ./academic-applications workspace migration-preview --json
```

Review the exact Pack binding, Application count, projection conflicts, required backup bytes, and
`migration_plan_sha256`. Commit only that digest to a new backup destination:

```console
canisend --workspace ./academic-applications workspace migrate \
  --expected-plan-sha256 MIGRATION_PLAN_SHA256 \
  --backup-destination ./backups/academic-before-v3 --json
```

CanISend revalidates the plan, creates and verifies the backup before mutation, and fails without a
commit when revisions, Pack identity, managed projections, or digest changed. Keep the generated
backup even after the migrated Workspace passes `workspace check`.

## Upgrade from an archive

Extract the new archive into a separate directory. Run the new executable by its explicit path before changing the
installed command:

```console
./canisend-VERSION-TARGET/canisend version --json
./canisend-VERSION-TARGET/canisend doctor --json
```

Use `canisend.exe` on Windows. Replace the installed executable and its release notice bundle as one versioned unit.
Do not merge files from different releases.

Then inspect and check each Workspace with the new executable:

```console
canisend --workspace ./applications workspace status --json
canisend --workspace ./applications workspace check --json
```

Opening a Workspace applies only the reviewed, contiguous database migrations embedded in that
binary. It does not silently perform the separately approved semantic v2→v3 transition. Migration
history, exact Pack compatibility, and integrity checks fail closed. After all Workspaces pass,
regenerate any exported host pack or update project Skills from the desktop Agent setup journey.
Use a new export directory; do not overwrite a pack used by an active host session.

Alpha.7 installs the clean Agent v4 resources under `.agents/skills` for Codex or `.claude/skills`
for Claude Code. Their ownership manifests are `.agents/canisend-agent-v4.json` and
`.claude/canisend-agent-v4.json`. Install and update replace only unchanged manifest-owned files;
uninstall performs a complete digest preflight and refuses user-modified or unmanaged files.
Pre-v4 layouts are not upgraded in place: remove them explicitly, then perform a clean v4 install.

The Application Dossier, Content Catalog, contextual Agent guidance, and metadata/private search
indexes do not add a migration. They are rebuilt from current SQLite rows and immutable artifact
identities. A private search index is memory-only and discarded after the consented call. There is
therefore no read-model migration, backup payload, or rollback step; refreshing the application
rebuilds it under the currently running binary.

## Roll back safely

First determine whether the new binary opened any real workspace.

### The new binary did not open a workspace

Replace it with the retained verified previous executable and notice bundle, then run `version` and `doctor`. No
workspace action is needed because no migration could have run.

### The new binary opened a workspace

Do not assume that reinstalling the old executable makes the workspace compatible. If the new release appended a
migration, the older binary is designed to reject the future schema without mutation. There is no in-place downgrade
command and no supported deletion of migration records.

The refusal is emitted before the newer Workspace is configured or migrated and identifies both
schema versions. Its recovery action is: upgrade CanISend, or restore a verified pre-upgrade backup
to a new path. Do not repeatedly open or modify the newer Workspace with an incompatible binary.

Restore the pre-upgrade backup into a **new** destination, keep the upgraded workspace untouched for diagnosis, and
check the restored workspace with the old executable:

```console
./canisend-OLD workspace restore \
  ./backups/applications-before-VERSION \
  ./applications-restored-for-OLD --json
./canisend-OLD --workspace ./applications-restored-for-OLD \
  workspace check --json
```

Only redirect normal work to the restored path after the old binary accepts it. Never restore over either workspace.
If Beta and RC have the same schema, an older binary may still open the workspace, but the release qualification
matrix—not an assumption—must prove that exact version pair.

A backup automatically created by a newer binary's semantic v2→v3 migration is not a substitute
for this old-binary backup: it already contains the database schema used by the newer binary even
though v2 semantic authority remains. Keep both backups and label their creating binary/version.

User-edited Markdown or Typst projections are not migration authority. Preserve them separately before choosing
between an upgraded workspace and a restored pre-upgrade workspace; never copy an edited projection into SQLite or
the content-addressed blob store.

Do not attempt to preserve or copy GUI navigation memory, Catalog results, returned search
snippets, Dossier JSON, or Agent assistance as rollback authority. After selecting the restored
workspace, CanISend recreates these views from the accepted old workspace. Codex and Claude
transcripts remain owned by those hosts and follow their own retention and session procedures.

## Uninstall

Stop active commands and agent tasks, then remove the CanISend executable and its notice bundle from the installation
directory. Do not delete a workspace as part of binary uninstall.

Confirm that each retained workspace and backup directory still exists. They contain user-owned private data and are
not registered with an online CanISend account. Delete them only after an explicit data-retention decision and after
confirming that no rollback, audit, or application work still depends on them.

Removing an exported Codex/Claude host pack is separate from removing the binary. A host pack contains no private
workspace bodies by default, but remove it from host configuration before deleting its directory.

## Release-candidate acceptance

Before R11.3 can close, maintainers must perform this procedure on macOS arm64, macOS Intel, Linux GNU, Linux musl,
and Windows MSVC with the exact signed Beta and RC archives. Evidence must show:

- pre-upgrade `workspace check` plus a verified backup;
- RC opening and checking a representative Beta workspace;
- the expected old-binary behavior for unchanged or advanced schema;
- successful restore of the pre-upgrade backup into a new path with the Beta binary;
- binary and notice-bundle uninstall without workspace deletion;
- regenerated host packs and the documented quick-start succeeding after upgrade.

The same version pair must pass from clean release tags; locally rebuilt substitutes are not qualification evidence.
