# Typst template and final-preview execution plan

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

Status: secure system-viewer fallback implemented; native matrix execution remains pending.

- macOS: qualify the blob-backed PDF iframe in the packaged WKWebView app.
- Windows: qualify the same bytes in WebView2.
- Linux glibc and musl packaging: qualify WebKitGTK PDF support separately.
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

## Acceptance gates

- `cargo run -p xtask --locked -- desktop template-audit`
- focused renderer, store, application-facade, and desktop-command tests
- `cargo clippy` with warnings denied for affected crates
- Svelte type checking, interaction tests, accessibility guard, and production build
- native packaged-app preview smoke tests on macOS, Windows, Linux glibc, and Linux musl
- previewed PDF SHA-256 equals the exported PDF SHA-256 for every document kind
- no runtime Typst CLI, Node.js, system-font discovery, package download, or network dependency
