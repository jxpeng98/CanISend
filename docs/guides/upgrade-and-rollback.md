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

## Confirm Workspace v4 authority before mutation

With the new binary, inspect each Workspace before running an Application or Agent write:

```console
canisend --workspace ./applications workspace status
```

Confirm `canisend.workspace/v4`, then list the independently Pack-bound Applications. One neutral
Workspace may contain both built-in Packs:

```console
canisend --workspace ./applications application list --json
```

The clean-v4 CLI does not migrate Workspace v2/v3. An unsupported Workspace fails before mutation with
clean-v4 initialization guidance; preserve it for use with its exact historical release.

## Upgrade from an archive

Extract the new archive into a separate directory. Run the new executable by its explicit path before changing the
installed command:

```console
./canisend-VERSION-TARGET/canisend version --json
./canisend-VERSION-TARGET/canisend doctor --json
```

Use `canisend.exe` on Windows. Replace the installed executable and its release notice bundle as one versioned unit.
Do not merge files from different releases.

From Beta.7, upgrade each Workspace with one command, then check its integrity:

```console
canisend --workspace ./applications workspace upgrade
canisend --workspace ./applications workspace check
```

Opening a supported Workspace applies the contiguous database migrations embedded in the
binary. `workspace upgrade` also refreshes all existing project Skills (Codex, Claude and
generic), restores missing managed files and removes unchanged obsolete managed files.
It preflights all selected installations for customizations before updating any Skills.
Repeated upgrades are safe. The command does not import unsupported Workspace v2/v3 state.
Templates are bundled in the new CLI; existing Applications retain their bound Pack and history.

Use `workspace upgrade --host codex` (or `claude`) to select or install one Host's project
Skills. Without `--host`, a Workspace with no installed Skills stays that way. These commands
return readable text in a terminal and JSON when piped or passed `--json`. They do not prompt for approval.
If the executable moved, run `host setup --host HOST` for the updated MCP registration command.
Reconnect the Host after upgrading; installing Skills does not change its MCP configuration.

User-wide Skills remain separate: run `host setup --host HOST --scope global` to update them.
Before Beta.7, use `host setup --host HOST` for project updates as well. `host status` compares
manifests and actual bytes with the running binary and provides conflict guidance:

| Status | Action |
|---|---|
| `ready` | Resources match; reconnect after a binary/Skills update and rediscover tools |
| `update-available` | Pause active tasks, then run setup with the new binary and same scope |
| `incomplete` | Run setup to restore missing managed files and complete the update |
| `user-modified` / `unmanaged` | Preserve custom files and review conflicts before setup |

Setup checks all owned files before writing. It replaces files individually and writes the
manifest last; this is not a transaction over the entire Skills directory. A stopped update can
leave old, current or missing files. With the same new binary, inspect status and rerun setup;
matching old-manifest or current bundled bytes can be reconciled, while custom bytes are refused.
Malformed ownership manifests or unsupported resource formats need diagnosis, not forced edits.
Keep custom guidance outside managed Skill files, and never change manifest hashes to bypass a
conflict. Skills installation is not a workspace migration or a backup of customized Skills.

After setup, restart/reconnect the Host, rediscover tool schemas and discard every old preview.
Re-read Application revision and Pack identity before continuing. Use the new registration command
if the executable moved. `ready` verifies resources, not provider connectivity or human approval.
Replacing the executable alone does not replace installed Skills; using an older executable is
not a guaranteed compatible Skills or Workspace rollback. Retain the matched old bundle and
pre-upgrade backup; test rollback against a separate restored Workspace first.

The CLI installs clean Agent v4 resources under `.agents/skills` for Codex or `.claude/skills`
for Claude Code. Their ownership manifests are `.agents/canisend-agent-v4.json` and
`.claude/canisend-agent-v4.json`. Install and update replace only unchanged manifest-owned files;
uninstall performs a complete digest preflight and refuses user-modified or unmanaged files.
Pre-v4 layouts are not upgraded in place: remove them explicitly, then perform a clean v4 install.

For missing managed Skill files, run `workspace upgrade` (or the same scoped `host setup` command) to restore the embedded version,
then check `host status`. User-edited or unmanaged files require review; setup and removal refuse
to overwrite or delete conflicting files. A broken executable is repaired by reinstalling a verified
archive, not by changing Workspace data. Use `workspace repair` only for its documented managed
projections; it does not repair the installed executable or replace user-edited content.

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

Before removing the executable, optionally remove its owned project Skills with
`canisend --workspace ./applications host remove --host codex --json` (or `--host claude`). This
preserves the Workspace and refuses modified or unmanaged Skill files. Remove the corresponding
MCP registration separately in the Host; `host remove` does not edit that configuration.

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
