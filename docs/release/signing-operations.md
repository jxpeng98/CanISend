# Native Release Signing Operations

This runbook provisions the external identities required by `release/signing-policy.json`. It does not weaken the
policy when an account, credential, or service is unavailable. Alpha may be unsigned; Beta, release-candidate, and
Stable runs must fail closed until all Apple and Azure requirements pass.

Never paste a certificate, password, private key, access token, or secret value into a commit, issue, note, terminal
transcript, Actions log, or release artifact. Repository audits inspect configuration names only.

## 1. Apple Developer ID and notarization

The Apple Account Holder creates a **Developer ID Application** certificate for software distributed outside the Mac
App Store. Install the certificate and its private key in a local Keychain, verify the exact identity with
`security find-identity -v -p codesigning`, and export only that identity as a password-protected PKCS #12 file.

Create an App Store Connect **team** API key for notarization. Do not use an individual API key: Apple documents that
individual keys cannot use `notarytool`. Record the team key ID and issuer UUID, download the `.p8` private key once,
and keep the original in an access-controlled credential store. Use the narrowest team-key role that successfully
authorizes the Notary service for the account; do not grant unrelated administrative access merely for CanISend.

Configure these repository Actions secrets:

- `APPLE_DEVELOPER_ID_P12_BASE64`: base64 of the exported PKCS #12 bytes;
- `APPLE_DEVELOPER_ID_P12_PASSWORD`: the export password;
- `APPLE_NOTARY_KEY_P8_BASE64`: base64 of the team API private-key bytes.

Configure these repository Actions variables:

- `APPLE_SIGNING_IDENTITY`: the complete `Developer ID Application: ...` identity shown by Keychain;
- `APPLE_TEAM_ID`: the ten-character Developer Team ID;
- `APPLE_NOTARY_KEY_ID`: the team API key ID;
- `APPLE_NOTARY_ISSUER_ID`: the team API issuer UUID.

Prefer stdin so secret bytes do not become command arguments or shell history. The following examples assume the
files are already stored securely and are executed from an authenticated maintainer workstation:

```console
base64 -i DeveloperIDApplication.p12 | \
  gh secret set APPLE_DEVELOPER_ID_P12_BASE64 --repo jxpeng98/CanISend
gh secret set APPLE_DEVELOPER_ID_P12_PASSWORD --repo jxpeng98/CanISend
base64 -i AuthKey_KEYID.p8 | \
  gh secret set APPLE_NOTARY_KEY_P8_BASE64 --repo jxpeng98/CanISend
```

The workflow decodes those values only inside a temporary macOS signing directory, imports the certificate into a
temporary keychain, and deletes both on exit. A real qualification run must prove the configured identity, Team ID,
fixed code identifier, hardened runtime, secure timestamp, absence of `get-task-allow`, accepted submission, and an
error-free downloaded notarization log.

## 2. Azure Artifact Signing Public Trust

Use Microsoft's current Artifact Signing setup flow to register `Microsoft.CodeSigning`, create an Artifact Signing
account, complete **Public** identity validation, and create a **Public Trust** certificate profile. Public Trust Test
is not acceptable for a public CanISend Beta or later release.

Create or select a Microsoft Entra application or user-assigned managed identity for GitHub OIDC. Add federated
identity credentials for the exact repository contexts that may run a non-publishing dry-run and the authorized
release tag. With the current no-environment workflow, those subjects are
`repo:jxpeng98/CanISend:ref:refs/heads/rewrite/rust-native` for branch dry-runs and
`repo:jxpeng98/CanISend:ref:refs/tags/v0.7.0-beta.1` for the first Beta tag. Confirm the token claims in GitHub and
add separately reviewed subjects for later RC/Stable tags; do not create a broad credential that trusts unrelated
repositories or refs. Assign the identity only the
**Artifact Signing Certificate Profile Signer** role at the certificate-profile scope; Contributor or Owner is not
required for signing.

Configure these repository Actions variables:

- `AZURE_CLIENT_ID`;
- `AZURE_TENANT_ID`;
- `AZURE_SUBSCRIPTION_ID`;
- `AZURE_ARTIFACT_SIGNING_ENDPOINT`, including the trailing slash, for example
  `https://eus.codesigning.azure.net/`;
- `AZURE_ARTIFACT_SIGNING_ACCOUNT`;
- `AZURE_ARTIFACT_SIGNING_PROFILE`;
- `WINDOWS_SIGNING_EXPECTED_SUBJECT`, exactly matching the Public Trust signing certificate subject.

The three Azure IDs are identifiers, not client secrets. CanISend does not configure an Azure client secret: the
release job requests a short-lived GitHub OIDC token. The Windows job signs only the resolved `canisend.exe`, uses
SHA-256 plus the Microsoft RFC 3161 timestamp service, and rejects a signature whose subject or timestamp differs
from policy.

## 3. Repository configuration audit

After configuration, run:

```console
./scripts/audit_github_signing_configuration.sh jxpeng98/CanISend
```

The script compares only secret and variable names. It cannot retrieve secret values and does not prove that a
certificate, key, federation, role assignment, or service endpoint works. A missing name returns a nonzero status.
The release workflow's `signing-readiness` job performs the first value-shape check without printing values.

## 4. Qualification sequence

1. Keep the workspace on the qualified Alpha version and run the ordinary CI plus a non-publishing Alpha release
   dry-run. This proves unsigned Alpha remains independent of private credentials.
2. Refresh `release/beta-readiness.json` against current issues and release evidence.
3. Advance every Rust workspace package and exact internal dependency to `0.7.0-beta.1`; regenerate the contract
   snapshots whose product version is intentionally variable.
4. Run the complete workflow manually from the protected rebuild branch. Manual dispatch must remain
   non-publishing even when all signing jobs succeed.
5. Download the assembled assets. Run `xtask release verify`, inspect both Apple evidence files and the Windows
   evidence file, verify native signatures on clean machines, and retain the run ID in the R11.2 note.
6. Regenerate Homebrew, Scoop, and WinGet candidates from those exact signed assets and run their official native
   validators plus clean install, upgrade, and uninstall tests.
7. Only after all evidence passes, create the annotated Beta tag at the qualified commit. A tag push is the sole
   publication path.
8. Download the public assets again, verify their GitHub attestations, preview then write
   `xtask release record-beta-qualification`, and commit that ledger evidence before choosing the feature-freeze
   baseline.

Never test credential availability by weakening `release/signing-policy.json`, changing a target to `signing: none`,
or publishing a locally signed replacement archive. A failed external service leaves the release unpublished.

## 5. Rotation and incident response

- Revoke an exposed App Store Connect API key immediately and replace all three Apple secret values as one reviewed
  change window.
- Revoke a compromised Developer ID certificate through Apple, create a replacement, and requalify both macOS
  targets. Previously signed releases follow Apple's revocation behavior and must be assessed separately.
- Remove the Entra federated credential or Artifact Signing profile-signer role immediately if the GitHub/Azure trust
  relationship is suspect. Revoke the certificate profile when Microsoft guidance requires it.
- Delete obsolete repository settings after rotation; never leave an old and new identity silently interchangeable.
- Treat any signer-subject change as a reviewed policy and documentation change. Update
  `WINDOWS_SIGNING_EXPECTED_SUBJECT` only after independently verifying the new Public Trust profile.

## Official references

- [Apple Developer ID certificates](https://developer.apple.com/help/account/certificates/create-developer-id-certificates/)
- [Apple App Store Connect API keys](https://developer.apple.com/documentation/appstoreconnectapi/creating-api-keys-for-app-store-connect-api)
- [Apple custom notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)
- [Microsoft Artifact Signing setup](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart)
- [Microsoft Artifact Signing roles](https://learn.microsoft.com/en-us/azure/artifact-signing/tutorial-assign-roles)
- [Microsoft GitHub OIDC authentication](https://learn.microsoft.com/en-us/azure/developer/github/connect-from-azure-openid-connect)
