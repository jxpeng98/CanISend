# ADR-RN-0010: Derive Package Channels from Verified Release Assets

**Status:** Accepted

**Date:** 2026-07-18

## Context

Homebrew, Scoop, and WinGet represent another supply-chain surface after native archives. Hand-maintained URLs,
hashes, archive paths, or versions can silently diverge from the bytes that passed CanISend's release matrix. The
native bundles also contain a versioned top-level directory, so assuming a root-level executable produces broken
installers even when the download hash is correct.

A channel candidate is useful before publication, but it must not imply that an external channel is live or that an
unsigned Alpha archive is ready for broad installation.

## Decision

`xtask release channels TAG ASSETS OUTPUT` accepts only a complete directory that passes `xtask release verify`.
The verifier checks every archive and supplemental file against both `SHA256SUMS` and the release manifest, including
the declared target, archive format, executable, runner, signing kind, size, and SHA-256 digest.

The command emits one immutable source record and deterministic candidates for:

- a Homebrew Cask using `arch`, architecture-specific SHA-256 values, and the nested `binary` path;
- a Scoop manifest using the Windows ZIP hash plus its versioned `extract_dir`;
- a three-file WinGet 1.12 portable/ZIP manifest with an exact `NestedInstallerFiles` path.

Homebrew uses a Cask, not a Formula, because CanISend distributes upstream-built native binaries. This follows
Homebrew's current distinction between source-building Formulae and upstream precompiled Casks.

Every source record is canonical JSON with `candidate_only: true` and `publication_authorized: false`. Repository
release checks regenerate every candidate in memory and require exact path and byte equality. Historical candidate
sets remain available for audit. A signed Beta or later release produces a new set rather than mutating an old one.

External channel publication remains a separate Stable-stage action. It requires official package-manager
validation, clean native install/upgrade/uninstall evidence, final community-signed archive hashes, two clean RC
matrices, and explicit roadmap authorization.

## Consequences

- Package metadata cannot drift independently from verified release bytes.
- Archive-layout mistakes are caught before external submission.
- Candidate generation is cross-platform Rust; package-manager-native validators remain mandatory before publishing.
- Alpha candidates can exercise the pipeline without creating a public installation promise.
- Release manifests are now verified more deeply because package-channel generation depends on their artifact data.

## Rejected alternatives

- Hand-edit package manifests after each release: rejected because URLs, nested paths, and digests can diverge.
- Publish the first generated candidate immediately: rejected because generation does not prove signing, install,
  upgrade, uninstall, repository policy, or rollback behavior.
- Use a binary-only Homebrew Formula: rejected because current Homebrew policy routes upstream precompiled binaries
  through Casks.
- Flatten release archives only for package managers: rejected because it would create channel-specific release bytes
  and weaken checksum/provenance reuse.

## Revisit when

Revisit the channel set after Stable usage data justifies Linux package managers, Windows arm64, or macOS universal
binaries. Any new channel must preserve verified-byte derivation and explicit publication authorization.
