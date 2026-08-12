# Project Control

## Authority

Trellis controls the current development task, its planning artifacts, relevant spec context, and
cross-session journal. It does not replace CanISend's existing authorities:

1. Accepted ADRs own product, architecture, trust, and platform decisions.
2. `Cargo.toml`, `release/*.json`, `docs/contracts/*.json`, tags, and exact artifacts own machine
   and release facts.
3. `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md` owns 1.0 work ordering and gates.
4. GitHub Issues and milestones are the public projection of P0/P1 Roadmap work.
5. Trellis tasks describe the bounded execution needed to advance one of those authorities.

When sources disagree, stop the affected claim, correct the lower-authority projection, and add a
regression only when drift can be detected mechanically.

## Execution Model

- The Master Roadmap is the backlog and work-order authority; GitHub milestones and Issues are its
  public projection.
- `08-10-1-0-roadmap-trellis-control` is the 1.0 programme parent. Create delivery children only
  for Roadmap work that is Ready or In progress; do not mirror the full GitHub backlog.
- Keep one critical-path child current. At most one architecture/safety child and one
  qualification/evidence child may run beside it.
- `M3-ARCH-001` / Issue #182 is Verified and archived. `M3-ALPHA9-001` / Issue #183 is the next
  critical-path child but remains Planned until its entry and release actions are explicitly
  authorized. `M3-EVID-005` / Issue #70 remains the evidence child but waits for real invited users
  and the next exact qualified Alpha. Create `M4-READY-001` / Issue #71 only after the M3 exit gate
  passes; later Beta, RC, and Stable work remains in GitHub until its entry gate is satisfied.

## Planning Horizons

- **Current:** review and explicitly authorize entry for `M3-ALPHA9-001` from the exact protected
  source without treating merged source as qualified release evidence.
- **Near term:** qualify that exact Alpha.9, then complete the mixed-Application cohort and provider
  evidence, close supported blockers, refresh Beta readiness, qualify Beta.1, and activate feature
  freeze.
- **Medium term:** qualify two distinct clean RC matrices and the upgrade, documentation,
  package-manager, accessibility, feedback, and final-notes evidence classes.
- **Long term:** explicitly authorize and publish exact `v1.0.0`, establish 1.0.x support, then
  consider deferred packs/platforms only from measured demand and a new accepted boundary.

## Task Rules

- Give a Roadmap-linked task its Roadmap ID, GitHub Issue and milestone, priority, owner role,
  dependencies, authoritative files, expected evidence, and verification tier. Keep requirements
  in `prd.md` and the compact searchable projection in `task.json.meta`.
- One Trellis task should produce one independently verifiable outcome. Split only when children
  can be planned, checked, and archived independently.
- Do not copy the full Roadmap into `.trellis/`; link it and retain only task-specific context.
- Do not mark a task complete from local output alone when its owner is protected CI, native
  qualification, public bytes, user evidence, or explicit maintainer authorization.
- Apply the minimum-sufficient checks in `.trellis/spec/backend/quality-guidelines.md`.

## Lifecycle Mapping

| Trellis state | Roadmap/GitHub state | Rule |
|---|---|---|
| `planning` | Planned or Ready | The PRD names the state and any missing entry evidence |
| `in_progress` | In progress | Start only after planning review and entry-gate confirmation |
| archived | Verified or Deferred | Archive only after linked public/evidence state is reconciled |

Trellis completion does not prove release qualification. When a lower projection disagrees with
an ADR, machine fact, or the Master Roadmap, keep the affected transition blocked and correct the
lower projection before continuing.
