# Stage 4P P1 — Unified entrypoint foundation

Date: 2026-07-31

## Scope

This source batch establishes one reusable GUI/CLI/MCP executable without changing macOS staging,
signing, version, tag, publication, feature-freeze state, or the current 128 MiB App budget. The
separate `canisend` binary and App-bundled CLI remain in place until the package and lifecycle
cutover is independently qualified.

## Implementation

- The previous CLI binary body is now the public `canisend_cli::run` library dispatcher.
- The standalone `canisend` target is a thin entrypoint over the same dispatcher.
- On macOS, `canisend-gui` opens the Tauri desktop for no arguments, Finder's launch argument, or
  an explicit single `--gui`; all CLI and MCP arguments use the shared dispatcher without opening a
  WebView.
- Desktop CLI discovery still prefers `Contents/Resources/bin/canisend` and the development
  sibling. When neither exists, it accepts the current executable only if it is a regular,
  non-symlinked file named `canisend-gui`.
- Unit and process tests cover deterministic mode selection, the public Agent v2 version response,
  MCP command availability, resource-CLI precedence, unified fallback, and rejection of arbitrary
  or symlinked host files.

## Size measurement

The Apple Silicon `release-alpha` unified executable is `63,696,000` bytes. Using the previous
bundle's `271,664` bytes of non-executable content gives an indicative one-file App size of
`63,967,664` bytes, 43.68% below the `113,577,984`-byte Alpha.5 baseline.

This is a source-build measurement, not a release claim. The current staging scripts still package
the second CLI, and only a staged, signed, exact candidate may establish the lower frozen budget.

## Verification

Completed while implementing this batch:

- `cargo test -p canisend-app -p canisend-cli -p canisend-gui --locked`;
- unified `canisend-gui version --json` and `mcp --help` process regressions;
- `cargo build --profile release-alpha -p canisend-gui
  --features canisend-gui/custom-protocol --locked`;
- strict Clippy for app, CLI, and GUI targets;
- `cargo fmt --all -- --check` and `git diff --check`; and
- `cargo run -p xtask --locked -- release check`.
