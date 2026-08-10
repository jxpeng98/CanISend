# Implementation plan: exact Alpha.8 qualification

## 1. Establish authority

- [ ] Merge the current Trellis branch through protected CI.
- [ ] Add `M3-ALPHA8-001` to the Master Roadmap and create its GitHub milestone/Issue.
- [ ] Move Issue #70 and conditional Issue #68 to the Alpha.8 milestone; preserve Alpha.7 Issues.
- [ ] Update this task with the exact Issue URL, milestone, dependencies, and owner.

## 2. Land the cohort-blocking product source

- [ ] Reinspect PR #174 scope, head, and all required checks.
- [ ] Mark ready and merge only after explicit authorization; record the merge commit.

## 3. Repair release authority

- [ ] Branch from the updated protected `main`.
- [ ] Fix canonical sequential-Alpha document replacement at the existing writer.
- [ ] Generalize eligible-v4-Alpha readiness/transition checks while preserving exact binding.
- [ ] Update `refresh_beta_readiness.sh` to validate the ledger's exact eligible Alpha.
- [ ] Update active release docs and error wording without rewriting historical notes.
- [ ] Add focused Alpha.8-positive, Alpha.6-negative, and mismatch-negative tests.
- [ ] Run focused tests, `bash -n`, Alpha.8 dry-run, `git diff --check`, and the final source gate.
- [ ] Commit, push, open a protected PR, and merge only after explicit authorization.

## 4. Apply the Alpha.8 source transition

- [ ] From a clean updated branch, save and review the full dry-run plan.
- [ ] Run `prepare-stage v1.0.0-alpha.8 --write` only after explicit authorization.
- [ ] Verify the written paths/digests equal the preview; run the source gate once.
- [ ] Commit only controlled stage files, push, and merge through protected CI.

## 5. Qualify and publish exact bytes

- [ ] Freeze the exact merged source and request release-operator authorization.
- [ ] Run the build-once candidate matrix and inspect every required target/artifact/integrity job.
- [ ] Create the reviewed annotated tag and promote the same artifacts without recompilation.
- [ ] Download and independently verify every public asset and update-channel response.

## 6. Rebind provider evidence

- [ ] Run exact Alpha.8 Codex, Claude Code, Claude Desktop, and bounded MCP-host scenarios.
- [ ] Commit the body-free provider record and validation note with exact resource/Pack digests.
- [ ] Reconcile Roadmap/GitHub/public truth and mark `M3-ALPHA8-001` Verified.
