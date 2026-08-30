# Record Beta.1 qualification ledger

## Goal

Record exact public `v1.0.0-beta.1` in the qualification ledger from a fresh independent asset
download, after repairing the single pending-ledger shape mismatch that currently prevents the
existing recorder from accepting the stage transition's canonical output. Do not generate package
channels, activate feature freeze, count users, or advance to RC.

## Confirmed entry

- `M4-CANDIDATE-001` / Issue #74 is Verified through protected PR #204 and merge
  `a223d9e6a2cd9e9195f98fdf7a052184f71de7d0`.
- Exact public `v1.0.0-beta.1` binds source
  `6e1397b79031cad54e794ccdc9edca2153f23b3e`, candidate run `33281162734`, artifact
  `9723581536`, and manifest SHA-256
  `2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`.
- Candidate/public byte equality and all 20 public attestations were independently verified.
- Issue #75 is Ready. The checked-in ledger is Beta / `beta-qualifying`, has planned freeze with
  no baseline, and has a status-only pending Beta record.
- The stage renderer and checked-in ledger use `{"status":"pending"}`. The existing Beta recorder
  instead requires unused null placeholder fields, so its canonical precondition cannot accept the
  state produced by the supported Alpha-to-Beta transition.

## Requirements

### R1 — Repair the owning precondition once

- Make `beta_qualified_ledger` accept the exact status-only pending Beta object emitted by
  `initial_alpha_qualification_ledger` and preserved by `prepare-stage`.
- Update the existing focused positive/negative regression to use that canonical input.
- Keep provider dogfood bound to the current public checkpoint during Alpha, but after Alpha bind
  it to the exact qualified Beta-readiness Alpha entry authority. Do not relabel Alpha.10 host
  evidence as Beta.1 evidence or repeat provider dogfood.
- Add no schema, migration, compatibility branch, second recorder, dependency, or workflow.

### R2 — Re-establish public asset identity

- Download every asset from the exact public GitHub prerelease into one fresh temporary directory.
- Re-run the existing complete release verifier and verify every public asset attestation against
  repository `jxpeng98/CanISend`, the release workflow, and exact source commit.
- Require 20 assets, the expected manifest digest, complete checksum coverage, community-signing
  evidence, and exact tag/source identity before preview.

### R3 — Preview and write one bounded ledger mutation

- Run `record-beta-qualification v1.0.0-beta.1 33281162734 ASSETS` without `--write` and retain its
  body-free report outside the repository.
- Require the report to bind the exact tag, source, candidate run, manifest digest, and one
  before/after ledger hash pair.
- Commit the recorder repair and task plan first. Run the identical command with `--write` only
  from a clean worktree, then prove preview and write reports agree apart from mode/write markers.
- Accept only `release/qualification-ledger.json` as the command-owned write.

### R4 — Reconcile qualified checkpoint truth

- Add one dated body-free qualification note and update README, RELEASE, Master Roadmap, Trellis
  project control, parent/current task state, and GitHub Issue #75 through protected review.
- After the ledger write, make Beta.1 the latest publicly qualified checkpoint while retaining
  Beta / `beta-qualifying` as the machine stage.
- Keep package channels, feature-freeze activation, invited-user evidence, RC, and Stable pending.

### R5 — Use minimum-sufficient verification

- Run Rust format, the focused Beta qualification regression, affected xtask Clippy, one final
  `release check`, `git diff --check`, and protected Fast CI.
- Do not rebuild native assets, rerun the complete workspace suite locally, repeat provider
  dogfood, run Claude hosts, publish package indexes, or run extended assurance.

## Acceptance criteria

- [x] The existing recorder accepts the canonical status-only pending Beta shape and still rejects
      already-qualified, wrong-stage, zero-run, malformed-source, frozen, or noncanonical state.
- [x] Post-Alpha source gates accept provider dogfood only through the exact qualified
      Beta-readiness Alpha binding and still reject mismatched provider evidence.
- [x] One fresh public download verifies exact tag/source/manifest, all 20 assets, checksums,
      signing records, and 20 attestations before qualification preview.
- [x] Preview and write reports bind tag `v1.0.0-beta.1`, source `6e1397b...`, candidate run
      `33281162734`, and manifest digest `2435c335...`, with identical before/after ledger hashes.
- [x] Write mode starts clean and modifies only `release/qualification-ledger.json`.
- [x] The ledger records Beta status `qualified`, exact tag/source/run, and the three canonical
      signing-evidence targets while freeze remains planned with a null baseline.
- [x] User-facing and project-control truth distinguish qualified Beta.1 from pending channels,
      feature freeze, cohort evidence, RC, and Stable.
- [ ] Focused checks, final source gate, and protected Fast CI pass on the exact PR head; Issue #75
      becomes Verified only after merge.

## Out of scope

- Product, App, CLI, MCP, Pack, Skill, Workspace, schema, or compatibility behavior.
- New release workflow, artifact rebuild, tag movement, release replacement, signing-policy change,
  package-channel generation/publication, freeze activation, cohort evidence, RC, or Stable.

## Blocking open questions

None. Exact public evidence, the Roadmap, Issue #75, and the existing recorder define the outcome.
