# R0 App-first architecture and Workspace v4 blocker

Status: Implemented; final Git/source-gate evidence pending
Date: 2026-09-04

## Goal

Make the approved App-first direction authoritative using the official Codex App Server session
boundary and remove the clean Workspace v4 legacy compatibility blocker without weakening explicit
legacy-repository behavior.

## Requirements

- Amend or supersede only the external-host-first portions of accepted architecture and roadmap
  authorities. Retain Rust, Tauri 2, Svelte 5, the `canisend-app` facade, MCP product tools, external
  handoff fallback, and exact-byte PDF preview.
- Record the approved extension path: R0-R4 remain Codex-only; after MVP, R5 may add a generic ACP v1
  stdio channel with Claude Agent as its first qualification target. Do not implement or scaffold
  that channel in R0.
- Define the MVP Codex boundary as the installed `codex app-server --listen stdio://` interface.
  Record executable discovery, supported-version and capability checks, existing Codex sign-in reuse,
  redacted diagnostics, and rollback. Do not add `codex-acp`, an auto-downloader, or a one-provider
  abstraction.
- Trace **Prepare AI Workspace** from Svelte through the typed bridge, Tauri command, and App facade.
  A clean Workspace v4 path must not pass a legacy `selected_job_id` into the shared runtime scope
  resolver or call `Application::job_detail`, `agent_context`, or another retired v2 surface.
- Preserve a clearly labelled compatibility path for pre-v4 repositories and keep it fail-closed.
- Record that Git owns source and release provenance, while private Workspace content remains under
  CanISend's revision, digest, receipt, Blob, and audit authority.
- Record that future file snapshots cover only CanISend-managed projection/export outputs. R0 may
  record `similar` 3.2.0 qualification evidence but must not add the unused dependency before R2.
- Keep Electron and PDF.js behind the existing objective preview gate.

## Acceptance Criteria

- [x] Accepted ADR and master-roadmap text describe App-first Codex App Server + MCP, the
      conversation-centered Workbench, the Tauri decision, Git boundary, managed-file boundary, and
      post-MVP generic ACP/Claude qualification path without contradicting current machine authority.
- [x] A clean Workspace v4 completes **Prepare AI Workspace** without the legacy compatibility error
      and without invoking any Workspace v2 scope-catalog or job-detail operation.
- [x] An explicit pre-v4 repository case retains its intended compatibility result and does not enter
      the clean-v4 path silently.
- [x] The smallest owning Rust regression plus the typed bridge/screen regression fail before and
      pass after the root-cause fix.
- [x] The existing exact-PDF preview remains unchanged and its prior native qualification is cited;
      no Electron migration or PDF.js dependency is introduced.
- [x] R0 adds no App Server client, provider abstraction, diff dependency, Workspace Git repository,
      or broad frontend redesign.

## Notes

- The reproduced cause was a retained `agentUiState.selectedJobId`: when the current global Job
  selection became empty, `AgentView` did not clear it before loading the runtime catalog. The
  shared desktop `resolve_scope` then invoked legacy `Application::job_detail` for Workspace v4.
- The owner explicitly approved the final converged parent plan and R0 implementation on 2026-09-04.
