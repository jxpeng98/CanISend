# Contributing to Rust-Native CanISend

## Toolchain

Use the toolchain pinned in `rust-toolchain.toml`. The active product does not use Python development tooling.

## Required checks

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p xtask -- release check
cargo build --release --locked
```

## Architecture

- Keep dependencies pointed inward according to ADR-RN-0002.
- Put public versioned JSON types in `canisend-contracts`.
- Keep domain rules and port traits in `canisend-core`.
- Keep SQLite/blob details in `canisend-store`.
- Keep HTTP, parsers, providers, and rendering in `canisend-io`.
- Do not let agent hosts write `.canisend/` internal state.
- Do not add a Python runtime or test dependency.

New dependencies require a documented purpose, compatible license, and evidence that they do not introduce an
unplanned end-user runtime.

## Changes and tracking

Update the Rust-native roadmap when a tracked task is completed. Add a dated note for phase transitions, dependency
decisions, material risks, and release evidence. Commits use Conventional Commits and should represent one auditable
milestone.
