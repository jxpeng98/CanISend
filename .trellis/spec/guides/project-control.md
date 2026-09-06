# Project Control

Updated: 2026-09-04 — owner-approved direct development without Trellis skills or hooks.

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
- Keep one integration slice active. Independent preparatory work may overlap only with explicit
  file ownership and stable contracts; do not overlap competing storage or approval changes.
- A local Roadmap row may carry owner, scope, acceptance, and pending Issue projection until public
  synchronization is authorized. Pending projection never counts as completed release governance.

## Current sequence

ADR-RN-0023 supersedes App-first delivery priority. Close out R0/R1 source and retain partial R2
with its open gates; do not claim R2 acceptance. Reconcile the current feature branch with the
owner-confirmed Beta integration target, preserving its version/release records and user changes.
The master roadmap maps LF-C01–12: CLI boundary audit, independent build/resources/install, trusted
headless approval, one Host plus same-device resumption, then bounded local collaboration.
Cross-device work and unfinished embedded R2-R6 work are deferred.

Public Beta.1, private Beta.2 candidate evidence, active freeze, and historical observations retain
their exact identities. Qualify new CLI bytes and explicitly test any future cohort-binding change;
the current validator and user thresholds remain unchanged by a direction document.

## Validation and release control

- Follow `../backend/quality-guidelines.md`; one invariant has one primary test owner.
- Reuse existing dual-Pack fixtures, protocol processes, adapter smokes, and release tooling.
- Run changed-language checks and focused positive/negative regressions during edits. Run Tier 2
  once on the final applicable PR head; protected Fast CI owns the complete workspace suite.
- Native and extended assurance remain with their scheduled or exact-candidate owners. Recheck a
  passing assertion only after a relevant change or a newly observed failure.
- Report source implementation, protected CI, exact artifact qualification, and user validation
  separately. A checked plan or local command cannot substitute for the other evidence classes.
- Policy-bearing adapter removal, `AGENTS.md`, `PRODUCT.md`, and spec changes still need exact
  post-baseline freeze exceptions when committed. Do not fabricate a future commit hash, broaden
  automatic path exemptions, or alter the ledger merely to make an uncommitted change look released.

## Retained project material

`.trellis/tasks/`, `.trellis/spec/`, and `.trellis/workspace/` remain readable documents with stable
paths. Existing task JSON and context manifests are compatibility metadata, not execution gates.
Keep completed records and private-data exclusions intact; no forced migration or archival is needed.
The old workflow, scripts, template hashes, and upstream license are retained for provenance or
explicit manual use. Do not regenerate removed skills/hooks with `trellis init` or `trellis update`
unless the owner explicitly reinstalls Trellis. User-global tools and product Agent v4 Skills are
outside this removal.
