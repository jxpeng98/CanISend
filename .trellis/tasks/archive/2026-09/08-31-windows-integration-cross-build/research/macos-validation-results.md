# macOS feature-completeness validation results

**Validated:** 2026-09-01

**Source:** `main@0d11c456d726990ff9940f404a6aebad24cf72fc`

**Host:** macOS 27.0 (`26A5425a`), Apple Silicon `arm64`

## Toolchain

- Rust/Cargo: `1.97.0`, target `aarch64-apple-darwin`.
- Node: `26.8.1`, satisfying the repository's `>=26.5.0` constraint.
- pnpm: exact repository pin `11.17.0`, executed from a `/private/tmp` cache without a global
  installation. The ordinary shell default remains `11.22.0`.

## Result

No core, CLI, MCP, host, data-integrity, frontend, accessibility, packaging-layout, or App-managed
CLI defect was found on the current macOS `main`.

The complete isolated real-window workflow is not fully proven. Computer Use launched the same App
and exposed its full primary navigation, but its normal launch did not inherit the synthetic HOME.
When the UI-control runtime invoked the repository's exact isolated LaunchServices shape,
`/usr/bin/open` returned `NSOSStatusErrorDomain -10827 (kLSNoExecutableErr)` even though the bundle's
`CFBundleExecutable`, Mach-O host, executable mode, hashes, and signatures all verified. Directly
spawning the bundled host from that runtime ended with `SIGABRT`. This is classified as a UI-control
environment limitation, not a confirmed product failure. No real Workspace or provider was used.

## Capability matrix

| Capability | Result | Evidence |
|---|---|---|
| Rust contracts, Core, Store, IO, CLI, MCP, GUI, xtask | Pass | `cargo test --workspace --locked`: all test binaries passed; five repository-owned network/performance/release tests remained explicitly ignored |
| Ordinary-user CLI quickstart | Pass | `scripts/smoke_documented_quickstart.sh`: version, doctor, help, Workspace v4, Profile, check, backup, restore, repair |
| Mixed generic and academic Applications | Pass | Quickstart and MCP smoke; preview Workspace contains both built-in Pack IDs |
| Profile, Evidence, requirements, plan, deliverables, guarded writes, export | Pass | `scripts/smoke_agent_v4_mcp.sh` completed the dual-Pack lifecycle |
| Codex/Claude Skills and MCP host lifecycle | Pass | `scripts/smoke_host_v4.sh`: project/global setup, status, removal, and legacy refusal |
| Backup, restore, reopen, and integrity | Pass | CLI/MCP smokes plus preview `workspace check` status `healthy`, zero issues |
| App-managed CLI migration/update/rollback/uninstall | Pass | `scripts/smoke_macos_gui_cli_lifecycle.sh`: one focused ignored-by-default native test executed and passed |
| Frontend component/unit behavior | Pass | Vitest: 13 files, 80 tests passed |
| Visual, keyboard, reflow, localization, and automated accessibility | Pass | Playwright: 17 tests passed, including Chinese dark/compact at 200% text |
| Production frontend build | Pass | Vite production build completed inside the preview script |
| Temporary macOS App layout and integrity | Pass | Unified arm64 host, final-byte manifest, version/size checks, ad-hoc signatures, and bundle verification passed |
| Synthetic preview Workspace | Pass | Healthy Workspace with two referenced Blobs and both generic/academic Applications |
| Isolated real-window workflow | Incomplete evidence | Main window and navigation launched; synthetic-HOME launch was blocked by the Computer Use runtime's LaunchServices context |
| Real external provider/application submission | Not exercised | Out of scope; CanISend remains non-submitting and tests used local/synthetic fixtures |
| Notarization, release signing, DMG, and public distribution | Not exercised | Owned by native release qualification, not this validation |
| Windows branch integration and `cargo-xwin` | Deferred | Await the completed, reviewable Windows branch |

## Commands and retained artifacts

- `cargo test --workspace --locked`
- `scripts/smoke_documented_quickstart.sh target/debug/canisend ...`
- `scripts/smoke_host_v4.sh target/debug/canisend ...`
- `scripts/smoke_agent_v4_mcp.sh target/debug/canisend ...`
- `scripts/smoke_macos_gui_cli_lifecycle.sh`
- `scripts/build_macos_design_preview.sh`
- `pnpm --dir apps/canisend-desktop test`

Preview receipt:
`/private/var/folders/2d/mn0mj39d5fj5j3qj2b1g0mnc0000gn/T/CanISend.design-preview.oJ484HW1fx/canisend-design-preview.receipt.json`

Preview App:
`/private/var/folders/2d/mn0mj39d5fj5j3qj2b1g0mnc0000gn/T/CanISend.design-preview.oJ484HW1fx/CanISend Design Preview.app`

Unified host SHA-256:
`b556849e950bae46748236439d78d8b63364a4874eccee5cfb2c7aedd0601ac6`

Receipt SHA-256:
`753f356b8544d8ed686e2a2db715a767a884461b2a99fef056633b8effcd6c85`
