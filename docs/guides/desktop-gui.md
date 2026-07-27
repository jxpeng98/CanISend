# CanISend desktop GUI preview

The native desktop interface is now available as a macOS-first development preview. It operates the
same Rust v2 workspace as the `canisend` CLI and does not use CLI process output for workflow
operations. The Command line page has one bounded exception: it invokes only the exact installed
`canisend` path with fixed version arguments and a short timeout. It never invokes a shell or
inspects Python, package managers, or their environments.

The GUI is not part of the historical qualified `0.7` release archives. It is the first
implementation slice for the unified `1.0` product line, beginning with `1.0.0-alpha.1`.

## Build and launch on Apple Silicon

From the repository root:

```console
cargo build -p canisend-cli -p canisend-gui --release --locked
./target/release/canisend-gui
```

Build both executables so the GUI can discover the sibling native CLI. The current release build is
a native arm64 Mach-O executable. It opens one window with Overview, Jobs, Discovery, Profile,
Agent integration, Workspaces, Command line, and Diagnostics navigation.

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
7. Open **Profile** to import a local Markdown, text, or JSON profile source. Choose its sensitivity
   before confirming a private read.

Every file, URL, PDF, workspace, job, and workflow mutation uses the same bounded Rust services and
authoritative SQLite/blob store as the CLI.

## Discover and promote jobs

Open **Discovery** to review local lead batches or refresh one supported public source:

1. Choose CSV or JSON for a user-reviewed local batch. Host-agent discovery uses the same bounded
   JSON contract and must be selected explicitly.
2. Confirm private local reading before previewing a file. CanISend shows accepted and rejected
   rows plus diagnostics before an explicit commit; changing the file does not change the reviewed
   in-memory report.
3. For RSS/Atom, jobs.ac.uk, Greenhouse, or Lever, provide the public endpoint and separately
   confirm network access. Refresh remains user-invoked and is previewed before commit.
4. Inspect active or historical leads, freshness, source metadata, and bounded possible-duplicate
   suggestions. Suggestions never merge records automatically.
5. Promote one reviewed lead explicitly. The resulting job keeps the discovery provenance and
   provides the safe next action for importing its advert.

## Prepare and complete Agent tasks

The selected job's Workflow view includes an **Agent task** panel. It exposes only operations ready
under the authoritative stage graph:

1. Choose the bounded operation and either Host agent or Configured provider when the current stage
   permits that mode.
2. Review the task ID, lease expiry, exact job revision, declared input artifacts, output kind,
   candidate schema, and required consent scopes.
3. Choose a new or empty directory and confirm private reading. Provider execution requires a
   separate send consent. The export contains only the descriptor's declared revisions and a
   digest-bound manifest.
4. Load one regular bounded completion JSON file. CanISend validates the candidate without mutation
   and shows its resulting artifact before a separate commit.
5. Cancel a prepared task when necessary. Expired or stale tasks provide a prepare-again recovery
   action; a replacement receives a new lease and rechecks every revision.

Task state is stored in the workspace, so the GUI, CLI, Codex, Claude, or another Agent v2 host sees
the same prepared, committed, cancelled, or stale state after reopening.

## Inspect Agent integration and export resources

Open **Agent integration** to inspect the compiled Agent v2 protocol without exposing source
bodies. The page shows product and format versions, capability and stage registries, discovery
adapters, and either a workspace summary or one optional active-job summary. Blockers and bounded
next actions are plain text with copy controls; copying never executes a command.

Choose Codex, Claude, or Generic, then select a destination. Export is enabled only after the GUI
previews the destination as new or empty; the application layer verifies it again before writing.
On success the page shows the manifest path, resource count, and exact exported files. The GUI
never launches the exported host or exposes a general shell.

## Recover and repair a workspace

Open **Workspaces** and choose **Restore backup** to recover a verified CanISend backup. Select the
backup directory, a separate new or empty destination, and a local display name. The confirmation
shows both paths before any work begins. CanISend verifies the backup and restores into the new
destination first; only after success does the GUI register and select it. The source backup and
previous active workspace are not changed.

Choose **Repair active** to rebuild only missing or repair-required managed projections from
verified workspace records. The confirmation identifies the active path. User-edited projections
are preserved, and a second repair that has nothing to rebuild succeeds with zero changes.

If an integrity check reports a missing or invalid authoritative blob, do not use projection
repair. Stop writing and restore from a verified backup instead.

## Control workflow stages

After a workflow starts, each stage card shows its authoritative state, execution mode, expected or
current output, blockers, and next actions:

- a ready stage offers **Begin stage** with exactly the modes in its compiled descriptor;
- a running or awaiting-user stage offers **Complete stage** using a current compatible artifact
  UUIDv7;
- a complete or stale stage offers **Rerun stage** when allowed; and
- blocked stages explain their blockers without offering a mutation.

CanISend resolves the artifact reference from the workspace and validates it before completing a
stage. The GUI never accepts a caller-supplied kind, revision, or digest. Rerun first displays every
affected descendant stage and current output, then requires explicit confirmation. Each dialog
also shows a copyable equivalent CLI command as text; it does not execute that command.

Generic stage completion still requires an existing compatible artifact from the CLI or Agent v2.
Evidence, criteria, and plan decisions use their dedicated structured GUI controls rather than an
artifact-ID field. Match creation remains an Agent v2 task operation; the GUI displays the current
revision-bound match without synthesizing one.

## Review evidence and make an application decision

The Profile and selected-job views expose one revision-bound decision path:

1. On **Profile**, select a job and load proposed, editable, or currently confirmed evidence.
2. Review kind, summary, quoted source, sensitivity, and confirmation/exclusion state. Source
   identity and byte spans remain read-only. Confirming a revision previews its downstream effect.
3. On the selected job, load proposed, editable, or confirmed criteria. Review requirement,
   importance, kind, confidence, and source identity before confirming.
4. Inspect the current criterion-to-evidence match. Strength, rationale, evidence identities, gaps,
   and prohibited claims are read-only. If no match exists, the page identifies the workflow/task
   action needed.
5. Load the application-plan candidate, choose Apply, Hold, or Withdraw, and review positioning,
   priorities, risks, planned documents, and derived blockers. Match identity and derived blockers
   remain read-only. Confirming the plan requires a separate explicit action.
6. Close and reopen the same workspace to load the confirmed evidence, criteria, match, and plan
   revisions from the shared store.

Every mutation shows an in-progress state and an explicit success or failure result. Controls use
text labels in addition to status color, support keyboard focus, and retain English and Simplified
Chinese labels.

## Accessibility and appearance

The navigation rail exposes native AccessKit names and roles, and every dialog moves initial focus
to its first required control. Use Tab and Shift-Tab to traverse the workspace switcher,
navigation, appearance settings, and page actions. Custom navigation and job-row controls draw an
amber focus ring; at high text scale, focused navigation controls scroll into view.

The **Accessibility & appearance** section provides:

- an immediately applied, persistent **English** or **简体中文** language choice;
- system-initialized light or dark appearance;
- normal or compact density with a minimum interactive height;
- **Reduce motion**, which disables widget and scroll animation; and
- 100%, 125%, 150%, or 200% text size.

Language, appearance, density, reduced-motion, window, and text-scale state persist across normal restarts.
The standard Command-plus, Command-minus, and Command-0 zoom shortcuts also work. Diagnostics
reports the active text size, window-system display scale, and reduced-motion state without
including job, profile, draft, or provider bodies.

After staging an app, developers with macOS Accessibility automation permission can run the bounded
smoke against an isolated HOME:

```console
./scripts/smoke_macos_gui_accessibility.sh /path/to/CanISend.app
```

The smoke independently verifies the app manifest and ad-hoc signature, then checks English and
Simplified Chinese native control names, AccessKit landmarks/headings, exact Tab order, 200% focus
visibility, reduced motion, and Command-0 reset.
Real IME composition and native directory/file selection remain native release-matrix checks
because they change global input-source or Finder UI state and do not belong in the fast edit loop.

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

The script copies both exact executables, licenses, and privacy/GUI guidance before applying free
ad-hoc integrity signatures to the nested executables and outer app. It then writes
`CanISend.app.manifest.json` beside the app with SHA-256 digests of the final signed GUI, bundled
CLI, `Info.plist`, and internal bundle metadata. The integrity manifest must stay outside the app:
signing the outer bundle changes its main executable, so embedding that final executable digest
would create a self-reference and invalidate either the digest or the signature. The script does
not provide Developer ID identity or notarization.

Create the frozen Apple Silicon Alpha ZIP directly from the two release binaries with:

```console
./scripts/package_macos_gui_release.sh \
  ./target/release/canisend-gui \
  ./target/release/canisend \
  /path/to/release-assets
```

The command produces `CanISend-VERSION-aarch64-apple-darwin.zip` with exactly
`CanISend.app` and `CanISend.app.manifest.json` at the top level. It strips resource forks and
extended attributes from the ZIP so no `__MACOSX` or AppleDouble entries can expand the frozen
contract; code signatures remain preserved in Mach-O and regular bundle files.

Verify a staged bundle independently before launch:

```console
./scripts/verify_macos_gui_app.sh \
  /path/to/CanISend.app \
  /path/to/CanISend.app.manifest.json
```

Release qualification verifies the archive after a fresh bounded extraction:

```console
./scripts/smoke_macos_gui_release_archive.sh \
  /path/to/CanISend-VERSION-aarch64-apple-darwin.zip \
  /new/path/to/smoke-output
```

That smoke rejects unsafe paths, symbolic links, an unexpected top level, more than 4096 entries,
or more than 256 MiB uncompressed; then it checks the final signatures and companion hashes, runs
the bundled CLI doctor and synthetic workflows, and launches the packaged GUI.

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

Codex and Claude continue to use Agent v2 rather than GUI automation. The GUI and CLI share the
same discovery records, prepared tasks, body-free Agent context, profile evidence, job criteria,
current matches, and application plans through the application facade and authoritative workspace.
Document, review, package, and render screens remain scheduled for later Stage 4 slices.

## Workspace registry and retention

On macOS, launcher metadata is stored at:

```text
~/Library/Application Support/CanISend/workspaces.json
```

It contains only the registry format, aliases, canonical paths, pinned/default state, and
last-opened timestamps. Choosing **Remove from list** changes only this registry. It never deletes
the workspace, SQLite database, blobs, projections, exports, or backups.

## Implemented preview coverage

- Persistent native light/dark appearance, compact density, 100–200% text scaling, reduced motion,
  AccessKit landmarks/headings/live regions, visible keyboard focus, and background-operation
  feedback.
- Native CanISend version detection, one-click user-level install/migration/update/uninstall,
  rollback restoration, PATH diagnostics, online release checks, and copyable terminal checks.
- Workspace create/register/switch/status/check/backup/restore/repair and registry removal.
- Job search, active/archive visibility, create, detail, archive, and source metadata.
- Local Markdown/text/JSON/text-PDF and supplied public URL intake.
- Profile source catalog/import plus structured evidence review, correction, exclusion, and
  confirmation.
- Structured criteria review and confirmation, read-only current match inspection, and explicit
  application-plan decision and confirmation.
- Discovery source/lead inspection, reviewed local imports, consent-bound public refresh,
  duplicate suggestions, and explicit lead promotion.
- Revision-bound Agent task preparation, scoped input export, completion preview/commit, cancel,
  stale detection, and prepare-again recovery.
- Body-free Agent v2 capability/context inspection and verified Codex, Claude, or generic
  resource-pack export.
- Workflow start, body-free stage/blocker timeline, descriptor-bound begin/complete, and
  preview-confirmed rerun.
- Body-free product diagnostics and embedded renderer/resource self-check.

Not yet implemented in the GUI:

- document, review, package, render, and application-package export screens;
- Developer ID/notarized release signing, Intel native qualification, or non-macOS GUI packages.

These remaining operations stay available through the CLI or Agent v2 while GUI coverage expands.
The desktop application still never submits an application.
