# GF1 workflow-pack stage-graph implementation record

**Date:** 2026-08-02

**Roadmap task:** GF1-DAG-001 additive core foundation; partial foundation for M1F-DAG-001 and
GF2-INVALID-001

**State:** Implemented and source-gate verified in this change. Workspace v3 persistence and
execution, stale propagation over stored Applications, and linked work-item inspection remain
required before the full roadmap task becomes Verified.

## Implemented boundary

- Added the canonical transparent `StageId` contract with
  `<workflow-pack-id>:<local-stage-id>` identity and component validation.
- Added a pure `WorkflowPackStageGraph` compiler over a typed Pack workflow definition.
- Qualified every stage and dependency with the owning Pack ID so equal local IDs from different
  Packs cannot collide or satisfy each other's lookups.
- Rechecked stage count, duplicate stages, execution-mode count and uniqueness, self/duplicate/
  missing dependencies, terminal existence, cycles, and terminal reachability at the core boundary.
- Compiled a stable lexical Kahn topological order independent of manifest declaration order.
- Added deterministic descriptor, output, execution-mode, ancestor, and descendant queries for
  future scheduling, invalidation, and scoped rerun services.
- Added a verified-bundle entrypoint that consumes the manifest already bound by the Pack registry.

The existing `WorkflowStage`, `StageDescriptor`, `StageGraph`, Workspace v2 SQL values, Agent v2,
CLI, MCP, and desktop contracts are unchanged. They remain the academic compatibility runtime
until the Workspace v3 migration slice can switch authority atomically.

## Defensive invariant

A Pack controls declarative graph shape but does not add execution behavior. Stage outputs and
execution modes remain closed kernel enums; unknown JSON variants fail Schema validation. The core
compiler never accepts a foreign Pack's `StageId`, silently chooses a Pack version, or infers an
undeclared dependency.

All stages must contribute to the terminal stage. This prevents disconnected side workflows from
escaping the eventual readiness/review path. Ancestor and descendant results follow the compiled
topological order, providing a deterministic foundation for later stored dependency invalidation.

## Test coverage

- canonical `StageId` construction, JSON round trip, component access, and malformed IDs;
- stable graph compilation across reordered stage declarations;
- two independent roots, multi-parent dependencies, output/mode queries, ancestors, and
  descendants;
- same local ID isolation across two Pack namespaces;
- duplicate stage/dependency/mode, self dependency, missing dependency, missing terminal, cycle,
  and terminal-disconnected graph rejection;
- empty/oversized graph and empty/oversized execution-mode rejection;
- unknown output and execution-mode JSON value rejection; and
- verified bundle to dynamic graph compilation while the fixed v2 graph tests remain unchanged.

## Verification

```console
cargo test -p canisend-contracts -p canisend-core --locked
cargo clippy -p canisend-contracts -p canisend-core \
  --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- release check
```

## Remaining boundary

GF2-MODEL-001 and GF2-STORE-001 must introduce Pack-bound Applications and persist exact
`(Pack ID, version, digest)` plus `StageId` values before this graph can become execution
authority. GF2-INVALID-001 then ports stale propagation and scoped rerun to the dynamic graph and
proves that only descendants of the changed stage are invalidated. Agent v2 and Workspace v2 must
continue to use the fixed compatibility graph until the backup-backed migration is complete.

## Rollback

Revert the additive `StageId`, graph compiler, tests, documentation, and Roadmap evidence row
together. Existing Workspace files and runtime behavior require no rollback because this slice
does not read or write persisted stage state.
