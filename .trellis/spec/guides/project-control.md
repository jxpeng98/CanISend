# Project Control

Updated: 2026-09-07 — owner-approved separation of development checks and release qualification.

## Authority

1. Accepted ADRs own product, architecture, trust, and platform decisions.
2. `Cargo.toml`, `release/*.json`, `docs/contracts/*.json`, tags, and exact artifacts own machine
   and release facts.
3. `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md` owns 1.0 ordering and gates.
4. GitHub Issues and milestones project the Roadmap publicly; do not invent synchronization.
5. Existing implementation plans hold bounded scope, acceptance, evidence, and the next action.

When sources disagree, correct the lower-authority projection before advancing the affected claim.
Do not rewrite historical release identities or weaken a product control to accelerate delivery.

## Direct execution

- Read `AGENTS.md`, the current Roadmap slice, its existing plan, and the relevant backend spec.
- Continue authorized work directly. A task directory, status field, phase transition, subagent,
  journal entry, or bookkeeping commit is not a prerequisite.
- Keep one short execution checklist per independently verifiable outcome. Existing PRD/design/
  implement files may be maintained in place; do not duplicate them into another tracking system.
- Small changes need only a scope and acceptance note. Add design detail only for unresolved API,
  data, consent, migration, or recovery contracts. Ask the owner only when such a decision cannot
  be resolved from existing authority, or when an external/irreversible action is not authorized.
- Trace the actual owner and all callers, implement one complete slice, run its smallest meaningful
  checks, inspect the full diff, and update evidence plus the next step once.
- Keep each integration slice reviewable. Independent implementation may proceed when its code
  dependencies and automated checks pass; pending human qualification is not a development lock.
  Coordinate overlapping storage or approval edits through explicit file ownership.
- Reuse the current local branch. Branches/worktrees are for conflicting parallel work or risky
  experiments, not task bookkeeping. Integrate completed work locally; prefer fast-forward merges
  and preserve uncommitted work. Do not bulk-merge or delete unrelated historical branches.
- Commit meaningful local milestones. A PR is optional until external review is requested or a
  protected remote integration requires it. Consolidate related local work into one such review;
  do not change remote protections or force-push as a shortcut.
- A local Roadmap row may carry owner, scope, acceptance, and pending Issue projection until public
  synchronization is authorized. Pending projection never counts as completed release governance.

Private Beta.2 baseline evidence is retained: M4-BETA2-001/002 reached protected main through
PRs #219/#220; M4-BETA2-003 binds private candidate run `33824463477`, artifact `9920609356`,
and protected source `2ae2b507b953eef3101aa9689bd60f91a0046605`. This is not public qualification.

## Current sequence

ADR-RN-0023 supersedes App-first delivery priority. Close out R0/R1 source and retain partial R2
with its open gates; do not claim R2 acceptance. Reconcile the current feature branch with the
owner-confirmed Beta integration target, preserving its version/release records and user changes.
The master roadmap maps LF-C01–12: CLI boundary audit, independent build/resources/install, trusted
headless approval, automated lifecycle/resumption, then bounded local collaboration. Actual Host
acceptance remains attached to the affected candidate and may run alongside later development.
Cross-device work and unfinished embedded R2-R6 work are deferred.

Public Beta.1, private Beta.2 candidate evidence, active freeze, and historical observations retain
their exact identities. Qualify new CLI bytes and explicitly test any future cohort-binding change;
the current validator and user thresholds remain unchanged by a direction document.

## Validation and release control

- Follow `../backend/quality-guidelines.md`; one invariant has one primary test owner.
- Reuse existing dual-Pack fixtures, protocol processes, adapter smokes, and release tooling.
- Run focused regressions when they resolve a bug or protect a trust/data boundary. Batch broader
  changed-language checks and Tier 2 at a completed feature, public-contract change, substantial
  merge/conflict resolution, or candidate milestone. Do not run a suite after every edit or commit.
  A clean fast-forward with unchanged tested content needs no duplicate run. Fast CI owns the
  complete remote workspace suite when work is synchronized.
- Tier 2 is `xtask source check`. `xtask release check`, fresh human Host evidence and exact
  freeze dispositions belong to candidate qualification. Their pending state must be recorded;
  it does not require repeated approval or evidence-only commits during ordinary development.
- Native and extended assurance remain with their scheduled or exact-candidate owners. Recheck a
  passing assertion only after a relevant change or a newly observed failure.
- Report source implementation, protected CI, exact artifact qualification, and user validation
  separately. A checked plan or local command cannot substitute for the other evidence classes.
- The full release check retains exact post-baseline freeze validation before qualification.
  Collect required dispositions against actual candidate history there; never fabricate a future
  commit hash or relabel an unqualified development change as released.

## Retained project material

`.trellis/tasks/`, `.trellis/spec/`, and `.trellis/workspace/` remain readable documents with stable
paths. Existing task JSON and context manifests are compatibility metadata, not execution gates.
Keep completed records and private-data exclusions intact; no forced migration or archival is needed.
The old workflow, scripts, template hashes, and upstream license are retained for provenance or
explicit manual use. Do not regenerate removed skills/hooks with `trellis init` or `trellis update`
unless the owner explicitly reinstalls Trellis. User-global tools and product Agent v4 Skills are
outside this removal.
