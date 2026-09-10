# CanISend 1.0.0-beta.6

Published to npm `next` for macOS ARM64. See the [release record](../RELEASE.md) for the
exact source, package checksum and CI verification.

Optional Auto approval covers routine work on one Application and selected Profile Sources
for up to 60 minutes. Manual requests combine their permissions in one form, and confirmation
content uses readable text with real paragraphs. New private Sources and exports still ask.
Evidence, revision, digest and single-use token checks remain enforced.

Shared Agent guidance and stage Skills are shorter, obsolete project-local Trellis injection
is removed, and five-Skill discovery is unchanged. Managed upgrades preserve user edits.
Agent/Workspace v4 and Pack identities are unchanged.

## Highlights

This testing release focuses on the standalone Rust CLI and versioned Agent integration.
The native executable does not require Python, Node.js, an external SQLite library or a Typst command;
the npm launcher requires Node.js. Desktop CI and full native qualification remain paused.

The clean `canisend.workspace/v4` authority is a neutral container. Academic and generic
Applications coexist in one Workspace, and each Application owns its exact Pack ID, version,
digest, revision, and stage state. Shared Profile Sources and Evidence require explicit
Application associations and consent; one Application cannot silently read or invalidate another.

The App can initialize a new v4 Workspace and optionally install integrity-managed Codex and
Claude Code Skills plus project-local MCP guidance. The same initialization, host
setup/status/remove, basic-data import/read, recovery, and MCP stdio entry point remain available
through the standalone CLI when the App is closed or absent.

`canisend.agent/v4` uses one canonical task-resource model and an
`orient -> propose -> preview -> approve -> commit -> verify` sequence. Guarded mutations bind
the exact Workspace, Application, Pack, revision, snapshot digest, preview digest, consent, and
single-use process-local token. Denial, expiry, replay, stale context, wrong Application, or wrong
Pack fails without mutation.

The two built-in Packs share connected URL, pasted-text, local-file, and text-PDF intake plus
Requirement, Plan, Deliverable, review, rendering, export, backup, and recovery rules through the
same Rust application facade. CanISend prepares evidence-bound materials but never submits an application.
It also never logs in or uploads on the user behalf.

## Compatibility

- This development line uses `canisend.workspace/v4`, `canisend.agent/v4`, Agent schema
  version `4.0.0`, and `canisend.agent-host-resources/v4`.
- Earlier Skills, Agent v2/v3 requests, job aliases, host-resource layouts, and
  Workspace v2/v3 files are unsupported. They fail before mutation with clean-v4 guidance.
- It does not migrate Python-era Workspaces or preserve the `0.6.x` Python command tree.
- Rust-native schema migrations are append-only. A binary rejects unsupported or future authority
  before mutation.
- This npm release contains only an Apple Silicon macOS CLI. The paused full release policy
  declares five CLI targets and an Apple Silicon desktop with a version-matched CLI in DMG/ZIP
  bundles; those artifacts require separate qualification and are not part of this npm release.

## Install and verify

For this npm testing release, follow the [CLI installation guide](../docs/guides/installation.md).
The archive and desktop verification requirements below apply when a full native release resumes.

Download the archive for one supported target together with `SHA256SUMS`, the release manifest, notices, and
stage-required signing evidence. Verify their checksums, GitHub build provenance, manifest identity, and platform
signature before extracting the executable. Follow the
[native release verification guide](https://github.com/jxpeng98/CanISend/blob/main/docs/guides/release-verification.md)
and reject any incomplete or mismatched release unit.

Apple Silicon GUI users must also download the matching macOS GUI qualification JSON. The DMG
record binds its exact image bytes, read-only mount, Applications link, companion hashes, and
nested plus outer ad-hoc signatures. The portable ZIP record additionally binds the packaged CLI
doctor, synthetic workflow, and GUI launch.

The release manifest includes Intel GUI compilation evidence only when required by the validated
release-stage policy. When it is omitted, a separate scheduled workflow checks that development
source still builds in release profile on the Intel runner, but its body-free record cannot
authorize publication or make a native runtime support claim. A required record binds
exact-candidate compile-only evidence without publishing an Intel GUI archive.

The macOS desktop uses ad-hoc signing. Signed standalone macOS and Windows archives use ad-hoc or
self-signed Authenticode integrity signatures when required by their release-stage policy. These
signatures are not publicly trusted publisher identities; Gatekeeper, Unknown Publisher, or
SmartScreen warnings may still occur.

After extraction, run `canisend version --json`, `canisend doctor --json`, and the
[documented quick-start](https://github.com/jxpeng98/CanISend/blob/main/docs/guides/quick-start.md) before using private
application data.

## Upgrade and rollback

Check and back up every important workspace before replacing a binary. Retain the previous verified archive and its
notices. If the new binary opens a workspace, do not roll back by merely reinstalling the old executable: restore the
pre-upgrade backup into a new directory and check it with the old binary. There is no in-place database downgrade.
Follow the complete
[upgrade, rollback, and uninstall guide](https://github.com/jxpeng98/CanISend/blob/main/docs/guides/upgrade-and-rollback.md).

## Security and privacy

CanISend enables no telemetry, analytics, crash upload, or background reporting by default. User confirmation remains
authoritative for evidence, criteria, application decisions, review dispositions, exports, and final use. Provider
requests require explicit consent; portal login, upload, and submission are outside the product boundary.

## Known limitations

Read `KNOWN_LIMITATIONS.md` in the release assets before using real data. Text-based PDFs are supported; scanned or
image-only PDFs require external OCR and user review. User-authored Typst, external Typst packages/files, system or
user fonts, OCR, GUI automation, portal automation, and Linux arm64 archives are outside the 1.0 release scope.

## Feedback and support

Report reproducible problems through the repository issue templates. Include only sanitized public diagnostic
fields, exact release/target identity, and reproduction steps. Never attach a workspace, backup, application package,
private advert/profile content, provider request, token, certificate, or credential. The 1.0 line has no
service-level agreement or long-term-support commitment; consult the support policy shipped with the repository for
the current version window.
