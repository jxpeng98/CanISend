# Qualify Alpha.10 for lean Beta.1 readiness

## Goal

Replace the obsolete pre-Beta cohort gate with one body-free, machine-validated readiness record
for exact public Alpha.10. This lets the maintainer decide whether to begin the separate Beta.1
stage task without inventing user evidence, changing Alpha.10 bytes, or repeating native and
external-host qualification already owned by the public Alpha.10 record.

## Background and confirmed facts

- `M4-READY-001` / GitHub Issue #71 is Ready in milestone 5, `Beta — Contract freeze`.
- Protected merge `121e4b62ad19c90bad88ad53c63735cc112fbfd9` records exact public Alpha.10 as
  the Codex-qualified Beta-entry checkpoint.
- `release/provider-dogfood.json` uses `canisend.provider-dogfood/v2` and contains the required
  non-mutating Academic and Generic Codex CLI scenarios, exact source/artifact identity, Agent and
  Workspace v4 identity, host-resource identity, both Pack digests, and all four Skill digests.
- Active `release/beta-readiness.json` is still canonical pending state.
- `scripts/refresh_beta_readiness.sh` still requires the old invited-user record and rejects every
  open public Issue. That cannot represent the accepted Roadmap because invited-user evidence now
  starts on public Beta.1, while planned Beta, RC, and Stable Issues intentionally remain open.
- `xtask` currently validates the old `canisend.beta-user-evidence/v1` thresholds and does not
  place host-resource or Skill digests directly in the readiness contract.
- Public Beta.1, feature freeze, package-channel candidates, and post-Beta cohort evidence are
  separate Roadmap outcomes and are not authorized by this task.

## Requirements

### R1 — Version the changed readiness semantics

- Advance the active readiness schema to `canisend.beta-readiness/v2`.
- Preserve historical release evidence byte-for-byte; only the active 1.0 readiness authority may
  change.
- Keep the existing 24-hour freshness and five-minute future-skew limits.

### R2 — Bind the exact qualified Alpha surface

- Bind Alpha.10 tag, protected source commit, public release URL, candidate run, and the exact
  `canisend.provider-dogfood/v2` file digest.
- Bind Agent v4, Workspace v4, host-resource v4, task-resource-model digest, Pack v1, both built-in
  Pack digests, and the four current Skill digests.
- Bind exactly the required Academic and Generic Codex scenario IDs from the validated provider
  record. Claude host observations remain non-blocking and are not copied into readiness.

### R3 — Replace user evidence with body-free maintainer validation

- The refresh command accepts one body-free maintainer-validation JSON object containing a schema,
  passed status, reviewer token, known-limitations review result, and checked-in evidence-note
  path/digest.
- Exact-field validation rejects private or unknown fields, stale note digests, an empty reviewer,
  a failed status, or an unreviewed known-limitations result.
- The readiness record explicitly reports zero synthetic users, zero invited users, and zero
  completed user flows, identifies public Beta.1 as the cohort start, and keeps cohort completion
  required before RC.1.

### R4 — Review applicable blockers without requiring an empty backlog

- Query only public Issue number, state, and label names; do not read titles, bodies, comments,
  attachments, or private application data.
- Treat an open Issue carrying both `priority:P0` and `state:blocked` as an applicable release
  blocker. Any such Issue stops refresh and is recorded only by number in a rejected preview.
- Permit planned, ready, and future-milestone Issues to remain open.
- Review these exact blocker classes: data loss, privacy, evidence, Pack, rendering, recovery,
  host setup, supported install, and release integrity.
- A qualified record must combine the public blocker projection with exact Alpha.10 provider,
  contract, public-release, and maintainer-review evidence; an empty blocker list alone is never
  sufficient.

### R5 — Fail closed at the Beta transition

- `verify-beta-readiness`, `release check`, and the Alpha-to-Beta authority check accept only the
  canonical v2 record.
- Missing, stale, mismatched, failed, private-field-bearing, old cohort-bound, falsely user-counted,
  or unresolved-blocker evidence must fail.
- `prepare-stage v1.0.0-beta.1` may be previewed after qualification but this task must not apply
  the stage write, create or move a tag, dispatch a candidate, publish a release, or activate
  feature freeze.

### R6 — Keep delivery lean

- Reuse the existing readiness file, refresh script, provider validator, exact-field validator,
  evidence-note validator, and release checks.
- Add no dependency, service, workflow, schema directory, fixture framework, or host matrix.
- Use one focused validator regression, shell syntax validation, Rust formatting and affected
  Clippy, one final source gate, and protected Fast CI. Do not run a local full workspace suite,
  native rebuild, desktop suite, or Claude real-host matrix for unchanged product bytes.

## Acceptance criteria

- [ ] `release/beta-readiness.json` is a fresh, qualified
      `canisend.beta-readiness/v2` record bound to exact public Alpha.10.
- [ ] Its contract block contains Agent v4, Workspace v4, host-resource v4, task-resource-model,
      both Pack, and all four Skill identities/digests.
- [ ] Its provider block binds the checked-in provider record digest and exactly the two required
      Codex Pack scenarios.
- [ ] Its maintainer block is body-free, note-digest-bound, and records reviewed known
      limitations.
- [ ] Its cohort block truthfully contains three zero counts and the Beta.1-to-RC.1 boundary.
- [ ] Open planned Roadmap Issues do not block refresh; any open P0 Issue labeled
      `state:blocked`, any unresolved blocker, or any uncleared required blocker class does.
- [ ] One focused regression proves rejection of stale/private/false-cohort/unbound evidence and
      acceptance of the canonical shape.
- [ ] The refresh dry run and clean-worktree write validate successfully; Beta.1 stage preview
      succeeds without changing stage.
- [ ] The final branch passes `git diff --check`, `bash -n scripts/refresh_beta_readiness.sh`,
      Rust format, the focused test, affected Clippy, one `release check`, and protected Fast CI.
- [ ] Roadmap/runbook/Trellis/GitHub projections describe lean readiness and retain post-Beta
      cohort evidence as mandatory before RC.1.

## Out of scope

- Beta.1 stage write, candidate build, native matrix, tag, promotion, public verification,
  qualification-ledger write, channel candidates, or feature-freeze activation.
- Invited-user recruitment or evidence; it starts on public Beta.1 under `M3-EVID-005`.
- Re-running Alpha.10 native, App, Codex, Claude, or package-manager qualification.
- Product feature work, compatibility with pre-v4 Workspaces/Agents/Skills, new Packs, remote MCP,
  provider credentials, telemetry, or private content retention.

## Blocking open questions

None. The Roadmap, Issue #71, exact Alpha.10 evidence, and the maintainer's Codex-first/minimum-test
direction resolve the product, risk, and acceptance boundaries.
