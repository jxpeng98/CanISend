# ADR-RN-0008: Ship Platform-Specific Native Archives Before Package-Manager Distribution

**Status:** Accepted

**Date:** 2026-07-17

## Context

Rust produces binaries for target triples rather than one executable that runs on every operating system and CPU.
CanISend needs a release sequence that proves the packaged executable works without development runtimes before
adding multiple installation channels.

## Decision

The initial release matrix is:

- `aarch64-apple-darwin`.
- `x86_64-apple-darwin` or a validated universal macOS archive.
- `x86_64-unknown-linux-gnu`.
- `x86_64-unknown-linux-musl`.
- `x86_64-pc-windows-msvc`.

GitHub Release archives, SHA-256 checksums, SBOM, provenance, licenses, and a signed release manifest are the first
distribution mechanism. Homebrew and Scoop/WinGet follow after archive installation and update behavior are proven.
PyPI is not used for the Rust product.

Every packaged target runs version, doctor, workspace initialization, agent capabilities, local advert import,
synthetic task completion, host asset export, and offline embedded PDF rendering.

## Consequences

- Cross-platform native release jobs are required even when normal pull requests test on Linux only.
- Platform signing is a stable-release gate; ADR-RN-0012 defines the current free community trust tier.
- Linux ARM64 is deferred until a reliable build and smoke environment is available.
- Archive contents and installation instructions must remain consistent across platforms.

## Rejected alternatives

- Publish only `cargo install`: rejected because it requires users to install a Rust toolchain and compile dependencies.
- Publish only a container: rejected because CanISend is a local workspace CLI used by host agents.
- Package-manager-first release: rejected because it multiplies update and rollback surfaces before the binary is
  proven.

## Revisit when

Revisit additional targets and package managers after beta usage and native runner evidence justify them.
