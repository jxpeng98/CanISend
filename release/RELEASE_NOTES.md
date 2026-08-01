# CanISend 1.0.0-alpha.5

## Highlights

CanISend 1.0 combines a Svelte-based macOS desktop interface, standalone command-line application, and versioned
agent integration in one Rust-native product. This release completes the desktop transition from egui to
Tauri + Svelte while preserving the same typed Rust application facade, local workspace, CLI, and Agent v2
contracts. It installs without Python and does not require Python, Node.js, Java, a separately installed SQLite
library, or a Typst command.

The macOS GUI supports persistent English and Simplified Chinese interfaces, native accessibility names, 100–200%
text scaling, light/dark appearance, reduced motion, and the complete application workflow. Apple Silicon users can
install from the new read-only DMG with an Applications drag target or retain the portable ZIP distribution.

The desktop now shares one checked-in shadcn-svelte Nova component system across the shell and every product page.
Its restrained default radius, denser panels and spacing, clearer selected-tab surfaces, reusable page patterns,
and overflow-safe responsive text make more of each workflow visible without crowding the interface. Comfortable
and compact density now produce intentionally distinct layouts, while focus, keyboard, reduced-motion, bilingual,
and 200% text behavior remain covered by the browser and packaged-App accessibility matrices.

The desktop presents that workflow as one six-stage application journey with a shared workspace/application context,
restored job selection, one recommended next action, and body-free recent-action continuity. Agent next actions and
reviewed task results return to the exact criteria, profile, document, review, package, or render surface instead of
opening disconnected tabs.

Terminal settings now check the bundled/installed CLI automatically and expose the consent-gated Add to PATH action
when needed. First-run profile initialization uses a localized editable Markdown scaffold and persists through the
same revisioned application read model returned by the GUI and CLI. Optional in-App Agent turns can be cancelled by
their exact workspace/runtime/job scope without saving a partial response. Selecting a different workspace returns
the Agent screen to the recommended external-host flow and clears rendered conversation state from the previous
workspace. New and resumed Codex bridge turns explicitly enforce the read-only sandbox.

The product provides local-first job intake from user-supplied files, text PDFs, and public URLs; discovery imports;
evidence and criteria workflows; matching; application planning; structured drafting and review; readiness checks;
editable exports; and embedded PDF rendering. Codex, Claude, and custom hosts integrate through the versioned
`canisend.agent/v2` JSON protocol and generated agent packs. CanISend prepares application materials but never
submits an application.

External Codex/Claude handoff remains the recommended reasoning surface. A portable MCP adapter exposes thirteen
typed tools, including four approval-gated mutations; the optional in-App runtime bridge remains read-only. CanISend
does not embed a model provider or duplicate the host-owned conversation transcript. The App identifies the nine
read-only/preview tools separately from the four approval-gated writes, and a stale binding for a moved or deleted
workspace cannot prevent active workspaces from using their own host sessions. Runtime discovery reports only
observed executable/version evidence and never infers sign-in or host-tool configuration.

## Compatibility

- This release line uses `canisend.workspace/v2`, `canisend.agent/v2`, and public schema major version 2.
- It does not migrate Python-era workspaces or preserve the `0.6.x` Python command tree.
- Rust-native workspace migrations are append-only. An older binary rejects a future schema without mutation.
- The Apple Silicon macOS application bundles a version-matched CLI; standalone CLI archives
  cover the five declared targets. The desktop is distributed as a read-only DMG with an
  Applications drag target and as a portable ZIP; both contain the same ad-hoc-signed
  `CanISend.app` and external integrity manifest.

## Install and verify

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
