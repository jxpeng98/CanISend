# Beta.1 feature-freeze execution

## 1. Preparation PR

- [x] Extend the existing automatic-path predicate only for `.trellis/tasks/` and
      `.trellis/workspace/`.
- [x] Extend the existing feature-freeze policy test with positive record paths and negative
      script/spec/workflow paths.
- [x] Clarify Beta artifact source versus repository freeze baseline in release guidance and
      current Roadmap wording.
- [x] Keep the ledger and exception record in canonical planned state.
- [x] Update task/control metadata and move Issue #77 to In progress only after task activation.

## 2. Minimum preparation gate

- [x] `cargo fmt --all -- --check`
- [x] Existing focused feature-freeze policy regression
- [x] `cargo clippy -p xtask --all-targets --locked -- -D warnings`
- [x] One final `cargo run -p xtask --locked -- release check`
- [x] `git diff --check` and Trellis task validation
- [ ] Protected Fast CI on the exact preparation PR head
- [ ] Merge preparation before resolving the final baseline

## 3. Exact activation transaction

- [ ] Fast-forward `main` to the protected preparation merge and branch from it.
- [ ] Resolve the full HEAD through Git; never extrapolate an abbreviated hash.
- [ ] Run `activate-feature-freeze FULL_HEAD_COMMIT` dry-run.
- [ ] Run the same command with `--write` from the unchanged clean HEAD.
- [ ] Compare baseline, exact two paths, and before/after digests between reports.
- [ ] Inspect the two JSON records for matching frozen baseline, unchanged classes, and zero
      exceptions.
- [ ] Commit only the two activation records first.

## 4. Reconcile automatic release state

- [ ] Add one body-free dated activation evidence note.
- [ ] Update README, RELEASE, Roadmap, Trellis control, and current/parent task truth.
- [ ] Keep cohort, RC, Stable, package publication, and native lifecycle work pending.

## 5. Minimum activation gate

- [ ] One final `cargo run -p xtask --locked -- release check`
- [ ] `git diff --check` and Trellis task validation
- [ ] Protected Fast CI on the exact activation PR head
- [ ] Merge the activation PR after protected checks pass
- [ ] Mark Issue #77 Verified with exact reports, commit, PR, runs, and merge
- [ ] Make Issue #70 Ready and archive this task

## Stop conditions

- Stop on an unprotected/stale baseline, dirty write worktree, planned-state drift, unexpected
  path, digest mismatch, nonempty initial exception list, or changed Beta source identity.
- Stop if any step would publish externally, start RC.1, contact users, weaken executable Trellis
  controls, or repeat unrelated qualification matrices.
