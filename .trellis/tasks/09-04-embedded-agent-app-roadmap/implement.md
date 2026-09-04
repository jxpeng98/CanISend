# Embedded Agent App Roadmap and MVP Checklist

Status: Approved roadmap; R0 complete; R1 not started
Date: 2026-09-04

## Roadmap at a glance

```text
R0 architecture/UX gate
  -> R1 Codex App Server vertical slice
     -> R2 MCP + approvals
        -> R3 simplified Workbench
           -> R4 MVP hardening and release gate
              |---- MVP cut ----|
              -> R5 generic ACP + Claude qualification (post-MVP)
              -> R6 legacy removal (after parity)
```

This sequence proves the protocol and safety loop before paying for a broad frontend rewrite. No
calendar dates are assigned until R0 is approved and the first vertical slice exposes real effort.

## Phase controls

| Phase | Depends on | PRD coverage | Minimum completion evidence | Rollback point |
| --- | --- | --- | --- | --- |
| R0 | Approved final plan | R1, R5, R7-R9 authority | ADR/roadmap reconciliation, clean-v4 positive/legacy negative regressions, qualified dependency record | Revert authority docs and blocker patch; current external handoff remains |
| R1 | R0 complete | R2, R6, R8 session metadata | Fake-App-Server protocol suite, desktop command tests, one native Codex smoke | Disable embedded capability and retain one-shot/external paths |
| R2 | R1 complete | R3, R8, R9 | Store/App comparison tests, approval failure matrix, one committed and denied trace | Stop exposing MCP/file-history additions; append-only Workspace data remains readable |
| R3 | R2 complete | R4, R5, R8, R9 UI | Frontend tests/build plus keyboard, 200% text, locale, theme, and density evidence | Restore old routes as primary while keeping additive backend work |
| R4 | R3 complete | All MVP requirements | Recovery matrix, exact-output checks, native smoke, final source gate, evidence map | Disable embedded entry point; preserve Workspaces and external handoff |
| R5 | R4 complete | R10 post-MVP ACP/provider parity | Fake-ACP lifecycle suite plus separately qualified Claude Agent smoke | Disable the ACP channel only; Codex and external handoff remain |
| R6 | Measured parity and rollback window | R7 cleanup | Saved-link/recovery regressions and operation/architecture reconciliation | Restore old route/runtime from the last parity head |

Every phase is implemented as a just-in-time Trellis child task that records the exact Git commit or
PR head and links each listed requirement to its focused checks. A phase does not begin while its
dependency or exit evidence is incomplete.

## R0 — Architecture, authority, and Workspace v4 blocker

Outcome: the product direction is authoritative and clean Workspace v4 preparation is usable.

- [x] Approve this PRD and the Tauri-for-MVP decision.
- [x] Record the approved conversation-center/right-Inspector UX in the App-first Codex ADR.
- [x] Write an ADR for App-first Codex App Server + MCP architecture; supersede only the
      external-host-first parts
      of the existing desktop/agent decisions.
- [x] Reconcile the authoritative 1.0 roadmap and feature-freeze boundary before product code lands.
- [x] Define the supported installed `codex` discovery/version/App Server capability policy and MVP
      authentication statement. Reuse Codex sign-in state; never collect or persist provider tokens.
- [x] Remove the clean Workspace v4 dependency on legacy Agent/Job/Task/Workflow scope lookup.
- [x] Keep the existing v4 handoff-method regression and add one cross-layer clean-v4 screen
      regression plus one explicit legacy-repository behavior regression.
- [x] Record preview acceptance criteria and confirm the existing exact-PDF path before considering
      PDF.js or Electron.
- [x] Record the Git boundary in the ADR: Git owns source/release provenance; Workspace content uses
      the existing local revision/audit authority and is never auto-committed or pushed.
- [x] Record the approved file boundary: snapshot only outputs written and registered by the
      CanISend projection/export pipeline; never recursively scan arbitrary Workspace files.
- [x] Record `similar` 3.2.0 dependency, license, advisory, and source qualification; add and lock it
      only with the R2 comparison implementation. Retain no fallback that shells out to Git.

Exit: a clean Workspace v4 prepares successfully; architecture/release authorities name the new
direction; no shell migration is open-ended.

## R1 — Codex App Server vertical slice

Outcome: a user can hold one streamed Codex conversation inside the App.

- [ ] Add a deterministic fake Codex App Server process for offline protocol tests.
- [ ] Implement newline-delimited JSON-RPC framing with bounded input/output.
- [ ] Implement `initialize`/`initialized` and required-method/capability validation.
- [ ] Implement thread start/resume, turn start, notification/server-request streaming,
      `turn/interrupt`, and shutdown.
- [ ] Assign desktop session/turn IDs and monotonic event sequence numbers before normalizing provider
      events.
- [ ] Reuse the existing Codex executable discovery path, run `codex --version`, and spawn exactly
      `codex app-server --listen stdio://` without a shell.
- [ ] Separate stdout protocol parsing from bounded/redacted stderr diagnostics.
- [ ] Add timeouts, request correlation, child-exit handling, and exact-child cleanup.
- [ ] Expose one Tauri start/load command, one turn command with a streaming channel, one cancel
      command, and one status/readiness command.
- [ ] Version and migrate the existing session registry; persist only resumable metadata and
      body-free receipt references, never transcript bodies.
- [ ] Add a minimal conversation panel to prove streaming, cancel, restart, and resume.

Exit: fake-App-Server tests pass and a local Codex session streams in the App without invoking the old
process-per-turn path.

## R2 — CanISend MCP and safe review loop

Outcome: the embedded agent can do useful product work without bypassing CanISend invariants.

- [ ] Start the agent in a controlled session directory outside raw Workspace product storage.
- [ ] Inject the existing CanISend MCP server configuration bound to the selected Workspace and
      Application.
- [ ] Default-deny direct Codex file, command, and network permissions in the MVP.
- [ ] Render host permission requests separately from CanISend mutation approvals.
- [ ] Route proposals through the current preview, approve, commit, verify, audit, and recovery path.
- [ ] Correlate each MCP tool call to its CanISend operation/receipt and, for commits, to the bound
      preview digest, approval outcome, revision, audit event, and output digest where applicable;
      never persist the raw single-use approval token.
- [ ] Verify deny, timeout, cancellation, stale preview, replay, verification failure, and agent crash
      all fail closed.
- [ ] Redact prompt, tool, evidence, credential, and generated bodies from default diagnostics.
- [ ] Expose one read-only Workspace v4 Application history operation through `canisend-app`, reusing
      existing revision metadata rather than adding a Git or event-store dependency.
- [ ] Add append-only file-snapshot and file-snapshot-entry migrations; populate them only from the
      successful managed projection/export output batch and bind every exact byte sequence to the
      existing content-addressed BlobStore.
- [ ] Add one read-only manifest-comparison operation that returns snapshot metadata and A/M/D/U
      paths without loading file bodies.
- [ ] Add one read-only selected-file operation that verifies only the requested old/new Blobs and
      returns typed text hunks or binary metadata/preview handles.
- [ ] Implement the selected-file text path in `canisend-app` with `similar`'s Myers algorithm, exact
      whitespace and line endings, three context lines, a 250 ms computation budget, and explicit
      256-file, 4-MiB-per-side, 20,000-line-per-side, and 2,000-row bounds.
- [ ] Keep the file-diff operations desktop-only in the MVP; do not expose historical private bodies
      as agent/MCP tools.

Exit: a Codex session can propose and complete one representative Application mutation through MCP,
but cannot commit it without explicit in-App approval or access product storage directly.

## R3 — Simplified Workbench

Outcome: the full MVP journey is understandable from one surface.

- [ ] Add Work, Library, and Settings as the only primary destinations.
- [ ] Add the Workspace/Application switcher and provider state to the Workbench header.
- [ ] Build the conversation timeline, composer, session actions, and progress states.
- [ ] Add a quiet details disclosure for body-free session, turn, operation, receipt, and audit IDs.
- [ ] Build the collapsible Inspector for Context, Evidence, Changes, Consent, History, and Preview.
- [ ] Add History inside the Inspector with revision, actor, reason, timestamp, snapshot digest, and
      body-free originating session/turn references.
- [ ] Add the History/Files flow: current-versus-previous summary, changed file tree, text comparison,
      and PDF/binary preview comparison.
- [ ] Render text changes as an escaped unified view with old/new line numbers, hunk headers,
      `-`/`+` markers, exact whitespace, and a clear limited state; do not add Monaco, CodeMirror, or
      a second frontend diff implementation.
- [ ] Reuse current Applications, Workflow, Delivery, evidence, and bridge behavior behind the new
      composition instead of rewriting domain logic.
- [ ] Keep external handoff in a secondary action when Codex App Server is unavailable or
      incompatible.
- [ ] Remove legacy capability widgets from the clean Workspace v4 primary path.
- [ ] Make the Inspector a drawer at narrower supported desktop widths.
- [ ] Verify keyboard-only use, visible focus, reduced motion, 200% text scale, English/Chinese,
      light/dark, and both density modes on the critical path.

Exit: a user can ask, inspect, approve/reject, inspect revision history, preview, and export without
navigating to an external agent or a separate Agent page.

## R4 — MVP hardening and release gate

Outcome: the vertical slice is bounded, recoverable, diagnosable, and eligible for controlled use.

- [ ] Cover missing executable, auth required, incompatible version, malformed/oversized message,
      App Server exit, hung request, cancel race, restart, and stale-session recovery.
- [ ] Bound message size, stderr, pending requests, in-memory event history, startup, request, and
      shutdown time.
- [ ] Show an actionable readiness diagnostic without exposing sensitive content.
- [ ] Prove reconstruction of one committed and one denied body-free trace chain across App restart.
- [ ] Prove session-registry migration and embedded-client rollback preserve Workspace data.
- [ ] Prove the history view works without Git installed and never creates or modifies a Workspace
      `.git` directory.
- [ ] Prove retained file comparisons remain byte-identical after restart and a simulated generator
      upgrade, and that failed snapshot commits leave no manifest or referenced-Blob inconsistency.
- [ ] Prove unchanged digests never read Blob bodies, only the selected changed path is loaded, all
      file/byte/line/row limits fail visibly, and a bounded pathological fixture cannot freeze the UI.
- [ ] Confirm the exact PDF preview digest matches exported bytes and the system-viewer fallback.
- [ ] Keep the installed Codex CLI as an explicit internal-MVP prerequisite. Any future bundled CLI
      requires a separate distribution, license, signature, provenance, and update decision.
- [ ] Run focused Rust tests, frontend checks/tests/build, the clean-v4 smoke, one native macOS App Server
      smoke, and the final repository source gate.
- [ ] Update user documentation, privacy/authentication wording, capability inventory, and rollback
      instructions.

Exit: every PRD acceptance criterion is evidenced on the final implementation head, or the embedded
entry point remains disabled while external handoff continues to work.

## MVP cut line

The MVP includes R0 through R4 and no more. It supports:

- one desktop window and one active Codex App Server session;
- clean Workspace v4 preparation;
- streaming prompt/response and cancellation/resume;
- CanISend MCP tools;
- explicit host-permission and product-approval handling;
- one Workbench covering context, evidence, proposed changes, consent, exact file history, and PDF
  preview; and
- bounded recovery and diagnostics.

## Sustainability and traceability checklist

- [ ] Keep Rust/Tauri/Svelte and the accepted crate dependency directions; do not add Electron,
      React, a Node runtime, or a second product store.
- [ ] Keep Codex-specific translation inside the Rust App Server/session boundary; do not add a
      generic provider trait until a second provider is implemented.
- [ ] Reuse existing ActionReceipt, Agent v4 receipt, revision, digest, approval, audit, and operation
      registry contracts.
- [ ] Give persisted session metadata an explicit format version and forward-migration test.
- [ ] Record the supported Codex CLI range and required App Server capabilities in diagnostics.
- [ ] Redact all prompt, response, tool-argument, evidence, credential, and generated-content bodies
      from correlation metadata and default logs.
- [ ] Maintain one requirement-to-test/evidence map per R0-R4 Trellis phase task.
- [ ] Record the exact Git commit or PR head for each completed phase without staging private
      Workspace files.
- [ ] Keep file snapshots append-only, content-addressed, and inside the existing Store/Blob authority;
      do not introduce a Git wrapper or second object database.
- [ ] Keep comparison two-stage and on demand: manifest hashes first, one selected verified Blob pair
      second; never scan the Workspace or persist derived patches.
- [ ] Keep the external handoff rollback path until the replacement reaches measured parity.

## Validation commands

Run the smallest affected checks during each phase, then the full listed source checks once on the
final implementation head:

```text
cargo fmt --all -- --check
cargo test -p canisend-store
cargo test -p canisend-app
cargo test -p canisend-gui
cargo clippy -p canisend-store -p canisend-app --all-targets -- -D warnings
cargo clippy -p canisend-gui --all-targets -- -D warnings
pnpm --dir apps/canisend-desktop format:check
pnpm --dir apps/canisend-desktop check
pnpm --dir apps/canisend-desktop test
pnpm --dir apps/canisend-desktop build
cargo run -p xtask --locked -- release check
```

The native macOS App Server smoke and any packaging qualification remain separately named evidence; they
are not inferred from these source checks.

## R5 — Generic ACP channel and Claude qualification, after MVP

Start only after R4 evidence exists.

- Recheck ACP v1 and the Registry at implementation time; pin the implemented baseline and keep
  Registry installation/update out of the first slice.
- Add a deterministic fake ACP process covering initialize/auth, capability negotiation,
  session new/resume/prompt/update/cancel/close, permission requests, malformed/oversized messages,
  timeout, and process exit.
- With Codex App Server and ACP both concrete, extract only their shared lifecycle contract and keep
  wire-specific types at their respective Rust boundaries.
- Advertise the minimum safe ACP client capabilities. Do not expose general filesystem writes or a
  terminal; reject an Agent whose required capabilities exceed the host policy.
- Validate the selected Claude Agent ACP adapter against current official Claude Agent SDK/CLI
  sessions, streaming, permissions, MCP behavior, authentication, distribution, terms, branding,
  provenance, packaging, and native behavior.
- Add provider selection without provider-specific branches in product workflows, approval logic,
  storage, or trace records.
- Prove that disabling or removing the ACP channel leaves Codex sessions, Workspaces, and external
  handoff unchanged.

Exit: the Workbench runs one qualified Claude Agent session through generic ACP with the same safe
CanISend MCP review loop as Codex, and rejects incompatible agents without mutation.

## R6 — Legacy removal, after parity

Start only after the Workbench has measured parity and a rollback window.

- Delete the one-shot in-App provider runtime.
- Remove obsolete legacy Agent/Job/Task/Workflow panels and callbacks from the primary UI.
- Retire old routes only after saved links and recovery paths are handled.
- Reduce root `App.svelte` and bridge wiring where the Workbench has made it redundant.
- Reconcile operation registry, documentation, tests, and architecture records.

## Explicitly skipped

- Electron migration — add only after the preview gate in `design.md` fails.
- PDF.js — add only when users need controls the exact-byte iframe does not provide.
- Auto-download/update or bundling of agent runtimes — add only when distribution is approved and provenance can
  be verified.
- Automatic ACP Registry installation and a promise to support every listed Agent — add only after
  per-agent qualification and a signed distribution/update policy exist.
- Multi-agent/concurrent sessions — add only after a single session is reliable and demand is
  measured.
- Transcript storage — add only with a retention, encryption, migration, export, and deletion policy.
- Git-backed Workspaces, automatic commits/pushes, and repository sync — add only as a separately
  approved, opt-in private projection with conflict and recovery contracts.
- Arbitrary Workspace file scanning/watching — add only as a separately approved product with path,
  privacy, symlink, concurrency, retention, and recovery contracts.
- Rename inference, whitespace-ignore, intraline highlighting, blame, merge, and an editor framework
  — add only after the bounded unified view has a measured usability gap.
- General file/terminal access, rich document editing, and automatic submission — outside the product
  safety boundary for this roadmap.

## Implementation handoff

After approval, begin with R0 only. Create just-in-time Trellis child tasks for the current phase;
do not scaffold R1–R6 in advance. Each implementation phase must retain its smallest positive and
negative regression and stop at its exit criterion.
