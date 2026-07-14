# ADR-017: Own Draft And Review Runs By Document-Scoped Stage Instance

**Status:** Accepted for Task 9 of Stage 3

**Date:** 2026-07-14

## Context

ADR-016 made every required document visible, but the resumable runtime still keyed state, cache lookup, retry,
failure recovery, and descendant invalidation only by the logical stage name. A second Draft or Review executor would
therefore compete with Cover Letter for the same `draft` or `review` record even if its run directory and output path
were different. The latest run could hide the other document, a retry could reuse the wrong pending task, and state
reconstruction could collapse two valid manifests into one record.

The Stage 1 wire contracts are already used across CLI, Codex, Claude Code, and other shell hosts. Non-document Parse,
Evidence, Confirm, Match, and Brief tasks must retain their existing 1.0 on-disk shape. Document identity also must not
be inferred from private labels, prose, provider output, or list position.

## Decision

CanISend defines the identity of a document-scoped stage instance as the composite key
`(stage, document_id)`. `document_id` is the stable identifier from the current core-validated Required Document Plan.
It is permitted only on Draft and Review in this slice. Non-document stages retain `(stage, null)` and continue to
emit the frozen 1.0 control-record shape.

### Versioned control records

Document-scoped TaskSpec, TaskResult, CandidateSubmission, ValidationReport, RunManifest, promotion, terminal-claim,
and WorkflowState records use control-contract version 1.1 and carry the same `document_id`. Readers accept both 1.0
and 1.1. Existing 1.0 Draft/Review records may omit the field; reconstruction associates a legacy Cover Letter run
with the single current Cover Letter ID when that association is unambiguous. No existing immutable record is
rewritten during migration.

Non-document runs keep version 1.0 and do not gain a serialized `document_id: null` field. This preserves the exact
cross-host TaskSpec and preparation-receipt shape while allowing a job's state view to upgrade to 1.1 as soon as its
first document-scoped run is prepared.

### Runtime ownership

- workflow records are unique and canonically ordered by `(stage, document_id)`;
- pending-task reuse, attempt counts, terminal results, state reconstruction, and cache lookup use the composite key;
- Review resolves the Draft instance with the same `document_id`;
- invalidating a non-document upstream stage stales every affected document instance, while invalidating one Draft
  stales only Review for that document;
- one active run per job remains the concurrency boundary for this slice;
- an adapter instance must echo the TaskSpec document ID and independently verify that the current plan still maps
  that ID to its supported document kind, confirmed `prepare` action, schema, and authoritative target.

The CLI accepts optional `--document-id` on status, prepare, run, and cancel. For backward compatibility, the sole
current Cover Letter target is resolved automatically when the option is absent. AgentResponse exposes only the
stable ID as body-free control metadata; document labels, source text, Brief content, evidence, prompts, and generated
prose remain outside control records.

## Consequences

- two Draft records or two Review records can coexist without overwriting cache, retry, failure, or recovery state;
- adding Research Statement no longer requires a second runtime or document-kind stage names;
- old non-document tasks remain byte-shape compatible and old Cover Letter runs remain readable;
- output ownership is still adapter-specific, so a capability cannot become `available` until it has a unique target,
  structured schema, current-basis validator, guarded promotion path, and matching Review behavior;
- global one-run-at-a-time coordination is intentionally retained; per-document parallel execution is not implied.

## Rejected Alternatives

- Add `research_draft`, `teaching_draft`, and similar stage names: rejected because document kind is an execution
  target, not a lifecycle stage, and the approach would duplicate retry/review logic indefinitely.
- Use authoritative output paths as identity: rejected because paths are adapter-owned mutable contract details and
  are not the stable Brief identity agents need for dispatch.
- Put every document in one Draft artifact and transaction: rejected because independent retry, review, and promotion
  would be lost and one failure would couple unrelated files.
- Replace WorkflowState with the derived execution plan: rejected because ADR-016's plan describes intended work,
  while immutable run records describe execution history and currentness.
- Rewrite all old records to 1.1: rejected because immutable audit evidence and cross-host task hashes must not change.

## Revisit When

Revisit before allowing parallel runs within one job, moving Package to per-document execution, or changing stable
document identity across a Required Document Plan migration.
