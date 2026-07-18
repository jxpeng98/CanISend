# R11.1 native Alpha release closeout

**Date:** 2026-07-18

**Roadmap item:** R11.1

**Release:** [`v0.7.0-alpha.1`](https://github.com/jxpeng98/CanISend/releases/tag/v0.7.0-alpha.1)

## Publication identity

The annotated tag resolves to qualified commit `4cec4ec48cc2e96f3798dde0b438d3aaa617a2f8`. GitHub Actions tag run
`29633386835` completed successfully and published a non-draft prerelease named `CanISend 0.7.0-alpha.1`. The public
release remained bound to the annotated tag even though later Roadmap commits advanced `rewrite/rust-native`.

The tag workflow repeated rather than reused the branch qualification matrix:

- release identity: 44 seconds;
- complete source gates: 5 minutes 58 seconds;
- Linux musl x86_64: 11 minutes 1 second;
- macOS Intel x86_64: 16 minutes 20 seconds;
- Linux GNU x86_64, including performance and full-workflow budgets: 16 minutes 51 seconds;
- Windows MSVC x86_64: 22 minutes 22 seconds;
- macOS arm64: 23 minutes 42 seconds; and
- final assembly, verification, attestation, and publication: 1 minute 19 seconds.

Every target passed its workspace tests, release build, archive creation, exact extracted-archive smoke, and upload.
Alpha signing was explicitly not required; the release policy still fails closed for later stages until
credential-backed signing jobs exist and pass.

## Public assets

The release publishes 12 files: five platform archives, `SHA256SUMS`, CycloneDX 1.6 SBOM, release manifest,
third-party notices, release notes, known limitations, and issue-collection guidance. The manifest declares five
targets, five supplemental files, locked dependencies, `canisend.agent/v2`, `canisend.workspace/v2`, and exact source
commit `4cec4ec48cc2e96f3798dde0b438d3aaa617a2f8`. The SBOM identifies `canisend-cli 0.7.0-alpha.1` as the root
application and records 501 components.

## Independent verification

All assets were downloaded again from the public GitHub Release after publication. Verification was performed on
those downloads, not on the workflow staging directory:

1. `xtask release verify v0.7.0-alpha.1` accepted the release identity and all 11 checksum-listed files.
2. `shasum -a 256 -c SHA256SUMS` matched every listed archive and supplemental file.
3. The manifest identified the exact tag, stage, source commit, five archives, no default telemetry, and locked
   dependencies.
4. The CycloneDX document parsed as specification 1.6 with the expected Rust application root.
5. `gh attestation verify` accepted all 12 public files while enforcing repository `jxpeng98/CanISend`, signer
   workflow `.github/workflows/release.yml`, and source digest
   `4cec4ec48cc2e96f3798dde0b438d3aaa617a2f8`.

## Explicit feedback channel

The public `ISSUE_COLLECTION.md` states that CanISend enables no default telemetry, analytics, crash upload, or
background reporting. It prohibits attaching workspaces, backups, task exports, application packages, provider
requests, tokens, or credentials and requires synthetic replacement of private job/profile/application data.

The tag also contains the public `.github/ISSUE_TEMPLATE/bug.yml` form. It captures exact version, target, release
blocker class, sanitized reproduction, expected/actual result, and mandatory privacy confirmations. This provides an
explicit Alpha issue-intake path without adding any background collection mechanism.

## Transition

All R11.1 checklist items are proven: five archives, packaged-binary smokes, published trust/support files,
real-source dogfood, and explicit no-telemetry issue intake. R11.2 Beta is active. Its next work is to audit Alpha
blocker classes, freeze agent/workspace contracts for the beta line, produce Homebrew and Windows channel candidates,
and implement credential-backed macOS notarization and Windows Authenticode gates.
