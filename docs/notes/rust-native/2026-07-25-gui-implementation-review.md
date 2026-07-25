# 2026-07-25 macOS GUI implementation review

**Scope:** `canisend-app`, `canisend-gui`, workspace registry, terminal CLI bridge, update check,
and staged macOS `.app`

**Baseline:** `2339cdb` plus the evidence/documentation follow-up recorded here

**Release assessment:** usable Alpha vertical slice; not feature-complete and not an RC

## Implemented product surface

The current GUI is a real native adapter over `canisend-app`; it does not invoke CLI commands for
workspace, job, source, or workflow behavior.

Implemented:

- native macOS app shell with system light/dark selection and compact density;
- persistent, body-free workspace registry;
- workspace create, register, switch, status, integrity check, backup, and remove-from-list;
- job list, search, archived filter, create, detail, archive, and source metadata;
- supplied public URL and local Markdown/text/JSON/text-PDF intake with explicit consent;
- workflow start and ten-stage status/blocker display;
- body-free product diagnostics and embedded renderer self-check;
- exact bundled/sibling CanISend CLI discovery;
- CLI version inspection, install/migration/update, digest-protected uninstall, and rollback;
- manual allowlisted GitHub release check with no automatic download or execution; and
- macOS `.app` staging with version-matched CLI, hashes, notices, and ad-hoc integrity signing.

## Findings resolved in this review

### Accessibility and presentation

- Replaced the transparent central frame that rendered as near-black in dark mode.
- Added explicit high-contrast button foreground colors and adaptive positive, warning, error,
  information, and neutral status colors.
- Added a WCAG AA contrast regression test for semantic text and button pairs.
- Added explicit accessible information for custom clickable job rows.
- Associated visible form labels with title, institution, URL, workspace-name, and search fields.
- Added named Banner, Navigation, Main, Status, Heading, Alert, and live-region AccessKit semantics.
- Added initial focus for workspace, job, and source-import dialogs plus Escape dismissal.
- Added visible amber focus for custom navigation/job controls and focus-following navigation
  scrolling at high text scale.
- Added persistent 100–200% text size, light/dark appearance, compact density, and reduced motion.
- Made all page bodies vertically scrollable at the declared minimum window size.
- Truncated long diagnostic paths within their column while preserving the complete value in the
  hover text.
- Corrected the no-workspace health label from `Not checked` to `No workspace`.

### Interaction and recovery

- A disconnected or panicked background worker now clears the busy state and presents a recovery
  message instead of leaving the application permanently busy.
- Workspace switching, job opening, registry removal, filter refresh, and relevant form controls
  are disabled while a background mutation is running.
- Successful operation notices are no longer erased by the automatic job-list refresh that follows
  them.
- Successful registry persistence clears an earlier registry error.
- Archive and managed-CLI uninstall now require explicit confirmation and state their retention or
  restoration behavior.
- Form errors clear when the related value, consent, file, or directory changes.
- A dialog cannot be dismissed while its submitted background action is still running.
- Workspace aliases are validated before workspace creation so an invalid name cannot create an
  unregistered workspace as a side effect.

### Bounded local input

- The registry rejects non-regular/symlink files, files over 1 MiB, more than 256 entries,
  duplicate/non-absolute paths, invalid defaults, control characters, and aliases over 128 bytes.
- Registry output is size-checked before commit and failed atomic renames remove their temporary
  file.
- Local regression fixtures prove oversized/inconsistent registries fail closed and that removing
  a registry entry never deletes a workspace.

## Native evidence collected

- `cargo test -p canisend-gui --locked`: 10 passed.
- `cargo clippy -p canisend-gui --all-targets --locked -- -D warnings`: passed.
- `cargo run -p xtask --locked -- release check`: passed.
- Release CLI and GUI executables built successfully.
- A complete temporary `.app` was staged and passed strict ad-hoc signature verification.
- The packaged app launched on Apple Silicon.
- Light- and dark-mode review passed at the preferred window size.
- The app remained legible at the declared 800 × 600 content minimum.
- The repeatable native accessibility smoke passed exact Tab order, named landmarks/headings,
  keyboard-only reduced motion, 200% focus visibility, and Command-0 reset.
- macOS reported a 2.00 physical-pixels-per-point Retina scale.
- Simplified Chinese Pinyin composition displayed native candidates and committed
  `无障碍测试`; the original input source was restored.
- Native directory and file `Open` dialogs returned their selected disposable fixtures to the app.
- The minimum-size Command line page scrolled vertically and kept long paths inside the content
  column.
- No CLI install/update/uninstall action was invoked against the real user destination during this
  review.

## Remaining GUI gaps

These are planned product work, not hidden implementation:

1. Workspace restore and deterministic projection repair.
2. Workflow begin, complete, and scoped rerun actions.
3. Profile evidence, criteria, match, and application-plan confirmation.
4. Discovery source management and lead promotion.
5. Agent task preparation, private-input consent export, completion, and host-pack controls.
6. Document, review, package, render, and export screens.
7. Complete ordinary mutation coverage in the CLI-to-GUI parity manifest.
8. Human auditory review of exact VoiceOver phrases on the clean-tag Alpha package.
9. macOS Intel compilation/native qualification, final icon/bundle metadata, and exact Alpha
    release manifest/SBOM/provenance evidence.
10. GUI-level workspace restore/repair recovery tests.

## Recommendation

Keep the GUI classified as a `1.0.0-alpha.1` vertical slice. Stage 2 accessibility and
disposable-user CLI lifecycle checks are complete. Stage 2 still requires parity-scope decisions,
cold-start measurement, and package/update-fixture freezing; Alpha publication then requires exact
clean-tag package evidence and truthful release notes. Beta remains blocked on complete ordinary
CLI-to-GUI workflow coverage.

The later [Stage 2 Alpha lifecycle evidence](2026-07-25-r12-stage2-alpha-lifecycle.md) completes the
bounded GUI reopen, disposable CLI lifecycle, exact public `v0.7.0-rc.2` upgrade/verified-backup,
final-byte bundle-integrity, and deterministic AccessKit/keyboard/scale/IME/file-dialog items.
Human auditory VoiceOver review remains an exact-package task.
