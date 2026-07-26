# Verify a CanISend Native Release

Treat the executable, its release tag, checksum, provenance, notices, and known limitations as one release unit.
Do not use an archive when any identity check fails.

## Required release assets

A complete `1.0` release contains:

- five standalone CLI target archives;
- one Apple Silicon macOS desktop archive named
  `CanISend-VERSION-aarch64-apple-darwin.zip`;
- `CanISend-VERSION-aarch64-apple-darwin-qualification.json`, which binds the
  exact desktop ZIP to its native package checks;
- `SHA256SUMS`;
- `canisend-VERSION-manifest.json`;
- `canisend-VERSION-sbom.cdx.json`;
- `KNOWN_LIMITATIONS.md`, `RELEASE_NOTES.md`, and `THIRD_PARTY_NOTICES.md`.

Beta, release-candidate, and Stable releases additionally contain:

- `CanISend-VERSION-x86_64-apple-darwin-gui-compilation.json`, which records
  exact-candidate Intel release-profile compilation without claiming an Intel GUI archive or
  native runtime qualification;
- `canisend-VERSION-aarch64-apple-darwin-signing.json`;
- `canisend-VERSION-x86_64-apple-darwin-signing.json`;
- `canisend-VERSION-x86_64-pc-windows-msvc-signing.json`.

Stable additionally contains one `canisend-VERSION-channel-publication.json` record and five canonical Homebrew,
Scoop, and WinGet manifest assets. The record must scope authorization to `github-release-assets` and keep
`external_index_submission: false`; a release asset is not proof that a third-party package index accepted it.

The manifest binds the product version, exact Git commit, stage, protocol, schema, workspace format,
all five CLI targets, the macOS desktop surface, and every archive digest. The five CLI entries
remain under `artifacts`; the GUI is a separate `desktop_artifacts` entry so package-manager
generation cannot confuse an application bundle with a standalone CLI archive. For non-Alpha
signed CLI targets, `signing_evidence` names the exact evidence file. The GUI entry always names
its qualification record and requires the nested and outer ad-hoc signatures, including in Alpha.
For Alpha, `desktop_compilation` must be an empty array and the release must not contain Intel GUI
compilation evidence. For Beta and later, it records the Intel compile-only boundary with
`archive: null`, `native_runtime_qualified: false`, and an exact candidate evidence reference.
`SHA256SUMS` covers every downloadable release file except itself.
For Stable, the repository verifier also regenerates every package-manager manifest from the three referenced final
archive hashes and checks its recorded external repository path.

## Verify the release tag

Fetch the tag and ensure the manifest's `source.commit` is the commit selected by that exact tag:

```console
git fetch origin tag vVERSION
git rev-list -n 1 vVERSION
```

The workspace version and tag are exact SemVer matches. A release workflow refuses
`v1.0.0-alpha.2` while the binary and Cargo workspace still report `1.0.0-alpha.1`.

## Maintainer candidate-to-tag promotion

The native release workflow builds a release version once. Before creating a tag, dispatch
`native-release` with the exact future tag from the intended source commit and wait for the entire
candidate run to succeed. The run assembles, verifies, attests, and retains one complete
`canisend-TAG-release-assets` artifact for 30 days, but it cannot create a GitHub release.

After review, create and push an annotated tag at that exact commit. The tag-triggered workflow
does not compile or package the product again. It locates the newest successful unexpired
workflow-dispatch candidate with all three matching identities:

- exact release tag in the artifact name and manifest;
- exact tagged source commit in both the workflow run and manifest; and
- exact `native-release` signer workflow and source digest in every GitHub attestation.

It then repeats complete checksum and manifest verification, uploads the same bytes to a draft,
smokes every archive after downloading it from that draft, and publishes only if all native checks
pass. A missing or expired candidate, lightweight tag, commit mismatch, unknown draft asset, or
provenance mismatch blocks promotion. If the candidate is older than 30 days, bump or rebuild the
future version before creating its tag; never replace one file inside an existing candidate.

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
gh attestation verify CanISend-VERSION-aarch64-apple-darwin.zip --repo jxpeng98/CanISend
gh attestation verify canisend-VERSION-manifest.json --repo jxpeng98/CanISend
gh attestation verify SHA256SUMS --repo jxpeng98/CanISend
```

The verification must identify `jxpeng98/CanISend` and the repository's native release workflow. An attestation
proves which GitHub Actions identity built the bytes; it does not replace operating-system code signing.

## Verify the macOS desktop archive

The desktop ZIP has exactly two top-level entries: `CanISend.app` and
`CanISend.app.manifest.json`. Reject an archive with another top-level entry, a symbolic link, or
a different filename. After extracting, compare the companion manifest's SHA-256 values with the
final signed GUI, bundled CLI, `Info.plist`, and `BUNDLE.json`, then verify the application:

```console
codesign --verify --deep --strict --verbose=4 ./CanISend.app
codesign --display --verbose=4 ./CanISend.app
./CanISend.app/Contents/Resources/bin/canisend version --json
./CanISend.app/Contents/Resources/bin/canisend doctor --json
```

The signature display must report an ad-hoc signature. The bundle metadata and qualification
record must state `developer_id: false` and `notarized: false`; a claim of Apple publisher trust is
invalid for this channel. The qualification JSON must use
`canisend.macos-gui-qualification/v1`, name the same ZIP, match its SHA-256 and size, report
`macos-15`/`aarch64-apple-darwin`, and keep every declared bounded package check true.

For Beta and later, the Intel compilation JSON must use
`canisend.macos-gui-compilation/v1`, bind the release tag and source commit, report an `x86_64`
release binary hash from `macos-15-intel`, and explicitly keep `archive_published`,
`native_runtime_qualified`, and `support_claim` false. It is evidence that the exact candidate
source compiles for Intel macOS, not an installable or supported Intel GUI.

During Alpha development, the scheduled `intel-gui-compile` workflow provides a body-free
compile-regression record. That record uses a separate scheduled schema, is not included in the
release manifest, cannot authorize publication, and does not change the absence of an Intel GUI
archive or support claim.

## Verify platform signing evidence

For Beta or later, inspect the signing JSON named by the selected artifact's manifest entry. It must use schema
`canisend.code-signing-evidence/v2`, report `status: verified`, match the release version and target, and bind its
`archive.file`, `archive.size`, and `archive.sha256` to the downloaded archive. The evidence file itself must also
match both its manifest supplemental-file entry and `SHA256SUMS`.

macOS evidence must identify `kind: apple-adhoc`, identity `adhoc`, the fixed code identifier
`io.github.jxpeng98.canisend`, a valid hardened-runtime signature, and no `get-task-allow`. It must explicitly report
`developer_id: false`, `secure_timestamp: false`, `notarized: false`, and
`gatekeeper_trusted_publisher: false`.

Windows evidence must identify `kind: windows-authenticode-self-signed`, exact subject
`CN=CanISend Community Build`, and the artifact-specific thumbprint. It must report an intact but untrusted status,
`self_signed: true`, `certificate_trusted: false`, SHA-256 file digest, `timestamp_present: false`, and service
`powershell-self-signed-authenticode`.

A source checkout at the same version can repeat every structural, checksum, and evidence-binding check:

```console
cargo run -p xtask --locked -- release verify vVERSION /path/to/release-assets
```

Also run the native operating-system checks in the [installation guide](installation.md). Reject a non-Alpha release
that omits any required evidence, reports a different signer or trust tier, does not match the final archive, or
cannot pass the native integrity check. Do not reject the documented absence of public trust as if it were drift;
instead, reject evidence that falsely claims public trust.

## Inspect the SBOM and notices

The CycloneDX 1.6 SBOM is generated from the locked dependency graph reachable from both
`canisend-cli` and `canisend-gui`. Its composition names both application roots and therefore
covers the five standalone CLI archives and the macOS desktop archive. It includes internal crates
and conditional target dependencies, so it may list a component that is not linked into the one
archive you downloaded. `THIRD_PARTY_NOTICES.md` plus the asset license files inside the archive
are the redistribution notices.

Before using private data, read `KNOWN_LIMITATIONS.md`, extract the archive, and run:

```console
./canisend version --json
./canisend doctor --json
./canisend agent capabilities --json
```

Use `canisend.exe` on Windows. `doctor` performs an offline embedded-renderer test and verifies embedded resources;
it makes no provider request and sends no telemetry.
