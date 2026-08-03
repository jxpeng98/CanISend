# GF4-AGENT-001 — Generic Agent v3 and MCP implementation

Date: 2026-08-03

## Outcome

CanISend now exposes the canonical neutral `canisend.agent/v3` context and nine generic MCP
operations for the exact built-in generic Application Pack. Codex and Claude handoffs can start a
new Application, resume by Application ID, confirm a user-approved Plan, compose Deliverables,
perform consented private review, approve the exact reviewed snapshot, recover from a stale
review, and export locally without academic nouns or submission authority.

The thirteen Agent v2 MCP tools remain available as a separately named compatibility subset. The
combined server registry is deterministic and every tool remains classified once as read-only or
guarded write.

## Implementation

- Added body-free v3 capability, context, Application, Deliverable, blocker, operation, and host
  handoff read models in `canisend-app`.
- Added exact generic-Pack admission for context and every mutation. Migrated academic and current
  academic Workspaces fail closed with remediation.
- Added Agent-aware Store entry points so Agent creation and composition record `HostAgent`, while
  Requirement/Plan confirmation and final approval remain user decisions.
- Registered v3 capabilities, context, list, create, plan, compose, review, approve, and export MCP
  tools without changing the Agent v2 tool semantics.
- Bound approval to a session-local single-use token containing the exact Application revision and
  snapshot digest returned by private review. Failed stale commits restore the token; recovery
  still requires a fresh review and token.
- Updated Codex, Claude, generic-host, and `canisend-application` assets to prefer neutral v3 nouns
  and route to Job-specific skills only for the exact academic compatibility Pack.

## Defensive invariants

- Routine context never serializes source bodies, Requirement statements, metadata values,
  Deliverable titles, or Deliverable bodies.
- Private review and export require explicit scoped consent.
- Plan confirmation and approval require explicit user authorization in addition to host write
  approval.
- Stale revision, changed snapshot digest, replayed token, wrong token type, wrong Pack, unsafe
  destination, and missing consent fail without the requested mutation.
- CanISend never uploads or submits an Application.

## Verification evidence

- `cargo test -p canisend-app -p canisend-mcp -p canisend-resources --locked`
  - app: 90 passed, 1 ignored;
  - MCP: 3 passed;
  - resource manifest/host assets: 12 passed.
- `cargo test -p canisend-cli --test mcp_protocol --locked`
  - 5 passed, including deterministic tool annotations and body-free Agent v3 stdio context.
- Focused MCP lifecycle covers new, resume, denied decision/no mutation, compose, denied private
  read, review, stale concurrent commit, restored stale token, refreshed review, approval, replay
  rejection, and export next-action recovery.

The macOS linker continues to emit the known `__eh_frame` compact-unwind size warning in debug
test binaries; tests complete successfully.

## Remaining boundary

GF4-EXAMPLE-001 still owns the four offline synthetic scenario families. GF5 owns the canonical
cross-surface operation registry and semantic parity matrix. Native dual-Pack qualification,
governance linkage, and independent committed-evidence inspection remain required before the
roadmap can mark the item Verified or qualify the first usable Alpha.
