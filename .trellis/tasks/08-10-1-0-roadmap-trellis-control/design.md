# Design: Roadmap-backed Trellis project control

## Boundary

This change is a governance integration, not a new project-management system. Existing authorities
stay in place; Trellis receives only the context required to execute the current bounded outcome.

## Management layers

| Layer | Authority | Responsibility |
|---|---|---|
| Product and release truth | ADRs, contracts, ledgers, tags, runs, artifacts | Decisions and exact facts |
| Work ordering | 1.0 Master Roadmap | Milestones, dependencies, gates, priorities |
| Public projection | GitHub milestones and Issues | Reviewable public state and evidence links |
| Active execution | Trellis parent and just-in-time child tasks | PRD, design, implementation plan, context, journal |
| Delivery evidence | PR, protected CI, native qualification, dated notes | Proof required to become Verified |

Lower layers may link higher layers but may not override them.

## Portfolio shape

`08-10-1-0-roadmap-trellis-control` is the programme parent. It owns the execution model and the
cross-child integration review, but no implementation. One child applies this governance contract;
only Ready/In-progress Roadmap work becomes a delivery child. The existing
`08-10-alpha8-cohort-beta-evidence` task retains its stable ID and is the current Roadmap delivery
child.

Future work remains in GitHub until its entry gate is satisfied:

1. `M3-EVID-005` / Issue #70 — post-Beta.1 cohort evidence required before RC.1 planning.
2. M5 RC.1 work — next only after Issue #70 becomes Verified.
3. Remaining M5 and M6 work — remains backlog until its milestone entry gates pass.

## Task contract

Each active child keeps task-local details in `prd.md` and a compact searchable projection under
`task.json.meta`:

- `roadmap_id`
- `github_issue`
- `github_milestone`
- `owner_role`
- `dependencies`
- `authoritative_files`
- `expected_evidence`
- `verification_tier`

No Trellis script changes are required because `meta` is already the supported extension point.

## Lifecycle mapping

| Trellis | Roadmap/GitHub | Rule |
|---|---|---|
| `planning` | Planned or Ready | PRD states which one and names missing entry evidence |
| `in_progress` | In progress | Only after planning review and `task.py start` |
| archived | Verified or Deferred | Archive only after linked evidence/state is reconciled |

Trellis completion never proves a release claim. Protected CI, exact candidate evidence, public
artifacts, or maintainer authorization remains the owner where the Roadmap requires it.

## Drift handling

When Roadmap, GitHub, and Trellis disagree, stop the affected transition, correct the lower
authority, and retain the discrepancy in the current task until reconciled. The current audit has
already found future GitHub Issue wording that still names v2/v3 or "dual-pack" semantics; it is
recorded but not remotely changed by this local task.

## Rejected complexity

- No automatic GitHub sync: protected state needs human/evidence review.
- No generated dashboard: the Master Roadmap, `task.py list`, and GitHub already provide the three
  required views.
- No custom statuses or scripts: existing Trellis lifecycle plus task metadata is sufficient.
- No 26-task import: it would create a second stale backlog.

## Rollback

Remove the parent/child link and revert the two documentation changes. No product data, release
ledger, GitHub state, or runtime behavior is affected.
