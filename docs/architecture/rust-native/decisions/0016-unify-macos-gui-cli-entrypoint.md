# ADR-RN-0016: Unify the macOS GUI, CLI, and MCP entrypoint

- Status: Accepted
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

Desktop terminal installation and MCP configuration prefer the current
`Contents/Resources/bin/canisend` or development sibling while the existing package contract
remains active. If neither exists, they may use the current executable only when it is a regular,
non-symlinked file named `canisend-gui`. An arbitrary host executable must never become an
installation source.

## Migration boundary

This decision is implemented in qualified stages:

1. create and test the shared dispatch and safe current-executable fallback;
2. rebind staging, terminal installation, MCP configuration, metadata, and archive verification;
3. remove the second App-bundled executable;
4. qualify CLI, GUI, MCP, Finder launch, accessibility, signing, clean install, upgrade, rollback,
   and package integrity against the exact candidate; and
5. freeze a lower App-size budget only from that candidate's measured bytes.

Until stages 2–4 pass, release scripts continue to package the separate CLI and the existing 128 MiB
apparent-size budget remains authoritative.

## Consequences

The GUI executable grows slightly because it now links Clap and MCP dispatch, but a qualified
single-file App removes the much larger duplicated static stack. CLI-only archives retain the
conventional `canisend` filename, while a terminal-installed copy of the unified file can also use
that destination name because dispatch is argument-based rather than executable-name-based.

The mode-selection and CLI process contracts require dedicated tests. No-argument process tests
must not be used in headless CI because they intentionally open the desktop.
