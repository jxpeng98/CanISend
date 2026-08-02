# CanISend workflow-pack contract v1

**Format identifier:** `canisend.workflow-pack/v1`

**Schema version:** `1.0.0`

**Canonical schema:** `schemas/workflow-pack/v1/manifest.schema.json` in the embedded resource
catalog

**Runtime status:** Additive contract foundation. The current Alpha Job/Agent v2 runtime does not
load external workflow packs yet.

## Boundary

A workflow pack is declarative data and bounded resources. It describes domain vocabulary,
metadata fields, Requirement and Evidence categories, a stage DAG, Deliverable kinds, references
to kernel-registered capabilities, validators, localization, and declarative ID migrations.

A manifest cannot contain or select native libraries, shell scripts, JavaScript, WebAssembly,
executables, provider credentials, database handles, or arbitrary network/filesystem access.
Capability identifiers are references only. A future registry must match them against a
kernel-owned allowlist before a pack becomes usable.

## Identity and compatibility

- Pack IDs use a lowercase reverse-domain namespace, for example
  `org.canisend.academic-job`.
- The pack ID must be below its publisher namespace.
- Pack and resource versions are semantic versions.
- Kernel, Agent, and Workspace compatibility values are semantic-version requirements.
- Item IDs use bounded lowercase kebab case and remain stable within the publisher namespace.
- Every localized label contains the declared default locale and cannot reference undeclared
  locales.

`content_digest` is a strongly typed SHA-256 declaration. This contract slice validates its
shape. GF1-REG-001 will define and enforce canonical bundle hashing, resource-byte verification,
immutable snapshot storage, and pack substitution protection before the runtime installs or opens
external packs.

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

Manifest validation does not grant trust. Resource bytes, the declared content digest, registered
capability availability, publisher trust, pack installation, immutable snapshot binding, and
migration approval remain separate registry/runtime gates.

## DAG rule

Stages declare their prerequisites in `depends_on`. The graph must be acyclic, the terminal stage
must exist, and every declared stage must be an ancestor of that terminal stage. This prevents a
pack from shipping unreachable side workflows that bypass final review/readiness sequencing.

## Resource rule

The v1 resource kinds are `prompt`, `template`, `example`, and `translation`. Paths use the same
portable safe-relative-path primitive as other CanISend projections and cannot target
`.canisend`. Script and executable extensions are rejected independently of the declared kind.

Templates remain data passed to a future kernel-registered bounded renderer. A template resource
does not grant filesystem, network, package-resolution, system-font, or process-execution access.

## Compatibility promise

Adding this schema does not change `canisend.agent/v2`, `canisend.workspace/v2`, the current fixed
workflow, or existing Workspace data. Later v3 and migration slices must use this contract without
silently reinterpreting an existing Application or pack version.
