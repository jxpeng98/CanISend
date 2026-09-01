# Implementation plan

- [x] Capture `HEAD`, worktree state, macOS architecture/version, and Rust/Node/pnpm versions.
- [x] Select a temporary/non-global `pnpm@11.17.0` invocation; record any toolchain mismatch.
- [x] Run `cargo test --workspace --locked`.
- [x] Build the debug CLI and GUI host required by the existing smoke scripts.
- [x] Run `scripts/smoke_documented_quickstart.sh` against an isolated temporary Workspace.
- [x] Run `scripts/smoke_host_v4.sh` and `scripts/smoke_agent_v4_mcp.sh` against isolated temporary
      homes/workspaces.
- [x] Run `scripts/smoke_macos_gui_cli_lifecycle.sh`.
- [x] Run `scripts/build_macos_design_preview.sh` without publishing or opening it automatically;
      capture the receipt, App path, and verification output.
- [x] Attempt the isolated preview launch and observe the primary navigation. The Computer Use
      runtime could not carry the synthetic HOME through LaunchServices; record the exact blocker
      instead of inspecting real Workspace data.
- [x] Confirm the worktree changed only under this Trellis task.
- [x] Produce a pass/fail/not-exercised capability matrix and identify any release-blocking gap.
- [x] Leave Windows integration and `cargo-xwin` execution pending until the future Windows branch is
      complete and merged through review.

## Review gates

- Start execution only after the user reviews this scoped plan.
- Do not modify product code in response to a failure.
- Do not run release publication, notarization, Windows SDK download/licence acceptance, or protected
  remote mutations.
