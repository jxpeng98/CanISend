# ADR-008: Validate Candidates Before Single-File Atomic Promotion

**Status:** Accepted

**Date:** 2026-07-11

## Context

Host agents and deterministic executors must produce the same authoritative application artifacts without receiving
different write privileges. The existing orchestrator can write declared process output directly to a target path,
but process exit status and prompt instructions are not sufficient enforcement for stage execution.

An agent result may be stale, malformed, written outside the intended candidate directory, based on changed inputs,
or inconsistent with the task that requested it. Applying it directly could corrupt authoritative application state.

## Decision

Every executable stage uses a versioned TaskSpec and TaskResult. The TaskSpec declares:

- task, run, job, and stage identity;
- executor mode;
- canonical input fingerprint and safe input artifact references;
- allowed reads;
- one candidate output and one authoritative target for the Stage 1 Parse slice;
- output validator/schema and acceptance criteria;
- expected prior authoritative-output hash;
- privacy tier and required consent identifiers.

The executor writes only into the declared run candidate directory and returns a TaskResult that echoes task identity,
stage, input fingerprint, candidate path, and candidate hash. Deterministic execution goes through the same candidate
and apply services as current-host work.

Apply performs optimistic compare-and-swap validation:

1. reload the immutable TaskSpec;
2. verify TaskResult identity and terminal status;
3. recompute current inputs and reject stale work;
4. verify the authoritative target still matches its expected prior hash;
5. reject absolute paths, traversal, backslashes, and symlink escapes;
6. require the candidate to be inside the declared run candidate directory;
7. verify the candidate content hash;
8. validate JSON shape, Parsed Job semantics, and source-text receipts;
9. atomically replace the single authoritative target with validated bytes;
10. write safe promotion and finalized run receipts, then refresh the rebuildable state view.

Rejected candidates never modify the authoritative target. If recovery observes the target already matching the
validated candidate hash, it may finalize missing receipts; if it matches neither the expected prior nor candidate
hash, the run becomes a conflict requiring review.

## Consequences

- Host agents never need direct authority to write `parsed_job.json`.
- Deterministic and host-agent Parse share validation and promotion behavior.
- Stale results and concurrent attempts fail safely.
- Manual authoritative edits are preserved as output drift rather than overwritten.
- A single Parse output can use real atomic replacement; later multi-file stages require a separate transaction or
  journal decision.
- The legacy orchestrator cannot become the stage promotion boundary until it submits TaskResults to this service.

## Rejected Alternatives

- Trust agent-declared paths and privacy: rejected because prompts are not technical enforcement.
- Promote on process exit zero: rejected because success does not prove declared outputs, freshness, or schema.
- Let deterministic execution bypass staging: rejected because two promotion paths would drift.
- Overwrite an edited authoritative file: rejected because it destroys user review and provenance.

## Revisit When

Revisit before multi-output Draft or Package stages, pessimistic filesystem locking, or a transport that cannot use
workspace-local candidate files.
