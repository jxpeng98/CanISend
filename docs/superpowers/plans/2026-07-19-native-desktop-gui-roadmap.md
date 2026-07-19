# CanISend Native Desktop GUI Roadmap

**Status:** Approved product direction; implementation is gated behind `v0.7.0` Stable

**Target release:** `0.8.x`, beginning with a native GUI Alpha

**Related plans:**

- [Rust-native greenfield roadmap](2026-07-17-rust-native-greenfield-roadmap.md)
- [Post-0.7 roadmap](2026-07-18-post-0.7-roadmap.md)

## 1. Decision Summary

CanISend will add a small native desktop application over the existing Rust product. The GUI is a second product
surface, not a second implementation of the workflow.

The desktop application must:

1. let a user create, register, open, check, back up, restore, and switch local CanISend workspaces;
2. expose every ordinary state-changing CLI operation through a typed screen, form, or bounded action;
3. support supplied URLs, local Markdown/text/JSON/CSV files, and text-based PDFs without requiring an API;
4. show the application workflow, blockers, next actions, agent tasks, documents, reviews, package readiness, and
   rendered outputs without requiring the user to interpret JSON;
5. keep the CLI and Agent v2 protocol available for Codex, Claude, scripts, and advanced users;
6. reuse the same Rust application services, validation, consent, storage, and error taxonomy as the CLI;
7. remain local-first and require no Python, Node.js, Java, browser server, or external Typst runtime at use time; and
8. prepare and export application materials but never submit an application.

The `0.7` Release Candidate and Stable gates remain feature-frozen. GUI architecture notes and an isolated toolkit
spike may be prepared before Stable, but product code, release contracts, and package contents change only after
R11.4 closes.

## 2. Product and Terminology Boundary

### 2.1 Workspace, not Git repository

The GUI calls a CanISend data directory a **workspace**. A workspace is the existing authoritative unit containing
`canisend.toml`, SQLite state, immutable blobs, and managed projections. The user may colloquially call it a
repository, but the GUI must not imply that Git is required or that private application data should be pushed to a
public source repository.

The desktop application maintains a small **workspace registry** in the operating system's per-user application
configuration directory. The registry contains only launcher metadata:

- canonical local path;
- user-visible alias;
- pinned/default state;
- last-opened time;
- last known format and body-free health state; and
- whether the user requested read-only inspection when a writer is active.

It does not contain job bodies, CVs, evidence, drafts, provider credentials, tokens, or authoritative workflow state.
Removing a workspace from the registry never deletes its directory. Deleting workspace data is outside the initial
GUI and requires a separate future design.

### 2.2 Two native entry points, one product

The release unit contains two native entry points built from the same source and version:

- `canisend`: the existing CLI and Agent v2 interface; and
- `CanISend` / `canisend-gui`: the desktop application, with the platform-appropriate executable or application
  bundle name.

The GUI does not spawn the CLI for normal product behavior. Both adapters call a shared typed application facade.
The CLI remains fully usable when the GUI is not installed, and agent hosts do not need to automate GUI controls.

### 2.3 Initial non-goals

- Cloud sync, user accounts, and shared remote workspaces.
- Application portal automation or automatic submission.
- An embedded unrestricted shell or arbitrary command textbox.
- Editing SQLite, blobs, task leases, or internal records directly.
- Replacing the Agent v2 JSON protocol with GUI automation.
- Mobile or browser delivery.
- Scanned-PDF OCR unless separately promoted by the post-0.7 evidence gate.
- A visual editor that attempts to replace a full word processor.

## 3. Primary Users and Outcomes

### 3.1 Applicant using the GUI only

The user can:

1. open or create a workspace;
2. add a job from a URL, PDF, or supported local file;
3. inspect normalized source metadata and start the workflow;
4. see which stage is ready, running, awaiting the user, blocked, stale, or complete;
5. complete user decisions and hand bounded tasks to a configured provider or an external agent host;
6. review generated documents and findings;
7. check readiness, render PDFs, and export a complete package; and
8. back up the workspace before an upgrade or significant revision.

### 3.2 Applicant working with Codex or Claude

The GUI shows prepared agent tasks, exact consent, declared inputs, and completion state. It can reveal or copy the
bounded task directory and the exact reproducible CLI command, but the agent continues to use Agent v2. When an
agent completes a task, the GUI refreshes authoritative state and shows the validated result or a structured recovery
action.

### 3.3 Maintainer or advanced user

The GUI exposes body-free diagnostics, versions, capability availability, schema/resource metadata, workspace
integrity, and exact action receipts. It never turns diagnostics into a general shell.

## 4. Information Architecture

The application uses one desktop window with a persistent workspace switcher, a compact left navigation rail, a
page header, and a body-free activity drawer for long-running operations.

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ CanISend     [Workspace: My Applications ▾]          Health ✓   Activity 2 │
├──────────────┬───────────────────────────────────────────────────────────────┤
│ Overview     │ Page title                         Primary bounded action     │
│ Jobs         ├───────────────────────────────────────────────────────────────┤
│ Discovery    │                                                               │
│ Profile      │ Context, forms, workflow, documents, or settings             │
│ Workspaces   │                                                               │
│ Settings     │                                                               │
├──────────────┴───────────────────────────────────────────────────────────────┤
│ Body-free status · current operation · recovery action · command receipt    │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Overview

- current workspace health and backup status;
- active jobs grouped by next action;
- recent agent tasks and long-running operations;
- explicit warnings for stale artifacts, blocked stages, or a workspace opened by another writer; and
- primary actions: **Add job**, **Continue next action**, and **Back up workspace**.

### 4.2 Jobs

The list supports search, active/archived filters, institution, deadline, workflow state, and last update. Hidden
filters are forbidden; every active filter remains visible and removable.

Each job opens a focused work area with five tabs:

1. **Summary** — title, institution, sources, current stage, blockers, next action;
2. **Sources** — URL/PDF/file metadata, import receipts, and explicit re-import actions;
3. **Workflow** — the durable stage graph and the action appropriate to each stage;
4. **Documents** — current structured documents, revisions, and export/render state; and
5. **Review & Export** — findings, user dispositions, package readiness, PDFs, and exports.

### 4.3 Add-job flow

The add-job flow is a short, recoverable wizard:

1. enter the title and institution;
2. choose supplied URL, local file/PDF, or create without a source;
3. preview source type, destination policy result, size, and normalized metadata;
4. explicitly approve any network fetch or private import boundary; and
5. create the job, import the source, and offer **Start workflow**.

The URL and file paths remain first-class choices. No API configuration is required. If a source is rejected, the
error appears beside the field and includes a safe recovery action such as selecting another file, reviewing redirect
details, or importing saved content locally.

### 4.4 Discovery

Discovery shows configured public sources, adapter limits, refresh status, normalized leads, possible duplicates,
and promotion into direct job intake. Refresh is always user-invoked. CSV, JSON, RSS/Atom, URL, and host-agent import
remain visible alternatives to API-backed sources.

### 4.5 Profile

Profile separates source documents from the confirmed evidence catalog. Sensitive imports show their classification
and consent boundary before reading. Evidence proposals, exclusions, corrections, and confirmation use structured
forms; the GUI does not silently convert a proposal into confirmed evidence.

### 4.6 Workspaces

The workspace manager lets the user:

- create a workspace in a chosen empty directory;
- register an existing workspace;
- pin, rename, reorder, or select a default registry entry;
- open the directory in the operating system file manager;
- inspect status and run the integrity check;
- make a verified backup;
- restore a verified backup into a different empty directory;
- repair deterministic projections after a successful integrity assessment; and
- remove the entry from the GUI registry without deleting user data.

Destructive or overwrite-like operations never use the primary action style and always state exactly which path is
affected. Restore-to-new-path remains mandatory.

### 4.7 Settings and diagnostics

Settings contain only application presentation and explicit integration preferences:

- theme, text scale, reduced motion, and compact/comfortable density;
- default workspace registry entry;
- configured discovery source metadata;
- optional provider configuration and consent policy;
- default export directory behavior; and
- body-free diagnostic export.

Credentials use an operating-system secret service when one is explicitly implemented and qualified. Until then,
the GUI may accept a process environment reference or one-session secret but must not write a credential to the
workspace registry or diagnostics.

## 5. CLI-to-GUI Coverage Contract

Every ordinary user mutation must have a typed GUI action. Read-only developer introspection may live under
Diagnostics, but no command family disappears.

| CLI family | Primary GUI surface | Initial coverage |
|---|---|---|
| `version`, `doctor` | About and Diagnostics | full, read-only |
| `workspace` | Workspace manager | full |
| `job` | Jobs and add-job wizard | full |
| `profile` | Profile | full |
| `discovery` | Discovery | full |
| `workflow` | Job → Workflow | full |
| `task`, `agent context` | Workflow task panel | full for task lifecycle and context |
| `criteria`, `match`, `plan` | Job → Workflow | full |
| `document`, `review` | Job → Documents / Review & Export | full |
| `package`, `render` | Job → Review & Export | full |
| `agent capabilities/assets` | Agent integration and Diagnostics | full |
| `schema`, `resource` | Diagnostics | read-only inspection/export |

Each action returns the same structured success/error envelope used to produce CLI JSON. The GUI renders human
labels, field-level validation, next actions, and receipts from that typed result. An **Advanced** disclosure may show
and copy the equivalent CLI command for learning, support, and reproducibility; it is not executed through a shell.

A checked-in parity manifest becomes the authority for command coverage. Adding a new user-facing CLI operation must
either map it to a GUI action or mark it CLI-only with a reviewed reason and expiry milestone.

## 6. Technical Architecture

### 6.1 Shared application facade

The current CLI entry point combines `clap` parsing, application orchestration, workspace opening, error mapping, and
presentation. GUI work begins by extracting the orchestration into a new `canisend-app` crate.

```text
                       ┌───────────────────────┐
                       │ Codex / Claude / user │
                       └───────────┬───────────┘
                                   │
              ┌────────────────────┴────────────────────┐
              │                                         │
┌─────────────▼─────────────┐             ┌─────────────▼─────────────┐
│ canisend-cli              │             │ canisend-gui              │
│ clap + stdout/exit policy │             │ desktop views + app state │
└─────────────┬─────────────┘             └─────────────┬─────────────┘
              │ typed actions / receipts               │
              └────────────────────┬────────────────────┘
                                   ▼
                       ┌────────────────────────┐
                       │ canisend-app           │
                       │ use-case facade        │
                       │ consent + error mapping│
                       └───────────┬────────────┘
                                   ▼
              canisend-core · store · io · resources · contracts
```

`canisend-app` owns internal typed action requests, application receipts, workspace composition, and cancellation /
progress ports. It does not own terminal strings or GUI widgets. Existing service and contract crates remain the
authorities for workflow, storage, intake, resources, and external envelopes.

The GUI must never construct SQL, authoritative paths, internal blob references, or stage transitions. It submits a
typed action and observes a typed receipt plus a refreshed read model.

### 6.2 GUI toolkit baseline

The provisional implementation baseline is pinned `egui`/`eframe`:

- it stays within the Rust build and native executable model;
- its MIT/Apache licensing is compatible with this repository;
- it is intentionally suited to small, interactive Rust tools; and
- it provides native desktop integration and an AccessKit accessibility path.

R12.0 must still qualify it before the dependency is accepted. The spike verifies the four initial GUI targets,
keyboard traversal, screen-reader semantics, IME and multiline text, file selection and drag/drop, light/dark modes,
high-DPI behavior, binary size, cold start, and background work. If a blocker cannot be fixed within the spike,
Slint is the bounded fallback evaluated before application features begin. Tauri and a separately maintained web
frontend are outside the initial spike because the first GUI should add one Rust presentation layer, not a second
frontend ecosystem.

### 6.3 UI and worker state

- The GUI event loop owns presentation state only.
- Filesystem, SQLite, network, PDF, discovery, provider, backup, render, and export operations run off the UI thread.
- At most one mutating action is dispatched per active workspace at a time; safe read models may refresh between
  receipts.
- Every long operation has visible queued/running/succeeded/failed/cancel-requested state.
- Cancellation is offered only where the underlying service can preserve its transaction and artifact invariants.
- Closing the window during an operation must either finish an atomic commit or leave a retryable durable state.
- External CLI writes are handled as a normal concurrency conflict: refresh, retry, or reopen read-only; never force
  an unsafe overwrite.

### 6.4 Read models

GUI screens consume purpose-built body-minimized read models rather than assembling state through many ad hoc store
queries. Initial read models are:

- workspace summary and health;
- active/archived job list;
- job summary with sources and next action;
- workflow timeline with blockers and freshness;
- profile source/evidence summary;
- discovery source/lead summary;
- agent task summary;
- document/review summary; and
- package/render/export summary.

Private bodies are loaded only on a screen that needs them and are not retained in global activity history or crash
diagnostics.

## 7. Visual and Interaction System

The GUI uses a restrained, content-first desktop style appropriate to academic work:

- flat surfaces with clear borders instead of decorative glass or heavy shadows;
- semantic theme tokens with teal as the primary navigation/action family, orange for the single emphasized next
  action, and red only for destructive/error states;
- a 4/8 px spacing rhythm, predictable page widths, and adjustable compact density;
- system UI fonts by default so offline builds do not download fonts; any embedded alternative requires a license and
  readability review;
- one consistent vector icon family, with no emoji used as structural controls; and
- text labels plus icons for important actions and every non-obvious status.

The minimum useful window is `800 × 600`; the preferred layout is `1100 × 720` or larger. At narrow widths the
navigation rail collapses, while the current job and primary action remain visible.

Accessibility is a release requirement, not post-polish:

- all functions are reachable by keyboard in visual order;
- focus is always visible and no modal creates a keyboard trap;
- controls have accessible names, roles, state, and recovery-focused error text;
- color is never the only indication of workflow state;
- text meets WCAG AA contrast in light and dark themes;
- reduced motion disables nonessential animation;
- text scaling does not clip critical actions; and
- progress indicators name both the current operation and the completed/remaining stage.

## 8. Packaging and Platform Matrix

The existing CLI target matrix remains unchanged. GUI support is claimed only after exact packaged-app smoke tests.

| Target | CLI `0.8` | GUI Alpha | Notes |
|---|---:|---:|---|
| `aarch64-apple-darwin` | yes | planned | native `.app` bundle plus CLI |
| `x86_64-apple-darwin` | yes | planned | native `.app` bundle plus CLI |
| `x86_64-pc-windows-msvc` | yes | planned | GUI `.exe` plus CLI `.exe` |
| `x86_64-unknown-linux-gnu` | yes | planned | desktop bundle/archive plus CLI |
| `x86_64-unknown-linux-musl` | yes | no initial claim | CLI-only until window-system dependencies qualify |

GUI and CLI bytes, versions, licenses, notices, checksums, SBOM entries, provenance, and community-signing evidence
belong to one release manifest. The existing free signing policy is extended rather than replaced: ad-hoc hardened
runtime signatures on macOS and transparent self-signed Authenticode evidence on Windows remain community-build
trust signals, not public identity claims.

The first Alpha uses GitHub Release archives. DMG, MSIX, AppImage, desktop package repositories, auto-update, and
external store publication require separate lifecycle and rollback designs; they do not block the portable GUI
Alpha.

## 9. Testing Strategy

The fast loop remains Rust-native and focused:

1. **Application facade tests** — typed action, consent, receipt, and error parity against bounded local fixtures.
2. **View-model tests** — reducers and navigation using in-memory read models; no window or network required.
3. **GUI component tests** — keyboard order, enabled/disabled state, focus, field errors, and critical layout states.
4. **CLI/GUI parity fixtures** — the same action produces equivalent structured receipts and authoritative state.
5. **Packaged desktop smoke** — launch, register/init workspace, URL/PDF/file intake fixture, workflow status, backup,
   render, export, close, reopen, and uninstall while retaining the workspace.
6. **Manual native qualification** — screen reader, high DPI, IME, theme, reduced motion, file dialogs, and platform
   bundle/signature behavior.

Per-edit work uses affected-crate tests, formatter, and relevant Clippy targets. Full four-target GUI packaging and
assistive-technology checks run in native release qualification, not in every source edit. Pixel snapshots are used
only for small deterministic components; cross-platform full-window pixel identity is not a release contract.

Initial performance budgets:

- a release build reaches the workspace chooser or selected-workspace overview within two seconds on the maintained
  reference machine;
- no network, PDF, backup, rendering, or provider operation blocks the UI event loop;
- a 100-job list remains interactive while filters change; and
- diagnostics contain no private source or document bodies.

## 10. Delivery Roadmap

Estimates are planning ranges for one experienced engineer and start only after R11.4 Stable closes.

### R12.0 — Toolkit and application-boundary spike (2–4 days)

- [ ] Write the GUI architecture ADR and dependency/license record.
- [ ] Prototype one workspace list, one job list, one workflow timeline, and one validated form.
- [ ] Verify the four GUI target builds and record the musl boundary.
- [ ] Test keyboard, focus, accessibility tree, IME, high DPI, theme, file dialog, and background operation behavior.
- [ ] Measure binary size and cold start, then accept `egui`/`eframe` or switch to the bounded fallback.

**Exit:** The toolkit and release-target boundary are fixed with a runnable synthetic prototype; no domain behavior is
duplicated in UI code.

### R12.1 — Shared application facade and parity authority (1–2 weeks)

- [ ] Add `canisend-app` with typed actions, receipts, read models, progress, and cancellation ports.
- [ ] Move orchestration out of the 4,000-line CLI entry point by command family without changing Agent v2.
- [ ] Make `canisend-cli` a thin parser/output adapter over the facade.
- [ ] Add the checked-in CLI-to-GUI parity manifest and fixtures.
- [ ] Keep all existing CLI contract, workspace, and packaged-binary tests green.

**Exit:** CLI behavior is backed by the shared facade, and a fake GUI adapter can execute every ordinary user action
without spawning a process.

### R12.2 — App shell and workspace registry (1 week)

- [ ] Add `canisend-gui` and the design-token/theme foundation.
- [ ] Implement first-run, workspace switcher, overview, activity drawer, diagnostics, and safe error recovery.
- [ ] Implement workspace create/register/status/check/backup/restore/repair/remove-from-registry.
- [ ] Add background worker, single-workspace writer queue, close/reopen behavior, and external-writer recovery.
- [ ] Add keyboard navigation, focus, text scaling, and light/dark coverage from the first screen.

**Exit:** A user can manage multiple local workspaces and complete backup/restore without the CLI or data deletion
risk.

### R12.3 — Job intake and workflow console (1–2 weeks)

- [ ] Implement job list, filters, archive, detail, and add-job flow.
- [ ] Implement supplied URL and local file/PDF import with preview, consent, progress, and recovery.
- [ ] Implement workflow timeline, blockers, freshness, next actions, begin/complete/rerun, and receipts.
- [ ] Implement criteria, match, and application-plan review/confirmation screens.
- [ ] Show equivalent copyable CLI commands without executing a shell.

**Exit:** A GUI-only user can intake a job and advance it through the first user-confirmed plan boundary.

### R12.4 — Evidence, agents, documents, review, and export (2–3 weeks)

- [ ] Implement profile sources and evidence proposal/confirmation.
- [ ] Implement discovery source refresh/import, lead review, duplicate suggestion, and promotion.
- [ ] Implement agent task preparation, consented input export, completion status, and host-pack export.
- [ ] Implement documents, findings/dispositions, readiness, projections, render, and private export consent.
- [ ] Complete the command coverage manifest and body-free diagnostics export.

**Exit:** Every ordinary state-changing CLI operation is available through a typed GUI action, and the documented
end-to-end application workflow completes without a terminal.

### R12.5 — Native GUI Alpha packaging (1–2 weeks)

- [ ] Extend manifest, checksums, SBOM, notices, provenance, and signing records to both executables.
- [ ] Package the four claimed GUI targets while preserving the five-target CLI release.
- [ ] Run packaged desktop workflow, upgrade, rollback, uninstall, and workspace-retention smokes.
- [ ] Publish GUI quick-start, workspace management, privacy, agent handoff, and troubleshooting guides.
- [ ] Publish `0.8.0-alpha.1` only after the GUI Alpha ledger is machine-checkable.

**Exit:** A clean supported desktop installs a runtime-independent GUI, completes the synthetic workflow, and uninstalls
without removing its workspace.

### R13 — GUI Beta and Stable qualification (3–5 weeks after Alpha)

- [ ] Triage public GUI reports without enabling telemetry.
- [ ] Fix data-loss, privacy, accessibility, workflow-parity, and packaging blockers before visual enhancements.
- [ ] Freeze GUI action contracts, workspace registry format, and bundle layout at Beta.
- [ ] Pass two clean-tag native RC matrices with upgrade, rollback, and assistive-technology evidence.
- [ ] Publish `0.8.0` Stable only when CLI and GUI version/support policies describe one product release unit.

**Exit:** The GUI is a supported product surface rather than an experimental launcher.

## 11. Definition of Done

- [ ] No GUI operation requires Python, Node.js, Java, an external database, or an external Typst executable.
- [ ] CLI, GUI, and Agent v2 commit through the same validators and authoritative workspace.
- [ ] Every ordinary state-changing CLI command has a typed GUI action.
- [ ] URL, PDF, file, JSON, CSV, RSS/Atom, manual, and agent paths remain visible where relevant.
- [ ] GUI errors identify the field or operation and provide a safe recovery action.
- [ ] Workspace registry removal cannot delete workspace data.
- [ ] Long-running operations do not freeze the event loop and preserve transaction/retry invariants.
- [ ] All functionality is keyboard reachable with visible focus and named accessible controls.
- [ ] Light/dark contrast, text scale, reduced motion, IME, and high-DPI checks pass on claimed targets.
- [ ] Packaged GUI launch/workflow/reopen/uninstall smokes pass on all four claimed GUI targets.
- [ ] The musl CLI remains supported and is not mislabeled as a GUI target.
- [ ] Release manifests bind both binaries, exact source, checksums, SBOM, provenance, notices, and community signing.
- [ ] The GUI never submits applications, runs arbitrary shell input, or uploads private workspace content by default.

## 12. Principal Risks and Mitigations

| Risk | Impact | Mitigation | Gate |
|---|---|---|---|
| GUI duplicates CLI orchestration | state and error behavior diverge | extract `canisend-app` before feature screens | R12.1 parity fixtures |
| UI toolkit is not accessible enough | supported users cannot operate the product | qualify keyboard/accessibility tree first; bounded fallback before feature work | R12.0 spike |
| Native GUI breaks musl portability | existing portable CLI regresses | ship musl CLI-only; separate target claims | release target verifier |
| Event-loop blocking freezes the app | poor UX and interrupted work | worker queue, progress receipts, atomic services | long-operation tests |
| Workspace registry becomes a second authority | paths and state drift | store launcher metadata only; refresh from workspace authority | registry tests |
| GUI hides consent or evidence provenance | privacy and trust regress | render exact consent, revision, source, and receipt data from shared contracts | parity/privacy tests |
| Dependency and binary size grow sharply | slow builds and downloads | toolkit spike, feature minimization, per-target size budget | R12.0/R12.5 |
| Cross-platform visual differences consume time | roadmap stalls on cosmetics | semantic/layout contracts, focused snapshots, native functional gates | GUI test policy |
| GUI feature work destabilizes `0.7` Stable | current release is delayed | design only before R11.4; implementation begins after Stable | source/release gate |

## 13. First Execution Iteration

After `v0.7.0` Stable, the first implementation iteration is intentionally bounded to R12.0 and the first vertical
slice of R12.1:

1. inventory the current CLI command handlers and publish the parity manifest;
2. write the toolkit/application-facade ADR;
3. prototype a synthetic workspace switcher, job list, workflow timeline, and validated add-job form;
4. add the typed `version`, `doctor`, `workspace status`, `workspace check`, and `job list` actions to
   `canisend-app`;
5. route the matching CLI commands through the facade without output changes; and
6. run focused facade/CLI tests plus four-target compile qualification for the prototype.

The iteration stops if the toolkit cannot meet keyboard, accessibility-tree, IME, target, or packaging requirements.
It does not begin broad GUI feature work until the ADR and shared-facade slice are accepted.
