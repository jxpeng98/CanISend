# Resumable Kernel And Parse Slice Implementation Plan

**Status:** Complete — local acceptance; remote candidate CI remains a release gate

**Date:** 2026-07-11

**Branch:** `feat/resumable-workflow-foundation`

**Roadmap:** `docs/superpowers/specs/2026-07-11-cli-first-workflow-optimization-roadmap.md`

## Goal

Implement Stage 1 of the CLI-first optimization roadmap: a versioned, resumable stage kernel and one complete Parse
vertical slice. The implementation must preserve the existing pipeline while proving candidate staging, freshness
validation, atomic promotion, immutable run evidence, and fresh-session continuation.

## Fixed Decisions

- Python and the file workspace remain authoritative.
- Only Parse is executable in this stage; all other registry entries are declared but unsupported.
- Stage runtime job paths must remain inside an initialized workspace. Legacy `run` keeps its existing path behavior.
- `parsed_job.json` is the only authoritative artifact promoted by the new runtime in this stage.
- Task specifications and finalized run manifests are immutable.
- `state.json` is a replaceable/rebuildable current view, not the sole history source.
- Parse fingerprinting uses a canonical input projection and excludes downstream/status/profile changes.
- Current-host preparation never calls another model provider.
- No MCP or agent-response schema 1.1 work is included.

## Task 0: Freeze Architecture And Baseline

- [x] Add ADR-007 for workflow state, immutable runs, and reconstruction.
- [x] Add ADR-008 for TaskSpec/TaskResult validation and single-file promotion.
- [x] Record the current focused baseline: 38 pipeline, workflow-state, and agent-CLI tests passed.
- [x] Confirm the branch starts with a clean worktree.

## Task 1: Versioned Persistent Models And Schemas

**Create:**

- `src/canisend/stage_models.py`
- `schemas/workflow-state.schema.json`
- `schemas/task-spec.schema.json`
- `schemas/task-result.schema.json`
- `schemas/run-manifest.schema.json`
- `tests/test_stage_models.py`

- [x] Write failing tests for strict fields, identifiers, paths, hashes, status consistency, and JSON Schema validity.
- [x] Define `ArtifactFingerprint`, `StageRecord`, `WorkflowStateV1`, `TaskSpecV1`, `TaskResultV1`,
  `ValidationReportV1`, and `RunManifestV1`.
- [x] Reject absolute paths, parent traversal, unknown fields, malformed hashes, and inconsistent completion fields.
- [x] Package every public schema and add wheel-resource assertions.
- [x] Verify model and schema tests pass.

## Task 2: Validated Stage Registry

**Create:**

- `src/canisend/stage_registry.py`
- `tests/test_stage_registry.py`

- [x] Write failing registry topology and unsupported-stage tests.
- [x] Declare Intake, Evidence, Parse, Confirm, Match, Decide, Brief, Draft, Review, Package, Verify, and Render.
- [x] Validate unique IDs, known dependencies, acyclicity, and output ownership.
- [x] Mark only Parse implemented with deterministic and host-agent modes.
- [x] Expose descendant calculation for precise invalidation.
- [x] Verify registry tests pass.

## Task 3: Atomic Store, State Reconstruction, And Run Evidence

**Create:**

- `src/canisend/stage_store.py`
- `tests/test_stage_store.py`

- [x] Write failing tests for atomic JSON replacement, immutable file creation, state loading, and corrupt-state
  recovery.
- [x] Use same-directory temporary files, flush/fsync, and `os.replace` for replaceable views and authoritative
  single-file promotion.
- [x] Create immutable files with exclusive creation and reject content changes on retry.
- [x] Keep all runtime paths within `jobs/<job-id>/workflow/` and reject symlink escapes.
- [x] Rebuild state from finalized manifests when `state.json` is missing or invalid.
- [x] Verify interrupted or rejected operations leave authoritative files unchanged.

## Task 4: Parse Projection, Fingerprint, And Staleness

**Create:**

- `src/canisend/stages/__init__.py`
- `src/canisend/stages/parse_stage.py`
- `tests/test_parse_stage.py`

- [x] Write decision-table tests for relevant and irrelevant input changes.
- [x] Reuse `parse_job_advert` and `validate_parsed_job`.
- [x] Hash the advert and canonical metadata projection rather than raw `job.yaml`.
- [x] Exclude status, timestamps, notes, writing preferences, profile evidence, and downstream artifacts.
- [x] Detect output drift independently from input staleness.
- [x] Propagate Parse staleness only through registry descendants.

## Task 5: Prepare, Apply, And Deterministic Run Services

**Create:**

- `src/canisend/stage_runtime.py`
- `tests/test_stage_runtime.py`

- [x] Write failing fresh-session, cache, stale-result, bad-identity, unsafe-path, bad-hash, invalid-schema, and
  output-drift tests.
- [x] `prepare(parse)` writes one immutable TaskSpec and creates a candidate directory.
- [x] deterministic execution writes a candidate and TaskResult through the same apply boundary.
- [x] host-agent preparation performs no provider construction or invocation.
- [x] apply recomputes current inputs and validates identity, scope, candidate hash, JSON shape, parsed-job semantics,
  and expected prior output hash.
- [x] successful apply atomically promotes `parsed_job.json`, finalizes a run manifest, and updates state.
- [x] rejected apply records a safe finalized failure without modifying the authoritative output.
- [x] unchanged deterministic reruns return a cache hit without changing authoritative mtime/hash.

## Task 6: CLI-First Surface

**Modify:**

- `src/canisend/cli.py`
- `src/canisend/agent_protocol.py` only for scalar capability discovery if required
- `tests/test_stage_cli.py`
- `tests/test_agent_cli.py` only if capabilities change

- [x] Add `stage status`, `stage prepare`, `stage apply`, and deterministic `stage run`.
- [x] Keep text as the default and emit exactly one JSON document in JSON mode.
- [x] Return stable operational errors without private paths, source queries, or candidate bodies.
- [x] Let a fresh CLI runner continue the same prepared task.
- [x] Do not add structured fields silently to `canisend.agent/v1` schema `1.0.0`.
- [x] Verify CLI tests pass.

## Task 7: Legacy Pipeline Compatibility

**Modify only after the Parse slice passes independently:**

- `src/canisend/pipeline.py`
- `tests/test_pipeline.py`

- [x] Extract or reuse one deterministic Parse service without changing output semantics.
- [x] Keep `canisend run`, dry-run, LLM flags, text output, job status, Typst candidate protection, and git behavior.
- [x] Do not route Match, Draft, Review, Package, Verify, or Render through the new runtime yet.
- [x] Verify the complete existing pipeline suite passes unchanged.

## Task 8: Documentation, Packaging, And Exit Review

- [x] Document the Stage 1 CLI loop with fake paths and no private bodies.
- [x] Update `CHANGELOG.md` under Unreleased.
- [x] Run focused stage, protocol, pipeline, workspace, and package suites: 167 passed.
- [x] Run the complete suite on Python 3.11, 3.12, and 3.13: 477 passed on each interpreter.
- [x] Build sdist/wheel, run Twine and resource checks, and perform a clean-wheel Parse smoke test.
- [x] Review for private paths, secrets, scope leakage, and untracked application artifacts.
- [x] Mark Stage 1 locally complete after every exit criterion passes; retain remote candidate CI as the release gate.

## Exit Review Record — 2026-07-11

- Focused Stage 1, protocol, pipeline, workspace, and packaging suite: 167 passed before final matrix validation.
- Complete suite: 477 passed independently on CPython 3.11.15, 3.12.12, and 3.13.14.
- Persistence and recovery evidence covers immutable records, same-directory atomic replacement, stale-result
  rejection, compare-and-swap concurrency, output drift, corrupt-state reconstruction, and promotion-receipt recovery.
- Distribution evidence: sdist and wheel built; Twine metadata and packaged-resource checks passed.
- Clean-wheel evidence: CPython 3.12.12 installed the wheel, created the packaged fake workspace, ran Stage status,
  promoted deterministic Parse, observed a true second-run cache hit, installed all four runtime schemas, and resolved
  all 14 workspace skills.
- Boundary evidence: 128 repository, skill, protocol, stage-model, storage, registry, and runtime checks passed; no
  local absolute path, secret value, MCP/platform implementation, non-Parse executable stage, or real application
  artifact was found in the change set.
- The branch is not pushed. Remote CI remains required before preparing or publishing the candidate milestone.

## Required Acceptance Matrix

| Requirement | Automated evidence |
|---|---|
| Fresh process resumes prepared work | CLI and runtime fresh-runner tests |
| Parse cache is a true no-op | Authoritative hash and mtime assertions |
| Relevant inputs invalidate Parse | Metadata/advert decision table |
| Profile/downstream changes do not invalidate Parse | Negative invalidation tests |
| Stale result cannot promote | Freshness mismatch test |
| Invalid or unsafe candidate cannot promote | Schema/hash/path/symlink tests |
| Manual output drift is preserved | Expected-prior-hash test |
| Current-host mode calls no provider | Provider construction spy |
| Run evidence is immutable | Exclusive-write/retry tests |
| State is reconstructable | Missing/corrupt state recovery tests |
| Legacy behavior is compatible | Existing pipeline and CLI suites |
| Public contracts are packaged | Wheel resource and clean-install tests |
