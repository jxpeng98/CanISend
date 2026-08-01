# ADR-RN-0016: Unify the macOS GUI, CLI, and MCP entrypoint

- Status: Accepted and implemented for macOS packaging
- Date: 2026-07-31
- Decision owner: CanISend maintainer

## Context

The Alpha.5 macOS App packages a Tauri executable and a second standalone CLI executable. Together
they account for 99.76% of the measured App because both statically link the same application,
SQLite, intake, TLS, resource, and embedded rendering stack. The separate CLI is currently used by
terminal installation and Agent MCP configuration, so deleting it only in the staging script would
break product and release-integrity contracts.

## Decision

`canisend-cli` exposes its parser, command dispatcher, JSON output, and MCP server through a reusable
Rust library entrypoint. The thin `canisend` binary calls that entrypoint and remains the CLI-only
distribution artifact.

On macOS, `canisend-gui` links the same entrypoint and selects its mode from process arguments:

- no arguments, Finder's single `-psn_*` argument, or a single `--gui` opens the desktop;
- every other explicit argument is parsed by the shared CLI dispatcher; and
- invalid or ambiguous arguments fail through the normal CLI contract instead of silently opening
  the desktop.

Desktop terminal installation and MCP configuration use the current executable only when it is a
regular, non-symlinked file named `canisend-gui` (or `canisend-gui.exe` on Windows). A legacy App
resource CLI, development sibling, or arbitrary host executable never takes precedence over the
version-matched unified host.

## Migration boundary

This decision is delivered in qualified stages:

1. create and test the shared dispatch and safe current-executable fallback;
2. rebind staging, terminal installation, MCP configuration, metadata, and archive verification;
3. remove the second App-bundled executable;
4. qualify CLI, GUI, MCP, Finder launch, accessibility, signing, clean install, upgrade, rollback,
   and package integrity against the exact candidate; and
5. freeze a lower App-size budget only from that candidate's measured bytes.

Stages 1–3 are implemented. Stage 4 remains a native candidate gate: the exact ZIP/DMG must still
pass CLI lifecycle, GUI, MCP, Finder launch, accessibility, signing, clean install, upgrade,
rollback, and package-integrity qualification before publication.

## Consequences

The GUI executable grows slightly because it now links Clap and MCP dispatch, but a qualified
single-file App removes the much larger duplicated static stack. CLI-only archives retain the
conventional `canisend` filename, while a terminal-installed copy of the unified file can also use
that destination name because dispatch is argument-based rather than executable-name-based.

The installed `canisend`/`canisend.exe` basename selects CLI help when no arguments are present;
the `canisend-gui`/`canisend-gui.exe` basename selects the desktop. Explicit CLI and MCP arguments
always select the shared dispatcher. These rules are covered by platform-policy unit tests. A
no-argument desktop-host process test must not be used in headless CI because it intentionally
opens the desktop.
