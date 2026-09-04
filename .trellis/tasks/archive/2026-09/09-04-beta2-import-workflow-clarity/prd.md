# Beta2 import workflow clarity

## Goal

Make local source intake understandable and consistent with the canonical v4 Application model.

## Requirements

- Accept one supported local file through native Tauri drag and drop anywhere the desktop UI
  already offers Profile or Application file intake; keep the file-picker path available.
- Allow `.typ` files in Application local-source selection because the owned IO adapter already
  supports bounded Typst text input.
- Explain truthfully that the original file stays in place, an integrity-checked internal copy is
  stored in the selected Workspace, and new records belong to Applications rather than legacy
  `jobs/` projections.
- Use the canonical Application collection and `application_count` in summary surfaces so a newly
  created Application appears immediately without creating a duplicate legacy Job.
- Keep English and Simplified Chinese copy concise, natural, and equivalent.
- Preserve existing consent, bounded parsing, and authoritative Store boundaries.

## Acceptance Criteria

- [x] Dropping one `.typ`, Markdown, text, JSON, or PDF file into Application local intake selects
      it; Profile intake accepts its existing supported formats.
- [x] Multiple-file drops are rejected with a localized, actionable error.
- [x] The selected Workspace and storage behavior are visible before import, and the created
      Application remains discoverable after leaving the Applications page.
- [x] Today and Workspace health show canonical Application counts, not legacy Job counts.
- [x] No new write path creates or mirrors a legacy Job record or `jobs/` projection.
- [x] Focused desktop tests, Svelte type checking, formatting, and the production UI build pass.

## Notes

- This is a lightweight cross-view repair. The existing Tauri event API and Store contracts are the
  design authority; no dependency or new persistence abstraction is needed.
- Verified with 89 desktop unit tests, 18 Playwright visual/accessibility tests, 32 `canisend-io`
  tests, Svelte diagnostics, formatting, Clippy, production UI build, and native host compilation.
- A clean macOS preview exposed and verified the fix for a Svelte context-update loop. The native
  flow then imported a `.typ` source, created a third Application, kept it selected outside the
  Applications page, and showed `3` in both Today and Workspace health.
