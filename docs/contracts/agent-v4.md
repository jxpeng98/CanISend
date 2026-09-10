# Agent v4 task and resource contract

**Protocol:** `canisend.agent/v4`

**Workspace:** `canisend.workspace/v4`

**Schema version:** `4.0.0`

**Canonical task model resource:** `agent.v4.task-resource-model`

## Boundary

Agent v4 is the current CLI Host workflow. It is a clean protocol, not a
compatibility adapter for earlier Skills, host layouts, job aliases, Agent v2/v3 messages, or
Workspace v2/v3 state. An unsupported protocol, Workspace, unknown field, legacy operation ID, or
incomplete context fails before an application-facade mutation is attempted.

CanISend remains the state authority. Codex, Claude Code, and other MCP clients own conversation
and reasoning state but never write SQLite, Blobs, projections, or `.canisend` paths directly.
MCP is the preferred structured transport; the standalone native CLI is a semantic equivalent and
does not require the desktop App to be open.

### MCP confirmation requests

MCP commit inputs use `request_confirmation`, not a model-supplied approval assertion. True
requests user authorization; false consumes the selected preview without committing and revokes
any Auto approval grant. By default, each guarded request needs an actual native form `accept`
with `{"confirm":true}`. Decline, cancel, false, unsupported form capability, malformed responses
and timeout fail closed. Never answer a form for the user.

For routine work, the form also offers an optional, unchecked `auto_approve` boolean. Only the
user's accepted `{"confirm":true,"auto_approve":true}` form response establishes standing
permission. It covers one canonical Workspace path/UUID, Application UUID and exact Pack
ID/version/digest in this MCP connection for up to 60 minutes. Switching scope, rejecting a form,
explicit cancellation, expiry or reconnecting clears it. Invalid or stale previews fail without
clearing an otherwise valid grant. Model tool inputs
cannot enable it; unknown response fields and non-boolean values remain invalid.

The repository allowlist covers Application-private reads, Requirement extraction/revision/
confirmation, exclusive pasted Source revision, Plan proposal/confirmation, drafting/revision
and review disposition. Source operations qualify only for exact Source references already used
by current Requirements. A Profile/Evidence form can additionally grant one exact Profile Source
ID/revision/digest. Its private reads, source-backed Evidence confirmation and Profile/Evidence
associations for this Application then reuse that grant, including facts newly confirmed from
the same Source. Evidence provenance is resolved from its immutable catalog and digest, never
from a model-supplied origin. Adding another Source requires explicit opt-in and does not extend
the active grant's expiry. Ungranted Sources, exports and unrecognized operations still ask.
The grant does not authorize external Host tools, network access or submission.

`request_private_read` and `request_private_export` still request separate consent scopes; neither
flag grants permission. One form lists all requested permissions for the exact operation, so a
commit that needs private access and a content change asks once. An applicable standing grant
can satisfy routine private reads and mutations. Private export remains individually authorized.
Every mutation still needs its exact current
preview, revision, digest and single-use token; standing permission does not bypass validation.

Application-scoped tool results expose body-free `_meta["canisend/approval"]`: `mode` (`ask` or
`auto`), `automatic` (whether this call used standing permission), `scope`, `profile_sources` (the
explicitly granted Source references), and `remaining_seconds`.
This connection metadata is not persisted in business receipts. Existing `user` confirmation
fields identify the user's authorization, including delegation; they do not prove individual
human inspection of each automatic decision. Hosts should preserve that distinction in reports.

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
`canisend_evidence_confirm_commit` requests authorization of that exact catalog and its required
private read in one form, or uses the Source grant. It creates Workspace Evidence without changing
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
ID, task ID, generation, and candidate digest. `request_private_read` requests scoped authorization
before reading the payload. The existing draft validator then prepares `local-task.draft.preview`;
this does not approve the candidate or change the Application. Use the returned token and digest
with the existing `canisend_deliverable_draft_commit`, using individual or active Auto approval.
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

Native confirmation messages render the Broker-owned preview as labeled text with real
paragraph breaks, placing proposed changes before reference details. This presentation does
not alter request values, digests, single-use tokens or the machine-readable tool result.
Hosts present result summaries and requested document text as readable prose or Markdown;
raw envelopes and escaped JSON strings are not the default user-facing response.

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

### Instruction ownership

Five stable Skill entrypoints share one operating contract in `canisend-workspace`.
Stage Skills link to that installed sibling and own only their task-specific behavior;
the application-workflow Skill routes end-to-end work without creating another state
model. Rediscover schemas after connection/version changes and refresh affected state
after commits. Guide/Skill resource revisions are independent of the v4 wire schema.

The older `prompts/*` resources remain bound into the academic Pack for declared v2
artifact tasks. They are not included in Agent v4 Host packs and must not supply v4
candidate shapes. Their bytes and Pack history remain unchanged by instruction edits.
No standalone Codex/Claude plugin is shipped: distribution consists of the native MCP
server, managed Skills and optional exported Host packs. Tauri runtime plugins belong
to the deferred desktop surface, not Agent instruction discovery.
