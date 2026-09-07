# Embedded Agent App Roadmap and MVP Checklist

> 2026-09-06 closeout: App-first execution is superseded by CLI-first delivery under
> [ADR-RN-0023](../../../docs/architecture/rust-native/decisions/0023-prioritize-cli-first-local-agent-workflows.md). R0/R1 source completion is retained; R2 is partial
> and unaccepted; unfinished R2-R6 scope is deferred. Historical checklists below are not the
> active queue. The master roadmap owns the next CLI-first slice.

Status: App-first stage superseded; LF-C01–08 local source integration complete; human and release qualification open
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

The [master roadmap](../../../docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md#33-approved-cli-first-delivery-sequence)
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

The final source gate detected the expected third-party lock fingerprint change from enabling
RMCP elicitation. Exact base/head TOML comparison proved the sole change is `rmcp 3.0.1 -> url`:
all 751 package identities/versions/checksums and other edges are unchanged. CLI reverse dependency
inspection confirmed `url 2.5.8` already belongs to the IO/HTTP/render graph. Fresh installed
`cargo-deny 0.19.7` passed advisories, bans, licenses and sources, with existing duplicate, unused
license-allowance and yanked `chacha20 0.10.1` warnings. Review date and fingerprint are refreshed;
all reachability restrictions and the 2026-09-07 review/expiry deadlines are preserved.


### LF-C04 protected closeout and LF-C05 acceptance preparation

LF-C04 merged through PR #229 at `83c9120d915851a0113f2490458c3c396a8f25d0` on
2026-09-06. The merge tree equals checked head `f2deab6cee8d84cbf0f19747c3021fc8a61a7571`.
Final source validation passed with 31 exact freeze exceptions and the existing five drift items
and three stage blockers. Fast CI `34029310377` passed all six required jobs; dependency assurance
`34029310398` passed. This closes source integration, not actual Host/user qualification.

LF-C05 reuses the documented quickstart fixture and the checked native development binary.
The generated environment is `/private/tmp/canisend-lfc05-interactive`, outside product data and
source control. Its `README.md`, `HOST-PROMPT.txt` and `prepared-environment.json` record the launch
steps and body-free identities. The copied binary digest is
`058c27963b8d6a39b5459f4475b9e826a878ab306f7c1e9fed56cda54495b4e2`; it is not a release archive.
Both Packs, synthetic Profile Source, backup/restore, project-only Skills and a clean Workspace
check passed. The new project MCP file comes from `host setup`'s exact configuration snippet.
No global Host configuration or authentication was changed, and no worker was automatically started.

- [x] Prepare independent native binary and two Pack-bound synthetic Applications at revision 1.
- [x] Verify generated Codex Skills/configuration, basic lifecycle and canonical Workspace integrity.
- [x] User starts Codex CLI 0.152.0 and rejects the native CanISend confirmation form (boolean false).
- [x] Verify unchanged canonical state after rejection, then obtain actual acceptance of a fresh exact preview.
- [ ] Complete both Pack journeys, review/export/recovery and another local Host session's resumption.
- [ ] Record actual body-free operation/revision/digest outcomes; exclude synthetic responders.

The user ran the first real interaction in Codex CLI 0.152.0 on 2026-09-06 and reported that
cancellation did not occur. The MCP `requirement.confirm.commit` receipt was `confirmed` at
11:28:59Z; generic Application revision changed from 1 to 2 and its selected Requirement became
confirmed at revision 2. A read-only CLI recheck verified snapshot
`49515485d819e89c7751ada193a4206c784c46fe9d14f5e98ea076fae331c08c`.
The user's separate Workspace check reported healthy state. Integrity success does not establish
consent or cancellation success. The copied binary still matches the recorded SHA-256; the
observed calls used MCP preview/commit/show, without a shell mutation fallback.

The user subsequently clarified that a prompt appeared and they selected confirmation/Trust,
not cancellation. This attempt therefore does not demonstrate a cancellation-handling defect.
The wording alone does not distinguish a Host tool-trust prompt from CanISend's exact-change
elicitation form; keep that distinction unqualified. LF-C05 acceptance has not passed.
Preserve the generic revision-2 fixture. A read-only check confirmed the academic Application
remains at revision 1 with a proposed Requirement; `CANCEL-RETEST-PROMPT.txt` in the prepared
environment targets that untouched fixture with a fresh preview. The next real user test must
cancel the exact-change form and verify unchanged revision and snapshot, without retry or
fallback. Do not reuse the original generic revision-1 prompt, infer approval, reset state, or
weaken confirmation checks. Continue the full journeys only after the actual cancel/accept checks.

The second reported cancellation returned MCP `approval.denied` and preserved both Applications.
Read-only canonical checks confirmed academic revision 1 and its original snapshot, and generic
revision 2 and its recorded snapshot. Inspection of the actual MCP invocation at 12:07:27Z found
`approved: false`: the Host selected the explicit denial path before server elicitation. This is
valid negative evidence for explicit denial, not evidence that a user cancelled the native form.
The reported academic Requirement ID omitted its final `8`; the canonical ID remains
`01a0766f-74d7-7510-85ac-2657256a48d8`. The prior preview is consumed; the next attempt needs a
fresh preview and `approved: true` solely to enter the separate Host confirmation gate, followed
by actual user cancellation. The updated external prompt makes this distinction explicit.

The third attempt at 12:17:10Z invoked exactly one guarded MCP commit with `approved: true`.
It returned `consent.host-confirmation-required` after 23.3 seconds, without an authorized
operation. Read-only canonical checks again confirmed academic revision 1 with the original
snapshot and generic revision 2 with its recorded snapshot. This establishes rejection of a
guarded commit without Host confirmation; it does not identify the underlying form action.
The user-supplied report says no form response was supplied. The shared error also covers
decline, cancellation, unsupported capability and transport failure, so native UI cancellation
remains unqualified until the operator identifies the visible prompt and actual action. Do not
request another identical run before resolving that observation. No product code defect is
demonstrated by these denials; full journey and local-session resumption remain pending.

The operator then clarified that the third attempt displayed the actual form and they selected
`false`. Combined with the recorded `approved: true` invocation, Host-consent error and unchanged
canonical snapshots, this closes the real-Host negative confirmation check. Record the precise
action as boolean false (explicit rejection), not a proven MCP `action: cancel` response: raw
form response metadata was not captured. No further rejection retest is needed. Next: an actual
positive confirmation of a fresh academic preview, both Pack journeys, and a separately opened
local session reading the persisted records. `CONTINUE-JOURNEY-PROMPT.txt` and
`RESUME-CHECK-PROMPT.txt` in the prepared environment provide these bounded operator steps.

The subsequent positive attempt was blocked by Host automatic approval review before MCP
execution. Read-only inspection of the Host operation records found the 12:24:44Z commit reused
the same token issued at 12:17:03Z and used in the rejected 12:17:10Z invocation; no intervening
fresh preview was recorded. Tokens were compared without copying their values into this record.
This violated the continuation prompt's fresh-preview requirement. The Host cited repeated use
after failure and the stop-on-failure instruction. No new receipt or form was produced. A fresh
canonical check still shows academic revision 1 and its original snapshot. Preserve the completed
negative confirmation evidence, but keep positive acceptance blocked. Do not retry the token or
change execution paths. Further positive execution requires informed user approval and a fresh,
reviewed preview in its owning MCP process; stop if a new preview cannot be obtained.

Further diagnosis of the operator's `approval.missing-or-replayed` result identified a Host
state-transfer error. At 12:29:13Z a fresh preview was obtained, but its token remained only in
the local `newToken` variable; no `store()` retained the preview across tool invocations. At
12:30:55Z the commit again supplied the token used in the 12:17:10Z denial. The preceding expiry
check used the new preview's deadline for that old token, so it did not validate the submitted
binding. CanISend correctly rejected the consumed token. No product change is justified by this
trace. Correct the Host procedure to retain one complete preview record with `store()`, then
derive token, digest and expiry together from `load()` after explicit approval. Never reconstruct
a token from conversation text or substitute an old literal. Preparing that retained preview is
read-only; further commit remains stopped pending the operator's next explicit authorization.

The corrected Host procedure retained the complete fresh preview and verified it with a separate
`load()` invocation. The operator then explicitly authorized one commit, reviewed the displayed
academic revision-1 Requirement confirmation form, and selected True. The reported
`requirement.confirm.commit` response was `confirmed`; the selected local record was cleared and
no retry or subsequent business operation occurred. Read-only canonical inspection independently
confirmed Application revision 2, Requirement revision 2 and `confirmed` state, with snapshot
`873660d43fc2194f3cbac34b376dcb4a9048f86fec3d32b967deb9564827b9c7`.
The operation response did not return a separate audit receipt field; do not invent an audit ID
or claim independent audit verification. Real native-form positive and negative confirmation
checks are now complete. The updated continuation prompt starts with Plans for the two existing
revision-2 Applications; full journeys and separate-session canonical resumption remain pending.

Academic Plan preparation then stopped on `input.invalid`: the Host selected only cover-letter
after a partial validator error and omitted the required CV. Inspection of the complete checked
academic manifest, whose digest matches the Application's bound Pack, confirms minimum/maximum
1/1 for both `cover-letter` and `cv`; research-statement and teaching-statement are 0/1 each.
The failed preview performed no commit and was not retained, per the operator report. The
correction is a fresh Plan preview containing both mandatory kinds, not relaxing Pack validation.
This also exposes a Host discovery limitation: the current MCP surface has no complete Pack
catalog read operation, and the CLI resource command lists metadata only. Record this as an
acceptance usability gap; do not describe first-error probing as complete catalog discovery.

### LF-C05 independent evidence review and replacement fixture — 2026-09-07

The operator authorized subagent verification. One read-only subagent independently reviewed
the acceptance mapping and confirmed that historical native False/True evidence does not close
full dual-Pack journeys or separate-session resumption. Both parent and subagent found the old
`/private/tmp/canisend-lfc05-interactive` directory absent. Its last revision-2 states are historical
observations, not states reverified today. No original backup was found in the inspected locations.
The later academic Plan preview expired without a commit, per the operator report.

A new fixture was prepared under gitignored `dist/lfc05-20260907`, reusing the documented
quickstart smoke: two Packs, basic data, backup and restore passed. Its copied binary retains
SHA-256 `058c27963b8d6a39b5459f4475b9e826a878ab306f7c1e9fed56cda54495b4e2`.
Generic Application is `01a07b4c-f6a1-79f5-a27d-a5d0cb5991fa`; academic Application is
`01a07b4c-f6f8-7805-9628-e9fdaef063ec`. Both begin at revision 1. This is a new acceptance round,
not recovery or resumption of the missing fixture. Project-only Host resources were installed;
no Host session or automated form responder was started. The local README and HOST-PROMPT retain
the native human gates while keeping each fresh preview and its approval in one active turn.
Subagents may review artifacts and state independently, but do not receive active approval grants.

### LF-C05 automation-first acceptance optimization — 2026-09-07

The operator authorized automated verification before the remaining brief real-Host check.
Reused the existing dual-Pack MCP smoke and owning protocol/Broker tests; no new runner, product
control change or automated response in a human Host was introduced. An independent subagent
review identified the missing exact reopen/restore assertion. The smoke now compares each
Application's complete canonical data with its final MCP snapshot after reopening and restoring,
verifies original exports through a new CLI process, and emits `acceptance-summary.json` with an
explicit automated-only classification. Scoped export directories are excluded by the existing
backup contract: restored export discovery is asserted empty, while canonical draft/review state
and referenced Blob integrity are verified. Re-export requires fresh consent.

Actual local checks passed: enhanced dual-Pack smoke at `dist/lfc05-auto-verified-20260907`,
all 5 MCP protocol tests, all 9 ApprovalBroker tests, Bash syntax and diff whitespace. The app
test link emitted the existing large `__eh_frame` warning; all selected tests passed. Both Pack
flows reached revision 7 with confirmed Plans, one generic and two academic Deliverables, reviewed
local exports, and exact state after reopening/restoring. These are synthetic fixtures, not
evidence-backed real application writing or human Host qualification. The form timeout branch
has no dedicated elapsed-time regression and is not claimed as covered. No Rust source or CI
configuration changed; protected CI and native qualification were not run for this local slice.
Independent subagent review found no actionable issues in the script or guide; it was a static
review, not another test run or Windows qualification.

The repeatable procedure is [Agent acceptance](../../../docs/guides/agent-acceptance.md).
Use existing native False/True evidence for the matching build/Host; keep remaining real Host
steps in one active conversation, with immediate user form responses rather than cross-task
preview copying. Next: complete the brief actual dual-Pack Host journey and separate-session
canonical resumption. LF-C05 remains pending those observations; LF-C06 is not started by these
automatic checks.

### Actual Host hold-Plan boundary — 2026-09-07

The new user-started Host reports Codex CLI 0.153.4 (different from the historical 0.152.0 form
evidence). Canonical inspection confirms academic Application `01a07b4c-f6f8-7805-9628-e9fdaef063ec`
at revision 3 with a confirmed Requirement and a draft `hold` Plan containing required
cover-letter and CV. Generic remains revision 1. The operator reports zero confirmed Evidence,
healthy Workspace and no drafts/exports. No independent audit ID was returned.

The 10:13:00Z Plan confirmation preview was retained; the 10:13:07Z invocation correctly loaded
its token/digest/expiry together and cleared the selected record. Host automatic review rejected
the commit before native confirmation because the transcript did not establish the user's
choice of this exact Plan. This was not the earlier token-transfer bug. The MCP schema requires
explicit approval of the exact preview before `approved: true`, and the server additionally
requires its native form. Do not weaken either control or retry the rejected call. The concrete
remaining decision is whether the operator accepts this hold Plan; even confirmation does not
resolve the evidence gap or establish drafting/export readiness. Further execution needs that
informed decision and a fresh preview. Real Host qualification remains incomplete.

Preliminary LF-C06 audit found that the existing TaskService/TaskDescriptor and ReviewService are
Job-bound. Their existing lease/transaction/Blob patterns are reusable, but their public data types
cannot be relabelled as Application v4. Any coordination slice must bind Application/Pack/input
versions, keep coordination generation separate from content revision, and account for the
business-commit/accepted-task recovery boundary. No collaboration implementation or migration has
been made. Respect the LF-C05 dependency before selecting that implementation; LF-C07 needs two
user-started real Hosts, LF-C09 remains optional, LF-C10/11 qualification and publication remain
separate, and LF-C12 remains deferred.

### LF-C05 development fixes — 2026-09-07

Owner authorized implementation after the actual Host refusal. The MCP request contract now
uses `request_confirmation` and `request_private_read/export`: these request native forms and
never assert a prior human decision. Actual native acceptance remains mandatory; legacy input
fields are rejected. No rejected interactive commit was retried and no Host policy was bypassed.
Hosts must rediscover schemas and obtain fresh previews from the updated binary.

Added exact Application Pack manifest discovery through CLI/MCP and its Orientation permission,
so Hosts read all required kinds before proposing a Plan. Added a public guarded v4 Evidence
confirmation path using existing ProfileSource, catalog, Broker, transaction and Blob primitives.
It checks source revision/digests, byte quotes, privacy and current Application context, stores
only the exact approved Workspace catalog, and leaves association to the existing separate tools.
No Job, database migration or direct database fixture seed was introduced.

Actual local checks:
- Source-bound Evidence owning regression: passed (positive, private denial, source/quote/range/
  revision/digest mismatch, stale source, cross-Application, restart and single-use rejection).
- Exact Pack catalog owning regression and CLI dual-Pack binary contract: passed.
- Orientation exact-operation regression: passed.
- MCP protocol suite: 5 passed, including old-field rejection and native Evidence False/True.
- ApprovalBroker suite: 9 passed, including manual-clock expiry and replay cases.
- Formatting and affected Store/App/MCP/CLI all-target Clippy: passed.
- Updated native debug binary dual-Pack smoke: passed at `dist/lfc05-fixed-final-20260907`;
  both Applications reached revision 7, all Deliverables retained Evidence inputs, and exact
  reopen/restore plus original export verification passed. Its summary explicitly records
  `human_host_acceptance: false`.

The source gate detected the new guide in the domain-coupling inventory; inspected classification
is `compatibility-surface`, then the checked inventory was refreshed. The subsequent
`cargo run -p xtask --locked -- release check` passed schemas, resources, inventory, documentation,
release truth, dependency policy and version checks, then stopped at
`provider dogfood Skill identity or digest is stale`. Existing exact-artifact human evidence in
`release/provider-dogfood.json` must not be relabelled as qualification of these changed Skills.
Protected Fast CI and real Host/new-artifact qualification remain unproven. LF-C05 remains open;
next step is a user-controlled updated Host with fresh native confirmation, then canonical
resumption and truthful evidence renewal. No release fact, deadline or validator was weakened.

### CLI development-flow simplification — 2026-09-07

The owner requested a review and removal of excessive development gates and manual stops. The
CLI-first architecture remains appropriate; the avoidable delay came from coupling development
to current human/release evidence, making CLI CI wait for the desktop, duplicating property
coverage, and allowing validator unit tests to depend on live qualification dates and records.

Implemented one source entry point, `xtask source check`, by reusing the existing checks. Full
`release check` still calls all 34 original checks; human/artifact readiness, freeze dispositions
and dependency dates retain their release or existing dedicated workflow owners. No release
validator, historical qualification record, product confirmation, or dependency deadline was
weakened. LF-C06 depends on the automated LF-C05/code contracts, with actual Host acceptance
remaining a candidate qualification item. This supersedes the earlier instruction to wait for
human LF-C05 completion before dependent implementation; no LF-C06 code is implemented here.

Fast CI now runs CLI/shared quality and tests independently of desktop UI. GUI Rust maintenance
runs in the existing desktop job, the unused frontend artifact handoff is removed, and property
tests run once in the source workspace suite. Existing job names and the release workflow remain
intact. Validator regressions use an explicitly synthetic temporary fixture; live dependency
expiry is checked by the existing dependency workflow and full release command. Active project
instructions, the ADR amendment, roadmap and acceptance guide describe this boundary consistently.

The now-complete source check also found and resolved earlier API projection omissions: typed
operation/semantic mappings, new MCP request-field markers, and the current source package's
operation-registry binding. Actual CLI/MCP coverage is recorded; no absent GUI implementation or
human run was invented.

Verification: `cargo run -p xtask --locked -- source check` passed; all 91 xtask tests passed;
xtask all-target Clippy, Rust formatting, workflow YAML parsing and `git diff --check` passed.
The full release check still rejects the stale provider-dogfood Skill digest, proving the old
record cannot qualify current bytes. Protected GitHub CI and new real Host/candidate qualification
have not run. Next development can proceed from the passing source checks without another process
approval; collect remaining real evidence when the affected candidate is ready.

### Local integration and milestone checks — 2026-09-07

Owner prefers local development/merges, checks at important milestones, and no routine PRs.
Updated the active instructions, contributing guide, quality guide, ADR amendment and roadmap:
reuse the current branch; isolate only conflicting parallel work or risky experiments; make
meaningful local commits and merges; batch broader checks at completed features, contract changes,
substantial conflict resolution or candidate boundaries. Keep focused bug/consent/data-integrity
regressions. PRs are only for requested external review or required protected remote integration.

Current `feat/lf-c05-host-acceptance` and local `main` both point to `83c9120d`; work is uncommitted,
so there is no committed divergence to merge. No branch, PR, push, deletion or remote protection
change was made. This clarification is documentation-only; validate whitespace and active docs,
without repeating the already-passing Rust suites from the preceding milestone.

### LF-C06 local task coordination — 2026-09-07

Implemented the next bounded source slice under the owner's local-development policy. The new
CLI-only `local-task list/prepare/show/claim/submit/cancel/candidate-show` family reuses the nullable
Job association in the existing tasks table, immutable Blobs and audit/transaction primitives.
There is no schema migration, daemon, worker launcher, separate database or new approval system.
Existing legacy `task` commands remain rejected.

Tasks bind an exact Application/Pack/snapshot plus scoped Source/Profile/Evidence metadata and
Profile revision. Immediate transactions fence claims and submissions with a separate generation
and a 15-minute lease; stale inputs, stale generations and old leases fail. Candidate objects are
bounded to 256 KiB, immutable and explicitly untrusted. Listing returns at most 100 recent metadata
records; candidate bodies require explicit private-read consent and survive backup/restore.
Coordination audit uses host-agent, not an invented human approval. No Application revision or
business state changes, and no accepted-task state can misrepresent a later business commit.

Milestone checks: Store owning positive/negative regression passed (two connections, expiry and
reclaim, stale generation/lease/inputs, bounded candidates, archived Application, row consistency,
reopen and retained candidate). CLI binary-contract run passed 12/13 initially; the one failure
was a previous hard-coded MCP tool count, replaced with the actual public tool catalog and its
focused rerun passed. The new CLI handoff test passes private-read denial, all seven commands,
unchanged Application and exact backup/restore. Affected all-target Clippy and source check passed.
The coupling inventory accurately records the one new Store/test file reusing Job-null columns.

LF-C06's local coordination slice is source-complete. LF-C07 still owns actual two-Host work,
exact-candidate review and its existing Broker commit/receipt integration; LF-C08 owns the fuller
race/crash qualification. No real Host or release qualification is claimed. Continue from these
local results without a PR or another process approval.


### LF-C07 local candidate commit integration — 2026-09-07

Scope: connect an exact persisted Submitted local task to the existing Deliverable draft Broker.
The new MCP-only `local-task.draft.preview` requires native private-read consent bound to the
Application, task, generation and candidate digest. It validates the retained typed compose
request through the existing Pack-qualified draft validator. The existing `deliverable.draft.commit`
shows the Broker-owned exact candidate and requires actual native confirmation; no second commit
API, approval system, daemon, migration, branch or PR was added.

The owning Application transaction rechecks task state/generation, candidate bytes and digest,
Application snapshot and ancillary inputs, then saves both the draft and Committed task binding.
The durable completion metadata identifies the resulting Application revision/snapshot, while
its audit actor remains host-agent. It does not fabricate a separate human audit receipt.

Validation: the new synthetic MCP peer test passes private-read refusal, reviewer-process restart,
exact form binding, native False with no mutation, cancelled-token replay refusal, and fresh
preview/native True with matching committed metadata. The existing lifecycle also passes. The
all-tool binding test required new typed parameter samples and its updated scoped-tool count.
Store owns stale/cancelled/generation/digest/content checks and injected task-save failure rollback.
Affected all-target Clippy and Tier 2 source check pass; public inventory is 39 CLI / 131 Tauri /
40 MCP tools (29 read-only, 11 guarded). These are source checks, not protected CI or release facts.

Next: LF-C08 bounded race/crash qualification over these same transaction and recovery paths.
Actual two-Host user acceptance remains an LF-C07 qualification item; it does not block independent
local implementation. No upload, submission, push, PR or release qualification was performed.


### LF-C08 and first-stage source integration — 2026-09-07

The owner explicitly selected LF-C01–08 source integration and automated validation as the first
stage. Human Host acceptance and exact standalone/collaboration release qualification are outside
this completion claim; their existing gates remain open. Integration stays on local main with no
PR, push, new branch, GUI removal, new coordinator, or product consent bypass.

LF-C08 reuses the existing local-task fixture with independent SQLite connections and synchronized
threads. Competing claims, different submissions and distinct candidate commits each have exactly
one winner. A losing submitted task remains unchanged and the Application advances once. A
cancellation racing composition produces one complete outcome. Existing owning checks retain
stale-input, generation/lease expiry, altered-candidate, duplicate/replay and transaction-rollback
coverage. No production concurrency change was needed.

The MCP subprocess test now explicitly kills and waits for a reviewer while its native form is
unanswered, then verifies unchanged candidate metadata and refusal of its token after restart.
It also accepts a synthetic fixture form, waits for the durable commit without reading stdout,
kills that process and recovers the result through canonical state. A lost receipt cannot cause
a duplicate commit. These are actual process interruptions with synthetic consent, not real user
Host acceptance or OS power-loss qualification.

First-stage completion audit:

| Scope | Current evidence |
|---|---|
| LF-C01–02 CLI-first boundary | CLI default member and independent CI ownership; source dependency/operation checks |
| LF-C03 standalone resource lifecycle | Binary relocation/install/reinstall, isolated consumer environment, resource repair, Host setup/removal and backup/restore tests |
| LF-C04 exact headless approval | App Broker owning tests and MCP false/malformed/stale/replay/wrong-context/restart assertions |
| LF-C05 both Pack journeys | Dual-Pack MCP smoke completes source-bound Evidence, Requirement/Plan/draft/review/export and exact canonical reopen/restore |
| LF-C06 candidate coordination | CLI handoff/retained private candidate/backup restore plus Store lease and current-input tests |
| LF-C07 atomic candidate acceptance | Exact native form binding and one transaction for draft plus committed task revision/snapshot |
| LF-C08 competing/crashed operations | Three local-task owning tests including real concurrent connections; interrupted MCP reviewer and unread-receipt recovery |

Final milestone results: 13 binary-contract tests and 6 MCP protocol tests pass; App library
137 passed, 1 existing public-network test ignored; all 3 Store local-task tests pass. Affected
Clippy, formatting and source check pass. The Host and dual-Pack scripts now include the new
40-tool catalog (29 read-only, 11 guarded); the first smoke attempts exposed their stale count/list
assertions, which were corrected. Desktop bootstrap assertions now compare the shared catalogs
instead of independent stale counts; its focused bootstrap regression passes.

Successful smoke artifacts: `/private/tmp/canisend-first-stage-host-20260907-final` and
`/private/tmp/canisend-first-stage-mcp-20260907-final`. Backup/restore verifies canonical state and
original export manifests; it does not claim that scoped exported files are copied into restores.
The App/desktop test links emitted a nonfatal macOS debug-unwind-size warning; no test failed.

Next boundary: separately qualify actual single/two-Host journeys and exact standalone CLI bytes
under LF-C05/07/10/11 when that qualification work is selected. Do not automatically start optional
Hosts, cross-device synchronization or release publication after this source integration stage.


### LF-C10 local candidate automation integration — 2026-09-07

Continued the next roadmap boundary with a local release-profile CLI candidate. LF-C09 is optional;
LF-C12 remains deferred. This step supplies actual packaged-byte automation for LF-C10, not its
full native/human release qualification. No release record, cohort threshold, historical evidence,
GITHUB_RUN_ID or prescribed environment was fabricated or rewritten.

The candidate was built with `cargo build -p canisend-cli --profile release --locked` from product
source `7d41ce73c9b6fe40fffb9d5c2e64712f537683a3`, Rust 1.97.0, aarch64-apple-darwin. Actual host:
macOS 27.0 build 26A5425a, arm64; this is not the prescribed macos-15 qualification runner.
Existing native packaging and four-argument archive smoke were reused.

Fixed a discovered artifact-integrity gap: packaging now validates the executable's reported
build target before staging, and archive smoke compares the extracted runtime target and full
product identity with RELEASE.json. Raw archive inspection also exposed macOS AppleDouble entries;
packaging disables their inclusion, and the final archive contains exactly 12 expected files.
A new bounded shell regression rejects wrong-target packaging,
relabeled archives and stale RELEASE metadata before business smoke. It runs once in existing
Linux Fast CI. The isolated consumer check uses the extracted executable, empty HOME/PATH and an
unrelated cwd, retaining only Windows SystemRoot where required. jq is a harness/build dependency,
not a consumer runtime dependency.

The existing dual-Pack smoke now exercises generic CLI local-task prepare/claim/submit followed by
explicit private-read MCP preview and the existing native draft commit. It asserts exact committed
task/Application/digest binding; academic direct drafting and later review/export/restore stay
covered. Consent responses remain synthetic fixture inputs only.

Actual results:

- Release build, native package and extracted archive smoke passed: version/doctor/resources,
  isolated consumer init/check, documented two-Pack quickstart, project/global Host lifecycle,
  guarded dual-Pack flow with local candidate handoff, exact canonical reopen/restore and export
  verification, install/removal with Workspace retention.
- All three archive identity refusal fixtures passed; shell syntax and source check passed.
- Preserved pre-existing local Beta.1 binary reports source `0d11c456d726`, SHA-256
  `bccf0eade0703fe120b9f4ce37172ec482460992ed7cd5cb251ecfbdc97a2635`. Local replacement with
  the extracted Beta.2 binary preserved both Application snapshots exactly; the pre-upgrade backup
  restored and checked with both versions; removing the installed executable retained the Workspace.
  This is a local cross-version test, not the beta→rc archive-pair qualification policy.
- Full `xtask release check` passed source and dependency checks, then failed on the existing stale
  provider dogfood Skill identity/digest. Later release checks were not reached or claimed passed.

The superseding archive is in `dist/lfc10-local-20260907/packages-clean/`, with its completed
`archive-smoke-clean/` run. Earlier candidate output is retained as diagnostic evidence only.
Artifacts and body-free evidence are under `dist/lfc10-local-20260907/`; `validation.json` identifies
exact archive/executable digests, actual environment, commands/results and unqualified status.
The archive is a local candidate only. Real Host acceptance, clean native matrix, signing and
exact release/upgrade/cohort qualification remain open under LF-C05/07/10/11. No push, PR, tag,
release, platform-support expansion or human acceptance claim was made.


### LF-C11 local collaboration recovery evidence — 2026-09-07

The extracted LF-C10 release binary now runs the existing dual-Pack smoke with exact comparisons
of committed local-task metadata and private candidate JSON after both process reopen and backup
restore. Both recovered candidate reads reject missing private-read consent before the positive
explicit fixture read. The full flow passes at `dist/lfc11-local-20260907/`; its acceptance summary
records committed-task and private-candidate restoration separately from Application restoration.
This closes a gap in the previous automation, which checked Application snapshots and exports
but did not compare the committed task result and candidate payload after restoring the backup.

Shell syntax, diff validation and source check pass. No runtime code or candidate binary changed;
the test uses the same `0ea764742c7f4be9e55921290c4bc1a353b898bc97ec7c82b48fafa4833a8994`
extracted binary recorded in the LF-C10 local validation. Actual two-Host user acceptance and the
prescribed native/release matrix are still unperformed. Those missing evidence classes prevent
claiming full LF-C10/11 qualification; automated fixture responses cannot replace them.


### Local acceptance closeout and reusable simulated Host — 2026-09-07

The owner confirmed that this next integration stage ends at local automated acceptance and
requested a Host suitable for automated build tests. Reused the existing `McpProcess` protocol
fixture rather than adding an Agent service or a second approval engine. A test-only
`CANISEND_TEST_CLI_BINARY` override selects an absolute candidate executable for both the MCP
server and CLI cross-checks; without it the existing Cargo-built executable remains the default.

Fast CI already runs the full simulated Host suite. The native release workflow now additionally
runs that suite once on the extracted Linux GNU candidate using its existing release-profile
build cache. Scripted form responses and isolated fixture Workspaces stay in test code. No real
Host session, model account or interactive consent is required for these automated tests.

All six protocol tests pass against the exact local release archive's extracted binary, covering
form acceptance/refusal/malformed capabilities, context checks, replay, interruption and recovery.
CLI Clippy, formatting and source check pass. The guide documents the one-command override and
retains the explicitly untested full elapsed-time form-timeout case. Local LF-C10/11 acceptance
is complete under the clarified scope; real Host and native release qualification remain pending.

### CLI initialization onboarding — 2026-09-07

Owner requested a Skills installation choice during Workspace initialization. Reused `host setup`
for explicit `workspace init --host codex|claude|generic --scope project|global`; interactive
terminals offer Codex/Claude project/user locations and default to skipping. JSON, redirected
streams and `--no-skills` never prompt. Initialization remains an error on an existing Workspace;
if subsequent Skills setup fails, the error states that initialization succeeded and directs the
user to `host setup`. Existing user-file conflict protection is retained.

Initialization without setup now returns a next action explaining locations; setup prints the
installed directory and distinguishes resource readiness from unverified MCP registration.
The generated Workspace README and installation guide now describe the CLI-first startup flow.
No product consent, MCP registration, user global configuration or release record is modified.

Validation: all 14 CLI binary contract tests pass, including selected installation and refusal to
overwrite an unmanaged Skill. Eight local PTY cases pass (both Hosts/project+user, default skip,
invalid input, no-skills and JSON), with isolated HOME directories. CLI all-target Clippy,
formatting, diff validation and `xtask source check` pass. Local PTY reproduction and logs are
retained under `dist/init-onboarding-check/`. Previous packaged `552a072` bytes are unchanged;
this source slice needs a fresh package before distribution. Cargo/npm distribution remains the
next installation-channel scope; no registry publication or other package-manager work performed.

### Skills strengthening and upgrade mitigation — 2026-09-07

Owner authorized stronger project Skills and bounded upgrade mitigation using existing installers.
The four Skills now distinguish CLI setup from guarded business mutations, preserve previously
supplied choices, limit denial to the denied operation, and avoid inventing missing audit fields.
Workspace guidance covers initialization, scope/discovery, version/resource checks, conflict
preservation, reconnect/tool rediscovery and expired preview disposal. Host guides and Intake UI
metadata follow the same task ownership. No real consent or provider configuration is changed.

`host status` now returns state-specific next actions for setup/repair, conflict review or
reconnection, including selected Host/scope/directory and the unverified connection boundary.
The existing installer still owns digest preflight, file replacement and manifest-last writes;
no second upgrade subsystem or guaranteed atomic directory upgrade was introduced. A new owning
regression covers current/old/missing file mixtures, a late user-edit conflict with no partial
writes, resumed upgrade and repeat setup for both Hosts.

The resource suite exposed an existing task-model omission (`application.pack.show`) and a stale
fixed-phrase Skill assertion. Updated the resource projection to the existing canonical contract
and checked the current native confirmation field while retaining exact task coverage checks.
Historical release/Host evidence and protocol versions remain unchanged.

Checks: 17 resource tests and 14 CLI binary tests pass; affected all-target Clippy, formatting,
diff validation and source check pass. All four Skill frontmatter/UI metadata files parse and
match their names/default prompts (Ruby YAML validation; the Skill helper could not run because
PyYAML was unavailable). A local old-552a072-to-new-debug Skills update passed for Codex and
Claude, including same-version digest detection, repeat setup, healthy restored Workspace and
exact preservation of both completed Application snapshots. Fixture script/result and logs are
under `dist/skills-upgrade-check/`.

Next: rebuild the distributable candidate with these resources before Cargo/npm installation
validation. Existing archives are unchanged. Real-model Skill selection, human Host acceptance,
full power-loss/concurrent-install qualification and release publication are not claimed.

### Whole-application Skills — 2026-09-07

Owner clarified that Skills must support the complete application journey, beyond operational
boundaries and upgrade guidance. Added `canisend-application-workflow` as the discoverable entry
for outcome clarification, stage routing, resumption, local collaboration and final delivery.
It composes the existing ten canonical tasks rather than adding a task kind or state store.

Strengthened stage guidance for opportunity/constraint analysis, Profile-to-Evidence preparation,
Requirement decomposition, explicit fit/gap reasoning, material-set planning, academic and generic
writing criteria, scoped revision, substantive/cross-document review and export inspection.
Retained native consent, provenance, exact context, user-edit protection and no-submission limits.
Added end-to-end behavioral scenarios to the existing acceptance guide, explicitly separating
model writing/selection quality from synthetic protocol tests.

Integrated the fifth Skill into embedded declarations, all Host install/export/status/removal
paths, starter routing and future generated release bindings. Historical release records remain
unchanged. Regression coverage includes four-to-five Skill upgrades, canonical task ownership,
Host pack parity, idempotency and modified-file protection. Actual check results follow below.

Validation completed: 17 resource tests, the owning app Host-pack regression, 14 CLI binary tests,
Host setup lifecycle smoke and the full dual-Pack MCP lifecycle smoke pass. Both local exports,
exact reopen/restore and committed-candidate recovery pass with the updated binary. An actual
old four-Skill installation updated to five Skills for Codex and Claude, repeated setup was
unchanged, and both restored Application snapshots remained identical. All five frontmatter/UI
metadata files validate; affected all-target Clippy, formatting, shell syntax and source check pass.

The domain inventory now includes the Pack-conditional academic writing examples in the Materials
Skill (kernel guidance, no domain-specific API change). Regenerated only the current package
contract's resource-manifest binding (89 to 91 entries); no historical candidate evidence changed.
Local evidence is retained in `dist/application-workflow-upgrade-check/`,
`dist/application-workflow-host-check/` and `dist/application-workflow-business-check-final/`.
Initial smoke attempts stopped on old four-Skill inventory assertions; updated exact five-Skill
expectations passed. Earlier outputs remain diagnostic fixtures, not acceptance records.

The next step is a fresh distributable build and scenario-based actual model assessment when
requested. Existing archives remain unchanged; scripted candidates do not establish writing
quality, real Host approval, provider discovery or publication readiness.
