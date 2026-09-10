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

### Full Skills contract review — 2026-09-07

Reviewed all five SKILL.md bodies, five OpenAI metadata files and three Host guides against the
current CLI dispatch, MCP parameter types and Store preconditions. Fixed operational mismatches:

- Workspace: CLI-only Application creation and Profile Source import no longer depend on an
  invented MCP preview; distinguished explicit CLI private-read consent from MCP form requests.
- Intake: current build has no standalone Source intake/association MCP tool; use creation input
  and returned Sources, report missing replacement capability, and never recreate state silently.
  Requirement extraction and confirmation are distinct; confirmation covers the exact current set.
- Materials: proposal is not Plan confirmation; initial draft requires the complete catalog-valid
  material set and only works before materialization. Existing Deliverables use revise. Candidate
  reasoning precedes preview; the audit tool inspects stored drafts after commit.
- Review/export: disposition applies to the current Application revision, not arbitrary finding
  IDs or waivers; discover a callable reconciliation path and preserve private-data scope in viewers.
- Workflow and Host guides: routing now agrees with CLI-only operations and material-set handoff;
  no fallback to a denied write and no claims of unsupported Source/material-set update capability.

Checks: all five frontmatter/UI pairs validate; every referenced MCP tool exists in the current
40-tool catalog; 17 resource tests and source check pass. No runtime implementation changed.
The inventory scanner reclassifies Materials' explicit catalog-required academic set example
under academic-pack; updated its existing summary without adding a domain-specific API.
Added current-surface behavioral cases to the existing acceptance guide. Logs/tool reference
inventory are in `dist/all-skills-review/`. This is contract/content review, not a live-model
behavior or writing-quality pass. Existing archives remain unchanged; rebuild before distribution.

### Local Cargo/npm installation entrypoints — 2026-09-07

Added `scripts/smoke_local_installers.sh NEW_OUTPUT_DIRECTORY`: offline Cargo path installation,
private native npm tarball packing/install into an isolated prefix, exact installed-byte parity,
Workspace initialization with five Skills, existing Host setup/removal smoke and six synthetic
MCP protocol tests through each installed command. Uninstall preserves both fixture Workspaces.
The fixture reuses native bundle staging, includes license notices, and has no npm dependencies,
install scripts or runtime downloader. It currently supports native Unix hosts only.

The first run exposed a macOS npm symlink-entry failure during automatic Host setup. Canonicalize
only the inferred current executable in the shared CLI resolver; explicitly supplied symlinks
still fail the existing application validation. Added a positive/negative binary regression.

Checks passed on aarch64-apple-darwin: complete installer smoke in
`dist/local-installers-fixed/` (result.json and per-channel logs), symlink regression, affected
all-target Clippy, formatting, shell syntax and source check. Initial failed fixture remains in
`dist/local-installers-ca47583/` as diagnostic evidence. Cargo reported an existing yanked
chacha20 0.10.1 lockfile warning; installation succeeded with the unchanged locked dependencies.
No package was published and no user installation prefix was modified. Cargo remains unpublished;
the private npm fixture does not establish public package naming, multi-platform dispatch or
Windows installation qualification. Next: expand channel/platform qualification only when requested;
registry publication and real Host/model acceptance remain separate.

### Cargo/npm registry publication preparation — 2026-09-07

Owner authorized actual publication plus tag automation. Enabled crates.io publication only for
the eight CLI dependency crates; desktop and xtask remain private. All eight packages passed
Cargo's multi-package `publish --dry-run --locked --allow-dirty`, including isolated package
compilation. No uploads occurred. Existing locked chacha20 yanked warning remains.

Added npm native-package staging for the five existing CLI targets and a thin `canisend-cli`
launcher with exact optional dependency versions, libc selection, stdio and exit/signal handling.
Two launcher tests cover platform dispatch and failures. A real macOS native package plus launcher
was packed and installed locally; five-Skill init and six simulated Host tests passed. Other
platform runtime installations are not yet verified in this slice. Package names are proposed
pending owner confirmation; no npm package was uploaded.

Added reusable `package-registries.yml`, called by the existing tag release workflow after
verified public release success. It reuses verified assets, checks complete platform coverage,
runs Cargo dry-run and Linux npm/Host checks, then publishes the dependency crates, npm native
packages and CLI entry. Requires NPM_TOKEN/CARGO_REGISTRY_TOKEN Actions secrets. Workflow YAML,
Node syntax, launcher tests and source check pass; the remote workflow has not run or been pushed.

Publication remains blocked: npm whoami returned 401; no Cargo environment/file credential was
found; GitHub authentication works but repository secret listing is empty. Full release check
also fails on `provider dogfood Agent v4 contract binding is stale`; historical real-Host records
were not rewritten. No tag, push or real registry publication occurred. Logs are retained in
`dist/registry-preparation/`. Next: confirm package ownership/names, restore registry login and
CI secrets, and obtain current candidate qualification before publishing exact immutable versions.

### Public package names confirmed — 2026-09-07

Owner selected `canisend` for both crates.io and npm. Renamed the Cargo entry package while
retaining its existing source directory and Rust library target, updated workspace dependencies,
lockfile, dependency policy, active build/test commands and CI package selection. npm now emits
`canisend` plus `canisend-{platform}` optional native packages. Historical records stay unchanged.

Checks passed: eight-package Cargo publication dry-run (including isolated `canisend` build),
source check, workspace default-package assertion, two launcher tests, workflow YAML/shell syntax,
and packing/installing/running the renamed npm entry with the macOS ARM64 native package.
Public npm and Cargo sparse-index name lookups returned 404; this is not a reservation or proof
of publication rights. Logs: `/tmp/canisend-rename-publish.log`, `/tmp/canisend-rename-source.log`;
local npm artifacts: `dist/npm-canisend-name/`. Publication credentials and current release
qualification remain outstanding as recorded above; no registry upload, tag or push occurred.

### Recover completed stages instead of repeating confirmation — 2026-09-07

Owner reported repeated acceptance stops and requested smoother application flow. The decisive
record was already present: `host-acceptance-true-2026-09-07.json` reports a successful native
Requirement commit at 20:08:39Z, confirmed_by=user, academic revision 2, and unchanged generic
Application. A fresh canonical CLI read independently matches snapshot
`f7466b6de48ae990707ce575b1535fca0a90361addac827c0d7beb87749f3db0`.
The subsequent conflict was a duplicate decision after success, not failure to persist approval.
The coordinating agent had relied on the older rejection recap and incorrectly requested another
True test. Preserve all three records (rejection, success, repeat conflict) without relabelling.

Added shared ApplicationModelConflict remediation: read canonical Application/stage state,
reuse an already-matching result, otherwise reconcile before a fresh preview. The conflict remains
non-retryable and cannot issue a consent token or new commit receipt. Strengthened Workflow/Intake
Skills and acceptance guidance to read latest results before repeating a step, skip completed
stages, and stop only the denied mutation rather than unrelated authorized work. Updated the
local acceptance launcher to resume from actual state instead of always starting the False case.
Existing candidate binary and installed candidate Skills remain untouched for evidence identity.

The owning MCP lifecycle regression now confirms Requirements, rejects a duplicate preview with
read-state remediation, verifies no additional form and identical state, then proceeds through
Plan and the rest of the existing lifecycle. It passes. Affected all-target Clippy, formatting,
source check and diff check pass. Package bindings were inspected and remain unchanged because
no declaration/catalog inventory changed. The new Skill behavior is source guidance, not a fresh
live-model acceptance claim. Rebuild and install the new resources before testing changed Host
behavior; no repeated Requirement confirmation is needed on the completed original fixture.
No native approval was supplied by the agent and no registry publication occurred.

### Application journey versus engineering completion — 2026-09-07

Owner requested continued development and an explicit mapping between the user's application
journey, implemented features and the acceptance work. LF-C01–08 source integration and local
LF-C10/11 automation are complete as scoped; this does not mean every future application-editing
scenario or public release qualification is complete.

| User journey | Existing implementation / source evidence | Remaining user-level evidence or capability |
|---|---|---|
| Install and initialize | Rust CLI, embedded Packs/five Skills, scoped setup/upgrade/remove, local Cargo/npm installation | Native platform matrix and actual registry installations after publication |
| Understand call and verify applicant facts | Source-backed creation, Requirements, Profile Sources, Evidence confirmation/association | Real Host content reasoning; existing Source replacement after downstream work lacks a complete public adapter path |
| Decide fit and materials | Exact Pack catalog, Requirement decisions, proceed/hold Plan proposal and confirmation | Complete the current Host's Evidence and dual-Pack Plan steps; assess gap handling |
| Draft, revise, review and export | Full-set initial drafts, individual revisions, evidence audit, review disposition, scoped local export | Actual writing quality and visual document inspection; original source/material-set changes need explicit supported paths |
| Resume and collaborate | Canonical reads, backup/restore, leases, persisted candidates and interrupted-commit recovery | Second real session resumption; two-Host qualification only for the collaboration claim |
| Submit to external portal | Outside current product scope | CanISend does not upload or submit Applications |

Recent False/True work is LF-C04 consent behavior observed through LF-C05 real Host qualification,
feeding the LF-C10 candidate evidence. It is one integration boundary, not the entire application
journey or a development stage to repeat for every Requirement. The successful True observation
is already recorded; continuing means the first unfinished application dependency. Run owning
positive/negative automated regressions for changed boundaries; repeat human cases only for
changed interactions or required exact candidate binding. Registry build/publish CI belongs to
LF-C03/10 distribution and must not be confused with model writing-quality validation.

Next implemented slice: improve existing CLI `application show` as the continuation entrypoint.
It now prints proposed/confirmed/excluded Requirement counts, Plan state/decision/blocker count,
and each Deliverable state. Existing `next_actions` points to the relevant read operation based
on canonical metadata; the snapshot response remains unchanged. These are navigation hints,
not executable permissions, automatic writes, evidence-readiness or export-readiness claims.
No new command, workflow store or confirmation mechanism was introduced.

Validation: the existing mixed-Pack CLI test checks navigation for unfinished Requirements; the
MCP lifecycle test checks navigation after confirmation and exact unchanged snapshot data before
proceeding to Plan. Both pass. A PTY smoke on the original confirmed fixture displayed one confirmed
Requirement, no Plan and the catalog/Evidence next read. The initial text-output assertion used a
pipe, where JSON is intentional; retained the proper JSON contract assertion and inspected text
through a PTY. Removed duplicate next-action rendering found there. Affected all-target Clippy,
formatting, source check and diff check pass. No Application mutation or additional native form
was performed. Future release artifacts need rebuilding to include this source change.

Next development priority: audit and complete bounded source/requirement change handling for an
existing Application, with explicit downstream invalidation and recovery, before claiming the
whole iterative application experience complete. Continue the original real Host fixture at its
first unmet dependency; do not repeat the successful Requirement confirmation. Publication remains
subject to the owner's retained Beta conditions.

### Iterative application changes: execution plan — 2026-09-07

Owner authorized planning and implementation of the next development stage. Outcome: revise an
existing Application without recreating it or silently treating old Plans/materials as current.
Reuse existing repository dependency invalidation, immutable revision history, Blob references,
ApprovalBroker and native forms. No new workflow database, background service or migration is
needed for the initial slice. Preserve the current qualified fixture and historical evidence.

| Slice | Scope | Acceptance | State |
|---|---|---|---|
| I1 | Read-only exact update-impact preview at the repository owner; use the same transition and invalidation rules as commit | Preview and commit agree on projected snapshot/digest and stale IDs; preview writes no head/history/audit; bad revision/identity/deletion fails; concurrent change invalidates the expected revision | Complete locally |
| I2 | Typed single-Requirement revision with source/span checks, private-read scope and native preview/commit; expose through existing adapters with explicit schema changes | Retain Requirement identity, increment its revision, clear obsolete confirmation, stale only affected downstream work; no-op is read/completed; denial/replay preserve state | Complete locally (CLI/MCP; recovery remains I3) |
| I3 | Explicit Source revision/association updates and usable re-confirm/replan/revise paths after invalidation | Keep original bytes and lineage; preserve unaffected Evidence/drafts; do not auto-approve revised content or reuse stale exports; unsupported material-set changes are explicit | I3a and bounded I3b pasted-text updates complete locally; shared/file/URL updates remain |
| I4 | Dual-Pack end-to-end revision/recovery automation and updated Skills/CLI guidance; rebuild and perform one bounded real Host journey | Both Packs revise existing input through new consent to current materials/export; old revisions recoverable; conflicting writers and interrupted responses resolve from canonical state | Pending I2–I3 |

Implementation notes: ApplicationModelRepository::prepare_update already owns Requirement-to-Plan
and Plan/Evidence-to-Deliverable invalidation. Existing v4 extraction only adds proposals before
confirmation and cannot serve as an implicit correction API. Do not loosen it globally or reuse
an old confirmation token. I1 exposes the shared calculation without opening a new mutation route.
A projected snapshot is neither durable state nor permission and must never be presented as a
commit receipt. I2 must distinguish editing an existing decision from repeating the same decision.
Source revision/association atomicity and exact-set confirmation after partial correction must be
resolved in I2/I3 before their public adapter paths are declared complete.

Checks: owning Store tests per slice; relevant formatting/Clippy for Rust; source gate at adapter/
contract integration. Do not repeat unrelated native approval cases during I1. Candidate-specific
Host and release gates remain open without blocking source implementation; no publication in this
stage. Reuse the current branch and record meaningful local commits, without routine PR creation.

I1 completed: `ApplicationModelRepository::preview_update` reads current state, verifies the base
revision, and calls the same `prepare_update` plus semantic validation as normal commit. Its
separate preview type contains base/projected digests and exact stale Plan/Deliverable IDs, without
commit time, actor, receipt or grant. The commit path still recalculates and validates inside its
transaction; the original candidate is committed, never a caller-forged stale projection.

Extended the existing Requirement invalidation test to prove preview/commit parity, unchanged
head/history/audit after preview and rejection of stale preview/commit after another revision.
Added a bounded negative test for wrong base revision, wrong identity, persisted Requirement
deletion, forged stale Plan and semantically invalid Requirement content; all leave canonical
state/history/audit unchanged. Both tests, Store all-target Clippy, formatting and diff check pass.
Logs are `/tmp/canisend-impact-test.log` and `/tmp/canisend-impact-clippy.log`. No schema migration,
CLI/MCP tool, business mutation or real Host confirmation was added/run. I2 is the next slice:
construct validated typed corrections and bind the exact impact into the existing native-consent
preview/commit workflow before exposing a user-facing edit operation.


I2 completed locally — 2026-09-07: added typed `ApplicationRequirementReviseRequestV4` and
`canisend_requirement_revise_preview` / `canisend_requirement_revise_commit` through the CLI's
existing MCP server. The request can replace one existing Requirement's category, statement,
priority and exact associated Source span. Source association/revision/digest, normalized UTF-8
span and Pack validation are shared with extraction. Identity and history remain; changed content
increments the Requirement revision, clears its confirmation and lets the repository stale exact
downstream revisions. Existing decisions and immutable material content are preserved.

The Broker binds the current snapshot, typed request and exact semantic impact (including any
Opportunity Source reference addition). Commit timestamps remain repository-owned and are not
promised as a projected final digest. An unchanged request returns `preview.status=unchanged`
without an approval token or mutation confirmation. Private Source reads retain their separate
consent boundary. Changed requests use native confirmation with no model-supplied `approved`
field. No unrestricted snapshot update, new CLI mutation switch or desktop adapter was added.
The operation registry marks these as canonical MCP leaves; this does not claim Tauri parity.

Inspection also found that Source associations can change independently of the Application
revision. Both extraction and revision now recheck the exact current association inside the
Application write transaction, sharing one Store-owned validator. The regression validates a
candidate, unlinks its Source, then exercises the commit boundary directly: no Application head,
history or audit change. A positive associated-Source case matches the projected digest.

Validation: the Store transaction regression and all seven Application mutation tests pass,
including no-op, denial/replay/stale grants, private reads, invalid Source/span/category/identity,
explicit Source rebinding and correction after material export. Five MCP protocol tests passed
in the complete run; the cross-Application catalog test initially lacked the new typed field and
scoped-tool count. After updating those fixtures, its focused rerun passes for both Packs. The
simulated native False/True correction flow passes and issues no form for unchanged content or
replay. Affected all-target Clippy, formatting, shell syntax, diff and `xtask source check` pass.
The catalog now has 42 tools (30 read/preview, 12 guarded writes); embedded registry, active package
contract digest and smoke inventories are synchronized. No generated schema file drift occurred.
Logs: `/tmp/canisend-i2-{transaction,tests,rebind,mcp,binding,clippy,source}.log`.

Next is I3: explicitly revise/associate Sources and make partial Requirement re-confirmation,
stale Plan replacement and material recovery usable together. The existing exact-set confirmation
and existing-Plan guards remain; Intake guidance states this recovery limitation instead of
instructing another doomed confirmation. I2 is a bounded correction API, not acceptance of the
complete iterative journey. No real Host consent, candidate evidence refresh, publication, tag,
push or PR was performed; release and live-model qualification remain separate.


I3a recovery completed locally — 2026-09-07: split I3 into independently verifiable recovery
and Source-update work. This milestone closes the existing-input correction path from I2 through
fresh material review/export. I3b still owns public Source revision/association updates; I3 as a
whole and I4 dual-Pack/recovery acceptance are not complete.

Requirement confirmation now decides exactly all currently proposed entries and preserves the
identity, revision and decision of other entries. Empty completed sets, omitted proposals and
attempts to overwrite existing decisions fail without a grant. Confirmation preview/commit share
one validated candidate builder and the repository impact calculation. Plan proposal can rebuild
only a stale Plan, keeping its UUID, advancing its revision, clearing old confirmation and binding
current confirmed Requirements. Existing material kinds/counts must match; additions/removals fail
with an explicit unsupported-change message. Draft Plan confirmation accepts existing stale
materials, which remain unchanged until individual Deliverable revisions under the new Plan.
Review and export remain blocked while those materials are stale. All typed mutations reject
archived Applications before issuing a preview grant.

Repository invalidation also handles decisions missing from the old Plan input list: reopening
an excluded/confirmed decision or adding a newly confirmed criterion makes that Plan and its
current outputs stale. Merely excluding an unconsumed proposal leaves unrelated inputs current.
A focused positive/negative regression proves preview/commit parity for those cases. No manual
stale transitions, unrestricted snapshot API, migration, additional tool or approval mechanism
was introduced. Shared Store validation replaces duplicated commit-only checks.

Checks passed: two focused Store impact tests; all seven Application mutation tests, extended
through correction, partial confirmation, replan, material regeneration, review and export; an
additional focused archived-Application regression; and the simulated MCP Host's generic lifecycle
through the same recovery sequence. The protocol check verifies unchanged unrelated decisions,
Plan identity, stale material preservation before regeneration and the original export manifest.
Negative coverage includes missing/overwritten decisions, denied confirmation, premature Plan,
unsupported material-set changes, stale review/export and archived mutation. Existing I2 replay
and stale-grant assertions remain in the owning tests. Relevant all-target Clippy, formatting,
diff checks and `xtask source check` pass. Logs: `/tmp/canisend-i3-{store,app,archive,mcp,clippy,source}.log`.
Intake/Materials Skills, MCP descriptions and public behavior contracts now explain this path.
No source schema/catalog inventory or active package binding changed.

Next: I3b, a bounded Source-update preview/commit with exact revision/association impact, immutable
bytes/history and explicit handling of shared Source references; then I4 dual-Pack iterative
resumption acceptance. This run did not test new Source imports/replacements, the desktop recovery
UI, real Host/model behavior, or a fresh native package. No real consent form, candidate evidence
rewrite, publication, tag, push or PR was performed.

### Current-source Cargo/npm distribution candidate — 2026-09-07

Owner requested distribution and registry publication before I3b. Built clean source
`c5ac46a4c7834db06c38610a601fe655eccd9088` with the release profile, signed the macOS ARM64
executable using the existing ad-hoc policy, and reused native/npm packaging. Artifacts and exact
hashes are retained in `dist/registry-c5ac46a/` (`README.md`, `result.json`, `SHA256SUMS`): eight
Cargo source packages, the npm `canisend` entry and macOS ARM64 native package, and the standalone
archive. Other native targets are not included; npm `packages.json` correctly says `complete: false`.

Checks passed: all eight crates compiled in Cargo publication dry-run; native archive consumer,
dual-Pack lifecycle, reopen/restore, export and uninstall smoke; npm installation, exact binary
parity, five Skills in `.agents/skills`, two launcher tests, and all six simulated Host protocol
tests against the installed npm command. Bounded path/config/token-pattern inspection found no
matches in the ten registry archives. It is not an exhaustive secret audit. Existing locked
`chacha20 0.10.1` yanked warning remains.

Initial Cargo preflight failures were traced to same-version temporary-registry caches from
`cd479fd`, not missing source files. Newly generated tarballs contained the revision interfaces,
but downstream crates reused old registry sources and compiled metadata. Backed up seven old
CanISend source-cache directories and sixteen corresponding temporary-registry build fingerprints;
the original eight-package preflight then passed. Failed and successful logs remain under `checks/`.
Use a fresh `CARGO_TARGET_DIR` for future same-version publication preflights to isolate their
temporary registries. No product source or dependency version was changed for this recovery.

npm login is verified as `jxpeng98`; Cargo credentials are configured. Registry name checks returned
404 for both entry names, seven Cargo dependency names and five npm native names. This does not
reserve them or prove upload permission. GitHub Actions secrets are still absent, and existing
remote main/tag protections remain intact. The full release check still fails on historical
provider-dogfood/current Agent contract binding; no historical record was rewritten, and no real
Host form was answered. The owner previously retained Beta release conditions, so these locally
verified artifacts are not qualified for upload. No publication, tag, push or PR occurred.

Next for distribution: resolve historical/current release-evidence binding, qualify changed Host
behavior and the complete native matrix, integrate through existing remote protections, then
publish the dependency crates and npm native packages before the `canisend` entry. Configure the
existing tag workflow's registry credentials as part of that publication. I3b remains deferred.

### Owner-directed local npm testing publication — 2026-09-07

After the remaining Beta and CI conditions were disclosed, the owner requested a new version and
manual npm CLI upload using the existing local login, with GitHub CI setup/publication deferred
to the owner. Scope this run to npm `next`; Cargo versions stay aligned but no Cargo upload is
performed. This supersedes the previous upload stop for this bounded testing distribution only.
Full qualified-checkpoint, native-matrix, real-Host and cohort claims remain pending.

Advance active source projections to `1.0.0-beta.3` without rewriting historical provider evidence,
Beta readiness/freeze or qualification history. The npm packer now derives dependencies from the
supplied bundles; a single-platform entry declares its OS/CPU/libc restrictions, and its generated
README lists the actual supported targets. The full five-target CI completeness check remains.
One packaging regression exercises both a single-platform package and the full native matrix.
The manual npm entry also includes the exact committed source archive so the matching CLI source
is available without waiting for the later GitHub integration.

Acceptance: source check, fresh release build, exact archive and installed npm lifecycle checks,
local publication preflight, native-package-first upload, registry readback and clean installation
from npm. Keep actual upload results and artifact hashes under `dist/npm-beta3/`; record results
here after the attempted publication. No GitHub tag, push, PR or CI-secret configuration is needed
for this owner-authorized npm testing publication.

Result: published `canisend-darwin-arm64@1.0.0-beta.3` first, verified its registry digest, then
published `canisend@1.0.0-beta.3`. npm requested separate browser verification for each publish;
the owner completed the npm verification and both CLI invocations returned success. No CanISend
product confirmation was involved. The `next` channel is available; registry readback also shows
`latest` assigned to this first version. This release supports macOS Apple Silicon only.

Exact source is local commit `9774d125fdbd55583475103171de0cdbb6488571`. Node launcher/packaging
tests (three), source check, fresh release build, ad-hoc signature verification, archive smoke,
local npm install, and all six simulated Host tests passed. A new cache and isolated install prefix
downloaded `canisend@next` from the public registry; package SHA-512/SHA-1, native bytes, included
source bytes and product identity match the prepared artifacts. Workspace initialization installed
all five Skills in `.agents/skills`, and Workspace check passed. Actual records and SHA-256 values:
`dist/npm-beta3/publication.json` and `dist/npm-beta3/checks/registry-*.json`.

The isolated global-prefix npm install also triggered the machine's mise maintenance, whose log
records removal of older Claude Code, Antigravity CLI, Quarto, Codex and Bun versions. A read-only
installed-tool check confirmed all five currently active versions remain installed. This was an
environment side effect, not a CanISend install script; npm package scripts were disabled.
Prefer a nonglobal npm prefix for future isolated verification to avoid global-install shim hooks.

The full release check still reports historical provider-dogfood/current Agent binding drift;
historical evidence and the qualification ledger remain byte-identical. Cargo was not uploaded;
no GitHub push, tag, PR, CI configuration or CI secret change was made. Future automated uploads
must use a new version, since both published npm version records are immutable. The manual npm
publication request is complete; I3b can resume independently of formal Beta qualification.

### Single-package npm distribution — 2026-09-08

The owner requested one npm package instead of `canisend` plus `canisend-darwin-arm64`.
Advance source to `1.0.0-beta.4` and embed supplied native executables in `canisend/native/`.
The existing launcher selects the local platform path. No dependencies, install script or binary
download is needed. The packer and existing CI recipe now emit and publish exactly one archive;
the full CI matrix includes all five executables in that archive. Historical Beta.3 packages
remain available for existing installations. This manual testing version still supports only the
locally verified macOS ARM64 target; the multi-target packaging test uses synthetic fixtures.

Reuse the owner's manual npm `next` publication authority. Verify the new source, native archive,
single tarball, nonglobal isolated install and simulated Host before uploading only `canisend`.
Then independently download from npm and compare exact bytes, version and Workspace/Skills setup.
Keep artifacts and actual results under `dist/npm-beta4/`. Cargo publication, GitHub integration
and formal Beta qualification remain outside this run. Historical evidence is preserved.

Result: published only `canisend@1.0.0-beta.4` after the owner completed npm browser verification.
Source commit: `2886c3c6a652aefedf07b2a011141777d66fec57`. The 26,104,790-byte archive embeds the
macOS ARM64 executable and matching source archive; its SHA-256 is
`032243b6fab7e86d531e5893e31e141bb40a169742a9da60439e952bcfc25b55`.
Node packaging/dispatch checks, source gate, release build, signing and archive lifecycle checks,
isolated single-package install, all six simulated Host tests and publication dry-run passed.

Public npm readback matches the local SHA-512/SHA-1. A fresh nonglobal prefix/cache installed
exactly one package with optional dependencies disabled, verified the embedded executable/source
bytes, initialized Workspace with all five Skills and checked healthy state. The generated MCP
configuration points to the executable inside `canisend/native/darwin-arm64/`. `next` now points
to Beta.4; `latest` remains Beta.3. Actual evidence: `dist/npm-beta4/publication.json` and `checks/`.
No old package was removed or republished. The existing tag workflow was updated locally to emit
and upload one archive; no CI configuration, GitHub push/tag/PR or Cargo upload occurred.

Full release qualification still reports historical provider-dogfood/current Agent binding drift;
all five retained evidence files remain byte-identical. The single-package npm request is complete.
Next independent development remains I3b; broader native/Host qualification and CI activation retain
their existing scope.

### npm package README refresh — 2026-09-08

Owner requested necessary installation guidance in the published package README, edited with
humanizer's faithful, idiomatic and restrained English style. Update the existing packer text
and root README: version checks, supplied platforms, Rust/native versus Node launcher behavior,
Workspace and five Skills, explicit MCP registration/reconnection, application scope, privacy,
backup and upgrade, stale command resolution, license and corresponding source. Preserve actual
platform/version interpolation and all product consent/evidence controls.

Checks: all three existing npm packaging/dispatch tests pass; the actual local tarball README
matches its rendered preview, has no unresolved template escapes, lists only the supplied target,
and retains identical native bytes. Preview: `dist/npm-readme-preview-20260908/canisend/README.md`;
verification: `dist/npm-readme-preview-20260908/readme-verification.json`. This is a documentation
preview using Beta.4 bytes, not a new release. Existing publication archives are unchanged.
Next publication must use a new version to deliver this README through npm. No Rust tests, native
rebuild, registry mutation, version bump, or GitHub integration is needed for this wording change.


### Operational advantages: bounded I3b — 2026-09-08

Owner requested that the product's practical advantages become concrete, traceable and consistent
capabilities. Reuse the iterative Application plan and existing Rust/SQLite/Blob/MCP boundaries.
This milestone implements Source correction for one existing Application; it does not introduce
another workflow store, scheduler, approval mechanism or release qualification stage.

| Advantage | Observable capability and acceptance | Runnable evidence | Scope |
|---|---|---|---|
| Fewer omissions after input changes | Preview names the Source revision/digest and affected Requirement, Plan and Deliverable identities; approved Source/association/Application changes commit together; dependent work becomes stale and cannot be exported as current | `guarded_mutations_split_requirements_plan_and_deliverables_without_replay` in `crates/canisend-app/src/application_mutations_v4.rs` | Existing generic recovery fixture, extended through Source correction after reviewed export |
| Less repeated work when continuing | Same Source/interpretation returns `unchanged` without a grant; fresh state after Host restart retains IDs and completed changes; denial/replay/conflict does not create another revision | `source_revision_survives_host_restart_in_both_packs` in `crates/canisend-cli/tests/mcp_protocol.rs` | Synthetic generic and academic MCP Hosts, separate False and fresh True requests |
| Traceable materials and inputs | Retain old Source bytes/digest and Application history; preserve material contents when marking them stale; malformed requests and transaction failure cannot leave partial Source, association, Application or audit authority | `source_revision_preserves_history_and_rolls_back_partial_writes` in `crates/canisend-app/src/application_mutations_v4.rs`, plus the recovery fixture above | Isolated temporary Workspace, invalid spans/digests/set, rejected grant, competing preview, injected SQLite failure and post-preview shared association |

Implemented `canisend_source_revise_preview` and `canisend_source_revise_commit`. Supply the exact
current Application revision, associated Source reference, replacement pasted text and a map of
every existing Requirement using that Source, keyed by its unchanged ID. Validate the Pack and
UTF-8 spans before staging immutable bytes. Commit rechecks exclusive Source association inside
the same transaction as the Application revision. Reuse the existing native form and single-use
Broker; no production confirmation response is synthesized. All affected Requirements return to
proposed and follow I3a's existing confirm/replan/material-review recovery.

The first public adapter is MCP (served by the CLI); no direct `source revise` CLI command is
claimed. File/PDF/URL and shared Source updates, Requirement addition/removal, and material-set
changes remain explicit limitations. Pasted text is supplied directly; this route does not fetch
or reread the original file/URL. A rejected database transaction may leave unreferenced immutable
staged Blobs for ordinary garbage collection, but no partial authoritative revision or audit.

Intake and Workflow Skills, Agent v4/association contracts, operation catalogs and the active
package operation digest are synchronized. Historical release receipts/ledgers are unchanged.
The private-file fixture also verifies that this route cannot relabel a local-file Source as
pasted text. All 44 MCP tools remain covered by the Application binding inventory.

Validation commands (local source acceptance):

- `cargo test -p canisend-app --lib --locked --offline source_revision_preserves_history`
- `cargo test -p canisend-app --lib --locked --offline guarded_mutations_split_requirements_plan_and_deliverables_without_replay`
- `cargo test -p canisend-app --lib --locked --offline requirement_extraction_requires_consent_for_private_local_source_reads`
- `cargo test -p canisend --test mcp_protocol --locked --offline` (7 cases; after the inventory
  fixture correction, rerun its `application_binding_covers_every_tool` case)
- Changed-crate all-target Clippy with `-D warnings`, formatting and `git diff --check`.
- `cargo run -p xtask --locked --offline -- source check`.
- Skill Creator `quick_validate.py` on the two edited Skills, using the repository `.venv`
  because system/bundled Python lacks PyYAML.

Results: all three focused Application checks pass; all seven MCP protocol cases pass across the
suite run and the focused inventory-fixture rerun. Changed-crate all-target Clippy, final App
Clippy, formatting, diff check, both Skill validators and the final source gate pass. The macOS
App test linker emitted its large unwind-section warning; no test failed after fixture fixes.
Source gate reports 44 MCP leaves and no new dependency or migration. No version bump, registry
publication, remote CI run or real Host acceptance is claimed. Next: I4 dual-Pack complete Source-change-to-new-export
acceptance using these supported operations; shared/file/URL revision requires its own bounded
intake/provenance design. Human candidate qualification remains separate from this local milestone.

### npm Beta.5 publication — 2026-09-08

Owner authorized publishing the new CLI. Reuse the local `next` single-package macOS ARM64
distribution exception, include ModernPro CV 2.1.1 and coverletter 1.0.2, source-revision support,
and the current installation README. Preserve historical release/Host qualification records.
The general sequential-stage command requires a qualified active Beta; this bounded npm source
iteration reuses the prior Beta.4 controlled source-version update without altering that gate.
Acceptance: source check, release build, signed native archive smoke, local npm install and
resource/Workspace checks, then npm publication and fresh registry-byte verification.
Evidence destination: `dist/npm-beta5/`. npm credentials currently need browser reauthentication.
Next: validate the candidate before login and publication; no GitHub/Cargo release is claimed.

Candidate preflight found two omissions from the preceding source-revision slice: Host smoke
counts still expected 42/30/12 instead of 44/31/13, and the declared Requirements task omitted
`source.revise.` even though the operation registry assigned both revision phases to that task.
Add the narrow prefix to the shared task contract and embedded model, retain intake isolation,
and update exact smoke counts. Rebuild the candidate/source archive after the fix; earlier
Beta.5 preflight bytes are not publication bytes. The previously failing resource-registry
semantic test is an acceptance check for this fix.

Final candidate: source `b24545cad0eb7d6ad8b6060e3f9d09ddc1a21f4d`, macOS ARM64,
`dist/npm-beta5/npm/canisend-1.0.0-beta.5.tgz` (26,206,000 bytes), SHA-256
`41c2f6a0c6dbfa044db17ebbe241b1dae837b044bb40ad7fa6fda0f3cb9d255b`.
Source and Clippy gates, 17 resource tests, task-ownership regression, three npm tests, seven
exact-binary MCP tests, signing, isolated npm installation, and complete archive lifecycle smoke
passed. The smoke's explicit tool-name list now includes the two source-revision tools. The
source archive and native bytes match the installed package. Publication dry-run passed.
Full qualification is not claimed: the retained dependency advisory review expired on 2026-09-07,
and historical real-Host gates remain separate. No exception date or qualified record was changed.
Publication currently awaits npm browser reauthentication after `npm whoami` returned E401.
Next: publish this exact tarball with `--tag next`, independently install from npm, and compare
registry digest, native/source bytes, template versions, and Workspace/Skills initialization.

### CLI-first CI pause — 2026-09-08

Owner requested pausing GUI/App Actions and retaining essential CLI automation. Fast CI's
desktop UI and browser accessibility jobs now require `CANISEND_ENABLE_DESKTOP_CI=true`;
the repository variable is false. GitHub's native workflow disable control pauses
`desktop-platform-qualification.yml` and `intel-gui-compile.yml` without deleting their recipes.
CLI Linux/Windows tests, macOS quality/tests, and shared Application-layer tests stay enabled.
The Python product-file guard explicitly allows the two reviewed template maintenance scripts.

Validation: YAML parsing and assertions confirm desktop gating and independent CLI jobs;
`cargo run -p xtask --locked -- source check` and `git diff --check` pass locally.
The automatic approval review rejected broader disabling of release/fuzz/upgrade/package-manager
workflows as outside explicit GUI/App scope; those workflows remain unchanged. Formal releases
still require their GUI evidence, and no publication or qualification is claimed here.
Next: synchronize the existing local CLI milestone and this workflow change to main, verify
GitHub job selection, and obtain explicit owner scope before pausing non-desktop workflows.

### Confirmed nonessential workflow pause — 2026-09-08

The owner explicitly confirmed pausing release, fuzz, upgrade, and package-manager
workflows, and requested archival for later reuse. Disable those workflows plus the
obsolete Rust spike through GitHub, retaining Fast CI and dependency assurance.
Save exact snapshots of eight current workflow files and the last historical Rust
spike in `.github/workflow-archive/2026-09-08/`, with hashes and restoration guidance.
Keep original definitions at validator-owned paths; the reusable registry workflow
has no independent trigger and its release caller is disabled. No release check,
qualification record, or dependency exception is relaxed.

Acceptance: snapshot hashes and original-byte comparison, YAML parsing, diff check,
and remote workflow-state inventory. Continue PR #230 for the Fast CI desktop gate;
existing source checks passed before this snapshot-only change. Remote CI and merge
remain separate acceptance facts, not implied by archival or workflow disabling.

### Merge and direct npm release — 2026-09-08

Owner requested merging the CLI milestone and enabling direct npm publication through
`release.yml`. A mandatory macOS contract test still expected 40 MCP bindings; the
registry and other inventories correctly contain 44. Its focused regression and
formatting pass after correcting that count. The expired dependency review remains
reported separately; no exception date or protection rule is changed.

Add a manually dispatched, main-only npm prerelease job in the trusted publisher's
existing workflow filename. Reuse native packaging, archive lifecycle smoke, npm
packaging tests, and exact MCP checks; include the matching source archive and embedded
templates. Publish only the supplied macOS ARM64 platform to `next` with OIDC, then
compare registry/native/source bytes and initialize a fresh Workspace. Original full
release jobs require an explicit full-release variable and `npm_only=false`; GUI,
Cargo, and GitHub release publication remain paused by default.

Local evidence: MCP inventory regression, formatting, source gate, three npm tests,
workflow YAML and shell syntax, job permissions/route assertions, and diff check pass.
Next: required PR checks, normal merge, enable only the new npm release route and run
it on main; record the actual registry result without claiming formal qualification.

The next complete remote suite exposed one additional stale fixture: the freeze
regression expected four Skills rather than the five currently shipped. The other
90 xtask tests passed. Correct only the cardinality assertion; retain all malformed,
legacy, and unbound freeze rejection cases. Linux, Windows, and macOS quality gates
passed on `15fb346`; the corrected head still needs its own required CI run.

### Agent instruction review and Beta.6 preparation — 2026-09-10

Scope: review repository guidance first, simplify shipped Host guides/Skills second,
then review the result for routing, consent, recovery and installation consistency.
Prepare the next npm CLI candidate without claiming publication or full qualification.

Audit decisions:

- Remove 52 tracked Claude/Trellis adapter files (339,653 bytes): their hooks still
  injected a task/dispatch workflow that the owner had already removed. Keep retained
  `.trellis/` project records and user-local/global settings; Git preserves the adapters.
- Root AGENTS and project control shrink from 1,691 to 644 words. Add a small CLAUDE
  entrypoint referring to the same AGENTS file, instead of reintroducing hooks.
- Keep the five stable, distinct Skill entrypoints. Merge shared state/consent/retry
  rules into Workspace and link stage Skills to that installed sibling. Skill text
  drops from 4,590 to 2,088 words; Host guides from 718 to 368 words. These are text
  measurements, not claims of measured model accuracy or usage improvements.
- Clarify actual adapter limits, complete-set drafting, unchanged previews, read-before-
  retry and modified-file preservation. Discover schemas per connection/version change
  and refresh affected state rather than replaying the whole workflow. Guide/changed
  Skill resources use 4.0.1 while the v4 protocol and resource layout remain unchanged.
- No standalone Host plugin exists. Retain the native MCP server and five Skills;
  desktop Tauri plugins remain scoped to the paused GUI. Older v2 prompts remain exact
  Pack-bound artifacts, outside v4 Host packs; no Pack digest or history is changed.
- Correct stale publication projections: Beta.5 succeeded through OIDC at main
  `a2f8b06a75f3fcee8c6aaf638ac3e2310c6ae423`, run `34285066117`. Prepare Beta.6 using
  the established bounded source-version update, preserving historical qualification.

Second-pass review covered: new setup versus existing work; direct stage selection;
partial versus complete initial drafting; denial versus unknown commit outcome;
Requirement/Source correction and stale downstream state; modified Skills during an
upgrade; missing exports after restore. No external user form was answered. The resource
suite also verifies actual exported sibling links for Codex, Claude and generic packs,
existing managed upgrades, and refusal to overwrite user edits. Review is a local
instruction/schema walkthrough, not independent real-Host acceptance.

The domain keyword inventory still covers 189 files. Removing the literal `cover-letter`
from explanatory Skill text changes one resource classification/family count; the checked
inventory projection is refreshed without claiming a kernel architecture change.
The documentation checker no longer requires fixed AGENTS paragraphs; it verifies the
shared instruction entrypoints and scope-guide links. The assurance guide remains the
optional detail owner. Python guards retain only maintenance-script exceptions, and
release notes distinguish the npm candidate from paused desktop/full-matrix policy.

Source check, five Skill frontmatter checks, 17 resource integration tests, the focused
documentation regression, three npm packaging tests, Rust formatting/affected Clippy,
workflow YAML, instruction links and preserved-history/template/archive checks pass. Exact-package installation/upgrade
and lifecycle evidence will be recorded under `dist/npm-beta6/`; next is readiness review,
then protected CI and an authorized publication run. Dependency exception review and
full native/real-Host gates stay separate.

Exact-package review found one remaining fixed-version test assumption: the MCP smoke
required every Skill file to be 4.0.0. It now compares installed versions, sizes and hashes
with the binary's resource catalog, retaining the exact five-Skill IDs and v4 protocol.
Current resources pass; mismatched version/hash fixtures fail. The guarded dual-Pack
MCP lifecycle, reopen/restore and export smoke passes with this correction.

The first local package also passed fresh npm installation, native/source byte comparison,
and real Beta.5-to-Beta.6 managed upgrades for Codex and Claude. Both Hosts refuse edited
Skills without changing any managed files. Rebuild the final candidate from the commit
including the smoke correction; keep the first attempt separate from its final hashes.

Final local candidate: `1.0.0-beta.6`, source
`b78c22e6c9b5a0f347f80e5255e9a9db9dd217e0`, macOS ARM64 only. The npm tarball at
`dist/npm-beta6/npm/canisend-1.0.0-beta.6.tgz` is 26,113,760 bytes, SHA-256
`c001567706d92a2fe3151d64ebcf79b6d9e956f35203ddecebf103c4b46f744c`.
Fresh installation, exact native/source archive comparison, documented dual-Pack flow,
project/global Host lifecycle, guarded MCP lifecycle, reopen/restore, export verification,
uninstall/workspace retention and both Hosts' managed/customized upgrade cases pass.
`dist/npm-beta6/candidate.json` and `checks/` retain the results; the earlier local attempt
is superseded under `dist/npm-beta6-initial/`. This evidence-only update does not change
the candidate source identity above. Local npm preparation is ready; protected CI,
real-Host/full native qualification and publication are not claimed. Next: integrate
through protected CI, then use the authorized main-only npm release route.

### README download channels and quick start — 2026-09-10

Scope: make the README useful to a first-time CLI user, with direct GitHub binary
downloads alongside npm, a short Host setup path and a concrete request leading to
reviewed local files. Keep detailed release/architecture records in their owning guides.

README text drops from 1,072 to 712 whitespace-delimited words. The five native target
links and SHA256SUMS point to actual Beta.1 assets; npm `next` remains Beta.5 and `latest`
Beta.3, verified against GitHub and npm. Beta.6 remains a local source candidate.
The shared first run uses separate initialization and Host setup because the published
Beta.1 CLI does not support combined `workspace init --host`. Installation and detailed
quick-start introductions now describe the two channels and their runtime/version differences.

Validation: six download links match the published asset inventory. The downloaded
macOS ARM64 Beta.1 archive checksum and signature pass. Its CLI and the published Beta.5
npm CLI both pass version/doctor, README initialization, Codex/Claude setup with a returned
MCP registration command, and Workspace checks in isolated directories. Local documentation
links, `git diff --check`, and `xtask source check` pass. No Rust source changed or Rust
test suite was needed; this is not real-Host acceptance or a new artifact qualification.

Next: include these docs in protected integration and the next CI-built release. Existing
Beta.6 candidate bytes retain their recorded source identity; this documentation update
does not relabel or publish them.

### Optional routine Auto approval — 2026-09-10

Scope: reduce repeated product confirmations through an optional checkbox in the existing MCP
native form. Default remains individual approval. An explicit user grant permits routine private
Application reads and Requirement, exclusive pasted Source, Plan, draft and review operations for
60 minutes in one connection, bound to canonical Workspace path/UUID, Application and exact Pack.
New Requirement Sources, shared Profile/Evidence access and changes, exports and unknown actions
remain individually guarded. Model flags cannot grant authority. Scope changes, denial, failed
authorization, explicit cancellation, expiry and reconnect revoke it. Existing preview, revision,
digest, token, storage and audit checks remain. MCP metadata distinguishes delegated calls from
individual human inspection; stored user-authority fields keep their existing representation.

The shared application policy owns the scope and allowlist; MCP owns native forms and serialized
authorization/dispatch. No extra model service, tool, CLI flag, storage format or dependency was
introduced. README, the contract/consent/integration guides and ADR-RN-0023 describe the option.
The three Host guides and four affected Skills advance to resource version 4.0.2 with the package
manifest projection updated; templates, Packs, historical records and GUI/CI scope are unchanged.

Validation: the shared-policy regression passes for default-off, scope changes, expiry, revocation
and sensitive exclusions. All 8 MCP protocol tests pass, including a synthetic opt-in journey with
six successive mutations and private reads using one form, shared-data/export/new-Source refusal,
replay/stale preview rejection, model-flag rejection, cancellation and reconnect. The added pending
form cancellation check also passes without a timeout. All 17 resource integration tests, four
Skill validators, affected-package Clippy with warnings denied, Rust formatting and source check
pass. Source inventory remains 189 files; no inventory projection change is needed. The app test
linker reports its existing large unwind-section warning; execution passes.

Next: protected integration and a fresh CI-built npm candidate including this feature. The earlier
local Beta.6 tarball from b78c22e6 predates this implementation and is not relabelled. Publication,
actual interactive Host acceptance and full native qualification are not claimed by these checks.

### Evidence approval friction — 2026-09-10

The owner supplied a four-group Evidence workflow with repeated private-read and commit prompts.
The previous allowlist excluded all shared Profile/Evidence operations, and commit separately
requested each permission. This follow-up lets the user include an exact Profile Source revision
in the existing Application/session grant. Its reads, source-backed Evidence confirmation and
Profile/Evidence links reuse that grant; other Sources still need explicit opt-in. Provenance
comes from the exact immutable Evidence catalog and digest through the Store/application facade.
Source additions do not renew the grant's lifetime. Each manual request now lists its combined
read/export/write permissions in one form. Invalid/stale previews remain rejected without
revoking valid standing authority; user denial, cancellation, scope changes, expiry and reconnect
still revoke it. No tool, schema, dependency, persisted format, external access or submission was added.

Validation: 9 MCP protocol tests pass, including four Evidence confirmation/association pairs
plus the Profile link with one Source-grant form, one-form manual commits, invalid quotes and
digests, new-Source denial, cancellation and stale/replayed previews. The shared-policy test,
17 resource integration tests, Skill validation, formatting, affected Store/app/MCP/CLI Clippy
and source check pass. Host guides and the shared Workspace Skill advance to 4.0.3; the package
resource-manifest digest is synchronized. Historical artifacts and other template/Pack versions
remain untouched. The app test retains the existing linker unwind-section warning and passes.

The default local command still reports Beta.4 (2886c3c6a652), so the earlier local source
improvement was not in that executable. Next: build a fresh local Beta.6 npm/native candidate
from this implementation commit, validate its exact installed bytes, and reconnect the Host
with the newer binary. This does not claim that the screenshot's actual registered MCP command
has been inspected, that an existing user session was upgraded, or that npm/remote CI is complete.

Local candidate completed from `6cf9e8f101576ce8bd23ea587cb310322c1df7bb`:
`dist/npm-beta6-evidence-approval/npm/canisend-1.0.0-beta.6.tgz`, 26,142,858 bytes,
SHA-256 `d5ae1940df0b4e93ca724e281d584fd28796af8ab25b3bd75750b3b0b12c8aba`.
The signed macOS ARM64 native archive and offline npm installation retain the exact
built binary; the npm source archive matches the implementation commit. All 9 MCP
protocol tests pass against that installed package, including the one-form Evidence
journey. Native archive installation/uninstall, documented dual-Pack workflow,
project/global Host lifecycle, guarded MCP workflow and backup/restore pass. Isolated
Beta.4-to-Beta.6 Codex/Claude upgrades install exact current Skill bytes and preserve
user-modified files by refusing overwrite. `candidate.json`, `checks/` and
`INSTALL-LOCAL.md` in the candidate directory retain identity, checks and activation
steps. Earlier candidates remain unchanged. Next: install/register/reconnect in the
actual Host for interactive acceptance; protected integration and npm publication
remain pending. No real Workspace, global CLI or current Host session was changed.

### Readable confirmation content — 2026-09-10

The owner's screenshot shows a native consent form displaying the serialized request,
including escaped paragraph breaks and internal JSON keys. The shared MCP form now
renders the exact Broker-owned values as labeled text: operation title, permissions,
changes and proposed content precede reference details. Auto approval scope and Source
references use the same renderer. Strings are not decoded twice or rewritten, and
commit digests, tokens, validation, authorization and structured results are unchanged.
The server instruction and shared Workspace Skill require readable conversational
results and document paragraphs; the Skill advances to 4.0.4 with its manifest projection.

Validation: the renderer regression covers paragraphs, Unicode, quotes, literal
backslashes and reference values without mutating the preview. All 9 MCP protocol tests
pass, including a multiline revision displayed intact in the real protocol form, exact
fingerprint binding, token omission, denial, replay and the prior single-form workflow.
All 17 resource tests, the Skill validator, formatting, affected MCP/CLI Clippy and
source check pass. Next: build a separate local candidate containing both usability
fixes and validate its installed binary; retain the earlier candidate's identity.
This does not claim actual Host visual acceptance, remote CI or npm publication.

Local macOS ARM64 candidate completed from `5af7243bebfcde05ee809a39fd0b683a3f5bbca8`:
`dist/npm-beta6-readable-confirmations/npm/canisend-1.0.0-beta.6.tgz`, 26,154,790 bytes,
SHA-256 `b150ac4e0c24d15ca3baa4380f87bb46fa6ea7a8a96a5ec97756e86c771e9922`.
The offline npm installation matches the signed binary and archived source exactly.
All 9 MCP tests pass against the installed package, including readable multiline
confirmation and Source-grant reuse. Native lifecycle, dual-Pack/recovery/MCP smokes,
Beta.4-to-Beta.6 Codex/Claude upgrades and customization preservation pass. Identity,
logs and installation/reconnection steps are retained in `candidate.json`, `checks/`
and `INSTALL-LOCAL.md`. Earlier packages remain unchanged. Actual Host visual
acceptance, protected CI and publication remain pending; the real Workspace, global
installation and running Host have not been changed.

### Beta.6 npm publication — 2026-09-10

The owner authorized publication. PR #231 merged the prepared changes after all required
Fast CI checks passed in run `34532533933`; GUI jobs remained skipped under the existing
pause. The delayed PR event made supplemental run `34532410149` redundant, so it was
cancelled once the formal run started. No branch protection or dependency record was changed.

The main-only `release.yml` npm path published `canisend@1.0.0-beta.6` to `next` in
run `34533668709`, source `e9ba8ea7364dbad152482db7ba99a45a0273007d`, macOS ARM64 only.
The registry tarball is 26,412,877 bytes with SHA-256
`35093c1ef29cd9642f905f0a1c5abcaa20d6dcea9f6314e2fb1abe5677350ba8`.
CI publication, provenance, exact-package MCP/lifecycle checks and registry reinstallation
passed. Independent npm download matches the retained CI tarball, SHA-1/SHA-512 registry
metadata, installed native binary and source archive. npm verifies the registry signature
and attestation; native signature, doctor, fresh Workspace and exact five-Skill installation
also pass. `dist/npm-beta6-published-34533668709/verification.json` retains local evidence.

`latest` remains Beta.3. The dependency-assurance run `34532533950` fails on the existing
RUSTSEC-2024-0320 review deadline of 2026-09-07; this independent npm publication does not
renew exceptions or claim full native qualification or actual Host acceptance. The previous
local candidate bytes remain unchanged; no real Workspace, global CLI or Host was upgraded.
README, installation, release notes and current release records now describe the published
package. Next: integrate this documentation-only publication record through protected CI.
The updated publication projections pass the source gate and `git diff --check`.

### Cargo and PyPI testing channels — 2026-09-10

The owner authorized publication through locally authenticated Cargo and PyPI. All eight
Cargo packages at source `d59984438c1a34e12743398b0d0ad0a5a953836d` passed locked packaging
and independent build verification. The first five uploaded successfully; crates.io's new-crate
rate limit delayed the remaining three. Exact candidate archives and hashes are retained in
`dist/cargo-beta6-publication/`. Registry completion and installation remain pending.

PyPI had no Rust packaging configuration. Added Maturin binary-wheel metadata and two small
build/install scripts, with the version inherited from Cargo and GPL/third-party notices plus
corresponding source included. The local `1.0.0b6` macOS ARM64 wheel passed strict Twine
metadata validation, native signature, version/source identity, doctor, fresh Workspace and
all nine existing MCP protocol tests. Bash syntax, workflow YAML/routing and source checks
passed. `release.yml` now has an independent main-only PyPI route using the retained `pypi`
environment/OIDC publisher, with artifact retention and exact registry-byte comparison.
Protected CI, PyPI upload and registry installation remain pending. Full native/GUI and actual
Host qualification remain outside this scope; no dependency exception was renewed.
