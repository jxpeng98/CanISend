# Prepare the Beta.1 stage transition

## Goal

Move checked-in source from `1.0.0-alpha.10` / `pre-beta` to
`1.0.0-beta.1` / `beta-qualifying` through the existing dry-run-first transition command, while
preserving exact Alpha.10 readiness/freeze evidence and making no candidate, tag, publication, or
qualification claim.

## Confirmed facts

- `M4-FREEZE-001` is Verified through PR #202 and protected merge
  `5e1eb9f3ea16020c49bc10ef9777a268fb342528`.
- The current dry-run is body-free and non-mutating. It reports Alpha.10→Beta.1, 18 controlled
  files, and preserves readiness, contract freeze, feedback, and Alpha candidate history.
- The readiness audit is valid for at most 24 hours from `2026-08-29T21:32:54Z`; write mode also
  requires a clean worktree.
- Repository inspection found cross-stage projection drift: current-source updates are confined to
  the sequential-Alpha helper, while the release truth checker hard-codes Alpha / `pre-beta`.

## Requirements

### R1 — Repair the shared cross-stage projection path

- Reuse the existing sequential-Alpha source-version replacements for every stage transition.
- Keep only readiness/freeze/feedback invalidation conditional on Alpha→next Alpha.
- Cover README, RELEASE, native preview, desktop fallback, parity, performance baseline, Issue
  template, release workflow default, known limitations, and the package contract.
- Add no second transition engine, dependency, compatibility layer, or writer command.

### R2 — Make active release truth stage-aware

- Derive the expected Roadmap machine-stage/status and current candidate identity from the source
  version instead of requiring Alpha / `pre-beta` unconditionally.
- Preserve the public checkpoint as Alpha.10 until exact Beta.1 bytes are later published and
  independently qualified.

### R3 — Apply one reviewed transaction

- Commit the renderer/regression repair before write mode so the worktree is clean.
- Re-run the preview, inspect every controlled digest, and verify readiness freshness and signing
  configuration immediately before write.
- Apply the identical target once with `--write`; compare its file list and before/after digests to
  the reviewed preview.
- Preserve `release/beta-readiness.json`, `release/beta-contract-freeze.json`,
  `release/feedback-snapshot.json`, and the checked-in `packaging/candidates` history
  byte-for-byte.

### R4 — Reconcile only current-state projections

- Set Roadmap, RELEASE, Trellis, and GitHub to Beta staged / `beta-qualifying`, with the Beta.1
  candidate as next and publication/qualification/feature freeze still pending.
- Keep Issue #74 and all candidate/native work separate.

### R5 — Use minimum-sufficient verification

- One focused cross-stage regression, Rust format, affected xtask Clippy, signing-readiness checks,
  one final `release check`, and protected Fast CI.
- Do not run a local full workspace suite, native build, desktop suite, provider-host matrix,
  package-manager lifecycle, or extended assurance.

## Acceptance criteria

- [ ] Alpha→Beta preview includes every current-source projection and performs no write.
- [ ] Sequential Alpha still invalidates pending evidence; cross-stage transitions preserve the
      qualified Alpha.10 readiness/freeze/feedback bytes.
- [ ] Wrong target stage/iteration, stale readiness, dirty write, or stale/incomplete freeze fails
      before mutation.
- [ ] Write output matches the reviewed preview file/digest set apart from mode/write markers.
- [ ] Source is `1.0.0-beta.1`; ledger is Beta / `beta-qualifying`; release-note heading and all
      current-source projections agree.
- [ ] Public checkpoint remains `v1.0.0-alpha.10`; Beta is explicitly staged but not built,
      published, qualified, or frozen.
- [ ] Focused checks, final source gate, and protected Fast CI pass on the exact PR head.

## Out of scope

- Beta candidate build, native signing/integrity matrix, tag, promotion, publication, independent
  download verification, qualification ledger record, package channels, feature-freeze activation,
  cohort evidence, RC, Stable, or any product behavior change.

## Blocking open questions

None. The Roadmap, Issue #73, qualified authorities, transition policy, and observed preview define
the required outcome and safety boundary.
