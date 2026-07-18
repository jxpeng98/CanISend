# R11.2 free community-signing policy

**Date:** 2026-07-18

**Roadmap item:** R11.2

**Status:** Implemented in source; native Beta qualification pending

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

## Qualification path

1. Run focused tests and the complete source release gate.
2. Push the implementation and require ordinary CI to pass on the exact commit.
3. Refresh Beta readiness, apply the guarded `0.7.0-beta.1` transition, and run a nonpublishing five-target matrix.
4. Download and independently verify all assets, v2 signing evidence, native signatures, and GitHub attestations.
5. Regenerate and validate package-manager candidates from those exact bytes.
6. Publish only the qualified annotated Beta tag, verify the public assets again, and record qualification.

The roadmap signing checkbox remains open until step 6 supplies public evidence. Paid publisher identity is a
separate future enhancement and is no longer a `0.7` blocker.
