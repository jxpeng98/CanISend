# First usable Alpha gap audit after GF4

Date: 2026-08-03

## Decision

GF4 source implementation is complete after the four synthetic generic examples, but the first
usable roadmap Alpha is **not yet proven**. The active 1.0 roadmap names Alpha.6 as the first
framework checkpoint and makes all M1F exit gates plus the M1A P0 set prerequisites. Implementing
later generic features early does not waive those gates or silently turn Alpha.5 into Alpha.6.

This audit uses current files and source-gate behavior, not roadmap intent, as evidence.

## Proven current foundation

- Both exact built-in Packs load through the verified Pack registry.
- Workspace v3 authority, backup-backed v2 migration, Pack invalidation, neutral projections, and
  the canonical generic flow have focused source tests.
- CLI, desktop, Agent v3, and MCP expose the generic flow; Agent v2 remains exact academic-Pack
  compatibility.
- Four offline synthetic domain families complete through validated PDF export with no
  submission.
- Fast CI runs the locked Rust workspace and Svelte checks on macOS, and the source release gate
  verifies the existing release/status contracts.

These facts are necessary but do not prove the missing requirements below.

## Unproven or contradicted Alpha.6 P0 gates

| Roadmap gate | Current evidence | Audit result |
|---|---|---|
| M1-ADR-001 | ADR-RN-0013 still has `Status: Accepted` and explicitly chooses egui/rejects Tauri, while ADR-RN-0015 accepts Tauri/Svelte. No accepted ADR records the current nine product crates, MCP adapter, unified host, actual/target graphs, or the temporary Store→IO exception/expiry. | Contradicted/incomplete |
| M1-GRAPH-001 | No source-gated allowed-edge registry was found. Current Cargo compilation does not reject a newly introduced but architecturally forbidden normal/dev/build/target/optional/feature edge. | Missing |
| M1-OP-001 | There is a typed legacy compatibility mapping and an Agent v3 operation list, but no single typed leaf-level `OperationId`/`OperationStatus` authority classifying composites, aliases, CLI, Tauri, MCP, and adapter-only leaves. | Incomplete |
| M1-OP-002 | Existing CLI/GUI and Svelte parity JSON covers earlier operation families; no check derives or verifies the current Clap, Tauri, and 22-tool MCP leaves against one canonical registry. | Incomplete |
| M1-OP-003 / GF5-PARITY-001 | Focused fixtures cover important paths, but there is no machine-listed two-Pack semantic matrix for every shared read/mutation, stale/replay, wrong Pack/context, and no-mutation outcome. | Incomplete |
| M1-APPR-001 | MCP still owns `MutationPreviewStore`; desktop retains separate preview/approval state. There is no shared TTL, CSPRNG, context-binding, disposition, capacity, replay, and concurrency broker. | Missing |
| M1-TEST-001 | Agent v3 stale/replay recovery is covered, but the required broker, every preview-store family, DB-busy/transient/permanent I/O, capacity race, wrong context, and concurrent exactly-once matrix cannot exist before the shared broker. | Incomplete |
| M1-MSRV-001 | Cargo declares Rust 1.92 while CI and release workflows pin Rust 1.97. No locked 1.92 CI evidence was found. | Contradicted |
| M1-CI-002 | Fast CI runs frontend check/test/build, and release builds frontend assets, but the manual release source path does not prove that the required Svelte check, unit, production build, and critical browser/accessibility checks are all non-bypassable. | Incomplete |
| M1F exit gate | Generic source fixtures are strong, but academic canonical-v3 semantic parity across CLI, MCP, Agent v3, and desktop is not yet one source-gated matrix. | Incomplete |
| Alpha.6 qualification | No exact Alpha.6 five-target packaged-binary, migration/rollback, dual-surface, and clean-tag qualification evidence exists for the current commit. | Missing |

## Progress after this audit

GF5-OP-001 / M1-OP-001/002 now have source implementation evidence in the
[operation registry contract](../../contracts/operation-registry-v1.md) and
[GF5 operation-registry record](2026-08-03-gf5-operation-registry.md). The typed registry owns the
exact 86-leaf Clap, 111-handler Tauri, and 22-tool MCP inventories; status, duplicate,
false-sharing, Pack mismatch, compatibility, and source-drift checks are release-gated.

This advances ordered-path item 1 from missing to implemented source evidence. It does not satisfy
M1-OP-003 semantic parity or qualify the M1A exit gate by itself. The next ordered implementation
item was the current architecture ADR plus dependency-edge policy.

M1-ADR-001 / M1-GRAPH-001 now also have source implementation evidence in
[ADR-RN-0019](../../architecture/rust-native/decisions/0019-current-product-graph.md), the
[machine-readable dependency policy](../../architecture/rust-native/workspace-dependency-policy-v1.json),
and the [implementation record](2026-08-03-m1-architecture-dependency-graph.md). The accepted ADR
records the actual and target graphs; ADR-RN-0002 and the egui ADR-RN-0013 are superseded; all 29
current internal Cargo edges are source-gated across normal/dev/build/target/optional/feature
dimensions. The sole Store→IO exception is review-bound to 2026-08-10 and expires 2026-08-17.

This advances ordered-path item 2 from missing/contradicted to implemented source evidence. It
does not resolve the Store→IO exception or the M1B target graph. The next ordered P0 implementation
item is the shared approval broker and complete M1-TEST-001 matrix.

P1 CI/dependency improvements, governance work-item linkage, independent evidence review, dogfood,
target-user validation, signing, and later Alpha.7 dual-Pack qualification also remain, but they do
not replace the P0 implementation order.

## Ordered path from current HEAD

1. Completed in source: GF5-OP-001 with M1-OP-001/002 now provides one typed canonical leaf
   registry and source-gated Clap/Tauri/MCP mappings.
2. Completed in source: ADR-RN-0019 supersedes the obsolete graph/egui authorities and the complete
   internal dependency-edge policy is release-gated (M1-ADR/GRAPH).
3. Replace duplicated preview stores with the shared approval broker and its complete failure and
   concurrency suite (M1-APPR/TEST).
4. Build the two-Pack semantic parity matrix and machine-list uncovered leaves
   (M1-OP-003/GF5-PARITY).
5. Align the MSRV and close the release/frontend non-bypass path.
6. Rewrite the user documentation around Pack selection and v2→v3 boundaries (GF5-DOC).
7. Run the exact native Alpha.6 candidate, migration, backup/restore, rollback, package, and
   release-integrity gates on a clean candidate commit.

Only after evidence proves every applicable gate should the release status advance from Alpha.5
and the goal be considered achieved. GF4 completion alone is not an Alpha qualification event.
