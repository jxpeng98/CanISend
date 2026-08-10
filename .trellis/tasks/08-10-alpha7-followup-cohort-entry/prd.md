# Reconcile Alpha.7 follow-up and cohort entry

## Goal

Establish one auditable decision and exact-byte entry gate for the invited Alpha cohort after the
published Alpha.7 checkpoint and the post-release Workspace bootstrap fixes.

## Current Baseline

- Public `v1.0.0-alpha.7` is bound to source
  `9986a6a63b596b7760b4721a7e97c36aedce6d51`; Roadmap task M3-ALPHA7-001 is Verified in
  [Issue #61](https://github.com/jxpeng98/CanISend/issues/61).
- [PR #174](https://github.com/jxpeng98/CanISend/pull/174) contains post-publication bootstrap,
  Agent Skills scope, Typst Profile, starter-resource, and sidebar improvements. Its Fast CI is
  green, but the PR is still Draft and its bytes are not published Alpha.7.
- M3-EVID-005 and the cumulative cohort/Beta-readiness evidence are tracked by open P0
  [Issue #70](https://github.com/jxpeng98/CanISend/issues/70).

## Requirements

1. Review and merge or explicitly close PR #174 without bypassing protected CI.
2. Classify each observed post-Alpha.7 usability problem as a cohort blocker or non-blocking
   follow-up; create no new feature work unless an observed P0/P1 exit assertion lacks an owner.
3. Record whether the cohort remains on exact public Alpha.7 or requires a new qualified public
   checkpoint after PR #174. Later `main` or PR bytes must never be described as published Alpha.7.
4. Bind every cohort run to one exact tag, source commit, artifact/run identity, Agent/Skills and
   Pack digests, scenario ID, consent, and body-free outcome.
5. Preserve the Roadmap's invited cohort coverage and denominator rules; route aggregate readiness
   evidence through Issue #70 and `release/beta-readiness.json`.
6. Start Beta.1 preparation only after the M3 exit gate has no unresolved P0/P1 blocker and the
   refreshed readiness record accepts the exact tested checkpoint.

## Constraints

- The Master Roadmap, accepted ADRs, release JSON, protected GitHub Issues, tags, and artifacts
  remain authoritative; this Trellis task is execution bookkeeping.
- Do not commit private application bodies, Profiles, Evidence, transcripts, paths, credentials,
  or provider tokens.
- Do not expand Alpha/Beta scope while resolving cohort entry.
- A local package or green source CI does not qualify public bytes.

## Acceptance Criteria

- [ ] PR #174 has an explicit reviewed disposition and protected checks remain green.
- [ ] The exact cohort checkpoint decision is committed and points to its tag/source/run/artifacts.
- [ ] Every invited run uses the Roadmap's mixed-Pack, host, language, input, recovery,
  accessibility, and no-submission coverage without private bodies.
- [ ] Failures have P0/P1 dispositions and minimum safe links; affected scenarios are rerun on the
  exact replacement checkpoint when bytes change.
- [ ] Issue #70 and Beta readiness contain consistent numerators, denominators, exclusions, digests,
  and zero unresolved supported blockers.
- [ ] Beta.1 remains blocked until its Roadmap entry and explicit authorization gates pass.

## References

- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md` §§11.6, 11.7, 12, and 19
- `.trellis/spec/guides/project-control.md`
- `.trellis/spec/backend/quality-guidelines.md`
- `release/provider-dogfood.json`
- `release/beta-readiness.json`

This is complex release-governance work. Add `design.md` and `implement.md`, review them, and only
then start the task.
