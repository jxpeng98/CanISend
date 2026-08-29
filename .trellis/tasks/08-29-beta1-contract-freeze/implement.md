# Beta.1 contract-freeze implementation

## 1. Repair the shared authority

- [ ] Advance the active freeze schema to v2; keep historical v1 bytes unchanged.
- [ ] Make the builder validate complete readiness v2 before using the Alpha baseline.
- [ ] Build the contract block from `alpha_package_contract_bindings` plus only the existing
      readiness-owned resource/task/Skill fields, rejecting overlap drift.
- [ ] Replace legacy Agent/Workspace snapshot authority and migration 13 with the exact current v4
      contract surface and complete migration inventory.

## 2. Bind remaining frozen surfaces

- [ ] Add one deterministic digest/count record across the four checked-in schema families.
- [ ] Derive exit-class values and every error-code mapping from `canisend-contracts`.
- [ ] Validate and digest the exact Alpha.10 package contract to bind CLI/macOS layouts.
- [ ] Reuse one complete validator from both `check_beta_contract_freeze` and
      `check_beta_transition_authorities`.

## 3. Leave one focused regression

- [ ] Replace or extend the existing freeze/transition regression in place so canonical v2 passes
      and legacy, unknown, v2/v3, stale-digest, changed-exit, package-layout, and baseline-only
      variants fail in one table-driven test.
- [ ] Do not create a fixture framework or duplicate the same assertion at adapter layers.

## 4. Produce the exact record

- [ ] Run and inspect `cargo run -p xtask --locked -- release freeze-candidate`.
- [ ] Update `release/beta-contract-freeze.json` to exactly that candidate.
- [ ] Update only the stage-transition runbook, Roadmap/release status, Trellis spec, and task
      projection that own the changed freeze fact.
- [ ] Keep Beta stage, candidate, publication, cohort, and feature freeze pending.

## 5. Minimum verification

- [ ] `git diff --check`
- [ ] `cargo fmt --all -- --check`
- [ ] One focused freeze-v2 validator regression
- [ ] `cargo clippy -p xtask --all-targets --locked -- -D warnings`
- [ ] Direct `release freeze-candidate` equality/check
- [ ] One final `cargo run -p xtask --locked -- release check`
- [ ] Protected Fast CI on the exact PR head
- [ ] No local full workspace suite, native rebuild, desktop suite, provider-host matrix,
      package-manager lifecycle, or extended assurance

## 6. Protected reconciliation

- [ ] Merge the bounded PR after Fast CI.
- [ ] Mark Issue #72 / `M4-FREEZE-001` Verified and keep milestone 5 open.
- [ ] Archive this task and make `M4-STAGE-001` / Issue #73 the next delivery task.
- [ ] Do not run `prepare-stage v1.0.0-beta.1` in this task.

## Stop conditions

- Stop if qualified readiness becomes stale for staging, an applicable P0 blocker appears, or any
  Alpha10/provider/package identity disagrees.
- Stop if the generated freeze contains Agent/Workspace v2 or v3, incomplete schema/migration/
  Skill/Pack authority, a false layout digest, or unknown/private fields.
- Stop if implementation would change product bytes, publish an artifact, apply the stage
  transition, or weaken evidence, privacy, consent, path, recovery, or release-integrity controls.
