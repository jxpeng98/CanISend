# R11.2 free community-signing policy

**Date:** 2026-07-18

**Roadmap item:** R11.2

**Status:** Qualified and published in `v0.7.0-beta.1`

## Decision

The `0.7` release line will not configure paid Apple Developer ID/notarization or Azure Artifact Signing. It uses the
free `community-build` tier defined by ADR-RN-0012 and `release/signing-policy.json` v2.

This is not a downgrade to silently unsigned Beta artifacts. Both macOS targets must carry a verified ad-hoc
signature and Windows must carry a verified ephemeral self-signed Authenticode signature. The platform evidence
schema is v2 so no previous public-trust assertion can be confused with the new contract.

## Preserved release invariants

- signing happens before packaging;
- the evidence hash and size match the exact signed executable;
- extracted archive smoke compares the packaged executable byte-for-byte with the signed input;
- `xtask release bind-signing-evidence` binds evidence to the exact final archive;
- the manifest and `SHA256SUMS` cover all archives and evidence;
- GitHub OIDC provenance covers the complete public release unit;
- Beta, RC, and Stable assembly fails on a missing or noncanonical macOS/Windows record.

## Explicit limitations

macOS evidence states that Developer ID, secure timestamp, notarization, and trusted-publisher status are absent.
Windows evidence states that its certificate is self-signed and untrusted and that no timestamp exists. Gatekeeper,
Unknown Publisher, and SmartScreen warnings remain possible. The Windows thumbprint is specific to a single build.

The signature evidence proves native integrity characteristics. Repository identity comes from the tag, checksum,
manifest, and GitHub provenance. Neither layer is documented as paid operating-system publisher trust.

## Source qualification

Exact implementation commit `f0a46ea8e9677eb2fb8ed700f98ea5b63a303cb0` passed all eight ordinary CI jobs in
run `29647788613`. The Windows render job parsed `scripts/sign_windows_self_signed.ps1` successfully; Rust quality,
dependency policy, all three recovery jobs, and the macOS/Linux/Windows render and documented-workflow jobs passed.
A local macOS arm64 executable was ad-hoc signed, verified, packaged, and accepted by the v2 final-archive binding
command.

The first native Beta attempt, run `29648128815`, passed both macOS signing paths but failed before Windows signing
because PowerShell reports Cargo's ordinary hardlinked release binary through `LinkType`. Commit
`24054abf40995707a6f212a890ebd87bef606476` corrected the invariant to reject Windows reparse points rather than
hardlinks and made that exact check part of the source contract. Focused xtask tests, Clippy, and the release check
passed before the fix was pushed.

## Qualification evidence

1. Beta readiness was refreshed and the guarded `0.7.0-beta.1` transition was applied.
2. Ordinary CI `29649032447` and nonpublishing release run `29649035321` passed at exact source
   `24054abf40995707a6f212a890ebd87bef606476`.
3. All dry-run assets passed the repository verifier; all 15 provenance attestations were checked against the exact
   branch workflow and source; both extracted macOS binaries passed native `codesign`; and the Windows evidence
   recorded a present, self-signed, untrusted, untimestamped SHA-256 Authenticode signature.
4. Homebrew, Scoop, and WinGet candidates were regenerated from those bytes. The Cask passed `brew style`; all
   candidates remained nonpublishing.
5. Annotated tag `v0.7.0-beta.1` resolved to exact source
   `24054abf40995707a6f212a890ebd87bef606476`. Tag run `29650151493` repeated the five-target matrix, published the
   prerelease, and produced fresh platform evidence and GitHub OIDC provenance.
6. Every public asset passed the verifier and tag-level provenance checks. Both public macOS archives passed native
   `codesign` inspection, release notes matched the checked-in bytes, and the Beta qualification recorder bound the
   public run and three signing targets in `release/qualification-ledger.json`.

The R11.2 signing checkbox is complete. Paid publisher identity is a separate future enhancement and is not a `0.7`
blocker. Gatekeeper, Unknown Publisher, and SmartScreen warnings remain documented limitations of this free trust
tier.
