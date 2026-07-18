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

The manifest binds the product version, exact Git commit, stage, protocol, schema, workspace format, all targets, and
each archive digest. `SHA256SUMS` covers every downloadable release file except itself.

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
