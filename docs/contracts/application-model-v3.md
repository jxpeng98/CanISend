# CanISend application-model contract v3

**Format identifier:** `canisend.application-model/v3`

**Schema version:** `3.0.0`

**Canonical schemas:** `schemas/v3/*.schema.json` in the embedded resource catalog

**Runtime status:** Additive neutral model, transactional repository, backup-backed semantic
migration, and a verified built-in academic reference Pack. Job/Agent v2 remains the compatibility
runtime until canonical v3 adapters land. Neutral Store projections and dependency-scoped Pack
migration are implemented; Agent v3/shared-surface operations remain separate roadmap tasks.

## Boundary

The v3 application model describes one exact Pack-bound application snapshot without importing
the academic v2 ontology. It defines neutral Opportunity, Application, Requirement, Plan, and
Deliverable records plus their revision references and aggregate consistency rules.

The contract does not install a Pack, select a newer Pack version, write a Workspace, migrate v2
data, execute a workflow stage, render content, export files, or submit an application. It is a
typed interchange and future persistence boundary only.

## Identity and Pack binding

Opportunity, Application, Requirement, Plan, and Deliverable use distinct strongly typed UUIDv7
identities. A valid UUID cannot be substituted for another entity kind in Rust APIs without an
explicit conversion through the shared kernel identity.

Every record stores the same exact `ApplicationPackBindingV3` value:

- workflow Pack ID;
- semantic Pack version; and
- verified Pack content SHA-256 digest.

The aggregate validator rejects any record whose binding differs in any component. There is no
`latest` selector and no version-only match. Pack-owned Deliverable kinds remain qualified as
`<workflow-pack-id>:<local-deliverable-kind-id>` and fail when their namespace differs from the
record's Pack binding. Other Pack taxonomy values use validated local item IDs interpreted only
inside that exact binding.

An ordinary Application commit cannot change any Pack-binding component. A dedicated migration
may advance only to a verified higher version of the same Pack ID when that target declares the
current version as a predecessor. The reviewed migration atomically rewrites every entity binding,
maps declared taxonomy IDs, and preserves immutable source history. A direct Pack-ID replacement
requires a future import/clone boundary rather than migration.

## Records

### Opportunity

An Opportunity records a neutral title, Pack-defined metadata, bounded source identities,
creation time, revision, and archive state. It does not require a job, institution, faculty,
course, funder, buyer, or other domain-specific field.

### Application

An Application references one Opportunity, repeats the exact Pack binding, carries Pack-defined
metadata, and has a neutral `draft`, `active`, `paused`, `completed`, or `archived` lifecycle.
Creation and update timestamps are ordered by parsed RFC 3339 time rather than textual form.

### Requirement

A Requirement belongs to one Application and records:

- a Pack-owned category;
- a bounded statement and neutral priority;
- an exact source content identity, revision, SHA-256 digest, and nonempty byte span;
- `proposed`, `confirmed`, or `excluded` state; and
- revision identity.

Only an explicit user decision with a timestamp can confirm or exclude a Requirement. A host
Agent, configured provider, or system actor cannot acquire that authority.

### Plan

A Plan binds exact Requirement revisions, Pack-owned decision and Deliverable identifiers,
bounded constraints and blockers, user decision authority, and its own revision. Draft Plans
cannot carry final authority. Confirmed Plans require a Pack-owned decision plus an explicit user
and timestamp. A stale Plan preserves its earlier Requirement revisions and either its complete
confirmed user decision or its undecided draft state; it cannot pretend those inputs are current.

The kernel owns execution-mode values. A Pack may select only one of those registered modes; it
cannot introduce an executable mechanism by naming one in data. An omitted Deliverable cannot
select an executor.

### Deliverable

A Deliverable binds one Application, the Plan revision that actually produced it, a
Pack-qualified Deliverable kind, neutral title/state, optional exact content revision and SHA-256
digest, a bounded MIME type, and revision-bound Evidence inputs.

`planned` carries no materialized content. `draft`, `review-required`, `approved`, and `stale`
require both exact content and a valid type/subtype media token. A current materialized
Deliverable must be included and not omitted by its exact snapshot Plan. A stale Deliverable may
preserve an earlier Plan revision and Kind so audit/history never rewrites what produced it.

## Pack-defined metadata

Opportunity and Application metadata keys are validated Pack item IDs. Values are closed typed
data:

- short or long text;
- signed integer;
- boolean;
- validated `YYYY-MM-DD` calendar date;
- bounded HTTP/HTTPS URL without controls or whitespace;
- bounded text list; or
- Pack-owned choice ID.

Unknown JSON fields fail structural validation. Metadata and collections have semantic upper
bounds; byte-oriented Workspace and Agent adapters must additionally enforce their own input-byte
and JSON-shape limits before deserialization.

## Aggregate invariants

`ApplicationModelSnapshotV3` permits a new draft with no extracted Requirements, no Plan, and no
Deliverables. Absence is represented explicitly and is not replaced by fabricated Evidence.

When records exist, validation rejects:

- Opportunity/Application or record-level Pack mismatches;
- Application-to-Opportunity or child-to-Application mismatches;
- duplicate Requirement or Deliverable identities;
- duplicate, unknown, or future Requirement revision inputs;
- current Plans whose Requirement/blocker references are not exact;
- Deliverables without a Plan, current Deliverables bound to an older Plan, or stale
  Deliverables bound to a future Plan;
- current Deliverable kinds absent or omitted from the Plan;
- cross-Pack Deliverable kinds;
- non-user confirmations and decisions;
- inconsistent Deliverable content state; and
- invalid text, date, URL, media-type, span, uniqueness, or collection bounds.

Semantic violations carry stable `application_v3.*` codes and JSON pointers. External aggregate
candidates run generated JSON Schema validation before semantic validation.

## Schema registry

The seven v3 schemas are generated deterministically from Rust types:

- `application-pack-binding.schema.json`;
- `opportunity.schema.json`;
- `application.schema.json`;
- `requirement.schema.json`;
- `plan.schema.json`;
- `deliverable.schema.json`; and
- `application-model.schema.json`.

They use an independent `ApplicationModelSchemaId` registry and do not modify the frozen set of
40 Agent v2 public schemas. The source gate checks registry completeness, Draft 2020-12
meta-schema validity, canonical metadata, deterministic bytes, exact generated file sets, and the
embedded resource catalog.

A regression test recursively inspects every v3 `required` array for academic-only fields and
rejects fixed v2 `ArtifactKind`, `ApplicationDecision`, and `DocumentKind` types or academic
Deliverable values.

## Compatibility promise

This additive contract does not change `canisend.agent/v2`, existing Job data, Job projections, or
fixed academic document behavior. Schema migration 14 adds dormant v3 model tables; migration 15
adds the migration ledger and immutable legacy bindings. Only the explicit dry-run-first,
verified-backup migration may create the `canisend.workspace/v3` authority row. Agent v2 and `job`
compatibility remain bounded to the academic reference Pack.
