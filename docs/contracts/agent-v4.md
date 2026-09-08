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

### MCP confirmation requests

MCP commit inputs use `request_confirmation`, not a model-supplied approval assertion. True
requests the server's native exact-preview form; false consumes the selected preview without
committing. The server authorizes a change only after an actual `accept` response with exactly
`{"confirm":true}`. Decline, cancel, false, unsupported form capability, malformed responses and
timeout fail closed. Single-use tokens remain bound to the same MCP process, exact preview and
current context. A request to display a form is not permission to answer it for the user.

Private operations similarly use `request_private_read` and `request_private_export` to request
separate native consent. These flags do not assert consent; private data remains unavailable
until the corresponding form is accepted. Commit and private consent are independent gates.

This is a breaking correction to the developing MCP input schemas: old `approved`,
`confirmed_private_read` and `confirmed_private_export` fields are rejected rather than treated
as aliases. Hosts must rediscover the tools and discard old previews when reconnecting to the
updated binary. Internal trusted application services retain their explicit approval/consent
types; this change does not turn model input into an approval or alter persisted v4 data.

Use `canisend_application_pack_show` (CLI `application pack show`) to read the complete verified
manifest for an Application's exact Pack binding before selecting Deliverables. A validator's
first error is not a complete catalog. The response includes every kind's minimum and maximum;
it does not expose user Source or Deliverable bodies.

`canisend_evidence_confirm_preview` accepts an exact imported ProfileSource reference and an
`EvidenceProposalSet`. Quotes must match byte ranges in that source's normalized artifact;
source digests, Profile revision and sensitivity are checked before a preview is issued.
`canisend_evidence_confirm_commit` requests native confirmation of that exact catalog, with
separate private-read consent when required. It creates Workspace Evidence without changing
the Application or automatically associating Evidence. Use the existing guarded association
tools afterward. Changed source/context, denial and replay cannot commit the saved preview.

### Local task coordination

The CLI `local-task list/prepare/show/claim/submit/cancel/candidate-show` commands coordinate bounded
Deliverable draft candidate work on one device. A task binds an exact Application snapshot;
its coordination generation is separate from the Application revision. These commands do not
add canonical Agent task kinds. `local-task list --application ID` returns the latest 100 tasks
as body-free metadata so a new session can recover task IDs without the old conversation.

A claim lease permits a worker to hand off a candidate. It grants no Evidence, content-change,
export, or private-read approval. CLI private payload reads require `--confirm-private-read`.
Submitted candidates remain untrusted input; submission or cancellation never advances the
Application revision.

`canisend_local_task_draft_preview` loads one exact Submitted task candidate using its Application
ID, task ID, generation, and candidate digest. `request_private_read` requests native consent
before reading the payload. The existing draft validator then prepares `local-task.draft.preview`;
this does not approve the candidate or change the Application. Use the returned token and digest
with the existing `canisend_deliverable_draft_commit`, whose native form remains mandatory.
A successful commit creates the approved draft and marks the local task Committed atomically.
A stale task, changed Application or inputs, cancellation, denial, or replay cannot commit the
handoff. There is no separate local-task commit tool, GUI integration, or two-Host qualification.

## Canonical tasks

The task-resource model declares exactly ten composable tasks:

| Task | Neutral operation family | Context |
|---|---|---|
| `orientation` | `workspace.status`, `application.list`, `application.show` | Workspace; optional Application |
| `profile-evidence` | `profile.*`, Evidence list/show/propose/confirm | Workspace; optional Application |
| `intake` | `source.intake.*`, `source.association.*`, `source.list`, `source.show` | exact Application |
| `application-create` | `application.create.*` | Workspace before creation |
| `requirements` | `requirement.*` | exact Application |
| `fit-plan` | `plan.*`, `evidence.association.*` | exact Application |
| `drafting` | `deliverable.*` | exact Application |
| `review` | `review.*` | exact Application |
| `export` | `export.*` | exact Application |
| `recovery` | `workspace.check`, `workspace.backup.*`, `workspace.restore.*`, `workspace.repair.*` | Workspace; optional Application |

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

## Bounded correction and recovery

The CLI/MCP mutation facade confirms exactly the current `proposed` Requirement set, not every
historical decision. Omitted proposals or attempts to overwrite a confirmed/excluded entry fail
before a grant. Read the current set after every correction; an already completed decision does
not need another commit. Requirement confirmation previews include repository-derived downstream
invalidation when applicable.

`requirement.revise` preserves the Requirement identity and returns changed content to proposed;
unchanged content issues no mutation grant. After the proposed set is decided, `plan.propose`
can rebuild a stale Plan with its existing UUID and next revision, fresh Requirement inputs and
cleared confirmation. `plan.confirm` then confirms that draft while existing materials remain
stale. Rebuilding requires the same materialized kinds/counts; unsupported material-set changes
fail explicitly. Each Deliverable must be revised under the newly confirmed Plan, reviewed again
and exported to a fresh destination. Historical revisions and exports remain intact.

These are extensions of the existing preview/commit boundaries, with unchanged native consent,
expected revision, single-use token and privacy checks.

`source.revise.preview` / `.commit` are callable through MCP for a pasted-text Source exclusively
associated with the selected Application. The request binds its current Source reference, exact
Application revision, new text, and every existing Requirement using that Source. New text and
validated spans advance the Source, association and Application atomically; affected Requirements
return to proposed and dependent work becomes stale through the same repository calculation.
Old Source bytes, revisions and exports remain available. Identical text and interpretation return
`unchanged` without a grant. A changed interpretation of identical text uses `requirement.revise`.
Shared Sources, file/URL revisions, new Source imports and Requirement additions/removals remain
outside this bounded operation. There is no direct `source revise` CLI subcommand or Tauri binding;
the implemented surface inventory owns callable bindings.

## Generated schemas and examples

The source gate generates and verifies seven schemas under `schemas/agent/v4/`:

- `canisend.operation-registry/v4`;
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
