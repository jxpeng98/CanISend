# CanISend Tauri + Svelte UI migration roadmap

**Status:** Active — Stage TS2 complete; TS3 next

**Decision date:** 2026-07-27

**Authority:** [ADR-RN-0015](../../architecture/rust-native/decisions/0015-replace-egui-with-tauri-svelte.md)

## 1. Outcome

Replace the egui presentation layer with a modern, accessible desktop application while preserving
the complete Rust application facade, local-first data model, CLI behavior, Agent v2 protocol, and
packaged-binary user experience.

The migration is complete only when:

- all 35 operation families in `docs/contracts/cli-gui-parity-v1.json` are implemented through
  typed Tauri commands and Svelte views;
- English and Simplified Chinese remain supported;
- macOS keyboard, screen-reader, reduced-motion, dark-mode, and packaged-startup checks pass;
- the exact packaged application contains the version-matched CLI;
- the old egui code and dependencies are removed; and
- users still need no Python, Node.js, external browser runtime, or package manager.

## 2. Selected stack

The initial lock targets, current when this roadmap was accepted, are:

| Layer | Selection | Initial version |
|---|---|---:|
| Desktop runtime | Tauri | 2.11.5 |
| Tauri JavaScript API | `@tauri-apps/api` | 2.11.1 |
| Tauri CLI | `@tauri-apps/cli` | 2.11.4 |
| UI compiler | Svelte | 5.56.8 |
| Build tool | Vite | 8.1.5 |
| Component system | shadcn-svelte | 1.4.2 |
| Accessible primitives | Bits UI | 2.18.1 |
| Icons | `@lucide/svelte` | 1.27.0 |
| Styling | Tailwind CSS | 4.3.3 |

Dependency updates remain locked and reviewed; `latest` is never used in CI or release builds.

## 3. Design system

CanISend uses a calm, information-dense academic workspace rather than a marketing-page or
mobile-first visual pattern.

### Information architecture

1. **Today:** active applications, deadlines, blockers, and next actions.
2. **Opportunities:** saved jobs, discovery sources, leads, and source intake.
3. **Application workspace:** workflow, criteria and match, plan, documents, review, package, and
   render.
4. **Profile:** profile sources and evidence confirmation.
5. **Agent integration:** task preparation, export, completion, and host packs.
6. **Workspaces:** registry, health, backup, restore, and repair.
7. **Settings and diagnostics:** language, appearance, CLI lifecycle, updates, schemas, resources,
   and doctor output.

### Visual rules

- Use neutral paper and ink surfaces with an indigo action accent and semantic success/warning/error
  colors.
- Use native system typography; no runtime font download is permitted.
- Use a 4/8 px spacing scale, 12 px control radius, restrained borders, and limited elevation.
- Support comfortable and compact density without changing information hierarchy.
- Use Lucide icons only; navigation labels remain visible and icons never carry meaning alone.
- Use token-driven light and dark themes with WCAG AA text contrast.
- Use 150–220 ms state transitions and respect `prefers-reduced-motion`.
- Keep keyboard focus visible, touch/click targets at least 44 px, and destructive actions behind
  explicit confirmation.

## 4. Architecture

```text
Svelte views and shadcn-svelte components
                  │
         typed TypeScript client
                  │ invoke
         Tauri command boundary
                  │
          canisend-app facade
          ├── canisend-store
          ├── canisend-io
          ├── canisend-contracts
          └── canisend-resources
```

The Tauri boundary owns transport DTOs, input validation, async/blocking dispatch, capability
configuration, and error normalization. It does not reimplement business rules.

## 5. Ordered stages

### TS0 — Decision and migration contracts

- [x] Select Tauri, Svelte, shadcn-svelte/Bits UI, Tailwind, and Lucide.
- [x] Preserve `canisend-app` as the product authority.
- [x] Define side-by-side migration and atomic cutover.
- [x] Delay Beta until the new desktop reaches parity.

### TS1 — Foundation and read-only vertical slice

- [x] Add `apps/canisend-desktop` for Svelte/Vite sources.
- [x] Add `crates/canisend-desktop` for the Tauri runtime.
- [x] Establish semantic theme tokens, responsive desktop shell, bilingual navigation, dark mode,
  reduced motion, and compact density.
- [x] Add only registry-sourced shadcn-svelte components and Lucide icons.
- [x] Implement typed `product.summary` and `product.doctor` commands.
- [x] Add frontend type checks, unit tests, Rust command tests, and a non-publishing macOS build.

**Exit:** The new application launches offline, renders the modern shell, calls the Rust facade,
and passes focused checks without changing the public egui package.

### TS2 — Workspaces, jobs, and source intake

- [x] Move workspace registry ownership out of egui-specific code.
- [x] Implement workspace init/status/check/backup/restore/repair.
- [x] Implement job create/list/show/archive.
- [x] Implement direct job source intake for local PDF, JSON, Markdown, text, and URL inputs with
  explicit private-read or network-fetch consent.
- [x] Add empty, loading, integrity/stale, success, and recoverable-error states.

### TS3 — Discovery, profile, and workflow

- [ ] Implement discovery sources, structured CSV/JSON batch preview/import, network refresh, leads,
  suggestions, and promotion.
- [ ] Implement profile sources, private-read consent, evidence review, criteria, match, and plan.
- [ ] Implement workflow start/status/begin/complete/rerun.
- [ ] Implement task preparation/export/completion/cancel and Agent v2 context.

### TS4 — Documents and delivery

- [ ] Implement document workspace and acceptance.
- [ ] Implement review findings and disposition confirmation.
- [ ] Implement package checks, export, projection recovery, and render export.
- [ ] Implement schema/resource inspection and bounded catalog export.
- [ ] Implement CLI install/update/rollback/uninstall and manual update checks.

### TS5 — Parity, accessibility, and cutover

- [ ] Pass all 35 operation-family contract entries with no deferred rows.
- [ ] Add macOS VoiceOver semantics, keyboard traversal, file-dialog, drag/drop, and window-state
  qualification.
- [ ] Update startup measurement to wait for a stable accessible Svelte landmark.
- [ ] Replace the release binary and packaging scripts atomically.
- [ ] Remove egui, eframe, rfd, renderer code, old UI snapshots, and egui font notices.

### TS6 — Native qualification and Alpha checkpoint

- [ ] Run the fast source gate and exact packaged Apple Silicon application tests.
- [ ] Run Intel macOS compile evidence plus five-target CLI release qualification.
- [ ] Verify bundled assets, CSP, capabilities, ad-hoc signature, checksums, SBOM, and provenance.
- [ ] Publish only after an explicit version and release authorization.
- [ ] Refresh the Beta readiness baseline after public feedback.

## 6. Test strategy

The ordinary edit loop runs:

1. Svelte formatting, `svelte-check`, and focused Vitest tests;
2. focused Tauri command/DTO Rust tests;
3. affected `canisend-app` tests;
4. relevant Clippy targets; and
5. the release source check when contracts or packaging change.

The native release workflow owns WebView application packaging, ad-hoc signing, exact archive
smoke tests, accessibility launch evidence, and candidate-to-public byte continuity. Windows and
Linux UI packaging remain out of scope until macOS cutover is complete.

## 7. Immediate execution

Begin TS3 without changing the current Alpha3 release executable. Structured CSV/JSON discovery
batches belong to Opportunities because they create reviewable leads rather than a single job
source; keep their preview/commit boundary intact when migrating that screen. Do not start Beta
preparation while the parity manifest still describes the egui implementation.
