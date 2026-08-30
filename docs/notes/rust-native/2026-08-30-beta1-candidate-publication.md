# Beta.1 candidate publication and public verification

Date: 2026-08-30

## Exact release binding

- Tag: `v1.0.0-beta.1`
- Annotated tag object: `96d386b136bdfddea47abeb354df1413f5c346a7`
- Source commit: `6e1397b79031cad54e794ccdc9edca2153f23b3e`
- Candidate workflow run:
  <https://github.com/jxpeng98/CanISend/actions/runs/33281162734>
- Candidate artifact: `canisend-v1.0.0-beta.1-release-assets` (`9723581536`)
- Promotion workflow run:
  <https://github.com/jxpeng98/CanISend/actions/runs/33283530240>
- Promotion evidence artifact: `9723780512`
- Public-verification evidence artifact: `9723814046`
- Public release:
  <https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-beta.1>
- Release-manifest SHA-256:
  `2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`
- `SHA256SUMS` SHA-256:
  `3af34e9ac644ef4dabc550b3af57c3a5dc587bcd34e35457fcc5f8ea3653950a`
- Public asset count: `20`

The annotated tag peels to the exact protected source. The nonpublishing candidate run completed
the five CLI targets, supported macOS App packages, signing records, SBOM, checksums, release
manifest, and attestations. Promotion reused artifact `9723581536`, reported
`recompiled_during_promotion: false`, and skipped every source, build, signing, packaging, and
attestation job.

All draft CLI and App ZIP/DMG smokes passed before publication. A fresh independent download then
verified the complete public release, including all 19 manifest-managed assets and all 20 GitHub
attestations. The candidate and public directories compared byte-for-byte with no differences.
Every attestation identifies the repository release workflow and exact source commit above.

## Contract and signing boundary

The manifest declares `canisend.release-manifest/v1`, public schema `4.0.0`, Agent v4,
Workspace v4, host-resource v4, five supported CLI targets, telemetry disabled, and required
archive and desktop signing evidence.

The two macOS archives use valid Apple ad-hoc signatures with hardened runtime and without
`get-task-allow`. They are not Developer ID signed or notarized and do not establish Gatekeeper
publisher trust or secure timestamps. The Windows executable is signed with the documented
self-signed CanISend community certificate; it is not chain-trusted or publicly timestamped.
These community-signing limits remain visible and are not qualification failures under the
accepted Beta matrix.

## Disposition

`v1.0.0-beta.1` is an exact, build-once, publicly verified prerelease. Its checked-in ledger
remains Beta / `beta-qualifying`: this evidence does not record Beta qualification, generate or
publish package channels, activate feature freeze, count invited users, authorize RC, or
authorize Stable. `M4-LEDGER-001` owns the next transition from independently downloaded public
assets.

This note retains no application body, transcript, prompt, credential, private user content, or
host path.
