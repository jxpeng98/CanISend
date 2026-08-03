# Agent v3 and MCP contract

Status: implemented for the exact built-in generic Application Pack in GF4-AGENT-001.

## Scope

`canisend.agent/v3` is the canonical neutral Agent protocol. It uses Application, Requirement,
Plan, Deliverable, Pack, and Pack-qualified stage identities. Agent v2 remains a deprecated
compatibility adapter for the exact academic Pack and cannot mutate generic Applications.

CanISend is the durable state authority. Codex, Claude, or another host owns the conversation,
reasoning, and host tools. Neither protocol grants an Agent direct SQLite, Blob, `.canisend`,
projection, upload, or submission authority.

## Canonical MCP registry

| Operation | MCP tool | Class | Required boundary |
|---|---|---|---|
| `agent-v3.capabilities` | `canisend_agent_v3_capabilities` | Read | No Workspace body |
| `agent-v3.context` | `canisend_agent_v3_context` | Read | Body-free; optional Application ID |
| `application.list` | `canisend_applications_list` | Read | Body-free |
| `application.create` | `canisend_application_create` | Write | Host write approval; exact Pack fields and UTF-8 spans |
| `application.plan` | `canisend_application_plan` | Write | Host write approval and explicit user decision |
| `application.compose` | `canisend_application_compose` | Write | Host write approval and expected revision |
| `application.review` | `canisend_application_review` | Private read | Explicit private-read consent |
| `application.approve` | `canisend_application_approve` | Write | Host write approval, explicit user approval, single-use review token |
| `application.export` | `canisend_application_export` | Private write | Host write approval, private-export consent, safe relative path |

The advertised registry contains these nine v3 tools and the frozen thirteen-tool Agent v2
compatibility subset. Every advertised tool is classified exactly once as read-only or guarded
write. Host configuration requests approval for writes.

## Body-free context

Routine v3 context contains only:

- compiled product and protocol versions;
- Workspace v3 identity;
- exact Pack ID, version, and content digest;
- Application IDs, lifecycle, revisions, snapshot digests, counts, and stages;
- Deliverable IDs, kinds, states, revisions, and content digests;
- blockers and one or more exact next actions; and
- `submission_supported: false`.

It excludes source bodies, Requirement statements, opportunity/application metadata values,
Deliverable titles, and Deliverable bodies. A selected Application must belong to the Workspace
and match the exact verified generic Pack binding. Academic or mismatched Pack contexts fail with
stable remediation before mutation.

## Review and approval binding

Private review verifies current Deliverable Blobs and returns their bodies only after explicit
consent. It also creates an opaque token held only in the current MCP process. The token is bound
to:

- Application ID;
- expected Application revision; and
- exact Application snapshot SHA-256.

Approval requires the user to approve every returned body. The server reloads and compares the
current revision and digest before committing. A successful approval consumes the token. Replay,
wrong-operation tokens, eviction, restart, or missing tokens fail. A stale commit restores the
token only so the same failure remains diagnosable; recovery requires refreshed context, a new
private review, and a newly bound token.

## Actor and consent ownership

Agent-created intake and Agent-composed Deliverables record `ActorKind::HostAgent`. Requirement
confirmation, Plan decision, Deliverable approval, and private export remain user-owned actions
and record the existing user actor at the Store boundary. All writes retain optimistic revision
checks and exact Pack checks.

## Host handoff

Codex and Claude handoffs use the same body-free v3 context and MCP operation names. Their
bootstrap instructions preserve CanISend state authority, host session authority, exact Pack and
revision bindings, scoped consent, untrusted-input handling, and the no-upload/no-submission
boundary. The bundled `canisend-application` skill routes generic v3 first and enters Job-specific
skills only when the exact academic compatibility Pack requires them.

## Verification

The focused suite proves:

- empty-context creation guidance and ID-scoped resume;
- private source/metadata absence from routine context;
- Agent versus user audit actors;
- Codex and Claude neutral handoffs;
- exact MCP tool inventory and annotations over stdio;
- explicit Plan, private-read, approval, and export gates;
- review-token single use and cross-operation rejection;
- stale revision/digest failure, token restoration, refreshed review, and successful recovery;
- academic/generic Pack isolation; and
- `submission_supported: false` throughout the flow.
