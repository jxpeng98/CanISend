# Decision Spine Foundation Implementation Plan

**Status:** Complete — Stage 2 locally accepted on 2026-07-12; no remote CI or publication claimed

**Date:** 2026-07-11

**Branch:** `feat/decision-spine-foundation`

**Roadmap:** `docs/superpowers/specs/2026-07-11-cli-first-workflow-optimization-roadmap.md`

## Goal

Implement Stage 2 of the CLI-first roadmap as a sequence of reviewable vertical slices. The first slice establishes
stable criteria and a resumable Confirm stage; the second establishes stable private Evidence catalogs and durable
proposed matching; the third adds user-owned corrections and application decisions; the final slices add Brief,
advert-driven document planning, and guarded structured views without entering Draft.

## Fixed Decisions

- Python, the file workspace, the existing CLI, and `canisend.agent/v1` remain the platform contract.
- Parsed Job v1 remains unchanged; Stage 2 semantic fields live in separate versioned artifacts.
- Stable IDs are derived from normalized source receipts plus normalized parser interpretation, never list positions,
  line numbers, sibling counts, or user-corrected prose.
- `confirmed_corrections.yaml`, `application_decision.yaml`, and `application_brief.yaml` are user-owned.
- Core-owned projections may become stale; user-owned values are preserved and marked for review.
- AgentResponse 1.0 carries artifact references and scalar extensions only.
- `canisend run` remains compatible while current deterministic Match may safely supply selected Markdown/Typst views.
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
confirmation, or a package-readiness signal. Task 5 records the separate user-owned Decision; Tasks 6-7 subsequently
delivered Application Brief, required-document planning, guarded view migration, and the full Stage 2 exit review.

## Task 5: User-Owned Corrections And Decision

**Status:** Locally accepted on 2026-07-11. This accepts the Task 5 slice only. Tasks 6-7 and the complete Stage 2 exit
review remain open; no remote CI run or package publication is claimed here.

- [x] Add explicit-consent, create-if-absent templates and bounded strict safe-YAML validation.
- [x] Add scoped compare-and-swap updates with revision/hash conflicts, single-winner claims, immutable private
  candidates, and immutable privacy-safe receipts.
- [x] Keep an absent or `undecided` Decision distinct from confirmed apply, hold, or skip, and keep an unknown empty
  extraction distinct from an explicit `confirmed_empty` correction.
- [x] Preserve accepted values byte for byte when their basis changes and return explicit review/reconfirmation
  actions derived from current Criteria and Match receipts.
- [x] Never copy rationale or correction bodies into mutation claims, receipts, workflow control records, errors,
  ordinary output, or AgentResponse.
- [x] Expose status, initialization, scoped update, and recovery as host-neutral Agent operations without adding a
  platform API or allowing an agent to replace a whole user YAML file.
- [x] Require current Parse and Confirm before every semantic correction patch; rerun Confirm after each accepted
  patch before applying another correction.

### Task 5 Slice Acceptance Matrix

Local acceptance is recorded by contract and by the settled branch-wide regression suite. The final Task 5
acceptance run passed all 754 tests on Python 3.11, 3.12, 3.13, and 3.14.

| Requirement | Accepted automated evidence |
|---|---|
| User ownership is preserved | Status is read-only; initialization is create-if-absent; Parse, Confirm, Match, and legacy runs preserve existing YAML bytes |
| Strict bounded inputs | Alias, merge, tag, duplicate-key, depth, event-count, byte-limit, non-regular, symlink, hard-link, and path-race rejection tests |
| Scoped writes only | Discriminated correction and decision patch tests reject unknown fields and whole-file replacement inputs |
| Explicit consent | Init, update, and recovery fail closed without `--confirm-user-owned-write` |
| Honest correction state | Unknown empty extraction, active `confirmed_empty`, stale empty basis, later non-empty extraction, orphan, withdraw, and one-patch-per-current-Confirm tests |
| Honest Decision state | Missing, undecided, current apply/hold/skip, reset, preserved stale basis, and explicit reconfirmation tests |
| Cooperative CAS | Revision/hash conflict, single-winner claim, idempotent retry, candidate integrity, final reread, and parent/path swap tests |
| Recovery is durable | Accepted-write/receipt-failure, after-link/before-unlink crash markers, fresh-session audit, ordinary hard-link rejection, and idempotent `user-mutation recover` tests |
| Privacy tiers hold | User YAML and private candidate are Tier 2; immutable receipt is Tier 1; private sentinels are absent from control and Agent output |
| Fresh sessions agree | CLI and Agent context tests reconstruct the same correction and Decision actions from job-local durable state |
| Installed artifacts are complete | Packaged receipt schema plus CI/release clean-install smoke for status and explicit create-if-absent operations |

### Task 5 Slice Exit Review

The local acceptance run also rebuilt the sdist and wheel, passed packaged-resource and Twine metadata checks, and
installed the wheel into a clean Python 3.14 environment. The installed CLI then completed the sequential
`run-example -> Evidence -> Parse -> Confirm -> corrections status/init -> Match -> decision status/init` smoke and
produced two private user-owned YAML records, two body-free mutation receipts, and successful manifests for all four
executable stages. Remote CI and publication remain separate release gates.

The update service coordinates cooperative CanISend writers for one stable job directory. It does not claim to
linearize a normal editor saving during the final replace window, a same-user process maliciously renaming paths, a
relocated job directory, remote filesystems, or multi-user collaboration. Users and agents should run `status`
immediately before a mutation and avoid concurrent manual saves. Direct manual YAML edits remain supported: CanISend
status and stage reruns validate and report their current raw-byte hash without normalizing or rewriting them. An
explicitly consented scoped update creates the canonical next revision and may not preserve comments.

Private correction text and rationale stay in the Tier 2 user YAML/private candidate (and corrected Criteria
projection where applicable). Claims, receipts, errors, ordinary CLI output, and AgentResponse remain body-free; a
receipt is a Tier 1 integrity record, not a copy of the accepted private value. A Decision remains present when its
Criteria/Match basis changes, while `status` derives `review_required`; stale state is never written back into the
user-owned file. Reset, clear, withdraw, and supersession do not erase private-mode candidates (0600 on POSIX) or correction
history. Job folders remain private/git-ignored, and removing events or a whole job is a separate retention decision;
automatic secure erasure and deletion from backups/snapshots are not claimed.

## Task 6: Application Brief And Required-Document Plan

**Status:** Locally accepted on 2026-07-12; Task 7 subsequently completed the full Stage 2 exit review.

### Task 6.0: Freeze Ownership, Privacy, And Execution Boundaries

- [x] Accept ADR-012: keep `application_brief.yaml` user-owned Tier 2 and
  `required_document_plan.json` core-owned Tier 2.
- [x] Lock body-free Tier 1 mutation and workflow control records with private-sentinel tests.
- [x] Prove Brief mutation and document planning use no configured provider, network, MCP transport, platform API, or
  host-specific session state.
- [x] Keep Stage 2 out of application-facing Draft and readiness claims.

### Task 6.1: Complete Strict Brief And Requirement-Set Contracts

- [x] Use the strict user-owned revision and RFC 3339 control timestamp types for Application Brief.
- [x] Model language, writing style, motivation, emphasis, exclusions, requirement-set confirmation, and document
  choices with explicit field-level confirmation state.
- [x] Represent the requirement-set basis as `unconfirmed`, `confirmed`, or `confirmed_empty`; never derive
  `confirmed_empty` from an empty Parsed Job list, missing field, ambiguity, or extraction failure.
- [x] Define stable semantic document IDs, source receipts, deterministic normalization/deduplication, explicit
  unknown reasons, and orphan-choice representation.
- [x] Reject unknown fields, duplicate IDs, inconsistent confirmation/action combinations, unsafe paths, malformed
  hashes, and invalid empty-state combinations.
- [x] Regenerate the Brief, required-document-plan, and mutation-receipt schemas and prove static/runtime parity.

### Task 6.2: Extend Guarded User-Owned Mutation To Brief

- [x] Replace every two-artifact `corrections else decision` branch with an explicit, fail-closed artifact mapping
  before adding Brief to claims, receipts, candidates, recovery, Agent references, and consents.
- [x] Add create-if-absent Brief initialization. Bootstrap concrete legacy `job.yaml` language/style values exactly
  once; keep unknown/placeholders unconfirmed and never synchronize an existing Brief from metadata.
- [x] Add bounded discriminated scoped patches for each supported Brief decision; reject whole-file replacement.
- [x] Require explicit `--confirm-user-owned-write`, the latest raw-byte SHA-256, and the latest revision for each
  initialization, update, and recovery action.
- [x] Preserve direct manual YAML bytes during status and stage reruns; preserve a stale Brief rather than clearing or
  rewriting accepted values.
- [x] Cover single-winner claims, final rereads, atomic replacement, idempotent retry, private immutable candidates,
  body-free receipts, interrupted publication, and fresh-session recovery.
- [x] Require a current confirmed `decision=apply` for Brief creation or semantic update; keep read-only status
  available for undecided, hold, skip, stale, unavailable, or missing Decision states.

### Task 6.3: Build The Deterministic Required-Document Projection

- [x] Normalize advert-derived document requirements and source receipts without using a fixed application bundle.
- [x] Store the explicit human confirmation of the requirement-set basis in the user-owned Brief and project it into
  the core-owned plan.
- [x] Require the current confirmed apply Decision, current declared upstream artifacts, and the exact raw Brief hash
  before preparing or promoting a plan.
- [x] Fingerprint the validated Decision raw hash and basis, requirement-set/source basis, raw Brief hash, and
  relevant contract versions without requiring a mutation receipt for a valid manual edit.
- [x] Resolve exactly one task per current normalized requirement and reconcile Brief choices by stable document ID.
- [x] Preserve confirmed optional omissions, but emit executable blockers for unconfirmed requirement sets,
  unresolved choices, `required + omit`, required documents without a preparation action, orphaned choices, and stale
  or unavailable bases.
- [x] Allow an empty plan only from a current explicit `confirmed_empty` basis.
- [x] Run plan generation through deterministic-only candidate validation, guarded submit, atomic promotion, cache,
  drift detection, cancellation, recovery, and fresh-session reconstruction.

### Task 6.4: Expose A Body-Free Host-Neutral Workflow

- [x] Add Brief status, create-if-absent initialization, scoped update, and recovery operations to the CLI and Agent
  capabilities without changing the AgentResponse v1 body contract.
- [x] Expose `application_brief` and `required_document_plan` only as Tier 2 artifact references with safe relative
  paths and hashes; expose counts, opaque IDs, states, and blocker codes only as scalar control metadata.
- [x] Require separate Tier 2 agent-reading consent before a host reads either body; do not require body access for
  status, context, or deterministic execution.
- [x] Preserve `phase=unknown` plus scalar stage extensions where required by the frozen AgentResponse 1.0 phase
  vocabulary.
- [x] Derive the same readiness, blockers, and next actions in fresh Codex, Claude Code, and generic shell sessions.

### Task 6.5: Prove Privacy, Compatibility, And Distribution

- [x] Add model/schema, initialization/bootstrap, field patch, CAS, conflict, recovery, stale-basis, orphan, and
  document-resolution tests.
- [x] Prove unique motivation, exclusion, style, and source-text sentinels remain only in their allowed Tier 2 data
  artifacts and never enter AgentResponse, claims, receipts, manifests, errors, or ordinary output.
- [x] Keep legacy `run`, dry-run, LLM flags, Markdown, Typst protection, git behavior, Parsed Job v1, TaskSpec v1, and
  AgentResponse v1 compatible.
- [x] Update canonical Skills, workspace mirrors, platform bridges, handoff examples, file contracts, privacy and
  quality-gate guidance only after the implementation contract passes focused tests.
- [x] Extend cross-OS CI, built-wheel, release, TestPyPI, and local-release fake-data smoke through Brief and
  required-document planning.
- [x] Run the full Python 3.11-3.14 suite, schema/resource checks, sdist/wheel build, Twine validation, and a clean-wheel
  fresh-session smoke before recording local Task 6 acceptance.

### Task 6 Exit Review

Locally accepted on 2026-07-12. The acceptance run includes:

- 919 tests passing on each Python 3.11, 3.12, 3.13, and 3.14 interpreter;
- strict runtime/standalone-schema parity, guarded Brief mutation, source-bound basis, privacy sentinel, recovery,
  drift, cache, cancellation, and fresh-session coverage;
- adversarial review of missing, ambiguous, negated, conditional, alternative, qualified, truncated, multiline, and
  unreconciled document source context, with no remaining blocker/high finding;
- regenerated schemas, packaged-resource checks, sdist/wheel build, Twine validation, and clean installed-wheel
  Decision Spine smoke through deterministic Brief.

This accepts Task 6 only. Task 7 view migration, compatibility convergence, documentation/exit review, and Stage 2 as
a whole remain open. No remote CI or published-package result is claimed.

## Task 7: Views, Compatibility, Documentation, And Exit Review

- [x] Keep legacy `run`, dry-run, LLM flags, Markdown, Typst protection, and git behavior compatible.
- [x] Begin rendering fit/checklist Markdown from structured matches only after parity tests pass.
- [x] Document the fresh-session Stage 2 CLI loop and manual YAML ownership boundaries.
- [x] Update the changelog and packaged skill references.
- [x] Run focused, full Python 3.11-3.13, distribution, resource, clean-wheel, privacy, and recovery checks.
- [x] Complete the Stage 2 exit review only when every roadmap exit criterion has automated evidence.

### Task 7 And Stage 2 Exit Review

Locally accepted on 2026-07-12. Automated evidence includes:

- exact currentness/hash/graph/parsed-job/profile-provenance gates with legacy fallback for stale, drifted/tampered,
  invalid, differently parsed, and profile-override runs;
- provider-path preservation for `--llm-drafts`, direct-library compatibility, dry-run/git behavior, protected Typst
  candidates, state reconstruction, and byte preservation across every Decision Spine artifact;
- stable-ID structured HR joins, fail-closed unresolved/unknown states, explicit confirmed-empty semantics,
  adversarial Markdown/citation escaping, correction propagation, and evidence-body exclusion;
- 942 passing tests on Python 3.11, 3.12, 3.13, and the additional Python 3.14 development interpreter;
- exact canonical skill-mirror checks, schema/resource validation, sdist/wheel build, Twine validation, package
  inspection, and a clean installed-wheel smoke through Brief plus guarded structured Markdown/Typst views.

Every Stage 2 roadmap exit criterion has automated evidence. No blocker/high/medium finding remains after red-team
review. This local acceptance is not a remote CI or published-package result and does not claim Draft, package, or
submission readiness.

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

This checkpoint accepts only Tasks 0-3 and the first-slice compatibility boundary. Task 4 Evidence/Match and Task 5
user-owned corrections/Decision were accepted separately above. Brief, required-document planning, view migration,
and the final Stage 2 exit review remain open in Tasks 6-7; Stage 2 as a whole is not complete.
