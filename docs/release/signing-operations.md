# Native Release Signing Operations

## Community signing trust tier

CanISend currently publishes the free `community-build` tier defined by `release/signing-policy.json`. Community
signing is an integrity control, not a publisher identity. It does not require paid accounts, long-lived signing
keys, GitHub Actions secrets, or repository variables.

The trust boundary has three layers:

1. `SHA256SUMS` and the release manifest bind every exact asset.
2. GitHub build provenance identifies the repository workflow and source that produced those bytes.
3. Native signatures make post-build executable changes visible in `codesign` or Authenticode inspection.

The third layer is intentionally not publicly trusted. macOS Gatekeeper can warn because ad-hoc output has no
Developer ID or notarization. Windows SmartScreen can warn because the Authenticode certificate is self-signed.
Release notes, evidence, and user guidance must never describe these artifacts as Apple-notarized, Developer-ID
signed, Microsoft-trusted, or warning-free.

## Platform implementation

For Beta, RC, and Stable, `scripts/sign_macos_adhoc.sh` signs each macOS executable with identity `-`, identifier
`io.github.jxpeng98.canisend`, hardened runtime, and no timestamp. It verifies the signature, rejects any certificate
authority, Team ID, timestamp, or `get-task-allow` entitlement, and emits v2 evidence.

`scripts/sign_windows_self_signed.ps1` creates a 3072-bit RSA code-signing certificate in the runner's current-user
store. Its private key is non-exportable and the certificate is removed immediately after signing. The verifier
requires the embedded certificate to be self-issued as `CN=CanISend Community Build`, requires the code-signing EKU,
rejects missing, damaged, trusted, or timestamped signatures, and emits the actual untrusted Authenticode status.
The certificate thumbprint is expected to change between builds.

Linux GNU and musl executables are not platform-signed. Their release archives use the same checksums, manifest, and
GitHub provenance boundary as every other target.

## Local and CI checks

No GitHub signing configuration is required. Confirm policy/workflow consistency with:

```console
./scripts/audit_community_signing_configuration.sh
./scripts/check_signing_readiness.sh beta
cargo run -p xtask --locked -- release check
```

The first command rejects a reintroduced paid-signing dependency. The readiness command confirms only the global
stage policy; each native runner independently fails closed when its platform tool or signature verification is
unavailable.

## Release qualification sequence

1. Refresh the blocker/readiness snapshot and prepare the reviewed release-stage transition.
2. Run the complete non-publishing five-target matrix from the exact candidate commit.
3. Download the assembled assets and run `xtask release verify`.
4. Inspect the two `apple-adhoc` records and the `windows-authenticode-self-signed` record. Confirm every limitation
   flag remains false where public trust would otherwise be implied.
5. Verify every asset's GitHub attestation and compare the native signature with its artifact-specific evidence.
6. Regenerate package-manager candidates from those exact verified bytes and run their native lifecycle tests.
7. Only then create and push the annotated release tag. Re-download and independently verify the public assets
   before recording qualification.

Never replace a failed artifact with a locally rebuilt archive, reuse signing evidence from another run, turn a
signed target into `signing: none`, or change a false public-trust field merely to pass a release gate.

## User approval boundary

Users should first verify checksums and GitHub build provenance. If the operating system warns, use only its normal
per-application approval UI after verifying the exact artifact. Do not advise users to disable Gatekeeper,
SmartScreen, antivirus, or execution policy globally.

## Future paid tier

Apple Developer ID/notarization and a publicly trusted Windows certificate remain possible future enhancements, but
they are not `0.7` release prerequisites. Introducing either requires a reviewed policy/schema change, secret and
identity operations, native clean-machine qualification, and documentation that distinguishes new public-trust
artifacts from existing community builds.
