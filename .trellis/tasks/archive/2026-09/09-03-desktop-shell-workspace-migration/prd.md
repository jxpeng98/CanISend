# Refine desktop shell and harden Workspace migration

## Goal

Give the desktop App more usable primary-content space, make navigation and feedback visually
consistent, and provide evidence that the supported Workspace migration path preserves user data
and can recover safely. The result remains an unpublished test candidate.

## Background

- `App.svelte` currently renders the CanISend tagline in the sidebar header, a `Stored locally`
  panel in the sidebar footer, and language, theme, and density controls in the main header.
- The main header also carries the current-view label. Moving the appearance controls out of it
  allows the header to be removed or reduced without losing a control.
- App-wide success notices and bridge failures are currently inserted above each page's content.
  Page-local Alerts also carry durable health state, consent boundaries, and form validation.
- System diagnostics is an Accordion styled to resemble a card in `TodayView.svelte`, while the
  surrounding dashboard uses the shared Card pattern.
- Settings already owns appearance and version information. It does not currently explain the
  local-first storage boundary.
- The current product creates and opens `canisend.workspace/v4` Workspaces. The repository also
  retains a verified academic v2-to-v3 migration service, but the current desktop bridge rejects
  that retired legacy operation and no v3-to-v4 migration is exposed.
- SQLite schema migrations are append-only and transactional per migration. The v2-to-v3 semantic
  migration already requires an exact preview digest, creates a verified backup, rechecks state
  inside one immediate transaction, rolls back injected failures, and restores only to a new path.
- The user confirmed that this task covers current and future v4 SQLite schema upgrades, not an
  import path for retired v2/v3 Workspaces. This candidate has no new schema version, so it must not
  invent a migration solely for testing.

## Requirements

1. Remove the sidebar tagline and tighten the header so navigation begins higher.
2. Remove the `Stored locally` footer panel and add concise local-storage/privacy information to
   Settings alongside the existing version/about information.
3. Move language, light/dark appearance, and comfortable/compact density controls from the main
   header into the desktop sidebar without duplicating preference state or persistence.
4. Increase primary-content space by removing redundant shell chrome while retaining the current
   view's accessible name, keyboard skip link, mobile/responsive behavior, and drag-region needs.
5. Replace transient App-wide success/error banners above page content with a non-layout-shifting
   popup notification that preserves error severity, retry, optional affected-content navigation,
   keyboard access, and screen-reader announcement. Durable content, consent, health, and inline
   field validation remain in context.
6. Make System diagnostics use the same shared card surface, spacing, border, and action hierarchy
   as adjacent dashboard sections while preserving progressive disclosure and diagnostic state.
7. Audit other shell and Settings boxes touched by this work for the same shared Card/Item patterns;
   fix confirmed inconsistencies without redesigning unrelated feature pages.
8. Preserve fluent, concise English and Simplified Chinese labels for every moved or added control.
9. Make the complete pending SQLite migration sequence atomic so a later migration failure cannot
   leave an earlier migration committed. Preserve contiguous history and future-schema refusal.
10. Qualify the v4 upgrade boundary with the existing verified backup/restore-to-new-path flow,
    current-schema reopen, and negative cases for malformed backups, occupied destinations, retired
    storage, incomplete history, and newer schemas. Do not claim a cross-version migration when this
    candidate has no schema delta.
11. Reuse existing components, preference state, migration services, and test fixtures; do not add
    a notification or migration dependency when the installed UI primitives and Store APIs suffice.

## Acceptance Criteria

- [ ] The sidebar brand shows CanISend without a secondary description, and primary navigation is
      visibly closer to the top.
- [ ] `Stored locally` no longer appears in the sidebar; Settings shows the product version and a
      concise bilingual explanation that Workspace data stays local unless the user approves an
      operation that leaves the device.
- [ ] Language, theme, and density are keyboard-operable in the sidebar, persist exactly as before,
      expose clear accessible names, and remain usable at supported text scales.
- [ ] The main content no longer reserves horizontal or vertical header space for those controls,
      and no supported viewport or 100–200% text fixture overflows.
- [ ] Transient success and error feedback appears as a popup without shifting page content; retry,
      destination navigation, focus behavior, and live-region semantics are covered by tests.
- [ ] Inline consent, validation, health, and explanatory states remain attached to their owning
      controls or content rather than becoming interruptive dialogs.
- [ ] System diagnostics visually matches adjacent cards in light/dark and comfortable/compact
      modes while its disclosure and Run diagnostics behavior continue to work.
- [ ] Focused component/unit and Playwright accessibility/visual checks cover the revised shell,
      notification, bilingual labels, Settings information, and diagnostics surface.
- [ ] A failed migration in the middle of a pending sequence leaves `user_version`, migration
      history, and schema objects at the exact pre-upgrade state; a corrected retry then succeeds.
- [ ] Fresh v4 checks prove current-schema reopen, backup integrity, restore to a separate path,
      malformed/legacy backup refusal, occupied-destination preservation, incomplete-history
      refusal, and no mutation of a newer schema.
- [ ] The final diff contains no new UI dependency, no unsupported compatibility alias, and no
      public release, tag, or publication change.

## Out of Scope

- Replacing the existing visual identity or redesigning every product view.
- Turning durable guidance, consent boundaries, health summaries, or field errors into modal
  interruptions.
- Automatic in-place downgrade or overwriting an existing Workspace during recovery.
- Automatic backup creation during application startup; the documented pre-upgrade check and
  verified backup remain an explicit operator action.
- Claiming Beta-to-RC migration qualification before an exact release pair introduces a real
  schema or resource transition.
- Publishing a release, changing the product version, or claiming native release qualification.
