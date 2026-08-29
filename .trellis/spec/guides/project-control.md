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
- `M3-ARCH-001` / Issue #182 is Verified and archived. `M3-ALPHA9-001` / Issue #183 is Verified
  against exact public Alpha.9 and its body-free host evidence. `M3-DEPS-001` / Issue #195 and
  `M3-HEADLESS-001` / Issue #193 are Verified through protected PR #196 and archived.
  `M3-ALPHA10-001` / Issue #194 is Verified by exact public Alpha.10 and the Codex-first
  qualification evidence. `M4-READY-001` / Issue #71 and `M4-FREEZE-001` / Issue #72 are
  Verified through protected PRs #201 and #202. `M4-STAGE-001` / Issue #73 is the active
  implementation child; Beta.1 source is staged at `beta-qualifying`, pending protected CI and
  merge. `M4-CANDIDATE-001` / Issue #74 is next after verification.
  `M3-EVID-005` / Issue #70 runs on public Beta.1 and remains required before RC.1;
  later RC and Stable work remains in GitHub until its entry gate is satisfied.

## Planning Horizons

- **Current:** pass protected CI, merge, and verify the exact Beta.1 source-stage transition.
- **Near term:** build, qualify, and publish one build-once Beta.1,
  then run the mixed-Application invited cohort on public Beta.1 without treating synthetic
  dogfood as user evidence.
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
