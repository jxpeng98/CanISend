# Beta.1 qualification execution

## 1. Repair and prove the recorder precondition

- [x] Confirm the active ledger, stage renderer, and validator all use status-only pending Beta.
- [x] Change the recorder's equality guard to that same canonical object.
- [x] Update the existing positive/negative regression; add no second test module.
- [x] Run format, the focused regression, and affected xtask Clippy.
- [x] Commit the repair and task plan so write mode can begin from a clean worktree.

## 2. Freshly verify exact public assets

- [x] Create one fresh temporary directory and download all public `v1.0.0-beta.1` assets.
- [x] Require 20 files and the expected manifest/checksum identities.
- [x] Run the existing complete release verifier.
- [x] Verify all 20 attestations against the repository release workflow and exact source commit.
- [x] Confirm the public release remains a non-draft prerelease at the immutable annotated tag.

## 3. Preview and record qualification

- [x] Run the non-mutating recorder with candidate run `33281162734` and inspect its JSON report.
- [x] Reconfirm a clean worktree, then run the identical command once with `--write`.
- [x] Compare tag, source, run, manifest, and before/after hashes between reports.
- [x] Confirm only `release/qualification-ledger.json` changed and inspect its exact semantic diff.
- [x] Stop instead of manually editing any generated qualification field.

## 4. Reconcile projections

- [x] Add one body-free qualification evidence note.
- [x] Update README, RELEASE, Roadmap, Trellis project control, and current/parent task truth.
- [x] Make Beta.1 the qualified public checkpoint but keep stage `beta-qualifying`, package
      channels pending, freeze planned, cohort evidence pending, RC absent, and Stable unauthorized.
- [x] Move Issue #75 to In progress and keep the next task inactive before protected merge.
- [x] Bind post-Alpha provider dogfood validation to exact Beta-readiness entry evidence without
      rewriting the Alpha.10 provider record or rerunning a host.

## 5. Minimum final gate

- [x] `git diff --check`
- [x] `cargo fmt --all -- --check`
- [x] Focused recorder positive/negative regression
- [x] `cargo clippy -p xtask --all-targets --locked -- -D warnings`
- [x] One final `cargo run -p xtask --locked -- release check`
- [x] Protected Fast CI on the exact PR head
- [x] No local full workspace suite, native rebuild, provider matrix, package lifecycle, or extended
      assurance

## 6. Protected reconciliation

- [x] Merge the bounded PR after protected checks pass.
- [x] Mark Issue #75 / `M4-LEDGER-001` Verified with exact evidence.
- [x] Make `M4-CHANNEL-001` Ready and archive this task.
- [x] Do not generate channels or activate feature freeze in this task.

## Stop conditions

- Stop on any tag, source, run, manifest, checksum, signing, asset-count, or provenance mismatch.
- Stop if preview/write hashes differ, write starts dirty, or any path besides the ledger changes.
- Stop if qualification would claim trusted Apple/Windows publisher identity, package publication,
  feature freeze, invited users, RC, Stable, upload, or submission.
