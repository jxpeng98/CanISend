# Implementation plan: exact Alpha.9 qualification

## 1. Activate and freeze the entry inventory

- [x] Obtain planning-review approval, run `task.py start`, and persist the in-progress task state.
- [x] Reconcile the Master Roadmap, Issue #183, milestone, dependencies, and current protected
      `main`; record any accepted changed-byte P0/P1 blocker before source freeze.
- [x] Recheck date-bound dependency/signing authorities and confirm no Alpha.9 tag exists.
- [x] Keep the task limited to release qualification; defer feature requests and Issue #70 cohort
      evidence.

## 2. Apply the controlled Alpha.9 transition

- [x] From a clean updated branch, run the read-only command below and save/review the complete
      30-file digest plan:
      `cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.9`
- [x] Obtain separate explicit authorization for write mode.
- [x] Run `cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.9 --write`.
- [x] Compare every written path and after-digest with the reviewed preview; reject extra changes.
- [x] Run `git diff --check` and
      `cargo run -p xtask --locked -- release check` once on the final transition head.

## 3. Merge and freeze the exact source

- [x] Commit only task-control and controlled transition changes with auditable Conventional
      Commits.
- [x] Push and open a protected PR; inspect required checks and changed paths.
- [x] Obtain separate merge authorization, merge without bypassing protection, and record exact
      merge commit `S`.
- [x] Confirm the Alpha.9 tag is absent and `S` is still protected `main` before candidate dispatch.

## 4. Build once and qualify the candidate

- [x] Obtain separate candidate-dispatch authorization.
- [x] Dispatch `.github/workflows/release.yml` from exact `S` with
      `tag=v1.0.0-alpha.9`, a reviewed body-free cache epoch, and
      `promote_existing_tag=false`.
- [x] Require all source, five-target CLI, supported macOS App, archive, Agent/MCP, accessibility,
      migration, recovery, signing-readiness, assembly, and provenance jobs to pass.
- [x] Download the candidate bundle and record run `C`, artifact `A`, source `S`, manifest,
      checksums, Pack/resource digests, SBOM, signing, and provenance identities.
- [x] Confirm packaged smokes cover both Packs, render/export, backup/restore, repair, stale
      revision, and failure-atomic cleanup; do not add duplicate tests.

The first candidate (`31600099628`) passed its automated matrix but was rejected during manual
artifact inspection because its active manifest and SBOM declared legacy v2 contracts. It cannot
be promoted. The changed-byte repair must merge through protected CI and obtain a separate
replacement-candidate authorization before this section is considered qualification evidence.

- [x] Merge the reviewed metadata repair through protected CI and freeze replacement source `S2`.
- [x] Obtain separate authorization and dispatch one replacement candidate from `S2`; record
      replacement run `C2` and artifact `A2`.
- [x] Verify the corrected v4 manifest/SBOM tuple, checksums, signatures, and provenance before
      requesting tag or promotion authorization.

PR #187 merged through protected CI as `S2`
`4876c5669b7ae48ca053b5e06e0005419d2051f6`. Replacement candidate `C2` is successful run
`31609344160`; its complete release artifact `A2` is `9147597003` with GitHub digest
`sha256:da3c6a5c0aab4cc7f41c2fb1a33fc3c2769232ed74d0333e73f0a33cd5d489d9`. Independent download
verification passed all 15 checksum-listed files, the active manifest and SBOM contain the exact
v4 tuple, and signed provenance binds all 16 local files to `S2` and the release workflow with no
digest mismatch. Tagging and promotion remain separately authorized.

## 5. Promote the same bytes and verify them publicly

- [ ] Present the exact candidate evidence and obtain separate authorization for the annotated
      tag and publication transition.
- [ ] Create and push annotated `v1.0.0-alpha.9` at `S`; require promotion to locate `C/A` and
      rebuild nothing.
- [ ] Inspect draft/native verification before publication and record promotion run `P`.
- [ ] Download every public asset, verify the checksum manifest, release manifest, attestations,
      provenance, and stage-appropriate signatures, and confirm update-channel identity.
- [ ] Keep Alpha.8 tag, release, notes, and artifacts unchanged.

## 6. Bind exact public host evidence

- [ ] Reconfirm explicit synthetic-data consent and one-session host configuration authority.
- [ ] Use the downloaded public Alpha.9 CLI to initialize a fresh synthetic dual-Pack Workspace
      and run the bounded guarded MCP lifecycle.
- [ ] Run the canonical Codex CLI Generic, Claude Code Academic, and Claude Desktop Generic
      Requirement preview/cancel scenarios; require zero mutation and zero submission.
- [ ] Record stale-host rejection where required, verify final Workspace integrity, and restore
      temporary host configuration byte-for-byte.
- [ ] Add a dated body-free note cross-linking the candidate render/recovery jobs, then update
      `release/provider-dogfood.json` with exact Alpha.9 identities and note digest.
- [ ] Run the provider validator through the final release source gate; retain no bodies, paths,
      transcripts, tokens, credentials, or private content.

## 7. Reconcile and hand off

- [ ] Commit the evidence-only record and note, push a protected PR, and merge only after explicit
      authorization and passing checks.
- [ ] Reconcile the Master Roadmap, Issue #183, GitHub milestone, public release, and Trellis task
      to Verified only when all identities agree.
- [ ] Archive this task after the verified state is committed and public.
- [ ] Hand off Issue #70 as separate invited-user work against exact public Alpha.9; do not infer
      Beta readiness or Beta publication authorization.

## Review and stop conditions

- Stop if any authority source disagrees, any required job is missing, any identity differs, or a
  date-bound exception is stale.
- Stop if the candidate expires, the tag already exists, the protected source changes, or
  promotion would rebuild product bytes.
- Stop if host testing would expose non-synthetic content or cannot restore temporary config.
- After publication, never amend Alpha.9; open a later sequential Alpha for changed-byte fixes.
