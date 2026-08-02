# GF2 neutral application-model implementation record

**Date:** 2026-08-02

**Roadmap task:** GF2-MODEL-001

**State:** Implemented and source-gate verified in this change. Workspace v3 persistence,
v2-to-v3 migration, generic projections, dependency invalidation, and Agent v3 operations remain
separate roadmap tasks.

## Implemented boundary

- Added distinct UUIDv7-backed Opportunity, Application, Requirement, Plan, and Deliverable ID
  types without replacing v2 Job or artifact identities.
- Added additive neutral record types under `canisend.application-model/v3`.
- Bound the aggregate and every child record to exact workflow Pack ID, semantic version, and
  content digest.
- Used Pack local item IDs for metadata, categories, and decisions, plus Pack-qualified
  `DeliverableKindId` for cross-Pack isolation.
- Added generic typed metadata without a job, institution, academic profile, fixed document kind,
  or domain-specific required field.
- Required exact content identity/revision/digest and source spans for Requirements.
- Preserved user authority over Requirement confirmation/exclusion and Plan confirmation.
- Bound Plans to exact Requirement revisions and Deliverables to exact Plan revisions.
- Allowed an honest new-draft state with no Requirements, Plan, Deliverables, or fabricated
  Evidence.
- Added an independent seven-entry v3 Schema registry without changing the frozen 40-schema Agent
  v2 registry.
- Generated and embedded all seven canonical Schema files with their own `3.0.0` metadata.
- Generalized embedded Schema version verification to compare each resource declaration against
  its generated `x-canisend-version`, retaining digest and catalog verification across v1/v2/v3.

## Contract invariants and regression coverage

- unknown fields and invalid strong primitives fail structural validation;
- exact Pack digest mismatches and foreign Pack Deliverable kinds fail closed;
- stale Requirement and Plan revision references fail aggregate validation;
- only explicit user authority can confirm/exclude a Requirement or confirm a Plan;
- planned Deliverables cannot carry content, while materialized states require exact content and
  a valid bounded media type;
- dates, URLs, source spans, metadata, sources, Requirements, blockers, Deliverables, and text
  collections are bounded or semantically validated;
- a neutral generic funding fixture validates without academic fields;
- draft/no-Evidence state validates without synthesizing Requirements or content; and
- generated Schema tests scan required fields and serialized type definitions for academic v2
  coupling.

## Verification

```console
cargo test -p canisend-contracts --locked
cargo test -p canisend-resources --locked
cargo run -p xtask --locked -- schemas check
cargo run -p xtask --locked -- resources check
cargo clippy -p canisend-contracts -p canisend-resources -p xtask \
  --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- release check
```

## Remaining boundary

GF2-STORE-001 must design and persist Workspace v3 authority behind neutral application services.
It must retain exact Pack snapshots, typed identities, revisions, confirmations, audit context, and
transactional stale propagation. This contract intentionally does not create SQLite tables or
partially migrate v2 authority.

GF2-MIG-001/002 must map one v2 Job to one Opportunity plus one Application under the exact
academic Pack, using dry-run, verified backup, failure-atomic commit, old-binary behavior, and
semantic-inventory tests. GF2-PROJ-001 and GF2-INVALID-001 then own generic projection and
Pack-change invalidation behavior.

Before Beta, Agent v3 must adopt these nouns and Pack bindings as its canonical surface. Agent v2
and Job commands remain unchanged compatibility surfaces in this slice.

## Rollback

Revert the v3 Rust module, independent Schema registry, seven generated/embedded resources,
resource version check, documentation, and Roadmap evidence row together. No Workspace or runtime
data rollback is needed because this slice performs no persistence or migration.
