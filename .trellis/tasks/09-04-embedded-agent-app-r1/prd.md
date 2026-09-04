# R1 Codex App Server vertical slice

Status: In progress; implementation and focused verification complete, exact source gate pending
Date: 2026-09-04

## Goal

Replace the Codex process-per-turn path with one bounded, resumable, streamed Codex App Server
session inside the existing desktop Agent conversation panel. The slice proves transport,
readiness, continuity, cancellation, and recovery without adding CanISend product tools yet.

## Requirements

1. The desktop reuses the existing configured-executable discovery and `codex --version` probe,
   then starts exactly `codex app-server --listen stdio://` without a shell from an App-owned
   session directory outside the selected Workspace.
2. The client speaks newline-delimited App Server JSON-RPC, sends `initialize` once followed by
   `initialized`, reads authentication readiness with `account/read`, and fails with an actionable
   state when the executable, authentication, handshake, or required method is unavailable.
3. One Codex process is owned per desktop window. It starts or resumes one stored thread, starts one
   turn at a time, streams normalized events, interrupts the exact active turn, and shuts down the
   exact child on restart, protocol failure, or App exit.
4. Request IDs are correlated to exactly one response. Protocol lines, pending requests, stderr,
   startup, requests, turns, interruption, and shutdown are bounded. Stderr is redacted and never
   parsed as protocol input.
5. Every frontend event carries a desktop session ID, optional desktop turn ID, and monotonic
   sequence number. The frontend receives assistant text deltas and body-free status/correlation
   metadata, never raw approval arguments, provider tokens, or private diagnostic bodies.
6. During R1, the Codex thread is read-only and has no CanISend MCP injection. Command and file
   approval requests are surfaced as body-free events and declined automatically; unsupported
   server requests fail closed. Interactive host permissions and product mutations remain R2 work.
7. The existing App-local session registry migrates from v1 to v2 and persists only bounded
   resumable metadata: runtime/version, Workspace scope, desktop session/turn IDs, Codex thread/turn
   IDs, last status, and timestamps. Prompt and response bodies are never written by CanISend.
8. The existing Agent conversation panel shows readiness, streamed assistant text, cancel,
   new-conversation restart, and stored-thread resume. It remains keyboard operable and retains the
   existing English/Chinese and accessibility behavior.
9. Clean Workspace v4 stays Workspace-scoped and never performs a legacy Agent, Job, Task, or
   Workflow lookup. The existing external handoff and Claude one-shot path remain rollback
   fallbacks; Codex must no longer use its one-shot path.
10. Offline tests use a deterministic fake App Server process. CI does not require a Codex login,
    provider network access, or user credentials.

## Non-goals

- CanISend MCP tool injection, product mutation approval, or direct Workspace access (R2).
- The full Workbench/Inspector redesign (R3), rich preview, file history/diff, ACP, or Claude App
  Server support.
- WebSocket transport, multi-session scheduling, transcript persistence, a provider abstraction,
  a protocol crate, Electron, PDF.js, or a new runtime dependency.

## Acceptance criteria

- [ ] A fake server proves initialize, readiness, new thread, resume, multiple deltas, completion,
      cancellation, child exit, malformed/oversized framing, timeout, and fail-closed approval.
- [ ] A local signed-in Codex smoke streams one response in the App, cancels one turn, restarts one
      conversation, resumes it after process restart, and records only body-free registry metadata.
- [ ] `agent_runtime_catalog` reports missing, authentication-required, connecting, ready, running,
      cancelling, recoverable-disconnect, incompatible, and failed states without exposing account
      or token data.
- [ ] The Tauri boundary has one start/load command, the existing catalog as readiness/status, one
      streamed turn command, and the existing exact-scope cancel command.
- [ ] A clean Workspace v4 regression proves the embedded path never reaches legacy Job resolution.
- [ ] Registry migration reads a v1 fixture, writes canonical v2, preserves the resumable provider
      thread ID, and contains no prompt, response, transcript, approval token, or evidence body.
- [ ] Focused Rust and frontend checks pass, followed by the required final source gate on the final
      implementation head.

## Product decisions carried forward

- Tauri 2, Rust, Svelte 5, and the current Agent panel remain the implementation surface.
- Codex App Server is the session transport; MCP remains the later CanISend tool boundary.
- The tested development baseline is `codex-cli 0.152.0`; compatibility is proven by handshake and
  required-method behavior rather than an invented version range.
