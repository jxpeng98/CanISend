# Embedded Agent App Roadmap

> 2026-09-06 closeout: App-first execution is superseded by CLI-first delivery under
> [ADR-RN-0023](../../../docs/architecture/rust-native/decisions/0023-prioritize-cli-first-local-agent-workflows.md). R0/R1 source completion is retained; R2 is partial
> and unaccepted; unfinished R2-R6 scope is deferred. Historical checklists below are not the
> active queue. The master roadmap owns the next CLI-first slice.

Status: In progress — supporting; R0/R1 source complete; R2a bound-server source implemented, provider/consent gate pending
Date: 2026-09-04

## Goal

Make the CanISend desktop App the primary surface for agent-assisted work by embedding a safe,
streaming Codex client, while keeping the Rust core as the local system of record and preserving
evidence, consent, review, export, recovery, and audit invariants. A user must also be able to compare
the exact files produced by the current and a retained earlier build.

R0 removed the clean Workspace v4 failure in source; its original acceptance target was:
`Legacy Agent, Job, Task, and Workflow compatibility is not supported in Workspace v4`.

## Problem

At the original planning baseline the App started a provider process per turn or handed work to an
external Codex/Claude host. R1 now implements persistent App Server transport with recorded local
stream/resume/cancel evidence. It does not yet prove private-tool isolation, safe MCP consent, or
rehydration of prior conversation items in the UI. The remaining product journey still needs the guarded tool loop and composed Workbench before
users can complete their work inside the App. R0's clean-v4 source correction and R1 transport must
not be treated as qualification of a new released App.

The frontend also exposes product subsystems as separate large pages. This makes the user navigate
between Agent, Applications, Workflow, Delivery, Workspaces, and supporting records when the actual
job is a single loop: ask, inspect, approve, preview, and export.

Content preview is a real requirement, but the current exact PDF path already returns verified bytes
to an in-App iframe and has passed native qualification. A desktop-shell migration is therefore not
an MVP prerequisite.

Record history alone does not satisfy file traceability. The current projection manifest represents
the latest file at a path, and regenerating an old revision with newer code does not prove the bytes
that were produced at the time. Exact file snapshots and file-to-file comparison are therefore part
of the revised requirement.

## Product decisions

1. The App is the default user experience; Codex remains an underlying agent engine.
2. Codex App Server's official stdio JSON-RPC interface is the MVP session boundary. MCP remains the
   tool boundary into CanISend.
3. Codex is the only MVP provider. After implementation phase R4, post-MVP phase R5 adds one generic
   ACP v1 stdio channel and uses a qualified Claude Agent adapter as its first live acceptance target.
   Other ACP agents are supported only after individual capability, distribution, authentication,
   permission, and provenance checks.
4. Tauri remains the MVP shell. Electron is reconsidered only if a bounded Tauri preview prototype
   fails objective acceptance criteria that PDF.js or a system-viewer fallback cannot meet.
5. The product owner confirmed the conversation-led Workbench on 2026-09-04. Conversation stays in
   the center and the right Inspector owns context, evidence, changes, consent, history, and preview.
   The redesign reuses the existing Svelte, shadcn-svelte, and semantic-token system.
6. Existing external handoff remains available as a fallback, not the primary flow.
7. Rust, Tauri 2, Svelte 5, TypeScript, Vite, and the existing SQLite-backed core remain the supported
   architecture. The installed Codex CLI's App Server is the only new MVP runtime boundary.
8. Git remains the source, planning, test, and release provenance system. A private CanISend
   Workspace remains a local product data directory, not a Git repository; its content authority is
   the existing revision, digest, receipt, dependency, and audit model.
9. File history covers only files written and registered by CanISend's managed projection/export
   pipeline. Arbitrary user-managed Workspace files are not scanned, watched, or snapshotted.
10. The MVP uses an in-process Rust diff engine to present Git-style line hunks from verified Blobs.
    It does not invoke system Git, embed a Git repository implementation, or materialize temporary
    comparison files.

## Requirements

Requirement labels R1-R10 below are product requirements, not implementation phases R0-R6. The
[execution checklist](implement.md) maps them to the master roadmap's M4-APP-001–005 units.

### R1 — Clean Workspace v4

- Preparing an AI Workspace from a clean Workspace v4 must use only Workspace v4 Application and
  Pack capabilities.
- The primary App path must not request legacy Agent, Job, Task, or Workflow scope catalogs.
- Existing legacy repositories may retain an explicitly labelled migration path outside the clean
  v4 flow.

### R2 — Embedded Codex session

- The App must initialize one `codex app-server --listen stdio://` process, complete the official
  initialization handshake, start or resume a thread, start a turn, stream notifications and server
  requests, interrupt an active turn, and shut down cleanly.
- The Codex executable path, version, App Server capability, and authentication readiness are
  explicit and diagnosable. The embedded App uses a dedicated Codex configuration and separate
  provider-owned login; it never collects or stores provider tokens.
- A deterministic fake App Server owns protocol tests; CI must not require a provider login or
  network access.
- Experimental WebSocket transport is out of scope.

### R3 — CanISend tool boundary

- The Codex session receives the existing CanISend MCP server configuration bound to the selected
  Workspace and Application.
- The agent process runs from a controlled session directory, not the raw Workspace directory.
- Effective host isolation is proven before initialization and on start/resume/turn: unrelated
  MCP/plugins/hooks/configuration and direct model reads/commands cannot gain authority. A working
  directory or read-only sandbox alone is insufficient; missing capabilities keep private tools off.
- Every tool ID path and no-ID/list response respects the selected Application. Read-only labels
  cannot classify private bodies as safe metadata.
- Private-read/provider-send consent, mutation approval, and private-export consent are separately
  bound to user-selected scope. A model boolean, arbitrary user-input question, or inherited automatic
  approver does not establish consent. Direct host-permission requests are labelled and denied.
- Product mutations retain the existing preview, approve, commit, and verify lifecycle.
- Codex host permissions and CanISend mutation approval are visually and semantically distinct.

### R4 — Workbench UI

- The active Application opens in one Workbench containing a conversation timeline, composer,
  provider/session status, progress, and a collapsible Inspector.
- The Inspector presents current context, evidence, proposed changes, validation, consent, local
  revision history, and exact output preview without requiring navigation to a separate Agent page.
- Users can start, resume, cancel, retry, reject, approve, and open the external-agent fallback from
  this surface.
- The primary navigation is reduced to Work, Library, and Settings.
- Reuse pack-defined stages/readiness/next actions, Requirement-to-Evidence coverage and stale/missing
  associations, and selectable Deliverables with draft/review/validation/export status.
- Guided analyze/draft/revise/check/export actions use existing operations and product Skills. Context
  selection shows the exact private scope and destination before provider transmission.
- Supported manual inspection, review, and export remain available when the provider is unavailable.
- The UI preserves keyboard operation, visible focus, reduced motion, text scaling, English/Chinese,
  light/dark themes, and compact/comfortable density.

### R5 — Preview

- Text, Markdown, structured records, and proposed changes use native Svelte rendering.
- Final PDF preview uses the exact verified bytes that export would write, with digest visibility and
  a system-viewer fallback.
- Rich PDF navigation may use PDF.js inside Tauri only after user testing demonstrates a gap.
- Preview content must not gain Node.js or unrestricted native capabilities.

### R6 — Recovery and diagnostics

- The App stores only versioned resumable metadata: Codex version, desktop session/turn identifiers,
  Codex thread/turn identifiers, selected Workspace/Application, timestamps/status, and body-free
  receipt references. Transcript bodies stay Codex-owned in the MVP.
- Process exit, malformed output, unsupported protocol versions, oversized messages, stale sessions,
  and App restart must lead to bounded recovery or a clear actionable error.
- Reopen/resume retrieves bounded known-thread provider history into memory with stable ordering and
  deduplication. If provider history is unavailable, show that gap and keep product state usable;
  do not imply empty local messages prove there was no earlier conversation.
- Logs must exclude prompt, evidence, credential, and generated-content bodies by default.

### R7 — Migration safety

- The Workbench is introduced alongside the current shell and reuses the existing Tauri facade and
  domain operations.
- Old views and the one-shot runtime are removed only after the replacement has parity and focused
  regressions.
- No feature may bypass `canisend-app` or read `.canisend`, SQLite, or blob storage directly.

### R8 — Sustainability and traceability

- Codex-specific behavior must end at the Rust session boundary. Product workflows, approvals,
  storage, and UI state must not depend on Codex-specific transcript formats. Do not add a generic
  provider trait until a second provider is implemented.
- The desktop session registry must have an explicit schema version and a tested forward migration
  for any persisted metadata change.
- Each normalized Agent event must carry a desktop session ID, turn ID, monotonic sequence, event
  kind, timestamp, and optional bounded opaque provider event reference.
- Each CanISend tool result shown in the timeline must preserve the existing operation/receipt
  identifiers. A committed path must be traceable from session and turn through operation, bound
  preview digest and approval outcome, committed Application revision, audit event, and output
  digest where applicable. Raw single-use approval tokens must never enter trace metadata.
- Trace records must remain body-free: no prompt, response, private evidence, credential, generated
  document, or MCP argument body is copied into diagnostics or correlation metadata.
- The implementation must reuse the accepted dependency graph, operation registry, ActionReceipt,
  Agent v4 receipt, revision/digest, and audit contracts instead of creating a second event store.
- Committed origins are durable product-audit metadata, independent of bounded session-cache
  retention. They survive eviction, explicit session deletion, and backup/restore. An attachment
  failure reports a trace gap without retrying the committed mutation.
- Reuse one existing checklist with dependencies, primary test/evidence owner, rollback, and exact
  source identity. Trellis task/phase/journal mechanics are not execution gates. Keep source checks,
  protected CI, artifact qualification, and user validation distinct.

### R9 — Exact file-version comparison

- Each retained build snapshot must have an immutable manifest binding Application/build identity,
  relative path, media type, byte size, SHA-256 digest, and the exact historical bytes in the existing
  BlobStore.
- The manifest includes only paths emitted and registered by CanISend's managed projection/export
  pipeline. It must never discover files by recursively scanning the Workspace.
- Comparing two snapshots must classify files as added, modified, deleted, or unchanged without
  reading file bodies or unrelated Workspace content; body bytes are loaded only after the user
  selects a changed file.
- Modified UTF-8 text files must offer a bounded line-level comparison. JSON may additionally offer
  a structured view. PDF and other binary files must show digest/size changes and side-by-side or
  selectable previews rather than a misleading text diff.
- The text view uses exact whitespace and line endings, Git-style old/new line numbers and `-`/`+`
  markers, and three unchanged context lines. It reports a clear limited state when input, compute,
  or response bounds prevent a complete rendered view.
- Snapshot paths are logical outputs relative to the generation root, independent of export
  destination. Record actual generator/build provenance and link predecessors within the same kind.
- Reuse only an identical current head with the same full generation binding. A -> B -> A and a
  changed generator preserve history; equal bytes still deduplicate in BlobStore.
- The default comparison is the current snapshot against its same-kind immediate predecessor, with
  an explicit selector for retained snapshots of that kind.
- Comparison is read-only, supports at most 256 managed entries per snapshot, loads at most 4 MiB per
  selected text side and 20,000 lines per side, verifies both Blob digests before use, and never
  writes temporary comparison files into the Workspace.
- Historical comparison and repair/restore of newly snapshotted outputs use stored verified bytes,
  not the current serializer, template, font, renderer, or Pack. Existing unsnapshotted recovery is
  labelled separately and cannot claim exact historical output.

### R10 — Post-MVP generic ACP channel

- This post-MVP requirement starts only after the Codex App Server MVP completes implementation phase
  R4. It must not add code, dependencies, or provider abstractions to implementation phases R0-R4.
- When the second concrete provider is implemented, extract the smallest session contract shared by
  Codex App Server and ACP: initialize/readiness, start/resume, prompt, streamed updates, permission
  response, cancellation, and close.
- The ACP channel uses ACP v1 over stdio JSON-RPC, validates protocol and advertised capabilities,
  and keeps agent-specific extensions outside product workflows and storage.
- Claude Agent is the first planned live qualification. Its adapter must be checked against current
  official Claude Agent SDK/CLI behavior, authentication, distribution, terms, branding, resume,
  permissions, and MCP behavior before CanISend labels it supported.
- CanISend advertises only the minimum client capabilities it safely provides. An ACP agent that
  requires unrestricted filesystem writes, terminal execution, or an unsupported permission model
  is incompatible rather than silently over-privileged.
- ACP Registry metadata may support discovery and diagnostics after qualification, but the first R10
  slice does not auto-install or auto-update agents.
- Harness and similar products remain MCP consumers or external handoff targets unless they publish
  a separately qualified embedded-session interface. A remote MCP bridge requires its own privacy,
  authentication, availability, and consent design.

## Non-goals

- Electron migration in the MVP.
- Claude-provider parity in the MVP.
- A generic ACP client or ACP Registry installation flow in the MVP.
- A general IDE, arbitrary terminal, arbitrary file editor, or autonomous multi-agent orchestration.
- Automatic provider installation, provider account creation, subscription handling, or App-owned
  API credentials.
- Persisting complete transcripts in CanISend.
- Initializing Git inside a Workspace, automatically staging/committing user content, or pushing it
  to a remote repository.
- An optional user-managed private Git export or projection mirror; this requires a separate privacy,
  conflict, retention, and recovery design after MVP.
- Scanning, watching, or versioning arbitrary user-managed files outside CanISend's registered
  projection/export output set.
- Rename detection, semantic merge, blame, branching, staging, or a general source-code review tool.
- Rich DOCX/HTML editing, PDF annotations, or automatic third-party submission.
- Expanding the supported desktop platform matrix as part of this roadmap.

## Acceptance Criteria

- [x] R0 source: a clean Workspace v4 completes **Prepare AI Workspace** without the legacy compatibility error
      and without a Workspace v2 scope-catalog call.
- [x] R1 source: from the App, a user can start a Codex App Server session, send a prompt, see incremental updates,
      cancel a turn, and resume the known session after restarting the App.
- [ ] The agent can discover and call the existing CanISend MCP operations for the selected
      Workspace/Application without direct access to product storage.
- [ ] A proposed mutation cannot be committed without the existing preview and explicit approval;
      deny, stale-preview, replay, and verification-failure cases remain fail-closed.
- [ ] The Workbench supports the end-to-end loop of ask, inspect evidence, review a proposal, approve
      or reject it, preview exact output, and export without opening an external agent.
- [ ] The exported PDF digest matches the bytes shown by the in-App preview, and the system-viewer
      fallback remains available.
- [ ] Provider absence, authentication need, App Server crash, malformed/oversized messages, cancellation,
      and restart each have a focused deterministic regression and actionable UI state.
- [ ] The fake App Server integration, affected Rust tests, frontend checks/tests, and the final source gate
      pass on the final implementation head.
- [ ] Keyboard-only and 200% text-scale checks pass for the Workbench's critical path.
- [ ] External handoff remains usable as a fallback until the embedded path has parity.
- [ ] For a representative committed Agent change, diagnostics can reconstruct the body-free chain
      `session -> turn -> tool call -> CanISend operation -> preview/approval outcome -> revision/audit`
      while the product store remains the only authority.
- [ ] From the Workbench, a user can inspect a committed Application revision's actor, reason,
      timestamp, snapshot digest, and originating session/turn references without Git being installed
      or a `.git` directory being created in the Workspace.
- [ ] From the Workbench, a user can compare the current and previous retained file snapshots and see
      the exact added, modified, and deleted managed paths; selecting a modified text file loads only
      its two digest-verified historical Blobs and shows a bounded Git-style line comparison.
- [ ] A PDF or other binary change is reported by path, media type, size, and old/new digest with
      preview access; the App never presents binary bytes as a text diff.
- [ ] Reopening the App or upgrading its renderer does not change the bytes or reported diff of a
      previously retained file snapshot.
- [ ] Each completed phase records its exact source identity and primary test/evidence reference in
      the existing checklist, without staging or publishing private Workspace content.
- [ ] R2 isolation/private-read consent, durable origin/cache loss, logical file identity/Blob repair,
      and R3 bounded provider-history display pass their dedicated acceptance criteria.
- [ ] R4 qualifies the exact embedded Beta; M4-APP-005 updates the existing cohort validator before
      one formal user cohort runs on that build with unchanged thresholds. Preserve Beta.1 history;
      do not require a second complete cohort or count historical UI flows as new Workbench evidence.
- [ ] A session-registry migration, Codex CLI upgrade, and embedded-client rollback each have a focused
      regression that preserves existing Workspace/Application data.

### Post-MVP R10 acceptance

- [ ] The same Workbench can select Codex App Server or one qualified ACP agent without changing
      CanISend product workflows, approval semantics, storage authority, or trace format.
- [ ] A fake ACP process proves initialization, capability negotiation, start/resume, streaming,
      permission response, cancellation, malformed/oversized input, exit, and close without network
      access or provider credentials.
- [ ] The first supported Claude Agent adapter passes the common lifecycle cases plus separately
      recorded authentication, distribution, permission, MCP, and native smoke evidence.
- [ ] An ACP agent missing a required baseline capability or requesting an unadvertised filesystem or
      terminal capability is rejected with an actionable state and no product mutation.
- [ ] Disabling the ACP channel leaves Codex App Server, existing Workspaces, and external handoff
      unaffected.

## Constraints

- Accepted architecture decisions and the master 1.0 roadmap remain authoritative until explicitly
  superseded or amended.
- The repository is under feature-freeze controls. Implementation cannot begin as an untracked broad
  exception; R0 must establish the approved product and release boundary.
- The current Tauri command facade, consent protocol, exact-byte preview, and operation registry are
  assets to reuse, not systems to rewrite.
- The accepted desktop product boundary names a CanISend data directory a Workspace and explicitly
  does not require it to be a Git repository.
- Rust 1.97, Tauri 2, Svelte 5, TypeScript/Vite, and Node/pnpm as build-time-only dependencies remain
  the support boundary for the MVP. A change to that boundary requires a superseding ADR.
- Authentication behavior is provider-owned. The App must not imply that a third-party provider can
  use a consumer subscription when its terms require API credentials.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Codex App Server schemas change | Test a supported Codex CLI range, probe initialization capabilities, and use generated schemas plus a fake App Server fixture. |
| ACP implementations vary or Registry metadata drifts | Require the ACP v1 baseline plus per-agent qualification; use a fake ACP fixture and do not auto-install in the first slice. |
| Two approval systems confuse users | Label host permission separately from CanISend record approval and never combine their actions. |
| The agent gains direct product-data access | Prove effective policy, inherited configuration, every tool path, and explicit private-read/provider-send consent before enabling private tools. |
| Frontend rewrite stalls product work | Add one Workbench vertical slice, reuse existing views/operations, then retire old routes after parity. |
| Tauri preview proves inconsistent for a richer need | Define a measurable preview gap, try PDF.js in Tauri, and evaluate Electron only if that bounded prototype fails. |
| Provider content leaks into local diagnostics | Persist metadata only and redact message/tool bodies by default. |
| Traceability grows into a second event store | Persist only session metadata and receipt references; keep revisions and audit truth in the existing Rust store. |
| Git captures private Workspace data or creates two authorities | Never initialize, stage, commit, or push Workspace content automatically; keep Git on the development/release side of the boundary. |
| Historical files are regenerated by newer code | Persist the exact generated bytes and immutable manifest at snapshot time; verify their digests before comparison. |
| File snapshots grow over time | Track only registered managed outputs, deduplicate exact bytes in BlobStore, enforce the existing output-count boundary, and make retention a separately approved policy. |
| Large or pathological text freezes the App | Compare manifests before bodies, load one selected pair on demand, cap bytes/lines/rendered rows, and use a timed diff algorithm with an explicit limited state. |
| Framework/provider upgrades drift silently | Record the supported Codex range, probe App Server capabilities, validate lockfiles/operation registry, and retain a tested rollback path. |

## Authority boundary

ADR-RN-0022 and the master roadmap approve the direction; R0/R1 source evidence is retained in their
archived checklists. The owner requested the 2026-09-04 roadmap/process revision. R2 product
implementation is not claimed by this edit. The master roadmap owns ordering and exact freeze,
artifact, and user gates; this supporting plan changes no release or platform claim.

## Notes

- `design.md` contains the architecture contracts and measured Electron gate.
- `implement.md` contains the roadmap and MVP implementation checklist.
- Research snapshots under `research/` record the local and upstream evidence used for these
  decisions.
