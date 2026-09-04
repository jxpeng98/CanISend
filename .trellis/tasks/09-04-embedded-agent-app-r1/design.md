# R1 design

Status: Implemented; exact source gate pending

## Smallest viable shape

Keep the implementation in `crates/canisend-desktop/src/agent_runtime.rs` and reuse its executable
discovery, process limits, registry access, errors, and Tauri commands. Use `std::process`,
`std::thread`, `std::sync`, `serde_json`, and Tauri's installed `ipc::Channel`; add no dependency,
provider trait, protocol crate, or Electron work.

The existing `AgentRuntimeState` becomes the owner of Codex connections keyed by Tauri window label.
Each connection owns the exact child, a locked stdin writer, a stdout reader, a separate bounded
stderr reader, pending response senders keyed by numeric request ID, one active turn sink, and atomic
request/event counters. A `Drop`/shutdown path closes stdin, terminates, and waits for that child.

## Tauri contract

| Command | Change |
| --- | --- |
| `agent_runtime_catalog` | Extend the existing command with the current Codex readiness/session snapshot. |
| `start_agent_session` | New command; validate scope/consent, initialize App Server, and start or resume a thread. |
| `run_agent_turn` | Keep the existing request/result and add one `Channel<AgentStreamEvent>` argument; Codex uses the persistent connection, Claude keeps the fallback. |
| `cancel_agent_turn` | Keep the existing request/result; Codex sends `turn/interrupt`, then uses bounded exact-child termination only if needed. |

`AgentStreamEvent` is one normalized, tagged Rust/TypeScript union. Every variant includes
`sequence`, `desktop_session_id`, and optional `desktop_turn_id`; variants cover status,
assistant-text delta, body-free server-request notice, completion, interruption, and failure. Raw
provider JSON does not cross into JavaScript.

## Protocol lifecycle

1. Resolve and version-probe Codex; create an App-owned session directory.
2. Spawn `codex app-server --listen stdio://` with piped stdio and no shell.
3. Send `initialize` with CanISend client name/version and non-experimental capabilities, await one
   bounded response, then send `initialized`.
4. Send `account/read`. Treat `requiresOpenaiAuth && account == null` as authentication required;
   retain no account fields.
5. If canonical v2 metadata contains a Codex thread ID and restart was not requested, call
   `thread/resume` with `excludeTurns: true`; otherwise call `thread/start` with the controlled cwd,
   `sandbox: "read-only"`, and `approvalPolicy: "never"`.
6. Send `turn/start` with one text input. Route `item/agentMessage/delta` to the Tauri channel and
   finish only on the matching `turn/completed` or a terminal protocol/process error.
7. Send `turn/interrupt` with the exact thread/turn IDs when cancelled. On restart or shutdown,
   close the connection and wait for the exact child.

Wire objects omit the `jsonrpc` field, matching the official App Server transport. Unknown optional
notifications are ignored with a redacted diagnostic. Invalid JSON, missing required correlation,
duplicate responses, or an oversized line terminates only the affected connection. Known command
and file approval requests emit a body-free notice and receive `decline`; every other server request
receives a protocol error and leaves the session non-ready.

## Bounds

- Prompt: retain the current 16 KiB limit.
- One protocol line: 1 MiB; pending requests: 16.
- Retained stderr: retain the current 256 KiB cap and redact Workspace/user-home paths.
- Startup: 10 seconds; ordinary request: 30 seconds; turn: retain 10 minutes.
- Interrupt: 5 seconds; graceful shutdown: 2 seconds before exact-child kill/wait.

These constants live beside the existing runtime limits and each non-trivial limit has one focused
positive/negative regression.

## Registry v2

`canisend.agent-session-registry/v2` retains the current scope and external session ID, and adds
optional bounded fields for desktop session/turn ID, external turn ID, provider version, and a
finite last status. Loading v1 maps its external session ID to the resumable thread ID and leaves new
fields empty; the next successful session mutation writes v2 atomically with private permissions.
No receipt-reference field is added until R2 has a real receipt to reference.

## UI flow

The existing Svelte Agent panel remains. On send it appends the user message and one empty assistant
message, then appends text deltas to that message. Status events drive an `aria-live` status/badge;
completion reconciles metadata without duplicating the final text. Cancel retains partial text.
"New conversation" closes the current Codex connection and starts a new thread on the next send;
normal entry resumes stored metadata. Conversation bodies stay in the existing in-memory UI state.

## Failure and rollback

Missing/incompatible Codex or authentication-required states keep external handoff available. A
failed connection is discarded before retry; it is never silently reused. The old Codex one-shot
branch remains in source only until this slice passes, then becomes unreachable for Codex; Claude is
unchanged. Registry v2 is App-local, so rolling back embedded mode does not migrate Workspace data.

## Test fixture

Add one feature-gated fake App Server binary used only by focused tests. It accepts the production
`app-server --listen stdio://` arguments and selects deterministic scenarios from its test working
directory. Tests cover real child lifecycle without a shell, login, or network. Generated Codex
schemas are development evidence only and are not vendored.
