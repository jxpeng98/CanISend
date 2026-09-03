# Design: desktop shell refinement and v4 migration safety

## Approach

Keep this as one task and one pull request because the UI work and migration guard jointly qualify
the same unpublished desktop test candidate. Reuse the existing shell state, shadcn-svelte
primitives, Store migration runner, and recovery APIs. Add no dependency and no legacy migration
surface.

## Boundaries

- `App.svelte` continues to own appearance preferences and transient App-wide action feedback.
- `SettingsView.svelte` owns durable product, version, local-storage, and accessibility settings.
- `TodayView.svelte` keeps diagnostics as a secondary, progressively disclosed dashboard action.
- Shared Card, Alert, Button, Accordion, Item, and Sidebar primitives remain the visual authority.
- `canisend-store::Database` owns append-only SQLite schema migration and atomicity.
- Existing App/Store backup and restore APIs remain the recovery boundary. Restore is always to a
  new destination; startup does not create an unsolicited backup.
- Retired Workspace v2/v3 formats remain unsupported by the current desktop/CLI v4 surface and fail
  before mutation.

## Desktop shell and feedback

1. Reduce the sidebar brand block to the existing icon and `CanISend` name, then let Workspace
   context and navigation begin immediately below it.
2. Replace the footer's local-storage badge with one compact appearance control group containing
   the existing language, theme, and density actions. The same state and local-storage preference
   object continue to drive both sidebar shortcuts and detailed Settings controls.
3. Remove the redundant sticky main toolbar. Native decorated windows retain their platform title
   bar; each page retains its visible `h1`, the document title retains the current view, and the
   existing skip link still targets `#main-content`.
4. Move the existing local-first label and a tightened, accurate bilingual description into the
   Settings appearance/about area beside the existing version and legal information.
5. Render `bridgeError` or `notice` in one fixed, dismissible viewport popup using the existing
   Alert and Button primitives. Errors keep retry; successful actions keep optional result
   navigation. The popup does not steal focus or auto-dismiss, uses `alert` for errors and `status`
   for success, and never participates in page layout.
6. Keep inline field validation, consent notices, health summaries, and durable explanatory Alerts
   in their owning views. They are content, not transient shell notifications.

## Diagnostics and surface consistency

Wrap the existing diagnostics Accordion in the shared Card structure and use Card header/content
spacing instead of a one-off card imitation. Preserve the same disclosure button, protocol and
platform values, live diagnostic summary, disabled/running states, and action. Review the adjacent
Today and Settings surfaces for one-off containers caused by this move, but do not restyle
unrelated views.

## SQLite migration transaction

Replace the repeated per-version transaction calls with one ordered migration table and one
immediate transaction covering every pending version:

1. reject a newer schema before configuration or mutation;
2. select only versions after the current `user_version` and require contiguous numbering;
3. execute each existing append-only SQL migration and record its `schema_migrations` row inside
   the same transaction;
4. verify the final `user_version` before committing once; and
5. roll back the entire pending sequence on SQL, capacity, lock, history, or version mismatch.

No migration SQL file or current schema version changes. A focused injected mid-sequence failure
proves that the first pending schema object, migration row, and `user_version` also roll back, then
that a corrected retry succeeds.

## Upgrade and recovery evidence

This candidate has no schema delta, so it cannot honestly qualify a Beta-to-RC data migration.
Fresh checks will instead prove the mechanisms the next real upgrade depends on:

- current v4 reopen and integrity check;
- verified v4 backup and restore to a new path;
- malformed/legacy backup and occupied-destination refusal without mutation;
- incomplete migration history and newer schema refusal without mutation; and
- full pending-sequence rollback and retry at the database owner.

Exact cross-version qualification remains with the release matrix when a real source/target pair
exists.

## Compatibility

- Workspace remains `canisend.workspace/v4`; database schema remains version 20.
- Appearance preference key and shape remain unchanged.
- No Tauri command, public operation, payload, Pack, Skill, or CLI surface changes.
- Existing bilingual strings are reused where accurate; only moved/new shell labels are tightened.

## Risks and rollback

- A fixed popup can cover content at high zoom. Constrain it to the viewport, allow wrapping and
  dismissal, and verify 100% and 200% text fixtures.
- Removing the toolbar changes keyboard order. Update the accessibility test to assert the sidebar
  controls and skip-link destination rather than the deleted toolbar order.
- A single migration transaction holds the writer lock for the whole pending sequence. Migration is
  startup-only and correctness outweighs partial progress; retain the bounded busy timeout and test
  rollback.
- Feature freeze requires the product/test commit to be bound by one exact, sorted exception before
  the source gate and protected PR merge.
- Roll back the UI and migration-runner commit together. Never downgrade a Workspace in place;
  restore its verified pre-upgrade backup to a separate path.
