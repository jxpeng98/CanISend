# Embedded Agent App Roadmap and MVP Checklist

> 2026-09-06 closeout: App-first execution is superseded by CLI-first delivery under
> [ADR-RN-0023](../../../docs/architecture/rust-native/decisions/0023-prioritize-cli-first-local-agent-workflows.md). R0/R1 source completion is retained; R2 is partial
> and unaccepted; unfinished R2-R6 scope is deferred. Historical checklists below are not the
> active queue. The master roadmap owns the next CLI-first slice.

Status: App-first stage closed as superseded; CLI-first closeout and baseline reconciliation in progress
Date: 2026-09-04

## Roadmap at a glance

```text
R0 authority/clean-v4 [source complete] -> R1 App Server [source complete]
  -> R2a effective isolation, bound MCP, explicit consent/approval
  -> R2b durable audit origin, exact managed-file history
  -> R3 complete Workbench and bounded provider-history display
  -> R4 exact embedded Beta, recovery/accessibility, changed-journey user qualification
     +-> R5 ACP/Claude after MVP
     +-> R6 cleanup after parity and rollback window (independent of R5)
```

The [master roadmap](../../../docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md#33-approved-app-first-delivery-sequence)
owns ordering, M4-APP-001–005, and release gates. R2a proves the provider boundary before broad UI
work. R2b preparation can overlap only once receipt/binding contracts are stable. Re-estimate effort
from that proof; the old short Beta-only estimate does not cover the expanded App scope.

## Execution and evidence

Use this checklist and the existing [R2 checklist](../09-04-embedded-agent-app-r2/implement.md).
Do not create another Trellis task/phase/journal hierarchy. Each reviewable outcome records its owner,
dependencies, primary acceptance evidence, rollback, and exact source when available. Product
implementation, protected CI, artifact qualification, and user evidence are separate facts.

R0/R1 checked items below describe recorded source work, not new public qualification:
[R0 evidence](../archive/2026-09/09-04-embedded-agent-app-r0/implement.md) and
[R1 evidence](../archive/2026-09/09-04-embedded-agent-app-r1/research/app-server-evidence.md).
R1 source is `8240fae7da0eb5ecafc2e3dce7bf7609d05f9cf3`; the review baseline is
`d7c0abfea46aa8d7b9f63d9b1ac7bd7fac93abea`.

## R0 — Architecture, authority, and Workspace v4 blocker

Outcome: the product direction is authoritative and clean Workspace v4 preparation is usable.

- [x] Approve this PRD and the Tauri-for-MVP decision.
- [x] Record the approved conversation-center/right-Inspector UX in the App-first Codex ADR.
- [x] Write an ADR for App-first Codex App Server + MCP architecture; supersede only the
      external-host-first parts
      of the existing desktop/agent decisions.
- [x] Reconcile the authoritative 1.0 roadmap and feature-freeze boundary before product code lands.
- [x] Define the supported installed `codex` discovery/version/App Server capability policy and MVP
      authentication statement. Use a dedicated Codex configuration and separate provider-owned login
      (owner amendment 2026-09-05); never collect or persist provider tokens.
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

- [x] Add a deterministic fake Codex App Server process for offline protocol tests.
- [x] Implement newline-delimited JSON-RPC framing with bounded input/output.
- [x] Implement `initialize`/`initialized` and required-method/capability validation.
- [x] Implement thread start/resume, turn start, notification/server-request streaming,
      `turn/interrupt`, and shutdown.
- [x] Assign desktop session/turn IDs and monotonic event sequence numbers before normalizing provider
      events.
- [x] Reuse the existing Codex executable discovery path, run `codex --version`, and spawn exactly
      `codex app-server --listen stdio://` without a shell.
- [x] Separate stdout protocol parsing from bounded/redacted stderr diagnostics.
- [x] Add timeouts, request correlation, child-exit handling, and exact-child cleanup.
- [x] Expose one Tauri start/load command, one turn command with a streaming channel, one cancel
      command, and one status/readiness command.
- [x] Version and migrate the existing session registry; persist only resumable metadata and
      body-free receipt references, never transcript bodies.
- [x] Add a minimal conversation panel to prove streaming, cancel, restart, and resume.

Exit: fake-App-Server tests pass and a local Codex session streams in the App without invoking the old
process-per-turn path.

## R2 — Bound tools, consent, and exact history

Owner: desktop/application maintainer for R2a; Store/application maintainer for R2b.
Detailed acceptance and steps live only in the [R2 plan](../09-04-embedded-agent-app-r2/implement.md).

- [ ] R2a / M4-APP-001 proves effective provider isolation, inherited configuration handling,
      start/resume/turn policy, and required MCP readiness on the exact supported CLI.
- [ ] Every Application-ID path and no-ID/list output respects the selected scope. Private reads and
      provider sends require explicit consent; model flags and automatic approvers grant nothing.
- [ ] One representative MCP preview/approve/commit/verify flow and its denial/stale/replay/cancel/
      crash cases pass through the existing facade and broker for both Packs.
- [ ] R2b / M4-APP-002 retains body-free committed origins in product audit beyond registry deletion,
      with an explicit post-commit trace-gap result rather than a mutation retry.
- [ ] Exact snapshots use logical paths, actual generator identity, same-kind predecessors, and
      current-head-only reuse; projection repair/restore uses verified retained Blobs.
- [ ] Bounded desktop-only manifest and selected-file comparisons pass their owning tests; no
      Workspace scan, Git, transcript store, or persisted patch is introduced.

Exit: both R2 units pass. Disable private tool/history entry points to roll back; preserve committed
product data and use compatible backups when an older binary cannot open the additive schema.

## R3 — Simplified Workbench

Owner: desktop/product maintainer. Entry: accepted R2 contracts.

Outcome: the full MVP journey is understandable from one surface.

- [ ] Add Work, Library, and Settings as the only primary destinations.
- [ ] Add the Workspace/Application switcher and provider state to the Workbench header.
- [ ] Build the conversation timeline, composer, session actions, and progress states.
- [ ] Load bounded provider-owned history for a registered thread after reopen/resume, with stable
      ordering, deduplication, scope checks, and explicit unavailable-history state; no local transcript.
- [ ] Compose pack stages, readiness blockers, and next actions from existing Application models.
- [ ] Show Requirement-to-Evidence coverage, missing/stale associations, and selectable Deliverables
      with draft/review/validation/export state.
- [ ] Add explicit context selection and provider-send consent; guided analyze/draft/revise/check/
      export actions reuse existing facade/Skills operations.
- [ ] Keep supported manual inspection, review, and export usable without an available provider.
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

Exit: both Packs complete context -> evidence -> Deliverable -> review -> exact preview/export in
one Workbench. Resume shows provider history or an explicit gap; provider absence preserves supported
manual work. Old routes remain the rollback until measured parity.

## R4 — Exact embedded candidate and user qualification

Owner: release/validation maintainer. Entry: R3 source and inspected protected integration evidence.
The [master M4 gate](../../../docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md#embedded-validation-before-rc)
owns artifact and user thresholds; this checklist does not duplicate or weaken them.

- [ ] Close remaining recovery integration gaps for auth/version readiness, bounded process failure,
      cancel/restart, scope changes, and unavailable history. Reuse passing R1/R2 owner regressions.
- [ ] Prove committed origin survives registry deletion and fresh-path restore; retained bytes and
      comparisons remain unchanged across generator upgrades. Denials retain only bounded cache
      history and are not promised indefinite product-audit retention.
- [ ] Confirm exact PDF preview/export identity, no-Git operation, and manual provider-unavailable
      behavior in the integrated Workbench.
- [ ] Qualify the authorized exact embedded Beta candidate with recorded source/manifest, Codex
      version/capabilities, migrations, Packs, and operation identities. Run affected native/runtime,
      keyboard/VoiceOver/200%-scale and recovery gates on their owned targets; macOS proves no other OS.
- [ ] Prepare the existing evidence-contract/scenario update during R2/R3. Under M4-APP-005, test
      exact build/journey linkage and missing/stale/wrong-build/mixed-denominator rejection before
      the new target is accepted for RC.
- [ ] Run the single formal Issue #70 cohort on the qualified embedded Beta with unchanged user/flow/
      quality thresholds. Reuse consented participants/fixtures; preserve Beta.1 history, do not pool
      incompatible UI results, and do not require two complete cohorts.
- [ ] Update user/privacy/authentication guidance, capability inventory, and rollback instructions.
      Keep Codex installed/version-qualified; bundling remains a separate decision.

Exit: M4-APP-004/005 evidence and machine linkage pass on the exact embedded build. Source checks
alone do not qualify the MVP. Disable embedded entry if its qualification fails; retain external
handoff and manual product operations. Release/publication still requires its own authorization.

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

## Verification flow

Follow [AGENTS.md](../../../AGENTS.md) and the
[quality guidelines](../../spec/backend/quality-guidelines.md). During an edit run one focused
positive/negative check at the owner of each changed invariant, plus only necessary adapter wiring
and changed-language formatting/static analysis. Reuse existing dual-Pack, fake-provider, and recovery
fixtures. Do not copy the same assertion into every adapter or run full Rust suites for prose changes.

Run `cargo run -p xtask --locked -- release check` once on each final applicable integration head.
Protected Fast CI owns the complete workspace suite. Native packages and extended assurance run
only for their affected scope or exact candidate. Record actual checks and evidence once; a source
commit, green local command, or checked checklist is not user or artifact qualification.

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

Start only after R4, measured Workbench parity, and a rollback window. R5 completion is not a dependency.

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

R2a is in progress in the existing [R2 checklist](../09-04-embedded-agent-app-r2/implement.md).
Optional Application binding and fail-closed protocol regressions are implemented locally. Continue
with effective provider isolation and a correlated positive consent proof before embedded MCP
enablement; retain active-freeze controls. R2b and overall R2 acceptance remain pending. No additional
Trellis activation or repeated phase-approval ceremony is required.

## CLI-first closeout and LF-C01

Updated: 2026-09-06. Owner: maintainer. This section is the current bounded checklist; R0-R6 above
is retained history. The owner confirmed the latest working branch and main as the integration
scope. The downloaded 2026-09-05 package supplies LF-C01–12 design input; it does not execute
LF-C02–12, change release authority, or create a second task ledger.

### Scope and acceptance

- [x] Adopt ADR-RN-0023 and map the CLI-first sequence into the existing master roadmap.
- [x] Close App-first execution as superseded; retain R0/R1 completion and defer partial R2 without
      claiming isolation, consent, history, signed-in flow, or artifact acceptance.
- [x] Inspect current branch/main, user changes, manifests, entry points and existing test owners.
- [x] Preserve and review existing source/process changes in independently reviewable commits.
- [x] Integrate latest main/Beta.2 and the prior intake fix, resolve conflicts, record exact freeze
      dispositions, and run focused checks.
- [x] Run the final source gate and inspect protected CI on the integration PR.
- [x] Inspect protected Fast CI and merge the integration PR; close superseded PR work accurately.
- [x] Confirm clean local main and record exact integration evidence. Next implementation: LF-C02.

### Audited baseline and boundaries

Closeout began at `d7c0abfea46aa8d7b9f63d9b1ac7bd7fac93abea` on
`feat/beta2-cli-skills-readiness`, 16 commits ahead of its remote with existing R2/process edits.
Remote main `095236c5a5a1d536f3f596abc9ea436bc38567a2` already carries Beta.2 source and private
candidate records; this older checkout reports Beta.1. Keep main's version and release facts during
integration. PR #225 contains earlier intake work; its macOS quality log reports missing exact
feature-freeze exception coverage, while its other five Fast CI jobs passed. That run does not
qualify the forthcoming integration head.

| Entry / owner | Existing implementation | Coupling or unproven boundary | Existing primary check |
|---|---|---|---|
| `canisend` / `canisend-cli/src/lib.rs` | CLI dispatch through `canisend-app`; stdio MCP bypasses command rendering | Root defaults select all 10 members; no GUI in the audited host CLI normal/build/dev tree | CLI `binary_contract`: help/errors, resource doctor, no-App Host setup and mixed-Pack recovery |
| `canisend-cli/build.rs` | Records Git, target and compiler identity | Builder tools, not consumer runtime requirements; no frontend build invocation | CLI source/version contract |
| `canisend-app` | Shared facade over contracts/core/io/store/resources | Domain owner, not desktop UI; keep it intact | Existing facade/dual-Pack regressions |
| `canisend-resources` | Build-generated embedded resource manifest | Verify exact standalone package resources/cwd/uninstall behavior separately | CLI resource doctor and Host setup; existing package smoke |
| MCP `serve_stdio_with_application` | Existing persistent server and optional Application guard | Independent MCP processes own separate approval Brokers; private reads and writes still require their own authority | `mcp_protocol`: guarded lifecycle, scope mismatch and unbound discovery |
| `canisend-app/src/approval.rs` | `Arc<Mutex<ApprovalState>>` within each Broker instance | Not interprocess storage; cross-connection confirmation is unproven | Existing Broker stale/scope/replay and protocol lifecycle regressions |
| CLI prepare/import/export/recovery | Existing v4 facade operations and dual-Pack fixtures | A complete new exact-Host windowless journey is not proven by source reading | CLI binary/MCP tests; exact native and real-Host gates later |
| `.github/workflows/fast-ci.yml` | Linux core/CLI selection already independent; macOS full-workspace lanes depend on desktop frontend | Separate selected CLI proof from retained full-workspace gates | Protected Fast CI |
| `.github/workflows/release.yml` | Existing explicit `-p canisend-cli` build and standalone archive smoke | Shared source preflight includes frontend; changes need a bounded pipeline review and exact-package evidence | Existing native release matrix, not run for this audit |

### Actual audit checks

- Download manifest: all 10 listed file SHA-256 digests match. Static package validation is not
  product evidence.
- `cargo metadata --format-version 1 --no-deps --locked --offline`: passed; 10 default members.
- `cargo tree -p canisend-cli -e normal,build,dev --locked --offline`: passed; no Tauri, Wry,
  WebKit, GTK or `canisend-gui` entry on the local host. Other target trees and packaged runtime
  qualification remain separate checks.
- No real provider login/model work, package build, publication, or cross-device implementation
  is part of LF-C01. Historical R2 probe results remain historical.

### Next small implementation: LF-C02

Start with `Cargo.toml` default-member selection for the existing CLI and `CONTRIBUTING.md` build
instructions. Inspect `.github/workflows/fast-ci.yml` for the smallest independent CLI check using
existing core lanes; retain full-workspace/desktop gates. Do not modify release packaging in this
first slice unless a demonstrated dependency requires a separate reviewed change. No crate
rename, storage/schema mutation, approval change, new dependency or GUI deletion is needed.

Prove default selection with Cargo metadata, inspect CLI normal/build/dev dependency trees for
supported targets, build the selected CLI without frontend steps, and reuse the binary resource/
help and MCP negotiation checks. Run the source gate once for a final CI/config integration head.
Rollback restores manifest/default selection and CI/docs only; product data is unchanged. LF-C03
then addresses exact standalone resources/install evidence; LF-C04 owns trusted approval gaps.

### Closeout review scope

The shared MCP guard and registry checks preserve their existing positive/negative coverage.
The App shell still feeds legacy dossier selection into AgentView, so complete v4 shell selection
is an open R2 gate; see the correction in R2 evidence. This closeout does not implement or qualify
that deferred App journey. The retained runtime does not inject CanISend private MCP tools.

Local frontend verification after combining branches: all 97 tests across 15 files passed after
fixing the first integration CI failure in provider sign-in status composition. The existing UI
system guard remains unchanged; the shared Alert preserves the status announcement. Svelte/TypeScript reports
zero errors/warnings and frontend formatting passes. Five registry tests, five MCP protocol tests
and two local-input tests passed. Desktop protocol: three passed, one signed-in provider smoke intentionally ignored. Runtime: 15
passed. Strict Clippy passed for App, CLI, MCP, IO and GUI; Rust formatting passed. Source gate and
protected CI remain separate integration gates. The existing macOS linker unwind-size warning
remains non-blocking; no signed-in provider or native artifact qualification was run.

Integration source gate at `2a0500b7` passed with 24 exact freeze exceptions; 5 drift items and
3 release-stage blockers remain reported. PR #226 first CI run `34002975360` found one UI
composition violation (96/97 frontend tests passed). Fix `b3794a8` reuses the existing Alert;
the full 97-test frontend suite, Svelte check and changed-file formatting pass locally. Its exact
exception is recorded; the final head must pass the source gate and fresh protected CI.

The second integration CI run `34003247185` passed desktop UI, browser accessibility and Linux
checks, then macOS quality rejected the retained Python probe under its unchanged Rust-only guard.
Commit `3085ff9` retires that deferred App probe; its full source and prior observations remain linked
from R2 evidence. No probe was reimplemented or executed during closeout. The exact tracked-file
guard passes locally. Fresh final-head source validation and protected CI remain required.

The same run's macOS suite found the exact Tauri inventory assertion still expected 129 leaves;
`login_codex` and `start_agent_session` are both exported and registered, making 131. Commit
`25fc2f0` synchronizes that assertion and its contract table. All 45 Contracts library tests,
Contracts all-target Clippy, Rust format and diff checks pass. The exact CI workspace/all-target/
all-feature Clippy command also passed locally; the initially uncached feature dependencies were
fetched from the unchanged lockfile. Protected CI owns the remaining full-workspace tests.

### Completed LF-C01 integration

PR #226 and ancestral #225 merged as `03897b71d0a6a5762b60e8ad5180d3b75502d40e` on
2026-09-06. The merge tree equals verified head `773168ca`; all six required Fast CI checks
passed in run `34003574951`, and dependency policy passed in `34003574969`. Local main was
clean and synchronized. Final source gate passed with 27 exact exceptions; the five drift items
and three release-stage blockers remain. No release or native qualification was performed.

## LF-C02 — default standalone CLI build

Updated: 2026-09-06. Owner authorized execution after LF-C01. Baseline: `03897b71`.

### Scope and acceptance

- [x] Select only `crates/canisend-cli` through Cargo `default-members`; preserve all workspace members.
- [x] Document default build/run/test commands and explicit full-workspace/desktop selection.
- [x] Reuse Linux/Windows core CI to assert exact default selection, reject desktop dependencies,
      and run `cargo build --locked` before existing tests and Host/MCP smokes.
- [x] Prove the selected local build, five target dependency graphs and existing CLI/MCP behavior.

Actual local evidence: root `cargo build --locked --offline` passed without frontend steps;
root `cargo test --locked --offline --test binary_contract --test mcp_protocol` passed 12 CLI
and five MCP tests. Cargo metadata selects exactly `canisend-cli`. Normal/build/dev dependency
graphs for all five `release/targets.json` targets contain no GUI/Tauri/Wry/GTK/WebKit dependency;
missing locked target crates were fetched without changing the lockfile. The exact new CI shell
step passes locally and rejects empty, GUI-only and mixed default selections in bounded fixtures.
The dependency pattern detects desktop package fixtures. Workflow YAML parses and diff checks pass.

Integration gate: record the exact source commit in the existing freeze ledger, run the source
gate on the final PR head, and require existing protected Fast CI before merging. The PR's check
and merge records own final integration status; local checks do not claim native package evidence.

Rollback restores Cargo default selection, CI and contributor instructions; product data is unchanged.
No Rust behavior, dependency, schema, consent, storage, GUI support or release packaging changed.
Next bounded slice is LF-C03: audit and verify standalone embedded resources and install lifecycle
using existing doctor/package smokes; exact native artifacts remain separately qualified.

### Completed LF-C02 integration

PR #227 merged as `ce12ad87becea78721210826b136b1bd2b296db9` on 2026-09-06. All six
required Fast CI jobs passed in `34023762310`; dependency policy passed in `34023762322`.
The merge tree matches verified head `9f632501`. Source gate passed with 28 exact exceptions;
local main was clean and synchronized. No package qualification or release was claimed.

## LF-C03 — standalone resources and install lifecycle

Updated: 2026-09-06. Owner authorized the next slice after LF-C02. Baseline: `ce12ad87`.

### Scope and acceptance

- [x] Audit embedded resource ownership and existing archive/Host lifecycle tooling before adding code.
- [x] Extend the existing CLI doctor regression to run a copied binary in a Unicode/space path,
      unrelated working directory, empty inherited environment and tool-free PATH. Retain only
      isolated home/temp paths and the Windows OS directory where needed.
- [x] Prove embedded rendering and resource integrity, registration of the copied binary's actual
      path, uninstall without Workspace mutation, and same-build reinstall with usable Skills/data.
- [x] Extend the owning Skill lifecycle test to restore a missing managed file; preserve its existing
      idempotence, versioned-update, user-edit refusal and uninstall checks.
- [x] Replace obsolete `job create`/`agent assets export` in the active native upgrade script with
      existing v4 quick-start and Host commands. Compare both Application snapshots across replacement.
- [x] Document pure CLI setup, executable selection, relocation, repair and separate Host removal.

### Evidence and limits

Resources are compiled with `include_bytes!` by `canisend-resources/build.rs`; verification and
Pack constructors read that embedded catalog. No runtime resource search in a source checkout or
App bundle was introduced. The existing stage/package scripts already produce a standalone binary
and notices, so no second installer, resource loader or package layout was added.

Local Apple Silicon checks passed: isolated standalone CLI regression; Skill lifecycle regression
including missing-file repair and edit-safe refusals; existing upgrade-policy and canonical-evidence
positive/negative tests. Existing `stage_native_bundle.sh` staged the development binary and notices.
A temporary driver executed the exact lifecycle section of `qualify_archive_upgrade.sh` with
the same staged build on both sides, from an unrelated directory. Both Packs, Profile Source,
Application snapshot preservation, backup/restore, Host regeneration and binary-only uninstall
passed. Existing Host and guarded dual-Pack MCP smokes also passed against that copied binary.
The driver did not execute release-pair verification or emit a qualification record. It proves
same-build replacement, not a Beta-to-RC upgrade. Rust format, affected Clippy and shell syntax
checks are integration prerequisites; final results belong to the PR record.

The existing Fast CI jobs own the portable Rust regressions on Linux, Windows and macOS. Final
source validation and exact freeze disposition precede protected integration. Fresh native archive,
signature, clean-machine and real Beta/RC-pair qualification remain LF-C10 and the existing native
workflows; no historical evidence is relabelled and no qualification ledger status changes here.

Rollback restores tests, the upgrade script and matching documentation/source-policy checks;
no product data or runtime API changed. Next bounded slice: LF-C04, audit the actual trusted
headless preview/confirmation/commit path and implement the smallest complete Broker-owned flow.

### LF-C04 — trusted headless confirmation (2026-09-06)

Owner authorized continuing the remaining LF-C sequence. Implement one shared MCP dispatch
boundary using the already installed RMCP form-elicitation capability and the existing in-process
ApprovalBroker. No daemon, persisted approval store, CLI `--yes` grant, new protocol identity or
Host-specific business rule is introduced. The existing `url` dependency gains an RMCP edge;
no dependency version is added or upgraded.

- [x] Trace all ten guarded MCP commits and all private-read/export flags.
- [x] Retrieve exact canonical previews through the owning Broker; review preserves original TTL
  and checks kind, Workspace, Pack, Application and revision/digest before displaying a change.
- [x] Require a separate server-request response for private access/export and final change
  approval; default false, reject unsupported peers and non-exact responses, bound wait to two
  minutes, observe request cancellation, cancel refused commits through existing handlers.
- [x] Preserve commit-time canonical revalidation, token consumption and process isolation.
- [x] Extend owning Broker regression and existing MCP lifecycle protocol fixture with refusal,
  cancellation, malformed/false confirmations, missing capabilities and restart/replay cases.
- [x] Existing dual-Pack packaged smoke passes with synthetic elicitation, backup, restore and reopen.
- [ ] Final source gate and protected CI on the integration head.
- [ ] Qualify actual user interaction on the selected Host/version under LF-C05; synthetic peers
  are explicitly excluded from real-session and user evidence.

Local checks passed: exact Broker review regression; all five existing MCP protocol tests,
including the full Requirement/Plan/Deliverable/review/export lifecycle and new rejection cases;
affected App/MCP/CLI Clippy; Rust formatting, shell syntax and diff whitespace.
The packaged smoke passed against the local development binary after preserving the router-owned
unknown-tool error. It is synthetic lifecycle evidence, not exact native artifact qualification.
Final source validation and exact freeze disposition belong to this PR.
The external Host owns reliable human presentation/response. Capability advertisement and client
name are not identity attestation, and this change does not isolate arbitrary same-user processes.
Codex CLI 0.152.0 is available locally; current official App Server documentation describes the
form request/response path, but neither fact qualifies its actual interaction or inherited policy.
See https://learn.chatgpt.com/docs/app-server (MCP server elicitation requests).

Next: complete LF-C04 integration, then LF-C05 real Host journey and canonical resumption. Keep
LF-C06–08 source coordination work independently reviewable, LF-C09 optional, LF-C10/11 exact
qualification separate, and LF-C12 deferred. No public release or historical-evidence relabelling
is authorized by this implementation scope.
