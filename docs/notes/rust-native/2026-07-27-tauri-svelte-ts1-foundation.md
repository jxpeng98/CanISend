# Tauri + Svelte migration TS1 foundation

Stage TS1 established a non-publishing replacement desktop application beside the current egui
release binary.

## Implemented boundary

- `apps/canisend-desktop` contains the Svelte 5, TypeScript, Vite, Tailwind CSS, and
  shadcn-svelte frontend.
- `crates/canisend-desktop` contains the macOS-first Tauri 2 runtime.
- shadcn-svelte button, card, badge, and separator components were installed from the official
  registry and remain reviewable checked-in source.
- All interface icons come from the maintained `@lucide/svelte` package.
- The Tauri application reuses the existing CanISend application icon; no replacement brand or UI
  icon was generated.
- Semantic light/dark tokens, English and Simplified Chinese navigation, comfortable/compact
  density, visible focus, 44 px controls, and reduced-motion behavior are present.
- `product_summary` and `run_doctor` are typed Tauri commands over `canisend-app`.
- Tauri capabilities are limited to `core:default`, and the CSP rejects external content,
  frames, and objects.

The frontend does not read or write workspace files. Browser preview mode does not simulate Rust
success data; product and diagnostic values are populated only by the real Tauri bridge.

## Verification

- `pnpm check`: 0 errors and 0 warnings.
- `pnpm test`: 4 tests passed.
- `pnpm build`: production frontend built in approximately 2–3 seconds.
- `cargo test -p canisend-desktop --locked`: 2 tests passed.
- `tauri build --no-bundle`: the exact production frontend was embedded and the optimized macOS
  executable completed successfully.

The first full optimized Tauri link took 3 minutes 37 seconds because it compiled the new runtime
and the existing embedded Typst/PDF stack from a cold release target. This is a release boundary,
not the ordinary Svelte edit loop.

The in-app visual check exercised a constrained desktop viewport, English and Simplified Chinese,
light and dark themes, and compact density. It found and corrected an explicit Vite `$lib` alias
gap plus an overly narrow hero layout. The final page had no horizontal overflow and no browser
console errors.

## Remaining boundary

The new navigation items and mutation actions remain intentionally disabled until their TS2–TS4
Rust commands exist. The public `canisend-gui` executable, packaging scripts, performance baseline,
and 35-operation parity manifest still refer to egui. They must not be switched until TS5.
