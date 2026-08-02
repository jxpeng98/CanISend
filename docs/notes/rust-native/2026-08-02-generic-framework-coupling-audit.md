# Generic framework coupling audit

**Date:** 2026-08-02

**Source baseline:** `84263ba6b3d8ec467aeaba8e05c5ec2ce2e81cfe` plus the scope-rebase
documentation in this change

**Parent tasks:** M0-SCOPE-001, M0-SCOPE-002, GF0-AUDIT-001, GF0-BOUNDARY-001

## Outcome

CanISend has a reusable evidence/revision/approval/review/export/recovery engine, but its public
domain boundary is not generic. The framework transition must change contracts, persistence, and
all adapters before Beta. Moving only UI labels or templates would leave the job-specific ontology
intact.

## Reproducible scan

The audit used active product, desktop, contract, and embedded-resource paths. Historical plans
were excluded because they preserve prior facts.

```console
rg -l 'job_id|JobId|jobs/JOB_ID' crates desktop docs/contracts resources
rg -l 'ResearchStatement|TeachingStatement|research-statement|teaching-statement' \
  crates desktop docs/contracts resources
rg -l 'WorkflowStage' crates desktop docs/contracts resources
rg -l 'academic|faculty|jobs\.ac\.uk' crates desktop docs/contracts resources
```

| Pattern family | Matching files | Interpretation |
|---|---:|---|
| `job_id`, `JobId`, `jobs/JOB_ID` | 82 | Aggregate identity and projection boundary are job-specific |
| fixed research/teaching Deliverables | 48 | Core, render, templates, operations, and tests assume four academic documents |
| fixed `WorkflowStage` | 11 | Public workflow state uses a ten-value enum rather than a pack DAG |
| academic/faculty/jobs.ac.uk vocabulary | 18 | Domain copy, fields, examples, and discovery behavior are mixed into active surfaces |

The 82-file identity/projection group is concentrated in:

| Area | Matching files | Primary owner |
|---|---:|---|
| embedded resources | 24 | Academic pack or v2 compatibility resources |
| store implementation | 16 | Workspace v3 persistence/migration |
| app facade | 15 | Neutral Application services |
| desktop Rust adapter | 7 | Pack-driven surface and approval context |
| SQLite migrations | 5 | v2 golden input and v3 migration |
| public Rust contracts | 5 | Agent/Workspace v3 model |
| CLI tests | 4 | v2 compatibility plus v3 canonical fixtures |
| other CLI/MCP/IO/store/app tests | 6 | Adapter mapping and shared semantic parity |

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

## First implementation boundary

The next code slice is GF1-SCHEMA-001: introduce the typed, data-only workflow-pack manifest and
its validation limits without changing the current Job, Agent v2, or Workspace v2 runtime. That
slice is additive and reversible. Dynamic stages, v3 contracts, migration, and academic extraction
remain separate reviewed slices after the manifest contract is proven.
