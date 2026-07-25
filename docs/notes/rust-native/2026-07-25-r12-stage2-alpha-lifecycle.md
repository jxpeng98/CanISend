# 2026-07-25 R12 Stage 2 Alpha lifecycle evidence

**Scope:** macOS arm64 GUI create/import/workflow/reopen, GUI-managed CLI lifecycle, app-bundle
layout, ad-hoc signature, and final-byte integrity

**Product version:** `1.0.0-alpha.1`

**Source baseline:** `6f246e1` plus the candidate changes described here

**Completed at:** `2026-07-25T11:40:07Z`

**Classification:** local Alpha evidence; not clean-tag release qualification

## Fast deterministic coverage

`canisend-gui` now has a bounded synthetic reopen regression that:

1. initializes a new workspace;
2. creates a synthetic Lecturer in Economics job;
3. imports a local Markdown fixture with explicit private-read consent;
4. starts the ten-stage workflow;
5. saves and drops the GUI workspace registry;
6. reloads the registry and its default workspace;
7. reloads the job, source metadata, and workflow; and
8. proves the registry JSON does not contain the synthetic private-body sentinel.

The ordinary focused run passed seven GUI tests. The underlying `canisend-app` suite passed thirteen
tests with the public-endpoint network test intentionally ignored.

## Native GUI workflow

A release CLI and GUI were staged as a macOS app under a disposable HOME and temporary workspace.
The app was driven through its named macOS Accessibility controls and native file dialogs.

Observed workflow:

- created `Stage 2 synthetic applications` in a new empty directory;
- created `Lecturer in Economics` at `Synthetic University`;
- selected and imported a bounded local Markdown fixture after explicit consent;
- displayed only source kind, content type, and retrieval time, not the fixture body;
- started the durable workflow;
- displayed Intake as Complete, Parse as Ready, the remaining dependency states, and body-free
  blocker descriptions;
- exited normally;
- reopened with the same isolated HOME;
- automatically reopened the registered workspace with one active job and two artifacts; and
- exposed the reopened job row to macOS Accessibility as
  `Open Lecturer in Economics at Synthetic University`, then restored the source and workflow
  timeline.

The host's active Chinese IME opened its candidate panel when raw English key events were injected.
The app remained responsive. The smoke continued through a paste event while preserving and
restoring the prior clipboard. This is useful IME evidence, but it is not a complete multilingual
composition qualification.

## GUI-managed CLI lifecycle

`scripts/smoke_macos_gui_cli_lifecycle.sh` builds the release CLI, creates two distinct ad-hoc
signed byte units of the same product version, and runs an ignored native integration test in a
disposable directory. The test passed in 33.43 seconds and proved:

- an executable reporting `0.7.0-rc.2` is detected as an older migration candidate;
- installation preserves that previous command for rollback;
- the installed real CLI reports `1.0.0-alpha.1`;
- a different signed build is detected and installed as an update;
- uninstall verifies the managed digest and restores the previous command; and
- a synthetic workspace and job remain present throughout installation, update, rollback, and
  uninstall.

This test uses a bounded executable fixture for the old 0.7 interface. It does not yet qualify an
upgrade from the exact public `v0.7.0-rc.2` archive or verify a pre-upgrade workspace backup.

## Packaging integrity correction

The first staged bundle exposed a manifest-ordering defect: `BUNDLE.json` recorded the GUI SHA-256
before the outer app was signed, but outer `codesign` changes the main executable. The recorded GUI
digest therefore did not match final bytes.

The corrected layout removes final-byte digests from the signed app's self-referential metadata:

- `Contents/Resources/BUNDLE.json` uses `canisend.macos-app-bundle/v2` and records layout, version,
  signing tier, and the external-manifest rule;
- `CanISend.app.manifest.json` sits beside the app and uses
  `canisend.macos-app-integrity/v1`;
- the companion records the final signed GUI, bundled CLI, `Info.plist`, and internal metadata
  SHA-256 digests; and
- `scripts/verify_macos_gui_app.sh` independently verifies exact paths, digests, version agreement,
  absence of symlinks, canonical internal metadata, and the deep strict ad-hoc signature.

The corrected staged app passed independent verification and reopened the synthetic workspace.
Its final measurements were:

| Item | Result |
|---|---:|
| App directory | 94 MiB |
| GUI executable | 50,265,856 bytes |
| Bundled CLI | 48,808,384 bytes |
| Signing | ad-hoc, not Developer ID, not notarized |

## Remaining Stage 2 work

- Run the exact public 0.7 CLI/archive upgrade and verified-workspace-backup lifecycle.
- Complete keyboard traversal, VoiceOver announcements, text scaling, high-DPI, reduced-motion,
  and file-dialog qualification.
- Measure repeatable cold-start time against a declared threshold.
- Resolve or explicitly defer all planned GUI parity entries.
- Freeze the update-response fixtures and final Alpha package names around the v2 bundle layout.
