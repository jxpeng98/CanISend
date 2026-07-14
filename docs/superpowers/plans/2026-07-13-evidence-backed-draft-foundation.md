# Evidence-Backed Draft Foundation Implementation Plan

**Status:** Locally accepted — Tasks 0–9 complete through document-scoped run ownership

**Date:** 2026-07-13

**Branch:** `feat/evidence-backed-draft-foundation`

**Roadmap:** `docs/superpowers/specs/2026-07-11-cli-first-workflow-optimization-roadmap.md`

## Goal

Implement the first Stage 3 vertical slice as one guarded Cover Letter Draft and Review path. The slice must make
claim support and review blockers durable without letting an agent/provider write authoritative or user-owned files,
without changing the frozen AgentResponse/TaskSpec contracts, and without claiming package readiness.

## Fixed Decisions

- Python, the private file workspace, existing CLI, resumable runtime, and `canisend.agent/v1` remain the platform.
- ADR-013 defines a structured Cover Letter Draft whose every prose block is one explicit Claim.
- `cover_letter_draft.json` is core-owned Tier 2; Draft candidates are never written directly to the target.
- `ReviewFindingV1` is independent from generation; Draft remains proposed until a later Review stage.
- Strong factual claims require current Evidence references; unsupported facts are executable blockers.
- Host-agent and configured-provider Draft reuse the same immutable task, validator, and promotion boundary.
- Legacy `canisend run`, Markdown, Typst, dry-run, LLM flags, and git behavior remain compatible in this slice.
- Stage 3 does not submit, upload, answer sensitive declarations, or claim package/submission readiness.

## Task 0: Freeze Claim, Review, Privacy, And Promotion Boundaries

- [x] Accept ADR-013 for structured Claim blocks and separate Review findings.
- [x] Keep generation, review, package, verify, and render as distinct stages.
- [x] Bind Draft to current Parsed Job, Criteria, Evidence, Match, Decision, Brief, and document-plan hashes.
- [x] Keep private prose/evidence/review bodies out of TaskSpec, state, receipts, errors, and AgentResponse.
- [x] Preserve legacy application-facing files until structured projection has its own parity tests.

## Task 1: Strict Draft And Review Contracts

**Create:**

- `src/canisend/draft_models.py`
- `schemas/cover-letter-draft.schema.json`
- `schemas/review-findings.schema.json`
- `tests/test_draft_models.py`
- `tests/test_draft_schema_parity.py`

- [x] Define strict DraftBasis, Claim, DraftSection, CoverLetterDraft, ReviewFinding, and ReviewFindings models.
- [x] Reject unsafe IDs/paths, unknown fields, duplicate references, unstable ordering, and inconsistent states.
- [x] Require evidence for strong/partial factual claims and blockers for partial/unsupported claims.
- [x] Require motivation/future-intent/role-context claims to use their declared non-Evidence basis.
- [x] Derive claim/finding IDs independently of list position and validate them during candidate acceptance.
- [x] Generate and package schemas with runtime/standalone parity tests.

## Task 2: Current-Basis Draft Candidate Validator

- [x] Add `src/canisend/stages/draft_stage.py` with currentness fingerprints and candidate validation.
- [x] Require current confirmed apply Decision, current Brief, blocker-free required-document plan, and a confirmed
  `prepare` task for one `cover_letter` document.
- [x] Resolve every Claim Criterion/Evidence reference against current catalogs and exact basis hashes.
- [x] Reject hidden prose, fabricated references, stale basis, wrong document kind, duplicate IDs, and unsafe bodies.
- [x] Accept review-required candidates with explicit blockers; never accept a candidate that claims ready/final.

## Task 3: Resumable Host-Agent Draft Slice

- [x] Register Draft as host-agent-only with one core-owned `cover_letter_draft.json` output.
- [x] Prepare a truthful Tier 2 TaskSpec containing only declared job-local inputs and body-free control metadata.
- [x] Require explicit private-source consent before an agent reads inputs or prepares candidate scratch JSON.
- [x] Reuse guarded submit, immutable TaskResult, validation, atomic promotion, cache, drift, cancel, and recovery.
- [x] Prove a rejected/stale Draft changes no authoritative, user-owned, Markdown, Typst, or profile file.

## Task 4: Deterministic Review Slice

- [x] Generate `review_findings.json` from current Draft and upstream receipts without invoking a provider.
- [x] Make unsupported factual claims, missing required sections, and deterministically detectable Brief-exclusion
  conflicts executable blockers.
- [x] Keep partial support, semantic proportionality, every non-factual Claim-kind classification, and broader
  cross-document consistency review-required when deterministic proof is not possible.
- [x] Expose body-free finding counts/codes/status through AgentResponse.

## Task 5: Compatibility Views And First-Slice Exit

- [x] Render compatibility Cover Letter Markdown/Typst from the promoted structured Draft only after parity tests.
- [x] Preserve edited Typst candidates, direct-library behavior, LLM flags, and git controls.
- [x] Update canonical skills, compatibility mirror, examples, changelog, roadmap, and release smoke.
- [x] Run focused/full Python 3.11-3.13 plus additional 3.14, distribution, privacy, recovery, and clean-wheel checks.
- [x] Record local acceptance only after every first-slice exit criterion has automated evidence.

## Task 6: User Review Dispositions And Cover Letter Readiness

- [x] Accept ADR-014 for non-waivable blockers, user-owned dispositions, and derived document readiness.
- [x] Add strict `review_dispositions.yaml` and scoped patch contracts bound to exact Draft/Review hashes.
- [x] Reuse explicit-consent revision/hash CAS, immutable receipts, and recovery for one finding at a time.
- [x] Expose body-free status and next actions across the CLI and AgentResponse boundary.
- [x] Derive `blocked`, `review_required`, `revision_required`, or `reviewed` without rewriting Draft or Review.
- [x] Bind reviewed disposition receipts into compatibility projections and the Cover Letter package gate.
- [x] Complete schema, privacy, recovery, projection, cross-version, distribution, and clean-wheel validation.

## Task 7: Configured-Provider Structured Draft

- [x] Accept ADR-015 for Tier 3 consent and one shared Draft validation/promotion path.
- [x] Add `configured_provider` to the Draft registry/runtime without changing host-agent behavior.
- [x] Build a bounded sections-and-claims provider proposal and derive the trusted envelope/Claim IDs in core.
- [x] Bind candidate generation mode to the immutable TaskSpec and reject cross-mode candidates.
- [x] Add `stage run --mode configured-provider --allow-provider-backed` with body-free Agent responses.
- [x] Resume submitted candidates without a second provider call and fail closed on provider, schema, or input drift.
- [x] Update packaged prompt/resources, canonical skills, compatibility mirror, roadmap, changelog, and smoke coverage.
- [x] Complete focused/full cross-version, distribution, privacy, recovery, and clean-wheel validation.

## Task 8: Required-Document Execution Fan-Out

- [x] Accept ADR-016 for a pure derived execution plan rather than a second mutable workflow state.
- [x] Add a strict `DocumentExecutionPlanV1` contract bound to the exact Required Document Plan hash.
- [x] Register available, planned, and unregistered document executor capabilities without claiming implementation.
- [x] Derive one deterministic work item per requirement with blocked, omitted, ready, or unavailable semantics.
- [x] Expose body-free aggregate status and next actions through AgentResponse and `documents status`.
- [x] Prove status inspection makes no writes and private labels/source/Brief bodies never enter the response.
- [x] Complete schema, repository, workspace, packaged-resource, cross-version, and clean-wheel validation.

## Task 9: Document-Scoped Stage Instance Ownership

- [x] Accept ADR-017 for `(stage, document_id)` Draft/Review identity and one active run per job.
- [x] Version document-scoped control records as 1.1 while preserving non-document 1.0 wire shapes.
- [x] Propagate the stable Required Document Plan ID through TaskSpec, result, submission, validation, manifest,
  terminal claim, promotion receipt, state, CLI, and AgentResponse.
- [x] Key state uniqueness/order, pending-task reuse, attempts, reconstruction, cache, and failure recovery by the
  composite identity.
- [x] Scope Draft-to-Review dependency and descendant invalidation to the same document.
- [x] Read and resume legacy 1.0 Draft/Review records, including pending tasks, without rewriting immutable evidence.
- [x] Add explicit `--document-id` selection while retaining sole-Cover-Letter auto-resolution.
- [x] Prove two same-stage document records can coexist and malformed, cross-stage, or mismatched IDs fail closed.
- [x] Complete schema, privacy, compatibility, cross-version, distribution, and clean-wheel validation.

## Task 0–5 Validation Snapshot

Tasks 0–5 and the first Cover Letter vertical slice were locally accepted on 2026-07-13:

- `python -m pytest -q`: 1009 passed independently on Python 3.11.15, 3.12.12, 3.13.14, and 3.14.2;
- structured Markdown/Typst injection, parity, stale/tamper, profile, direct-library, LLM, edited-source, package-gate,
  privacy, recovery, and release-contract tests: passed;
- a generated hostile-text Typst fixture compiled successfully with Typst 0.15.0;
- canonical/workspace skill mirror check and repository-boundary audit: passed with no findings;
- `uv build`, Twine metadata check, and packaged-resource check: passed;
- clean-wheel Python 3.12 install and installed-package smoke passed through 8 stage runs, 10 immutable user-mutation
  receipts, host-agent Draft submit/apply, deterministic Review, structured projection, Decision Spine byte
  preservation, and fail-closed `check-package`.

This historical snapshot accepts the first Cover Letter Draft/Review/projection slice only. At that checkpoint,
Task 6, configured-provider Draft, all-document orchestration, broader cross-document review, complete Stage 3, and
package readiness remained later work.

## Task 6 Validation Snapshot

Task 6 was locally accepted on 2026-07-14:

- `python -m pytest -q`: 1026 passed independently on Python 3.11.15, 3.12.12, 3.13.14, and 3.14.2;
- strict model/schema parity, non-waivable blocker, stale/orphan/tamper, explicit reset, CAS conflict, private-body,
  receipt-recovery, projection-race, and package-gate tests: passed;
- canonical/workspace skill mirror, generated-schema, packaged-resource, and repository-difference checks: passed;
- `uv build`, Twine metadata check, and packaged-resource check for the 0.2.0 wheel: passed;
- a clean Python 3.12 wheel installation passed the installed-package smoke through 8 stage runs and 14 immutable
  user-mutation receipts, reaching Cover Letter `reviewed` while the independent package gate remained fail-closed.

This acceptance establishes readiness for the current Cover Letter document only. It does not establish application
package readiness, rendering approval, manual-submission readiness, a remote CI result, or a published release.

## Task 7 Validation Snapshot

Task 7 was locally accepted on 2026-07-14:

- `python -m pytest -q`: 1033 passed independently on Python 3.11.15, 3.12.12, 3.13.14, and 3.14.2; after the
  final additive Agent error-code registration, 1034 passed on Python 3.14.2 and the 25 affected Agent/CLI contract
  tests passed on Python 3.11-3.13;
- Tier 3 consent-before-write/call, exact seven-input transmission, untrusted prompt delimiting, cross-mode candidate
  rejection, core-derived identity/Claim IDs, bounded strict provider output, private-body non-persistence,
  configuration/call/validation failure, mid-call drift, no-call cache, and submitted-candidate recovery tests passed;
- deterministic Review accepted the promoted configured-provider Draft without weakening Draft output-drift or
  current-basis enforcement;
- canonical/workspace skill mirror, regenerated schemas, packaged-resource, compile, diff, and repository-contract
  checks passed;
- source smoke and a clean Python 3.12 wheel installation each passed the complete Decision Spine with 8 successful
  stage runs and 14 immutable user-mutation receipts using configured-provider Draft;
- `uv build`, Twine metadata checks, and the 0.2.0 wheel packaged-resource check passed.

This local acceptance adds one configured-provider Cover Letter execution mode. It does not claim remote CI,
publication, all-document orchestration, broader cross-document review, application-package readiness, rendering
approval, or submission readiness.

## Task 8 Validation Snapshot

Task 8 was locally accepted on 2026-07-14:

- `python -m pytest -q`: 1043 passed on Python 3.14.2;
- the 37 fan-out, Agent protocol, and real Brief-runtime integration tests passed independently on Python 3.11,
  3.12, 3.13, and 3.14;
- schema/model conditional parity, exact source-plan hash binding, one-item-per-requirement projection, confirmed
  omit, source blocker, duplicate Cover Letter cardinality, planned/unregistered executor, workflow-support scope,
  private-body absence, and read-only CLI tests passed;
- canonical/workspace skill mirror, generated-schema, compile, diff, repository, workspace, and packaged-resource
  checks passed;
- `uv build`, Twine metadata checks, and the 0.2.0 wheel packaged-resource check passed;
- a clean Python 3.12 wheel installation created a workspace containing the new schema, advertised
  `documents status`, returned the body-free missing-plan action, and imported the packaged execution contract and
  all 15 capability routes.

This acceptance establishes the required-document execution inventory and dispatch boundary only. Cover Letter is
still the sole available guarded document executor; second-document Draft, all-document completion, cross-document
Review, package readiness, remote CI, publication, rendering approval, and submission remain later work.

## Task 9 Validation Snapshot

Task 9 was locally accepted on 2026-07-14:

- `python -m pytest -q`: 1052 passed on Python 3.14.2;
- after the final omission of null document metadata from non-document Agent responses, the 15 affected
  CLI/Draft/Review compatibility tests passed;
- the 41 document-identity, current/legacy control-contract, Draft/Review runtime, CLI, and frozen Stage 1 fixture
  tests passed independently on Python 3.11.15, 3.12.12, 3.13.14, and 3.14.2;
- the 114 focused ownership/runtime tests and 115 repository, workspace, skill-distribution, release, and tracking
  tests passed;
- dual same-stage identity, scoped dependency/invalidation, explicit and automatic selection, malformed/mismatched
  target rejection, private-body absence, and exact non-document 1.0 serialization tests passed;
- legacy pending 1.0 Cover Letter tasks were associated with the sole current plan ID and could be reused, submitted,
  promoted, or cancelled without rewriting their immutable TaskSpec or preparation receipt;
- canonical/workspace skill mirror, schema, compile, diff, packaged-resource, and repository-contract checks passed;
- `uv build` and Twine metadata checks passed for the 0.2.0 sdist and wheel;
- a clean Python 3.12 wheel installation exposed the new schema and skill guidance, then passed the complete Decision
  Spine smoke with 8 successful stage runs and 14 immutable user-mutation receipts.

This acceptance establishes reusable per-document Draft/Review ownership, not a second document executor. Research
Statement schema, current-basis validation, guarded promotion, review behavior, package readiness, remote CI,
publication, rendering approval, and submission remain later work.

## First-Slice Exit Review

The compatible pipeline consumes the structured Draft only when current deterministic Match is usable for the same
parsed job and configured profile, Draft and Review pass their current deterministic validators, and Review has zero
blocker findings. It renders every Claim once, carries exact Draft/Review hashes, neutralizes Markdown structure, and
places Typst Claim text inside a text string. Any missing, blocked, stale, drifted, tampered, differently parsed,
mixed-profile, direct-library, or explicit `--llm-drafts` path uses the compatible legacy/provider behavior.

Open Review findings remain open and `review_state=proposed`; `check-package` binds the structured inputs and fails
APP-Q4 rather than infer readiness. This is a derived compatibility view, not a second promotion path. No remote CI,
publication, package readiness, rendering approval, or submission result is claimed by local acceptance.

## First-Slice Exit Criteria

- every Cover Letter prose block has one validated Claim ID;
- every strong factual claim resolves to current Evidence;
- unsupported and contradictory facts are explicit blockers;
- no Draft worker writes an authoritative/user-owned/Markdown/Typst/profile target directly;
- stale, invalid, or rejected candidates leave all authoritative bytes unchanged;
- a promoted Draft remains proposed until current Review succeeds;
- a missing or blocked required Cover Letter prevents later package readiness;
- the same CLI/task/result contracts work across supported shell-capable agent hosts.

## Explicit Non-Goals

- all-document Draft orchestration in the first slice;
- provider-owned trusted envelopes, direct promotion, or silent fallback between execution modes;
- final wording quality or automatic semantic truth certification;
- blocker/finding waivers (guarded dispositions are not waivers) or multi-user collaboration;
- multi-file transactions or direct Markdown/Typst promotion;
- portal work, upload, submission, account creation, or sensitive declarations.
