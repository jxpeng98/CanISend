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

## Scenario: desktop local-file intake and canonical Application context

### 1. Scope / Trigger

Use this contract when a Svelte desktop view selects or drops a local source, or displays the
current Application and Application count.

### 2. Signatures

- `chooseApplicationSource() -> Promise<string | null>` selects one supported path.
- `localFileDrop(node, options)` maps one native Tauri WebView drop to the same path field.
- `listGenericApplications(workspace) -> ActionReceipt<StoredApplicationModelV3[]>` supplies the
  canonical desktop collection for both built-in Packs.
- `WorkspaceStatus.application_count` is the persisted total; `canisend_io::read_local_text(path)`
  owns bounded text-type validation.

### 3. Contracts

- Application intake accepts PDF, Typst, plain text, `.md` / `.markdown`, and JSON. Profile intake
  accepts the same text formats without PDF. Picker filters and drop filters must stay aligned with
  the IO adapter.
- Drag and drop selects exactly one path. It does not read bytes, grant consent, or mutate the
  Workspace; preview and commit keep the existing consent and validation boundary.
- The original file stays in place. Commit stores the authoritative Application in
  `.canisend/state.sqlite3` and verified source bytes under `.canisend/blobs/sha256`; no editable
  duplicate is implied. Managed exports belong under `applications/APPLICATION_ID/`, not `jobs/`.
- Desktop selection, summaries, and counts use the v4 Application collection and
  `application_count`. `JobRecord`, `job_count`, and `jobs/` remain legacy Academic compatibility
  surfaces and must not be synthesized for a new Application.

### 4. Validation & Error Matrix

| Condition | Owning result |
|---|---|
| Drop contains zero or multiple paths | Localized UI rejection; existing selection is preserved |
| Dropped extension is not accepted by that intake | Localized UI rejection before preview |
| Native drop listener is unavailable | Localized fallback message; file picker remains usable |
| Path is a directory, symlink, oversized, unsupported, or invalid UTF-8 | Typed IO/Application preview failure; no mutation |
| Private-read consent is absent | Existing consent-required failure before source commit |

### 5. Good / Base / Bad Cases

- Good: drop one `.typ`, review the preview, commit it, and immediately see the new record in the
  Application selector and canonical counts.
- Base: choose the same file with the native picker and follow the identical preview/commit path.
- Bad: call `createJob`, increment `job_count`, copy the original beside the Workspace, or write a
  `jobs/` projection to make a v4 Application appear present.

### 6. Tests Required

- UI unit regression: inside-target supported drop, multiple-file rejection, unsupported-type
  rejection, and listener cleanup.
- IO regression: every advertised text extension remains bounded and maps to its expected kind.
- Desktop wiring: Svelte type check, focused source contract, production UI build, and native host
  compile when WebView integration changes.

### 7. Wrong vs Correct

```typescript
// Wrong: a new v4 Application disappears because the shell reads the legacy table.
const applicationCount = activeWorkspace.status.job_count;

// Correct: the shell follows the canonical v4 model.
const applicationCount = activeWorkspace.status.application_count;
const applications = (await listGenericApplications(activeWorkspace.path)).data;
```

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

## Examples

- `crates/canisend-app/src/error.rs` centralizes adapter-neutral failure classification.
- `crates/canisend-store/src/database.rs` owns schema configuration and append-only migrations.
- `crates/canisend-mcp/src/lib.rs` exposes guarded tools through the shared application facade.
- `apps/canisend-desktop/src/lib/bridge.ts` keeps the Svelte side at the typed IPC boundary.
