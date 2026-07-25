# CanISend desktop GUI preview

The native desktop interface is now available as a macOS-first development preview. It operates the
same Rust v2 workspace as the `canisend` CLI and does not use CLI process output for workflow
operations. The Command line page has one bounded exception: it invokes only the exact installed
`canisend` path with fixed version arguments and a short timeout. It never invokes a shell or
inspects Python, package managers, or their environments.

The GUI is not part of the qualified `0.7` release archives. It is the first implementation slice
for the future `0.8` desktop line.

## Build and launch on Apple Silicon

From the repository root:

```console
cargo build -p canisend-cli -p canisend-gui --release --locked
./target/release/canisend-gui
```

Build both executables so the GUI can discover the sibling native CLI. The current release build is
a native arm64 Mach-O executable. It opens one window with Overview, Jobs, Workspaces, Command line,
and Diagnostics navigation.

## First run

1. Open **Workspaces**.
2. Choose **Create workspace** and select a new or empty local directory, or choose
   **Register existing** and select a Rust v2 workspace containing `canisend.toml`.
3. Give the workspace a local display name. The name and canonical path are stored in the GUI
   registry; private workspace bodies are not copied into the registry.
4. Open **Jobs**, choose **Add job**, and enter the title and institution.
5. Open the job, choose **Import source**, then select:
   - a local Markdown, text, JSON, or text-based PDF after confirming private local read access; or
   - a user-supplied public HTTP(S) URL after confirming the network fetch.
6. Choose **Start workflow**. The job view shows the ten durable stages, current state, and blockers.

Every file, URL, PDF, workspace, job, and workflow mutation uses the same bounded Rust services and
authoritative SQLite/blob store as the CLI.

## Install the CLI from the GUI

Open **Command line** to inspect the exact CLI bundled with the GUI, the user-level destination
`~/.local/bin/canisend`, whether that directory is on `PATH`, and which `canisend` command the
GUI process environment resolves first. A Finder-launched app may see a different `PATH` from an
interactive login shell, so the page also provides terminal verification commands.

- **Install CLI** copies the known regular, non-symlink bundled executable, applies executable
  permissions, records its SHA-256 digest, and commits it atomically.
- **Update CLI** replaces only an unchanged GUI-managed binary.
- If an older or version-unaware CanISend installation occupies the destination, one click migrates
  or upgrades it. The click is the explicit install action; the previous file or symlink moves to a
  private rollback backup before installation.
- If the installed CanISend version is newer than the bundled version, installation is disabled so
  the GUI cannot downgrade it.
- **Uninstall managed CLI** verifies the installed digest first. It restores a preserved previous
  installation when present and never changes workspace data.
- If the managed binary or install record was changed outside CanISend, overwrite and uninstall
  fail closed so the user-owned change is preserved.

The GUI does not edit `.zprofile`, `.zshrc`, or another shell configuration file. If
`~/.local/bin` is not visible, the page provides a copyable PATH line; apply it intentionally and
open a new terminal. It also warns when a different package-manager shim takes precedence over the
new binary.

**Check for updates** contacts only the public CanISend GitHub Releases API after the user presses
the button. It compares the desktop/bundled version with the latest compatible Stable or Preview
release, displays the result, and provides a copyable release link. It does not run automatically,
send private application data, download an update, or execute an installer.

For a packaged macOS application, the version-matched source belongs at:

```text
CanISend.app/Contents/Resources/bin/canisend
```

Developers can stage the current ad-hoc-signed preview bundle with:

```console
./scripts/stage_macos_gui_app.sh \
  ./target/release/canisend-gui \
  ./target/release/canisend \
  /path/to/CanISend.app
```

The script copies both exact executables, licenses, privacy/GUI guidance, and an executable
SHA-256 manifest before applying free ad-hoc integrity signatures to the nested executables and
outer app. It does not provide Developer ID identity or notarization.

The GUI does not download a binary, run `curl | sh`, invoke a package manager, or expose a general
command textbox.

## Sharing a workspace with the CLI and agents

The GUI registry is a launcher convenience, not a second workspace. The CLI can inspect the same
directory at any time when no GUI mutation is running:

```console
canisend --workspace /path/to/applications workspace status
canisend --workspace /path/to/applications workflow status --job JOB_ID
canisend --workspace /path/to/applications agent context --job JOB_ID --json
```

Codex and Claude continue to use Agent v2 rather than GUI automation. The current GUI shows intake
and workflow state; agent-task preparation/completion and later evidence/document/review/export
screens remain scheduled for the next R12 slices.

## Workspace registry and retention

On macOS, launcher metadata is stored at:

```text
~/Library/Application Support/CanISend/workspaces.json
```

It contains only the registry format, aliases, canonical paths, pinned/default state, and
last-opened timestamps. Choosing **Remove from list** changes only this registry. It never deletes
the workspace, SQLite database, blobs, projections, exports, or backups.

## Implemented preview coverage

- Native light/dark app shell, compact density, keyboard-native widgets, visible status, and
  background-operation feedback.
- Native CanISend version detection, one-click user-level install/migration/update/uninstall,
  rollback restoration, PATH diagnostics, online release checks, and copyable terminal checks.
- Workspace create/register/switch/status/check/backup and registry removal.
- Job search, active/archive visibility, create, detail, archive, and source metadata.
- Local Markdown/text/JSON/text-PDF and supplied public URL intake.
- Workflow start and body-free stage/blocker timeline.
- Body-free product diagnostics and embedded renderer/resource self-check.

Not yet implemented in the GUI:

- workspace restore and projection repair;
- workflow begin/complete/rerun controls;
- criteria, evidence, match, and plan confirmation forms;
- discovery, agent-task, document, review, package, render, and export screens;
- a signed `.app` bundle, Intel qualification, or non-macOS GUI packages.

All of these operations remain available through the CLI while the GUI coverage expands. The
desktop application still never submits an application.
