# Agent v4 task and resource contract

**Protocol:** `canisend.agent/v4`

**Workspace:** `canisend.workspace/v4`

**Schema version:** `4.0.0`

**Canonical task model resource:** `agent.v4.task-resource-model`

## Boundary

Agent v4 is the only host workflow being developed for Alpha.7. It is a clean protocol, not a
compatibility adapter for earlier Skills, host layouts, job aliases, Agent v2/v3 messages, or
Workspace v2/v3 state. An unsupported protocol, Workspace, unknown field, legacy operation ID, or
incomplete context fails before an application-facade mutation is attempted.

CanISend remains the state authority. Codex, Claude Code, and other MCP clients own conversation
and reasoning state but never write SQLite, Blobs, projections, or `.canisend` paths directly.
MCP is the preferred structured transport; the standalone native CLI is a semantic equivalent and
does not require the desktop App to be open.

## Canonical tasks

The task-resource model declares exactly ten composable tasks:

| Task | Neutral operation family | Context |
|---|---|---|
| `orientation` | `workspace.*`, `application.list`, `application.show` | Workspace; optional Application |
| `profile-evidence` | `profile.*`, `evidence.*` | Workspace; optional Application |
| `intake` | `source.intake.*`, `source.association.*` | exact Application |
| `application-create` | `application.create` | Workspace before creation |
| `requirements` | `requirement.*` | exact Application |
| `fit-plan` | `plan.*`, `evidence.match.*` | exact Application |
| `drafting` | `deliverable.*` | exact Application |
| `review` | `review.*` | exact Application |
| `export` | `export.*`, `render.*` | exact Application |
| `recovery` | `workspace.check`, `workspace.backup`, `workspace.restore`, `workspace.repair` | Workspace; optional Application |

Pack vocabulary may label Requirements, stages, and Deliverables differently, but it cannot add a
host-specific business rule or change these task identities. No operation family contains an
academic, generic, professional-job, or other domain-specific alias.

## Exact context and resources

Every request binds a canonical Workspace UUID and the literal Workspace v4 format. Tasks acting
on an existing Application also bind:

- Application UUID;
- exact Pack ID, semantic version, and content digest;
- expected positive Application revision; and
- exact current Application snapshot SHA-256.

Task resources are typed, revisioned where applicable, digest-bound, privacy-classified references.
The bounded set may describe Workspace health, Profile, Evidence, Sources, Requirements, Plan,
Deliverables, review, export, or backup state. Secret material is never a task resource; routine
orientation does not contain private bodies.

## Mutation sequence

Read-only orientation uses `orient → verify`. Every mutation uses the complete sequence:

```text
orient → propose → preview → approve → commit → verify
```

The proposal binds request, schema, candidate digest, operation, and exact context. The preview
binds proposal and preview digests, expiry, and required consent scopes. Approval binds the same
task and preview digest and records only consent explicitly granted by the user. Commit adds one
opaque, process-bounded preview token. A mismatch, denial, expiry, replay, stale revision, wrong
Pack, wrong Workspace, or host restart fails without mutation and requires a new preview where
applicable.

Committed receipts contain the new revision, snapshot digest, audit-event identity, and typed
artifact references. `submission_performed` must always be `false`; CanISend renders and exports
but never uploads or submits an Application.

## Generated schemas and examples

The source gate generates and verifies six schemas under `schemas/agent/v4/`:

- `canisend.agent-task-request/v4`;
- `canisend.agent-proposal/v4`;
- `canisend.agent-mutation-preview/v4`;
- `canisend.agent-approval/v4`;
- `canisend.agent-commit-request/v4`; and
- `canisend.agent-receipt/v4`.

Embedded orientation and Source-intake commit examples are validated through generated structure,
strong primitives, and semantic rules. The resource manifest binds the task model, schemas, and
examples by exact byte size and SHA-256 so later Codex and Claude Code generators consume one
integrity-checked source.
