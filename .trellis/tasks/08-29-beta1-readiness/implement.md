# Lean Beta.1 readiness implementation

## 1. Update the machine contract

- [ ] Advance the readiness schema to v2 and replace the old Beta user-evidence validator with one
      small exact-field maintainer/cohort/provider validator.
- [ ] Extend `beta_readiness_contracts` in place with host-resource and four-Skill bindings; reuse
      those values in provider validation.
- [ ] Keep Alpha10 pending-state generation, freshness limits, and transition ordering unchanged.
- [ ] Replace the existing focused Beta-user-evidence regression in place with a readiness-v2
      acceptance/rejection regression. Do not create a fixture framework.

## 2. Update the existing refresh path

- [ ] Change `refresh_beta_readiness.sh` to accept body-free maintainer validation.
- [ ] Retain only public Issue number/state/labels and stop only for open
      `priority:P0` + `state:blocked` Issues.
- [ ] Generate canonical provider, cohort-zero, contract, maintainer, and nine-class blocker
      sections; validate before output or write.
- [ ] Preserve dry-run first and clean-worktree-only write behavior.

## 3. Align release guidance

- [ ] Update the stage-transition runbook, qualification-ledger guidance, Roadmap, project-control
      guide, and known limitations only where they still claim pre-Beta cohort or zero open Issues.
- [ ] Add one dated body-free maintainer-readiness note. Do not copy prompts, Issue bodies,
      application content, private paths, credentials, or tokens.
- [ ] Keep public Beta.1, cohort, RC, and Stable work pending.

## 4. Produce the exact readiness record

- [ ] Commit the validator/script/note changes so the worktree is clean.
- [ ] Build a temporary body-free maintainer-validation JSON from the reviewed note digest.
- [ ] Run the refresh dry run, inspect it, then run the clean-worktree `--write` path.
- [ ] Verify the written record and preview `prepare-stage v1.0.0-beta.1` without `--write`.
- [ ] Commit the exact readiness record separately so its timestamp and public Issue snapshot are
      auditable.

## 5. Minimum verification

- [ ] `git diff --check`
- [ ] `bash -n scripts/refresh_beta_readiness.sh`
- [ ] `cargo fmt --all -- --check`
- [ ] One focused readiness-v2 validator test
- [ ] `cargo clippy -p xtask --all-targets --locked -- -D warnings`
- [ ] One final `cargo run -p xtask --locked -- release check`
- [ ] Protected Fast CI on the exact PR head
- [ ] No local full workspace suite, native rebuild, desktop suite, Claude matrix, package-manager
      lifecycle, or extended assurance

## 6. Protected reconciliation

- [ ] Merge the bounded PR after Fast CI.
- [ ] Mark Issue #71 and `M4-READY-001` Verified; keep milestone 5 open.
- [ ] Archive this task and make `M4-FREEZE-001` / Issue #72 the next delivery task.
- [ ] Do not start Beta stage, candidate, publication, cohort, or feature freeze in this task.

## Stop conditions

- Stop if any readiness input contains private content, an unsafe path, an unknown field, a false
  user count, a stale digest, or a mismatched Alpha10/provider identity.
- Stop if an open P0 Issue is labeled `state:blocked`, a required blocker class is not clear, or
  the maintained evidence note cannot support the review.
- Stop if implementation would change product bytes, move Alpha10 identity, or weaken consent,
  recovery, privacy, path, or release-integrity controls.
