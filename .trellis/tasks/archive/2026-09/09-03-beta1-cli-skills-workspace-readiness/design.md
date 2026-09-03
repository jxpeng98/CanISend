# Design: private Beta.2 readiness

## Scope and boundaries

This change prepares a private `v1.0.0-beta.2` source candidate. It extends the existing release
state machine and packaged Agent v4 smoke in place. It does not add a release service, another test
framework, direct CLI mutation authority, Workspace legacy migration, or a publication path.

The existing application facade, operation registry, four embedded Skills, persistent MCP broker,
Workspace v4 database transaction, stage-transition renderer, and feature-freeze exception model
remain the owners of their current behavior.

## Release decision

Add one accepted ADR that amends the 1.0 sequence to allow exact sequential Beta iterations. Keep
the historical ADR unchanged. Update the machine policy and release documentation to describe:

- `beta.N -> beta.(N+1)` on the same release line only;
- a qualified active Beta and frozen feature baseline as preconditions;
- RC.1 as a later transition from the latest qualified Beta; and
- source preparation as separate from qualification, tagging, and publication.

## Qualification ledger

Retain schema `canisend.release-qualification/v1` and the existing `beta` field as the active/latest
Beta slot. Add `beta_history` only when the first sequential transition is prepared; an absent field
is treated as an empty history for the already-qualified Beta.1 ledger.

Before Beta.2 preparation:

- `beta` is the canonical qualified `v1.0.0-beta.1` record;
- `beta_history` is absent or empty; and
- `feature_freeze` remains `frozen` at its existing baseline.

After Beta.2 source preparation:

- the exact Beta.1 object is appended once to `beta_history`;
- `beta` becomes the canonical `{ "status": "pending" }` active slot;
- `workspace_stage`, release status, and release-notes status remain Beta values; and
- the feature-freeze object and historical Beta-entry evidence are unchanged.

Validation rejects malformed history, non-Beta tags, non-sequential or cross-line records,
duplicate tag/run/source identities, noncanonical signing targets, and a history entry that does not
match the source Beta being advanced. RC, Stable, upgrade, and package-manager checks continue to
use only the active `beta` slot. They never fall back to history.

`record-beta-qualification` accepts the canonical pending active Beta when either the original
planned-freeze Beta-entry state is present or a nonempty valid Beta history and frozen baseline prove
a sequential Beta. It records only the exact current workspace tag. Derived release status reports
Beta.1 as qualified history and Beta.2 as private/pending instead of calling preserved Beta.1
evidence current.

## Transition behavior

Extend the existing `prepare-stage` path rather than adding a command. The shared transition
validator enforces numeric `N + 1`; the ledger precondition requires the current workspace version
to match the active qualified Beta tag. The renderer appends that record to history and resets the
active slot within its existing transactional controlled-file write.

Dry-run and write mode must produce the same controlled paths and after-digests. Existing atomic
write and rollback behavior remains the only mutation mechanism. Beta iteration does not reset
Beta readiness, contract-freeze, feedback, freeze baseline, RC evidence, or Stable authorization.

## CLI, Skills, and Workspace gate

Extend `scripts/smoke_agent_v4_mcp.sh`, which already proves the dual-Pack guarded lifecycle,
backup, restore, and reopen path. In the same disposable fixture it will also run project-scoped
host setup/status and assert the four expected Skill files, ownership manifest entries, digests,
and MCP guidance. Existing focused tests continue to own refusal, idempotency, body-free output,
token replay, stale revision, migration rollback/retry, and recovery-destination failures.

No product code changes are planned for Workspace migration: the current single immediate
transaction already provides the required all-or-nothing behavior. A product fix is allowed only
if the extended smoke or an existing focused test reproduces a release-blocking defect.

## Delivery sequence

1. Policy/readiness PR: ADR, policy, validators, focused tests, documentation, and smoke extension.
   Commit release-blocker changes first, then add the exact feature-freeze exception in a separate
   evidence commit. Merge only after the source gate and Fast CI pass.
2. Source-transition PR: from the merged clean base, preview and apply
   `prepare-stage v1.0.0-beta.2 --write`, inspect the exact controlled set, then add its exact
   feature-freeze exception in a second commit. Merge only after the source gate and Fast CI pass.

Neither PR creates a tag, dispatches a workflow, publishes a release, or writes a Beta.2
qualification record.

## Risks and rollback

- Ledger ambiguity: canonical validation and negative fixtures fail closed before any write.
- Historical evidence drift: Beta.1 is copied byte-for-byte into append-only history; active
  consumers never infer a candidate from history.
- Freeze bypass: each nonautomatic change is bound to its exact preceding commit and sorted path
  set; a branch missing the evidence commit fails the release check.
- Transition failure: the existing transactional writer restores the pre-transition bytes; a PR
  can be reverted without changing public Beta.1.
- Cohort identity: existing Beta.1 cohort evidence remains historical. Rebinding invited-user or
  provider observations to a future public Beta.2 is deferred until publication is authorized.
