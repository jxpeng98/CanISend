# R2 Contracts, Protocol, and Resource Evidence

**Date:** 2026-07-17

**Status:** Complete

## Contract foundation

The contract crate now owns validated semantic version, UUIDv7 identity, lowercase SHA-256 digest, positive revision,
UTC RFC 3339 timestamp, and portable safe-relative-path primitives. Custom `Deserialize` implementations ensure
invalid strings cannot bypass constructors at JSON boundaries.

Initial public types cover jobs, sources, evidence, criteria, matches, plans, documents, findings, readiness, actors,
execution modes, privacy classifications, consent scopes, safe artifact references, task descriptors, and task
completion candidates. External candidates pass generated Draft 2020-12 validation, Rust deserialization, and then
domain semantic validation.

## Generated and embedded catalogs

- 15 versioned public schemas generated from Rust types.
- Canonical schema IDs under `https://schemas.canisend.dev/v2/` with schema version `2.0.0`.
- Deterministic sorted JSON and an `xtask` byte/file-set drift gate.
- 21 compiled resources covering agent guides, an example, a prompt, all schemas, and a Typst template.
- Build-generated typed `ResourceId` values and SHA-256/size/version metadata.
- Build rejection for missing, duplicate, undeclared, unsafe, or symlinked resources.
- Typed lookup plus export APIs that reject symlink and non-directory collisions.

## CLI and agent boundary

The binary now exposes `agent capabilities`, body-free `agent context`, `schema list/show`, and `resource list`.
Successful JSON commands emit exactly one stdout object and no stderr. Known failures return the same envelope on
stdout with grouped exits: usage `2`, validation `3`, conflict `4`, external I/O `5`, and internal invariant `6`.
The complete stable error registry is published by capabilities and documented in
`docs/contracts/agent-protocol-v2.md`.

Two committed JSON snapshots cover capabilities and context. Binary tests run each snapshot command twice and require
byte-identical output before comparing the parsed response with the committed snapshot.

## Verification evidence

Local verification passed:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo run -p xtask --locked -- release check
cargo build --release --locked
./target/release/canisend doctor --json
./target/release/canisend schema list --json
```

The local suite contained 19 Rust tests. The stripped macOS ARM64 release binary measured 711,232 bytes. GitHub
Actions run `29610852669` repeated the Rust-only clean-checkout gate, all tests, schema/resource drift checks, release
build, and packaged-binary smoke in 1 minute 59 seconds with no annotations.

R3 can now persist only validated identifiers, digests, revisions, actors, and safe references instead of inventing
storage-specific boundary strings.
