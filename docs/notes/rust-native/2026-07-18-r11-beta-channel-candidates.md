# R11.2 Beta package-manager channel candidates

**Date:** 2026-07-18

**Roadmap item:** R11.2

**Status:** Candidate generation complete; external publication remains prohibited

## Source baseline

The first candidate set was generated from a fresh local copy of the public `v0.7.0-alpha.1` release directory that
had already passed all 11 `SHA256SUMS` entries, repository release verification, and all 12 GitHub provenance checks.
The strengthened verifier repeated the complete file checks before generation.

The candidate source binds:

- tag `v0.7.0-alpha.1` and source commit `4cec4ec48cc2e96f3798dde0b438d3aaa617a2f8`;
- release manifest SHA-256 `d8c8b6563e0ddc081e6161c4ba89d2962f7323b15b8b97a5bf8a7b98fa619eb7`;
- macOS arm64 archive SHA-256 `7668a7c48878f755e6ac43abddfc50e2d10c4ed495ab94c3e221d98470eec572`;
- macOS Intel archive SHA-256 `4a50c1f2d58ea657116fad05463e675bdf67bc5df04f153eafefd648a6c335d3`;
- Windows x86_64 archive SHA-256 `a9b5ac290094dacc00c1b287704b764cfb98a5342ad81c2da7197de2e6c49ccd`.

The generated source record explicitly states `candidate_only: true` and `publication_authorized: false`.

## Implementation

`cargo run -p xtask -- release channels TAG ASSETS OUTPUT` now:

1. re-runs the complete release verifier;
2. rejects a pre-existing output directory;
3. selects only the two macOS archives and the Windows archive from the release manifest;
4. records exact source identity, manifest digest, archive names, sizes, and hashes;
5. renders Homebrew Cask, Scoop, and WinGet candidates with real nested executable paths;
6. re-reads and exact-compares the written tree before success.

`release check` scans all checked-in candidate versions, rejects symlinks, noncanonical source fields, unknown files,
missing targets, hand-edited output, and any source record that authorizes publication. It also requires retention of
the qualified native Alpha baseline.

[ADR-RN-0010](../../architecture/rust-native/decisions/0010-derive-package-channels-from-verified-release-assets.md)
records the verified-byte and publication-boundary decisions. [The packaging guide](../../../packaging/README.md)
records regeneration and native publication gates.

## Validation evidence

- The public Alpha release directory still passes the strengthened verifier: 11 files covered by checksums, with
  artifact and supplemental metadata matching their actual sizes and SHA-256 digests.
- Seven `xtask` tests pass, including exact architecture/archive/nested-path rendering and a negative test proving a
  source record cannot authorize publication.
- `cargo run -p xtask -- release check` passes all schema, resource, documentation, dependency-version, Beta
  readiness, contract-freeze, package-candidate, and five-target release-contract gates.
- The Homebrew candidate was changed from a Formula to a Cask after Homebrew 6 rejected architecture-scoped Formula
  URLs. `ruby -c` passes and `brew style` reports one file inspected with no offenses.
- Strict Homebrew audit could not start on this workstation because Homebrew requires Xcode/Command Line Tools 27.0
  while the host has 26.3. This host-toolchain condition is not treated as candidate validation; strict audit and
  clean Apple Silicon/Intel install tests remain mandatory before channel publication.
- Scoop JSON parses successfully and uses the exact release ZIP hash and versioned extraction directory.
- All three WinGet YAML files parse successfully and use Microsoft's current 1.12 schemas and portable ZIP nesting.
  Official `winget validate` plus Windows Sandbox install/upgrade/uninstall remains mandatory for the final signed
  candidate.

## Exit and next work

The roadmap requirement to add Homebrew and Windows installation channel candidates is complete: there is a
reproducible Rust generator, a real-release-derived baseline, deterministic repository gates, and explicit
nonpublication semantics. No external package repository was changed.

R11.2 now moves to credential-backed macOS code signing/notarization and the planned Windows Authenticode path. Once
final Beta assets exist, a new candidate set will be generated from those signed bytes and subjected to the native
validators and lifecycle tests above.
