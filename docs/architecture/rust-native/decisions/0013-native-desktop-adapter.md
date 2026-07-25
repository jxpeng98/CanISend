# ADR-RN-0013: Add a native desktop adapter over a shared application facade

**Status:** Accepted

**Date:** 2026-07-24

## Context

CanISend's Rust CLI and Agent v2 protocol provide the complete authoritative workflow, but ordinary
desktop users still need to translate forms and state into terminal commands. A GUI must not become
a second workflow implementation or use shell automation as an integration layer.

The GUI roadmap provisionally selected `egui`/`eframe`. The product is now macOS-first, with
Windows and Linux GUI qualification deferred until the native macOS path is usable.

## Decision

- Add `canisend-app` as a typed use-case facade over the existing contracts, core, store, I/O, and
  resource crates.
- Keep `canisend-cli` as the terminal/Agent v2 adapter and add `canisend-gui` as a second native
  adapter.
- Pin `eframe`/`egui` for the immediate-mode desktop presentation layer and use its native
  AccessKit integration.
- Use native file/folder dialogs for bounded user-selected paths.
- Run filesystem, SQLite, network, PDF, backup, render, and export work outside the UI event loop.
- Persist only body-free workspace launcher metadata outside workspaces. Authoritative state remains
  in each workspace.
- Treat terminal installation as a bounded distribution action over the version-locked bundled
  `canisend` executable. The GUI may copy, hash, atomically replace, and remove that executable,
  but it does not invoke CLI commands for product behavior or accept shell input. It may invoke the
  exact destination with fixed version-only arguments under an output and time bound.
- Install to a user-owned terminal directory by default. Never edit a shell profile implicitly,
  downgrade a newer CanISend version, replace an existing command without a user-invoked install
  action and rollback backup, or remove a managed binary whose digest changed after installation.
- Make online release checks manual, body-free, host-allowlisted, bounded, and non-installing.
- Keep GUI code out of the historical five-target `0.7` release unit. Under ADR-RN-0014, GUI
  release identity begins with the separately activated `1.0` line.

## Consequences

- CLI and GUI can share actions and errors without invoking one another as processes.
- A GUI user can install the version-matched native CLI for terminal and agent-host use. Migrating
  an earlier CanISend version preserves it for rollback, and GUI uninstall restores it.
- The workspace remains portable between CLI, GUI, Codex, and Claude.
- Native desktop dependencies increase build size and platform-specific qualification work.
- Linux musl remains CLI-only until a window-system boundary is supported.
- The original six-crate workspace decision is extended by two justified outer-layer crates.

## Rejected alternatives

- Spawn the CLI for workflow behavior and parse JSON: rejected because process discovery,
  cancellation, error recovery, and version skew would become product behavior. The fixed,
  timeout-bounded version handshake is the only process exception.
- Embed a general terminal: rejected because arbitrary command execution is outside the product
  boundary and would obscure consent, output, and failure ownership.
- Tauri or a browser frontend: rejected because it adds a second frontend toolchain and runtime
  packaging surface.
- SwiftUI-only implementation: rejected because it would fork presentation work before later
  Windows and Linux support.
- Direct GUI access to SQLite: rejected because it duplicates storage and workflow invariants.

## Qualification evidence required

- Facade/CLI parity fixtures for every mapped action.
- Keyboard order, visible focus, AccessKit names/roles, IME, theme, high-DPI, and file-dialog checks.
- Background-operation tests proving the event loop remains responsive.
- Native macOS bundle, ad-hoc signing, launch, workflow, reopen, and uninstall-retention evidence.
- CLI installer lifecycle fixtures covering clean install, version-aware migration, atomic update,
  newer-version downgrade refusal, previous-installation restoration, external modification
  refusal, and workspace non-interference.
- Release-response fixtures plus a manual public-endpoint update-check smoke.
