# Verify a CanISend Native Release

Treat the executable, its release tag, checksum, provenance, notices, and known limitations as one release unit.
Do not use an archive when any identity check fails.

## Required release assets

A complete `0.7` release contains:

- five target archives;
- `SHA256SUMS`;
- `canisend-VERSION-manifest.json`;
- `canisend-VERSION-sbom.cdx.json`;
- `KNOWN_LIMITATIONS.md`, `RELEASE_NOTES.md`, and `THIRD_PARTY_NOTICES.md`.

Beta, release-candidate, and Stable releases additionally contain:

- `canisend-VERSION-aarch64-apple-darwin-signing.json`;
- `canisend-VERSION-x86_64-apple-darwin-signing.json`;
- `canisend-VERSION-x86_64-pc-windows-msvc-signing.json`.

The manifest binds the product version, exact Git commit, stage, protocol, schema, workspace format, all targets, and
each archive digest. For non-Alpha signed targets, its `signing_evidence` field names the exact evidence file.
`SHA256SUMS` covers every downloadable release file except itself.

## Verify the release tag

Fetch the tag and ensure the manifest's `source.commit` is the commit selected by that exact tag:

```console
git fetch origin tag vVERSION
git rev-list -n 1 vVERSION
```

The workspace version and tag are exact SemVer matches. A release workflow refuses `v0.7.0-alpha.2` while the binary
and Cargo workspace still report `0.7.0-alpha.1`.

## Verify SHA-256

Download `SHA256SUMS` and the selected archive into the same directory. On macOS:

```console
grep '  canisend-VERSION-TARGET.ARCHIVE$' SHA256SUMS | shasum -a 256 -c -
```

On Linux:

```console
grep '  canisend-VERSION-TARGET.ARCHIVE$' SHA256SUMS | sha256sum -c -
```

On Windows PowerShell, compare the result with the matching line in `SHA256SUMS`:

```powershell
Get-FileHash .\canisend-VERSION-x86_64-pc-windows-msvc.zip -Algorithm SHA256
```

File names are part of the check. Do not accept a digest copied from another site, issue, or message.

## Verify GitHub build provenance

With the GitHub CLI installed, verify each downloaded asset against this repository:

```console
gh attestation verify canisend-VERSION-TARGET.ARCHIVE --repo jxpeng98/CanISend
gh attestation verify canisend-VERSION-manifest.json --repo jxpeng98/CanISend
gh attestation verify SHA256SUMS --repo jxpeng98/CanISend
```

The verification must identify `jxpeng98/CanISend` and the repository's native release workflow. An attestation
proves which GitHub Actions identity built the bytes; it does not replace operating-system code signing.

## Verify platform signing evidence

For Beta or later, inspect the signing JSON named by the selected artifact's manifest entry. It must use schema
`canisend.code-signing-evidence/v1`, report `status: verified`, match the release version and target, and bind its
`archive.file`, `archive.size`, and `archive.sha256` to the downloaded archive. The evidence file itself must also
match both its manifest supplemental-file entry and `SHA256SUMS`.

Apple evidence must identify `kind: apple-developer-id-notarization`, the fixed code identifier
`io.github.jxpeng98.canisend`, hardened runtime, secure timestamp, `notarization_status: Accepted`, an error-free
notarization log digest, and the expected Developer ID identity and Team ID. `stapling_supported: false` is correct
for the distributed standalone executable; Gatekeeper retrieves Apple's notarization ticket online.

Windows evidence must identify `kind: windows-authenticode-artifact-signing`, the exact signer subject and
thumbprint, `authenticode_status: Valid`, SHA-256 file and timestamp digests, a timestamp identity, and service
`azure-artifact-signing`.

A source checkout at the same version can repeat every structural, checksum, and evidence-binding check:

```console
cargo run -p xtask --locked -- release verify vVERSION /path/to/release-assets
```

Also run the native operating-system checks in the [installation guide](installation.md). Reject a non-Alpha release
that omits any required signing evidence, reports a different signer, lacks a timestamp, or cannot pass the platform
signature check.

## Inspect the SBOM and notices

The CycloneDX 1.6 SBOM is generated from the locked dependency graph reachable from `canisend-cli` across the
supported target matrix. It includes internal crates and conditional target dependencies, so it may list a component
that is not linked into the one archive you downloaded. `THIRD_PARTY_NOTICES.md` plus the asset license files inside
the archive are the redistribution notices.

Before using private data, read `KNOWN_LIMITATIONS.md`, extract the archive, and run:

```console
./canisend version --json
./canisend doctor --json
./canisend agent capabilities --json
```

Use `canisend.exe` on Windows. `doctor` performs an offline embedded-renderer test and verifies embedded resources;
it makes no provider request and sends no telemetry.
