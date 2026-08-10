# Qualify Alpha.8 and enter the invited cohort

## Goal

Turn the post-Alpha.7 bootstrap fixes into one exact, publicly qualified Alpha.8 checkpoint, then
collect the body-free invited-user evidence required to enter Beta preparation without relabelling
or rewriting the immutable Alpha.7 release.

## Background

- Public `v1.0.0-alpha.7` remains bound to source
  `9986a6a63b596b7760b4721a7e97c36aedce6d51` and Roadmap task `M3-ALPHA7-001` remains Verified.
- PR #174 is the approved cohort-blocking follow-up. At inspection on 2026-08-10 it was Draft,
  merge-clean at `cf28a2e164302628949986d762ca3928f482abf5`, and all six Fast CI jobs passed.
- The approved replacement checkpoint is `v1.0.0-alpha.8`; sequential-Alpha tooling permits only
  a one-step increment.
- The read-only Alpha.8 transition preview currently fails because the sequential-Alpha writer
  expects pre-publication wording that the Alpha.7 truth reconciliation correctly replaced in
  `docs/guides/known-limitations.md`.
- Beta-transition and readiness paths still contain active Alpha.7-only checks. Alpha.8 cannot
  become the Beta evidence baseline until those checks accept the exact current eligible v4 Alpha
  while continuing to reject Alpha.6 and mismatched evidence.
- M3-EVID-005 remains tracked by Issue #70. Synthetic data cannot satisfy its real-user cohort
  thresholds.

## Task Map

1. `08-10-alpha8-checkpoint-qualification` owns PR #174 disposition, release-authority repair,
   exact Alpha.8 build-once qualification, public reverification, and provider/host evidence.
2. `08-10-alpha8-cohort-beta-evidence` starts only after child 1 is Verified and owns invited-user
   execution, body-free evidence, Issue #70, and Beta-readiness refresh.

This parent owns cross-child ordering and the final M3-to-M4 entry decision. It is not an
implementation target.

## Requirements

1. Preserve Alpha.7 tag, source, artifacts, Issues, and evidence as immutable history.
2. Add a distinct P0 Roadmap/GitHub owner for Alpha.8 qualification instead of reopening or
   repurposing `M3-ALPHA7-001`.
3. Merge PR #174 only through protected checks and bind every later action to the merged source.
4. Repair sequential-Alpha and Beta-readiness authority at the shared release owner, with focused
   positive and negative regressions before any `--write`, tag, or publication action.
5. Build Alpha.8 once, qualify the supported native matrix, promote the same bytes, download and
   publicly reverify them, then rerun exact-host provider evidence.
6. Run the invited cohort only on those exact public Alpha.8 bytes and resource/Pack digests.
7. Refresh `release/beta-readiness.json` and enter `M4-READY-001` only after Issue #70 meets every
   measured threshold with zero unresolved supported blocker.

## Constraints

- No new product feature, legacy compatibility, hidden migration, support-platform expansion, or
  submission automation belongs to this path.
- No release write, merge, tag, promotion, or publication may bypass its explicit operator gate.
- No private Application body, Profile, Evidence, transcript, local path, credential, or provider
  token may enter Git, GitHub, logs, or release artifacts.
- Local packages, PR builds, or synthetic users never qualify public or cohort evidence.
- A changed candidate, resource digest, or blocker fix invalidates affected downstream evidence
  and activates the bounded rerun path in `M3-EVID-003`.

## Acceptance Criteria

- [ ] The Alpha.8 Roadmap/GitHub work item, milestone, owner, dependencies, and evidence contract
      are explicit without changing Alpha.7 history.
- [ ] PR #174 has a reviewed protected disposition and its exact merged source is recorded.
- [ ] Sequential-Alpha preview supports the published-current source form, Beta gates accept exact
      eligible Alpha.8 evidence, and negative tests reject Alpha.6, stale tags, sources, runs,
      contracts, resources, and user evidence.
- [ ] Exact public Alpha.8 artifacts pass build-once native qualification, promotion, independent
      download/reverification, and provider/host dogfood.
- [ ] Issue #70 records at least 5 invited and 8 cumulative users, 20 completed flows, both Packs,
      mixed-Application coverage, two academic and three non-academic scenario families, at least
      80% unassisted completion, complete traceability and backup/restore success, zero unsupported
      claims, reviewed exclusions, and an exact body-free evidence-note digest.
- [ ] Beta readiness binds exact Alpha.8 and M4 remains blocked until all M3 exit assertions pass.

## Out of Scope

- Beta.1 publication or feature-freeze activation.
- Post-1.0 Packs, marketplaces, Windows/Linux public GUI support, or hosted services.
- Automatic GitHub synchronization or fabricated cohort evidence.

## References

- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md` §§5, 11.6, 11.7, 12, and 19
- `docs/release/stage-transitions.md`
- `release/stage-transition-policy.json`
- `release/provider-dogfood.json`
- `release/beta-readiness.json`
- `.trellis/spec/guides/project-control.md`
