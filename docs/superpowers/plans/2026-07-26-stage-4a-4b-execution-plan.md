# CanISend Stage 4A and 4B execution plan

**Status:** Stage 4A and Stage 4B complete; exact non-publishing Alpha.2 candidate qualified

**Decision date:** 2026-07-26

**Source baseline:** `1.0.0-alpha.2` at `d657710`

**Qualified checkpoint:** `1.0.0-alpha.2` at `d091d14`, native run `30215276643`

**Parent roadmap:** [CanISend 1.0 release roadmap](2026-07-25-1.0-release-roadmap.md)

**Target checkpoint:** a qualified `v1.0.0-alpha.2` candidate after Stage 4B; Beta remains blocked
until the remaining Stage 4 operation families are implemented and frozen.

## 1. Outcome

Stage 4A and 4B convert the current usable Alpha vertical slice into an architecture that can
support the remaining 1.0 GUI workflow without duplicating CLI behavior, then add the recovery and
workflow-control operations needed before the higher-level application-preparation screens.

The ordered outcome is:

1. one typed `canisend-app` application boundary for the CLI and GUI operations in scope;
2. a modular GUI shell whose pages do not call storage or orchestration services directly;
3. unchanged CLI JSON, human-output, error-code, and exit-class contracts for migrated commands;
4. GUI workspace restore and projection repair;
5. GUI workflow begin, complete, and rerun with explicit state, validation, and recovery;
6. `22/35` CLI/GUI parity operation families marked `implemented`; and
7. a macOS development loop that remains below five minutes while native packaging stays
   release-only.

Stage 4A changes structure without adding product capability. Stage 4B changes capability without
changing the workspace schema or persisted contract formats.

## 2. Verified starting point

The plan is based on the following repository facts:

- `canisend-gui` has five pages: Overview, Jobs, Workspaces, Command line, and Diagnostics.
- The GUI already performs workspace create/status/check/backup, job intake, workflow
  start/status, CLI lifecycle, update check, and diagnostics through `canisend-app`.
- `canisend-cli` still calls `canisend-core`, `canisend-io`, and `canisend-store` directly for the
  same product operations.
- `canisend-gui/src/main.rs` contains application state, forms, workers, event reduction, page
  rendering, and dialogs in one file of more than 3,100 lines.
- `Workspace::restore` already verifies a backup, restores through a temporary directory, rebuilds
  managed projections, and atomically renames the restored workspace into a new destination.
- `ProjectionService::repair_all` already rebuilds missing or repair-required projections while
  preserving user-owned edits.
- `WorkflowService` already enforces stage graph, execution-mode, artifact-kind, revision,
  downstream invalidation, and transaction invariants for begin, complete, and rerun.
- The current parity manifest records `17 implemented` and `18 deferred-beta` operation families.
- Focused `canisend-app` and `canisend-gui` tests, relevant Clippy, and the source release check
  pass at the baseline.

The implementation must reuse these store invariants. It must not reproduce recovery or workflow
rules in GUI widgets.

## 3. Scope boundaries

### 3.1 Included

Stage 4A includes:

- roadmap and parity authority reconciliation;
- application receipt and error classification suitable for both CLI and GUI adapters;
- modularization of the current GUI without visible behavior changes;
- a typed worker request/event boundary;
- routing the existing overlapping Alpha CLI families through `canisend-app`;
- parity fixtures that prove the CLI contract did not change; and
- focused CI ownership and timing assertions.

Stage 4B includes:

- `workspace.restore`;
- `workspace.repair`;
- `workflow.begin`;
- `workflow.complete`;
- `workflow.rerun`;
- GUI recovery dialogs and workflow controls;
- English and Simplified Chinese strings and accessible semantics for the new controls; and
- application, CLI-adapter, GUI-state, store-regression, and packaged-app evidence.

### 3.2 Excluded

The following remain outside Stage 4A/4B:

- profile/evidence, criteria, match, application-plan, discovery, agent-task, document, review,
  package, render, schema, and resource GUI surfaces;
- an artifact editor or stage-specific completion form;
- arbitrary shell execution from the GUI;
- automatic application submission;
- automatic update download or installation;
- workspace schema or resource-format changes;
- Windows or Linux GUI publication;
- Developer ID signing, notarization, and paid signing;
- a parallel operation queue or multi-workspace concurrent writers; and
- Stable or Beta publication.

Stage 4B may accept a compatible artifact ID produced by an existing CLI or Agent v2 task. The
stage-specific artifact creation and confirmation screens belong to later Stage 4 work.

## 4. Target architecture

The target dependency direction is:

```text
canisend-cli parser/output ─┐
                            ├─> canisend-app actions/read models/receipts
canisend-gui pages/worker ──┘                 │
                                             ├─> canisend-store
                                             ├─> canisend-io
                                             ├─> canisend-core
                                             └─> canisend-resources
```

Rules:

- GUI pages emit typed requests and render typed read models.
- The GUI worker is the only GUI module that executes `Application` actions.
- CLI command handlers translate parsed arguments into the same typed requests.
- CLI rendering translates an application receipt into the existing Agent v2 envelope and human
  lines.
- Lower layers remain authoritative for validation and persistence.
- The application layer may classify errors and assemble read models, but it must not own a second
  copy of the stage graph, URL policy, backup rules, or projection rules.
- No adapter shells out to the other adapter.

### 4.1 Proposed `canisend-app` layout

```text
crates/canisend-app/src/
├── lib.rs
├── application.rs
├── error.rs
├── receipt.rs
├── system.rs
├── workspace.rs
├── job.rs
├── workflow.rs
├── cli_install.rs
└── update.rs
```

`lib.rs` re-exports the stable application surface. Operation-family modules own request/read-model
types and action implementations. `error.rs` owns one structured classification used by both
adapters.

### 4.2 Proposed `canisend-gui` layout

```text
crates/canisend-gui/src/
├── main.rs
├── desktop.rs
├── state.rs
├── worker.rs
├── i18n.rs
├── registry.rs
├── theme.rs
├── cli_bridge.rs
├── components/
│   ├── mod.rs
│   ├── notice.rs
│   ├── receipt.rs
│   └── workflow_timeline.rs
├── dialogs/
│   ├── mod.rs
│   ├── job.rs
│   ├── source.rs
│   ├── workspace.rs
│   └── workflow.rs
└── pages/
    ├── mod.rs
    ├── overview.rs
    ├── jobs.rs
    ├── workspaces.rs
    ├── command_line.rs
    └── diagnostics.rs
```

`main.rs` only configures and launches `eframe`. `desktop.rs` owns the top-level update/render
cycle. Page modules cannot depend on `canisend-store`, execute a process, or spawn threads.

## 5. Execution order and gates

The implementation is a sequence of small green migrations:

| Order | Work package | Behavior change allowed | Required gate |
|---|---|---:|---|
| A0 | Reconcile tracking authority | No | JSON parse, docs/release check |
| A1 | Normalize application receipts/errors | No CLI/GUI contract change | App tests + CLI snapshots |
| A2 | Split GUI state/pages/dialogs | No visible change | GUI tests + Clippy |
| A3 | Add typed worker request/event boundary | No visible change | Worker reducer tests |
| A4 | Route overlapping CLI families through app | No output change | Binary contract parity |
| A5 | Close Stage 4A evidence | No | Fast CI under five minutes |
| B1 | Add workspace restore/repair app actions | Yes | App/store recovery tests |
| B2 | Add workspace recovery GUI | Yes | GUI reducer/accessibility tests |
| B3 | Add workflow-control app actions | Yes | App/store workflow tests |
| B4 | Route workflow CLI commands through app | No output change | CLI parity tests |
| B5 | Add workflow-control GUI | Yes | GUI state/i18n/accessibility tests |
| B6 | Close Stage 4B and Alpha.2 candidate evidence | No further feature change | Source and native Alpha gates |

No package build, signing, notarization, Windows build, or Linux build belongs in A0–B5. Those
operations run only at B6 when an actual Alpha candidate is requested.

## 6. Stage 4A — architecture and contract alignment

### A0 — Reconcile roadmap and parity authority

Files:

- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md`
- `docs/superpowers/plans/2026-07-19-native-desktop-gui-roadmap.md`
- `docs/contracts/cli-gui-parity-v1.json`
- this execution plan

Tasks:

- Update the parent roadmap source baseline to `1.0.0-alpha.2`.
- Mark the older native GUI roadmap as historical design/detail input, not completion authority.
- Keep the 1.0 release roadmap as stage authority and the JSON parity manifest as operation
  authority.
- Add explicit Stage 4A and 4B status fields without marking implementation complete.
- Record `17/35` as the entry state and `22/35` as the Stage 4B exit target.
- Do not mark an operation `implemented` until its typed app action, GUI path, focused tests, and
  parity evidence all exist.

Exit:

- there is one unambiguous current stage;
- documentation does not claim that Alpha.2 is published; and
- `cargo run -p xtask --locked -- release check` passes.

### A1 — Normalize typed application receipts and failures

Files:

- `crates/canisend-app/src/lib.rs`
- new `application.rs`, `error.rs`, and `receipt.rs`
- `crates/canisend-contracts/src/agent.rs` only if an existing public type must be reused; no
  protocol version change

Tasks:

- Preserve `ActionReceipt<T>` as the typed success result.
- Extend the receipt with adapter-neutral metadata needed by Agent v2 and GUI:
  - artifacts;
  - required consents;
  - warnings; and
  - next actions.
- Use empty defaults so existing application call sites remain straightforward.
- Add builder methods instead of constructing receipt metadata ad hoc in adapters.
- Classify `ApplicationError` into:
  - stable `ErrorCode`;
  - status;
  - retryability;
  - user-safe message; and
  - optional remediation.
- Keep source error chains for diagnostics while preventing private body content from entering
  routine receipts.
- Move current application functions into family modules without changing operation names, data,
  or validation order.
- Add `canisend-core` as an application dependency only where the authoritative stage registry is
  required; do not copy stage descriptors.

Tests:

- receipt serialization omits no required Agent v2 metadata;
- default receipt metadata is empty and deterministic;
- every current `ApplicationError` variant maps to a stable error class;
- errors do not include imported document bodies;
- existing application vertical-slice tests remain green.

Exit:

- CLI and GUI can consume one success and one failure model;
- no persisted format or public Agent protocol version changes; and
- no GUI or CLI output has changed yet.

### A2 — Mechanically split the GUI

Files:

- `crates/canisend-gui/src/main.rs`
- new GUI modules listed in section 4.2

Tasks:

- Move forms and pending-confirmation state into `state.rs`.
- Move `CanISendDesktop` and the `eframe::App` implementation into `desktop.rs`.
- Move each existing page without changing labels, order, layout, actions, or accessibility
  semantics.
- Move reusable notice, receipt, and timeline rendering into components.
- Move job/source/workspace forms into dialogs.
- Keep registry, theme, CLI discovery, and i18n behavior unchanged.
- Keep one active background operation; do not add concurrency during the refactor.
- Keep all Application calls behind the worker boundary.
- Ensure `main.rs` contains launch/configuration only.

Tests:

- existing preference, registry, AccessKit, focus, i18n, and synthetic reopen tests pass;
- every page remains reachable from `Page::ALL`;
- page accessible labels remain stable in both languages;
- no page module imports `canisend-store`, `std::process::Command`, or thread-spawn APIs.

Exit:

- adding a new page or dialog does not require editing a 3,100-line file;
- there is no visible behavior change; and
- the GUI still builds as one native binary.

### A3 — Introduce the typed GUI worker boundary

Files:

- `crates/canisend-gui/src/worker.rs`
- `crates/canisend-gui/src/desktop.rs`
- relevant dialog/page modules

Contract:

```text
Page/Dialog -> WorkerRequest -> Application action -> WorkerEvent -> state reducer -> Page
```

Tasks:

- Define `WorkerRequest` variants for every current application action.
- Keep `WorkerEvent` typed by receipt/read-model result.
- Centralize background spawn, disconnection recovery, busy-state control, and repaint scheduling.
- Centralize post-success refresh rules:
  - workspace mutation refreshes workspace health and jobs;
  - job mutation refreshes job list and selected detail;
  - workflow mutation refreshes selected detail and timeline;
  - CLI mutation refreshes installation status.
- Ensure a disconnected worker clears busy state and does not invent a success receipt.
- Do not store closures containing page state in worker requests.

Tests:

- one request produces at most one terminal event;
- a disconnected worker returns the UI to an actionable state;
- a second mutation is rejected while one mutation is active;
- success and failure reducers preserve selected workspace/job consistently.

Exit:

- Stage 4B can add operations by adding request/event variants and reducer cases, without embedding
  business work in a page.

### A4 — Route overlapping CLI families through `canisend-app`

Files:

- `crates/canisend-cli/Cargo.toml`
- `crates/canisend-cli/src/main.rs`
- optional new `crates/canisend-cli/src/app_adapter.rs`
- `crates/canisend-cli/tests/binary_contract.rs`

The exact migration set is:

- `product.version`;
- `product.doctor`;
- `workspace.init`;
- `workspace.status`;
- `workspace.check`;
- `workspace.backup`;
- `job.create`;
- `job.import`;
- `job.list`;
- `job.show`;
- `job.archive`;
- `workflow.start`; and
- `workflow.status`.

Tasks:

- Add the exact workspace version of `canisend-app` to `canisend-cli`.
- Translate CLI arguments and explicit CLI user intent into application requests/consent tokens.
- Translate `ActionReceipt<T>` into the existing `AgentResponse` without changing:
  - operation;
  - status;
  - JSON data;
  - artifacts;
  - required consents;
  - warnings;
  - next actions;
  - human lines;
  - error code;
  - retryability; or
  - exit class.
- Preserve stdout/stderr separation and `--json` behavior.
- Remove direct orchestration for each migrated operation only after its parity test passes.
- Keep lower-layer CLI dependencies because later, non-migrated command families still use them.
- Do not route CLI operations through the GUI or spawn the installed `canisend` binary.

Parity fixtures:

- run the pre-migration and post-migration behavior against equivalent isolated workspaces;
- compare normalized JSON envelopes;
- separately assert stable human summary fragments and exit codes;
- include healthy and failing cases;
- include local source, URL-policy rejection, missing workspace, archived job, and workflow-not-found
  cases.

Exit:

- the 13 overlapping Alpha operations use the same application actions as the GUI;
- existing binary-contract snapshots remain unchanged; and
- direct lower-layer calls for those operations are removed from CLI command handlers.

### A5 — Stage 4A closure

Required commands:

```bash
cargo fmt --all --check
cargo test -p canisend-app --locked
cargo test -p canisend-gui --locked
cargo test -p canisend-cli --test binary_contract --locked
cargo clippy -p canisend-app -p canisend-gui -p canisend-cli --all-targets --locked -- -D warnings
cargo run -p xtask --locked -- release check
```

Fast CI:

- macOS quality and test lanes remain parallel;
- neither lane includes packaging, cross-compilation, signing, network update tests, long fuzzing,
  or dependency assurance;
- cached target is under two minutes per lane;
- a cold hosted run must remain below five minutes wall time;
- any regression above five minutes is triaged before Stage 4B feature work continues.

Stage 4A definition of done:

- [x] Roadmap and parity authority are unambiguous.
- [x] GUI structure is modular with no visible behavior change.
- [x] Worker requests/events are typed and centrally reduced.
- [x] The 13 overlapping Alpha CLI operations route through `canisend-app`.
- [x] CLI response and exit contracts are unchanged.
- [x] Focused and source gates pass.
- [x] The local Fast CI equivalent remains under the five-minute target; the current hosted run
  will start after the commits are pushed.

## 7. Stage 4B — recovery and workflow controls

### B1 — Add workspace restore and repair application actions

Files:

- `crates/canisend-app/src/workspace.rs`
- `crates/canisend-app/src/error.rs`
- `crates/canisend-app/src/lib.rs`
- `crates/canisend-app/tests/` recovery integration tests

New read models:

```text
WorkspaceRestoreReadModel
  backup: PathBuf
  destination: PathBuf
  workspace: WorkspaceStatusData

WorkspaceRepairReadModel
  workspace: PathBuf
  repaired_projections: usize
  check: WorkspaceCheckData
```

New actions:

```text
Application::restore_workspace(backup, destination)
Application::repair_workspace(workspace)
```

Invariants:

- restore verifies the complete backup before mutating the destination;
- destination must be absent or an allowed empty destination under the existing store policy;
- symlink, file, non-empty, partial, oversized, malformed, or digest-mismatched inputs fail safely;
- restoration remains staging-first and atomic;
- the original backup is never mutated;
- failed restore does not register or select a workspace;
- repair acts only on managed projections;
- user edits are never silently overwritten;
- repair is idempotent; and
- both receipts contain metadata, not private document bodies.

Tests:

- verified backup restores the same workspace identity and job count;
- corrupt manifest/database/blob is rejected;
- occupied or unsafe destination is rejected;
- injected mid-restore failure leaves no completed destination;
- restored projections pass workspace check;
- repair fixes missing/repair-required projections;
- repair preserves edited projections;
- a second repair returns zero and remains healthy.

Exit:

- both operations are available through typed application actions with structured errors.

### B2 — Add workspace recovery GUI

Files:

- `crates/canisend-gui/src/dialogs/workspace.rs`
- `crates/canisend-gui/src/pages/workspaces.rs`
- `crates/canisend-gui/src/worker.rs`
- `crates/canisend-gui/src/state.rs`
- `crates/canisend-gui/src/i18n.rs`
- `crates/canisend-gui/src/registry.rs` tests as needed

Restore flow:

1. User chooses a CanISend backup directory.
2. User chooses a new/empty destination.
3. GUI displays both canonical paths and explains that the backup is read-only.
4. User explicitly confirms restore.
5. Worker runs `Application::restore_workspace`.
6. Only after success, GUI registers the restored path, selects it, loads status, health, and jobs,
   and presents the receipt.
7. On failure, the previous active workspace and registry remain unchanged.

Repair flow:

1. Workspaces page displays projection repair requirements from workspace health.
2. User opens a repair summary.
3. GUI states that only missing or repair-required managed projections are rebuilt.
4. Worker runs `Application::repair_workspace`.
5. GUI refreshes health and reports the repaired count.
6. A zero-change result is a successful no-op, not an error.

UX and accessibility:

- native directory pickers only;
- no free-form shell/path command execution;
- keyboard reachable controls and deterministic focus after dialog open/close;
- accessible dialog, heading, status, and confirmation semantics;
- English and Simplified Chinese strings;
- long canonical paths wrap or scroll without clipping;
- activity state prevents a second workspace mutation.

Tests:

- restore registry mutation occurs only after success;
- restore failure retains the current selection;
- repair success refreshes health;
- confirmation cancellation performs no work;
- both language catalogs cover every new key;
- AccessKit exposes action names, warning text, and result status.

Exit:

- a GUI-only user can recover a verified backup into a new workspace and repair projections without
  opening a terminal.

### B3 — Add workflow control application actions

Files:

- `crates/canisend-app/src/workflow.rs`
- `crates/canisend-app/src/error.rs`
- `crates/canisend-app/src/lib.rs`
- focused app tests

New request/read models:

```text
WorkflowControlReadModel
  status: WorkflowStatusData
  stage_descriptors: Vec<StageDescriptor>

WorkflowBeginRequest
  job_id: EntityId
  stage: WorkflowStage
  mode: ExecutionMode

WorkflowCompleteRequest
  job_id: EntityId
  stage: WorkflowStage
  artifact_id: EntityId

WorkflowRerunPreview
  job_id: EntityId
  target: WorkflowStage
  affected_stages: Vec<WorkflowStage>
  affected_outputs: Vec<ArtifactReference>

WorkflowRerunRequest
  job_id: EntityId
  stage: WorkflowStage
```

New actions:

```text
Application::workflow_controls(workspace, job)
Application::begin_workflow_stage(workspace, request)
Application::complete_workflow_stage(workspace, request)
Application::preview_workflow_rerun(workspace, job, stage)
Application::rerun_workflow_stage(workspace, request)
```

Rules:

- allowed modes come from the authoritative compiled stage descriptor;
- begin is offered only for a `ready` stage;
- `user-decision` begins as `awaiting-user`; other supported modes begin as `running`;
- complete parses and resolves an authoritative artifact reference before calling the store;
- the store remains responsible for output-kind, job, revision, current/stale, and state
  validation;
- rerun is never offered for Intake;
- rerun preview lists the target and all descendants whose state/output may be invalidated;
- rerun requires an explicit GUI confirmation;
- success receipts contain refreshed workflow state and relevant artifact metadata;
- blockers and next actions come from the refreshed authoritative status.

Tests:

- invalid entity IDs map to input errors;
- blocked or already-running stage cannot begin;
- unsupported mode is rejected;
- valid mode produces the expected running/awaiting-user state;
- missing, stale, wrong-kind, or wrong-workflow artifact cannot complete a stage;
- compatible current artifact completes the stage and refreshes descendants;
- Intake rerun is rejected;
- rerun preview matches graph descendants;
- rerun invalidates the correct downstream states and output heads;
- failed mutations leave prior state unchanged.

Exit:

- all three workflow mutations are available as typed application actions and reuse store
  transactions.

### B4 — Route restore/repair and workflow mutations through the CLI adapter

Files:

- `crates/canisend-cli/src/main.rs`
- `crates/canisend-cli/src/app_adapter.rs`
- `crates/canisend-cli/tests/binary_contract.rs`

Tasks:

- Replace direct CLI orchestration for the five Stage 4B operations with application calls.
- Preserve existing argument names and accepted enum spellings.
- Preserve exact Agent v2 operation/status/data and error mapping.
- Preserve restore/repair human output meaning.
- Preserve workflow artifacts and next actions in the JSON envelope.
- Add parity cases for every success and important failure path.

No new CLI flag is required for GUI confirmation. An explicit `workflow rerun` CLI invocation
remains the CLI user's mutation intent; the GUI adapter owns its additional confirmation dialog.

Exit:

- CLI and GUI use the same five application actions;
- no CLI contract snapshot changes without a separately reviewed contract decision.

### B5 — Add workflow controls to the GUI

Files:

- `crates/canisend-gui/src/components/workflow_timeline.rs`
- `crates/canisend-gui/src/dialogs/workflow.rs`
- `crates/canisend-gui/src/pages/jobs.rs`
- `crates/canisend-gui/src/worker.rs`
- `crates/canisend-gui/src/state.rs`
- `crates/canisend-gui/src/i18n.rs`

Per-stage controls:

| Stage state | Primary GUI behavior |
|---|---|
| `blocked` | Disabled action; show authoritative blocker |
| `ready` | Begin; show only modes supported by the stage descriptor |
| `running` | Complete using a compatible artifact ID, or wait for agent/provider |
| `awaiting-user` | Complete using a compatible artifact ID |
| `complete` | Rerun with affected-stage preview and confirmation |
| `stale` | Show stale reason and offer rerun when allowed |

Tasks:

- Add an action area to the existing workflow timeline rather than a new top-level page.
- Show stage, state, execution mode, current output metadata, blockers, and next actions.
- Validate artifact ID syntax before dispatch and show field-level errors.
- Resolve the artifact through the application action; do not trust GUI-provided kind/revision.
- Show a copyable equivalent CLI command as text only.
- Rerun dialog lists affected stages and current output references before confirmation.
- Refresh selected job detail and timeline after every successful mutation.
- Preserve timeline selection and scroll position when practical.
- Localize all new labels, state descriptions, warnings, and receipts.
- Add keyboard focus and AccessKit semantics to menus, dialogs, state changes, and live notices.

Tests:

- control availability follows each stage state;
- mode choices exactly match descriptors;
- invalid artifact input never dispatches;
- rerun cancellation performs no mutation;
- successful events refresh the selected workflow;
- failure events retain the previous workflow and surface recovery;
- English and Simplified Chinese coverage is complete;
- actions remain disabled while the worker is busy.

Exit:

- a GUI user can begin, complete, and rerun a workflow stage without a terminal;
- the GUI does not invent stage transitions or bypass store validation.

### B6 — Stage 4B closure and Alpha.2 checkpoint

Update only after all implementation and evidence exist:

- mark the five operations `implemented` in
  `docs/contracts/cli-gui-parity-v1.json`;
- record `22 implemented` and `13 deferred-beta`;
- update the desktop guide and known limitations;
- record that stage-specific artifact creation remains deferred;
- update the Stage 4 status without declaring full feature completion.

Focused/source gates:

```bash
cargo fmt --all --check
cargo test -p canisend-store --test store_contract --locked
cargo test -p canisend-app --locked
cargo test -p canisend-gui --locked
cargo test -p canisend-cli --test binary_contract --locked
cargo clippy -p canisend-app -p canisend-gui -p canisend-cli --all-targets --locked -- -D warnings
cargo run -p xtask --locked -- release check
```

Native Alpha.2 candidate gate, run only once feature/source gates are green:

- build CLI archives on the five supported CLI targets;
- build the Apple Silicon macOS app with the version-matched bundled CLI;
- ad-hoc sign nested executables and the app;
- verify app manifest and checksums;
- run packaged restore/repair/workflow/reopen lifecycle in a disposable user profile;
- run CLI install/update/uninstall and workspace-retention lifecycle;
- verify English/Chinese launch and accessibility smoke;
- download the staged candidate and verify exact bytes before any publication decision.

Publication is a separate authorized action. Completing B6 prepares a candidate; it does not
automatically create a tag, release, or update response.

Stage 4B definition of done:

- [x] Verified backup restore works from the GUI into a new destination.
- [x] Projection repair is safe, idempotent, and visible in the GUI.
- [x] Workflow begin, complete, and rerun use typed application actions.
- [x] Rerun impact is previewed and explicitly confirmed.
- [x] CLI contracts remain unchanged for all five migrated commands.
- [x] New GUI strings and accessibility semantics pass in English and Simplified Chinese.
- [x] Parity is `22/35`, with exactly 13 explicit deferred-Beta families.
- [x] Local and hosted Fast CI remain below five minutes.
- [x] Exact downloaded macOS lifecycle and accessibility evidence passes before Alpha.2
  publication is considered.

## 8. Test ownership and timing policy

Use the smallest test that proves each change:

| Change | Required local tests | Scheduled/release owner |
|---|---|---|
| App request/read model | `canisend-app` unit/integration | workspace fast CI |
| Store invariant regression | named `canisend-store` test or store contract | scheduled assurance for broader cases |
| GUI state/rendering | `canisend-gui` unit/reducer/accessibility | packaged macOS smoke |
| CLI adapter | binary contract focused test | five-target release matrix |
| Docs/parity JSON | JSON parse + release check | release qualification |
| Malformed local input | bounded fixture regression | scheduled fuzzing |
| Package/signature | none in edit loop | Alpha/RC native gate |

The ordinary edit loop must not run:

- Windows/Linux compilation;
- Intel macOS native qualification;
- package staging;
- code signing;
- notarization;
- public update checks;
- long-running fuzzing;
- advisory/license scans; or
- clean-tag release matrices.

## 9. Atomic commit sequence

Recommended implementation commits:

1. `docs(stage4): reconcile 4a and 4b execution authority`
2. `refactor(app): split typed receipts errors and operation families`
3. `refactor(gui): split desktop pages dialogs and state`
4. `refactor(gui): centralize typed worker requests and reducers`
5. `refactor(cli): route alpha overlap through application facade`
6. `test(parity): freeze shared alpha adapter behavior`
7. `feat(app): add verified workspace restore and projection repair`
8. `feat(gui): add workspace recovery controls`
9. `feat(app): add workflow begin complete and rerun actions`
10. `refactor(cli): route workflow controls through application facade`
11. `feat(gui): add typed workflow controls and rerun preview`
12. `test(stage4): qualify recovery and workflow control slice`
13. `docs(stage4): close 4b parity and alpha2 limitations`

Rules:

- each commit must compile and pass its focused tests;
- structural and behavioral changes are not mixed in one commit;
- parity JSON changes only after implementation evidence;
- no version bump is required because the source is already `1.0.0-alpha.2`;
- no tag or publication occurs in the implementation commit sequence.

## 10. Estimate and critical path

Planning range for one experienced engineer:

| Package | Estimate |
|---|---:|
| A0 tracking authority | 0.5 day |
| A1 app receipt/error normalization | 1–1.5 days |
| A2 GUI modular split | 1–2 days |
| A3 worker boundary | 0.5–1 day |
| A4/A5 CLI migration and closure | 1–2 days |
| B1/B2 workspace recovery | 1.5–2.5 days |
| B3/B4 workflow app/CLI actions | 1.5–2.5 days |
| B5 GUI workflow controls | 1.5–2.5 days |
| B6 qualification/docs | 1–2 days |
| **Total** | **9–15 engineering days** |

Critical path:

```text
A0 -> A1 -> A3 -> A4 -> A5 -> B1/B3 -> B2/B4 -> B5 -> B6
           \-> A2 ----/
```

A2 may be developed alongside A1 conceptually, but changes must land sequentially to avoid
conflicting edits to the GUI and application boundary.

## 11. Principal risks and rollback

| Risk | Control | Rollback |
|---|---|---|
| CLI JSON or exit behavior drifts | pre/post parity fixtures and binary snapshots | revert only the relevant CLI adapter commit |
| GUI split changes behavior | mechanical moves first; no layout edits | revert GUI split without affecting app changes |
| Restore registers a failed destination | registry update only after success receipt | retain old active workspace and delete no user path |
| Repair overwrites edits | reuse `repair_all` invariant and edited-projection regression | disable GUI repair action; store remains unchanged |
| Rerun invalidates more work than expected | authoritative preview, explicit confirmation, atomic store transaction | retain action behind feature control until tests pass |
| Wrong artifact completes a stage | resolve authoritative reference and reuse store validation | surface structured conflict; no transaction commit |
| New UI blocks event loop | all mutations stay behind worker request/event boundary | remove the new dispatch path without store changes |
| Fast CI exceeds five minutes | focused packages, parallel lanes, release-only native work | move broad assurance back to scheduled workflow |
| Roadmap reports false completion | parity status changes only with evidence | restore prior JSON count and leave task pending |

No workspace migration is expected. Recovery mutations already use transactional/staging controls,
so reverting the UI/application adapter does not require rewriting a user workspace.

## 12. Final exit decision

Stage 4A is complete only when the architecture is ready for feature growth and current user
behavior is unchanged.

Stage 4B is complete only when all five new GUI operation families work through the shared
application boundary, their CLI behavior remains stable, parity reaches `22/35`, Fast CI stays
below five minutes, and the exact macOS Alpha candidate lifecycle passes.

After Stage 4B:

- `v1.0.0-alpha.2` may be considered for publication as an explicit Alpha checkpoint;
- Stage 4 continues with profile/evidence, decision workflow, agent tasks, discovery, documents,
  review, render, and export;
- `v1.0.0-beta.1` remains prohibited until the intended 1.0 surface is complete and the remaining
  deferred-Beta contracts are frozen.
