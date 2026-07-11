# ADR-003: Separate Operation Success, Gate Outcome, And Readiness

**Status:** Accepted

**Date:** 2026-07-10

## Context

`check-package` can execute correctly and still find blockers. Likewise, `packaged` means artifacts were generated,
not that an application is ready for manual submission. One boolean cannot represent operational failure, a negative
quality gate, and workflow readiness without ambiguity.

## Decision

- `ok` means the requested operation completed without an operational or domain error.
- `error` is present only when `ok` is false.
- A gate result is represented separately with `PASS`, `FAIL`, `STALE`, or `NOT_RUN`.
- Workflow readiness is represented separately with conservative values such as `blocked`, `action_required`,
  `review_required`, `ready_for_next_stage`, and `unknown`.
- A completed gate with blockers returns `ok: true`, `error: null`, a negative gate result, and the existing non-zero
  gate exit status.
- An operational failure returns `ok: false`, a stable error code, and exit status 1.
- Typer usage errors remain exit status 2 and outside the Phase 1 JSON contract.
- Cancelling an active prepared stage task is a successful state transition: it returns `ok: true`,
  `error: null`, `action_required`, and exit status 0. The terminal cancellation manifest preserves the audit trail,
  candidate, and prior authoritative output; cancellation neither applies the candidate nor clears output drift.
- Requesting cancellation when no matching active task exists is an operational failure with
  `stage.no_active_run` and exit status 1.

No internal state may imply that CanISend submitted an application. The strongest future readiness value is
`ready_for_manual_submission`.

## Consequences

- Agents distinguish retryable command failures from application blockers.
- Shell automation keeps useful non-zero gate behavior.
- `packaged`, review state, and manual-submission readiness cannot be conflated.

## Rejected Alternatives

- Set `ok: false` for every gate failure: rejected because the checker itself succeeded.
- Always exit 0 when JSON is valid: rejected because existing shell gate semantics are useful.
- Treat `job.yaml status: packaged` as ready: rejected because human and privacy gates may remain.

## Revisit When

Revisit when the stage runner introduces partial success, retry exhaustion, or multi-output rollback statuses.
