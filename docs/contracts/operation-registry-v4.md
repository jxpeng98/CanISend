# Neutral operation registry v4

**Format:** `canisend.operation-registry/v4`

**Workspace:** `canisend.workspace/v4`

**Agent protocol:** `canisend.agent/v4`

**Canonical resource:** `agent.v4.operation-registry`

**Generated schema:** `schemas/agent/v4/operation-registry.schema.json`

## Authority and scope

The v4 registry is the only operation vocabulary being implemented for the Alpha.7 App, native
CLI, MCP server, and first-party Agent resources. It is independent from the historical v1
registry and contains no compatibility aliases. A v1 adapter or old job-specific command cannot
be used as evidence that a v4 operation exists.

The registry contains the neutral namespaces `workspace`, `application`, `profile`, `source`,
`evidence`, `requirement`, `plan`, `deliverable`, `review`, and `export`. Pack manifests own labels,
stages, Deliverable kinds, and validators; they cannot add a host-only operation or business rule.

Workspace-scoped Profile Sources and confirmed Evidence remain invisible to an Application until
their respective `profile.association.*` or `evidence.association.*` operations create an exact,
revision-bound link. Each family provides a body-free Application-scoped list plus a guarded
preview/commit pair; neither list implies consent or association.

## Surface projection

Every canonical dotted operation ID has exactly three mechanically derived adapter names:

| Surface | Projection rule | Example |
|---|---|---|
| native CLI | replace dots with spaces | `deliverable draft preview` |
| MCP | prefix `canisend_`, replace dots with underscores | `canisend_deliverable_draft_preview` |
| Tauri | replace dots with underscores | `deliverable_draft_preview` |

The operation ID—not an adapter name—is recorded in Agent requests, receipts, audit events, and
release evidence. The native CLI, MCP, and Tauri implementations must reject any adapter that is
not present in the integrity-checked registry.

## Mutation rule

Read operations have no phase suffix. Every mutation is an exact `.preview` / `.commit` pair with
the same context and Agent task. Preview performs validation and returns a bounded diff, required
consent, expected revision, and an opaque single-use token. Commit requires the matching token and
approval. Stale, replayed, wrong-Workspace, wrong-Application, wrong-Pack, denied-consent, and
malformed requests fail without authoritative mutation.

`workspace.initialize.*` is the only host-context pair because Workspace authority does not yet
exist. All other operations require a Workspace; Application-context operations additionally bind
the exact Application, Pack, revision, and snapshot digest.

## Integrity and compatibility

The embedded resource manifest binds the registry bytes. Source gates validate version literals,
namespace completeness, unique IDs, unique mechanical adapter projections, phase suffixes,
preview/commit pairing, context equality, and Agent task ownership. Tokens indicating old Agent or
Workspace versions, academic or generic modes, or job-specific aliases are rejected.

Alpha.7 does not silently negotiate this contract with Workspace v2/v3, Agent v2/v3, old Skills,
or job aliases. Unsupported input is diagnosed before any application-facade mutation and points
to clean Workspace v4 initialization.
