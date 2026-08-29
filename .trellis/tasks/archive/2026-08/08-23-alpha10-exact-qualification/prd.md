# Qualify Alpha.10 for Codex-first Beta entry

## Goal

Reconcile the already-published exact Alpha.10 as the qualified entry to
`v1.0.0-beta.1`, using Codex as the required external Agent host and the smallest verification
tier that owns each invariant.

## Background

- The maintainer selected `v1.0.0-beta.1` as the immediate stable checkpoint. Final
  `v1.0.0`, RC qualification, package-manager publication, and long-term support remain later
  decisions.
- Alpha.10 is public from protected source
  `cd40180f2ff8ac957276f1948ba88da428511a82`. Candidate run `32678848156`, artifact
  `9503978913`, promotion run `33267148891`, all 16 public assets, attestations, and
  candidate/public byte identity have already passed.
- Exact Alpha.10 Codex CLI Generic and bounded MCP preview/cancel scenarios passed without mutation
  or submission. Claude Desktop also passed as an observation. Claude Code stopped before provider
  access and is recorded as `skipped-by-maintainer`.
- `release/provider-dogfood.json` remains bound to fully qualified Alpha.9 because its v1
  validator hard-codes three required hosts. The Roadmap therefore still names Alpha.9 as the
  provider checkpoint even though Alpha.10 is the latest verified public release.
- `release/beta-readiness.json` is canonical pending Alpha.10 evidence and the qualification
  ledger remains `pre-beta`. No Beta transition or user-evidence claim has occurred.
- The current verification tiers already assign focused checks, Fast CI, native matrices, and
  extended assurance to separate owners; acceleration requires removing duplicate execution, not
  weakening trust-boundary invariants.

## Requirements

### Codex-first qualification boundary

- Codex CLI is the required external Agent host for Beta entry. Exact public Alpha.10 must pass one
  canonical non-mutating Requirement preview/cancel scenario for each built-in Pack in one clean
  Workspace.
- Reuse the passing Alpha.10 Generic result and run only the missing Academic Codex scenario.
  Both results must bind the same public executable identity, Workspace/Agent v4 contracts, Pack
  digests, and Skill digests.
- Claude Code and Claude Desktop real-host sessions are non-blocking compatibility observations.
  Their generated resources and host-neutral MCP/CLI contracts remain checked from the canonical
  Agent v4 source, but an unrun or unauthenticated Claude session is never reported as passed.
- Preserve preview/approval/revision binding, consent, no-submission, body-free evidence, and
  zero-unauthorized-mutation requirements.

### Minimum-sufficient verification

- Reuse the completed Alpha.10 native candidate, same-byte promotion, public download,
  attestation, and byte-identity evidence. Do not rebuild, republish, or redownload the full asset
  set for this policy reconciliation.
- Run one focused provider-record validator regression, Rust formatting and affected `xtask`
  Clippy, then one final `release check` on the complete PR head. Protected Fast CI owns the full
  source suite.
- Do not run a local workspace suite, native matrix, desktop suite, Claude real-host matrix,
  package-manager qualification, or extended assurance for unchanged product bytes.
- Keep the smallest positive and negative checks for consent, data loss, recovery, path, privacy,
  and release integrity. Test reduction must not remove these owning assertions.

### Machine and body-free evidence

- Rev the active provider record to `canisend.provider-dogfood/v2`. Keep its existing bounded
  identity/consent/contract/Pack/Skill structure, require exactly the Academic and Generic Codex
  scenarios, and retain no private bodies or credentials.
- Add a new dated body-free note for the Codex-first policy and missing Academic result. Do not
  rewrite the earlier Alpha.10 gap note or any Alpha.9 evidence.
- Bind the provider record, note digest, exact Alpha.10 source/candidate/public identities, and both
  Pack scenarios before changing the current checkpoint claim.
- Keep Claude observations in the dated note and Trellis metadata, not in the required passed
  scenario set.

### Governance and Beta.1 handoff

- Update the Master Roadmap, support guidance, Trellis project control, parent task, Issue #194,
  milestone 10, and this task to state the same Codex-first boundary.
- Mark Alpha.10 qualified only after the evidence change reaches protected `main`. Close Issue
  #68 as not applicable when no post-fix rerun exists; do not mark it Verified from unrelated
  evidence.
- Rebind Issue #70 to Alpha.10 and move invited-user/cohort evidence after Beta.1 and before RC.
  Beta exists to collect that evidence; synthetic maintainer dogfood remains zero users.
- Refresh Issue #71 as the next `M4-READY-001` task. A separate Trellis child will own lean Beta
  readiness, one build-once Beta.1 candidate, same-byte publication, public verification, and
  feature-freeze activation.
- Keep Beta readiness pending in this task. No Beta tag, stage transition, or publication is
  authorized by this planning or Alpha.10 reconciliation work.

## Acceptance Criteria

- [ ] Roadmap, support guidance, provider record, milestone, Issues, and Trellis metadata identify
      Codex as the required external host and Claude real-host testing as non-blocking.
- [x] Provider-dogfood v2 binds exact public Alpha.10 and exactly two passed Codex scenarios,
      covering Academic and Generic Packs without mutation, submission, retained private content,
      or credential material.
- [x] Existing Alpha.10 native/public evidence remains immutable and is reused without rebuild,
      republish, or duplicate full-asset qualification.
- [x] The focused validator regression rejects a missing Pack scenario, unsafe outcome, stale
      contract/digest, failed status, and private-field injection.
- [ ] Formatting, affected `xtask` Clippy, one final `release check`, and protected Fast CI
      pass on the exact policy/evidence head.
- [ ] Issue #194 and milestone 10 close only after protected reconciliation; Issue #68 is closed
      without a false Verified claim; Issue #70 is deferred to post-Beta cohort evidence.
- [ ] Issue #71 and the Roadmap name Alpha.10 as the exact entry to a separately planned
      `v1.0.0-beta.1` transition with zero synthetic-user claim.

## Constraints

- Alpha.10 source, tag, release, artifacts, checksums, attestations, and historical notes are
  immutable.
- The 23 lock-bound dependency exceptions expire after 2026-09-07 UTC and must be reviewed again
  if the later Beta candidate reaches that boundary.
- Alpha.10 retains the `community-build` trust tier. Do not imply notarization, Developer ID,
  trusted Authenticode, public timestamping, or warning-free installation.
- Protected PR merge and the future Beta transition/publication remain explicit authorization
  gates.

## Out of Scope

- Product features, legacy Workspace/Agent/Skill compatibility, new workflows, new test
  frameworks, or provider-specific business logic.
- Repeating completed Alpha.10 native/public qualification or rerunning Claude Code/Desktop.
- Invited-user collection before Beta.1, RC qualification, final `v1.0.0`, external
  package-manager publication, or support-policy publication.
- Implementing or publishing Beta.1 inside this task.

## Parent Artifacts

- `../08-18-alpha10-release-integration/prd.md`
- `../08-18-alpha10-release-integration/design.md`
- `../08-18-alpha10-release-integration/implement.md`
