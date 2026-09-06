# Embedded App roadmap and execution review

Date: 2026-09-04
Status: Process/documentation revision; R2 product implementation pending

## Authority and baseline

The owner requested removal of project-local Trellis skills, preservation of overall project
control, and optimization of the roadmap and later development flow using the review findings.
The [master roadmap](../../superpowers/plans/2026-07-25-1.0-release-roadmap.md) remains the sole
work-order authority. ADR-RN-0022 retains the App-first Codex App Server/MCP direction and Tauri shell.

The review baseline is `d7c0abfea46aa8d7b9f63d9b1ac7bd7fac93abea` on
`feat/beta2-cli-skills-readiness`. R0/R1 source is complete with retained local evidence;
[R1's record](../../../.trellis/tasks/archive/2026-09/09-04-embedded-agent-app-r1/research/app-server-evidence.md)
binds source `8240fae7da0eb5ecafc2e3dce7bf7609d05f9cf3`. Those previously recorded tests are not
new tests from this review. Public Beta.1 remains `6e1397b79031cad54e794ccdc9edca2153f23b3e` and
feature freeze remains at `acf25dc483643ca9be0210320775708da116b715`. No new artifact, release,
provider qualification, protected merge, or user cohort is claimed.

## Findings carried into the plan

| Finding | Source evidence | Required outcome and owner |
| --- | --- | --- |
| Working directory/read-only sandbox does not prove provider isolation | `agent_runtime.rs` process spawn and start/resume policy | R2a: effective policy before initialization and on start/resume/turn; prove inherited configuration and denied private reads with local sentinels |
| Read-only tools return private content and accept caller consent booleans | MCP `deliverable.audit`, `review.inspect`, and `confirmed_private_read` | R2a: separate metadata, private-read/provider-send, commit, and export authority; exact user response and no automatic reviewer |
| Not every Application handler uses the shared parser; no-ID lists can disclose wider state | MCP `application.show`, source/association lists, `application.list` | R2a: one instance guard across all ID paths and explicit output filtering/disablement for Workspace-wide tools |
| Snapshot identity and recovery can misrepresent historical files | Export destination-prefixed paths; projection repair regenerates bytes; planned manifest-only deduplication | R2b: logical paths, actual generator identity, same-kind predecessor, current-head-only reuse, and verified Blob-backed repair/restore |
| Bounded registry retention cannot guarantee durable committed provenance | Parent trace requirement versus old R2 cache eviction design | R2b: minimum verified origin attached to canonical product audit; survive cache deletion and backup/restore, report attachment gaps without retry |
| Resume does not restore conversation display | `excludeTurns: true`; frontend messages stored only in memory | R3: bounded provider-history read for the registered thread, ordering/deduplication, and explicit unavailable-history state |
| Historical Beta.1 cannot qualify a new embedded UI/runtime | Current M4 and Issue #70 bind exact public Beta.1 | R4/M4-APP-005: update the existing validator for one formal cohort on the qualified embedded build with unchanged thresholds; preserve Beta.1 history without two full cohorts |

Source owners are the existing [desktop runtime](../../../crates/canisend-desktop/src/agent_runtime.rs),
[MCP server](../../../crates/canisend-mcp/src/lib.rs),
[projection/recovery service](../../../crates/canisend-store/src/application_projection_v3.rs),
[export service](../../../crates/canisend-store/src/application_flow_v3.rs), and
[frontend Agent state](../../../apps/canisend-desktop/src/lib/agent-state.svelte.ts).
The [R2 research](../../../.trellis/tasks/09-04-embedded-agent-app-r2/research/r2-existing-boundaries.md)
records schema/version limits; no current-version capability is inferred merely from newer docs.

## Revised delivery and control

Use R2a for effective isolation and consented MCP work, R2b for durable origin/exact history, R3 for
complete Workbench journeys, then R4 for exact artifact and user qualification. R3 reuses stages,
readiness, evidence coverage, Deliverables, context consent, guided actions, and supported manual
provider-unavailable operations. R5 ACP expansion stays after MVP; R6 cleanup depends on parity and
a rollback window rather than completion of a second provider.

The existing [R2 checklist](../../../.trellis/tasks/09-04-embedded-agent-app-r2/implement.md) is the
execution record. Independent preparation can overlap stable contracts, while competing Store and
approval edits remain sequential. Keep one primary test owner per invariant, one applicable source
gate per final integration head, and protected CI/native/extended gates with their existing owners.
Update evidence and next action once. No replacement task system or speculative framework is added.

## Trellis adapter removal

Removed 54 verified project-local files: 46 files in 12 `.agents/skills/trellis-*` skill directories,
plus eight Trellis-only `.codex` configuration, agent, and hook files. The removal manifest checked
path scope, file hashes, and symlink absence before deletion. No user-global files or product Agent
v4 Skills/resources were removed.

[AGENTS.md](../../../AGENTS.md), [CONTRIBUTING.md](../../../CONTRIBUTING.md), and the retained
[project control guide](../../../.trellis/spec/guides/project-control.md) now define direct execution.
Historical `.trellis/tasks/`, specs, journals, scripts, and upstream license/provenance keep their
paths. The old workflow is a retirement notice; metadata fields do not activate phases or block work.
No task creation, activation, automatic agent dispatch, journal, or bookkeeping commit is required.
Do not regenerate the deleted adapters unless the owner reinstalls Trellis.

## Verification and remaining gates

Executed on 2026-09-04:

- `git diff --check`: passed.
- Adapter inventory: all 54 planned removals verified across 12 skill directories and eight Codex
  adapter/configuration files; no product code, product Skills, dependencies, or release JSON changed.
- Local documentation check: 20 modified/new Markdown files, 157 local links/anchors, and five
  retained JSON/JSONL files passed path, anchor, parse, and whitespace checks.
- `cargo run -p xtask --locked -- release check`: passed; every reported check was `ok`, including
  active release truth, documentation, contracts, parity, dependency policy, and feature freeze.
  The existing release-status report still identifies four drift items, three stage-blocking;
  passing the source check is not RC readiness. The freeze report contains 14 existing exceptions.
 No Rust product implementation, dependency, release JSON, or public support fact changes in
this revision. No live provider or native matrix is required for this documentation/process change.

Policy-bearing adapter, AGENTS, PRODUCT, and spec edits still need exact feature-freeze exception
records when committed. This worktree has no new commit identity to record; the release ledger and
automatic path exemptions are unchanged. Public Issue projection for new M4-APP rows remains pending.
Protected CI, embedded implementation, exact artifacts, and consented user evidence remain separate
future work, not consequences of a passing local source check.
