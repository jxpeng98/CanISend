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

## Upgrade from an archive

Extract the new archive into a separate directory. Run the new executable by its explicit path before changing the
installed command:

```console
./canisend-VERSION-TARGET/canisend version --json
./canisend-VERSION-TARGET/canisend doctor --json
```

Use `canisend.exe` on Windows. Replace the installed executable and its release notice bundle as one versioned unit.
Do not merge files from different releases.

Then open and check each workspace with the new executable:

```console
canisend --workspace ./applications workspace status --json
canisend --workspace ./applications workspace check --json
```

Opening a workspace applies only the reviewed, contiguous Rust-era migrations embedded in that binary. Migration
history and integrity checks fail closed. After all workspaces pass, regenerate any exported host pack so its
manifest, schemas, prompts, examples, and product version come from the installed binary:

```console
canisend agent assets export --host codex \
  --destination ./canisend-codex-pack-VERSION --json
```

Export to a new directory; do not overwrite a pack that an active host session may still be using.

## Roll back safely

First determine whether the new binary opened any real workspace.

### The new binary did not open a workspace

Replace it with the retained verified previous executable and notice bundle, then run `version` and `doctor`. No
workspace action is needed because no migration could have run.

### The new binary opened a workspace

Do not assume that reinstalling the old executable makes the workspace compatible. If the new release appended a
migration, the older binary is designed to reject the future schema without mutation. There is no in-place downgrade
command and no supported deletion of migration records.

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

User-edited Markdown or Typst projections are not migration authority. Preserve them separately before choosing
between an upgraded workspace and a restored pre-upgrade workspace; never copy an edited projection into SQLite or
the content-addressed blob store.

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
