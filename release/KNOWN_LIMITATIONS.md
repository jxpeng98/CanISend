# CanISend 1.0 Alpha Release Limitations

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
- OCR, GUI automation, portal automation, and Linux arm64 packages are outside the `1.0` release scope.

## GUI boundaries

- `1.0.0-alpha.*` publishes the GUI for Apple Silicon macOS first. Alpha manifests contain no
  Intel GUI archive or compile-only release evidence. A separate scheduled Intel compile
  regression cannot authorize publication or support; Beta and later require exact-candidate
  compile-only evidence, while native Intel qualification and Windows/Linux GUI packages remain
  future work until explicitly supported.
- The GUI supports English and Simplified Chinese. CLI and Agent v4 structured contracts remain
  locale-neutral; terminal human-readable output is English in this Alpha.
- The GUI covers all 32 declared operation families and connects them through one persistent
  workspace/application header and six-stage journey. Some advanced candidate editors expose
  versioned JSON directly and therefore still require familiarity with the public schemas.
- Body-free desktop navigation memory contains the canonical workspace path, public job ID,
  active route, and latest action summary. It is local convenience state, not a transcript or
  authoritative workflow state, and clearing WebView storage removes it without changing a
  workspace.
- External Codex or Claude handoff is the primary Agent experience and requires the selected host
  to be installed/configured separately. The optional in-App runtime bridge remains read-only;
  host-only desktop plugins/connectors are not automatically inherited by a CLI process. Its
  response is shown after the bounded CLI turn completes rather than streamed token by token;
  the running local process can be cancelled without saving a partial response.
- Runtime discovery confirms only a bounded executable path and version probe. It does not verify
  sign-in, provider entitlement, MCP, search, skills, plugins, or connector availability; these
  remain owned by the selected host and are confirmed only when it runs.
- CanISend does not embed a model provider, duplicate host transcripts, or guarantee that every
  third-party host exposes the same search, plugin, connector, approval, or session behavior.

## Release trust boundary

`1.0.0-alpha.*` macOS GUI packages use ad-hoc integrity signatures. Verify `SHA256SUMS`, the GitHub
artifact attestation, the release tag, the ZIP or DMG GUI qualification record, and the included
notices.
Beta, release-candidate, and Stable community releases fail closed unless both standalone macOS
CLI executables pass ad-hoc signing verification and the Windows executable passes self-signed
Authenticode integrity verification. Their canonical signing evidence is bound to the final
archive hashes and included in the release manifest and checksum set.

Community signatures do not establish an operating-system-trusted publisher. The macOS executable has no Developer
ID certificate, secure timestamp, or Apple notarization, so Gatekeeper can warn or reject it. The Windows certificate
is ephemeral and self-signed with no public timestamp, so Windows can report Unknown Publisher, `NotTrusted`, or
`UnknownError`, and SmartScreen can warn. Its thumbprint is specific to one artifact. Verify `SHA256SUMS`, the exact
v2 signing evidence, and GitHub provenance before using the normal per-application approval UI.
Never disable an operating-system security control globally.

No telemetry is enabled or sent by default. Report reproducible problems through the repository issue templates;
remove private job or profile content before attaching diagnostics.
