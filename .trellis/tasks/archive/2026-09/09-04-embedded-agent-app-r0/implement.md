# R0 Implementation Plan

Status: Complete; source gate passed
Date: 2026-09-04

## Gate

- [x] Obtain explicit approval of the final converged parent plan: direct official Codex App Server
      for the MVP and generic ACP with Claude Agent first after MVP.
- [x] Start only this Trellis child and load the pre-development context/spec set.

## A — Reproduce and locate

- [x] Capture the exact clean Workspace v4 screen sequence and failing operation.
- [x] Trace all callers of `resolve_scope`, runtime catalog/run/cancel, and AgentView's mount/scope
      effects before editing.
- [x] Add the smallest failing cross-layer regression and retain the existing successful v4 handoff
      facade test as the base case.

## B — Fix once at the owning boundary

- [x] Separate clean-v4 Workspace/Application scope from the legacy job-scoped runtime path at the
      lowest shared boundary proven by the reproduction.
- [x] Do not swallow compatibility errors, fabricate a legacy job, or silently reinterpret a pre-v4
      repository.
- [x] Keep the patch bounded to preparation/runtime scope wiring; do not begin R1 session work or R3
      frontend redesign.

## C — Prove compatibility behavior

- [x] Prove clean v4 preparation completes without `Application::job_detail`, `agent_context`, or a
      v2 scope-catalog call.
- [x] Prove an explicit legacy-repository case retains its intended labelled result.
- [x] Run formatting and the smallest affected Rust and frontend checks.

## D — Reconcile authority

- [x] Add ADR-RN-0022 for App-first Codex App Server + CanISend MCP, conversation-led Workbench,
      Tauri retention, installed-Codex/auth policy, body-free traceability, Git boundary, managed-file
      boundary, post-MVP generic ACP/Claude qualification, and rollback.
- [x] Amend only the affected future direction in the authoritative 1.0 roadmap and related active
      architecture index/links required by repository policy.
- [x] Record the existing exact-PDF qualification and the objective PDF.js/Electron reconsideration
      gate.
- [x] Record `similar` qualification evidence only; do not add it to Cargo until R2 uses it.

## E — Close R0

- [x] Run `cargo run -p xtask --locked -- release check` once on the final R0 head after focused
      checks pass.
- [x] Record requirement-to-test evidence and the exact Git commit or PR head in this task.
- [x] Stop at the R0 exit criterion. Create R1 only after R0 is accepted.

## Exit criterion

Clean Workspace v4 preparation is usable, legacy behavior remains explicit, and accepted authorities
name the direct Codex App Server/MCP architecture. No App Server client, diff engine, or Workbench is
implemented in R0.

## Verification evidence

| Requirement | Fresh check | Result |
| --- | --- | --- |
| Screen clears a retained Job before catalog load | `pnpm --dir apps/canisend-desktop exec vitest run src/lib/agent-v4-screen.test.ts src/lib/bridge.commands.test.ts` | 30 passed |
| v4 runtime scope and explicit legacy scope | `cargo test -p canisend-gui` | 53 passed across the package targets |
| Existing v4 handoff remains body-free | `cargo test -p canisend-app clean_v4_workspace_prepares_agent_handoff_without_legacy_compatibility` | 1 focused test passed |
| Frontend type and behavior contracts | `pnpm --dir apps/canisend-desktop check` and `pnpm --dir apps/canisend-desktop test` | 0 diagnostics; 86 passed |
| Production frontend bundle | `pnpm --dir apps/canisend-desktop build` | Passed |
| Rust format and lint | `cargo fmt --all -- --check` and `cargo clippy -p canisend-gui --all-targets -- -D warnings` | Passed |

`cargo run -p xtask --locked -- release check` passed on
`b8a412e8b02aba35520191d508df0aaed2e49b81`. The implementation is
`d167cc0a25e0a4e10033b21e89a94f69e07482e2`; its exact feature-freeze record is
`ab12758417d980a6e2a517a98ba69985f20fb8a4`. Completion status is recorded by
`f09373a9e381dc402d1ebd5c2c7652c70b173f00` and its release-evidence exception
`ea8e7b2062727a1771f30e7899a6463c2ca4e147`. R1 remains unstarted.
