# Stage 4O — Skills management and macOS App size audit

Date: 2026-07-31

## Scope

This source batch adds management for the four CanISend-owned workflow Skills and explains the
measured macOS App size. It does not change the release version, stage, feature-freeze state,
signing, tag, package, or publication.

## Skills management

- The resource layer now inspects Codex, Claude, and generic project layouts and reports
  `not-installed`, `up-to-date`, `update-available`, `incomplete`, `user-modified`, or `unmanaged`.
- Each read model lists the four bundled Skills, resource version, expected file count, installed
  file count, discovery directory, and CanISend ownership manifest.
- Managed upgrades remove obsolete CanISend-owned files only after their current digest matches the
  previous manifest.
- Uninstall performs a complete preflight before deletion. A user-modified, symlinked, non-file, or
  unmanaged path stops the operation without removing the remaining managed files.
- The same application facade backs `agent assets status|install|uninstall`, Tauri commands, and the
  bilingual Svelte **Built-in Skills** card.
- Skill status is workspace/host scoped and survives application-scope or tab changes.

## Size evidence

The committed Alpha.5 baseline records:

- Tauri GUI: `60,470,016` bytes;
- bundled CLI: `52,836,304` bytes;
- whole App: `113,577,984` apparent bytes; and
- all non-executable App content: `271,664` bytes.

The two native executables account for 99.76% of the App. The Stage 4O Svelte production output is
about 684 KiB and development `node_modules` is not packaged. The size is therefore caused by two
statically linked Rust executables that share the application, SQLite, network/PDF, and embedded
Typst/font stack—not by the Svelte interface.

The active [macOS App size strategy](../../performance/macos-app-size-strategy.md) keeps the
version-matched CLI in place for now and recommends a separately qualified single multi-call
executable as the material reduction path.

## Verification

Completed while implementing the batch:

- `cargo test -p canisend-resources --locked`;
- `cargo test -p canisend-app -p canisend-cli -p canisend-gui --locked`;
- focused CLI binary-contract and symlink-parent regression tests;
- `pnpm --dir apps/canisend-desktop check`; and
- `pnpm --dir apps/canisend-desktop test -- --run`;
- `pnpm --dir apps/canisend-desktop build`;
- strict Clippy for resources, app, CLI, and GUI targets;
- `cargo fmt --all -- --check` and `git diff --check`; and
- `cargo run -p xtask --locked -- release check`, including 37/37 CLI/GUI and Svelte parity.
