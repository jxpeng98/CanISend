# Beta.1 candidate execution

## 1. Freeze entry identity

- [x] Confirm remote `main` remains `6e1397b79031cad54e794ccdc9edca2153f23b3e`.
- [x] Wait for Fast CI run `33280629465` to complete successfully and retain successful dependency
      assurance run `33280629375`.
- [x] Confirm Issue #74 is Ready, source is `1.0.0-beta.1` / `beta-qualifying`, and neither the
      tag nor release exists.
- [x] Stop on source movement, failed protected gate, stale required exception, or contradictory
      release authority.

## 2. Build and inspect once

- [x] Dispatch `.github/workflows/release.yml` on exact `main` for future tag
      `v1.0.0-beta.1` with `promote_existing_tag=false`; use the workflow's existing cache default.
- [x] Wait for the complete candidate run. Require every candidate job and
      `assemble-and-attest-release` to succeed.
- [x] Record the candidate run URL/ID and exact unexpired release-assets artifact ID.
- [x] Download the complete artifact to a fresh temporary directory.
- [x] Run existing candidate/release verification and verify each asset's GitHub attestation
      against the exact source commit.
- [x] Inspect the two Apple ad-hoc and one Windows self-signed signing records, manifest, SBOM,
      target set, App ZIP/DMG, checksums, and asset count. Retain no private bodies or credentials.

## 3. Promote the reviewed bytes

- [x] Create one annotated `v1.0.0-beta.1` tag at the exact candidate source and verify its object
      peels to that commit before pushing.
- [x] Push only the annotated tag and wait for the tag-triggered promotion workflow.
- [x] Require candidate lookup, candidate reverification, same-byte draft upload, five CLI draft
      smokes, desktop ZIP/DMG smoke, prerelease publication, and public verification to pass.
- [x] Confirm promotion evidence reports `recompiled_during_promotion: false` and identifies the
      exact candidate run/artifact.
- [x] Use existing resume/finalizer recovery only when its checked preconditions match; never move
      the tag or rebuild behind it.

## 4. Independently verify public Beta.1

- [x] Download every public asset to a second fresh temporary directory.
- [x] Verify `SHA256SUMS`, complete release structure, manifest source/tag, prerelease state,
      update response, and every GitHub attestation.
- [x] Compare candidate and public checksum/manifest bytes and confirm all asset digests agree.
- [x] Record exact tag object/source, candidate and promotion run IDs/URLs, artifact ID, manifest
      digest, asset count, public URL, and verification time in one body-free dated note.

## 5. Reconcile through protected review

- [x] Update the Master Roadmap and Trellis project-control truth to public-but-not-yet-qualified
      Beta.1; keep ledger, channels, freeze, cohort, RC, and Stable claims pending.
- [x] Update this task and its parent without rewriting historical release evidence.
- [x] Run `git diff --check`; documentation/control-only changes do not run local Rust tests.
- [ ] Commit and push the bounded evidence/control changes, open one PR, and rely on protected CI.
- [ ] After merge, mark Issue #74 `state:verified` with exact evidence and hand Issue #75 the public
      asset directory plus candidate run identity.
- [ ] Archive this task only after GitHub, Roadmap, Trellis, protected `main`, tag, release, and
      public bytes agree.

## Stop conditions

- Stop before tagging on any candidate job, checksum, manifest, signing, provenance, target, App,
  or source mismatch.
- Stop before publication if promotion cannot locate the exact reviewed artifact or would compile,
  sign, package, attest, or replace product bytes.
- After tag creation, never delete, force-update, or repoint the tag and never overwrite evidence
  to make a failed gate pass.
- Do not write Beta qualification, generate/publish package channels, activate feature freeze,
  count synthetic dogfood as users, or start RC work in this task.
