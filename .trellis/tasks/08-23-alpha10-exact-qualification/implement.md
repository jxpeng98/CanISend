# Codex-first Alpha.10 execution

## 1. Preserve completed immutable evidence

- [x] Protected Alpha.10 source `S` is
      `cd40180f2ff8ac957276f1948ba88da428511a82`.
- [x] Candidate run `C` `32678848156` and artifact `A` `9503978913` passed the five CLI
      targets, supported App packages, lifecycle/accessibility/integrity, SBOM, provenance, and
      community-signing gates.
- [x] Annotated tag object `6a43aa0889445ae5531736ac8e6d71cc363f6869` peels to `S`.
- [x] Promotion run `33267148891` reused `A` without recompilation and passed all six draft
      native smokes.
- [x] Independent public verification passed all 16 assets, 15 manifest-managed files,
      attestations, executable identities, and candidate/public byte equality.
- [x] Codex CLI Generic, Claude Desktop Generic, and the bounded MCP client passed non-mutating
      Alpha.10 preview/cancel scenarios. Claude Code is truthfully retained as
      `skipped-by-maintainer`.
- [x] Do not rerun any completed item above unless its recorded identity is shown to be false.

## 2. Implement the Codex-first evidence policy

- [x] After explicit approval of this revised plan, change the active provider schema constant to
      `canisend.provider-dogfood/v2`.
- [x] Reuse the existing validator and record shape. Replace the hard-coded three-host scenario
      set with exactly Academic and Generic Codex CLI scenarios; require an empty active
      `excluded_attempts` list.
- [x] Extend the existing provider validator regression in place so a missing Pack scenario and an
      unsafe mutation outcome are rejected. Do not add a helper, workflow, schema file, fixture
      directory, or test framework.
- [x] Update the Roadmap, support guidance, Trellis project-control guide, parent task, and current
      task metadata to distinguish required Codex evidence from non-blocking Claude observations.
- [x] Keep ADR-RN-0020 unchanged because Agent v4 architecture, resource generation, and available
      host adapters do not change.

## 3. Fill only the missing Codex scenario

- [x] Download only the public Alpha.10 Apple Silicon CLI archive and checksum material needed for
      the run into a fresh temporary directory; verify its public checksum and executable identity.
- [x] Reuse the existing synthetic-data authorization. With the App closed, create a clean
      Workspace and run one Academic Requirement confirm preview/cancel through Codex
      against the public Alpha.10 CLI.
- [x] Verify the Academic Requirement remains `proposed`, the Application revision is unchanged,
      no commit or submission occurs, and final Workspace integrity passes.
- [x] Retain only body-free host/version, exact binary/source identities, Pack/Skill digests,
      revisions, states, consent boundary, and outcome. Do not retain prompts, transcripts,
      private paths, bodies, credentials, or tokens.
- [x] Add a new dated Codex-first Alpha.10 note. Preserve the earlier Alpha.10 gap note and all
      Alpha.9 evidence byte-for-byte.
- [x] Rewrite `release/provider-dogfood.json` as v2 bound to Alpha.10, using the existing Generic
      result and the new Academic result.

## 4. Reconcile protected project state

- [x] Make Alpha.10 the current Codex-qualified Roadmap checkpoint and Beta.1 entry without
      claiming user evidence.
- [ ] Update Issue #194 and milestone 10 only after the policy/evidence PR reaches protected
      `main`.
- [ ] Close Issue #68 as not applicable, not Verified, because no post-fix replacement build owns
      an affected-scenario rerun.
- [ ] Rebind Issue #70 to Alpha.10 and move cohort evidence after Beta.1 and before RC.
- [ ] Refresh Issue #71 to own lean Alpha.10-bound Beta readiness and the separate Beta.1 release
      task.
- [ ] Archive this task and complete the Alpha.10 parent only after GitHub, Roadmap, provider
      record, task metadata, and protected `main` agree.

## 5. Minimum verification and delivery

- [x] Run `git diff --check`.
- [x] Run `cargo fmt --all -- --check`.
- [x] Run only the focused provider regression:
      `cargo test -p xtask --locked provider_dogfood_rejects_missing_stale_failed_or_private_records`.
- [x] Run affected Clippy:
      `cargo clippy -p xtask --all-targets --locked -- -D warnings`.
- [x] Run `cargo run -p xtask --locked -- release check` once on the final complete branch head.
- [ ] Commit one auditable policy/evidence change, push one PR, and accept protected Fast CI as the
      complete source owner.
- [x] Do not run a local full workspace suite, native matrix, desktop suite, Claude host matrix,
      package-manager qualification, or extended assurance.
- [ ] Obtain explicit authorization before protected merge.

## 6. Beta.1 handoff

- [ ] After the Alpha.10 task is protected and archived, create or refresh one bounded
      `M4-READY-001` Trellis child for Issue #71.
- [ ] Its planning must replace pre-Beta real-user thresholds with exact Alpha.10 Codex evidence,
      zero applicable P0 blockers, and body-free maintainer validation. User cohort collection
      moves after public Beta.1 and remains required before RC planning.
- [ ] The Beta child owns one readiness transition, one build-once Beta.1 native candidate,
      same-byte promotion, independent public verification, and feature-freeze activation.
- [ ] Beta stage write, tag, publication, and merge remain separate explicit authorization gates.

## Stop conditions

- Stop if the Academic Codex scenario mutates state, submits, exposes private content, or uses
  bytes that do not identify public Alpha.10.
- Stop if any source, tag, artifact, checksum, attestation, Pack, Skill, contract, or note digest
  differs from the recorded Alpha.10 identity.
- A product defect requires a later sequential prerelease. Never move the Alpha.10 tag, replace its
  assets, or weaken consent, recovery, privacy, path, or release-integrity controls.
- If the dependency review expires before a future Beta candidate, refresh it through its owning
  review; never extend dates only to unblock CI.
