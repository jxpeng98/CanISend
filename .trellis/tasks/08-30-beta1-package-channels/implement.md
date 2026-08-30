# Beta.1 package-channel candidate execution

## 1. Correct the owning package metadata

- [x] Confirm the three current description consumers and historical candidate regeneration gate.
- [x] Add one version-aware metadata boundary at `1.0.0-alpha.6` and feed all three renderers.
- [x] Extend the existing focused renderer regression for historical and current generic wording.
- [x] Do not edit historical candidate files.

## 2. Verify the exact source assets

- [x] Locate the retained independently downloaded Beta.1 asset directory or download all 20
      public assets into a fresh temporary directory.
- [x] Run the existing complete release verifier for exact `v1.0.0-beta.1`.
- [x] Confirm tag, source, manifest SHA-256, checksum coverage, signing records, and three required
      native archives before generation.

## 3. Generate one local-only candidate set

- [x] Confirm `packaging/candidates/v1.0.0-beta.1` does not exist.
- [x] Run `xtask release channels` once with the exact verified assets and target directory.
- [x] Inspect `candidate-source.json` and all five package manifests.
- [x] Require generic description/tags, GPL-3.0-only metadata, exact URLs/digests/nested paths,
      `candidate_only: true`, and `publication_authorized: false`.
- [x] Confirm no external package index or release authority changed.

## 4. Reconcile project control

- [x] Add one body-free Beta.1 channel-candidate evidence note.
- [x] Update README, RELEASE, Roadmap, Trellis control, and parent/current task truth.
- [x] Keep feature freeze, cohort evidence, RC, Stable, lifecycle qualification, and publication
      pending.
- [x] Move Issue #76 to In progress; do not make Issue #77 active before protected merge.

## 5. Minimum final gate

- [x] `git diff --check`
- [x] `cargo fmt --all -- --check`
- [x] Existing focused channel-rendering regression
- [x] `cargo clippy -p xtask --all-targets --locked -- -D warnings`
- [x] One final `cargo run -p xtask --locked -- release check`
- [x] Trellis task validation
- [ ] Protected Fast CI on the exact PR head
- [x] No full local workspace suite, native rebuild, host matrix, package lifecycle, or extended
      assurance

## 6. Protected reconciliation

- [ ] Merge the bounded PR after protected checks pass.
- [ ] Mark Issue #76 / `M4-CHANNEL-001` Verified with exact evidence.
- [ ] Make `M4-FREEZE-002` / Issue #77 Ready and archive this task.
- [ ] Do not activate feature freeze or publish a package channel in this task.

## Stop conditions

- Stop on any tag, source, manifest, checksum, archive, URL, digest, nested path, license, or
  candidate-publication mismatch.
- Stop if generation would overwrite an existing directory, mutate an external index, change the
  qualified ledger, rewrite historical candidates, or claim lifecycle validation not performed.
