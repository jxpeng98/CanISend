# R1 Rust Workspace Foundation Note

**Date:** 2026-07-17

**Status:** Foundation implemented alongside the archived Python tree; repository cutover still awaits R0 native CI

## Implemented

- Root Cargo workspace with six product crates and Rust `xtask`.
- Rust `1.97.0` toolchain pin with Rust 2024 edition and declared Rust `1.92` floor.
- Shared release profile, formatting, Clippy, and dependency-policy configuration.
- `canisend.agent/v2` response/error envelope.
- Truthful capability registry that distinguishes `available` foundation work from `planned` product work.
- Embedded generic agent bootstrap guide with a build-time SHA-256 manifest.
- Native `version`, `doctor`, and `agent capabilities` commands.
- Binary integration tests that require exactly one JSON object on stdout.
- Rust `xtask` checks for schema generation and embedded resource integrity.

## Evidence

`cargo test --workspace` passed seven tests:

- Three packaged-binary contract tests.
- Two envelope tests.
- One capability-registry test.
- One embedded-resource manifest test.

The following checks passed locally:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p xtask -- schemas check
cargo run -p xtask -- resources check
cargo build --release --locked
```

The initial stripped release binary was approximately 613 KB before storage, HTTP, PDF, and Typst production
dependencies are introduced. This number is a foundation baseline, not the final binary-size budget.

Representative JSON results reported:

- `protocol = canisend.agent/v2`.
- `workspace_format = canisend.workspace/v2`.
- `version = 0.7.0-alpha.1`.
- `python_required = false`.
- One verified embedded resource.

## Boundary still in force

The Python source and Pytest tree remain in the working branch only because the R0 native dependency matrix has not
finished. They are not referenced by Cargo. After R0 passes, R1 will remove them and replace the active README and CI
before declaring the repository Rust-only.
