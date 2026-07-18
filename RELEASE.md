# Rust-Native Release Policy

## Current state

No Rust-native CanISend release has been published yet. Version `0.7.0-alpha.1` is the first native release candidate
under active R11 verification. The repository now has a fail-closed five-target archive, packaged-smoke, checksum,
CycloneDX SBOM, release-manifest, and GitHub OIDC provenance pipeline. A tag is not created until its branch dry-run
passes.

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

## Stage gates

- **Alpha:** five unsigned archives are permitted only when exact archive smokes, SHA-256 coverage, SBOM, notices,
  known limitations, release manifest, and GitHub artifact attestations pass. GitHub Releases must mark the build as
  a prerelease.
- **Beta:** protocol v2 and workspace v2 migration contracts freeze. macOS archives must be signed and notarized;
  Windows must pass the planned Authenticode policy. The workflow currently fails closed for this stage until those
  credential-backed jobs exist and pass.
- **Release candidate:** features freeze, beta-to-RC workspace upgrades pass, and the complete clean-tag matrix passes
  twice with release notes, uninstall, and rollback guidance.
- **Stable:** the same signed archives, package-manager manifests, support policy, and measured next-roadmap inputs
  publish together.

Publication happens only on an exact `vVERSION` tag push. Manual workflow dispatch is a non-publishing dry-run even
when it validates the same version string.

## Verification

Release consumers should verify `SHA256SUMS` and GitHub artifact attestations before extraction. The complete
procedure is documented in [the release verification guide](docs/guides/release-verification.md).

CanISend contains no default telemetry. Release feedback is collected through privacy-scoped issue templates; users
must remove private advert, profile, application, workspace, provider, and credential content before submission.
