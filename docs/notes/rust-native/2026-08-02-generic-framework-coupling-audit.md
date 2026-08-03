# Generic framework coupling audit

**Date:** 2026-08-02

**Source baseline:** `ff316863d68c4781bb59b1c2a81a74aece2e2ca2` plus the checked inventory
implementation in this change

**Parent tasks:** M0-SCOPE-001, M0-SCOPE-002, GF0-AUDIT-001, GF0-BOUNDARY-001

## Outcome

CanISend now has a domain-neutral v3 kernel and two built-in workflow Packs, while bounded v2/job
compatibility remains intentionally present before Alpha.6. The remaining academic/job terms are
not treated as an undifferentiated cleanup list: every matching active file is generated into an
inventory and assigned to the kernel, academic Pack, optional adapter, compatibility surface, or
removal decision.

The checked authority is the
[domain-coupling inventory v1 contract](../../contracts/domain-coupling-inventory-v1.json). Its
stable digest means a new, removed, or reclassified coupling fails the source gate until the full
generated inventory is inspected and the contract is deliberately updated.

## Reproducible scan

The scanner covers active product, desktop, contract, embedded-resource, guide, fixture, and
repository-tooling paths. Historical plans, notes, release candidates, build output, and dependency
trees are excluded because they preserve prior facts or generated third-party content. The checked
inventory contract excludes itself so its search needles and projection examples cannot inflate
the generated findings.

```console
cargo run -p xtask --locked -- scope inventory --json
cargo run -p xtask --locked -- scope check
cargo run -p xtask --locked -- release check
```

The current generated inventory contains 188 unique files and digest
`60432078d2a735a9308b30fcd7ec115bfa4fb773eb4f35c0c869754075f79abe`.

| Pattern family | Matching files | Interpretation |
|---|---:|---|
| legacy job identity and command surfaces | 135 | Bounded v2/job names must remain compatibility-only |
| fixed academic Deliverables | 69 | Four-document assumptions belong to the academic Pack or compatibility layer |
| fixed `WorkflowStage` | 19 | The legacy enum cannot become the canonical Pack DAG |
| academic/faculty/jobs.ac.uk vocabulary | 82 | Each use must be Pack-, adapter-, compatibility-, or kernel-bound |

Pattern families may overlap within one file. Repository areas also overlap, for example an inline
Rust test can count as both Rust and test evidence.

| Required area | Matching files |
|---|---:|
| Rust | 75 |
| SQL | 7 |
| JSON Schema | 31 |
| embedded resources | 47 |
| desktop UI and bridge | 25 |
| active guides | 10 |
| fixtures and tests | 78 |
| projection paths and contracts | 16 |

| Ownership classification | Files | Boundary |
|---|---:|---|
| kernel | 35 | Neutral enforcement or explicit Pack-bound handling |
| academic Pack | 8 | Academic vocabulary, resources, templates, and manifest authority |
| optional adapter | 2 | Registered Opportunity-source behavior and its contract |
| compatibility surface | 143 | Agent v2, job CLI, Workspace v2, legacy SQL/projections, and their tests/docs |
| removal | 0 | No current matched file is silently assigned to removal; future removals must update the digest |

`scope check` also requires at least one matching file in every required area, rejects symlinked or
non-text scan inputs, rejects an unclassified finding, and compares the generated file-level
inventory, counts, families, and classifications with the checked contract. The complete list is
emitted only by `scope inventory --json` so the contract remains a compact review authority rather
than a second hand-edited copy of every path.

## Ownership classification

### Kernel

- safe IDs, timestamps, hashes, revisions, immutable blobs, audit events, dependency invalidation;
- Workspace/Application lifecycle, tasks, approval, consent, review, package, render, projection,
  backup, restore, check, and repair invariants;
- bounded URL/file/PDF/JSON/CSV/text parsing and registered render/intake ports; and
- factual-claim provenance, unsupported-claim rejection, and no-submission boundary.

### Academic reference pack

- “job”, “institution”, academic profile vocabulary, and discipline-specific Evidence labels;
- intake/parse/criteria/evidence/match/plan/draft/review/package/render stage labels;
- Cover Letter, CV, Research Statement, and Teaching Statement Deliverable definitions;
- current academic prompts, examples, templates, validators, and localized copy; and
- jobs.ac.uk association metadata.

### Optional registered adapters

- jobs.ac.uk, Greenhouse, Lever, and public feed Opportunity discovery;
- text-PDF, HTML, local-file, and provider/host boundaries; and
- Typst rendering and managed projection ports.

Adapters remain kernel-registered code. A workflow pack may select/configure an allowed adapter
but cannot ship executable network, filesystem, renderer, or provider code.

### Compatibility surfaces

- `canisend.agent/v2` envelopes and job-specific schemas;
- `job` CLI commands and `--job` arguments;
- Workspace v2, current SQL migration inputs, `job_id` wire fields, and fixed enum values; and
- managed `jobs/JOB_ID/` projections.

Compatibility is bounded to `org.canisend.academic-job`. These surfaces cannot address a generic
pack and cannot become the canonical implementation path.

### Canonical v3 replacements

- `Opportunity` plus `Application` and `application_id`;
- `Requirement`, `Plan`, `Deliverable`, pack-qualified `StageId`, and
  `DeliverableKindId`;
- `canisend.agent/v3`, `canisend.workspace/v3`, and `canisend.workflow-pack/v1`; and
- `applications/APPLICATION_ID/` managed projections.

### Removal or deferral

- academic-only branches in shared v3 services are removed after academic-pack extraction;
- no arbitrary executable pack hooks, shell/JavaScript/native extensions, or external Typst
  packages enter 1.0;
- dedicated first-party domain packs beyond academic and generic are deferred; and
- v2 compatibility removal requires a later major-version ADR and migration evidence.

## High-risk boundaries

1. **Identity split:** current Job identity also keys workflow and application state. The v3 model
   must deterministically map it to Opportunity/Application without duplicating revisions.
2. **Dynamic graph:** replacing `WorkflowStage` affects stale propagation, task operations,
   blockers, receipts, UI routing, and schema digests.
3. **Dynamic Deliverables:** `DocumentKind` is used by store queries, renderer selection,
   templates, package readiness, projection paths, and compatibility schemas.
4. **Migration atomicity:** schema, pack binding, dependencies, tasks, audits, and projections must
   never become a partially migrated mixed authority.
5. **Adapter parity:** current 37-operation accounting is v2/job-shaped. The v3 registry needs
   canonical neutral leaves and an explicit v2 alias class rather than a misleading count reuse.
6. **Resource trust:** moving domain logic into packs must not create an executable-plugin or
   unbounded-template bypass.

## Current implementation boundary

M0-SCOPE-002 owns only classification truth and drift detection. It does not remove compatibility
surfaces, alter Workspace data, advance the version, or authorize Alpha.6. M1F owns the reviewed
movement or retirement of each classified coupling, and every such change must intentionally
refresh this inventory before the release source gate can pass.
