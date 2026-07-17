# Stage 5 Resilience And Legacy Convergence Implementation Plan

**Status:** In progress — Tasks 0–5 accepted; orchestrator and host convergence are next

**Date:** 2026-07-17

**Branch:** `feat/resilience-legacy-convergence`

**Baseline:** Stage 4 `0.6.0b1`, post-release evidence commit `33a44146ba3dce65adb930bacbcec0cf6ba0ce93`

## Goal

Move the remaining monolithic application workflow behind the resumable stage runtime while preserving user-owned
inputs, legacy CLI/file compatibility, local-first privacy, and explicit full-advert intake. Prove no-op resume,
precise invalidation, cooperative concurrency, crash recovery, migration/rollback, output repair, and one validated
promotion path across direct CLI, host agents, and orchestration.

## Fixed Boundaries

- ADR-024 governs source stages, coordination, bundles, projections, sequence execution, migration, and orchestrator
  convergence.
- `intake` and `decide` are implemented source stages, never agent/core-generated authoritative outputs.
- Existing user-mutation consent and compare-and-swap contracts remain authoritative for corrections, Decision,
  Brief, Review dispositions, and package dispositions.
- Package and Render promote one strict JSON bundle each. Legacy Markdown/Typst/PDF files are validated recoverable
  projections, not alternate promotion paths.
- Legacy compatibility generation cannot imply document/package review, render approval, portal readiness, upload,
  submission, or receipt.
- Direct URL/PDF/text advert intake and Stage 4 discovery behavior remain unchanged.
- All tests use synthetic/private-safe fixtures. Failure injection is programmatic only.
- Stable release and Stage 6 work are outside this plan.

## Compatibility Matrix

| Surface | Stage 4 behavior | Stage 5 requirement |
|---|---|---|
| `canisend run` | Direct monolithic compute/write | Compatibility wrapper over resumable sequence |
| `--dry-run` | Preview legacy outputs | Read-only sequence plan with the same output inventory |
| `--llm-parser` | Provider writes through legacy pipeline | Registered guarded Parse provider mode |
| `--llm-drafts` | Provider writes legacy prose directly | Registered guarded document Draft path; no direct package write |
| Legacy Markdown/content JSON | Written directly | Projected from accepted Package bundle |
| Editable Typst source | Preserved with `*.generated.typ` conflict | Same behavior through projection journal |
| `check-package --write-report` | Direct report write | Verify candidate validation/promotion |
| `render-typst` | Direct multi-PDF compile | Render bundle plus recoverable PDF projection |
| Old job without `workflow/` | Readable; runtime added on mutation | Still readable; explicit reversible migration available |
| Orchestrator declared output | Exit-zero direct write | Registered stage tasks submit/apply through runtime |

## Delivery Sequence

### Task 0: Audit And Contract Freeze

- [x] Audit registry/runtime coverage, legacy pipeline outputs, Verify/Render commands, recovery primitives, and
  orchestrator promotion behavior.
- [x] Accept ADR-024.
- [x] Freeze the Stage 5 task graph, compatibility matrix, privacy/ownership boundaries, and exit evidence.
- [x] Create the dedicated Stage 5 branch from the Stage 4 post-release evidence commit.

### Task 1: Per-Job Coordination And Failure Model

- [x] Add one portable crash-released, reentrant per-job coordination service.
- [x] Guard stage prepare/submit/apply/cancel/run mutations; require sequence, projection, migration, and repair to use
  the same service as they are added.
- [x] Preserve read-only status and body-free error semantics.
- [x] Add deterministic failure-injection hooks at claim, promotion, receipt, and state boundaries; require projection
  hooks with the projection service in Task 3.
- [x] Prove concurrent prepare/apply/cancel attempts have exactly one safe winner and no stale promotion.

### Task 2: Source Stages And Complete Registry

- [x] Add explicit registry execution kinds for source and task stages.
- [x] Implement dynamic Intake status over safe full-advert and job-metadata receipts.
- [x] Implement dynamic Decision status over the current confirmed user-owned decision basis.
- [x] Expose source stages through text and `canisend.agent/v1` status without prepare/apply authority.
- [x] Make descendant invalidation bind the exact source receipts.

### Task 3: Package, Verify, And Render Bundles

- [x] Add strict versioned Bundle/entry/projection-journal schemas and packaged resources.
- [x] Refactor legacy material computation into a pure Package bundle builder and validator.
- [x] Add guarded and explicit legacy-compatibility package modes without readiness ambiguity.
- [x] Add idempotent Package projection with Typst edit preservation and drift reporting.
- [x] Implement Verify over independently rederived package checks and promote one gate report.
- [x] Implement Render compilation into one validated bundle plus recoverable PDF projection.
- [x] Route `check-package --write-report` and `render-typst` through their registered stages.

### Task 4: Resumable Sequence And `run` Convergence

- [x] Add a deterministic registry-order sequence planner with current/no-op/blocked/execute/repair decisions.
- [x] Execute every eligible stage through prepare, submit, validate, apply, and projection services.
- [x] Stop safely on user-owned or provider-required stages and return actionable body-free status.
- [x] Replace the direct monolithic `canisend run` call with the sequence runner.
- [x] Preserve dry-run, output names, Typst candidates, CLI/AgentResponse shape, and git-staging behavior.
- [x] Prove a second unchanged run performs no current stage work and rewrites no current projection.

### Task 5: Migration, Rollback, And Drift Repair

- [x] Add read-only migration inspection for legacy workspace/job shapes.
- [x] Add explicit migration apply with body-free receipt and exact created/replaced metadata hashes.
- [x] Add conflict-safe rollback that restores/removes only unchanged migration-owned files.
- [x] Add explicit bundle-projection and state repair commands; never repair output drift silently.
- [x] Add pre-workflow, prior-schema, edited-Typst, missing-output, partial-projection, and rollback-conflict fixtures.

### Task 6: Orchestrator And Host Convergence

- [ ] Add an optional registered-stage task contract to orchestration plans.
- [ ] Prepare immutable TaskSpecs before dispatch and expose only declared reads/writes/consents.
- [ ] Submit worker candidate bytes through the guarded service and apply through the shared terminal claim.
- [ ] Retain generic non-stage orchestration behavior without presenting it as stage promotion.
- [ ] Prove direct CLI, Codex-style, Claude-style, configured-provider, and orchestrated execution produce equivalent
  validated receipts for the same candidate.

### Task 7: Skills, Documentation, Packaging, And Migration Guidance

- [ ] Update README, changelog, runtime/file/privacy contracts, canonical skills, compatibility mirror, and examples.
- [ ] Add Stage 5 migration, rollback, repair, concurrency, retry, and troubleshooting guidance.
- [ ] Package every new schema, example, skill, and guide into initialized workspaces and wheels.
- [ ] Preserve Stage 4 CSV/JSON/search/feed/adapter candidate semantics and URL/PDF/text full-advert compatibility.

### Task 8: Stage 5 Exit Acceptance

- [ ] Run focused concurrency, crash, retry, cancellation, projection, drift, migration, rollback, and orchestrator
  adversarial suites.
- [ ] Run the full supported Python 3.11–3.13 matrix plus the available development interpreter.
- [ ] Run Linux/macOS/Windows CLI and sequence smokes.
- [ ] Run source-tree, built-wheel, clean-install, schema/resource, Twine, mirror, and agent-host smoke gates.
- [ ] Record immutable evidence and mark Stage 5 complete only after every exit criterion passes.

## Exit Criteria

- stopping after any completed stage and resuming repeats no current work;
- changing one source invalidates only its true descendants;
- a crash, rejected candidate, cancelled task, or incomplete projection cannot corrupt or falsely current an
  authoritative artifact;
- concurrent cooperative attempts cannot both prepare/promote conflicting work;
- legacy workspaces, commands, output names, direct URL/PDF/text intake, and discovery catalogs remain readable and
  testable;
- migration and rollback preserve conflicts and never delete private/user-owned content;
- every declared output is schema/content/hash validated independently of process exit status;
- orchestrated and direct stage tasks use the same TaskSpec, submission, validation, terminal claim, and promotion
  path; and
- installed wheels contain every contract and resource needed to resume, migrate, repair, and run without a source
  checkout.

## Task 0 Audit Record

- The canonical graph declares 13 stages. Evidence, Parse, Confirm, Match, Brief, document-scoped Draft/Review, and
  Package Review are executable; Intake, Decide, Package, Verify, and Render are not implemented in the registry.
- `canisend run` still calls `run_job_pipeline`, which directly writes legacy Markdown/content/Typst artifacts and
  mutates `job.yaml` status. `check-package --write-report` and `render-typst` also write outside stage promotion.
- Single-file terminal claims, cancellation, stale detection, output drift, immutable run reconstruction, and
  interrupted-promotion finalization already exist and will be retained.
- The active-task state check is not sufficient to serialize two processes entering prepare simultaneously; Stage 5
  adds one cross-process coordination boundary rather than relying on mutable state as a lock.
- The current orchestrator writes declared stdout directly after a successful process exit; registered stage tasks
  must instead enter the existing guarded submission/apply service.
- User-owned Decision Spine files and editable Typst sources remain outside ordinary generated-output ownership.

## Task 1 Acceptance Record

- `workflow/job.lock` is a persistent, body-free coordination inode guarded by a crash-released OS lock. It rejects
  symlinked and multiply linked lock files, uses restrictive POSIX permissions, and is reentrant for nested runtime
  services in the same thread.
- Every existing public stage mutation enters the same coordination service. Read-only stage inspection remains
  lock-free and does not create workflow state.
- Programmatic failure points cover terminal claim, authoritative replacement, promotion receipt, manifest, and
  mutable-state refresh. Retrying or resuming converges to one terminal run without duplicate promotion.
- Spawned-process tests prove two concurrent prepares reuse one immutable TaskSpec and a concurrent apply/cancel pair
  produces exactly one terminal winner. Focused coordination/runtime/CLI evidence: `53 passed`.

## Task 2 Acceptance Record

- Registry definitions now distinguish `source` from `task` execution. Source stages cannot declare execution modes
  or generated authoritative outputs; Intake owns no write path to `job.yaml`/`job_advert.md`, and Decision owns no
  write path to `application_decision.yaml`.
- Intake reads bounded no-follow receipts, validates strict job metadata, distinguishes a reviewed full advert from a
  saved URL/feed stub, and derives a deterministic source identity without creating `workflow/`.
- Decision reuses the user-mutation basis validator and reports missing, undecided, basis-changed, apply, hold, and
  skip states without returning rationale text. Apply can advance; current hold/skip intentionally pause execution.
- Parse binds exact Intake files in its existing input fingerprint; Brief binds the exact Decision file and basis;
  dependency status adds no false Evidence invalidation when advert bytes change.
- JSON and text status are available through the existing AgentResponse surface. Prepare/run reject both source
  stages with `stage.source_read_only`. Focused registry/CLI/source-runtime evidence: `63 passed`; broader Decision
  Spine regression evidence: `179 passed`.

## Task 3 Acceptance Record

- Package promotes one strict `canisend.artifact-bundle/v1` JSON output. Its static projection scope cannot include
  source, user-owned, workflow, or arbitrary job paths; guarded input receipts are rechecked during validation.
  Explicit `legacy_compatibility` bundles remain non-ready and Verify rejects them.
- Projection uses the per-job coordination service, atomic writes, a versioned journal, deterministic failure points,
  idempotent no-op behavior, and `*.generated.typ` conflict preservation for edited user-facing Typst sources.
- Verify snapshots and independently rederives the package gate basis, accepts only a current guarded Package
  projection, and promotes one strict gate report. A valid FAIL report is a completed Verify result but blocks Render.
- Render compiles into an isolated temporary directory, validates PDF signatures, promotes one strict bundle, and
  projects recoverable `pdf/*.pdf` outputs only after promotion. Explicit compiler selection uses the same guarded
  prepare, submit, validate, apply, and projection path.
- `check-package --write-report` and `render-typst` retain a no-bundle legacy compatibility path. Once a Package
  bundle exists, neither command may fall back to a direct authoritative writer. Package/Verify/Render integration
  evidence: `4 passed`; bundle/schema/CLI/legacy regression evidence: `117 passed`.

## Task 4 Acceptance Record

- The read-only planner walks the registered topological order and reports every stage/document instance as current,
  execute, blocked, or repair. It exposes independent Evidence and Parse work together, prioritizes explicit repair
  over mutation, and never creates workflow state during preview.
- The runner resumes only eligible work through the shared guarded runtime, projects Package/Render bundles only
  after promotion, stops at Decision/Brief/host-agent/provider boundaries, and returns the same body-free
  AgentResponse envelope used by other agent operations.
- Configured-provider Parse now declares tier-3 full-advert consent and submits its candidate through the same local
  schema, source-grounding, input-currentness, terminal-claim, and promotion checks as deterministic/host execution.
- `canisend run` is a compatibility wrapper over the sequence rather than the direct monolithic writer. Its explicit
  legacy bundle preserves established filenames and git behavior without changing `job.yaml` to `packaged` or
  implying Decision, Review, Package, Verify, Render, or submission readiness.
- Projection journals replace the legacy Typst ownership manifest. Unchanged reruns rewrite neither stages nor
  projections; edited primaries receive `*.generated.typ` candidates; missing, invalid, or locally edited projection
  state fails closed and requires explicit repair.
- Sequence/runtime/CLI compatibility regression evidence: `185 passed`; full Package-to-Render resume/no-op/drift
  evidence: `1 passed`; retained Package/Verify/Render integration evidence: `4 passed`.

## Task 5 Acceptance Record

- `migration inspect` classifies pre-workflow, prior-schema, current-unmigrated, applied, rolled-back, and blocked
  states without creating `workflow/`, a lock, or any receipt. It inventories hashes/sizes for runtime control JSON
  while explicitly excluding candidate and prepared-input bodies.
- `migration apply` writes one immutable plan before mutation, exact backup bytes before replacement, a canonical
  state view only when needed, and one plan-bound body-free receipt. Re-entry after interruption accepts only the
  recorded before/after hash and converges without treating legacy outputs as successful stage evidence.
- Each rollback attempt has its own immutable receipt. Created metadata is removed and replaced metadata restored
  only while the current hash still equals the migration's after hash; changed files are preserved as conflicts.
  Advert, profile, Decision Spine, Draft/Review, Markdown, Typst, and PDF content are outside migration ownership.
- `repair projection` explicitly replays only a validated Package/Render bundle, replaces invalid journals, resumes
  partial/missing outputs, and preserves edited Typst primaries through `*.generated.typ`. A current replay rewrites
  no projection and reports truthful `unchanged` entries.
- `repair state` reconstructs only from recoverable immutable run/task evidence. It never rewrites authoritative
  stage outputs, so output drift remains visible after state repair. Sequence planning reports repair but never calls
  either repair service automatically.
- Strict schemas cover migration plan/receipt/rollback and repair receipts. Focused migration/repair, schema,
  runtime, store, CLI, AgentResponse, and projection evidence: `149 passed`; full Package-to-Render explicit-repair
  routing/no-silent-repair evidence: `1 passed`.
