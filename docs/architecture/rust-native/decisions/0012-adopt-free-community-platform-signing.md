# ADR-RN-0012: Adopt Free Community Platform Signing

**Status:** Accepted

**Date:** 2026-07-18

## Context

ADR-RN-0011 designed a paid publisher-identity tier using Apple Developer ID/notarization and Azure Artifact Signing.
Those services are useful for operating-system reputation and publisher identity, but they are not required to prove
which repository workflow produced an archive or whether the published bytes changed. The project is not configuring
paid signing for the current release sequence.

Dropping all checks would make Beta trust weaker and ambiguous. A free signature must therefore be described as a
platform integrity signal, never as an Apple- or Microsoft-trusted publisher identity.

## Decision

CanISend `0.7` uses the explicit `community-build` trust tier:

- macOS executables receive a `codesign` ad-hoc signature with the fixed code identifier and hardened-runtime flag;
- the Windows executable receives an ephemeral, non-exportable, self-signed Authenticode certificate named
  `CN=CanISend Community Build`, with SHA-256 and no public timestamp;
- Linux archives remain protected by checksums and GitHub OIDC build provenance;
- no external signing secret, repository variable, Apple account, or Azure account is required.

Beta, RC, and Stable still fail closed if their platform signing command fails, if evidence is missing, if signed
bytes differ from the packaged bytes, or if evidence does not bind the exact final archive. Canonical
`canisend.code-signing-evidence/v2` explicitly records `developer_id: false`, `notarized: false`,
`gatekeeper_trusted_publisher: false`, `certificate_trusted: false`, and `timestamp_present: false`. Evidence that
claims public trust is rejected.

Every final asset remains covered by `SHA256SUMS`, the release manifest, and GitHub build provenance. Those controls
are the public source-identity boundary. The platform signatures add tamper detection in native formats; they do not
establish a paid publisher identity or suppress Gatekeeper and SmartScreen warnings.

## Consequences

- The release can be built and published without paid credentials or long-lived private keys.
- macOS may require an explicit user approval through normal system UI because the binary is not notarized.
- Windows may show Unknown Publisher or SmartScreen warnings because the embedded certificate is self-signed.
- A Windows certificate thumbprint is release-artifact-specific, not a durable identity. It must match that
  artifact's evidence rather than a value copied from another release.
- The exact signed binary and archive remain machine-verifiable and provenance-bound.
- Moving to paid public-trust signing later requires a new policy/schema revision and fresh native qualification; it
  must not silently reinterpret v2 community evidence as public trust.

## Rejected alternatives

- Mark Beta as unsigned: rejected because native signature presence and post-sign archive binding remain useful.
- Call ad-hoc or self-signed output trusted: rejected because neither operating system grants publisher trust.
- Persist a shared self-signed private key: rejected because it adds secret rotation and compromise risk without
  providing public trust.
- Use an unofficial public timestamp endpoint: rejected because an external best-effort service would add a fragile
  release dependency while still not making the self-signed publisher trusted.

## Revisit when

Adopt a separate public-trust tier if project distribution, reputation, or support needs justify Apple Developer ID
and a trusted Windows code-signing service. Keep community-build limitations visible for every artifact produced
under this decision.
