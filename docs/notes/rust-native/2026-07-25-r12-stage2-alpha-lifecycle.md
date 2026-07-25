# 2026-07-25 R12 Stage 2 Alpha lifecycle evidence

**Scope:** macOS arm64 GUI create/import/workflow/reopen, GUI-managed CLI lifecycle, accessibility,
app-bundle layout, ad-hoc signature, and final-byte integrity

**Product version:** `1.0.0-alpha.1`

**Source baseline:** `2339cdb` after the lifecycle, packaging, and accessibility commits described
here

**Latest qualification at:** `2026-07-25T15:02:09Z`

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

The ordinary focused run passed ten GUI tests. The underlying `canisend-app` suite passed thirteen
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
upgrade from the exact public `v0.7.0-rc.2` archive or verify a pre-upgrade workspace backup. That
separate qualification is recorded below.

## Exact public 0.7 upgrade and backup

`scripts/qualify_macos_public_07_upgrade.sh` adds a repeatable, nonpublishing Apple Silicon
qualification path. It accepts a complete downloaded `v0.7.0-rc.2` asset directory and a new
body-free evidence destination. It does not access the normal user HOME or any real workspace.

The final run was performed from clean qualification-tool commit `80d2706` and passed in 31.37
seconds after the release build. Before running the lifecycle, the script:

- verified all 14 files covered by the public `SHA256SUMS` with the repository release verifier;
- bound the Apple Silicon archive to both the release manifest and signing evidence;
- bound the extracted CLI to the signing-evidence binary digest;
- verified the extracted public CLI's strict ad-hoc signature; and
- observed exact version `0.7.0-rc.2`.

The immutable public inputs and local candidate were:

| Item | SHA-256 |
|---|---|
| Public release manifest | `b961b9d544b190bade4c76311c8e4c5e95b97a97a17ad28f9b7d7a80c95568a1` |
| Public Apple Silicon archive | `0fef54fac09e25915f41f8cfb6315c1ab506e7180db00e82ef47f0036a4cb18d` |
| Extracted public 0.7 CLI | `5925697e900d0c8828847b7a17a40e34a65985c2a96c799c4df94c6b73f4693d` |
| Ad-hoc-signed 1.0 Alpha candidate CLI | `3a1b1402a9ba046290c24aca76731a4265eef364829b0ff97b810c0ac6656c65` |

The native integration test used the exact old CLI to create a synthetic workspace and job, run
integrity checks, and create a verified pre-upgrade backup. It then proved:

1. GUI inspection reports the public CLI as an older migration candidate.
2. Replacement without the explicit replace decision fails without changing the old executable,
   workspace database, or backup manifest.
3. Explicit migration installs the current CLI and preserves the exact public executable.
4. The current CLI opens and checks the workspace while the backup remains byte-identical and
   verified.
5. Both the public 0.7 CLI and current 1.0 CLI restore the backup into separate new workspaces,
   each retaining the synthetic job.
6. The old CLI either accepts the unchanged schema or rejects a future schema without mutation.
7. GUI-managed uninstall restores the exact old executable digest.
8. The upgraded workspace, backup, and both restored workspaces remain present.

The generated evidence uses `canisend.alpha-public-upgrade-evidence/v1`, records only versions,
digests, target, source commit, timestamps, and boolean checks, and explicitly records that no
publication occurred. The repository verifier and local binary/signature checks passed. A separate
`gh attestation verify` attempt on this host could not initialize a Sigstore verifier, so this local
Alpha record does not claim a fresh GitHub attestation verification and is not clean-tag release
qualification.

To repeat the test:

```console
gh release download v0.7.0-rc.2 \
  --repo jxpeng98/CanISend \
  --dir /private/tmp/canisend-v0.7.0-rc.2-assets
./scripts/qualify_macos_public_07_upgrade.sh \
  /private/tmp/canisend-v0.7.0-rc.2-assets \
  /private/tmp/canisend-v0.7.0-to-v1.0.0-alpha.1.json
```

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

## macOS accessibility and input qualification

Commit `2339cdb` adds persisted appearance/accessibility preferences, explicit AccessKit
landmarks/headings/live regions, dialog initial focus, visible custom-control focus, a
reduced-motion mode, and 100%, 125%, 150%, and 200% text-size controls. The navigation rail is now
scrollable and focused controls explicitly scroll into view; this corrects a clipping/focus defect
found during the first 200% native run.

The final staged Apple Silicon candidate retained the v2 external-integrity layout and passed
independent verification before every native run:

| Item | Result |
|---|---|
| Companion manifest SHA-256 | `0a822d473de222fdad35ebdb6084fdbe5ea2b513b96026cd1c85aacba16178b2` |
| Final signed GUI SHA-256 | `666cad51b27be20b2bba1fa60630855b7fc18b005822c97221cefb523865f541` |
| Bundled CLI SHA-256 | `6334b0ec543a93c79d88912a99504bb615ee93c3a84f1708eee101e495f195c7` |
| GUI executable | 50,006,704 bytes |
| Bundled CLI | 48,524,720 bytes |
| App directory | 94 MiB |
| Window-system display scale | 2.00 physical pixels per point |

`scripts/smoke_macos_gui_accessibility.sh` runs the exact staged app under a disposable HOME. Its
final run completed normally in about 31 seconds and proved:

- the primary navigation and main content landmarks are named;
- visible section/page headings expose heading roles;
- Tab order is Workspace, Overview, Jobs, Workspaces, Command line, Diagnostics, Dark appearance,
  Compact density, Reduce motion, and Text size;
- Command-equals reaches exactly 200%, the focused off-screen Reduce motion control scrolls inside
  the window, and Command-0 restores 100%;
- Reduce motion can be enabled entirely by keyboard; and
- the exact app exits normally after the smoke.

The fast deterministic tests separately prove light/dark semantic contrast, a 2 px amber focus
stroke, zero animation/scroll-animation in reduced-motion mode, strict preference persistence, and
AccessKit Heading, polite Status, and assertive Alert semantics.

The manual native matrix additionally passed:

- visible amber focus at 100% and 200%, plus legible light and dark views;
- preference persistence across a normal quit/reopen;
- real macOS Simplified Chinese Pinyin composition, including a visible candidate window and a
  committed `无障碍测试` value, followed by restoration of the original British input source;
- an `Open` directory picker that selected a disposable workspace directory;
- an `Open` file picker that selected a bounded local Markdown job fixture;
- explicit private-read consent before importing that local source; and
- a healthy workspace integrity check with one referenced blob after import.

The native tree exposed the body-free completion text
`Completed: Imported text/markdown; charset=utf-8 source`. Programmatic AccessKit role/live-region
coverage is complete for this Alpha task. This local record does not claim a human auditory review
of every VoiceOver phrase; that manual spoken-output pass remains part of the exact clean-tag native
package review.

## Remaining Stage 2 work

- Measure repeatable cold-start time against a declared threshold.
- Resolve or explicitly defer all planned GUI parity entries.
- Freeze the update-response fixtures and final Alpha package names around the v2 bundle layout.
