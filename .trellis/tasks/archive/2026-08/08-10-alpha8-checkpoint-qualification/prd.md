# Qualify the Alpha.8 replacement checkpoint

## Goal

Publish and independently reverify one exact `v1.0.0-alpha.8` checkpoint containing PR #174 and
the minimum release-authority repair, then bind fresh provider/host evidence to those bytes.

## Requirements

1. Create and link Roadmap task `M3-ALPHA8-001`, a P0 release-owned GitHub Issue, and an Alpha.8
   milestone before this task becomes Ready.
2. Merge the current Trellis control branch and PR #174 through their protected checks; record each
   exact merge commit.
3. Fix the sequential-Alpha preview so current published Alpha wording can transition to canonical
   Alpha.8 development wording without broad or silent replacement.
4. Generalize active Beta eligibility from literal Alpha.7 to the exact active v4 Alpha iteration
   7 or greater, preserving exact tag/source/run/URL/contracts/Pack/provider/user binding and
   rejection of Alpha.6 or mismatches.
5. Preview and apply `prepare-stage v1.0.0-alpha.8` from a clean worktree, review every controlled
   digest, and merge the stage source through the full source gate.
6. Build once, qualify, promote without rebuilding, publicly reverify, and record exact Alpha.8
   tag/source/run/artifact/manifest/integrity evidence.
7. Rerun exact Codex, Claude Code, Claude Desktop, and bounded MCP-host scenarios and update the
   body-free provider record before cohort entry.

## Acceptance Criteria

- [x] PR #174 and every release-authority change have explicit protected dispositions.
- [x] Focused tests prove published-source Alpha.7→Alpha.8 preview, Alpha.8 Beta eligibility, and
      rejection of Alpha.6 plus stale/mismatched readiness and provider identities.
- [x] Alpha.8 transition dry-run and final source gate pass before write; the written transition
      matches the reviewed controlled-file plan.
- [x] Public Alpha.8 artifacts are the exact qualified candidate bytes and pass independent public
      verification on all supported release targets.
- [x] Provider/host evidence binds Alpha.8, Agent v4, Workspace v4, Skills/resources, both Pack
      digests, consent, scenarios, and body-free outcomes.
- [x] `M3-ALPHA8-001` is Verified and Issue #70 can start without an unresolved supported blocker.

## Constraints

- Preserve Alpha.7 history and never move a published tag.
- Stop for explicit authorization before protected merge, `--write`, tag, promotion, or publication.
- Do not include new product features or private user content.
- Use Tier 3 only for the exact candidate; do not recreate the five-target matrix locally.

## References

- Parent `08-10-alpha7-followup-cohort-entry`
- PR #174
- `xtask/src/main.rs`
- `scripts/refresh_beta_readiness.sh`
- `release/stage-transition-policy.json`
- `docs/release/stage-transitions.md`
- `release/provider-dogfood.json`
