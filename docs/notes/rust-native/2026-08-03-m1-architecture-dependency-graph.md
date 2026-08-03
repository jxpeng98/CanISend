# M1-ADR-001 / M1-GRAPH-001 — Current architecture and dependency policy

Date: 2026-08-03

## Outcome

[ADR-RN-0019](../../architecture/rust-native/decisions/0019-current-product-graph.md) is the accepted
authority for the current nine-product-crate architecture, MCP adapter, Tauri 2/Svelte 5 desktop,
unified GUI/CLI/MCP host, automation boundary, actual graph, and target graph.

The initial six-crate ADR-RN-0002 is superseded by ADR-RN-0019. The contradictory egui choice in
ADR-RN-0013 is superseded by ADR-RN-0015, whose cutover record remains Accepted. No accepted
desktop ADR now selects both egui and Tauri.

## Machine-readable graph

`workspace-dependency-policy-v1.json` records:

- nine product crates and the separate `xtask` automation crate;
- all 26 current internal Cargo edges;
- the 25-edge target graph;
- exact normal/dev/build kind, target predicate, optional flag, default-feature behavior,
  dependency feature set, and rename state for every edge;
- one temporary `canisend-store -> canisend-io` normal-edge exception owned by
  M1-ARCH-001/002.

The original policy review found 29 edges and three M1-ARCH-003 CLI removals. Those removals are
now implemented and recorded by the
[facade-hygiene note](2026-08-03-m1-cli-facade-hygiene.md); the checked-in policy describes the
post-cleanup 26-edge actual graph.

The Store→IO exception must be reviewed by 2026-08-10 and expires on 2026-08-17. Its removal
condition is an app-owned prepare → render/project → revision-bound commit port with stale,
failure-atomicity, Blob-ledger, cleanup, and repair-convergence evidence. The graph gate fails when
the review becomes overdue; silence cannot extend the exception.

## Enforcement

`cargo run -p xtask --locked -- architecture graph-check` runs locked/offline Cargo metadata and
requires exact agreement with the policy. It rejects:

- a new internal edge;
- a removed or reclassified edge;
- normal/dev/build changes;
- target, optional, default-feature, feature-set, or rename changes;
- unknown crates, duplicate edges, or non-dev cycles;
- undocumented actual-to-target additions/removals;
- missing/contradictory ADR status; and
- missing, overdue, expired, or inconsistent temporary exceptions.

The check is part of `release check`. Unit regressions mutate an allowed edge, add an unapproved
edge, expire the exception, and preserve the full build/target/optional/feature/rename edge schema.

## Remaining boundary

This closes M1-ADR-001, M1-GRAPH-001, and M1-ARCH-003 source implementation. It does not close
M1-ARCH-001/002: the time-bounded Store→IO exception is deliberately visible, and the target graph
is not falsely reported as current.

The shared approval/preview broker has since been completed. The remaining M1B path is the
Store→IO ownership decision and renderer/projector failure, stale-revision, CAS-cleanup, and
repair-convergence evidence.
