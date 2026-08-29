# Freeze qualified Alpha.10 v4 contracts for Beta.1

## Goal

Replace the obsolete Beta freeze candidate with one exact, machine-validated contract record for
the qualified public Alpha.10 surface. The result must let the separate Beta.1 stage task prove
that Agent v4, Workspace v4, Skills/resources, both Packs, schemas, operations, exit semantics,
migrations, and package layouts have not drifted.

## Background and confirmed facts

- `M4-READY-001` / Issue #71 is Verified through protected PR #201 and merge
  `e939f1918c3e3febc59beb434690e7d13c3f7498`.
- `release/beta-readiness.json` is a qualified `canisend.beta-readiness/v2` record for exact public
  Alpha.10 source `cd40180f2ff8ac957276f1948ba88da428511a82`.
- The current `release freeze-candidate` output is obsolete: it emits
  `canisend.beta-contract-freeze/v1`, Agent v2, Workspace v2, and migrations frozen only through
  version 13.
- `release/alpha-package-contract.json` already validates the Alpha.10 v4 protocols, both Pack
  digests, resource manifest, operation registry, complete migration inventory, standalone CLI
  layout, and desktop bundle layouts.
- Beta readiness already validates the host-resource format, task-resource-model digest, and four
  current Skill digests. Stable exit classes live in `canisend-contracts`.
- Contract freeze, stage transition, candidate build/publication, qualification, package channels,
  feature-freeze activation, and cohort evidence are separate Roadmap outcomes.

## Requirements

### R1 — Version the active freeze authority

- Advance the active record to `canisend.beta-contract-freeze/v2`.
- Preserve `release/history/0.7/beta-contract-freeze.json` and all tagged Alpha evidence
  byte-for-byte.
- Retain exact Alpha.10 tag/source baseline identity.

### R2 — Reuse qualified authorities

- Build only after the complete readiness-v2 validator accepts the current Alpha.10 record.
- Reuse `alpha_package_contract_bindings`, `beta_readiness_v2_contracts`, existing schema/migration
  readers, and stable `ErrorCode`/`ExitClass` definitions.
- Reject disagreement between readiness, Alpha package bindings, the active package contract, and
  generated freeze values.

### R3 — Freeze the complete current surface

- Bind Agent v4, Workspace v4, host-resource v4, task-resource model, Pack v1, both built-in Pack
  digests, all four Skill digests, the resource manifest, operation registry, and complete current
  migration inventory.
- Bind one deterministic tree digest and per-family counts for public v2, application v3, Agent v4,
  and workflow-Pack v1 schemas.
- Bind the stable CLI exit classes and every current error-code-to-exit-class mapping.
- Bind the exact validated Alpha.10 package-contract file digest so standalone CLI and macOS bundle
  layouts cannot drift.
- Do not retain legacy Agent/Workspace v2 snapshots as an active Beta authority.

### R4 — Fail closed at every owning boundary

- `release check` and `prepare-stage v1.0.0-beta.1` must reuse the same complete v2 freeze
  validator, not accept a matching baseline alone.
- Reject v1 active records, Agent/Workspace v2 or v3, stale Pack/Skill/resource/operation/schema/
  migration/package digests, changed exit mappings, unknown fields, or a mismatched Alpha source.
- Keep pending-state generation canonical for future sequential Alpha work.

### R5 — Produce one reviewed record without staging Beta

- Run `release freeze-candidate`, inspect the complete JSON, and update only
  `release/beta-contract-freeze.json` plus necessary release-control documentation.
- Do not run or apply `prepare-stage`, change a version, create or move a tag, dispatch a native
  build, publish a release, or activate feature freeze.

### R6 — Keep verification minimum-sufficient

- Use one focused freeze-v2 acceptance/rejection regression, Rust format, affected `xtask` Clippy,
  one direct candidate/check pass, one final source gate, and protected Fast CI.
- Add no dependency, new command, writer mode, fixture framework, native matrix, host matrix, or
  compatibility layer.

## Acceptance criteria

- [x] `release freeze-candidate` emits exact `canisend.beta-contract-freeze/v2` for public
      Alpha.10 and no legacy Agent/Workspace identity.
- [x] The record binds both Packs, four Skills, resource/task model, operation registry, complete
      migrations, all active schema families, exit mappings, and the Alpha.10 package layout
      authority.
- [x] A full shared validator is used by both `release check` and the Alpha-to-Beta transition.
- [x] One focused regression accepts the canonical shape and rejects legacy, unknown, mismatched,
      and unbound variants, including a correct-baseline/incomplete-contract record.
- [x] `release/beta-contract-freeze.json` exactly equals the inspected candidate.
- [ ] Roadmap, release runbook, Trellis, and GitHub describe v4 contract freeze as Verified and
      leave `M4-STAGE-001` pending.
- [ ] The branch passes `git diff --check`, Rust format, the focused test, affected Clippy, the
      direct freeze check, one final `release check`, and protected Fast CI.

## Out of scope

- Beta.1 stage preview/write, version metadata, release notes transition, candidate build, native
  signing/integrity matrix, tag, promotion, publication, or qualification ledger.
- Package-channel candidates, feature-freeze activation, invited-user cohort, RC, or Stable work.
- Product behavior, UI, CLI/MCP operations, Packs, schemas, migrations, layouts, or compatibility
  changes; this task freezes the existing qualified surface only.

## Blocking open questions

None. The active Roadmap, qualified readiness record, current contract authorities, and the
minimum-test direction define the complete boundary.
