# Rust-Native Release Policy

## Current state

No Rust-native CanISend release has been published yet. Version `0.7.0-alpha.1` is the implementation target for the
first native development archive.

PyPI and TestPyPI are not release channels for the Rust product.

## Planned native targets

- `aarch64-apple-darwin`.
- `x86_64-apple-darwin` or a validated universal macOS archive.
- `x86_64-unknown-linux-gnu`.
- `x86_64-unknown-linux-musl`.
- `x86_64-pc-windows-msvc`.

## Local release foundation checks

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p xtask -- release check
cargo build --release --locked
./target/release/canisend version --json
./target/release/canisend doctor --json
./target/release/canisend agent capabilities --json
```

## Publication requirements

Before the first alpha archive is published, the release workflow must produce platform archives, SHA-256
checksums, an SBOM, license notices, build provenance, and packaged-binary smoke results. Embedded Typst rendering is
a release requirement for the complete product but is not falsely advertised as available in the current foundation
binary.

Stable release requirements additionally include macOS notarization and the planned Windows signing policy when
credentials are available.

The full release sequence is tracked in R9–R11 of the Rust-native roadmap.
