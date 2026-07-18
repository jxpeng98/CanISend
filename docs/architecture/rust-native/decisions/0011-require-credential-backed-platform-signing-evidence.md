# ADR-RN-0011: Require Credential-Backed Platform Signing Evidence

**Status:** Accepted

**Date:** 2026-07-18

## Context

Checksums and GitHub build provenance bind release files to a repository workflow, but they do not establish the
operating-system signing identity that macOS Gatekeeper or Windows Authenticode evaluates. A release manifest that
merely declares `signing_kind: apple` or `signing_kind: authenticode` is also not evidence that the final executable
was signed, timestamped, notarized where required, or left unchanged before packaging.

CanISend permits an explicitly unsigned Alpha, but Beta, release-candidate, and Stable artifacts need a fail-closed
boundary. Missing credentials, rejected notarization, a signer mismatch, a missing timestamp, or evidence that does
not match the final archive must stop the release before publication.

## Decision

`release/signing-policy.json` is the machine-checked authority for the release-stage boundary and the five supported
targets. `xtask release check` requires exact policy content, the signing scripts, the credential readiness job, the
OIDC permission, and commit-pinned Azure actions. Alpha may proceed without code signing. Beta, RC, and Stable must
have every required Apple and Azure configuration value before any target build starts.

The two macOS executables are signed with the configured Developer ID Application identity, the fixed identifier
`io.github.jxpeng98.canisend`, hardened runtime, and a secure timestamp. The workflow rejects `get-task-allow`,
submits a ZIP containing the exact signed executable with `notarytool`, waits for `Accepted`, downloads the
notarization log, and rejects any logged error. CanISend distributes a standalone command-line executable: Apple
creates a notarization ticket for that binary but does not currently support stapling the ticket to a standalone
binary. The evidence therefore records `standalone_ticket_stapled: false` and `stapling_supported: false` instead of
claiming an impossible offline staple.

The Windows executable is signed with Azure Artifact Signing Public Trust after GitHub OIDC authentication. The
action signs only the resolved target executable with SHA-256 and an RFC 3161 SHA-256 timestamp. A separate
PowerShell verifier requires both `signtool verify /pa /all` and a `Valid` `Get-AuthenticodeSignature` result, the
configured exact signer subject, and a timestamp certificate. Ordinary CI parses that verifier on a Windows runner
so a PowerShell syntax failure cannot remain hidden until release day.

Each platform verifier emits canonical `canisend.code-signing-evidence/v1` JSON before packaging. The binding command
first requires the evidence hash and size to match the actual signed executable. The extracted-archive smoke then
requires byte-for-byte equality between that executable and the packaged copy. Finally,
`xtask release bind-signing-evidence` adds the exact archive file name, size, and SHA-256. Assembly of a non-Alpha
release requires exactly these three bound files:

- `canisend-VERSION-aarch64-apple-darwin-signing.json`;
- `canisend-VERSION-x86_64-apple-darwin-signing.json`;
- `canisend-VERSION-x86_64-pc-windows-msvc-signing.json`.

The release manifest points each signed target to its evidence file, includes every evidence digest in the
supplemental-file set, and refuses unknown fields or mismatched bytes. Linux GNU and musl archives are not
code-signed; their integrity remains bound by `SHA256SUMS` and GitHub OIDC build provenance.

Signing material stays in GitHub Actions secrets or external Apple/Azure services. It is never written to the
repository, release archives, evidence JSON, or logs. Actual Beta qualification is not complete until credential-
backed jobs produce accepted evidence for all three signed targets.

## Consequences

- A non-Alpha release cannot silently downgrade to unsigned archives.
- Signer identity, secure timestamps, Apple notarization, and the final archive bytes are independently auditable.
- Package-manager candidates can be regenerated from the same signed bytes after release verification.
- Apple and Azure account provisioning remains an external operational prerequisite.
- Standalone macOS releases depend on Gatekeeper retrieving Apple's online ticket; an offline stapled-ticket claim
  is intentionally impossible for the current archive format.
- Credential-backed signing adds external-service time and failure modes to Beta, RC, and Stable release runs.

## Rejected alternatives

- Treat GitHub provenance as code signing: rejected because it does not provide an Apple Developer ID or Windows
  Authenticode identity.
- Trust the signing action's successful exit alone: rejected because exact signer, timestamp, notarization log, and
  post-package archive binding would remain unproved.
- Allow unsigned Beta when credentials are missing: rejected because release trust would vary silently by run.
- Package macOS as an application or disk image only to obtain a stapled ticket: rejected for `0.7` because the
  product contract is a standalone CLI archive; a format change requires separate design and install validation.
- Store exportable signing credentials in repository files: rejected because it expands secret exposure and makes
  rotation and least-privilege controls harder.

## References

- [Apple: Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- [Apple: Customizing the notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)
- [Microsoft: Artifact Signing trust models](https://learn.microsoft.com/en-us/azure/artifact-signing/concept-trust-models)
- [Microsoft: Authenticate to Azure from GitHub Actions with OIDC](https://learn.microsoft.com/en-us/azure/developer/github/connect-from-azure-openid-connect)
- [Azure Artifact Signing action](https://github.com/Azure/artifact-signing-action)

## Revisit when

Revisit the macOS packaging decision if an app bundle, installer package, or disk image becomes a supported channel.
Revisit Windows signing only if Artifact Signing changes its public-trust, identity, or timestamp contract. Any
replacement must preserve fail-closed credentials, exact post-sign verification, and final-archive evidence binding.
