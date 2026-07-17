# R1 Rust Workspace Foundation Note

**Date:** 2026-07-17

**Status:** Complete

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
- Complete removal of the active Python package, Pytest suite, package metadata, scripts, legacy contracts, and
  Python distribution workflows after preservation in `archive/python-v0.6.0b1-final`.
- Rust-only CI with an explicit guard against tracked Python product files and package metadata.
- Native preview build scaffolding for Linux, macOS, and Windows; public publication remains disabled until R11.

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

GitHub Actions run `29609526692` repeated the active-file guard, formatting, Clippy, all seven tests,
schema/resource checks, release build, and native binary smoke from a clean Linux checkout in 25 seconds. The run
completed without annotations.

The initial stripped release binary was approximately 613 KB before storage, HTTP, PDF, and Typst production
dependencies are introduced. This number is a foundation baseline, not the final binary-size budget.

Representative JSON results reported:

- `protocol = canisend.agent/v2`.
- `workspace_format = canisend.workspace/v2`.
- `version = 0.7.0-alpha.1`.
- `python_required = false`.
- One verified embedded resource.

## Completed boundary

The active branch is Rust-only. Python-era material is available through the archive tag for historical inspection,
but it is not a product dependency, compatibility target, build input, test input, or release channel. R2 begins
from the generated-contract and embedded-resource foundation established here.
