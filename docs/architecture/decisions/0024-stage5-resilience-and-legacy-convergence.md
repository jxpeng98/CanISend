# ADR-024: Converge Legacy Execution Through Source Stages, Guarded Bundles, And Recoverable Projections

**Status:** Accepted for Stage 5

**Date:** 2026-07-17

## Context

Stages 1–4 established immutable run evidence, single-output candidate validation and promotion, user-owned
Decision Spine inputs, document-scoped Draft/Review, aggregate package Review, and a source-neutral discovery
catalog. The original `canisend run` command still calls one monolithic pipeline that computes and writes many
Markdown, JSON, Typst, metadata, and candidate files directly. The registry also declares `intake`, `decide`,
`package`, `verify`, and `render` without implementing them.

The existing runtime already detects stale inputs, authoritative-output drift, late cancellation, and an interrupted
single-file promotion. It does not yet serialize simultaneous local writers, provide one resumable stage-sequence
runner, migrate legacy jobs explicitly, repair derived multi-file projections, or route the existing orchestrator
through the guarded submission/apply service.

Stage 5 must close those gaps without letting an agent write a supplied advert, an apply/hold/skip decision, a
user-owned disposition, or an editable Typst source as if it were core-owned output. It must preserve the current
CLI and readable legacy artifacts while making durable stage evidence—not process exit or chat state—the execution
authority.

## Decision

### Source stages remain user-owned

`intake` and `decide` become implemented source stages. They are inspected through the registry and participate in
fingerprints and descendant invalidation, but they have no prepare, submit, apply, or provider execution mode.

- `intake` is current only when the selected job metadata and full advert are safe, readable, bounded, and valid.
- `decide` is current only when the user-owned Decision is valid, explicitly confirmed, and bound to the current
  Criteria and Match receipts. Hold and skip remain current decisions but block Brief/package progression.

Core and agents may continue to use the existing explicitly consented user-mutation service. A source stage never
turns those files into generated outputs.

### Mutations use one crash-released per-job coordination boundary

Every stage prepare, submit, apply, cancel, sequence, projection, repair, and migration mutation acquires the same
job-local operating-system file lock. The lock is process-crash released, reentrant for nested core calls, bounded,
and body-free. Read-only status remains non-mutating and does not acquire an exclusive lock.

Immutable task, terminal-claim, promotion, and mutation receipts remain the concurrency authority after a process
releases the lock. The lock prevents cooperative local writers from entering check/prepare or check/promote windows
simultaneously; it is not a hostile same-user security boundary and does not make remote filesystems safe.

### Package and Render promote one structured bundle

`package` and `render` each promote one strict JSON bundle through the existing TaskSpec/TaskResult/validation/apply
service. A bundle contains normalized safe relative paths, media types, content hashes, and encoded bytes. It cannot
name runtime control paths, user-owned Decision Spine files, source inputs, or paths outside the selected job.

- The Package bundle owns generated compatibility Markdown, generated content JSON, and generated Typst candidates.
  It records whether it was built from the fully guarded document path or the explicit legacy-compatibility path.
- The Render bundle owns compiled PDF bytes and exact source receipts.
- `verify` owns the single `application_gate_report.json` result and independently rederives it from current package,
  review, disposition, and projection receipts.

Bundles are authoritative; compatibility files and PDFs are derived projections. Projection uses a versioned,
body-free journal and manifest, validates every candidate path/hash before writing, preserves edited Typst primaries,
and is idempotently replayable after interruption. A partial projection never makes the bundle or Verify current.
Output drift is reported and repaired only by an explicit repair operation or a new accepted bundle.

### One resumable sequence owns normal execution

A core stage-sequence runner inspects stages in registry order, skips current work, executes eligible deterministic
stages through the same guarded run service, projects accepted bundles, stops on user/provider blockers, and returns
a body-free receipt describing executed, reused, blocked, and repaired stages.

`canisend run` becomes a compatibility wrapper over that sequence. Its command name, workspace/job resolution,
dry-run behavior, generated filenames, Typst-edit preservation, and optional git staging remain compatible. When a
legacy job lacks a current Decision/Brief/Review path, Package may use the explicit compatibility mode so existing
generated files remain available, but Verify must fail closed and no package-readiness claim is created.

Provider-backed options must use registered stage execution modes and guarded candidate validation. They may not
fall back to direct monolithic writes.

### Recovery, retry, and failure evidence are explicit

Programmatic failure injection points exist after terminal claim, authoritative replacement, promotion receipt,
projection entry, projection manifest, and state refresh. They are test hooks, not environment-variable behavior.
Recovery must distinguish:

- safe replay because current bytes match the accepted candidate;
- incomplete projection that can continue from the bundle and journal;
- stale input requiring a new run;
- output drift or a conflicting claim requiring explicit repair/review; and
- cancelled or failed attempts that may never promote later.

Retry creates or reuses work only for the exact current fingerprint. A completed current stage is a cache hit, not a
new run.

### Legacy migration is explicit and reversible

Stage 5 adds inspect, apply, and rollback operations for old workspaces/jobs. Migration records a versioned,
body-free inventory and the exact hashes of runtime metadata it creates or replaces. It does not treat legacy files
as successful stage evidence without validation. Rollback removes or restores only files still matching the
migration receipt; conflicts are preserved and reported.

Old jobs remain readable before migration. Current workspace update behavior remains additive, and migrations never
delete profile, advert, Decision Spine, Draft/Review, generated material, or rendered content.

### Orchestrator stage tasks use the same promotion service

Generic orchestration tasks may retain their declared output behavior. A task that declares a registered CanISend
stage receives a core-prepared TaskSpec, supplies its candidate through the guarded submit service, and applies only
through the same validator and terminal claim used by direct CLI and host-agent execution. Process exit zero is not
promotion evidence.

## Consequences

- All canonical stages are inspectable through one registry while source ownership remains honest.
- A normal `run` can resume without repeating current work and without bypassing candidate validation.
- A crash may leave immutable evidence or an incomplete derived projection, but cannot silently establish a corrupt
  authoritative stage result.
- Simultaneous cooperative CLI/agent writers cannot prepare or promote conflicting work inside the same job.
- Legacy output filenames remain available and repairable without becoming the source of stage truth.
- Fully guarded readiness and legacy-compatible generation remain distinguishable.
- Stage 6 can measure recovery, cache precision, migrations, and clean installation against one execution path.

## Rejected Alternatives

- Make Intake or Decision ordinary generated stages: rejected because adverts and apply/hold/skip belong to the
  user/source boundary.
- Keep the monolithic writer behind the `run` command: rejected because a wrapper name does not create resumability,
  validation, or crash recovery.
- Treat a directory rename as a portable multi-file transaction: rejected because editable files, Windows behavior,
  and per-file legacy paths still require explicit conflict and replay semantics.
- Store only a projection manifest without a bundle: rejected because a partial write would lose the accepted bytes
  needed for deterministic recovery.
- Let the orchestrator promote output after exit zero: rejected because it bypasses TaskSpec binding, validation,
  freshness, cancellation, and compare-and-swap checks.
- Automatically migrate every workspace on read: rejected because inspection must stay non-mutating and rollback
  evidence must exist before persistent changes.

## Revisit When

Revisit before remote/network filesystems, hostile same-user execution, collaborative multi-user editing, portal
automation, upload/submission state, or secure deletion. Those require distributed coordination and authority beyond
this local workflow contract.
