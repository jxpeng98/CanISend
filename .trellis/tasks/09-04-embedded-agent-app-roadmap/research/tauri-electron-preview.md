# Tauri, Electron, and Preview Feasibility

Date: 2026-09-04

## Conclusion

Electron is feasible but not justified for the MVP. Keep Tauri, reuse the current exact-byte PDF
preview, and add PDF.js inside Tauri only if measured requirements need richer viewer controls.

## What the shells provide

Tauri's [architecture documentation](https://v2.tauri.app/concept/architecture/) describes a Rust
backend and HTML frontend rendered by the operating system webview. Its
[webview version reference](https://v2.tauri.app/reference/webview-versions/) maps Windows to
WebView2/Chromium, macOS to WKWebView, and Linux to WebKitGTK. Tauri supports bundled
[sidecar executables](https://v2.tauri.app/develop/sidecar/) and
[Rust-to-frontend channels](https://v2.tauri.app/develop/calling-frontend/) optimized for streaming,
so Codex App Server process management and live events do not require Electron.

Electron's [introduction](https://www.electronjs.org/docs/latest/) and
[process model](https://www.electronjs.org/docs/latest/tutorial/process-model) provide a bundled
Chromium/V8/Node main-renderer-preload architecture. This improves renderer consistency, but adds a
new IPC/preload boundary, a larger runtime, and an Electron update obligation.

Electron does not make preview implementation disappear. Its
[web embed guidance](https://www.electronjs.org/docs/latest/tutorial/web-embeds) recommends iframe,
`WebContentsView`, or avoiding embedded content and discourages the `<webview>` tag. The
[security checklist](https://www.electronjs.org/docs/latest/tutorial/security) requires remote or
untrusted content to remain isolated, sandboxed, free of Node integration, and protected by strict
IPC/navigation controls.

## Preview needs and smallest solution

| Need | MVP solution | Upgrade trigger |
| --- | --- | --- |
| Read draft text/Markdown | Sanitized Svelte rendering | Only if editing semantics require a specialist editor |
| Inspect structured changes | Native field diff | Only when a new domain type cannot be represented structurally |
| Inspect evidence/consent | Existing semantic components | Only after measured usability failure |
| Preview final PDF exactly | Existing verified bytes in iframe | PDF.js if search/thumbnails/zoom controls are required |
| Open outside the App | Existing system-viewer fallback | Retain even if PDF.js is added |
| Render arbitrary remote HTML | Out of scope | Separate threat model and product requirement |

The current exact PDF path is stronger than a generic Chromium print/preview claim because the App
shows the same verified bytes that it exports.

## Migration cost unique to this repository

A Tauri-to-Electron migration would require at least:

- a superseding architecture decision and release-boundary change;
- replacement of the current Tauri command/channel/raw-response bridge for 129 registered operation
  leaves and 123 frontend invoke call sites;
- a persistent Rust-process protocol or Node native binding around `canisend-app`;
- a preload API and per-message validation/capability policy;
- new dialogs, paths, window lifecycle, updater, packaging, signing, and native qualification;
- new accessibility and English/Chinese UI qualification across the supported targets; and
- retirement/migration of current Tauri tests and configuration.

That work may be worthwhile if consistent Chromium behavior becomes a proven product requirement. It
does not solve the present process-per-turn agent bridge or information-architecture problem.

## Objective reconsideration experiment

If richer preview becomes necessary:

1. write a fixture set and measurable viewer acceptance criteria;
2. test the current iframe on the supported targets;
3. add a bounded PDF.js prototype in the existing Svelte/Tauri surface;
4. record failures, package impact, memory, accessibility, and security results; and
5. only if it fails, build the same viewer slice in Electron and compare total migration cost.

Until that experiment produces contrary evidence, Tauri is the lower-risk architecture.
