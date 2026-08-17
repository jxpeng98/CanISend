# Complete Alpha.9 cohort and Beta evidence

## Goal

Collect and validate the real, consented, body-free user evidence required by Issue #70 using only
the exact publicly reverified Alpha.9 checkpoint.

## Entry Gate

- `M3-ALPHA9-001` is Verified.
- Public Alpha.9 tag, source, release run, artifact, manifest, Agent/Skills/resources, and both Pack
  digests agree with the provider-dogfood record.
- No unresolved supported P0/P1 blocker remains.

## Requirements

1. Run 5–8 invited users and reach at least 8 cumulative users and 20 completed flows.
2. Cover a mixed-Application Workspace, both built-in Pack IDs, two academic and three non-academic
   scenario families, supported hosts/languages/inputs, recovery, accessibility, and the
   no-submission boundary.
3. Measure unassisted completion at or above 80%, 100% audited claim traceability, 100% measured
   backup/restore success, and zero unsupported claims using explicit numerators/denominators.
4. Record exclusions only for withdrawal or documented external-host outage; product failures stay
   in the denominator and receive minimum-safe Issue links.
5. Commit a reviewed body-free cohort note and exact measured JSON, then refresh Beta readiness
   dry-run first and write only from a clean worktree after authorization.

## Acceptance Criteria

- [ ] Every count and token is backed by the reviewed body-free note; no synthetic user is counted.
- [ ] All thresholds and coverage classes pass on exact Alpha.9.
- [ ] Every failure has a P0/P1 disposition and changed-byte reruns use Issue #68.
- [ ] Issue #70 and `release/beta-readiness.json` agree on exact build, metrics, exclusions,
      contracts, resources, Packs, evidence-note digest, and zero unresolved blockers.
- [ ] `M4-READY-001` is the next task; Beta.1 remains unprepared and unpublished.

## Constraints

- This task cannot manufacture human evidence or change product bytes.
- No private content, paths, credentials, transcripts, or provider tokens are retained.
- A blocker that changes bytes returns to a new checkpoint task before affected evidence resumes.

## References

- Issue #70 / `M3-EVID-005`
- Parent `08-10-1-0-roadmap-trellis-control`
- `release/provider-dogfood.json`
- `release/beta-readiness.json`
- `scripts/refresh_beta_readiness.sh`

The exact Alpha.9 build entry gate is satisfied. This task remains Planned until explicit cohort
consent and the owner schedule exist; no invitation, provider send, or private-data action is
implied by the handoff.
