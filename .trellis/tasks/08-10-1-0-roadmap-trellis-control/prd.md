# Integrate the 1.0 Roadmap with Trellis project control

## Goal

Use Trellis as the execution layer for the existing 1.0 Master Roadmap without creating a second
roadmap, duplicating the GitHub backlog, or weakening release authority. A maintainer should be
able to identify the single current task, its Roadmap/GitHub identity, its entry and exit evidence,
and the next gated task from repository state alone.

## Background

- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md` is the sole active top-level Roadmap.
- Accepted ADRs, release contracts, ledgers, exact tags, runs, and artifacts remain higher-fidelity
  authorities for their owned facts.
- GitHub currently projects 26 open Roadmap Issues across Alpha.7, Beta, RC, and Stable. Only
  Issue #70 (`M3-EVID-005`) is labelled `state:in-progress`.
- The current Trellis child task `08-10-alpha7-followup-cohort-entry` owns the Alpha.7 cohort and
  Beta-entry decision and is linked to this parent task.
- Several future GitHub Issue titles still describe superseded v2/v3 or "dual-pack" wording. They
  must be reconciled before those Issues become active; their existence does not change current
  v4 Roadmap authority.

## Requirements

1. Preserve this authority order: accepted ADRs and machine/release facts, Master Roadmap, GitHub
   projection, Trellis execution task, session journal.
2. Keep one standing Trellis parent for the 1.0 delivery programme and create children only for
   work that is Ready or In progress. Do not mirror all GitHub backlog Issues into Trellis.
3. Require each Roadmap-linked child task to identify its Roadmap ID, GitHub Issue and milestone,
   priority, owner role, dependencies, authoritative files, expected evidence, and verification
   tier in `prd.md` or `task.json.meta`.
4. Map Trellis lifecycle to Roadmap state explicitly. `planning` may represent Planned or Ready;
   `in_progress` represents In progress; archive is allowed only after the Roadmap/GitHub work is
   Verified or explicitly Deferred.
5. Keep one critical-path child current. Permit at most one additional architecture/safety task
   and one qualification/evidence task, matching the Roadmap WIP rule.
6. Treat GitHub as the public projection, not the execution memory. GitHub state changes require
   protected evidence and must not be inferred from a local Trellis status.
7. Make the next-task rule deterministic: finish the current M3 cohort/Beta-readiness gate, then
   activate `M4-READY-001`; later Beta, RC, and Stable work remains in GitHub until its entry gate
   is satisfied.
8. Update the local Trellis project-control guide and active Roadmap governance text together so
   later sessions use the same model.

## Constraints

- Do not introduce a new tracker, generated dashboard, synchronization daemon, custom task status,
  or lifecycle hook.
- Do not copy the full Roadmap or all GitHub Issues into `.trellis/`.
- Do not edit GitHub Issues, milestones, labels, or rulesets without explicit authorization for
  those external changes.
- Preserve all existing uncommitted user work and historical plan wording.

## Acceptance Criteria

- [x] The Master Roadmap remains the only active top-level Roadmap and documents Trellis as its
      just-in-time execution layer.
- [x] `.trellis/spec/guides/project-control.md` defines the authority stack, required task fields,
      lifecycle mapping, WIP limit, drift rule, and next-task rule without duplicating backlog.
- [x] A single `1-0-roadmap-trellis-control` parent exists and links the current Alpha.7 cohort
      child task.
- [x] The current child identifies `M3-EVID-005`, Issue #70, Alpha.7 milestone, P0 validation
      ownership, dependencies, authoritative files, expected evidence, and verification tier.
- [x] Future M4/M5/M6 Trellis children are absent until their Roadmap entry gates are met.
- [x] Known v2/v3 and "dual-pack" GitHub projection drift is recorded for later authorized sync
      before affected Issues become active.
- [x] `git diff --check` passes; because the active Roadmap and shared project-control contract
      change, `cargo run -p xtask --locked -- release check` passes once on the final head.

## Out of Scope

- Product, Workspace, Agent, Pack, CLI, MCP, or desktop implementation.
- A second roadmap or a complete Trellis copy of the 26 open GitHub Issues.
- Automatic GitHub synchronization.
- GitHub Issue, milestone, label, or ruleset mutation in this task.
