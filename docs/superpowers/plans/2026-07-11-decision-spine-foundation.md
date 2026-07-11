# Decision Spine Foundation Implementation Plan

**Status:** In progress

**Date:** 2026-07-11

**Branch:** `feat/decision-spine-foundation`

**Roadmap:** `docs/superpowers/specs/2026-07-11-cli-first-workflow-optimization-roadmap.md`

## Goal

Implement Stage 2 of the CLI-first roadmap as a sequence of reviewable vertical slices. The first slice establishes
stable criteria and a resumable Confirm stage. Later slices add job-local evidence references, durable matching,
user-owned application decisions and briefs, and advert-driven document planning without entering Draft.

## Fixed Decisions

- Python, the file workspace, the existing CLI, and `canisend.agent/v1` remain the platform contract.
- Parsed Job v1 remains unchanged; Stage 2 semantic fields live in separate versioned artifacts.
- Stable IDs are derived from normalized source receipts, never list positions, line numbers, or corrected prose.
- `confirmed_corrections.yaml`, `application_decision.yaml`, and `application_brief.yaml` are user-owned.
- Core-owned projections may become stale; user-owned values are preserved and marked for review.
- AgentResponse 1.0 carries artifact references and scalar extensions only.
- The legacy `canisend run` remains compatible until structured outputs can replace its Markdown machine state.
- Stage 2 does not generate application-facing prose or claim submission readiness.

## Task 0: Freeze Ownership And Semantic Identity

- [x] Accept ADR-009 for stable semantic identifiers and separate projections.
- [x] Accept ADR-010 for user-owned corrections, decisions, and briefs.
- [x] Audit existing criteria, evidence, matching, runtime, CLI, and workspace ownership boundaries.
- [x] Create a dedicated Stage 2 branch from the locally accepted Stage 1 branch.

## Task 1: Strict Semantic Contracts

**Create:**

- `src/canisend/decision_models.py`
- `schemas/criteria.schema.json`
- `schemas/criterion-matches.schema.json`
- `schemas/confirmed-corrections.schema.json`
- `schemas/application-decision.schema.json`
- `schemas/application-brief.schema.json`
- `schemas/required-document-plan.schema.json`
- `tests/test_decision_models.py`

- [x] Define strict Criterion, SourceSpan, EvidenceRef, EvidenceGap, CriterionMatch, correction, decision, brief, and
  document-plan models.
- [x] Distinguish unknown, unconfirmed, confirmed, corrected, and confirmed-empty states.
- [x] Reject unsafe paths, malformed IDs and hashes, duplicate semantic IDs, inconsistent states, and unknown fields.
- [x] Package each public schema and validate model dumps against it.

## Task 2: Criteria Projection And Confirm Overlay

**Create:**

- `src/canisend/stages/confirm_stage.py`
- `tests/test_confirm_stage.py`

- [x] Project Parsed Job v1 criteria into stable `criteria.json` records.
- [x] Resolve one-based source spans and record ambiguity instead of silently choosing a source occurrence.
- [x] Apply valid user confirmations/corrections without changing stable criterion IDs.
- [x] Preserve unmatched corrections as orphaned reconciliation actions.
- [x] Fingerprint only the criteria projection, Confirm contract/schema, and validated correction overlay.
- [x] Prove reordering criteria or inserting unrelated advert lines preserves IDs.

## Task 3: Multi-Stage Runtime Adapter And Confirm Slice

**Modify:**

- `src/canisend/stage_runtime.py`
- `src/canisend/stage_registry.py`
- `src/canisend/stage_agent.py`
- `src/canisend/cli.py`
- existing stage tests plus Confirm runtime/CLI tests

- [ ] Extract stage-specific Parse behavior behind an internal adapter with no Parse behavior change.
- [ ] Reconstruct state from the latest valid manifest for every implemented stage.
- [ ] Enforce current direct dependencies before preparing a downstream stage.
- [ ] Run Confirm through candidate validation, atomic promotion, immutable evidence, cache, and drift protection.
- [ ] Keep Stage 2 phases represented as `phase=unknown` in AgentResponse 1.0 and expose the real stage as a scalar
  extension.

## Task 4: Stable Evidence Catalog And Durable Match

- [ ] Derive content-based EvidenceRef IDs without changing legacy display citations.
- [ ] Materialize a job-local evidence catalog so TaskSpec read scope remains truthful.
- [ ] Generate `criterion_matches.json` with deterministic ordering, explicit gaps, matcher provenance, and proposed
  review state.
- [ ] Require every essential criterion to have exactly one classification.
- [ ] Mark Match implemented only after its full prepare/apply/recovery slice passes.

## Task 5: User-Owned Corrections And Decision

- [ ] Add create-if-absent templates and strict safe-YAML validation.
- [ ] Add scoped compare-and-swap updates with revision/hash conflicts and immutable receipts.
- [ ] Keep `undecided` distinct from confirmed apply, hold, or skip.
- [ ] Preserve old values when their basis changes and return explicit review actions.
- [ ] Never copy rationale or correction bodies into workflow control records.

## Task 6: Application Brief And Required-Document Plan

- [ ] Model language, motivation, emphasis, exclusions, and document choices with field-level confirmation state.
- [ ] Bootstrap legacy language/style metadata only when a new brief is first created.
- [ ] Normalize advert document requirements without treating an empty list as confirmed none.
- [ ] Resolve required document tasks from confirmed requirements plus explicit brief overrides.
- [ ] Make unresolved or omitted required documents executable blockers for later Draft/Verify stages.

## Task 7: Views, Compatibility, Documentation, And Exit Review

- [ ] Keep legacy `run`, dry-run, LLM flags, Markdown, Typst protection, and git behavior compatible.
- [ ] Begin rendering fit/checklist Markdown from structured matches only after parity tests pass.
- [ ] Document the fresh-session Stage 2 CLI loop and manual YAML ownership boundaries.
- [ ] Update the changelog and packaged skill references.
- [ ] Run focused, full Python 3.11-3.13, distribution, resource, clean-wheel, privacy, and recovery checks.
- [ ] Complete the Stage 2 exit review only when every roadmap exit criterion has automated evidence.

## First Slice Acceptance Matrix

| Requirement | Automated evidence |
|---|---|
| Stable criterion identity | Reorder and unrelated-line insertion tests |
| Source receipts are reviewable | Exact span, missing, and ambiguous receipt tests |
| User correction does not change identity | Corrected-text projection test |
| Old correction is preserved | Orphan reconciliation test |
| Confirm is resumable | Fresh-runner prepare/apply and deterministic cache tests |
| Parse change invalidates Confirm | Dependency and fingerprint tests |
| Rejected Confirm candidate is harmless | Identity, hash, schema, path, and CAS tests |
| Multi-stage state is recoverable | Parse plus Confirm manifest reconstruction test |
| Agent v1 remains compatible | JSON schema, scalar extension, and privacy tests |
| Legacy pipeline is unchanged | Existing pipeline and CLI suites |
