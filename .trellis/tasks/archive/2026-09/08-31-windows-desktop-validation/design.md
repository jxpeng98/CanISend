# Design: Windows desktop validation gaps

## Design objectives

1. Repair each demonstrated Windows failure at its owning boundary.
2. Preserve the clean Workspace v4 and fail-closed legacy-operation contracts.
3. Avoid new runtime dependencies and avoid changing release/support authority.
4. Keep stock commands portable across hosts and keep native evidence attributed to the host that
   actually ran it.

## Cross-host delivery boundary

The task is delivered in two checkpoints on the same branch:

1. **Windows review checkpoint (now):** task metadata, PRD/design/implementation plan, exact Windows
   evidence, and macOS handoff only. No product source, tests, release metadata, or generated
   artifacts change in this commit.
2. **macOS implementation continuation (later):** fetch the remote branch, implement the designs
   below, run focused and source gates available on macOS, and commit the fixes separately.

The macOS continuation can validate portable behavior and the macOS host. It cannot replace native
Windows execution for `HKCU` mutation, malformed `.exe` process behavior, Windows Clippy/test
execution, or a Windows WebView2 smoke. Those items must remain explicitly pending until a native
Windows/CI owner supplies evidence. This is an evidence boundary, not an implementation blocker.

## Cross-layer map

```text
Workspace v4 store
  -> application_dossier/list_application_dossiers commands
  -> typed bridge receipts
  -> App.svelte selection projection
  -> workspace context, workflow, profile, delivery and agent views

CLI install backend
  -> CliInstallStatus.path_configuration_file + ProductSummary.target_os
  -> SettingsView platform copy

Unmanaged CLI path
  -> executable plausibility check
  -> bounded version child process
  -> bounded stdout projection
  -> version comparison
  -> install/replace decision (no mutation before the decision)

pnpm test:accessibility
  -> Playwright webServer
  -> repository-local Vite process
  -> accessibility browser suite
```

## 1. Supported dossier projection replaces retired job reads

`bridge.ts` intentionally rejects `job.list` and `job.show`, and its contract test requires that
behavior. The bug is therefore in `App.svelte`, which still calls those functions during every
workspace load, selection, and refresh.

The supported `ApplicationDossierReadModel` already contains the fields used by active shell
consumers: `workspace`, `job`, and `workflow`, plus the richer dossier metadata. The active shell
will use the selected dossier as its selected Application/job context:

- `listApplicationDossiers` becomes the single list read.
- `jobs` is the stable projection `dossiers.map((dossier) => dossier.job)` used by existing routing
  and non-Application views.
- the selected object is found in that same list on initial load;
- `getApplicationDossier` refreshes one selection after a mutation;
- no synthetic `JobDetailReadModel` with an empty `sources` array is created;
- bridge stubs stay fail closed.

The variable/prop name `selectedJob` may remain in this bounded repair to avoid a broad UI rename,
but its active type becomes the supported dossier model. Static contract coverage will assert both
sides of the boundary: no retired call in `App.svelte`, rejection retained in `bridge.ts`.

## 2. Platform-aware PATH guidance

The backend already exposes `ProductSummary.target_os`; `App.svelte` will pass that value into the
lazy Settings view. Internationalization receives three variants:

- Windows: persistent current-user PATH (`HKCU\\Environment\\Path`) and Windows activation advice;
- macOS: the existing managed `.zprofile` block wording;
- neutral fallback: persistent user PATH without naming an OS-specific file or registry key.

The configuration-location label becomes platform-neutral unless a platform-specific label is
needed. This avoids presenting a Windows registry location as a shell profile. Both English and
Simplified Chinese remain structurally identical.

## 3. Windows-safe CLI version probing

The current child lifetime is nominally bounded to 750 ms, but the caller unconditionally joins a
stdout reader and Windows can block before/around malformed executable handling. The repaired
probe has two defenses:

1. On Windows, read a bounded prefix and reject files that do not have the `MZ` executable prefix
   before calling `Command::spawn`. This directly covers the observed pre-existing arbitrary-byte
   `.exe` fixture.
2. Replace an unconditional stdout-thread join with deadline-aware result delivery. Child timeout
   or wait errors kill/wait the child and return `None`; stdout is accepted only if it arrives
   within the remaining bounded interval and does not exceed `MAX_VERSION_OUTPUT_BYTES`.

No version output from a timed-out/error process influences downgrade or replacement decisions.
Inspection remains read-only, so the installer still makes its no-downgrade decision before any
rename or copy. No new crate is required; standard library process, I/O, time, and channel
primitives are sufficient.

The Windows regression directly probes malformed bytes and retains the end-to-end
replacement/restore test. Existing Unix script-based version tests continue to prove legitimate
older/newer detection.

## 4. Platform-specific Rust compilation

`Write`, `MAX_SHELL_PROFILE_BYTES`, `PATH_BLOCK_START`, and `PATH_BLOCK_END` are owned solely by the
Unix profile writer. Apply `#[cfg(unix)]` at their declarations/imports. Cross-platform `Read` and
the version-probe constants remain unconditional.

## 5. Portable Playwright server launch

The Playwright process already runs inside the package lifecycle environment, where the local
`vite` binary is on `PATH`. Its `webServer.command` will invoke `vite` directly instead of starting
another `pnpm exec` resolution layer. The port, host, strict-port behavior, readiness URL, timeout,
and no-reuse policy remain unchanged. The stock `pnpm test:accessibility` command is the regression.

## Error and cleanup behavior

- Unsupported legacy operations remain explicit errors and are never translated into empty data.
- A dossier or content-catalog read failure still flows through the existing bridge error handler.
- A failed/timed-out version probe yields unknown version; it neither authorizes a downgrade nor
  mutates the destination.
- Version-probe children are killed/reaped on timeout, bounded output is discarded on error, and
  test/workspace/GUI fixtures are removed after validation.
- Playwright retains its existing process teardown and result directories remain ignored.

## Files expected to change during the macOS continuation

- `apps/canisend-desktop/src/App.svelte`
- `apps/canisend-desktop/src/lib/components/WorkspaceContextBar.svelte`
- `apps/canisend-desktop/src/lib/workflow-navigation.ts`
- `apps/canisend-desktop/src/lib/i18n.ts`
- `apps/canisend-desktop/src/lib/views/SettingsView.svelte`
- `apps/canisend-desktop/src/lib/accessibility-contract.test.ts`
- `apps/canisend-desktop/playwright.config.ts`
- `crates/canisend-app/src/cli_install.rs`
- task-local evidence and planning files under `.trellis/tasks/08-31-windows-desktop-validation/`

The exact list may shrink during implementation if the active type can remain structurally local;
it must not expand into release contracts or public support documentation without a new decision.

The current Windows checkpoint changes only task-local Trellis files and the developer journal
created by task initialization.
