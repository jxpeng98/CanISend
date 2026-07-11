# ADR-007: Keep Workflow State Rebuildable And Run Evidence Immutable

**Status:** Accepted

**Date:** 2026-07-11

## Context

The current application pipeline writes its outputs in one process and infers readiness from the files that remain.
It cannot distinguish a current completed stage from stale output, an interrupted attempt, or a manually changed
authoritative artifact. Chat history and process memory cannot provide durable recovery across host sessions.

Stage execution needs both a convenient current view and durable evidence of what was attempted. Treating one mutable
state file as the only history would make recovery depend on the last write succeeding and would make corruption or
manual edits difficult to diagnose.

## Decision

Each job may contain:

```text
workflow/
  state.json
  runs/<run-id>/
    task-spec.json
    preparation.json
    submission.json
    candidates/<artifact-name>.json
    tasks/<task-id>/result.json
    validation/report.json
    terminal-claim.json
    promotion.json
    manifest.json
```

`state.json` is a versioned, atomically replaceable current view. It records stage status, the latest run, the last
successful input fingerprint and output receipts, and safe stale/conflict reasons. It is not authoritative history
and must be reconstructable from finalized run manifests and current artifact hashes.

A TaskSpec and its independent preparation receipt are written once before work begins. Candidate JSON enters the run
directory only through the guarded submission service, which writes the candidate, TaskResult, and submission receipt
without following symlink or hard-link aliases. A finalized run manifest is written once when an attempt succeeds,
fails, or is explicitly cancelled. Retrying an identical immutable write is idempotent; attempting to replace it
with different content is an error. Preparation, submission, validation, promotion, and terminal receipts never
contain private bodies, absolute paths, URL query values, secrets, or raw provider output.

Persistent runtime paths are normalized job-relative POSIX paths. Stage-runtime mutation is allowed only for a job
inside an initialized workspace. Existing legacy CLI behavior for external absolute job paths remains unchanged until
its own migration is designed.

Current-state decisions use content hashes and declared stage contract versions, not modification times or the whole
package version. A stage is current only when its direct input fingerprint matches and every authoritative output
matches the last successful output receipt.

## Consequences

- A new process can resume from workspace evidence rather than chat or process memory.
- Missing or invalid `state.json` can be repaired without discarding immutable run history.
- A successful promotion followed by a state-write failure can be reconciled from output and run receipts.
- Manual output drift is distinguishable from input staleness and is never silently overwritten.
- A missing or corrupted preparation receipt makes its pending task ineligible; a stale mutable running view is
  discarded so a fresh task can be prepared instead of deadlocking the workflow.
- Explicit cancellation releases the single-active-task slot while preserving run evidence and any candidate.
- State writes require atomic replace semantics; immutable records require exclusive creation and content comparison.
- Old jobs without a `workflow/` directory remain readable and receive no files during read-only inspection.

## Rejected Alternatives

- Store progress only in `job.yaml`: rejected because run attempts and hashes would cause noisy rewrites and mix user
  metadata with runtime history.
- Store progress only in mutable `state.json`: rejected because a single interrupted or corrupted write would erase
  the evidence needed for recovery.
- Use chat history or provider sessions: rejected because they are not portable or deterministic.
- Use mtimes as freshness: rejected because clocks, copies, and harmless touches make them unreliable.

## Revisit When

Revisit before multi-file authoritative promotion or remote workspaces. Parse and Confirm each promote one JSON
artifact atomically; later multi-output stages require a separate journal or transaction design.
