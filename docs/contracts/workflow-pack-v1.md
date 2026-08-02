# CanISend workflow-pack contract v1

**Format identifier:** `canisend.workflow-pack/v1`

**Schema version:** `1.0.0`

**Canonical schema:** `schemas/workflow-pack/v1/manifest.schema.json` in the embedded resource
catalog

**Runtime status:** Additive contract, bounded byte verification, Trust Report, locale resolver,
and verified-bundle registry foundation. The current Alpha Job/Agent v2 runtime does not read Pack
directories, install external workflow packs, or project Pack vocabulary into the GUI yet.

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
- Localized variants preserve the names and occurrence counts of default-locale placeholders.
- Visible Pack text cannot contain bidirectional formatting, isolate, or override controls.

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
read limit before JSON parsing and must re-run the typed structural and semantic validation. The
core byte verifier now applies those limits to already-supplied bytes; a future filesystem adapter
must still bound reads and reject symlinks/non-regular files before constructing that candidate.

## Semantic validation

The typed validator rejects:

- unsupported schema versions or invalid publisher namespaces;
- invalid semantic-version requirements;
- missing fallback locales, empty/oversized labels, unknown locales, placeholder mismatch,
  bidirectional control characters, and duplicate IDs;
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

## Localization rule

Pack vocabulary and localized labels resolve only after exact bundle verification. The core
`WorkflowPackLocalizationRuntime` binds every locale selection to the Pack ID, version, and content
digest, so a selection from another Pack or snapshot fails closed.

The current desktop preference remains the existing closed `en`/`zh-CN` setting. Pack resolution
uses these deterministic candidates in order:

| Desktop preference | Pack locale candidates | Final fallback |
|---|---|---|
| `en` | `en` | declared Pack default |
| `zh-CN` | `zh-CN`, `zh-Hans`, `zh` | declared Pack default |

An arbitrary requested Pack locale resolves by exact ID, then its primary-language ID when one is
declared, then the Pack default. After a locale is selected, an individual label uses that locale
when present and otherwise uses its required default-locale value. Both decisions report whether
the result was exact, compatible, or a Pack-default fallback. Locale selections serialize without
vocabulary or label bodies and reproduce the same result after the persisted desktop locale is
restored.

Placeholders use `{lowercase-kebab-case}` with a 64-byte key limit; `{{` and `}}` escape literal
braces. Every localized variant must preserve the default variant's placeholder names and
occurrence counts, although order may differ. The contract accepts ordinary multilingual Unicode,
including combining marks and right-to-left scripts, while rejecting embedded bidi formatting,
isolate, and override controls that could visually reorder trusted UI context.

This foundation returns localized text plus body-free selection/fallback metadata only. It does
not interpolate untrusted values, infer a locale from private content, parse free-form translation
resources, or change the current v2 desktop copy.

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

## Deliverable catalog rule

Manifest Deliverable IDs are stable local kebab-case IDs. The kernel qualifies them as
`<workflow-pack-id>:<local-deliverable-kind-id>`—for example,
`org.canisend.generic-application:statement`—using `DeliverableKindId`. Equal local names in two
Packs are distinct and cannot satisfy each other's planning or readiness counts.

The order of `deliverables.kinds` is authoritative presentation and planning order and remains
part of the Pack content digest. For every Kind, the core compiler independently enforces:

- 1–64 kinds per Pack and unique qualified identity;
- `0 <= minimum <= maximum <= 32`, with `maximum > 0`;
- an optional template path resolving to a declared `template` resource;
- an optional Renderer selected by the Pack, with verified-bundle construction separately
  requiring admission by the kernel capability registry; and
- unique Validator-instance references resolving to selected capabilities and their bounded
  declarative parameters, after the same verified-bundle capability gate.

The compiled template binding freezes resource ID, path, version, byte size, and SHA-256 from the
verified bundle. Runtime count validation rejects foreign/unknown Kinds, a count below `minimum`,
and a count above `maximum`, including duplicate instances of a singleton Kind. This is the v3
catalog foundation; the four fixed academic `DocumentKind` values remain only in the current v2
compatibility runtime until Workspace migration.

## Resource rule

The v1 resource kinds are `prompt`, `template`, `example`, and `translation`. Paths use the same
portable safe-relative-path primitive as other CanISend projections and cannot target
`.canisend`. Script and executable extensions are rejected independently of the declared kind.

All v1 resource bodies are UTF-8 text data. Before typed bundle verification, the byte boundary
rejects invalid UTF-8, disallowed control bytes, executable shebangs, and common ELF, PE, Mach-O,
WebAssembly, and ZIP binary signatures. Actual resource count, individual byte size, and total
bytes are limited before Manifest references or declared sizes are trusted.

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

## Trust Report rule

Successful byte verification produces a body-free
`canisend.workflow-pack-trust-report/v1`. It records exact Pack identity/version/digest, origin,
declared publisher ID, byte/count totals, selected registered capabilities, and the passed bounded
validation gates. It always distinguishes the following facts:

- the candidate is `verified-data-only`, not trusted executable code;
- the publisher identity is declared metadata only and has not been authenticated;
- workflow-pack v1 specifies no publisher signature;
- Pack data receives no execution authority; and
- external installation remains disabled.

The report contains no Manifest resource body, prompt, template, translation, or example text.
Digest and Schema verification prove exact-data integrity and compatibility; they do not prove the
publisher's real-world identity, the quality of a Pack, or user approval to install or migrate it.

## Compatibility promise

Adding this schema does not change `canisend.agent/v2`, `canisend.workspace/v2`, the current fixed
workflow, or existing Workspace data. Later v3 and migration slices must use this contract without
silently reinterpreting an existing Application or pack version.
