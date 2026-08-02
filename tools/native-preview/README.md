# Native PDF preview qualification

This isolated tool verifies that the PDF Blob used by CanISend renders inside the native Tauri
WebView. It targets WKWebView on macOS, WebView2 on Windows, and WebKitGTK on Linux.

The test uses a deterministic black/white/blue PDF probe, captures the native window, and writes:

- `native-pdf-preview.png`, the native WebView screenshot;
- `native-pdf-preview.json`, the fixture hash, screenshot hash, WebView identity, dimensions, pixel
  ratios, and pass/fail status.
- `native-preview-host-size.json`, the SHA-256 and byte size of the production-equivalent and
  qualification-only hosts, including the test-only delta.

The scheduled matrix combines all three platform records into
`canisend.native-pdf-preview-matrix/v1`. A missing or failed direct renderer names the affected
platform and requires system-viewer review. It never adds PDF.js automatically.

The production boundary is deliberate. WebdriverIO has its own pnpm lock file here, and the native
server is compiled only when Cargo feature `preview-qualification` is explicitly enabled. Neither
side is present in a normal application build.

`@wdio/tauri-service` 1.2.0 imports `installMockSyncOverride` but declares native-utils 2.4.0,
whose published package does not export it. The isolated pnpm workspace therefore overrides only
that transitive package to native-utils 2.5.0, the first published compatible implementation.

## Local macOS run

Build the frontend and an isolated qualification host, then run the test with absolute paths:

```bash
pnpm --dir apps/canisend-desktop install --frozen-lockfile
pnpm --dir apps/canisend-desktop build
pnpm --dir tools/native-preview install --frozen-lockfile
CARGO_TARGET_DIR=/private/tmp/canisend-native-preview-target \
  cargo build --release --locked -p canisend-gui \
  --features custom-protocol,preview-qualification
CANISEND_NATIVE_PREVIEW_BINARY=/private/tmp/canisend-native-preview-target/release/canisend-gui \
CANISEND_NATIVE_PREVIEW_EVIDENCE=/private/tmp/canisend-native-preview-evidence \
WDIO_USE_NATIVE_FETCH=1 \
  pnpm --dir tools/native-preview test
```

The checked-in PDF probe is immutable test data. If its content intentionally changes, update the
pinned SHA-256 in the test and visually re-qualify all three native WebViews in the same change.

The matrix policy and its evidence summarizer have a dependency-free fast test:

```bash
pnpm --dir tools/native-preview test:policy
```
