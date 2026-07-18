# CanISend 0.7 Native Release Limitations

This release line is a greenfield Rust-native product generation. It does not read Python-era CanISend workspaces,
preserve the `0.6.x` command tree, or implement `canisend.agent/v1`.

## Input boundaries

- Text-based PDFs are supported. Image-only or scanned PDFs require OCR outside CanISend followed by user review;
  CanISend reports `pdf_text_unavailable` instead of silently inventing text.
- Public URL intake is user-invoked and accepts only bounded HTTP(S) responses after DNS and redirect validation.
- Discovery is adapter-based and user-invoked. CanISend is not an uncontrolled web crawler.

## Workflow boundaries

- CanISend prepares application materials but never creates accounts, fills portals, uploads files, or submits an
  application.
- Host agents and configured providers may propose bounded structured content. CanISend requires explicit consent
  before exporting private inputs or sending them to a provider.
- Human confirmation remains authoritative for criteria, evidence, application decisions, review dispositions, and
  final use of exported materials.

## Rendering boundaries

- The renderer compiles only CanISend's embedded template with escaped structured inputs and embedded fonts.
- User-authored Typst, external files, packages, bibliography/XML/YAML input, system fonts, and user fonts are not
  supported. Enabling any of these surfaces requires a new security review.
- OCR, GUI operation, portal automation, and Linux arm64 packages are outside the `0.7` release scope.

## Release trust boundary

`0.7.0-alpha.*` archives may be unsigned. Verify `SHA256SUMS`, the GitHub artifact attestation, the release tag, and
the included notices. Beta, release-candidate, and Stable community releases fail closed unless both macOS
executables pass ad-hoc signing verification and the Windows executable passes self-signed Authenticode integrity
verification. Their canonical signing evidence is bound to the final archive hashes and included in the release
manifest and checksum set.

Community signatures do not establish an operating-system-trusted publisher. The macOS executable has no Developer
ID certificate, secure timestamp, or Apple notarization, so Gatekeeper can warn or reject it. The Windows certificate
is ephemeral and self-signed with no public timestamp, so Windows can report Unknown Publisher, `NotTrusted`, or
`UnknownError`, and SmartScreen can warn. Its thumbprint is specific to one artifact. Verify `SHA256SUMS`, the exact
v2 signing evidence, and GitHub provenance before using the normal per-application approval UI. Never disable an
operating-system security control globally.

No telemetry is enabled or sent by default. Report reproducible problems through the repository issue templates;
remove private job or profile content before attaching diagnostics.
