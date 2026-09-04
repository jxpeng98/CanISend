# R0 Technical Design

Status: Implemented; final Git/source-gate evidence pending
Date: 2026-09-04

## Scope

R0 changes two things only: accepted architecture/release authority and the clean Workspace v4
**Prepare AI Workspace** failure. It does not implement the App Server client, Workbench, file
history, Electron, or a new provider abstraction.

## Authority change

Add ADR-RN-0022 as the narrow superseding decision for the App-first flow. It keeps ADR-RN-0019's
crate graph and ADR-RN-0020's Workspace v4/application-facade invariants, but replaces the
external-host-first product default from the implemented Stage 4G plan with:

```text
Svelte/Tauri Workbench -> installed codex app-server over stdio JSON-RPC
                       -> Workspace/Application-bound canisend MCP serve
                       -> canisend-app -> Store/Blob/revision/audit authority
```

The ADR must also preserve external handoff as rollback, retain Tauri pending the preview gate, and
separate source Git provenance from private Workspace content history. Reconcile only the affected
future direction in the master 1.0 roadmap; do not rewrite Stage 4G's historical implemented facts.
It records generic ACP with Claude Agent as a post-MVP R5 extension, but deliberately defines no
provider trait or ACP implementation during R0-R4.

## Workspace v4 failure chain

The v4 handoff facade is already correctly Workspace-scoped. The current desktop Agent surface also
loads the old runtime catalog with `agentUiState.selectedJobId`. Every runtime catalog/run/cancel
path enters `resolve_scope`; any present ID invokes legacy `Application::job_detail`, which
intentionally rejects Workspace v4.

The screen reproduction confirmed that `AgentView` retained `agentUiState.selectedJobId` when the
current global selection became empty, then used that stale value during its mounted runtime-catalog
load. The bounded correction clears retained state when that external scope changes and the shared
resolver independently detects Workspace v4 before any legacy Job lookup. Explicitly detected v2/v3
repositories retain their labelled legacy path; other open/status failures are returned unchanged.

## Safety invariants

- Clean v4 never calls legacy Agent, Job, Task, or Workflow operations.
- A pre-v4 repository never masquerades as v4 and retains its explicitly labelled result.
- Adapters continue through `canisend-app`; no direct SQLite, `.canisend`, or Blob access is added.
- The preparation path remains body-free and does not contact a provider.
- No source change initializes or writes a `.git` directory inside a user Workspace.

## Verification shape

Use one smallest Rust test at the shared owning boundary, one typed bridge/screen test for the
reported sequence, and one explicit legacy-repository case. Existing App-facade handoff coverage
remains the lower-layer base case. Documentation-only authority edits do not trigger unrelated test
suites; the final R0 head runs the source gate because accepted ADR/master-roadmap authority changes.

## Rollback

Revert ADR/master-roadmap reconciliation and the bounded blocker fix together. The existing external
handoff and one-shot runtime remain available, and no Workspace migration is introduced by R0.
