# Implementation Plan

## 1. Prepare the pull-request branch

- [ ] Fetch and inspect current `origin/main`; preserve unrelated local branches and user work.
- [ ] Create `fix/desktop-shell-workspace-migration` from current `origin/main`, set the Trellis
      branch/base/scope metadata, and commit the approved planning artifacts before product edits.
- [ ] Confirm `origin/main...HEAD` contains no prior closeout-only or unrelated commits.

Rollback point: stop before product editing if a clean branch cannot be prepared without rewriting
or discarding user work.

## 2. Refine the desktop shell

- [ ] Update the existing shell contract test first for a one-line brand, no local-storage footer,
      sidebar-owned appearance actions, no redundant main toolbar, and a fixed notification region.
- [ ] Remove the sidebar tagline, tighten its header, and replace the footer panel with the existing
      language, theme, and density actions.
- [ ] Remove the redundant sticky main toolbar while retaining page headings, document titles,
      native window behavior, and the skip link.
- [ ] Move concise local-storage/privacy copy into Settings beside version/about information.
- [ ] Move App-wide error/success feedback into one dismissible fixed popup while preserving retry,
      result navigation, severity, live-region semantics, and state clearing.
- [ ] Update English and Simplified Chinese keys without adding framework or implementation jargon.

Focused checks:

```console
pnpm --dir apps/canisend-desktop exec vitest run src/lib/i18n.test.ts src/lib/accessibility-contract.test.ts
pnpm --dir apps/canisend-desktop check
```

## 3. Normalize diagnostics and interaction coverage

- [ ] Rebuild System diagnostics from the shared Card plus existing Accordion primitives, preserving
      progressive disclosure and all states/actions.
- [ ] Update Playwright coverage for sidebar keyboard operation, persisted language/theme/density,
      non-layout-shifting popup feedback, dismissal/retry, diagnostics disclosure, and 200% Chinese
      layout.
- [ ] Run the application-shell accessibility spec and inspect one bounded pair of rendered states:
      English/light/comfortable at 1280×820 and Chinese/dark/compact at 960×680 with 200% text.
- [ ] Apply one batch of confirmed visual fixes, recheck once, then stop polishing.

Focused checks:

```console
pnpm --dir apps/canisend-desktop exec playwright test tests/visual/application-shell.a11y.spec.ts
```

## 4. Make pending schema migrations atomic

- [ ] Replace the repeated migration dispatch with the existing ordered SQL files executed under
      one immediate transaction; validate contiguous order and final schema version before commit.
- [ ] Add one focused regression that injects a valid pending migration followed by a failing one,
      asserts exact pre-upgrade schema/history/version after rollback, then proves a corrected retry.
- [ ] Retain the existing future-schema and incomplete-history no-mutation tests.

Focused checks:

```console
cargo test -p canisend-store --locked database::tests::migration_sequence_failure_rolls_back_entire_pending_chain_and_retries
cargo test -p canisend-store --locked database::tests::future_schema_and_incomplete_history_are_rejected_without_mutation
cargo fmt --all -- --check
cargo clippy -p canisend-store --all-targets --locked -- -D warnings
```

## 5. Qualify current v4 recovery boundaries

- [ ] Run the existing v4 recovery round trip, malformed/legacy backup refusal,
      occupied-destination preservation, current-schema reopen, and Store migration tests.
- [ ] Record exact commands/results and the honest no-schema-delta limitation in a body-free task
      evidence note; do not label this as cross-version release qualification.

Focused checks:

```console
cargo test -p canisend-app --locked workspace::tests::workspace_v4_recovery_surface_round_trips_and_rejects_legacy_backups
cargo test -p canisend-app --locked workspace::tests::malformed_backup_fails_without_creating_destination
cargo test -p canisend-app --locked workspace::tests::verified_backup_restores_atomically_and_rejects_conflicts
cargo test -p canisend-store --locked database::tests
```

## 6. Review and bind the frozen product change

- [ ] Run `trellis-check`, review the full diff for accidental churn, dead imports/styles, private
      data, generated output, and unsupported legacy migration exposure.
- [ ] Run the Impeccable detector once over the changed UI targets and resolve real findings.
- [ ] Commit the bounded product/test change with a Conventional Commit message.
- [ ] Add one exact feature-freeze exception for every changed nonautomatic path, bound to that
      unchanged product/test commit; use a commit-preserving merge method.

Rollback point: if the exception cannot bind the exact product commit and sorted paths, regenerate
it before running the source gate or pushing.

## 7. Run final candidate gates once

- [ ] Run:

```console
git diff --check origin/main...HEAD
pnpm --dir apps/canisend-desktop format:check
pnpm --dir apps/canisend-desktop test
pnpm --dir apps/canisend-desktop test:accessibility
pnpm --dir apps/canisend-desktop build
cargo fmt --all -- --check
cargo clippy -p canisend-store -p canisend-gui --all-targets --locked -- -D warnings
cargo test -p canisend-store --locked
cargo test -p canisend-app --locked workspace::tests
cargo run -p xtask --locked -- release check
```

- [ ] Build the unpublished macOS Design Preview from the clean exception-bound commit and verify
      its receipt, then smoke Workspace create/connect/reopen, check/backup/restore, both Packs,
      CLI, and v4 Skills without tagging or publishing.

## 8. Create, verify, and merge the pull request

- [ ] Push the branch and create an English PR description covering scope, evidence, migration
      limitation, exact freeze exception, risks, and rollback.
- [ ] Wait for all protected required checks; fix root causes and rerun only invalidated evidence.
- [ ] Merge through the protected commit-preserving path only when local, candidate, and CI gates
      pass; confirm no release, tag, or public artifact was created.
- [ ] Reconcile task evidence, update durable specs only for a newly learned invariant, commit
      closeout records, archive the task, and record the session.
