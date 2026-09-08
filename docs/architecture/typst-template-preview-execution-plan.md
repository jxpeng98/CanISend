# Typst template and final-preview execution plan

**Status:** Implemented — exact native qualification is tracked by M2 of the
[CanISend 1.0 delivery roadmap](../superpowers/plans/2026-07-25-1.0-release-roadmap.md)

## Outcome

CanISend will render versioned, self-contained Typst template bundles inside the Rust process and
preview the exact validated PDF blob that is later exported. End users do not install Typst,
Node.js, fonts, or a package manager.

The rendering invariant is:

```text
authoritative document revision
  -> declared template resource/version/SHA-256
  -> managed Typst artifact
  -> validated PDF artifact
  -> same verified bytes for preview and export
```

## Phase 1: explicit default bundle and exact PDF preview

Status: implemented.

The default routing listed below was the Phase 1 baseline and is superseded by the pinned ModernPro
routing in Phase 2. The resources remain embedded as explicit fallback templates.

- Route `cover-letter` to `template.cover-letter`; route `cv`, `research-statement`, and
  `teaching-statement` to `template.application-document`.
- Give both templates the stable `canisend_render_document` entrypoint.
- Record bundle ID, resource ID, resource version, SHA-256, and entrypoint in every generated Typst
  projection and in the release template contract.
- Read preview bytes only from the current render manifest, behind private-read consent.
- Revalidate the stored PDF digest, byte count, page count, and PDF structure before returning it.
- Return PDF bytes through Tauri's raw binary response and create a session-only browser blob URL.
- Display the preview with existing Button, Card, Badge, and Tabs styling; revoke the blob URL on
  document/workspace changes and component destruction.
- Export through the same verified blob reader. An integration test compares preview bytes with the
  exported file byte-for-byte.

## Phase 2: import the real ModernPro bundle

Status: embedded routing and deterministic renderer coverage implemented; native visual baselines
remain in Phase 3.

- Pin the canonical Typst Universe archives:
  - `modernpro-cv` 2.0.0, archive SHA-256
    `1d108f538571e804f96b59dc1f3c0b0e0dc275b3eb35c6368fd7cc89775851f0`;
  - `modernpro-coverletter` 1.0.0, archive SHA-256
    `d3c5e8031e8a74ab4ae6e3163b0f37d6ecebc972dd7a4b3b41fc99ff07585130`.
- Vendor the official package source under the archive hash, apply the bounded
  `prefer-explicit-configuration` compatibility patch, and append a small
  `canisend_render_document(data)` adapter. The patch fixes the upstream `_first-filled` fallback
  order so the configured embedded font wins over unavailable PT Serif.
- Route `cv` to `template.modernpro-cv`. Route `cover-letter` to ModernPro `coverletter`, and route
  `research-statement` plus `teaching-statement` to ModernPro `statement` through
  `template.modernpro-coverletter`.
- Keep `template.application-document` and `template.cover-letter` embedded and contract-declared as
  fallbacks. They are not selected silently.
- Resolve the upstream `PT Serif`/`Libertinus Serif` fallback to the already embedded
  `Libertinus Serif` family. Optional contact icons remain data-driven, so the package core and
  CanISend adapters require no external icon package.
- Derive the visible letter date from immutable document-generation metadata unless an explicit
  resolved date exists. Repeated renders of the same source are byte-identical in focused tests.
- Record upstream archive URLs/hashes, embedded resource hashes, adapter revisions, font contracts,
  license, routing, and fallback coverage in `release/typst-template-contract.json`.
- Preserve the upstream MIT terms in `THIRD_PARTY_NOTICES.md` so macOS, Windows, Linux, and native CLI
  release bundles inherit the notice through their existing packaging paths.

## Phase 3: native preview qualification and viewer fallback

Status: secure system-viewer fallback and native qualification harness implemented; scheduled
macOS, Windows, and Linux execution is configured to produce reviewable evidence.

- macOS: qualify the blob-backed PDF iframe in the native WKWebView host.
- Windows: qualify the same bytes in WebView2.
- Linux glibc desktop candidate: qualify WebKitGTK PDF support separately.
- Exercise the actual platform WebView with a deterministic one-page PDF Blob. Qualification saves
  a screenshot and a JSON record binding the fixture and screenshot SHA-256 values, user agent,
  viewport, and black/white/blue render ratios.
- Compile the embedded WebDriver server only behind the `preview-qualification` Cargo feature and
  build it in an isolated CI target directory. Production and release builds continue to enable
  only `custom-protocol`; the WebDriver server and its Rust dependencies are absent from shipped
  binaries.
- Keep WebdriverIO and pixel-analysis dependencies in `tools/native-preview` with an independent
  pnpm lock file. Normal frontend installs and desktop release builds do not install this test
  package.
- Build and hash a production-equivalent host before enabling the qualification feature, then record
  the qualification-only byte delta. A matrix summary rejects missing platform evidence, mismatched
  fixture or screenshot hashes, threshold failures, and any production feature set containing the
  WebDriver instrumentation.
- Centralize native thresholds and target mappings in a versioned policy. If direct preview fails,
  the summary names the exact platform for system-viewer review and keeps PDF.js as a separate,
  evidence-driven decision.
- If a platform WebView cannot display PDFs, export/open the same validated PDF with the system
  viewer. Do not silently render HTML or switch templates.
- The desktop fallback requires explicit private-export consent, reuses the existing job-scoped
  render export, resolves the exact requested PDF inside the real workspace, and verifies its byte
  count and SHA-256 before launching the configured system handler. The frontend is not granted a
  general-purpose path opener.
- Use `open` 5.4.0 without its legacy `insecure` Windows feature so the fallback remains a small,
  cross-platform Rust dependency and launcher options stay separated from the validated path where
  the operating system supports that boundary.
- A local macOS arm64 `release` comparison measured the complete fallback at 60,529,536 bytes versus
  the pre-change 60,446,320-byte host: +83,216 bytes (+0.14%). Windows and Linux package deltas remain
  part of the scheduled native matrix rather than being inferred from macOS.
- Add PDF.js only if native qualification proves that direct in-App preview is unreliable. This
  keeps the default package small and avoids bundling a second PDF renderer prematurely.
- If Typst compilation fails, keep the structured source and managed `.typ`, report bounded
  diagnostics, and optionally offer the previous successful PDF clearly marked as stale.
- Windows and Linux desktop packages remain nonpublishing qualification candidates under the current
  support policy. The musl archive is CLI-only and has no WebView to qualify; its render/export
  invariants remain covered by the portable Rust tests.

## Acceptance gates

- `cargo run -p xtask --locked -- desktop template-audit`
- focused renderer, store, application-facade, and desktop-command tests
- `cargo clippy` with warnings denied for affected crates
- Svelte type checking, interaction tests, accessibility guard, and production build
- native WebView preview smoke tests on macOS, Windows, and Linux glibc; portable render/export tests
  for the CLI-only Linux musl archive
- previewed PDF SHA-256 equals the exported PDF SHA-256 for every document kind
- no runtime Typst CLI, Node.js, system-font discovery, package download, or network dependency

## CLI template synchronization — 2026-09-08

Scope: synchronize the owner's local ModernPro CV 2.1.1 and coverletter 1.0.2 sources, preserve
existing exact Pack bindings, and make the same update reproducible for future CLI builds.
The earlier Phase 2 archive identities above remain historical provenance.

- [x] Vendor the current source entrypoints byte-for-byte and retain the offline adapters.
- [x] Remove the superseded configuration patch; pin source commits, byte counts and SHA-256.
- [x] Advance the academic Pack to 1.0.1 and retain complete 1.0.0 bytes in the binary.
- [x] Verify synchronization, current rendering, historical exact resolution, and the source gate.

Before building a CLI release after changing the upstream templates, run:

```sh
python3 scripts/sync_typst_templates.py /path/to/Typst-CV-Resume /path/to/typst-coverletter
python3 scripts/sync_typst_templates.py /path/to/Typst-CV-Resume /path/to/typst-coverletter --check
cargo run -p xtask --locked -- source check
```

The synchronization command reads local sources without downloading packages. It updates template
bytes, resource versions, Pack resource hashes and digest, source pins, and the release template
contract and current package bindings together. Changed template bytes advance the Pack patch version and archive its previous
complete bundle. Repeating the command on identical inputs changes nothing. Review the diff and
run the focused renderer and historical registry tests before packaging the CLI.

Cargo embeds current templates and historical Packs in every new binary. Installing the upgraded
CLI therefore updates its available templates without a separate Typst installation or template
download. New academic Applications use the current Pack; existing Applications resolve their
original version and digest, preserving approved artifacts and exports. A binary upgrade does not
implicitly approve migration or rewrite existing Applications. User-exported template copies are
snapshots; export a fresh resource catalog when a current standalone copy is needed.

This source update does not publish a CLI release or claim an exact native candidate qualification.
Evidence:

- `python3 -m unittest discover -s scripts -p test_sync_typst_templates.py`: passed; upgrade,
  read-only drift detection, idempotence, retained history, invalid input, and modified-resource rejection.
- Synchronization `--check` against both local repositories: passed.
- `cargo test -p canisend-io --locked --lib render::tests`: 11 passed, including all four document
  kinds, deterministic PDF bytes, escaped inputs, and zero-warning current-template fixtures.
- `cargo test -p canisend-app --locked --lib workflow_pack::tests`: 7 passed, including original
  Application reopen without mutation, exact historical resources, and substituted-digest rejection.
- `cargo test -p xtask --locked --bin xtask typst_template_contract_matches_embedded_latest_templates`:
  passed, including missing source pin and version mismatch rejection.
- Affected-package Clippy with all targets and denied warnings, formatting, and `xtask source check`:
  passed. Source bindings were refreshed; historical release qualification records were preserved.
- Built the development CLI and copied it to an isolated temporary directory. `doctor --json`
  verified embedded rendering without downloads; `resource list --json` reported CV 2.1.1,
  coverletter 1.0.2, and academic Pack 1.0.1 with the expected resource hashes. This was a development
  binary smoke, not a release-profile size or native archive qualification.
- The complete resource test target passed 16/17. The unrelated existing
  `operation_v4_registry_projects_one_neutral_surface_for_every_host` semantic assertion also fails
  in an isolated archive of the unchanged HEAD; it is not claimed as passing.

Next: include these resources in the next authorized CLI candidate, address the existing operation
registry test failure separately, and run the candidate's normal packaged-binary qualification.
