# CanISend macOS-first GUI execution plan

**Status:** Active

**Started:** 2026-07-24

**Latest implementation review:**
[2026-07-25 macOS GUI implementation review](../../notes/rust-native/2026-07-25-gui-implementation-review.md)

**Product decision:** Begin R12 engineering on macOS before the deferred Windows and Linux GUI
qualification work. The historical `0.7` CLI and Agent v2 evidence remains unchanged; the desktop
surface joins the unified `1.0` product line beginning with `1.0.0-alpha.1`.

## Outcome

Deliver a native `canisend-gui` application that operates the same local workspace as the CLI
without invoking a shell, duplicating SQLite rules, or requiring Python, Node.js, Java, a browser
server, or an external Typst runtime.

The first usable vertical slice must let a macOS user:

1. register, create, switch, inspect, check, and back up local workspaces;
2. list, search, create, archive, and inspect jobs;
3. import a supplied URL or a local Markdown, text, JSON, or text-based PDF source;
4. start a workflow and inspect its stages, blockers, freshness, and next actions;
5. hand off semantic work to Codex or Claude through the existing Agent v2 boundary; and
6. inspect only the installed CanISend version, migrate/upgrade it to the version-matched native
   CLI, and manually check for a newer release without inspecting runtime environments; and
7. close and reopen the application without losing registry or authoritative workspace state.

## Current baseline

- The Rust CLI, store, bounded I/O adapters, embedded resources, Agent v2 contracts, and renderer
  are implemented and locally qualified.
- `canisend-app` and `canisend-gui` provide the first typed macOS vertical slice.
- Workspace registry, job intake, supplied URL/file/PDF import, workflow status, diagnostics,
  terminal CLI lifecycle, and manual update checks are implemented.
- A version-matched macOS `.app` can be staged with hashes, notices, and ad-hoc integrity signing.
- Focused GUI tests, contrast regression, bounded-registry fixtures, strict Clippy, preferred/minimum
  window visual review, and named macOS accessibility control inspection pass.
- Restore/repair, full workflow controls, evidence/discovery/agent/document/review/package/render/
  export screens, disposable-user lifecycle, full native accessibility, and Intel qualification
  remain open.
- The source baseline remains `0.7.0-rc.2` until the atomic 1.0 release-line activation; GUI code
  must not be presented as part of the historical qualified `0.7` release.

## Architecture

```text
canisend-cli                         canisend-gui
clap / stdout / exit policy         egui views / native dialogs / app state
             \                       /
              \ typed actions       /
               canisend-app
       receipts / read models / consent boundary
                        |
       contracts · core · store · io · resources
```

`canisend-app` is the only new orchestration boundary. It opens workspaces and composes existing
services. It does not contain terminal formatting or widgets. `canisend-gui` never executes CLI
strings and never writes SQLite, blobs, or managed projections directly.

The bounded terminal installer is the only binary-distribution exception: it hashes and copies the
known app-bundled CLI to a user-owned destination. A fixed, timeout-bounded version handshake may
execute only the exact installed `canisend` path; no product commands or shell text are accepted,
and shell profiles are not edited. A separate manual update check reads only the allowlisted public
GitHub Releases endpoint.

The GUI registry contains launcher metadata only: path, alias, pinned/default state, and
last-opened time. Removing a registry item never deletes its workspace.

## Visual and interaction contract

- Flat, content-first desktop UI with clear borders and no decorative glass.
- System fonts; 4/8 px spacing; minimum window `800 × 600`, preferred `1100 × 720`.
- Deep teal navigation and primary controls, amber only for the next recommended action, red only
  for destructive or error states.
- Persistent workspace switcher, left navigation, page header, status banner, and activity area.
- Keyboard-reachable controls in visual order, visible focus, named controls, light/dark themes,
  and no color-only status communication.
- Operations longer than 300 ms run on a worker and show running/success/failure state. Mutating
  actions are serialized for the active workspace.

## Delivery slices

### Slice A — Foundation

- Add the GUI architecture ADR and toolkit dependency record.
- Add `canisend-app` with typed receipts and body-minimized read models.
- Implement product summary, workspace init/status/check/backup, job list/create/archive/import,
  and workflow start/status actions.
- Add a checked-in CLI-to-GUI parity manifest.

**Gate:** Facade tests prove actions commit through existing services and return no private bodies
from routine read models.

### Slice B — Usable macOS application

- Add `canisend-gui` using pinned `eframe`/`egui` and native `rfd` dialogs.
- Implement durable workspace registry and first-run workspace chooser.
- Implement Overview, Jobs, Job detail, Workflow, Workspaces, and Diagnostics screens.
- Implement local file and URL import, explicit network/private-read language, progress, and safe
  recovery errors.
- Implement theme and density controls and background action dispatch.
- Implement Command line installed/bundled CanISend version status, one-click
  migration/update/uninstall, preserved rollback, downgrade refusal, PATH diagnostics, a manual
  online release check, and copyable verification commands.

**Gate:** A locally built app creates a workspace, imports the bounded fixture, starts a workflow,
shows current stages, closes, and reopens the registered workspace.

### Slice C — Workflow completion surfaces

- Profile source/evidence screens.
- Discovery import, refresh, duplicate review, and lead promotion.
- Agent task preparation, private-input consent, host-pack export, and completion status.
- Criteria, match, plan, documents, review dispositions, package readiness, projections, render,
  and private export screens.
- Complete CLI-to-GUI mutation coverage.

**Gate:** The documented synthetic end-to-end workflow completes without a terminal.

### Slice D — macOS Alpha packaging

- Build a native `.app` bundle containing the GUI and version-locked CLI.
- Add icon, bundle metadata, licenses, notices, checksums, SBOM, provenance, and ad-hoc signing.
- Test Apple Silicon launch, keyboard operation, VoiceOver semantics, high DPI, IME, file dialogs,
  upgrade, backup/restore, uninstall, and workspace retention.
- Add Intel compilation and native qualification after the Apple Silicon path is accepted.

**Gate:** Publish `1.0.0-alpha.1` only from an exact packaged-app smoke and machine-checkable
qualification record.

### Slice E — Deferred platform work

- Windows x64 GUI packaging, Authenticode evidence, keyboard/screen-reader qualification.
- Linux x64 glibc GUI packaging and desktop integration.
- Keep Linux musl CLI-only until its window-system boundary is explicitly qualified.

## Test tiers

1. **Fast edit loop:** affected crate tests, formatter, and relevant Clippy target.
2. **Source gate:** facade parity fixtures, existing CLI binary contracts, and release checks that
   are applicable to the experimental GUI line.
3. **macOS native gate:** release GUI build, launch smoke, fixture workflow, close/reopen, and
   application bundle inspection.
4. **Extended GUI release gate:** assistive technology, both macOS architectures, signing,
   provenance, upgrade/rollback, and uninstall retention.

## Acceptance and safety invariants

- No GUI action spawns a shell or accepts arbitrary commands.
- The installer only handles the exact bundled `canisend` executable and exact `canisend`
  destination; it never runs an installation script.
- An unmanaged installation is preserved unless the user explicitly chooses replacement, and a
  changed managed binary is never overwritten or removed.
- One workspace remains authoritative across CLI, GUI, Codex, and Claude.
- File, URL, PDF, and agent inputs pass through the existing bounded adapters and validators.
- Routine UI status and activity history contain no job, profile, draft, or provider bodies.
- Private reads, network fetches, provider sends, and exports remain explicit.
- Registry removal and application uninstall never delete workspace data.
- The GUI prepares and exports application material but never submits an application.

## Immediate execution order

1. Record toolkit/architecture decision and parity inventory.
2. Implement and test the shared facade vertical slice.
3. Implement the macOS app shell, registry, navigation, and worker.
4. Connect workspace, job intake, and workflow screens to typed actions.
5. Build and launch the GUI on macOS arm64.
6. Record verified completion and carry remaining workflow screens into Slice C.
