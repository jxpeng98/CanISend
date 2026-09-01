# Windows integration and macOS cross-build evidence

**Inspected:** 2026-08-31

## Current scope decision

The user has deferred Windows integration until the remote Windows work is complete and merged
through review. The active scope is now validation of feature completeness on the current macOS
`main`; the Windows evidence below is retained only as the handoff boundary for the later phase.

The macOS repository already provides the minimum reusable evidence paths:

- `scripts/smoke_documented_quickstart.sh` for the ordinary-user CLI lifecycle;
- `scripts/smoke_host_v4.sh` for Codex/Claude host setup and removal;
- `scripts/smoke_agent_v4_mcp.sh` for the dual-pack MCP workflow through export and restore;
- `scripts/smoke_macos_gui_cli_lifecycle.sh` for App-managed CLI replacement and recovery;
- `scripts/build_macos_design_preview.sh` for an isolated frontend/native App build, visual tests,
  staging, ad-hoc signing, verification, mixed-pack seed data, and a receipt.

No new validation framework is needed. The current shell has Rust `1.97.0` on `arm64`, Node
`26.8.1` (satisfying `>=26.5.0`), and pnpm `11.22.0`; the desktop package pins pnpm `11.17.0`, so
execution must use that exact version without installing it globally or must report the mismatch.

## Source and repository state

- `origin` is `https://github.com/jxpeng98/CanISend.git`.
- After `git fetch origin`, local `main` and `origin/main` both resolve to
  `0d11c456d726990ff9940f404a6aebad24cf72fc`.
- GitHub currently has no new CanISend Windows branch or PR. The only open PR is the unrelated,
  stale PR #181.
- Fast CI run `33291606152` passed for `main@0d11c456`.
- Feature freeze is active from protected baseline
  `acf25dc483643ca9be0210320775708da116b715` with zero recorded exceptions.
- After the freeze, product or policy paths require an exact commit-bound `release-blocker` or
  `release-evidence` exception. Documentation and Trellis task records are automatic. A new feature
  is not permitted by the current 1.0 roadmap.

## Remote native Windows work

The active Codex task `验证Windows基础功能完整性` runs on the connected Windows host in
`C:\Users\Jiaxi\work\CanISend`. It began from a clean `main` tracking `origin/main`; there is not yet
a review branch or commit to integrate.

Evidence observed so far:

- Rust `1.97.0`, Node `26.5.0`, pnpm `11.17.0`, and locked Cargo dependencies are available.
- `cargo fmt --all -- --check` passed.
- `cargo run -p xtask --locked -- release check` passed.
- Desktop formatting, Svelte/type checks, 80 frontend tests, and the production frontend build
  passed.
- A Windows CLI smoke passed version/doctor, Workspace v4 initialization, mixed generic and
  academic Applications, Profile Source import, workspace check, backup/restore, Codex and Claude
  host setup/status/removal, and MCP help.
- The native Windows workspace suite exposed one reproducible hang in
  `cli_install::tests::replacement_preserves_and_restores_the_previous_installation`; the hang
  occurs while probing the existing Windows `canisend.exe` before replacement and exceeds the
  intended 750 ms timeout.
- With that one test isolated, 483 tests passed, zero failed, and three repository-marked tests
  were ignored.
- Strict native Windows Clippy exposed four platform-conditional unused-item warnings in
  `crates/canisend-app/src/cli_install.rs`, so the Windows quality gate is not currently clean.
- The remote task added `crates/canisend-desktop/tauri.windows-verification.conf.json` only as a
  temporary verification override after a Tauri child process selected the wrong Node/pnpm from
  `PATH`. This file is not yet validated as a product change and must not be merged by default.
- The Windows desktop no-bundle build and GUI runtime check remain in progress.

The remote task must finish, remove or justify temporary verification files, and provide a clean
review branch/commit before integration can begin.

## Existing platform ownership

- `.github/workflows/fast-ci.yml` already runs core, Store, IO, CLI, and MCP contracts on native
  `windows-2025`, plus the complete macOS development lanes.
- `.github/workflows/desktop-platform-qualification.yml` already owns native Windows desktop tests,
  a no-bundle Tauri build, GUI/CLI/MCP smoke, self-signing evidence, MSI/NSIS packaging, and offline
  WebView2 candidate checks.
- `release/native-test-ownership.json` and `release/targets.json` keep Windows CLI archives on
  native `windows-2025` and explicitly assign extended packaging/runtime checks to native workflows.
- The 1.0 roadmap treats Windows and Linux GUI packages as nonpublishing qualification candidates.
  Public Windows GUI support remains out of scope, while Windows x64 CLI is one of the five public
  CLI targets.

Therefore a macOS cross-build should be a fast supplementary development check. It must not replace
native Windows tests, installer/runtime qualification, signing evidence, or release artifact builds.

## Apple Silicon cross-build preflight

- Host: `arm64` / `aarch64-apple-darwin`.
- Rust: `rustc 1.97.0`; the repository pins `1.97.0` with Clippy and rustfmt.
- Installed Rust target list currently contains only `aarch64-apple-darwin`.
- Apple Clang `17.0.0` is installed.
- `cargo-xwin` `0.23.1` is already installed as an arm64 executable.
- `prlctl` is present, but unattended initialization failed in the current sandbox. The connected
  native Windows host provides stronger day-to-day runtime evidence than Windows 11 Arm emulation.
- The repository has no existing `cargo-xwin` command, script, or developer documentation.
- The Windows dependency graph contains `aws-lc-sys` through Reqwest/Rustls. Earlier plain
  `cargo check --target x86_64-pc-windows-msvc` failed for missing Windows SDK headers; this is the
  exact toolchain gap that `cargo-xwin` is intended to supply.
- Build scripts are host-runnable: desktop delegates to `tauri-build`, CLI reads Git/Rust target
  metadata, and resources generate Rust source from repository files.

`cargo-xwin` may need to download Microsoft CRT/SDK material and accept Microsoft's licence. No
download or licence acceptance has been performed in this planning phase.

## Minimum useful macOS development loop

1. Run the affected macOS/source checks at their owning layer.
2. Build the frontend once when the desktop host is affected.
3. Use `cargo xwin test --no-run` for the portable Windows test graph and `cargo xwin build` for the
   exact Windows CLI/desktop binaries that changed.
4. Verify generated `.exe` files as PE32+ x86-64 and record SHA-256 when transferring an artifact.
5. Use native Windows Fast CI for every protected integration and the existing scheduled/native
   workflow for desktop runtime and installers.
6. Use the connected Windows host for focused reproduction of Windows-only defects discovered
   during development. Do not claim a cross-build as a runtime pass.

The smallest repository change is likely one documented/reusable developer entry point around the
existing Cargo and pnpm commands. A new CI workflow, new dependency, public Windows GUI claim, or
cross-platform abstraction is not justified by current evidence.
