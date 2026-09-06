# Directory Structure and Ownership

> How product code and authority are organized.

---

## Overview

CanISend is a Rust workspace with inward-facing product boundaries and a Tauri/Svelte presentation
adapter. ADR-RN-0019 and `docs/architecture/rust-native/workspace-dependency-policy-v1.json` own
the exact dependency graph.

## Directory Layout

```
crates/
├── canisend-contracts/   versioned public types and schemas
├── canisend-core/        storage-independent Pack and domain rules
├── canisend-resources/   verified embedded Packs, schemas, templates, and Agent assets
├── canisend-io/          bounded parsers, network adapters, PDF, and rendering
├── canisend-store/       SQLite, immutable Blobs, revisions, recovery, and projections
├── canisend-app/         shared product use-case facade
├── canisend-mcp/         guarded MCP adapter over canisend-app
├── canisend-cli/         Clap/process adapter and MCP stdio entrypoint
└── canisend-desktop/     Tauri command boundary and unified native host
apps/canisend-desktop/    Svelte 5 desktop presentation
xtask/                    repository and release automation, not product runtime
docs/                     ADRs, contracts, guides, evidence notes, and the Master Roadmap
release/                  machine-readable release and qualification authority
```

## Module Organization

- Put versioned public JSON types and operation identity in `canisend-contracts`.
- Put domain rules and port traits in `canisend-core`.
- Put concrete SQLite/blob behavior in `canisend-store`; adapters must not bypass `canisend-app`.
- Put bounded external input and rendering behavior in `canisend-io`.
- Keep MCP, CLI, Tauri, and Svelte as adapters; do not create a host-specific workflow engine.
- Add repository/release automation to `xtask`, not the product crates.

## Scenario: compose rendering without a Store-to-IO production edge

### 1. Scope / Trigger

Use this boundary whenever Store-owned render, projection, export, repair, or restore code needs
Typst projection, compilation, or PDF validation.

### 2. Signatures

- `canisend_core::RenderExecutor` owns `project_document`, `render_pdf`, `validate_pdf`,
  `project_deliverable`, and the default `render_document` composition.
- Store entrypoints accept `&mut impl RenderExecutor` explicitly.
- `canisend_io::EmbeddedTypstCompiler` implements the Core trait; `canisend-app` constructs it.

### 3. Contracts

- Core carries verified records, content bytes, `RenderError`, and bounded output metadata.
- IO owns templates, projection rules, compilation, PDF parsing, and concrete limits.
- Store owns SQLite, immutable Blobs, paths, revision rechecks, audit, and recovery.
- Application export rechecks the exact Application revision and Pack binding after every executor
  call has completed and before publishing the create-new file batch.
- Store owns create-new export publication: a failed batch removes only files and directories
  created by that attempt before any export audit is written.
- Public operations, receipts, schemas, and Workspace formats do not expose the executor.

### 4. Validation & Error Matrix

| Condition | Owning result |
|---|---|
| Unresolved document fields | `StoreError::TemplateFieldsUnresolved` |
| Invalid projection input/invariant | Existing Store invalid-input or projection-invariant class |
| Compile, malformed/encrypted PDF, size, or time failure | `StoreError::EmbeddedRender(RenderError)` |
| Export file write fails | Typed Store IO error; no partial files or newly created directory chain remains |
| Application revision changes during rendering | Existing Application conflict; no export files or export audit |
| Restore projection fails | Staging directory is discarded; destination is not replaced |

### 5. Good / Base / Bad Cases

- Good: App injects one compiler and Store commits only after projection/render validation.
- Base: Store tests inject a fake executor to prove failure and stale boundaries.
- Bad: Store imports IO in production or constructs a fallback compiler internally.

### 6. Tests Required

- IO: project, compile, and validate through `RenderExecutor`.
- Store: success plus renderer/projector failure, invalid PDF, stale commit, Blob leftovers, and
  repair convergence without partial authority; inject a later export-file failure and assert
  earlier files plus the new directory chain are removed before audit; advance the Application
  from an executor callback and assert the post-render recheck rejects it before files or audit.
- App: both built-in Packs export, and Workspace repair/restore uses the concrete adapter.
- Architecture: locked actual and target graphs match with no temporary Store/IO exception.

### 7. Wrong vs Correct

```rust
// Wrong: persistence selects a concrete adapter.
let compiler = canisend_io::EmbeddedTypstCompiler::new();

// Correct: the App composition root supplies the Core port.
store_operation(..., &mut executor)?;
```

## Naming Conventions

Rust modules and files use `snake_case`; public types use Rust's `UpperCamelCase`. Versioned
contracts use explicit suffixes such as `V4` when the version is part of the public boundary.
Migrations are append-only numbered SQL files under `crates/canisend-store/migrations/`.

## Scenario: CLI project/global Agent Skills scope

### 1. Scope / Trigger

Use this contract whenever a CLI host command selects where managed Agent v4 Skills are installed,
inspected, or removed.

### 2. Signatures

- `canisend --workspace PATH host setup|status --host HOST [--scope project|global]`
- `canisend --workspace PATH host remove --host HOST [--scope project|global]`
- The CLI maps its Clap value to `AgentSkillsInstallRequest { host, workspace, scope }` and calls
  the existing `canisend-app` install, status, or uninstall operation.

### 3. Contracts

- `project` is the CLI default and resolves to the Workspace root.
- `global` resolves to the current user home through `canisend-app`; Unix reads `HOME` and Windows
  reads `USERPROFILE`.
- JSON results report `data.scope` as `project` or `global`; operation IDs and CLI leaf inventory
  do not change.
- MCP registration guidance remains bound to the selected Workspace. Neither scope overwrites host
  MCP configuration, and the CLI never writes `.canisend` directly.

### 4. Validation & Error Matrix

| Condition | Result |
|---|---|
| Scope omitted | Project installation is used |
| Unknown scope | Clap usage error before Workspace access |
| Global scope without a user home | App-owned invalid-input failure before Skill mutation |
| Unsupported legacy or unmanaged resources | Resource failure before managed-file mutation |
| Modified manifest-owned file during removal | Removal fails before deleting any managed file |

### 5. Good / Base / Bad Cases

- Good: setup, status, and removal use the same explicit global scope and isolated user home.
- Base: omitted scope preserves the existing project-local behavior.
- Bad: the CLI computes host directories itself or setup and removal silently use different roots.

### 6. Tests Required

- CLI parse regression for the project default and explicit global status/removal.
- Binary contract with an isolated `HOME`/`USERPROFILE`, asserting global files never land in the
  Workspace and the response reports the selected scope.
- Packaged host smoke for starter resources, project/global lifecycle, host-config non-mutation,
  and unsupported-legacy refusal.
- App/Resources owner tests retain missing-home, drift, user-modified, and safe-uninstall coverage.

### 7. Wrong vs Correct

```rust
// Wrong: the adapter selects a host directory and bypasses the shared facade.
install_embedded_agent_skills(host, &home.join(".agents"))?;

// Correct: the adapter passes typed intent to the existing application operation.
Application::install_agent_skills(&AgentSkillsInstallRequest { host, workspace, scope })?;
```

## Scenario: desktop Agent v4 Workspace handoff

### 1. Scope / Trigger

Use this contract when the desktop **Prepare AI workspace** action prepares an external Codex,
Claude, or generic-host handoff.

### 2. Signatures

- Tauri request: `prepare_agent_handoff { request: { host, workspace } }`.
- App facade: `Application::prepare_agent_handoff(&AgentHandoffRequest { host, workspace })`.
- The response identifies `canisend.agent/v4`, an `AgentContextBindingV4`, the
  `canisend-workspace` Skill, persistent MCP integration, and ordered `next_actions`.

### 3. Contracts

- The App facade opens the Workspace through `workspace_status_v4`; adapters do not synthesize or
  downgrade the binding.
- Handoff orientation starts with `canisend_workspace_status`, then
  `canisend_application_list`; Application selection and revision binding happen through Agent v4.
- The payload contains no legacy Agent context, assistance, Job, Task, or Workflow command fields.
- The handoff remains body-free even when the Workspace contains private-local Profile Sources.
- The Agent screen treats an empty selected-Job prop as an explicit Workspace scope. It must clear
  any retained Job-era conversation scope before loading the runtime catalog.
- The desktop runtime resolver identifies clean v4 through `workspace_status_v4` and rejects a
  supplied legacy Job selector before `job_detail`. Only an explicitly detected v2/v3 Workspace
  may enter the labelled legacy resolver path.

### 4. Validation & Error Matrix

| Condition | Result |
|---|---|
| Clean Workspace v4 | Body-free Agent v4 handoff is returned |
| Pre-v4 Workspace | Shared compatibility-unavailable failure before handoff generation |
| Missing or invalid Workspace | Existing typed Workspace open failure |
| Private-local source exists | Metadata-only handoff; source body is not serialized |
| Clean v4 plus retained legacy Job selector | Screen clears it; direct runtime input fails as `input-invalid` before legacy lookup |

### 5. Good / Base / Bad Cases

- Good: desktop installs v4 Skills and asks the App facade for one Workspace v4 handoff.
- Base: copy regenerates the same handoff and copies only an allowlisted command or prompt field.
- Bad: handoff generation calls `agent_context`, `agent_assistance`, or advertises their CLI forms.

### 6. Tests Required

- App regression: a clean Workspace v4 with a private-local sentinel prepares the handoff, reports
  the v4 protocol/actions, and never serializes the sentinel or legacy commands.
- Negative App regression: a legacy Workspace fails with `CompatibilityUnavailable`.
- Tauri and TypeScript bridge tests: `{ host, workspace }` crosses the boundary without a legacy
  Job selector, and clipboard requests remain field-allowlisted.
- Agent screen/runtime regressions: an empty current selection clears retained Job state before the
  runtime catalog loads; direct v4 runtime scope never invokes `job_detail`; explicit pre-v4 Job
  scope retains its labelled behavior.

### 7. Wrong vs Correct

```rust
// Wrong: a Workspace v4 action enters the retired compatibility surface.
let context = Application::agent_context(Some(&workspace), selected_job_id)?;

// Correct: the App facade derives one exact Workspace v4 authority binding.
let workspace = Application::workspace_status_v4(&workspace)?.data;
```

## Scenario: desktop Codex App Server session transport

### 1. Scope / Trigger

Use this boundary when the desktop starts, resumes, streams, cancels, or reports a Codex
conversation. R1 is transport-only: it does not grant Workspace access or inject CanISend MCP
tools.

### 2. Signatures

- `start_agent_session(window, state, AgentSessionStartRequest) -> AgentEmbeddedSession`
- `run_agent_turn(window, state, AgentTurnRequest, Channel<AgentStreamEvent>) -> AgentTurnResult`
- `agent_runtime_catalog(window, state, AgentRuntimeCatalogRequest) -> AgentRuntimeCatalog`
- `cancel_agent_turn(window, state, AgentTurnCancelRequest) -> AgentTurnCancelResult`
- The App-local registry is `canisend.agent-session-registry/v3`; v1/v2 loads migrate in memory
  and the next successful mutation writes v3. Legacy Job IDs never become Application IDs.

### 3. Contracts

- Reuse executable/version discovery, then spawn exactly
  `codex app-server --listen stdio://` with process-level policy overrides, without a shell,
  with piped stdio and an App-owned private
  working directory outside the Workspace. One exact child is owned per desktop window.
- Speak bounded newline-delimited App Server JSON-RPC: `initialize`, `initialized`, `account/read`,
  `thread/start` or `thread/resume`, `turn/start`, and `turn/interrupt`.
- Thread start/resume/turn select the pre-initialization named permission profile,
  `approvalPolicy: "never"` and `approvalsReviewer: "user"`. The versioned policy and its remaining
  inherited-configuration gates are described below. No CanISend MCP is injected.
- `AgentStreamEvent` carries only a monotonic sequence, desktop session/turn IDs, finite status,
  assistant delta, body-free method name, and bounded provider event ID. Raw provider payloads,
  approval arguments, stderr bodies, prompts, and transcripts do not cross the IPC boundary.
- Runtime requests carry separate optional `selected_application_id` and legacy `selected_job_id`.
  The shared resolver requires an existing Application in Workspace v4 and rejects mixed identities.
  Catalog filtering, active leases, start/resume, persistence and cancellation use the same scope.
- Registry v3 stores only runtime/version, Workspace/Application or legacy Job scope, desktop/provider session and turn IDs,
  finite last status, and timestamps. The Svelte reducer keeps conversation bodies in memory and
  reconciles the final response into the same streamed assistant message. Switching Workspace,
  Application or provider clears send confirmation; a conversation epoch drops stale catalog,
  start, result and stream updates even when the user returns to the original Application.
- Keep Claude on the existing bounded one-shot fallback until it has a separately qualified
  embedded-session protocol.

### 4. Validation & Error Matrix

| Condition | Result |
|---|---|
| Missing executable or version | `agent-runtime-unavailable` or `agent-runtime-incompatible` |
| `account/read` requires login with no account | `agent-runtime-authentication-required` |
| Malformed/oversized JSONL, wrong correlation, timeout, or child exit | Exact child is closed; state becomes recoverable or failed |
| Command/file approval request | Emit only its method/ID and reply `decline` |
| Any other server request | Reply `-32601`, close the connection, and report incompatible |
| Cancel matching active scope | Send exact thread/turn IDs; retain received partial text |
| Clean Workspace v4 plus Job selector | `input-invalid` before any legacy Job lookup |

Protocol bounds are 1 MiB per line, 16 queued messages, 4 MiB per response, 256 KiB retained
stderr, 10 seconds startup, 30 seconds per ordinary request, 10 minutes per turn, 5 seconds to
interrupt, and 2 seconds graceful child shutdown.

### 5. Good / Base / Bad Cases

- Good: one live Codex child resumes its stored thread and streams ordered deltas into one message.
- Base: missing or signed-out Codex leaves external handoff available; Claude still uses one-shot.
- Bad: run Codex once per turn, expose raw server request parameters, persist transcript bodies, or
  point the App Server working directory at the Workspace.

### 6. Tests Required

- Registry: v1 fixture preserves the thread ID, writes v2, and contains only the allowlisted keys.
- Fake child: initialize/readiness, start/resume, two turns, multiple deltas, cancellation,
  approval decline, unsupported request, malformed/oversized framing, timeout, child exit, exact
  cleanup, and body-free errors.
- Desktop: clean-v4 scope refusal, exact-scope cancellation, strict Clippy, and all library tests.
- Frontend: typed Channel request, ordered/duplicate delta reducer, Svelte type-check, tests, build,
  formatting, and an `aria-live` finite status.
- Manual evidence: one signed-in supported Codex CLI starts, resumes, restarts, and cancels without
  recording response bodies.

### 7. Wrong vs Correct

```rust
// Wrong: give the provider ambient Workspace access and forward arbitrary protocol JSON.
Command::new("codex").current_dir(workspace).spawn()?;

// Correct: isolate transport and expose one normalized, body-free event boundary.
Command::new(executable)
    .args(["app-server", "--listen", "stdio://"])
    .current_dir(app_owned_session_directory)
    .spawn()?;
```

## Examples

- `crates/canisend-app/src/error.rs` centralizes adapter-neutral failure classification.
- `crates/canisend-store/src/database.rs` owns schema configuration and append-only migrations.
- `crates/canisend-mcp/src/lib.rs` exposes guarded tools through the shared application facade.
- `apps/canisend-desktop/src/lib/bridge.ts` keeps the Svelte side at the typed IPC boundary.


### R2a optional MCP Application binding

`canisend mcp serve --application APPLICATION_ID` reuses `canisend-mcp`'s server and the existing App
facade. `open_with_application` validates the existing Application before startup. Every handler
with an Application ID uses the instance parser before facade/broker work. The four no-ID tools
and two association-list tools require unbound Workspace scope; the catalog remains unchanged.
Mismatched IDs return `application.binding-mismatch`; Workspace-wide results return
`application.workspace-scope-required` as JSON-RPC invalid-params errors. Existing `open` and
`serve_stdio` remain unbound compatibility entry points. Binding confers no consent authority.

The primary cross-Pack boundary check is
`canisend-cli/tests/mcp_protocol.rs::application_binding_covers_every_tool_and_preserves_unbound_discovery`.
The existing desktop runtime owns process-level named permissions before initialize and on
start/resume/turn, literal canonical session/installation read roots, no command networking, and
execution/hooks/plugins/apps feature overrides. The experimental policy is version-gated to
`codex-cli 0.152.0`; rejection must never fall back to legacy read-only permissions. The existing
fake protocol fixture owns bootstrap arguments, alias paths, policy rejection and unexpected
provider-request rejection. Positive real-provider consent and effective isolation remain R2a enablement gates.

The opt-in `scripts/probe_codex_app_server.py --codex /absolute/path/to/codex` owns exact provider
capability observations. It requires Python 3.11+, uses a disposable child configuration home and
loopback model plus stdio MCP with synthetic data, and emits only a body-free report. Do not add it
to Fast CI or treat it as authenticated/native release qualification. The 0.152.0 named-permission
probe requires the canonical executable path for both launch and filesystem rules; its observed
MCP elicitation metadata has no item ID, so production correlation must reject ambiguous matches.
Its explicit `--installed-account` mode only checks existing account availability with process-level
restrictions and no model turn; account recognition is separate from full signed-in flow qualification.
