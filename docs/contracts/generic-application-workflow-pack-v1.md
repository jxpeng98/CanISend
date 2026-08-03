# CanISend generic-application Workflow Pack v1 contract

**Pack ID:** `org.canisend.generic-application`

**Pack version:** `1.0.0`

**Format:** `canisend.workflow-pack/v1`

**Content digest:** `ffe269ae905b7fac851d82719f989876c7d310216b12922be6a5dd1aff67b321`

**Authority:**
`crates/canisend-resources/resources/workflow-packs/org.canisend.generic-application/manifest.json`

## Boundary

This is CanISend's domain-neutral starter Pack. It supplies declarative vocabulary, metadata,
taxonomies, stages, Deliverable kinds, a template, and validator bindings to the neutral v3
kernel. It does not add executable code, submission behavior, network authority, or a second
business engine.

The Pack enters the product through the same embedded-resource verification, bounded typed
manifest validation, exact resource-set/size/SHA verification, domain-separated content digest,
runtime compatibility, capability registry, stage compiler, Deliverable compiler, and locale
resolver as the academic reference Pack. Built-in origin does not bypass those checks.

## Canonical inventory

| Class | Exact inventory |
|---|---|
| Locales | `en`, `zh-Hans` |
| Opportunity metadata | optional `organization`, `reference`, `deadline`, `source-url` |
| Application metadata | optional `tracking-id`, `status`, `notes` |
| Stages | `intake`, `requirements`, `evidence`, `match`, `plan`, `compose`, `review`, `package`, `render` |
| Requirement categories | `eligibility`, `scope`, `evaluation`, `compliance`, `format`, `schedule`, `resources`, `other` |
| Evidence categories | `experience`, `capability`, `outcomes`, `credentials`, `references`, `resources`, `constraints`, `other` |
| Deliverables | required `primary-document`; zero to eight `supporting-document` instances |
| Intake references | local file, user URL, text PDF |
| Template | embedded domain-neutral `application-document.typ` |
| Validators | traceability, unsupported claims, placeholder-free, citation integrity, review complete |
| Renderer | registered bounded Typst renderer |

The graph has two legal roots, `intake` and `evidence`, and one terminal stage, `render`. The
`match` stage joins confirmed Requirements and Evidence before planning and Deliverable work.

## Neutrality invariant

No Pack metadata field is required. The kernel-owned Application title remains the only identity
value outside the optional Pack metadata. Declared field, category, stage, and Deliverable IDs do
not include the academic-only `institution`, qualification/teaching/research/employment taxonomy,
or cover-letter/statement/CV Deliverable set. English and Simplified Chinese vocabulary uses only
generic Application, Opportunity, Requirement, Evidence, and Deliverable terms.

These constraints are regression tested against the typed verified manifest rather than inferred
from documentation.

## Built-in registry

The app facade constructs one verified built-in registry containing the academic and generic Packs.
Resolution is exact by Pack ID, semantic version, and content digest. The registry never chooses a
latest version or substitutes bytes registered under an existing version.

## Current availability

GF4-PACK-001 makes the starter Pack embedded, verified, compiled, localized, and exactly
resolvable. GF4-FLOW-001, GF4-UI-001, and GF4-AGENT-001 provide canonical v3 execution plus
CLI/desktop/Agent Pack selection, field submission, resume, review, snapshot-bound approval, and
local export. Agent v2 and the `job` CLI remain academic-only compatibility surfaces and must fail
closed for this Pack.
