# Embedded Agent App Technical Design

Status: Approved architecture; R0 complete; R1 verification in progress
Date: 2026-09-04

UX decision: conversation-led Workbench approved by the product owner on 2026-09-04.

## 1. Design summary

Keep the existing Tauri 2 + Svelte 5 shell and replace the process-per-turn agent bridge with the
smallest useful client for the official Codex App Server interface. Run the installed
`codex app-server --listen stdio://` process directly. Give Codex the existing CanISend MCP server as
its only product-data boundary. Stream normalized session events to a new Workbench through Tauri
channels.

This is a strangler change around the current Rust facade, not a rewrite of the domain core or all
frontend views.

The supported stack remains Rust 1.97, Tauri 2, Svelte 5, TypeScript, Vite, shadcn-svelte/Bits UI,
and SQLite through the existing Rust store. The only new runtime boundary is a Codex App Server
child process; Node and pnpm remain build-time dependencies.

```text
Svelte Workbench
  | Tauri commands + streaming channel
  v
canisend-gui session manager
  | Codex App Server JSON-RPC over stdio JSONL
  v
installed codex app-server
  |
  | MCP configuration
  v
canisend mcp serve ----> canisend-app facade ----> local Workspace v4 store

Workbench Inspector ----> existing preview/approve/commit/export commands
                     ----> exact verified PDF bytes
```

Codex App Server owns the MVP conversation/session transport. MCP owns product tools. The existing
application facade owns product invariants.

The MVP does not contain a provider interface. When R5 introduces a second concrete implementation,
extract the session behavior already proven by Codex and implement one ACP v1 stdio channel beside
it. This keeps the current path small without locking the Workbench to Codex wire types.

## 2. Why Tauri remains the MVP shell

Electron is technically feasible and provides a consistent bundled Chromium runtime. It does not,
however, remove the need to build a PDF viewer, a safe preload/IPC boundary, child-process lifecycle,
or content isolation. Electron's own guidance discourages using its `<webview>` tag for general web
embedding and requires strict isolation for untrusted content.

CanISend already has:

- an accepted Tauri/Svelte architecture;
- 129 registered Tauri operation leaves and 123 frontend invoke call sites in the current audit;
- a Rust-only domain facade and Tauri capability policy;
- exact PDF bytes rendered in an iframe with a system-viewer fallback; and
- prior WKWebView, WebView2, and WebKitGTK preview qualification.

Migrating now would replace a working shell while leaving the missing agent-session and UX work
untouched.

| Criterion | Tauri 2 now | Electron now | MVP decision |
| --- | --- | --- | --- |
| Codex App Server child process | Supported through a Rust child process | Supported from main process | Tie |
| Stream events | Tauri channels/events | IPC | Tie |
| Exact PDF preview | Existing verified-byte iframe path | Chromium viewer or custom viewer | Keep existing |
| Cross-platform rendering consistency | OS webview differences | Bundled Chromium | Electron advantage, not yet a blocker |
| Existing integration | 129 operation leaves already wired | Requires a new IPC/preload bridge | Tauri advantage |
| Package size/update burden | Uses OS runtime | Bundles Chromium/Node | Tauri advantage |
| Security work | Existing capabilities and Rust boundary | New preload, IPC validation, sandbox and Electron update duty | Tauri advantage |

### Electron reconsideration gate

Do not reopen the shell decision because preview “feels difficult.” Reopen it only when all of the
following are true:

1. a named MVP/post-MVP workflow requires preview behavior the current iframe cannot provide;
2. the requirement has objective acceptance criteria, such as search, thumbnails, stable zoom,
   annotations, or identical rendering on named targets;
3. a bounded PDF.js-in-Tauri prototype fails those criteria on the supported native matrix; and
4. an Electron spike proves the same criteria while quantifying migration, package, memory, update,
   signing, accessibility, and security costs.

If those conditions are met, write a superseding ADR before migrating.

## 3. Backend shape

### 3.1 Location

Start inside the existing `crates/canisend-desktop/src/agent_runtime.rs`. Reuse its executable
discovery, process limits, session registry, Serde types, and Tauri boundary. Keep one module while it
remains readable; split only after the implemented vertical slice proves a concrete boundary. Do not
create a protocol crate for one consumer or enable extra Tokio features without a demonstrated need.

### 3.2 Minimal Codex App Server subset

Implement only the operations needed for the vertical slice:

- `initialize` followed by the `initialized` notification;
- authentication/readiness reporting from the installed Codex client;
- `thread/start` and `thread/resume`;
- `turn/start`;
- streamed notifications and bounded server requests, including approval requests;
- `turn/interrupt`; and
- bounded shutdown/restart.

Do not implement experimental WebSocket transport, multi-session scheduling, terminal rendering,
arbitrary filesystem capabilities, or vendor-specific extensions in the MVP.

Newline-delimited JSON-RPC messages are read and written on stdio. Stderr is captured as a bounded,
redacted diagnostic stream and never mixed into the protocol parser. Each request has an identifier,
timeout/cancellation path, and exactly one completion.

### 3.3 Process and session lifecycle

The desktop backend owns one session manager per App window:

1. Resolve an explicitly configured `codex` executable or a supported candidate already used by the
   desktop runtime.
2. Create a controlled session directory outside the selected Workspace's product storage.
3. Run `codex --version`, then spawn `codex app-server --listen stdio://` with piped
   stdin/stdout/stderr and no shell interpolation.
4. Initialize App Server and validate the required methods/capabilities against the supported Codex
   range.
5. Start a new thread or resume the stored Codex thread.
6. Forward normalized events through one Tauri channel.
7. On cancellation, request `turn/interrupt` before bounded process termination.
8. On App close or unrecoverable protocol failure, close pipes, terminate the exact child, and clear
   volatile state.

Only the versioned resumable metadata defined in section 3.6 is persisted: Codex version, desktop
session and turn identifiers, Codex thread/turn identifiers, Workspace/Application binding,
timestamps, last status, and body-free receipt references. Prompt, response, tool-input, evidence,
generated-body content, and raw single-use approval tokens are not added to CanISend storage in the
MVP.

### 3.4 MCP injection

The agent session receives an MCP server definition that launches the current `canisend` binary with
the selected Workspace and `mcp serve`. The MCP server continues to route through `canisend-app` and
the registered domain operations.

The selected Application is explicit session context, not inferred from a legacy Agent/Job/Task
scope. Clean Workspace v4 setup must not call the old scope catalog.

App Server and MCP run as separate protocol streams. The desktop App must not proxy arbitrary MCP
payloads through frontend JavaScript.

### 3.5 Permission and mutation model

Two controls remain separate:

- **Host permission** — a provider asks to read a file, execute a command, or use the network. The
  MVP defaults to deny because CanISend operations are available through MCP.
- **Product approval** — CanISend presents a deterministic proposed mutation or export and requires
  explicit user approval before commit.

Approving one never approves the other. Existing stale-preview, replay, consent, commit, verification,
recovery, and audit invariants remain authoritative.

### 3.6 Body-free trace chain

Traceability extends the current receipts and audit contracts; it does not create another product
event store.

```text
desktop_session_id
  -> codex_thread_id
  -> desktop_turn_id + codex_turn_id + monotonic event sequence
  -> bounded opaque provider event/tool-call reference
  -> CanISend operation + Agent v4 task/receipt
  -> preview digest + approval/consent outcome (never the raw token)
  -> committed Application revision + audit_event_id
  -> exact output digest (when produced)
```

Not every event reaches the end of the chain. A read-only call ends with its ActionReceipt; a denied
proposal ends with the denial and creates no revision.

| Data | Owner and retention |
| --- | --- |
| Codex/thread/version, Workspace/Application binding, last status | Existing App-local session registry, with an explicit schema version |
| Session/turn/tool-call correlation to body-free CanISend receipt references | Session registry metadata, removed only by explicit session deletion |
| Streamed prompt, response, tool arguments, and private evidence bodies | Memory/provider session only in the MVP |
| Operation, task, preview digest, approval outcome, revision, artifacts, and audit event | Existing CanISend receipts and Rust store; product audit survives session deletion |
| Error code, timing, Codex version, and correlation IDs | Bounded redacted diagnostics |

The desktop assigns its own stable session and turn identifiers and retains provider identifiers only
as bounded opaque external references. Each normalized event has a monotonic sequence so reconnects
can deduplicate or detect a gap. UI labels show human-readable status by default; a details disclosure
exposes the body-free IDs needed for support.

### 3.7 Git and product-history boundary

Git already tracks CanISend source, architecture records, Trellis plans, tests, and release inputs;
the packaged build already records its source Git revision. Keep that development provenance.

Do not initialize Git inside a user Workspace or make Git a runtime dependency. A Workspace contains
private SQLite state, immutable blobs, and managed projections whose authority, atomicity, recovery,
and consent semantics already live in `canisend-store`. Git would duplicate that truth without
capturing its transactional invariants.

For user-visible content history, expose the existing Application revision metadata through the
`canisend-app` facade as one read-only Workspace v4 operation. The Inspector can show revision,
actor, reason, timestamp, snapshot digest, and body-free session/turn receipt references. This reuses
the existing revision tables and session correlation metadata; it does not add a Git wrapper or a
second history store.

A future user-requested export to a private Git repository may be designed as an opt-in projection,
never as Workspace authority. It is outside the MVP because privacy, ignore rules, binary handling,
conflicts, retention, and recovery would need explicit contracts.

### 3.8 Exact file snapshots and comparison

The current store has the necessary byte authority but not a complete historical file manifest.
Application revisions retain snapshots, Deliverable content retains Blob references, and managed
files can be deterministically projected. However, `application_projection_v4_manifests` makes each
relative path unique and the projector updates that path, so the table describes the current
projection. Reconstructing an old file with newer generation code is not an exact historical copy.

The approved scope is the output set written and registered by CanISend's managed
projection/export pipeline. The manifest is populated from that pipeline's in-memory output batch;
it never walks the Workspace filesystem. Add two append-only Store-owned concepts:

```text
file snapshot
  id, predecessor_id, Application revision, Pack digest,
  generator/build revision, manifest digest, actor/reason/time

file snapshot entry
  snapshot_id, relative path, kind/media type, byte size,
  content SHA-256, existing BlobStore reference
```

After a successful managed generation, the pipeline stores every exact file byte sequence in the
existing content-addressed BlobStore, then atomically commits the snapshot, at most 256 manifest
entries, and their Blob references before publishing the batch. Identical bytes deduplicate
naturally. Managed files on disk remain repairable projections of the snapshot authority. Imported
source files and arbitrary files a user places elsewhere in the Workspace are not part of this file
snapshot; their existing content/evidence records remain authoritative.

The read-only comparison service first loads two immutable manifests and returns added, modified,
deleted, and unchanged paths without loading bodies. A second on-demand call accepts one selected
path. It verifies both Blobs before producing a bounded line comparison for supported UTF-8 text.
PDF and other binary entries return old/new digest and size plus preview handles. No historical
bytes are regenerated, and no comparison temp files are written into the Workspace.

The Inspector's History area contains a Files view: snapshot selector and summary first, changed file
tree second, exact text comparison or binary preview last. The default is current versus immediate
predecessor. The narrow Inspector uses a unified line view; the existing responsive drawer can give
that view more width without adding an editor framework.

### 3.9 Git-style diff engine

Git's presentation is useful here; its repository model is not. The MVP uses the existing immutable
Blob snapshots plus one small in-process line-diff dependency.

| Option | Decision | Reason |
| --- | --- | --- |
| `git diff --no-index` child process | Reject | Requires system Git and materialized paths, adds process/output parsing, and weakens the no-temp-file boundary. |
| `git2`/`gix` repository library | Reject | Adds repository/object/database semantics and, for `git2`, a native libgit2 boundary that this use case does not need. |
| `similar` 3.2.0 in `canisend-app` | Select | Pure Rust, dependency-free by default, supports line hunks, Git's default Myers family, exact whitespace, newline handling, and a computation timeout. |

R0 records the `similar` 3.2.0 qualification evidence under the repository's existing Apache-2.0,
advisory, and source policy. R2 adds and locks it only when the comparison implementation exists.
`canisend-store` owns snapshot rows and verified Blob reads; `canisend-app` owns the comparison use
case and is the only new consumer of the diff crate. Tauri
exposes versioned read-only summary and selected-file operations. Svelte receives typed rows and
only renders escaped text; it does not compute diffs or receive unrelated file bodies. File-body
diffs are not added as agent or MCP tools in the MVP.

The line algorithm is `Algorithm::Myers`, matching Git's default algorithm family, with exact
whitespace, preserved line endings, three context lines, and a 250 ms computation timeout. The
bounded approximation remains a correct change view even when it is not the minimal edit script.
The service applies these limits before or while producing a response:

- 256 managed manifest entries per snapshot;
- 4 MiB and 20,000 lines per selected UTF-8 side;
- one selected path per request; and
- 2,000 emitted diff rows, followed by an explicit limited marker and exact-file preview links.

The fast path compares `relative_path -> SHA-256` maps only. Equal digests are never opened. The
selected-file path reads and verifies only its old/new Blobs, computes grouped hunks on demand, and
does not persist a derived patch or cache in the MVP. Non-UTF-8 or declared binary content never
enters the text algorithm.

The renderer shows an A/M/D summary, path, old/new digest and size, hunk header, old/new line numbers,
and redundant `-`/`+` markers in addition to color. Line-ending-only and whitespace-only changes stay
visible by default. Rename inference, whitespace-ignore mode, intraline highlighting, blame, merge,
and a Monaco/CodeMirror diff editor are deferred until measured use requires them.

### 3.10 Post-MVP ACP channel

R5 introduces the second session implementation and only then extracts a shared boundary:

```text
Workbench normalized events
  -> Codex App Server session
  -> ACP v1 stdio session -> qualified ACP Agent

Either session -> Workspace/Application-bound CanISend MCP -> canisend-app
```

The extracted contract contains only lifecycle already required by both implementations:
initialize/readiness, start/resume, prompt, streamed update, permission response, cancel, and close.
ACP initialization negotiates its version and capabilities. Optional filesystem, terminal,
elicitation, modes, plans, and extension methods remain disabled unless a named CanISend requirement
and permission design explicitly enables them.

Claude Agent is the first live qualification target because its official Agent SDK/CLI provides
sessions, streaming, permissions, and MCP, while an ACP wrapper can supply the common wire boundary.
The wrapper remains a separately versioned and provenance-checked component; its presence in a
registry is not proof of support.

Other registry agents may be qualified later against the same fake-ACP and capability suite. Harness
currently fits the MCP-consumer/external-integration side of the boundary. A remote or pipeline Agent
cannot access a private local Workspace merely because both sides speak MCP; remote transport,
authentication, consent, and availability need a separate design.

## 4. Frontend shape

### 4.1 Shell

Reduce primary navigation to Work, Library, and Settings. Put the active Workspace/Application
switcher in the header. Keep old routes reachable during migration, but stop treating Agent,
Workflow, and Delivery as separate primary destinations.

### 4.2 Workbench

The Workbench uses an efficient three-region desktop layout:

```text
+--------------------------------------------------------------------+
| Workspace / Application     provider status      session actions   |
+----------------+--------------------------------+------------------+
| compact context| conversation and progress      | Inspector        |
| or stage rail  |                                | Context          |
|                |                                | Evidence         |
|                |                                | Changes          |
|                |                                | Consent          |
|                |                                | History          |
|                |                                | Preview          |
|                +--------------------------------+                  |
|                | prompt composer / cancel       |                  |
+----------------+--------------------------------+------------------+
```

The left region is omitted when it has no actionable stage/context information. The Inspector is
collapsible and becomes a drawer at narrower supported widths. Conversation remains the spatial
anchor; approvals and exact-output preview stay visible beside the event that produced them.

Reuse current shadcn-svelte primitives, semantic tokens, typography, focus behavior, density modes,
and locale machinery. Do not introduce a second component library, visual theme, or animation
system.

### 4.3 Frontend state

Keep protocol state out of the root `App.svelte` callback graph. A small Workbench controller/store
owns:

- provider readiness and connection state;
- current Codex thread/session metadata;
- normalized timeline events;
- active turn/cancellation state;
- pending host permission; and
- the currently inspected product proposal, history entry, or preview.

Product records continue to load and mutate through the existing typed bridge. Protocol events carry
stable identifiers and bounded display data; they do not become a parallel product store.

### 4.4 What the user sees

The App opens directly into useful work, not a metric dashboard:

- A first-time or empty Workspace shows one primary action to create/import an Application and one
  secondary action to connect Codex.
- A returning user lands on the most recently active Application and resumable session, with other
  Applications available from the header switcher.
- The left navigation contains only Work, Library, and Settings. It may show the current Application
  summary, but it does not duplicate the workflow as a second navigation tree.
- Work is conversation-led: the center timeline holds user intent, agent responses, and concise tool
  progress. The composer remains at the bottom.
- The right Inspector follows the selected event and shows Context, Changes, Consent, History, or
  Preview. History includes both provenance metadata and exact retained file-snapshot comparison.
  It is closed when there is nothing material to inspect.
- Library holds opportunities, profile/evidence sources, and inactive or completed Applications.
- Settings holds Workspace management, agent connection/readiness, privacy, appearance, and
  diagnostics.

There is no separate primary Agent page. “Agent” describes how work is performed inside an
Application, not another place the user must manage.

### 4.5 Interaction contract

| State | Center timeline | Inspector | Primary user action |
| --- | --- | --- | --- |
| Ready | Existing conversation and a clear prompt field | Current Application context on demand | Describe the outcome in natural language |
| Working | Streamed answer plus compact plan/tool progress | Evidence or tool detail only when selected | Continue, steer, or cancel |
| Host permission | Explanation of what Codex requested | Exact file/command/network request | Allow once or deny; default is deny |
| Product review | Agent summary says what will change and that nothing is written yet | Structured diff, evidence links, validation, and consent | Approve this bound preview or reject it |
| Verified | Commit/verify result appears beside the originating turn | Updated record, revision history, or exact output preview | Continue editing or export |
| Offline/error | Conversation remains readable with a precise recovery message | Readiness diagnostic | Retry, reconnect, or use external handoff |

A normal interaction therefore reads as one continuous exchange:

1. The user selects an Application and asks for an outcome in plain language.
2. Codex receives only the bound Application context and CanISend MCP tools.
3. Codex App Server streams narrative, progress, and any host permission request into the timeline.
4. Read-only tool results appear without blocking; a material mutation opens the Inspector.
5. The user reviews evidence, diff, validation, and consent, then approves or rejects the exact
   proposal.
6. CanISend commits and verifies through its existing facade; the Agent cannot mark its own proposal
   approved.
7. The Inspector can show the resulting revision and provenance beside the originating turn.
8. The Inspector shows the exact PDF bytes for final review, then the user exports deliberately.

The user may continue talking after any step. Rejecting a proposal changes no product state and
returns naturally to the conversation rather than opening an error page.

## 5. Preview strategy

Use the smallest renderer that preserves truth:

- text and Markdown: sanitized Svelte rendering;
- structured records and evidence: existing components and semantic tables/cards;
- proposed mutations: structured field diff, not a generated screenshot;
- final PDF: the existing exact-byte Blob URL/iframe path with digest and system-viewer fallback.

PDF.js is a post-observation enhancement for consistent controls such as page navigation, search,
thumbnails, and zoom. Electron is not a PDF feature.

Remote or provider-generated HTML is never loaded with native privileges. If a future requirement
needs remote content, it receives a separate threat model and navigation policy.

## 6. Failure handling

The App exposes finite, actionable states: not configured, executable missing, authentication
required, connecting, ready, running, cancelling, recoverable disconnect, incompatible App Server, and
failed.

Limits are required for line/message size, buffered stderr, pending requests, event history in memory,
startup time, request time, and shutdown time. Unknown optional events are ignored with a redacted
diagnostic; invalid framing or required-field loss terminates the affected session safely.

The external handoff remains available when Codex or its App Server capability is missing or
incompatible.

## 7. Verification design

- A tiny fixture executable speaks the implemented App Server subset and can emit success, streaming,
  permission, malformed, oversized, delayed, cancellation, exit, and resume scenarios.
- Rust tests own framing, request correlation, lifecycle, limits, redaction, and policy.
- Desktop command tests own Workspace/Application binding and the clean Workspace v4 regression.
- One read-only history test owns revision order, actor/reason/timestamp/digest exposure, body-free
  session correlation, and the absence of Workspace Git initialization.
- File-snapshot tests own append-only manifests, exact-Blob retention, added/modified/deleted
  classification, bounded text comparison, binary handling, generator-upgrade stability, and failure
  atomicity.
- Frontend tests own normalized event rendering, controls, approval separation, accessibility states,
  and exact-preview continuity.
- One native macOS smoke owns the first vertical slice. Existing scheduled/native gates retain their
  target ownership; Windows/Linux claims are not inferred from macOS.
- The final implementation head runs the repository's required focused checks and source gate.

## 8. Rollout and rollback

Ship the Workbench behind the existing embedded-agent capability boundary until it has parity. Keep
external handoff and the old views during the slice. If App Server readiness or recovery is inadequate, turn
off the embedded entry point without changing stored Workspace data. Remove the one-shot bridge and
old Agent page only in the post-parity cleanup milestone.

## 9. Sustainability controls

- Keep domain rules in `canisend-core`/`canisend-app`, storage and audit in `canisend-store`, MCP in
  `canisend-mcp`, Codex App Server process/session behavior in `canisend-gui`, and presentation in
  Svelte.
- Keep the Workbench controller independent of Codex event shapes by normalizing the implemented App
  Server subset once in Rust. Add a provider trait only when a second provider exists.
- In R5, extract that trait from the working Codex and ACP implementations rather than designing it
  in advance; unsupported ACP capabilities fail closed.
- Record the tested Codex CLI range and required App Server capabilities; reject an incompatible
  installation with an actionable readiness state rather than guessing. Generate TypeScript or JSON
  Schema fixtures from the tested CLI during development, not on every App start.
- Use the fake App Server process as the durable compatibility fixture. Real-provider smoke is evidence for
  integration, not the only regression.
- Version persisted session metadata and test its migration. Workspace data requires no migration for
  embedded-client rollback.
- Map every PRD requirement to a focused test/evidence item in its just-in-time Trellis phase task.
- Record the exact source Git commit or PR head with each phase's Trellis completion evidence, while
  keeping Workspace paths and content out of source-control evidence.
- Retain external handoff until the embedded path completes parity and a rollback window.
- Keep Electron, PDF.js, provider SDKs, ACP, transcript storage, and a new protocol crate outside the MVP
  until a measured requirement crosses their documented gate.
