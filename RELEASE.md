# Rust-Native Release Policy

## Current state

[`v0.7.0-alpha.1`](https://github.com/jxpeng98/CanISend/releases/tag/v0.7.0-alpha.1) is the first published
Rust-native CanISend release. It is an unsigned GitHub prerelease built from exact source commit
`4cec4ec48cc2e96f3798dde0b438d3aaa617a2f8` after a successful branch dry-run. Tag release run `29633386835`
repeated the complete source and five-target matrix, published 12 assets, and attached GitHub OIDC provenance.
Independent post-publication download verification matched every `SHA256SUMS` entry, accepted the release manifest,
and verified all 12 attestations against the source digest and signer workflow. R11.2 Beta hardening is active.
Homebrew Cask, Scoop, and WinGet candidates are now deterministically derived from the verified Alpha bytes and are
explicitly non-published. Credential-backed Apple Developer ID/notarization and Azure Artifact Signing workflows now
fail closed for every non-Alpha stage and require canonical evidence bound to the final archives. Real credential
qualification and final native package-manager validation remain Beta/RC gates.

PyPI and TestPyPI are not release channels for the Rust product.

## Published Alpha targets

- `aarch64-apple-darwin`.
- `x86_64-apple-darwin`.
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

Every published archive must include SHA-256 coverage, an SBOM, license notices, build provenance, and
packaged-binary smoke results. The published Alpha satisfies those gates and includes embedded Typst rendering in
the standalone executable.

Beta, release-candidate, and Stable requirements include macOS Developer ID signing plus accepted notarization and
Windows Azure Artifact Signing Public Trust. Missing credentials are a release failure, not an unsigned fallback.
Maintainer provisioning, name-only configuration audit, qualification, rotation, and incident steps are defined in
the [native signing operations runbook](docs/release/signing-operations.md).

The full release sequence is tracked in R9–R11 of the Rust-native roadmap.

## Stage gates

- **Alpha:** five unsigned archives are permitted only when exact archive smokes, SHA-256 coverage, SBOM, notices,
  known limitations, release manifest, and GitHub artifact attestations pass. GitHub Releases must mark the build as
  a prerelease.
- **Beta:** protocol v2 and workspace v2 migration contracts freeze. macOS archives must be signed and notarized;
  Windows must pass the Authenticode policy. The implemented workflow fails closed until configured credentials,
  exact signer checks, timestamps, notarization logs, and final-archive-bound evidence all pass.
- **Release candidate:** features freeze, beta-to-RC workspace upgrades pass, and the complete clean-tag matrix passes
  twice with release notes and the verified [upgrade, rollback, and uninstall procedure](docs/guides/upgrade-and-rollback.md).
- **Stable:** the same signed archives, package-manager manifests, the machine-checked
  [support policy](docs/release/support-policy.md), and measured next-roadmap inputs publish together.

Publication happens only on an exact `vVERSION` tag push. Manual workflow dispatch is a non-publishing dry-run even
when it validates the same version string.

## Verification

Release consumers should verify `SHA256SUMS` and GitHub artifact attestations before extraction. The complete
procedure is documented in [the release verification guide](docs/guides/release-verification.md).

CanISend contains no default telemetry. Release feedback is collected through privacy-scoped issue templates; users
must remove private advert, profile, application, workspace, provider, and credential content before submission.
