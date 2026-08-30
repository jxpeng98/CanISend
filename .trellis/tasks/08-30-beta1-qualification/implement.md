# Beta.1 qualification execution

## 1. Repair and prove the recorder precondition

- [x] Confirm the active ledger, stage renderer, and validator all use status-only pending Beta.
- [x] Change the recorder's equality guard to that same canonical object.
- [x] Update the existing positive/negative regression; add no second test module.
- [x] Run format, the focused regression, and affected xtask Clippy.
- [ ] Commit the repair and task plan so write mode can begin from a clean worktree.

## 2. Freshly verify exact public assets

- [ ] Create one fresh temporary directory and download all public `v1.0.0-beta.1` assets.
- [ ] Require 20 files and the expected manifest/checksum identities.
- [ ] Run the existing complete release verifier.
- [ ] Verify all 20 attestations against the repository release workflow and exact source commit.
- [ ] Confirm the public release remains a non-draft prerelease at the immutable annotated tag.

## 3. Preview and record qualification

- [ ] Run the non-mutating recorder with candidate run `33281162734` and inspect its JSON report.
- [ ] Reconfirm a clean worktree, then run the identical command once with `--write`.
- [ ] Compare tag, source, run, manifest, and before/after hashes between reports.
- [ ] Confirm only `release/qualification-ledger.json` changed and inspect its exact semantic diff.
- [ ] Stop instead of manually editing any generated qualification field.

## 4. Reconcile projections

- [ ] Add one body-free qualification evidence note.
- [ ] Update README, RELEASE, Roadmap, Trellis project control, and current/parent task truth.
- [ ] Make Beta.1 the qualified public checkpoint but keep stage `beta-qualifying`, package
      channels pending, freeze planned, cohort evidence pending, RC absent, and Stable unauthorized.
- [ ] Move Issue #75 to In progress and keep the next task inactive before protected merge.

## 5. Minimum final gate

- [ ] `git diff --check`
- [ ] `cargo fmt --all -- --check`
- [ ] Focused recorder positive/negative regression
- [ ] `cargo clippy -p xtask --all-targets --locked -- -D warnings`
- [ ] One final `cargo run -p xtask --locked -- release check`
- [ ] Protected Fast CI on the exact PR head
- [ ] No local full workspace suite, native rebuild, provider matrix, package lifecycle, or extended
      assurance

## 6. Protected reconciliation

- [ ] Merge the bounded PR after protected checks pass.
- [ ] Mark Issue #75 / `M4-LEDGER-001` Verified with exact evidence.
- [ ] Make `M4-CHANNEL-001` Ready and archive this task.
- [ ] Do not generate channels or activate feature freeze in this task.

## Stop conditions

- Stop on any tag, source, run, manifest, checksum, signing, asset-count, or provenance mismatch.
- Stop if preview/write hashes differ, write starts dirty, or any path besides the ledger changes.
- Stop if qualification would claim trusted Apple/Windows publisher identity, package publication,
  feature freeze, invited users, RC, Stable, upload, or submission.
