# Stage 4G connected agent workspace execution plan

**Status:** G1 through G7 implemented and locally package-qualified on macOS for Alpha.5; public
five-target qualification and publication remain separately authorized release work

**Decision date:** 2026-07-30

**Baseline:** `v1.0.0-alpha.4` at `b42817e812a4444dbcd2dd9f5c4c3c2ed50a96a9`

## 1. Outcome

Turn the Svelte desktop from a collection of CLI-shaped screens into a connected control plane
that can:

- hand the selected workspace and optional job to Codex or Claude without copying private bodies;
- keep full conversation, search, plugins, skills, MCP, and semantic reasoning in the chosen host;
- keep CanISend authoritative for data, validation, revisions, workflow, rendering, and export;
- discover the user's installed Codex and Claude runtimes for handoff status and an optional
  read-only convenience bridge;
- initialize a revisioned local profile and accept future agent-proposed job intake from a URL or
  PDF;
- present every proposed mutation as a typed preview that the user approves before commit; and
- keep CanISend, not the agent or an editable file projection, authoritative for application state.

Stage 4G does not replace Agent v2 or turn CanISend into a general-purpose agent client. It makes
Agent v2, the CLI, and the planned local MCP adapter easier to use from full-featured external hosts.

## 2. Architectural decision

Use an external-host-first architecture. CanISend is the control plane and system of record; Codex,
Claude, or another host is the reasoning plane. The in-App runtime bridge is optional and read-only.
Do not begin with direct provider APIs or make a rich embedded chat client a release dependency.

| Concern | External Codex/Claude host | In-App local bridge | Direct model API |
| --- | --- | --- | --- |
| Existing user login | Reused by the host | Reused by the host CLI | Separate credential flow required |
| Conversation continuity | Native host session | Resume host-owned ID | CanISend must own history |
| Search/MCP/skills/plugins | Full host surface | Only what that CLI exposes | Must be reimplemented |
| Transcript | Owned by host | Owned by host | Owned by CanISend |
| Product priority | Primary | Optional convenience | Deferred |

The integration has four layers:

1. `AgentHandoff` generates a body-free working-directory command, starting message, capability
   command, and job-scoped context command for the external host.
2. Agent v2 and the planned `CanISendMcpAdapter` expose the Rust application facade without
   duplicating business rules.
3. The optional `AgentRuntimeAdapter` discovers, probes, starts, resumes, and parses one local host
   runtime for read-only inspection.
4. `AgentSessionRegistry` binds `(workspace, runtime, optional job)` to a validated external session
   ID without storing conversation bodies.

## 3. Session-continuity model

### Durable authority

The external Codex or Claude host owns its transcript and session continuity. The recommended
handoff stores no session data in CanISend. Only the optional in-App bridge stores:

- canonical workspace path;
- runtime kind;
- optional validated job ID;
- external thread/session ID; and
- created/updated timestamps.

The macOS registry is:

```text
~/Library/Application Support/CanISend/agent-sessions.json
```

It is size- and entry-bounded, atomically replaced, private-permissioned, and rejected if redirected
through a symlink. Prompts, responses, provider tokens, tool results, and imported document bodies
are forbidden from this registry.

### Scope rules

- Workspace and job conversations have distinct bindings.
- Codex and Claude never share a binding.
- Switching App screens keeps in-memory messages and selection.
- Reopening the App restores the binding and can resume the host transcript; it does not recreate a
  locally stored transcript view.
- `New conversation` starts a new host session and replaces the binding only after a successful
  first turn.
- Only one turn may run for the same workspace/runtime/job scope at a time.

This avoids duplicating host history while still preserving continuity. CanISend will not add a
local transcript cache unless a later retention, deletion, encryption, and redaction design proves
it necessary.

## 4. Runtime behavior

### External-host handoff

- Validate and canonicalize the selected CanISend workspace and optional job.
- Generate a safely shell-quoted working-directory launch command.
- Generate a body-free bootstrap message defining CanISend as state authority.
- Direct the host to exact `agent capabilities` and `agent context` commands.
- Display the authority boundary and copy each handoff field through the native clipboard.
- Do not launch a provider, contact a network, export private bodies, or create a transcript.

### Optional Codex bridge

- Discover `codex` through inherited `PATH`, common GUI-safe user locations, Homebrew/system
  locations, or the ChatGPT App resource command.
- Treat executable and version discovery as the only observed evidence; never infer sign-in or
  host-tool configuration from availability.
- Start with `codex exec --json --sandbox read-only`.
- Resume with `codex exec --sandbox read-only resume --json SESSION_ID`.
- Pass the private request over stdin.
- Parse JSONL for thread ID, final message, errors, and visible tool-activity categories.
- Inherit normal Codex configuration, including CLI-exposed MCP, skills, plugins, and search.

### Optional Claude bridge

- Discover `claude` through the same bounded policy.
- Run print mode with JSON output and `plan` permission mode.
- Resume with the validated external session ID.
- Send the CanISend request over stdin.
- Use the normal runtime rather than `--safe-mode` or `--bare`, so CLI-exposed configuration remains
  available.

## 5. Safety and data-integrity invariants

- Preparing a body-free handoff requires no provider-send consent and contacts no provider.
- Every optional in-App turn requires explicit provider-send consent.
- Show runtime path, version, read/write mode, workspace, job scope, and session state before send.
- Use fixed executable discovery and fixed arguments; never invoke a shell or accept arbitrary
  executable text.
- Send prompt bodies through stdin so they do not appear in the process argument list.
- Bound prompt, stdout, stderr, duration, and concurrent scope use.
- Treat URL/PDF/job/profile content as untrusted data, never agent instructions.
- Never let an agent edit `.canisend`, SQLite, immutable blobs, or managed projections directly.
- Keep the optional in-App runtime bridge read-only; guarded MCP writes use the typed preview broker.
- A write proposal must follow `prepare -> preview -> consent -> facade call -> refreshed state`.
- CanISend never submits an application.

## 6. Execution sequence

### G1 — Runtime and session foundation

- [x] Add typed `codex` and `claude` runtime kinds.
- [x] Add bounded discovery and version probes for a packaged macOS GUI environment.
- [x] Add a body-free, scope-keyed, persistent agent-session registry.
- [x] Add process timeout, bounded output parsing, and one-turn-per-scope leasing.
- [x] Keep prompts out of command-line arguments.

**Exit:** The App can identify a local runtime and safely preserve its external session binding.

### G2 — Connected Agent screen

- [x] Add workspace/job/runtime/session status in one visible flow.
- [x] Keep selected runtime, selected job, draft prompt, consent, and rendered messages across tab
  switches.
- [x] Add new-conversation and resume behavior.
- [x] Display runtime path/version and observed event/tool activity.
- [x] Retain body-free context, capabilities, contract inspection, and host-pack export.
- [x] Provide English and Simplified Chinese copy with visible focus and live status.

**Exit:** A user can understand what the agent can see, where the conversation lives, and whether it
will resume.

### G3 — First-run continuity repairs

- [x] Distinguish CLI `PATH` active, configured-but-new-terminal-required, and missing states.
- [x] Add an explicit, idempotent `Add to PATH` action using a managed shell-profile block.
- [x] Add a no-source profile initializer with a useful Markdown scaffold.
- [x] Store initialized profiles through immutable blobs and revisioned SQLite metadata.
- [x] Explain profile storage and require explicit confirmation.

**Exit:** First-run users can install the CLI correctly and create an authoritative local profile
without preparing a separate import file.

### G4 — External-host handoff and product-boundary correction

- [x] Add a typed, body-free `agent.handoff.prepare` application operation.
- [x] Generate safely quoted launch, capabilities, and job-scoped context commands.
- [x] Generate a bootstrap message that keeps state in CanISend and reasoning in the host.
- [x] Make external Codex/Claude handoff the default Agent screen.
- [x] Add native copy actions and visible control-plane/reasoning-plane ownership.
- [x] Move the existing process-per-turn conversation into an optional read-only tab.
- [x] Preserve Host Pack export and Agent v2 inspection as secondary portable tools.

**Exit:** A user can enter the full host with the correct workspace and instructions without
exposing private bodies or mistaking CanISend for the conversation authority.

### G5 — Portable CanISend MCP/tool adapter

- [x] Expose a small, versioned CanISend MCP/tool surface backed by the Rust application facade.
- [x] Begin with read-only tools: capabilities, context, workflow status, job list/detail, and
  profile-source metadata.
- [x] Generate copyable Codex and Claude configuration snippets from the desktop.
- [x] Add protocol-version negotiation, deterministic tool ordering, bounded input/output, and
  malformed-request fixtures.
- [x] Keep MCP as a transport adapter; it cannot own or duplicate business rules.

The adapter lives in the isolated `canisend-mcp` crate but is invoked as
`canisend --workspace PATH mcp serve`. Reusing the shipped CLI avoids a second statically linked
application binary in the macOS App while keeping MCP protocol code outside the application facade.

**Exit:** Codex and Claude can inspect the same authoritative state through one portable,
versioned, read-only tool surface.

### G6 — Guarded link/PDF intake and task mutations

- [x] Add `job intake preview` for URL or local PDF supplied through the conversation.
- [x] Show source, extracted fields, provenance, validation issues, and intended mutations before
  confirmation.
- [x] Commit through existing URL/PDF/job services only after user approval.
- [x] Add task prepare/input/complete operations; preserve Agent v2 consent and stale-revision rules.
- [x] Export the same tool server for Codex and Claude so in-App and open-in-host flows behave alike.

The desktop and MCP adapters hold the exact prepared source in bounded Rust memory. URL content is
not fetched again during commit. Tokens are opaque, capped, single-use, and rejected when the job
revision changes. The Svelte Applications screen now shows body-free provenance, extraction counts,
validation notices, intended mutations, and explicit commit/discard actions. MCP expands from the
initial six read-only tools to thirteen tools: nine read-only inspection/preview tools and four
host-approval-gated mutations. Task completion preserves its existing preview-token, lease,
revision/hash, and candidate-schema checks.

**Exit:** “Here is a job link/PDF” becomes a guided, previewable intake flow without direct agent
writes to workspace internals.

### G7 — Whole-App information architecture and Alpha.5 qualification

- [x] Add a persistent workspace/job/context header shared by every workflow screen.
- [x] Turn screen navigation into a stage-based next-action flow rather than independent tabs.
- [x] Preserve selected job and last successful action globally.
- [x] Add deep links from agent proposals to the exact criteria/profile/document/review screen.
- [x] Run automated packaged macOS first-run, bundled CLI, documented workflow, external-host
  handoff, accessibility, update/rollback, uninstall, and workspace-retention dogfood.
- [x] Run packaged PATH repair, localized profile initialization, tab switching, route/locale
  restart, exact-scope local Agent cancellation, and cross-restart host-session resume with
  bounded local fixtures.
- [ ] Run consented real-account Codex and Claude turns through their normal local configuration.
- [x] Update limitations, privacy guidance, release notes, and native qualification fixtures.
- [ ] Qualify a new sequential Alpha only after exact packaged-byte tests pass.

The Svelte shell now owns one bounded `canisend.desktop.navigation/v1`-equivalent local memory
record for active route/detail, canonical workspace path, selected public job ID, and latest
body-free action summary. The workspace remains authoritative and every selection is reloaded
through the Rust facade. A six-stage rail and deterministic recommendation map connect discovery,
intake, profile, workflow, external Agent collaboration, and delivery. Workflow stages, Agent
context actions, and completed task operation kinds map to exact deep-link targets. English and
Simplified Chinese, dark mode, the 960 px macOS minimum window, keyboard focus, and restart route
restoration are covered by source/browser checks. The current Alpha.4-versioned source build also
passed local ZIP, DMG, launch, accessibility, external-host, CLI lifecycle, retention, PATH
repair, profile initialization, exact-scope cancellation, route/locale restart, and exact
cross-restart session-resume gates. The fixed runtime proves the App resumes the recorded
host-owned session ID without contacting a provider. This is local pre-release evidence, not
clean-tag Alpha.5 qualification. Consented real-provider dogfood, followed by exact clean-tag
Alpha.5 CI, remain the next gates.

**Exit:** The App has one connected workflow and the agent is an integrated collaborator, not an
isolated export tab.

## 7. Verification strategy

Use the repository verification tiers:

1. Focused Rust tests for the app/desktop crates, Svelte check/unit/build, relevant Clippy, and
   `git diff --check`.
2. Source gate after G5/G6 protocol or contract changes.
3. Native macOS package gate before the next Alpha.
4. Scheduled dependency, fuzz, signature, provenance, and five-target CLI assurance only at their
   owned gates.

Do not execute a real provider turn in automated tests. Use fixed local fake runtimes. Real-account
dogfood is manual because it can transmit workspace context and consume the user's provider
allowance.

## 8. Deferred decisions

- Direct OpenAI or Anthropic API adapters remain optional. They cannot inherit consumer-host
  conversations or plugins and require a separate credential, billing, history, retention, and
  tool-governance design.
- Persisting a duplicate transcript in CanISend is deferred.
- A long-running Codex App Server or Claude stream client is deferred until external-host handoff
  and the portable MCP adapter are proven. It remains an optional convenience, not the core.
- Write-capable arbitrary shell access is out of scope.
- Windows desktop runtime discovery and packaging follow the macOS-qualified flow later.
- Application submission and browser automation remain out of scope.
