# GF1 workflow-pack Deliverable catalog implementation record

**Date:** 2026-08-02

**Roadmap task:** GF1-DELIV-001 additive contract/core foundation; partial foundation for
M1F-DELIV-001, GF2-MODEL-001, and GF4-FLOW-001

**State:** Implemented and source-gate verified in this change. Neutral Deliverable persistence,
planning/drafting/review/package/render execution, academic compatibility mapping, and linked
work-item inspection remain required before the full roadmap task becomes Verified.

## Implemented boundary

- Added the canonical transparent `DeliverableKindId` contract using
  `<workflow-pack-id>:<local-deliverable-kind-id>` identity.
- Added a pure runtime catalog compiler over typed Pack Deliverable, resource, Renderer, and
  Validator declarations.
- Preserved the Manifest Kind array as authoritative presentation/planning order.
- Rechecked 1–64 Kind limits, duplicate IDs, and cardinality bounds of 0–32 with a nonzero maximum.
- Resolved template paths only to declared `template` resources and froze their resource ID,
  version, byte size, and SHA-256 declarations in immutable bindings.
- Required Renderer and Validator capabilities to be selected by the Pack; the verified-bundle
  entrypoint additionally inherits the kernel capability-registry and exact-byte checks.
- Resolved Validator instances to capability IDs and bounded declarative parameters.
- Added runtime count validation for unknown/foreign Kinds, missing required Kinds, and counts
  above the declared maximum, including duplicate singleton Deliverables.

The existing `DocumentKind`, document tables, plan/task operations, projections, embedded Typst
selection, Agent v2, CLI, MCP, and desktop contracts are unchanged. They remain the four-document
academic compatibility runtime until Workspace v3 and the reference Pack can migrate together.

## Defensive invariant

A Pack declares what Deliverables are needed but cannot add rendering or validation code. Template
bindings point only at digest-bound Pack data; Renderer and Validator IDs must already exist in the
kernel-owned registry established during verified-bundle construction.

The catalog does not infer a newest Pack, coerce a foreign Kind, or silently drop a required Kind.
Planning/readiness callers must present counts keyed by the exact Pack-qualified identity, and
validation is deterministic in declaration order after foreign Kind rejection.

## Test coverage

- canonical `DeliverableKindId` construction, JSON round trip, malformed forms, and cross-Pack
  distinction;
- ordered Kind descriptors with required/optional and singleton/multi-instance cardinalities;
- template resource identity/version/size/hash, Renderer, and Validator-parameter resolution;
- two Pack catalogs with equal local Kind names remaining isolated;
- valid inventory, missing required Kind, excessive singleton, and foreign Kind counts;
- empty/oversized catalog, duplicate Kind, zero/reversed/oversized cardinality;
- missing/wrong-kind/duplicate template resource declarations;
- unselected Renderer, duplicate/unknown Validator reference, duplicate Validator definition, and
  unselected Validator capability; and
- verified bundle to catalog compilation while all fixed academic v2 tests remain unchanged.

## Verification

```console
cargo test -p canisend-contracts -p canisend-core --locked
cargo clippy -p canisend-contracts -p canisend-core \
  --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- release check
```

## Remaining boundary

GF2-MODEL-001 must introduce neutral `Deliverable` records keyed by `DeliverableKindId` and bind
them to an exact Application Pack snapshot. GF2-STORE-001 then persists catalog-driven plans and
artifacts without fixed Kind columns or branches. GF3-PACK/COMPAT map the four current
`DocumentKind` values to the academic Pack; GF4-FLOW proves at least two different generic Kinds
through planning, drafting, review, package, render, and export.

## Rollback

Revert the additive ID, catalog compiler, tests, contract text, implementation record, and Roadmap
evidence row together. No Workspace or projection rollback is required because this slice does not
read, write, or reinterpret persisted documents.
