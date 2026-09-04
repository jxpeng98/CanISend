# Implementation plan: private Beta.2 native candidate

## Phase A — Repair and protect release truth

1. Add a focused active-release-truth regression for source-ahead Beta documents and implement the
   minimal validator guard.
2. Correct `RELEASE.md`, the active Roadmap status/checkpoints, and project-control horizon without
   changing machine release state.
3. Run formatting, the focused xtask test, strict xtask Clippy, and relevant documentation checks.
4. Commit the bounded release-blocker change, record its exact nonautomatic paths in a second
   feature-freeze exception commit, then run `release check` once on the final PR head.
5. Open an entry PR, wait for required CI, review its exact diff, and merge it.

## Phase B — Build and inspect one candidate

1. Resolve the entry merge commit from protected `main`; confirm Fast CI passed and that no
   `v1.0.0-beta.2` tag, Release, or candidate run exists.
2. Dispatch `.github/workflows/release.yml` on exact `main` with tag `v1.0.0-beta.2`, cache epoch
   `stage4-v1`, and `promote_existing_tag=false`.
3. Wait for every candidate job. Stop on any failure; do not tag, promote, finalize, or publish.
4. Resolve the complete release-assets artifact ID and download it into a new temporary directory.
5. Run existing `release verify-candidate` and `release verify` checks, then verify GitHub
   attestations and inspect exact target/signing/App/SBOM/manifest evidence.

## Phase C — Record evidence

1. Write one dated body-free candidate note with exact source, future tag, run, artifact, manifest,
   checksum, asset, signing-boundary, and verification identities.
2. Reconcile Roadmap, project-control, parent/current Trellis tasks, and release status language;
   leave the active Beta ledger pending and publication unauthorized.
3. Run `git diff --check`; prose/Trellis-only evidence changes do not repeat Rust tests.
4. Commit, create a protected evidence PR, wait for required CI, inspect, and merge it.
5. Confirm again that Beta.2 has no tag or Release, then archive the task and record the session.

## Stop conditions

- Stop before dispatch if protected `main` moved, entry CI failed, or release truth disagrees.
- Stop before evidence completion if any candidate job or independent artifact check fails.
- Stop before every tag, promotion, Release, package publication, qualification write, RC action,
  or synthetic cohort claim; each remains separately authorized work.
