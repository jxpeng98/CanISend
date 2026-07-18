# R11.2 Beta platform signing gates

**Date:** 2026-07-18

**Roadmap item:** R11.2

**Status:** Implementation complete; credential-backed qualification pending

## Source baseline

This slice builds on Rust-native source commit `870e2cb`, after the Beta contract freeze and verified-byte-derived
package-manager candidate generator. It does not change the qualified public Alpha or claim that a Beta artifact has
already been signed.

## Implemented boundary

`release/signing-policy.json` now fixes the release-stage behavior and exact signer requirements:

- Alpha may remain unsigned;
- Beta, RC, and Stable fail before builds when any Apple or Azure configuration is missing;
- macOS arm64 and Intel require Developer ID Application signing, hardened runtime, secure timestamp, and accepted
  Apple notarization with an error-free downloaded log;
- Windows x86_64 requires Azure Artifact Signing Public Trust over GitHub OIDC, SHA-256 Authenticode, an RFC 3161
  SHA-256 timestamp, an exact signer-subject match, and both `signtool` and PowerShell verification;
- Linux GNU and musl remain checksum- and provenance-protected rather than code-signed.

The release workflow signs before packaging, requires the signing evidence to match the actual signed binary, and
smokes an extracted archive whose executable must be byte-for-byte identical. It then uses
`xtask release bind-signing-evidence` to bind that canonical evidence to the final archive name, size, and SHA-256.
For every non-Alpha release, assembly and verification require two Apple evidence files and one Windows evidence
file. Evidence is included in the release manifest, `SHA256SUMS`, and GitHub provenance subject set.

[ADR-RN-0011](../../architecture/rust-native/decisions/0011-require-credential-backed-platform-signing-evidence.md)
records why provenance alone is insufficient and why a standalone macOS binary honestly records that its online
notarization ticket cannot be stapled.

## Local validation

- `scripts/check_signing_readiness.sh alpha` accepts the policy's unsigned Alpha path.
- The same script rejects Beta with all fourteen required settings absent and accepts a complete syntactically valid
  fixture configuration.
- The readiness, macOS signing, and archive-smoke Bash scripts pass `bash -n`.
- Actionlint 1.7.12 accepts both ordinary CI and the release workflow; Ruby's YAML parser accepts both files.
- Eleven `xtask` tests pass. They cover the exact policy/workflow contract, signed-binary and final-archive evidence
  binding, rejection of binary mismatch and missing Windows timestamp evidence, Beta protocol/migration freeze, and
  channel-candidate integrity.
- `cargo clippy -p xtask --all-targets -- -D warnings` passes.
- `cargo run -p xtask -- release check` passes all schemas, resources, guides, dependency versions, Beta readiness,
  frozen contracts, channel candidates, signing policy, and five release targets.
- A fresh macOS arm64 package passes the complete extracted-archive, documented-workflow, and host-agent smoke when
  compared with the expected release binary; substituting a different expected binary fails before execution.
- The name-only GitHub configuration audit fails with the live repository's fourteen missing settings and succeeds
  against an isolated fixture containing exactly the three required secret names and eleven variable names; neither
  path requests a secret value.
- Reassembly of the public Alpha inputs still produces an 11-file unsigned release with `signing_evidence: null` for
  all five targets, so this change does not retroactively make Alpha depend on private credentials.

GitHub Actions ordinary CI `29636557516` passed all eight jobs at exact signing implementation commit
`c7d1d4c79b5b9d0ca6f6ef4f91b14f1c354e3a03`. That run includes a successful native Windows PowerShell parse of
`scripts/verify_windows_authenticode.ps1`, the complete Rust quality and dependency gates, all three recovery jobs,
and all three staged rendering/documentation jobs. The implementation slice is therefore qualified independently
of the still-missing external credentials.

## Credential and service boundary

Implementation is not the same as completed signing. The next Beta release requires an active Apple Developer ID
certificate and App Store Connect notary key, plus an identity-validated Azure Artifact Signing Public Trust account,
certificate profile, Microsoft Entra identity, and GitHub OIDC federation. Secret values must be configured through
repository Actions settings and must never enter notes, commits, artifacts, or logs.

The roadmap checkbox “Complete macOS notarization and planned Windows signing” remains unchecked until a real
credential-backed Beta dry-run produces accepted and archive-bound evidence for both macOS targets and Windows. Any
missing credential or external-service rejection is an expected fail-closed release failure, not a reason to weaken
the policy.

The [signing operations runbook](../../release/signing-operations.md) records the Apple/Azure provisioning, least-
privilege, GitHub configuration, qualification, rotation, and incident sequence. A name-only audit of the live
repository on 2026-07-18 found zero configured Actions secrets and zero configured Actions variables: all fourteen
required names are currently missing. No secret value was requested or exposed. This is the concrete external
prerequisite for the first signed Beta dry-run.

## Next work

1. Finish Alpha release dry-run `29636580836` to prove the new readiness gate preserves the qualified unsigned path
   on all five
   native runners.
2. Provision Apple and Azure identities without exposing their values, refresh the Beta blocker ledger, and advance
   the workspace version to `0.7.0-beta.1`.
3. Run the complete non-publishing Beta matrix, inspect all three signing evidence files, regenerate package-manager
   candidates from the signed assets, and only then authorize the Beta tag.
