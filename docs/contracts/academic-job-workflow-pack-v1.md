# CanISend academic-job workflow Pack v1 contract

**Pack ID:** `org.canisend.academic-job`

**Pack version:** `1.0.0`

**Format:** `canisend.workflow-pack/v1`

**Content digest:** `3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07`

**Authority:**
`crates/canisend-resources/resources/workflow-packs/org.canisend.academic-job/manifest.json`

## Boundary

This is the first built-in reference Pack and the only Pack authorized to interpret legacy
academic Job/Agent v2 authority during Workspace v2→v3 migration. It is declarative data, not a
kernel ontology or executable plugin. The kernel continues to own evidence, consent, review,
export, recovery, audit, registered capability, and no-submission invariants.

The manifest and every declared body are embedded in the product resource catalog. Loading still
runs the bounded byte boundary, typed manifest validation, exact resource-set/size/SHA checks,
content-digest verification, runtime compatibility gate, capability registry gate, stage compiler,
and Deliverable catalog compiler. Built-in origin does not bypass any of those checks.

## Canonical inventory

| Class | Exact inventory |
|---|---|
| Locales | `en`, `zh-Hans` |
| Opportunity metadata | required `institution`; neutral Opportunity `title` remains kernel-owned |
| Stages | `intake`, `parse`, `criteria`, `evidence`, `match`, `plan`, `draft`, `review`, `package`, `render` |
| Requirement categories | `qualification`, `teaching`, `research`, `communication`, `leadership`, `service`, `employment`, `other` |
| Evidence categories | same eight stable legacy categories |
| Deliverables | required `cover-letter`, optional `research-statement`, optional `teaching-statement`, required `cv` |
| Prompts | Job parse, Evidence normalization, Evidence matching, document draft, document review |
| Templates | embedded ModernPro cover-letter/statement and CV templates |
| Validators | traceability, unsupported claims, placeholder-free, citation integrity, review complete |
| Intake references | local file, user URL, text PDF, RSS/Atom, jobs.ac.uk, Greenhouse, Lever |
| Renderer | registered bounded Typst renderer |

Stage prerequisites, outputs, and execution modes reproduce the current ten-stage academic graph.
The Pack graph uses qualified stage IDs while preserving the same dependency edges and allowed
manual, user-decision, Host Agent, configured-provider, and deterministic modes. Deliverable order,
cardinality, templates, renderer, and validators are compiled from the Pack rather than a second
academic catalog.

## Compatibility admission

Application-level Workspace migration resolves this checked-in Pack internally. Store admission
requires exact `org.canisend.academic-job` identity plus built-in origin, all eight Requirement and
Evidence categories, the `institution` field, and all four legacy Deliverables. An external bundle
that reuses the same ID fails before v3 authority or audit mutation.

The four network discovery references now resolve through the registered, Pack-qualified
[Opportunity-source adapter boundary](opportunity-source-adapters-v1.md). Agent v2 and `job` CLI
operations resolve through the exact-binding, fail-closed
[academic v2 compatibility boundary](academic-v2-compatibility-v1.md). GF3-UI-001 owns Pack-backed
presentation. External Pack installation, automatic latest-version selection, direct Pack-ID
conversion, and application submission remain unavailable.

## Parity evidence

Focused tests compare the verified Pack against the canonical v2 stage graph, dependency edges,
outputs, execution modes, Evidence/Requirement taxonomy, fixed `DocumentKind` order and
cardinality, exact prompt/template inventory, validator bindings, capability references, locales,
and v2→v3 golden migration. The former test-only two-stage Pack constructor has been removed, so
these paths consume one checked-in manifest and resource set.
