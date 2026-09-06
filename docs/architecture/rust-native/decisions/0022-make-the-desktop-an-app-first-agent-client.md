# ADR-RN-0022: Make the desktop an App-first Agent client

**Status:** Superseded in delivery priority by [ADR-RN-0023](0023-prioritize-cli-first-local-agent-workflows.md) on 2026-09-06; retained historical decisions and defensive constraints

**Date:** 2026-09-04

**Decision owner:** CanISend maintainer

**Clarified after R0/R1 review:** 2026-09-04; direction retained, execution and acceptance tightened.

## Context

CanISend already owns Workspace v4, Application revisions, evidence, consent, approvals, exports,
recovery, and audit through its Rust application facade. Agent v4 Skills and the persistent MCP
server let Codex and Claude Code use that authority, but the primary desktop flow still prepares an
external handoff or starts a process-per-turn CLI session. Users therefore leave the App for the
conversation and return to inspect product state, while the legacy Agent page still carries Job-era
scope state that a clean Workspace v4 must reject.

The desired product is a focused CanISend workbench, not a general IDE. Conversation should remain
inside the App while CanISend continues to own every product mutation and exact output. This change
must preserve the independently usable CLI and MCP surfaces, local-first privacy, rollback, and the
already-qualified exact-PDF path.

## Decision

### Desktop and protocol boundaries

The 1.0 embedded MVP keeps Rust, Tauri 2, Svelte 5, TypeScript, and Vite. It adds no Electron or
end-user Node.js runtime. The desktop uses two separate protocols:

```text
Svelte/Tauri Workbench -> installed codex app-server over stdio JSON-RPC
                       -> Workspace/Application-bound canisend MCP serve
                       -> canisend-app -> Store/Blob/revision/audit authority
```

- Codex App Server owns the embedded conversation session, streaming, interruption, and host
  permission requests.
- CanISend MCP owns structured product tools. It continues through `canisend-app` and cannot bypass
  consent, preview, approval, revision, verification, audit, or recovery.
- App Server and MCP remain distinct process streams. The Svelte frontend never proxies raw MCP or
  reads `.canisend`, SQLite, immutable Blobs, or source files directly.
- One App window owns at most one active Codex session in the MVP. Multi-session scheduling is
  deferred.

R1 must reuse the existing desktop executable discovery policy, run `codex --version`, and spawn
exactly `codex app-server --listen stdio://` without a shell. The embedded entry point is enabled
only for versions recorded in a checked-in compatibility record that pass the required
initialize/thread/turn/interrupt capability suite. Unknown or incompatible versions fail with an
actionable diagnostic and retain external handoff as rollback. CanISend does not download Codex or
infer support from executable presence alone.

**Authentication amendment approved by the owner: 2026-09-05.** The embedded App uses a dedicated
Codex configuration directory and a separate sign-in through Codex's supported ChatGPT login flow.
It does not reuse, copy or link the installed client's credentials or configuration. Codex owns
credential storage and refresh; CanISend never asks for, receives or persists an API key or ChatGPT
credential. Authentication-required state identifies this separate login. External handoff retains
the external client's existing configuration and sign-in. Separate configuration is necessary but
not sufficient for effective isolation: inherited host Skills and managed configuration still require
qualification before private tools are enabled. Protocol stdout is parsed as
bounded newline-delimited JSON-RPC; stderr is bounded and redacted before diagnostics. The
experimental WebSocket transport is outside the MVP.

### Effective tool and consent boundary

Before private embedded tools are enabled, R2 proves the actual provider process policy, including
inherited configuration/MCP/plugins/hooks and start/resume/turn consistency on the exact supported
version. A controlled working directory, read-only sandbox, or denial of approval requests alone
does not prevent already-permitted reads/commands. Missing isolation capabilities keep private
context and mutations disabled; authentication remains provider-owned without global config edits.

The selected Application is enforced across every tool handler, including no-ID enumeration and
Workspace-wide list results. Tool annotations do not classify data privacy. Allowlisted non-private
metadata may be automatic; private-read/provider-send consent, product preview/commit approval, and
private-export consent remain distinct. A model-supplied boolean or inherited automatic approver is
not the user's consent. The actual MCP approval request must be version-qualified and correlated to
its tool/item, Application, revision/digest, and current provider thread/turn. Reuse existing consent
and broker types; deny direct host permissions and revoke stale or scope-switched pending authority.

### Product experience

The primary desktop becomes a conversation-led Workbench:

- **Work** contains the active Workspace/Application switcher, central conversation and progress
  timeline, composer, session actions, and a contextual right Inspector.
- The Inspector owns Context, Evidence, Changes, Consent, History, and Preview. It becomes a drawer
  when the supported desktop width is too narrow.
- **Library** contains reusable sources, Profile/Evidence, opportunities, and completed
  Applications. **Settings** contains Workspaces, Agent readiness, privacy, appearance, and
  diagnostics.
- Work also exposes the existing Pack stages, readiness blockers, and next actions; Requirement-to-
  Evidence coverage and missing/stale associations; and selectable Deliverables with draft, review,
  validation, and export status. Explicit context selection shows private scope/provider destination.
- Guided analyze/draft/revise/check/export actions reuse existing facade/Skills operations. Supported
  manual inspection, review, and export remain usable without an available provider.
- Agent, Workflow, and Delivery stop being separate primary destinations after measured parity;
  their existing domain behavior is composed behind the Workbench rather than rewritten.

Conversation and reasoning stay provider-owned. CanISend persists only versioned resumable session
metadata and body-free correlation references in the MVP. It does not persist prompt, response,
tool-argument, evidence, generated-content, credential, or raw approval-token bodies in the session
registry or diagnostics. R3 loads bounded provider-owned history for the registered thread into
memory with stable ordering, deduplication, and Application scope checks. Unavailable/deleted provider
history is explicit; a cold restart cannot promise history without a working provider, and it does
not justify adding a local transcript database.

### Traceability and file history

Git remains the authority for source, ADRs, plans, tests, commits, tags, and release provenance. A
private user Workspace is not initialized as a Git repository, auto-committed, pushed, or made
dependent on an installed Git binary. Its authority remains SQLite, immutable content-addressed
Blobs, Application revisions, receipts, digests, approvals, and audit events.

The Inspector may expose that existing revision history and add exact append-only file snapshots,
but only for outputs written and registered by CanISend's managed projection/export pipeline. The
snapshot writer consumes the pipeline's successful output batch; it never recursively scans or
watches arbitrary Workspace files. Each entry binds path, type, byte size, SHA-256, and an existing
verified Blob. Comparison first reads manifests by path/digest and then loads only one selected
old/new Blob pair. Paths are logical outputs relative to their generation root; physical export
destinations remain in export records. Snapshots retain actual generator/build identity and same-kind
predecessors. Reuse only an identical current head with the full binding; A -> B -> A and a changed
generator retain provenance even when Blob bytes deduplicate. Repair/restore of newly snapshotted
outputs reads their verified retained Blobs, with older unsnapshotted recovery explicitly separate.

R2 may add `similar` 3.2.0 to `canisend-app` for bounded Myers line hunks after its lockfile source,
license, advisory, and maximum-input check is rerun. R0 records the candidate only; it adds no
unused dependency, Git subprocess, repository library, temporary comparison file, persisted patch,
or frontend diff engine.

Every committed Agent action must remain traceable through body-free identifiers:

```text
desktop session/turn -> provider thread/turn -> CanISend operation/receipt
                     -> preview digest + approval outcome
                     -> Application revision + audit event + output digest
```

The minimum committed origin is attached to canonical product audit identity through the existing
facade/Store, derived only from observed MCP items and verified typed receipts. It survives session
cache eviction/deletion and Workspace backup/restore. Transient read/denial correlation may remain
bounded session metadata. A post-commit origin-attachment failure reports the committed state and an
explicit trace gap without retrying the mutation. This adds no second product event stream.

### Shell and preview decision

Tauri remains the supported shell for the MVP. The current preview path displays the exact
validated/exported PDF bytes and already passed the native WKWebView, WebView2, and WebKitGTK
qualification. A renderer swap is not justified by a hypothetical future editor.

Electron is reconsidered only after a checked-in spike proves that required preview behavior cannot
meet exact-byte identity, accessibility, supported-platform consistency, and interaction targets in
Tauri, including a bounded PDF.js-in-current-WebView attempt where applicable. The decision must
also measure package size, startup/memory, CSP/IPC exposure, dependency/provenance, signing, and
release-matrix cost. Rich DOCX/HTML editing is not an MVP requirement.

### Extension path

R0 through R4 are Codex-only. After that MVP is qualified, R5 may implement a generic ACP v1 stdio
session channel and only then extract the lifecycle shared by two real providers. Claude Agent is
the first planned qualification target. ACP Registry presence is discovery evidence, not support or
installation authority; the first slice does not auto-install or update registry agents. Harness
and products without a qualified embedded-session interface remain MCP consumers or external
integrations.

## Compatibility and rollout

The clean Workspace v4 primary path never invokes legacy Agent, Job, Task, or Workflow operations.
Pre-v4 repositories continue to receive explicit fail-closed compatibility results and are never
silently reinterpreted or migrated. External handoff and the App-closed CLI/MCP journey remain
available throughout R0-R4 and are the rollback path if App Server readiness fails. Supported manual product
operations remain available. R2's additive Store migration keeps the existing future-schema refusal;
rolling back to an older binary requires a compatible backup rather than an unsafe database downgrade.

R0/R1 source completion does not qualify new App bytes. R2 is split into effective isolation/consent
(R2a) and durable history (R2b), followed by the complete R3 Workbench. R4 and M4-APP-005 qualify an
exact embedded Beta and its changed-journey user evidence before RC. Preserve public Beta.1 and its
historical observations. Prospectively update the existing machine cohort gate before its unchanged
thresholds can bind the embedded build; do not require two complete cohorts or silently pool builds. R5 remains post-MVP. R6 cleanup depends on R4, parity, and the rollback window, independently
of R5. The master roadmap owns execution and exact release authorization.

This ADR supersedes only the external-host-first product default and the former statement that the
desktop does not own an Agent client surface. ADR-RN-0015 remains authoritative for Tauri/Svelte,
ADR-RN-0019 for crate and adapter directions, and ADR-RN-0020 for Workspace v4, Agent v4, and
no-legacy compatibility.

## Consequences

- A normal Codex journey can eventually stay inside CanISend without making the provider the system
  of record.
- Session protocol changes stop at one Rust boundary; MCP and product contracts remain provider
  neutral.
- Exact local history remains available without Git, a second object database, or stored transcript
  bodies.
- Tauri, external handoff, and headless operation provide bounded rollback while the Workbench is
  introduced incrementally.
- Codex installation and a qualified compatible App Server version are explicit internal-MVP
  prerequisites.

## Rejected alternatives

- **Use ACP instead of App Server for the first Codex slice:** rejected because an extra adapter
  adds another version and failure boundary before the official Codex embedding path is proven.
- **Create a provider trait in R0:** rejected because one implementation cannot establish the useful
  common contract.
- **Make Git the Workspace history store:** rejected because it duplicates transactional product
  authority and mishandles private structured state and binary artifacts.
- **Switch to Electron now:** rejected because the exact-PDF requirement already passes in Tauri and
  no measured required capability currently justifies the runtime and release cost.
- **Persist full transcripts for traceability:** rejected because body-free operation, revision,
  approval, and digest correlation supplies auditability without duplicating private provider data.

## References

- [Codex App Server](https://learn.chatgpt.com/docs/app-server)
- [Codex authentication](https://learn.chatgpt.com/docs/auth)
- [Agent Client Protocol v1](https://agentclientprotocol.com/protocol/v1/overview)
- [Claude Agent SDK](https://code.claude.com/docs/en/agent-sdk/overview)
