# Evidence-Backed Draft Foundation Implementation Plan

**Status:** Locally accepted — Tasks 0–5 complete for the first Cover Letter Draft/Review vertical slice

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
- The first executable Draft mode is host-agent. Provider-backed Draft must later reuse the same validator.
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

## Validation Snapshot

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

This snapshot accepts the first Cover Letter Draft/Review/projection slice only. Remote CI, provider-backed Draft,
all-document orchestration, finding disposition, broader cross-document review, complete Stage 3, and package
readiness remain later work.

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
- provider-backed Draft before host-agent validation is accepted;
- final wording quality or automatic semantic truth certification;
- user-owned finding waivers or multi-user collaboration;
- multi-file transactions or direct Markdown/Typst promotion;
- portal work, upload, submission, account creation, or sensitive declarations.
