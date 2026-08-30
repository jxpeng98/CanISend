# Beta.1 stage-transition implementation

## 1. Repair cross-stage projections

- [x] Generalize the existing current-source update helper for every supported stage transition.
- [x] Keep readiness/freeze/feedback reset exclusive to sequential Alpha iteration.
- [x] Make active release-truth stage/status expectations derive from the source version.
- [x] Extend existing regressions to prove cross-stage updates and preserved evidence.
- [x] Run format, focused regressions, and xtask Clippy.
- [x] Commit the repair so write mode can start from a clean worktree.

## 2. Review write preconditions

- [x] Recheck Issue #73 is Ready and no applicable P0 blocker exists.
- [x] Confirm readiness age is at most 24 hours; stop and refresh with a new body-free maintainer
      review if it expired.
- [x] Run `./scripts/audit_community_signing_configuration.sh` and
      `./scripts/check_signing_readiness.sh beta`.
- [x] Capture SHA-256 for readiness, freeze, feedback, and Alpha candidate history.
- [x] Run and inspect `release prepare-stage v1.0.0-beta.1`; confirm dry-run and a clean worktree.

## 3. Apply the reviewed plan

- [x] Run the identical command once with `--write`.
- [x] Compare preview/write `from`, `to`, `files`, and `preserved_history` exactly.
- [x] Confirm the preserved evidence digests did not change.
- [x] Inspect the generated diff; accept only the reviewed current-state file set.

## 4. Reconcile projections

- [x] Update Roadmap, RELEASE, Trellis project control, and task state to Beta staged /
      `beta-qualifying`; keep public Alpha.10 and candidate/publication/qualification pending.
- [x] Update the owning stage-transition code-spec with the cross-stage projection invariant.
- [x] Mark Issue #73 In progress when implementation starts; do not close it before protected CI.

## 5. Minimum verification

- [x] `git diff --check`
- [x] `cargo fmt --all -- --check`
- [x] Focused stage-transition/source-projection regressions
- [x] `cargo clippy -p xtask --all-targets --locked -- -D warnings`
- [x] One final `cargo run -p xtask --locked -- release check`
- [x] Protected Fast CI on the exact PR head
- [x] No local full workspace suite, native candidate, desktop/provider matrix, package lifecycle,
      or extended assurance

## 6. Protected reconciliation

- [x] Merge the bounded PR after Fast CI.
- [x] Mark Issue #73 / `M4-STAGE-001` Verified and keep milestone 5 open.
- [x] Archive this task and make `M4-CANDIDATE-001` / Issue #74 next.
- [x] Do not build, tag, dispatch, publish, qualify, or activate feature freeze in this task.

## Stop conditions

- Stop if readiness is stale, signing readiness fails, an applicable P0 blocker appears, or the
  Alpha.10/readiness/freeze identities disagree.
- Stop if preview/write file or digest sets differ, an unreviewed path changes, or preserved
  evidence changes.
- Stop if the final state claims Beta publication/qualification or would require weakening a
  release-integrity, privacy, consent, path, recovery, or history invariant.
