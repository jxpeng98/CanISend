# Exact Alpha.10 execution

## 1. Activate and recheck entry

- [x] After explicit plan approval, run `task.py start`, create the bounded release branch, and
      mark Issue #194 In progress without changing release facts.
- [x] Confirm clean protected `main`, PR #196/#197 merge identities, no Alpha.10 tag/release, and
      public Alpha.9 immutability.
- [x] Run `cargo run -p xtask --locked -- release status --json`; require hard consistency and no
      blocking drift.
- [x] Confirm at entry that all 23 dependency exceptions share `reviewed_on=2026-08-24` and
      `review_by=expires_on=2026-09-07` while the current UTC date is 2026-08-24.
- [ ] Recheck current UTC against all 23 dependency exceptions immediately before push and
      candidate dispatch; stop at or after the 2026-09-07 expiry unless reviewed again.
- [x] Keep #70 open for real users. Use #68 only for actual affected-scenario evidence.

## 2. Apply the controlled Alpha.10 transition

- [ ] Re-run `cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.10` from the clean
      branch and compare the full 30-path/digest plan with the reviewed preview.
- [ ] Obtain separate write authorization, then run the same command with final `--write`.
- [ ] Require exactly the 30 planned controlled paths and matching after-digests; reject unrelated
      product, workflow, history, provider, or Roadmap changes.
- [ ] Run `git diff --check` and one final
      `cargo run -p xtask --locked -- release check` on the branch head.
- [ ] Commit task control separately from the mechanical transition, push one protected metadata
      PR, and inspect all required checks.
- [ ] Obtain merge authorization, merge without bypassing protection, update local `main`, and
      record its exact merge as source `S`.

## 3. Build and independently inspect one candidate

- [ ] Reconfirm `S` is protected `main`, dependency authority is current, and the tag is absent.
- [ ] Obtain candidate-dispatch authorization and run the existing workflow from `main` with
      `tag=v1.0.0-alpha.10`, body-free `cache_epoch=alpha10-v1`, and
      `promote_existing_tag=false`.
- [ ] Require `release-identity`, `signing-readiness`, `source-gates`, Windows release tests, all
      five CLI archive jobs, Apple Silicon App archive/DMG, and `assemble-and-attest-release` to
      pass. Do not reproduce the native matrix locally.
- [ ] Record candidate run `C` and complete artifact `A`; download `A` before its 30-day expiry.
- [ ] Run
      `cargo run -p xtask --locked -- release verify-candidate v1.0.0-alpha.10 S ASSET_DIR`
      and verify every file's GitHub attestation against `S` and the release workflow.
- [ ] Inspect the v4 contract tuple, two Pack digests, starter/resource manifest, four Skills, MCP
      inventory, executable identities, checksums, SBOM, signing limitations, and artifact digest.

## 4. Qualify affected and external-host scenarios

- [ ] Run the existing exact archive/Agent v4 smoke evidence for the App-closed mixed-Pack
      lifecycle, project/global host scope, export, backup, restore, reopen, and fail-closed cases.
- [ ] Obtain explicit synthetic-provider and temporary-host-configuration authorization.
- [ ] Back up host state and run the canonical Codex CLI Generic, Claude Code Academic, Claude
      Desktop Generic, and bounded MCP-client scenarios against the extracted candidate CLI with
      the App closed.
- [ ] Require preview/cancel or the specifically reviewed affected outcome, zero unauthorized
      mutation, zero submission, final Workspace integrity, and byte-for-byte host restoration.
- [ ] Retain only body-free versions, digests, counts, states, and outcomes. Draft the dated note
      but do not claim a public checkpoint before promotion.
- [ ] Mark #68 Verified only if its own acceptance is met; otherwise leave it open with a bounded
      evidence link.

## 5. Tag, promote, and verify public bytes

- [ ] Present `S/C/A` plus candidate/host results and obtain separate annotated-tag/publication
      authorization.
- [ ] Create annotated `v1.0.0-alpha.10` at `S` and push it. Require the tag-triggered workflow to
      locate `C/A`, run `verify-candidate`, and report `recompiled_during_promotion: false`.
- [ ] Require all five draft CLI smokes and the Apple Silicon App ZIP/DMG smoke before the workflow
      publishes the prerelease; record promotion run `P`.
- [ ] Independently download every public asset into a fresh directory, run
      `xtask release verify`, verify all attestations against `S`, and compare each public digest
      with `A`.
- [ ] Confirm the annotated tag peels to `S`, the update response identifies Alpha.10 as a
      prerelease, and public Alpha.9 still resolves to its original bytes and source.

## 6. Record evidence and reconcile authorities

- [ ] Finalize the body-free dated host note and `release/provider-dogfood.json` with exact
      Alpha.10 `S/C/A`, public manifest, Pack/resource/Skill digests, host outcomes, consent, and
      note digest.
- [ ] Update the Roadmap, #194, milestone 10, and Trellis records to Verified only after public and
      provider identities agree. Rebind #70 to Alpha.10 without closing it or changing its real-user
      denominators.
- [ ] Run `git diff --check` and one final `release check`, then merge the evidence-only PR through
      protected CI after explicit authorization.
- [ ] Archive this child and mark the parent release-integration task complete only after the
      protected reconciliation merge and public GitHub state agree.

## Stop conditions

- Stop before the next gate if any authority, source, tag, target, digest, artifact, attestation,
  signature limitation, host restore, or UTC-bound policy differs.
- A failed candidate is never tagged. Promotion never recompiles. A published tag is never moved.
- Synthetic host evidence never satisfies #70, Beta readiness, or an invited-user claim.
