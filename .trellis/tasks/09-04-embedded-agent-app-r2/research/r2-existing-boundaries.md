# R2 existing-boundary evidence

Checked: 2026-09-04 against `d7c0abfea46aa8d7b9f63d9b1ac7bd7fac93abea` and the pending R2 plan.
This is a source/schema review, not new product implementation or release qualification.

## Reuse and gaps

| Existing owner | Reuse | Gap the R2 plan must close |
| --- | --- | --- |
| `crates/canisend-desktop/src/agent_runtime.rs` | Persistent R1 process, bounded JSONL, discovery, start/resume/stream/cancel | `cwd`, read-only sandbox, and denied approval requests do not prove restricted reads or prevent inherited tools/configuration |
| `crates/canisend-app/src/agent.rs` | Existing stdio MCP configuration and provider consent | `writes` approval mode does not cover private bodies returned by read-only tools |
| `crates/canisend-mcp/src/lib.rs` | Current 36-tool catalog, consent types, single-use revision/digest broker | `application.show`, profile/evidence association lists bypass `parse_application_id`; no-ID lists need output scoping; caller-provided consent booleans are not host user grants |
| `crates/canisend-app/src/agent_session.rs` | Private atomic registry v2, v1 migration, body-free metadata | A bounded cache cannot be the durable owner of committed action provenance |
| `crates/canisend-store/src/application_v3.rs` | Immutable Application revision metadata and canonical audit rows | Expose/resolve exact audit identity from verified receipts; do not join invented IDs or approximate timestamps |
| `crates/canisend-store/src/application_projection_v3.rs` | Generated in-memory batch, pending transaction, repairable publication | Current repair/restore regenerates bytes; new exact snapshots must restore retained verified Blobs |
| `crates/canisend-store/src/application_flow_v3.rs` | Rendered batch, create-new export helper, audit transaction | Export paths include destination; use generation-relative logical paths for history and preserve physical location in export records |
| `crates/canisend-store/src/blob.rs` | Content-addressed verified bytes and `blob_references` for audit/backup | Retain historical output references; Blob deduplication must not collapse generator identity or A -> B -> A predecessor history |

`deliverable.audit` and `review.inspect` return private content despite read-only annotations. Review
all tools' actual inputs/results, including `application.list` returning full stored Application
models and source/association listing metadata. One parser edit cannot establish the complete bound
server policy.

The parent plan requires committed origins to remain traceable, while the old R2 design proposed
oldest-first registry eviction. Keep the cache bounded and attach minimum committed origin to the
existing product audit authority. Report a post-commit trace gap if attachment fails; do not repeat
the product write.

R1 resumes with `excludeTurns: true` and the frontend keeps `messages` in memory. Conversation
rehydration is therefore explicit R3 work, not already implemented R1 behavior. Fetch bounded known-
thread history from the provider, normalize/deduplicate in memory, and show unavailable history honestly.

## Provider evidence boundary

- The installed version inspected was `codex-cli 0.152.0`.
- Its generated schemas expose start/resume configuration, MCP call items, user-input/elicitation,
  and host permission requests. Configuration exposes required servers, tool allow/deny lists,
  timeouts, and approval modes. Schema presence is not evidence that the complete consent path works.
- The MCP user-input path is experimental. R2a must prove its actual safely correlated request,
  explicit user reviewer, per-tool prompting, and effective policy before enabling private tools.
- Newer documented restricted-read or history fields must not be assumed to exist in the installed
  version. Qualify the exact capability/version first; no silent fallback to broader authority.
- R0 qualified the `similar` 3.2.0 candidate without adding it. R2b adds/locks it only with actual
  comparison code and rechecks the resulting source/license/advisory and bounded-input behavior.

Official references consulted during review:

- [Codex App Server](https://learn.chatgpt.com/docs/app-server)
- [Codex configuration reference](https://learn.chatgpt.com/zh-Hans/docs/config-file/config-reference)
- [Codex MCP configuration source](https://github.com/openai/codex/blob/main/codex-rs/config/src/types.rs)
- [Codex MCP approval implementation](https://github.com/openai/codex/blob/main/codex-rs/core/src/mcp_tool_call.rs)

## Minimal implementation consequence

Extend existing runtime, consent/broker, MCP, revision/audit, projection/export, and Blob owners.
R2a owns effective policy and every tool path; R2b owns durable origin and exact snapshots/comparison.
Keep one Store migration owner and the existing desktop components. No new provider trait, Git,
Electron, transcript database, Workspace watcher, event store, or diff editor is needed.
