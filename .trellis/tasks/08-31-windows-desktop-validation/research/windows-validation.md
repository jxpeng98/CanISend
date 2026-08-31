# Windows validation baseline

## Scope and provenance

- Repository: `jxpeng98/CanISend`
- Local path: `C:\\Users\\Jiaxi\\work\\CanISend`
- Branch: `main`
- `HEAD` and `origin/main`: `0d11c456d726990ff9940f404a6aebad24cf72fc`
- Validation date: 2026-08-30 through 2026-08-31
- Host: Windows x86_64
- Toolchain: Rust 1.97.0, Node.js 26.5.0, pnpm 11.17.0

The remote refs were fetched with pruning before validation, and `main` was the newest remote
branch by committer date at the time of the run.

## Automated baseline

| Area | Result | Evidence |
| --- | --- | --- |
| Rust formatting | Pass | `cargo fmt --all -- --check` |
| Release source check | Pass with reported release drift | `cargo run -p xtask --locked -- release check` |
| Frontend formatting/check | Pass | Prettier and Svelte check: 0 errors, 0 warnings |
| Frontend unit tests | Pass | Vitest: 80 passed |
| Frontend production build | Pass | Vite production build |
| Accessibility | Harness defect, suite itself passes | stock server launch hung; direct local Vite then produced 14/14 passes |
| Rust workspace tests | Product-test hang | exact CLI replacement test did not finish; with that test skipped: 483 passed, 3 ignored, 1 filtered, 0 failed |
| Strict Clippy | Fail | four Windows-only unused-item warnings in `cli_install.rs` |
| Windows release GUI build | Pass | `target\\x86_64-pc-windows-msvc\\release\\canisend-gui.exe` |

`release check` did not authorize a new release claim. Release status reported four drift items,
three stage-blocking items, and source 35 commits ahead of `v1.0.0-beta.1`; the status was
non-authoritative for publication.

## CLI and MCP smoke

The actual release binary passed:

- `version --json`;
- `doctor --json`, including embedded fonts, Typst, and PDF verification with no runtime downloads;
- MCP help and a bounded MCP interaction;
- Workspace v4 init/status/check;
- generic and academic Application creation;
- profile import/list;
- backup and restore;
- Codex and Claude host setup/status/remove.

## Native GUI smoke

The release host launched successfully and the following visible areas were exercised:

- Today and native diagnostics (`windows/x86_64`, embedded resources and PDF healthy);
- Workspaces and a real clean Workspace v4 creation;
- Applications/Opportunities;
- Settings and Agent integration;
- English/Simplified Chinese language switching.

The created workspace reached schema v20 and passed integrity inspection. Test workspace registry
entries were removed afterward and the GUI process was closed.

## Reproducible gaps

### 1. Retired read operation invoked during workspace activation

Immediately after real GUI workspace creation, the shell showed:

```text
job.list is not available in clean Workspace v4. Use the neutral Applications collection and
Agent v4 workflows.
```

Code trace:

- `App.svelte` calls `listJobs` and `showJob` while loading/selecting a workspace.
- `bridge.ts` deliberately maps both calls to `unsupportedLegacyDesktopOperation`.
- `accessibility-contract.test.ts` explicitly requires those operations to remain retired.
- `listApplicationDossiers` and `getApplicationDossier` are supported native reads and contain the
  job/workflow fields used by the active shell.

Expected repair: remove the caller-side legacy dependency; do not weaken the bridge guard.

### 2. Windows Settings describes macOS mutation

English and Chinese Settings copy says CanISend writes a managed block to macOS `.zprofile`.
The Windows backend actually reports `HKCU\\Environment\\Path` and uses the current user's
persistent Environment registry value.

Expected repair: branch on backend-reported platform, preserve macOS wording, and provide a neutral
fallback.

### 3. Strict Windows Clippy warnings

`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` reports:

- unused `std::io::Write`;
- unused `MAX_SHELL_PROFILE_BYTES`;
- unused `PATH_BLOCK_START`;
- unused `PATH_BLOCK_END`.

All four are used only by `#[cfg(unix)]` shell-profile functions.

### 4. CLI version-probe hang

The exact test
`cli_install::tests::replacement_preserves_and_restores_the_previous_installation` repeatedly
remains at `running 1 test` on Windows. The fixture places arbitrary bytes at a path named
`canisend.exe`; install inspection tries to execute that unmanaged candidate to determine its
version. The nominal child timeout does not bound the complete call because process launch/output
collection can still block.

Expected repair: reject obviously malformed Windows executables before spawn and make stdout
delivery deadline-aware, while preserving no-downgrade and replacement/restore invariants.

### 5. Playwright web-server command is not portable in this toolchain

The committed `webServer.command` uses `pnpm exec vite`. Under the validated Windows environment,
that nested package-manager resolution did not start Vite before the 30-second deadline. Invoking
the same repository-local Vite binary directly started in about 2.1 seconds, after which the exact
accessibility suite passed all 14 tests.

Expected repair: let the package lifecycle environment resolve `vite` directly.

## Completeness assessment

The Windows CLI and core local-first flows are functional at this commit, and the Windows GUI
candidate builds, launches, renders, and reaches its principal views. Validation is not complete
enough to claim macOS-equivalent public GUI support because the five gaps above are reproducible
and because Windows GUI publication is explicitly deferred under current release authority.

The task is complete only after the bounded fixes pass their owning-layer regressions, the final
source gate, and a rebuilt native Windows smoke. Even then, the result is local candidate evidence,
not a support-policy promotion.

## Artifact and cleanup notes

- Ignored command artifacts were kept under `target/windows-verification` during investigation.
- The GUI workspace registry was restored to an empty test state after manual validation.
- No user workspace data, public release metadata, remote branch, or release artifact was changed
  during the baseline run.

## Cross-host disposition

Per maintainer direction, the Windows session stops after publishing this review as a task-only
checkpoint. Product changes and runnable follow-up validation move to macOS using
`research/macos-handoff.md`. Windows-native evidence remains required for Windows-only runtime
claims and is not substituted by the later macOS pass.
