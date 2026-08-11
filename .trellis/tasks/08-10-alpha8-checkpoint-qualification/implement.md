# Implementation plan: exact Alpha.8 qualification

## 1. Establish authority

- [x] Merge the current Trellis branch through protected CI.
- [x] Add `M3-ALPHA8-001` to the Master Roadmap and create its GitHub milestone/Issue.
- [x] Move Issue #70 and conditional Issue #68 to the Alpha.8 milestone; preserve Alpha.7 Issues.
- [x] Update this task with the exact Issue URL, milestone, dependencies, and owner.

## 2. Land the cohort-blocking product source

- [x] Reinspect PR #174 scope, head, and all required checks.
- [x] Mark ready and merge only after explicit authorization; record the merge commit.

## 3. Repair release authority

- [x] Branch from the updated protected `main`.
- [x] Fix canonical sequential-Alpha document replacement at the existing writer.
- [x] Generalize eligible-v4-Alpha readiness/transition checks while preserving exact binding.
- [x] Update `refresh_beta_readiness.sh` to validate the ledger's exact eligible Alpha.
- [x] Update active release docs and error wording without rewriting historical notes.
- [x] Add focused Alpha.8-positive, Alpha.6-negative, and mismatch-negative tests.
- [x] Run focused tests, `bash -n`, Alpha.8 dry-run, `git diff --check`, and the final source gate.
- [x] Commit, push, open a protected PR, and merge only after explicit authorization.

## 4. Apply the Alpha.8 source transition

- [x] From a clean updated branch, save and review the full dry-run plan.
- [x] Run `prepare-stage v1.0.0-alpha.8 --write` only after explicit authorization.
- [x] Verify the written paths/digests equal the preview; run the source gate once.
- [x] Commit only controlled stage files, push, and merge through protected CI.

## 5. Qualify and publish exact bytes

- [x] Freeze the exact merged source and request release-operator authorization.
- [x] Run the build-once candidate matrix and inspect every required target/artifact/integrity job.
- [x] Create the reviewed annotated tag and promote the same artifacts without recompilation.
- [x] Download and independently verify every public asset and update-channel response.

## 6. Rebind provider evidence

- [x] Run exact Alpha.8 Codex, Claude Code, Claude Desktop, and bounded MCP-host scenarios.
- [x] Commit the body-free provider record and validation note with exact resource/Pack digests.
- [x] Re-review date-bound dependency and Store→IO exceptions without moving their hard expiry.
- [x] Make release-policy tests derive current evidence paths and dates from machine authorities.
- [ ] Reconcile Roadmap/GitHub/public truth and mark `M3-ALPHA8-001` Verified.
