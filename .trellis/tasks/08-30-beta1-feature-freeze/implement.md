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
- [x] Protected Fast CI on preparation PR #207 head
- [x] Merge preparation as `bd83617efd3f1513d0635721566a4ad895311626`

## 2a. Final policy amendment before baseline

- [x] Make `RELEASE.md` freeze status state-independent without changing release authority.
- [x] Document that root `RELEASE.md` is nonautomatic after freeze.
- [x] Reconcile the spec and task plan; do not change Rust or machine release state.
- [x] Run `git diff --check`, Trellis validation, and one final `release check`.
- [x] Pass protected Fast CI run `33289215836` on PR #208 head.
- [x] Merge the amendment as final baseline
      `acf25dc483643ca9be0210320775708da116b715`.

## 3. Exact activation transaction

- [x] Fast-forward `main` to the protected preparation merge and branch from it.
- [x] Resolve the full HEAD through Git; never extrapolate an abbreviated hash.
- [x] Run `activate-feature-freeze FULL_HEAD_COMMIT` dry-run.
- [x] Run the same command with `--write` from the unchanged clean HEAD.
- [x] Compare baseline, exact two paths, and before/after digests between reports.
- [x] Inspect the two JSON records for matching frozen baseline, unchanged classes, and zero
      exceptions.
- [x] Commit only the two activation records first.

## 4. Reconcile automatic release state

- [x] Add one body-free dated activation evidence note.
- [x] Update README, Roadmap, Trellis control, and current/parent task truth.
- [x] Do not update nonautomatic root `RELEASE.md` after the baseline.
- [x] Keep cohort, RC, Stable, package publication, and native lifecycle work pending.

## 5. Minimum activation gate

- [x] One final `cargo run -p xtask --locked -- release check`
- [x] `git diff --check` and Trellis task validation
- [ ] Protected Fast CI on the exact activation PR head
- [ ] Merge the activation PR after protected checks pass
- [ ] Mark Issue #77 Verified with exact reports, commit, PR, runs, and merge
- [ ] Make Issue #70 Ready and archive this task

## Stop conditions

- Stop on an unprotected/stale baseline, dirty write worktree, planned-state drift, unexpected
  path, digest mismatch, nonempty initial exception list, or changed Beta source identity.
- Stop if any step would publish externally, start RC.1, contact users, weaken executable Trellis
  controls, or repeat unrelated qualification matrices.
