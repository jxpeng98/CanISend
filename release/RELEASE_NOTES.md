# CanISend 0.7.0-alpha.1

## Highlights

CanISend 0.7 is a greenfield Rust-native release. It installs as a platform-specific executable and does not require
Python, Node.js, Java, a separately installed SQLite library, or a Typst command.

The release provides local-first job intake from user-supplied files, text PDFs, and public URLs; discovery imports;
evidence and criteria workflows; matching; application planning; structured drafting and review; readiness checks;
editable exports; and embedded PDF rendering. Codex, Claude, and custom hosts integrate through the versioned
`canisend.agent/v2` JSON protocol and generated agent packs. CanISend prepares application materials but never
submits an application.

## Compatibility

- This release line uses `canisend.workspace/v2`, `canisend.agent/v2`, and public schema major version 2.
- It does not migrate Python-era workspaces or preserve the `0.6.x` Python command tree.
- Rust-native workspace migrations are append-only. An older binary rejects a future schema without mutation.
- Supported archives cover macOS arm64, macOS Intel, Linux GNU x86_64, Linux musl x86_64, and Windows MSVC x86_64.

## Install and verify

Download the archive for one supported target together with `SHA256SUMS`, the release manifest, notices, and
stage-required signing evidence. Verify their checksums, GitHub build provenance, manifest identity, and platform
signature before extracting the executable. Follow the
[native release verification guide](https://github.com/jxpeng98/CanISend/blob/main/docs/guides/release-verification.md)
and reject any incomplete or mismatched release unit.

Stage-required artifacts use the free `community-build` signing tier: macOS is ad-hoc signed and Windows uses an
ephemeral self-signed Authenticode certificate. These signatures provide native integrity evidence but are not
publicly trusted publisher identities; Gatekeeper, Unknown Publisher, or SmartScreen warnings may still occur.

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
user fonts, OCR, GUI automation, portal automation, and Linux arm64 archives are outside the 0.7 release scope.

## Feedback and support

Report reproducible problems through the repository issue templates. Include only sanitized public diagnostic
fields, exact release/target identity, and reproduction steps. Never attach a workspace, backup, application package,
private advert/profile content, provider request, token, certificate, or credential. The 0.7 line has no service-level
agreement or long-term-support commitment; consult the support policy shipped with the repository for the current
version window.
