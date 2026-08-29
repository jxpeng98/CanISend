# Beta.1 stage-transition design

## Boundary

```text
qualified Alpha.10 readiness + v2 freeze + stage policy
  -> repaired read-only preview
  -> exact clean-worktree transactional write
  -> Beta.1 source / beta-qualifying ledger
  -> protected Fast CI
```

This task changes source-stage metadata only. Candidate construction and publication remain in
`M4-CANDIDATE-001`.

## Root cause

`render_stage_transition` always updates Cargo/lock/desktop versions, the qualification ledger,
and the release-note heading. The remaining current-source projections live in
`insert_sequential_alpha_updates`, which is called only for Alpha→Alpha. Cross-stage writes
therefore omit ten existing projections. `check_active_release_truth_for_version` separately
requires README/RELEASE to match the source version and hard-codes Roadmap Alpha / `pre-beta`, so a
direct Beta write would create a source state that the final gate rejects.

## Smallest complete repair

1. Rename/split the existing helper into a common current-source projection update and a
   sequential-Alpha-only evidence reset.
2. Call the common update for every supported transition; call the reset only for Alpha→Alpha.
3. Derive active Roadmap stage/status expectations from `ReleaseStage` and the existing
   qualification-status helper.
4. Extend the existing transition/source-truth regressions rather than adding a fixture framework.

No new schema, command, dependency, abstraction layer, or migration is needed.

## Transaction and evidence flow

1. Commit the renderer and regression repair.
2. From that clean commit, generate the Beta.1 dry-run report.
3. Confirm readiness is still within 24 hours and run the existing name-only signing checks.
4. Run the same command with `--write`; its transaction owns only the reported files and rolls all
   replacements back on failure.
5. Update human Roadmap/Trellis current-state prose, then run the one final source gate.

The preview/write comparison uses `from`, `to`, `files`, and `preserved_history`. `mode` and
`writes_performed` are expected to differ.

## Preserved history

The qualified Alpha.10 readiness and v2 contract-freeze records, feedback snapshot, public tag,
provider evidence, Alpha candidate archive, and historical release records remain unchanged. Their
digests are captured before write and compared afterward.

## Rollback

Before protected merge, revert the bounded stage branch. After merge but before candidate build,
revert the stage-transition PR through the same protected path. Never rewrite Alpha.10 or any
published/historical evidence.
