# Implementation plan: Windows desktop validation gaps

## Phase 1: Publish the Windows review checkpoint (current host)

1. Confirm `HEAD == origin/main == 0d11c456d726990ff9940f404a6aebad24cf72fc` and review the
   task-only diff.
2. Start the Trellis task, create `fix/windows-desktop-validation` from `main`, and record the
   branch and bounded cross-layer scope in `task.json`.
3. Include only the Trellis task artifacts, Windows review evidence, macOS handoff, and the
   developer journal created by Trellis initialization. Do not edit product source, tests, release
   metadata, or generated artifacts on Windows.
4. Run `task.py validate`, `git diff --check`, verify every changed path is under `.trellis/`, and
   review the staged diff.
5. Commit the review checkpoint, push it to `origin/fix/windows-desktop-validation`, leave the task
   open, and stop Windows execution.

## Phase 2: Resume the branch on macOS

1. Fetch `origin/fix/windows-desktop-validation` and switch to the remote branch without rebasing it
   onto an unreviewed baseline.
2. Read `prd.md`, `design.md`, `implement.md`, `research/windows-validation.md`, and
   `research/macos-handoff.md` before editing.
3. Confirm the required Rust/Node/pnpm versions and record the macOS architecture and exact branch
   head.
4. Load the applicable Trellis specs through `trellis-before-dev`, then begin product changes.

## Phase 3: Repair the active desktop read path on macOS

1. Remove `listJobs` and `showJob` imports/calls from `App.svelte`.
2. Project the active jobs and selection from `listApplicationDossiers` and refresh the selected
   object through `getApplicationDossier`.
3. Narrow affected selection types in the context bar and navigation helper to the supported
   dossier shape; do not fabricate missing legacy source records.
4. Extend the desktop contract test to prove that the app shell has no retired reads while the
   bridge retains fail-closed stubs.
5. Run the focused Vitest file plus Svelte checking.

## Phase 4: Correct platform PATH presentation

1. Pass the backend-reported target OS to `SettingsView`.
2. Add aligned English/Chinese Windows, macOS, and neutral PATH descriptions.
3. Render a non-shell-profile label for the Windows registry target.
4. Add a focused contract regression for the platform branch and both language keys.
5. Run frontend formatting, the focused test, and Svelte checking.

## Phase 5: Bound CLI probing and clean platform cfgs

1. Gate Unix-only imports/constants with `#[cfg(unix)]`.
2. Add the Windows executable-prefix preflight.
3. Make stdout result collection deadline-aware and preserve the output cap, process kill/reap,
   and read-only decision order.
4. Add a Windows malformed-executable regression and retain the replacement/restore regression;
   compile it on macOS where possible, but do not report it as executed without a Windows runner.
5. Run Rust formatting, the exact CLI install tests, all `canisend-app` tests, and strict Clippy for
   the affected crate before expanding to the workspace gate.

## Phase 6: Repair the accessibility harness

1. Change Playwright's web server command to invoke the package-local `vite` executable directly.
2. Run `pnpm test:accessibility` with no temporary config on macOS and confirm the baseline suite
   passes.

## Phase 7: macOS final verification

Run the smallest focused checks first, then one final source gate on the final diff:

1. `cargo fmt --all -- --check`
2. `pnpm format:check`
3. `pnpm check`
4. `pnpm test`
5. `pnpm build`
6. `pnpm test:accessibility`
7. runnable CLI probe/replacement tests and `cargo test -p canisend-app --locked`
8. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` on the native
   macOS target, plus Windows-target check/Clippy only if that target is configured
9. repository fast workspace CI commands identified from the workflow
10. `cargo run -p xtask --locked -- release check`

Rebuild the macOS release host and perform a focused native smoke:

- launch the GUI and open/create an isolated Workspace v4;
- confirm no `job.list`/`job.show` banner;
- exercise Applications selection/empty state;
- inspect macOS PATH guidance in English and Chinese and exercise static coverage for Windows copy;
- confirm Today diagnostics report the actual macOS architecture with verified resources/PDF;
- close the host and remove the isolated workspace/registry entry.

Do not run signing, notarization, packaging publication, or extended release qualification.

## Phase 8: Record Windows-only closure separately

The macOS run must leave these items pending until a native Windows/CI job proves them:

1. strict Windows workspace Clippy;
2. execution of the malformed `.exe` and replacement/restore regressions;
3. real `HKCU\\Environment\\Path` configuration behavior;
4. stock Playwright server launch on Windows;
5. rebuilt Windows WebView2 GUI smoke.

Cross-compilation or frontend unit coverage may reduce risk but does not replace native execution.

## Phase 9: Review, commit, and push the macOS continuation

1. Run the Trellis quality review against PRD requirements and inspect the complete diff.
2. Update task/spec knowledge only where a reusable contract was learned; do not copy transient
   command output into permanent product specs.
3. Present the exact commit plan for confirmation required by the Trellis workflow.
4. Commit the reviewed macOS changes, verify the worktree, and push
   `fix/windows-desktop-validation` to `origin`.
5. Record the final commit and remote branch in the task before wrapping up.
