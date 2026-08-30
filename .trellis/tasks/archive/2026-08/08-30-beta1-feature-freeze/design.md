# Beta.1 feature-freeze activation design

## 1. Protected preparation followed by activation

Use a protected preparation sequence followed by one activation PR because the freeze baseline
must precede every post-freeze commit and GitHub creates a first-parent merge commit:

1. **Preparation PRs:** land the narrow Trellis record-path policy, its existing-test extension,
   guidance, and task controls while freeze state is still planned. Before the final preparation
   merge, make nonautomatic root `RELEASE.md` state-independent.
2. **Activation PR:** branch from the latest protected preparation merge, use that merge as the
   exact baseline, apply the existing two-file transaction, and reconcile only automatic
   documentation and Trellis record paths.

Combining nonautomatic preparation and activation in one PR would pass on the feature branch but
make the protected merge commit introduce unrecorded nonautomatic paths after the baseline.

## 2. Identity contract

- **Beta artifact source:** `6e1397b79031cad54e794ccdc9edca2153f23b3e`; immutable identity of
  the bytes published as `v1.0.0-beta.1`.
- **Freeze baseline:** the latest full protected preparation merge commit; immutable start of
  repository history enforcement for later 1.0 work.

Activation changes neither the Beta tag nor its artifact source. Both freeze records receive the
same repository baseline.

## 3. Minimum policy adjustment

Extend the existing shared `is_automatic_feature_freeze_path` predicate with two data-only path
prefixes:

- `.trellis/tasks/`
- `.trellis/workspace/`

Do not exempt `.trellis/` broadly. Scripts, workflow rules, specs, generated platform adapters,
and other control logic remain nonautomatic and require an exact `release-blocker` or
`release-evidence` exception after freeze. Extend the existing policy test for both allowed and
rejected Trellis paths.

Root `RELEASE.md` also remains nonautomatic. Its current-state wording must derive from machine
authorities so activation does not require a post-baseline policy-file edit.

## 4. Activation data flow

```text
protected preparation merge HEAD
  -> dry-run render
  -> review baseline + two paths + digest pairs
  -> clean-worktree --write from the same HEAD
  -> exact two JSON files
  -> activation commit
  -> automatic docs/Trellis reconciliation
  -> source gate + protected Fast CI
  -> protected merge
```

The existing renderer validates the full lowercase HEAD, qualified Beta state, canonical planned
ledger/exception state, and exact two-file output. No parallel writer or manual JSON editing is
introduced.

## 5. Failure and rollback

- Wrong, abbreviated, stale, or non-HEAD baseline: stop before render/write.
- Dirty write worktree: stop before mutation.
- Ledger/exception state drift or digest/report mismatch: stop and inspect; do not hand-edit.
- Preparation PR failure: revert only its bounded policy/docs/control merge while freeze is still
  planned.
- Activation PR failure before merge: revert the two generated files to their planned bytes.
- Activation failure after merge: revert the activation merge as a release-integrity event; never
  rewrite Beta.1, its public assets, or prior qualification evidence.
