# CanISend plan registry

**Status:** Navigational index — not a stage or release authority

**Last reviewed:** 2026-08-02

This registry prevents completed, historical, and supporting plans from competing with the current
1.0 delivery authority. File paths are preserved because release tooling, evidence notes, and
historical reviews link to them.

Only the [CanISend 1.0 delivery roadmap](2026-07-25-1.0-release-roadmap.md) may declare top-level
stage completion. A child plan may define implementation detail, but its unfinished release or
validation work is inherited by the parent roadmap.

## Status vocabulary

| Status | Meaning |
|---|---|
| Active — authoritative | Sole top-level work-order and stage authority |
| In progress — supporting | Bounded workstream with an explicit parent and exit |
| Implemented — qualification tracked by parent | Source work is complete; remaining evidence belongs to the parent |
| Completed | Delivery and its named evidence are complete |
| Historical | Preserved decision or execution context; not current status |
| Deferred | Explicitly outside the current release path with a named future target |

## Active authority

| Plan | Parent | Current exit |
|---|---|---|
| [CanISend 1.0 delivery roadmap](2026-07-25-1.0-release-roadmap.md) | None | Publish exact, qualified `v1.0.0` and establish the 1.0.x support path |

## Supporting workstreams

| Plan | Parent milestone | Remaining boundary |
|---|---|---|
| [Stage 4G connected Agent workspace](2026-07-30-stage-4g-connected-agent-workspace-plan.md) | M2/M3 | Exact-candidate smoke and real Codex/Claude provider validation |
| [Stage 4H Agent efficiency](2026-07-30-stage-4h-agent-efficiency-plan.md) | M3 | Exact-build external-host dogfood |
| [Cross-platform desktop size optimization](../../performance/cross-platform-desktop-size-optimization-plan.md) | M2 | Native candidate qualification; no support expansion |
| [Post-template size optimization](../../performance/post-template-upgrade-size-optimization-plan.md) | M2 | Bounded profile and native qualification |
| [Typst template and final preview](../../architecture/typst-template-preview-execution-plan.md) | M2 | Exact native preview/export evidence |

## Implemented and completed child/supporting plans

| Plan | Result |
|---|---|
| [Stage 4A–4B](2026-07-26-stage-4a-4b-execution-plan.md) | Application boundary, recovery, and workflow control |
| [Stage 4C–4F](2026-07-26-stage-4c-4f-execution-plan.md) | Original complete ordinary-operation parity |
| [Release-pipeline optimization](2026-07-26-release-pipeline-optimization-roadmap.md) | Build-once candidate promotion and release evidence |
| [Tauri + Svelte migration](2026-07-27-tauri-svelte-ui-migration-roadmap.md) | Source/cutover implemented; native qualification inherited by M2 |
| [Stage 4I–4M content integration](2026-07-30-stage-4i-content-integration-plan.md) | Source implemented; release qualification inherited by the parent |
| [shadcn-svelte migration](2026-07-31-shadcn-svelte-system-migration-plan.md) | Shared UI component system |

## Historical and superseded references

| Plan | Successor or current authority |
|---|---|
| [Rust-native greenfield rebuild](2026-07-17-rust-native-greenfield-roadmap.md) | 1.0 delivery roadmap |
| [Post-0.7 feedback roadmap](2026-07-18-post-0.7-roadmap.md) | Historical feedback record; 1.0 roadmap for current work |
| [Native desktop GUI design](2026-07-19-native-desktop-gui-roadmap.md) | 1.0 delivery roadmap |
| [macOS-first GUI execution](2026-07-24-macos-gui-execution-plan.md) | 1.0 delivery roadmap |
| [Earlier unified-host measurements](../../performance/unified-host-further-size-reduction-plan.md) | Post-template size plan |
| [Python-era Agent-native design](../specs/2026-07-10-agent-native-workflow-roadmap.md) | Rust-native 1.0 delivery roadmap |
| [Python-era CLI-first execution](../specs/2026-07-11-cli-first-workflow-optimization-roadmap.md) | Rust-native 1.0 delivery roadmap |

## Maintenance rules

- Add a plan here when it is created.
- Give every unfinished child plan a parent milestone and a bounded exit.
- Update classification in the same commit that completes, supersedes, defers, or reactivates a
  plan.
- Do not move or delete historical plans merely to remove stale wording.
- Preserve historical version numbers and link to new authority instead of rewriting old facts.
- Never infer completion from a checkbox alone; retain the test, note, run, or artifact reference.
