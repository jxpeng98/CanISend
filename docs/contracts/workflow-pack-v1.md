# CanISend workflow-pack contract v1

**Format identifier:** `canisend.workflow-pack/v1`

**Schema version:** `1.0.0`

**Canonical schema:** `schemas/workflow-pack/v1/manifest.schema.json` in the embedded resource
catalog

**Runtime status:** Additive contract and verified-bundle registry foundation. The current Alpha
Job/Agent v2 runtime does not load or install external workflow packs yet.

## Boundary

A workflow pack is declarative data and bounded resources. It describes domain vocabulary,
metadata fields, Requirement and Evidence categories, a stage DAG, Deliverable kinds, references
to kernel-registered capabilities, validators, localization, and declarative ID migrations.

A manifest cannot contain or select native libraries, shell scripts, JavaScript, WebAssembly,
executables, provider credentials, database handles, or arbitrary network/filesystem access.
Capability identifiers are references only. `canisend-core` matches them against a kernel-owned
registry while verifying a bundle; a match grants access only to that pre-registered capability.

## Identity and compatibility

- Pack IDs use a lowercase reverse-domain namespace, for example
  `org.canisend.academic-job`.
- The pack ID must be below its publisher namespace.
- Pack and resource versions are semantic versions.
- Kernel, Agent, and Workspace compatibility values are semantic-version requirements.
- Item IDs use bounded lowercase kebab case and remain stable within the publisher namespace.
- Every localized label contains the declared default locale and cannot reference undeclared
  locales.

`content_digest` is a strongly typed SHA-256 binding over the normalized manifest and every exact
declared resource byte. Bundle verification also checks resource set equality, resource size and
SHA-256 declarations, runtime compatibility, and capability availability before producing a
snapshot.

### Canonical bundle digest

The v1 digest algorithm is deliberately independent of input JSON object-key order:

1. validate and deserialize the manifest as `canisend.workflow-pack/v1`;
2. serialize it as compact JSON with every object key sorted recursively and replace
   `content_digest` with 64 ASCII zeroes;
3. initialize SHA-256 with the domain bytes `canisend.workflow-pack-bundle/v1\0`;
4. append each following segment as an unsigned 64-bit big-endian byte length followed by the
   exact segment bytes: `manifest`, normalized-manifest bytes, then for each resource in ascending
   portable path order: `resource-path`, path bytes, `resource-bytes`, and exact resource bytes;
5. encode the result as lowercase hexadecimal.

Array order remains part of the manifest. Resource map/insertion order does not. The manifest's
declared digest must equal the calculated digest; callers cannot select a different digest
algorithm.

## Structural limits

| Item | Limit |
|---|---:|
| Serialized manifest | 1 MiB |
| JSON nesting depth | 32 |
| JSON nodes | 20,000 |
| Locales | 16 |
| Application fields | 128 |
| Categories per taxonomy | 64 |
| Workflow stages | 64 |
| Deliverable kinds | 64 |
| Capability references per class | 64 |
| Validator instances | 128 |
| Resources | 512 |
| One declared resource | 8 MiB |
| Total declared resources | 64 MiB |
| Predecessor migrations | 64 |
| ID mappings per migration | 512 |

These are contract limits, not allocation advice. A byte-oriented loader must enforce its own
read limit before JSON parsing and must re-run the typed structural and semantic validation.

## Semantic validation

The typed validator rejects:

- unsupported schema versions or invalid publisher namespaces;
- invalid semantic-version requirements;
- missing fallback locales, empty/oversized labels, unknown locales, and duplicate IDs;
- invalid field option shapes;
- missing, duplicate, self-referential, cyclic, or terminal-disconnected workflow stages;
- empty or duplicate execution modes;
- invalid Deliverable cardinality, template paths, renderers, or validators;
- validator instances using capabilities not selected by the manifest;
- unknown readiness validators;
- duplicate resource paths, zero/oversized resources, total-size overflow, and script/executable
  file extensions;
- duplicate or identity migration mappings; and
- unsafe relative paths or malformed SHA-256 digests.

Manifest validation alone does not grant trust. Verified-bundle construction separately enforces
resource bytes, the declared content digest, current runtime compatibility, and registered
capability availability. Publisher trust, explicit installation, persistent snapshot binding, and
migration approval remain separate runtime/storage gates.

## DAG rule

Stages declare their prerequisites in `depends_on`. The graph must be acyclic, the terminal stage
must exist, and every declared stage must be an ancestor of that terminal stage. This prevents a
pack from shipping unreachable side workflows that bypass final review/readiness sequencing.

Manifest stage IDs are stable local kebab-case IDs. The kernel qualifies them as
`<workflow-pack-id>:<local-stage-id>`—for example,
`org.canisend.generic-application:review`—using the strongly validated `StageId` type. A stage
from one Pack therefore cannot satisfy a dependency or runtime lookup in another Pack, even when
both use the same local name.

The core compiler independently rechecks the 1–64 stage bound, unique stages and dependencies,
1–5 unique execution modes, declared dependencies, acyclicity, terminal existence, and terminal
reachability. It produces a stable lexical Kahn topological order that does not depend on manifest
stage declaration order. Descendant queries follow that order so later dependency invalidation
and scoped rerun behavior can remain deterministic.

`output` and `execution_modes` are closed kernel-owned enums. Unknown values fail Schema
validation before graph compilation; a Pack cannot introduce a new execution mechanism or output
authority by naming it in data.

## Resource rule

The v1 resource kinds are `prompt`, `template`, `example`, and `translation`. Paths use the same
portable safe-relative-path primitive as other CanISend projections and cannot target
`.canisend`. Script and executable extensions are rejected independently of the declared kind.

Templates remain data passed to a future kernel-registered bounded renderer. A template resource
does not grant filesystem, network, package-resolution, system-font, or process-execution access.

## Registry and snapshot rule

The core registry keys bundles by exact `(pack ID, pack version)` and resolves them only when the
caller also supplies the expected content digest. It has no `latest` selector. Re-registering the
same verified bytes is idempotent; attempting to replace a registered version with a different
digest fails as version substitution. A same-digest/different-content state fails as a digest
collision.

The generated `canisend.workflow-pack-snapshot/v1` value records pack ID, version, origin,
content digest, canonical manifest SHA-256, and the sorted resource identity/version/path/size/hash
inventory. It is an immutable binding value for future Workspace persistence; the current
in-memory registry does not yet install files or mutate a Workspace.

## Compatibility promise

Adding this schema does not change `canisend.agent/v2`, `canisend.workspace/v2`, the current fixed
workflow, or existing Workspace data. Later v3 and migration slices must use this contract without
silently reinterpreting an existing Application or pack version.
