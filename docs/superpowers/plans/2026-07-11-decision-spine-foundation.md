# Decision Spine Foundation Implementation Plan

**Status:** In progress — Criteria/Confirm and Evidence/Match vertical slices locally accepted

**Date:** 2026-07-11

**Branch:** `feat/decision-spine-foundation`

**Roadmap:** `docs/superpowers/specs/2026-07-11-cli-first-workflow-optimization-roadmap.md`

## Goal

Implement Stage 2 of the CLI-first roadmap as a sequence of reviewable vertical slices. The first slice establishes
stable criteria and a resumable Confirm stage; the second establishes stable private Evidence catalogs and durable
proposed matching. Later slices add user-owned application decisions and briefs, and advert-driven document planning
without entering Draft.

## Fixed Decisions

- Python, the file workspace, the existing CLI, and `canisend.agent/v1` remain the platform contract.
- Parsed Job v1 remains unchanged; Stage 2 semantic fields live in separate versioned artifacts.
- Stable IDs are derived from normalized source receipts plus normalized parser interpretation, never list positions,
  line numbers, sibling counts, or user-corrected prose.
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

- [x] Extract stage-specific Parse behavior behind an internal adapter with no Parse behavior change.
- [x] Reconstruct state from the latest valid manifest for every implemented stage.
- [x] Enforce current direct dependencies before preparing a downstream stage.
- [x] Run Confirm through candidate validation, atomic promotion, immutable evidence, cache, and drift protection.
- [x] Bind TaskSpecs to preparation receipts and active output baselines; allow only one active task.
- [x] Route candidate bytes through a guarded submit service that rejects symlink and hard-link aliases.
- [x] Add explicit cancellation so stale or abandoned tasks cannot deadlock upstream recovery.
- [x] Keep Stage 2 phases represented as `phase=unknown` in AgentResponse 1.0 and expose the real stage as a scalar
  extension.

## Task 4: Stable Evidence Catalog And Durable Match

**Status:** Locally accepted on 2026-07-11. This accepts only the Evidence/Match slice, not Stage 2 as a whole.

### Task 4.0: Freeze The Evidence Read And Privacy Boundary

- [x] Accept ADR-011: keep TaskSpec v1 job-relative and use a run-scoped, immutable, job-local Evidence snapshot.
- [x] Distinguish the private data plane from privacy-safe workflow control records.
- [x] Reject a TaskSpec v1 workspace `read_scope`, parent traversal, hidden direct profile reads, and platform-specific
  API dependencies for this slice.
- [x] Add implementation tests proving the accepted boundary; accepting the ADR is not implementation acceptance.

### Task 4.1: Stable Evidence Contract

- [x] Add a strict, separately versioned Evidence catalog model and `schemas/evidence-catalog.schema.json` without
  changing the frozen Parsed Job v1 or AgentResponse v1 contracts.
- [x] Freeze canonical Evidence ID normalization from semantic content and kind, excluding list position, legacy item
  locator, path, section, input order, and job identity.
- [x] Keep legacy citations such as `profile/generated/cv.evidence.md#Teaching/cv-001` as display locators and leave
  existing pipeline rendering unchanged.
- [x] Define deterministic duplicate handling and canonical kind mapping for legacy values such as `dated-entry` and
  `llm-augmented`.
- [x] Distinguish valid empty, unavailable, malformed-input failure, and stale-source unavailability so missing input
  is never interpreted as missing applicant evidence.

### Task 4.2: Safe Run-Scoped Evidence Materialization

- [x] Compute Evidence status fingerprints read-only, then have the core service write one immutable
  `workflow/runs/<run-id>/inputs/evidence-snapshot.json` during prepare.
- [x] Point the Evidence TaskSpec `inputs` and `allowed_reads` only at the real job-local snapshot.
- [x] Reject absolute generated-file paths, parent traversal, path escapes, symlinks, dangling symlinks, hard-link
  aliases, non-regular files, unsafe external profile roots, and bounded-input violations before reading bodies.
- [x] Recompute the live profile-evidence fingerprint at submit/apply so a post-prepare change makes the task stale.
- [x] Keep evidence bodies only in the private snapshot, Evidence candidate, and promoted catalog; exclude them from
  state, TaskSpec, receipts, manifests, errors, stdout, AgentResponse, and Match output.

### Task 4.3: Resumable Evidence Stage

- [x] Register Evidence as deterministic-only with `evidence_catalog.json` as its single authoritative output.
- [x] Validate the snapshot, catalog schema, semantic IDs, locators, content receipts, deterministic ordering, job
  identity, and input fingerprint before promotion.
- [x] Prove prepare, guarded submit, apply, cancel, cache, output drift, terminal-action competition, failure recovery,
  and fresh-session reconstruction through the shared runtime.
- [x] Mark Evidence implemented only after its full runtime and privacy acceptance suite passes.

### Task 4.4: Durable Deterministic Match

- [x] Make Match read only current job-local `criteria.json` and `evidence_catalog.json`; keep both direct dependencies
  required and current.
- [x] Generate locator-only `criterion_matches.json` with deterministic ordering, stable tie-breaks, fixed privacy-safe
  gaps, exact matcher strategy/version provenance, and `review_state=proposed`.
- [x] Require every current catalog criterion to have exactly one classification and reject unknown, duplicate,
  omitted, or extra criterion IDs against the current Criteria catalog.
- [x] Require every supported classification to resolve current Evidence IDs and receipts; require explicit gaps for
  `missing` and `unknown` without copying evidence text.
- [x] Keep unavailable Evidence distinct from a valid catalog with no supporting item.
- [x] Mark Match implemented only after its full prepare, guarded submit, apply, cancel, cache, drift, stale-input,
  terminal-action, recovery, and fresh-session slice passes.

### Task 4.5: Compatibility And Productization

- [x] Keep legacy `canisend run`, display citations, Markdown views, LLM flags, Typst protection, git behavior, Parsed
  Job v1, TaskSpec v1, and AgentResponse v1 compatible.
- [x] Represent Evidence with the existing AgentResponse evidence phase and Match as `phase=unknown` plus scalar
  `canisend.stage_id=match`; expose only privacy-safe counts and artifact references.
- [x] Prove Evidence and Match do not call a configured provider, network, MCP transport, or platform API.
- [x] Package the Evidence schema and extend installed-wheel, CI, release, and TestPyPI fake-data smoke through
  `Parse -> Confirm -> Evidence -> Match` before accepting the slice.
- [x] Update README, Skills, focused job-fit guidance, examples, changelog, and workspace references only after the
  implementation contract and tests pass.

## Evidence/Match Slice Acceptance Matrix

Locally accepted on 2026-07-11. Remote CI and TestPyPI still run their normal release gates; local acceptance does not
claim that a new distribution has already been published.

| Requirement | Accepted automated evidence |
|---|---|
| Stable Evidence identity | Normalization, locator-renumber, order, content-change, canonical-locator, and semantic-dedup tests in `test_evidence_stage.py` |
| Truthful TaskSpec scope | Runtime test proves Evidence reads only its run snapshot and Match only the two job-local catalogs |
| Safe materialization | Traversal, absolute path, external root, symlink, dangling symlink, hard-link, non-regular, race, fallback, and size tests |
| Evidence state is honest | Separate available, valid-empty, unavailable, malformed, missing-receipt, and stale-receipt tests |
| Private body stays in the data plane | Sentinels absent from every control record, Match output, AgentResponse, and ordinary CLI output |
| Evidence is resumable | Shared-runtime prepare, submit, apply, cancel, cache, drift, terminal race, recovery, and reconstruction suites |
| Every criterion is classified | Canonical rebuild enforces exact cross-catalog set equality and rejects omission, extra IDs, and tampering |
| Match references resolve | Opaque current catalog refs, input hashes, weak-ref, missing-gap, and distinct empty/unavailable gap tests |
| Match is deterministic | Input reorder, stable tie-break, semantic-ID ordering, schema receipt, and matcher-version tests |
| Dependency invalidation is precise | Profile changes stale Evidence, Match, and descendants while Parse and Confirm remain current |
| No provider or platform dependency | Provider/network/platform sentinels, deterministic-only registry checks, and portable safe-open fallback test |
| Existing users remain compatible | Legacy pipeline preserves structured outputs; Agent v1, TaskSpec v1, Markdown, Typst, and git suites remain green |
| Installed artifacts are complete | Packaged schema/resource checks and clean-wheel/CI/release Parse-to-Match fake-data smoke commands |

## Evidence/Match Slice Exit Review

The accepted slice adds only the deterministic Evidence and Match backbone. Private profile text may appear in the
immutable run snapshot, Evidence candidate, and promoted catalog, and is retained until the user removes the private
run or job. It does not appear in workflow control records or Match output. Typst-backed generated evidence from
older versions must be re-extracted when it lacks the source-hash receipt; changing a bound raw source likewise makes
the Evidence catalog unavailable until re-extraction.

Every Match classification remains `review_state=proposed`. No proposed result is an application decision, a claim
confirmation, or a package-readiness signal. Tasks 5-7—user-owned Decision, Application Brief, required-document
planning, view migration, and the full Stage 2 exit review—remain open.

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

## First Slice Exit Review

Locally accepted on 2026-07-11. The acceptance run includes:

- 561 tests passing on Python 3.11, 3.12, and 3.13, plus the Python 3.14 workspace development interpreter;
- stable-ID, source-section, ambiguity, correction-orphan, privacy, CAS, drift, cancellation, terminal-claim,
  failure-injection, and fresh-host recovery coverage;
- sdist/wheel build and packaged-resource validation;
- clean-wheel `run-example -> Parse -> Confirm` execution with preparation, submission, terminal-claim, promotion, and
  manifest evidence present;
- automated built-wheel and TestPyPI Parse-to-Confirm smoke commands in CI/release workflows.

This checkpoint accepts only Tasks 0-3 and the first-slice compatibility boundary. Task 4 Evidence/Match was accepted
separately in the second slice above. Decision, Brief, required-document planning, view migration, and the final
Stage 2 exit review remain open in Tasks 5-7; Stage 2 as a whole is not complete.
