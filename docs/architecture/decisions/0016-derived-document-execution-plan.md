# ADR-016: Derive Document Execution Fan-Out From The Required-Document Plan

**Status:** Accepted for Task 8 of Stage 3

**Date:** 2026-07-14

## Context

Brief already produces one confirmed task for every advertised application document, but the guarded Draft runtime
can execute only one Cover Letter. A blocker-free `required_document_plan.json` can therefore appear ready even when
another prepared document has no schema, validator, or promotion path. Extending the single workflow `draft` record
directly to several independent documents would also create ambiguous cache, retry, and authoritative-output
ownership semantics.

The framework needs an honest fan-out boundary before new research, teaching, supporting, diversity, publication,
email, or interview generators are added. That boundary must not copy private Brief, advert, Evidence, or document
bodies into the control plane, and it must not imply that a planned executor or one reviewed Cover Letter makes the
whole application package ready.

## Decision

CanISend adds a versioned, deterministic `DocumentExecutionPlanV1` projection derived from the exact current
`required_document_plan.json` bytes. It is a read-only projection rather than a new authoritative stage artifact.
Every advertised requirement has one stable work item whose state is exactly one of:

- `blocked`: the Brief/document plan or that task still has an executable blocker;
- `omitted`: an optional document has an explicitly confirmed omit action;
- `ready_to_prepare`: a confirmed prepare action has a currently available guarded executor;
- `executor_unavailable`: preparation was confirmed but no guarded executor is implemented.

The projection records only stable document IDs, normalized kinds, requirement/action metadata, execution capability,
safe target/schema identifiers where implemented, reason codes, counts, and the exact source-plan hash. It never
contains labels, source text, Brief bodies, claims, Evidence bodies, prompts, or generated prose.

### Capability registry

A core-owned registry is the single mapping from normalized document kind to execution capability. The existing
Cover Letter runtime is `available` and retains its current `draft` stage, schema, target, host-agent, and
configured-provider contracts. Research, teaching, supporting, diversity, publication, CV, personal-statement,
application-form, reference, and writing-sample routes are registered as `planned`; unknown kinds are
`unregistered`. Application email and interview preparation are also declared as planned workflow-support routes,
but they are not inferred as advertised submission requirements.

Adding a new available executor later requires its own structured artifact schema, input basis, validator, guarded
candidate promotion, Review behavior, and output ownership. Changing a registry entry alone cannot make a planned
executor available.

### Aggregate semantics

- `ready`: every confirmed prepare task is dispatchable now;
- `partially_dispatchable`: at least one task is dispatchable and at least one lacks an executor;
- `blocked`: source blockers remain, or no requested task can currently be dispatched;
- `no_work`: the confirmed plan contains no prepare action.

`documents status` exposes only body-free counts, hashes, state, generic blockers, artifact references, and next
actions through the existing AgentResponse v1 extension boundary. Status inspection is read-only. A partial plan may
still offer the current Cover Letter Draft action, but it must also report that complete fan-out is blocked.

## Consequences

- advertised documents now determine an explicit downstream execution inventory instead of silently falling outside
  the Cover Letter path;
- agent hosts can distinguish a user/Brief blocker from a framework capability gap without reading private bodies;
- each future document executor can be added independently without changing the Required Document Plan contract;
- the current single-output Draft stage is not falsely generalized before per-document retry and ownership semantics
  are designed;
- a planned, unavailable, drafted, or reviewed single document does not establish application-package readiness.

## Rejected Alternatives

- Add every document to the current Cover Letter Draft model: rejected because document structures and review rules
  differ and one stage record cannot truthfully represent several independent candidates.
- Generate legacy Markdown directly for unsupported kinds: rejected because it bypasses structured validation and
  guarded promotion.
- Persist a mutable execution-status file: rejected because it could drift from the authoritative Brief plan and
  stage outputs; Task 8 state is always re-derived from the current source hash.
- Treat registered `planned` routes as dispatchable: rejected because a name or future target is not an executor.
- Infer email/interview work as required submission documents: rejected because those are workflow-support artifacts
  unless the user explicitly requests them in a later contract.

## Revisit When

Revisit before making a second document executor available, introducing per-document run records, aggregating
cross-document Review, or promoting Package readiness.
