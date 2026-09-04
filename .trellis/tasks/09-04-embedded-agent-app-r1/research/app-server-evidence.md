# Codex App Server evidence

Date checked: 2026-09-04

## Official contract

Source: https://learn.chatgpt.com/docs/app-server

- App Server is the supported boundary for product integrations that need authentication, history,
  approvals, and streamed events.
- The default stdio transport is newline-delimited JSON. Request, response, and notification
  envelopes omit the JSON-RPC `jsonrpc` field on the wire.
- A client initializes once per connection, sends `initialized`, then uses `thread/start` or
  `thread/resume`, `turn/start`, streamed `thread/*`/`turn/*`/`item/*` notifications, and
  `turn/interrupt`.
- `account/read` reports whether authentication is required. Managed ChatGPT authentication remains
  owned by Codex; CanISend must not collect provider credentials.

## Local verified surface

- `codex-cli 0.152.0` is installed and exposes `app-server --listen stdio://`.
- Its non-experimental generated schema requires `clientInfo` for `initialize`; `thread/resume`
  requires `threadId`; `turn/start` requires `threadId` and `input`; `turn/interrupt` requires
  `threadId` and `turnId`.
- The streamed text notification is `item/agentMessage/delta`; terminal turn state is delivered by
  `turn/completed`.
- Command/file approval responses support an explicit decline decision. Other server-request shapes
  are broader and remain outside this R1 slice.
- Generated schemas were inspected in a temporary directory and are not copied into the repository.

## Repository evidence

- `agent_runtime.rs` already owns executable discovery, version probing, bounded process I/O,
  provider consent, runtime errors, and the current one-shot Codex/Claude turn.
- `agent_session.rs` already owns private atomic App-local session metadata and body-free tests.
- Tauri 2.11.5 and `@tauri-apps/api` 2.11.1 already provide typed IPC Channel support.
- `AgentView.svelte` already has the conversation, consent, new-conversation, send, cancel, and
  runtime controls needed for the vertical slice.
- The operation registry already lists the three current runtime commands, so R1 needs only one new
  `start_agent_session` leaf.
