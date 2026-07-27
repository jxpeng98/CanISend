# Tauri + Svelte TS2 workspace and job slice

**Date:** 2026-07-27

**Status:** Complete in source; not a release authorization

**Public version:** unchanged at `1.0.0-alpha.3`

## Outcome

TS2 turns the Svelte shell into a functional local-first desktop adapter without changing the
published egui executable. Users can now:

- create, connect, select, and remove local workspace shortcuts;
- inspect workspace status and integrity, create verified backups, restore into a separate
  directory, and repair managed projections;
- create, list, inspect, and archive academic job applications; and
- attach a local PDF, Markdown, text, or JSON advert, or fetch an advert URL after explicit
  consent.

The interface supports English and Simplified Chinese, light and dark themes, comfortable and
compact density, empty/loading/success/integrity/error states, and retry controls only for errors
classified as retryable by the Rust application facade.

## Architecture changes

The workspace registry moved from `canisend-gui` into `canisend-app`. The egui application now
re-exports that shared implementation, so both desktop adapters use the same bounded JSON format,
path canonicalization, alias validation, atomic save, entry limit, and non-deleting removal
semantics.

`canisend-desktop` exposes typed Tauri commands for:

- workspace registry list/create/connect/select/remove;
- workspace status/check/backup/restore/repair;
- job list/create/show/archive; and
- local and URL job-source import.

Blocking file, database, PDF, and network work runs outside the webview thread. Application errors
are normalized to a stable `{ code, message, retryable }` envelope.

The official Tauri dialog plugin is locked at `2.7.2`. Its capability is restricted to
`dialog:allow-open`; the frontend has no general filesystem permission. Local file imports require
an explicit private-read confirmation, and URL imports require a separate network-fetch
confirmation before the Rust consent tokens can be constructed.

## Input boundary

Direct job-source intake accepts one advert at a time:

- PDF;
- Markdown or plain text;
- JSON treated as a textual advert; or
- a supported remote URL.

Structured CSV and versioned JSON discovery batches are deliberately not attached to one job.
They create reviewable opportunity leads and therefore remain with the TS3 Opportunities
preview/commit workflow. This preserves the existing distinction between direct job intake and
bulk discovery import.

## Verification

The focused TS2 gate passed:

- `pnpm check`: 0 errors and 0 warnings;
- `pnpm test`: 5 tests passed;
- `pnpm build`: production bundle built successfully;
- `cargo test -p canisend-app -p canisend-desktop -p canisend-gui --locked`:
  - `canisend-app`: 49 passed, 1 ignored network-owned test;
  - `canisend-desktop`: 4 passed;
  - `canisend-gui`: 43 passed;
- strict Clippy for `canisend-app`, `canisend-desktop`, and `canisend-gui`, including all targets:
  passed with warnings denied.

Local visual review covered the minimum macOS window width, English and Simplified Chinese, dark
mode, sidebar overflow, disabled browser-preview controls, semantic headings, and horizontal
overflow. The browser console reported no warnings or errors.

## Next boundary

TS3 should implement Opportunities first, retaining the existing reviewed preview/commit
semantics for structured CSV/JSON and network discovery adapters. It can then migrate profile,
workflow, task, and Agent v2 surfaces without changing the Alpha3 release executable.
