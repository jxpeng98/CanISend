# Implementation plan: Windows completion and multi-platform verification

## Phase 0: Approval and execution entry

- [ ] Obtain explicit maintainer approval of this amended final planning summary.
- [ ] Run `task.py start` only after that approval; load `trellis-before-dev` and the relevant
      project-control, cross-layer, error-handling, and quality specs.
- [ ] Fetch `origin`, re-read the three exact refs, and stop if `origin/main` has new overlapping
      or nonautomatic changes.

## Phase 1: Continue the Windows branch without rewriting evidence

- [ ] Create the local tracking branch from `origin/fix/windows-desktop-validation`.
- [ ] Merge local `main@aaa4e98a` into it so both sibling evidence commits remain reachable.
- [ ] Confirm the merge changes only known automatic Trellis paths and validate both task
      directories.
- [ ] Commit this task's planning artifacts as automatic Trellis history and record exact refs.

Rollback point: before product commits, recreate the local branch from the remote checkpoint;
never amend the two published evidence commits.

## Phase 2: macOS release-integrity batch

- [ ] Add the smallest branch in the existing feature-freeze validator that detects a merge whose
      full tree equals a non-first parent tree.
- [ ] Skip duplicate merge accounting only for that content-preserving case.
- [ ] Extend the existing Git-history fixture with one valid tree-identical merge and one invalid
      merge containing independent content.
- [ ] Update `docs/release/feature-freeze.md` with the bounded rule.
- [ ] Run only the exact existing feature-freeze test containing those two cases. Defer formatting,
      Clippy, and the source gate to the completed macOS implementation boundary.

## Phase 3: macOS frontend batch

Complete the three related frontend repairs before running broader checks:

- [ ] Remove `listJobs`/`showJob` imports and active callers; derive selection from Application
      dossiers and the existing content catalog without fabricating legacy source records.
- [ ] Narrow affected context/navigation/Application-view props to fields already supplied by the
      dossier/job contracts.
- [ ] Pass backend `product.target_os` into Settings and add aligned English/Chinese Windows,
      macOS, and neutral PATH copy.
- [ ] Change only Playwright's web-server executable from nested `pnpm exec vite` to direct `vite`.
- [ ] Reuse the existing static/navigation tests and run one focused Vitest invocation after the
      frontend batch is coherent. Defer the full unit/build/accessibility suites to protected CI.

## Phase 4: macOS Rust CLI-install batch

- [ ] Gate the four Unix-only declarations/imports with `#[cfg(unix)]`.
- [ ] Add the Windows `MZ` prefix preflight before spawn.
- [ ] Replace unconditional stdout-thread join with deadline-aware standard-library channel
      delivery while retaining the output cap and child kill/reap behavior.
- [ ] Reuse the existing malformed candidate, version decision, no-downgrade, and
      replacement/restore coverage; run the single exact replacement/version-probe test invocation
      after the batch is coherent.
- [ ] Do not run `cargo-xwin` during implementation.

## Phase 5: close the macOS implementation stage

Run each non-test integration check once on the complete implementation, not after every slice:

- [ ] `cargo fmt --all -- --check`
- [ ] affected-package strict Clippy for `canisend-app` and `xtask`
- [ ] `pnpm --dir apps/canisend-desktop format:check`
- [ ] `pnpm --dir apps/canisend-desktop check`
- [ ] rerun only a focused regression invalidated by a later edit; do not start a full suite
- [ ] build and launch the macOS host once for an isolated Workspace v4 smoke covering selection,
      PATH copy, diagnostics, embedded resources, and PDF health; then remove the fixture

Commit the reviewed source in two auditable commits:

1. `fix(release): preserve freeze audit across protected merges`
2. `fix(windows): close desktop validation blockers`

Resolve both full SHAs and exact sorted nonautomatic paths, append two ordered
`release-blocker` entries, and commit only `release/feature-freeze-exceptions.json` as
`chore(release): record Windows blocker exceptions`. Do not amend or rebase recorded commits.

Finally run one `cargo run -p xtask --locked -- release check`. This is the only local Tier 2 source
gate for the completed branch.

## Phase 6: targeted native Windows closure

Reconnect the existing validated Windows checkout/runner; do not create a new user-visible task
without permission. On the exact branch head, run only Windows-owned checks:

- [ ] strict workspace Clippy with `-D warnings`;
- [ ] the exact malformed `.exe` and replacement/restore regression invocation;
- [ ] stock `pnpm test:accessibility` to prove the Windows Vite launch path;
- [ ] one Windows x64 release GUI build and clean Workspace v4 WebView2 smoke;
- [ ] English/Chinese Windows PATH copy plus real `HKCU\\Environment\\Path` configure/status with
      explicit test consent and cleanup;
- [ ] CLI version/doctor and MCP help from the rebuilt host;
- [ ] exact commit, host architecture, tool versions, results, and cleanup evidence without private
      paths or data.

Do not repeat portable full Rust/frontend suites on this host. If a targeted check fails, expand
only to the smallest owning sibling test needed for diagnosis. Unavailable or failing native
Windows evidence blocks merge.

## Phase 7: protected full-suite boundary and merge

- [ ] Fetch `origin/main` again; merge only automatic upstream drift and stop/replan on overlapping
      or nonautomatic drift.
- [ ] Push `fix/windows-desktop-validation`, open one PR to `main`, and review the complete diff,
      exception order, and absence of artifacts/secrets.
- [ ] Let the existing `desktop-ui`, `browser-keyboard-accessibility`, `core-linux`,
      `core-windows`, `macos-quality`, and `macos-tests` jobs run the full source suites once on the
      final PR merge ref. Do not duplicate those suites locally.
- [ ] Confirm the merge-ref tree equals the PR head tree and the bounded content-preserving merge
      rule passes the source gate.
- [ ] Merge through the configured protected path without squash, rebase, or commit rewriting.

Rollback point: before merge, correct only the failing batch and rerun only invalidated checks.
After merge, use a protected revert PR; never reset or force-push `main`.

## Phase 8: post-merge cross-platform boundary

On a clean local `main` updated to the exact protected merge commit:

- [ ] wait for and verify the required merge-commit Fast CI run; do not run a third local copy of
      the full suites;
- [ ] verify the merge tree and exception-bound source paths still pass the recorded integration
      contract;
- [ ] record Apple Silicon, Rust/Cargo, Node/pnpm, Clang, cargo-xwin, and Parallels availability.

If cargo-xwin needs uncached Microsoft SDK material, pause for explicit licence/download
authorization. Then run the single planned cross-build pass:

- [ ] `cargo xwin test --workspace --no-run --target x86_64-pc-windows-msvc --locked`
- [ ] `cargo xwin build --locked --release --target x86_64-pc-windows-msvc -p canisend-cli
      -p canisend-gui --features canisend-gui/custom-protocol`
- [ ] inspect each `.exe` with `file` and `llvm-readobj` when available;
- [ ] record exact paths, PE32+ x86-64 identity, sizes, and `shasum -a 256` values;
- [ ] do not execute Windows binaries on macOS or call compilation a runtime pass.

Dispatch the existing `desktop-platform-qualification` workflow on merged `main` and inspect its
Windows/Linux package/runtime evidence. Do not add a matrix, publish, use production signing, or
rerun the five-target release workflow.

## Phase 9: review and closure

- [ ] Run Trellis quality review against every acceptance criterion and inspect final Git/GitHub
      state.
- [ ] Record the PR, merge commit, CI/workflow runs, focused macOS evidence, cross-built paths and
      hashes, native Windows attribution, failures, and residual limits.
- [ ] Archive the Windows handoff only after its native criteria pass; archive this task after the
      post-merge cross-platform boundary passes.
- [ ] Commit closure records through a documentation-only protected PR if required; no product or
      release-authority bytes change in that closure.
