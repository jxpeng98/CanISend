# Tauri/Svelte desktop cutover

**Date:** 2026-07-28
**Stages:** TS3, TS4, TS5 source cutover

The desktop frontend now covers all 35 CLI/GUI operation families through typed Tauri commands
over `canisend-app`. Discovery and workflow previews remain Rust-owned, one-time, bounded
preview/commit transactions. Private reads, network access, provider sends, terminal installation,
and private exports retain separate explicit consent.

The Tauri runtime in `crates/canisend-desktop` now publishes the existing `canisend-gui` package
and binary name. The prior egui crate, eframe/rfd dependencies, renderer sources, embedded egui
font notices, and staging checks were removed. The CLI name, workspace format, Agent v2 protocol,
schemas, and macOS application layout did not change.

Svelte owns English and Simplified Chinese presentation, theme, density, reduced motion, and
100–200% text scaling. These body-free display preferences persist locally. The official Tauri
window-state plugin restores the native window, and the release CSP still allows only bundled
assets plus Tauri IPC.

Fast CI builds the locked production frontend once and passes the exact artifact to both macOS Rust
jobs. Release and scheduled Intel GUI builds explicitly install locked frontend dependencies and
build those assets before Cargo embeds them with the required `canisend-gui/custom-protocol`
feature. The source gate rejects a cutover ledger if the old crate/dependencies return, the
frontend-to-Rust build handoff disappears, or a GUI release build can silently fall back to
`devUrl`. Vite also uses the relative `./` production base required by Tauri's embedded custom
protocol; the source gate rejects a return to root-relative entry assets.

Focused verification for this cutover includes Svelte/TypeScript checks, Vitest transport tests,
the Rust application and Tauri adapter tests, strict Clippy, the 35-operation parity gate, and a
local ad-hoc-signed macOS package smoke. The exact Apple Silicon archive passed its integrity,
packaged CLI quick-start, host-agent, and GUI launch checks. The staged application also passed the
English/Simplified Chinese WebView accessibility smoke, including level-one heading semantics,
reduced motion, 200% text scale, and Command-0 reset. Five native readiness trials measured a
1,143.607 ms median and 1,162.858 ms maximum against the 2,000 ms budget. Publication remains
separately authorized.
