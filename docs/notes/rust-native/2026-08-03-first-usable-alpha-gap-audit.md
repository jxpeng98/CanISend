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
records the actual and target graphs; ADR-RN-0002 and the egui ADR-RN-0013 are superseded; all 26
current internal Cargo edges are source-gated across normal/dev/build/target/optional/feature
dimensions. M1-ARCH-003 removed the three direct CLI IO/Resources/Store edges through the
[application-facade cleanup](2026-08-03-m1-cli-facade-hygiene.md). The sole Store→IO exception is
review-bound to 2026-08-10 and expires 2026-08-17.

This advances ordered-path item 2 from missing/contradicted to implemented source evidence. It
does not resolve the Store→IO exception or the M1B target graph. The next ordered P0 implementation
item was the shared approval broker and complete M1-TEST-001 matrix.

M1-APPR-001 / M1-TEST-001 now have source implementation evidence in the
[shared approval broker record](2026-08-03-m1-shared-approval-broker.md). MCP and all former
desktop preview-store families use one app-owned ten-minute monotonic, CSPRNG, exact-context,
single-use broker. The bounded store counts waiting and in-flight grants, preserves the original
deadline only for explicitly transient restoration, and deterministically or periodically sweeps
expired private payloads. Broker, MCP Application/job/task, desktop migration, Svelte bridge,
stale/no-mutation, failure disposition, wrong kind/Workspace/Pack, replay, capacity, restart, and
concurrency tests are source-gated.

This advances ordered-path item 3 from missing/incomplete to implemented source evidence. It does
not establish cross-surface semantic parity for every canonical registry leaf. The next ordered P0
implementation item is M1-OP-003/GF5-PARITY-001.

M1-OP-003 / GF5-PARITY-001 now have source implementation evidence in the
[semantic parity contract](../../contracts/semantic-parity-v1.md) and
[implementation record](2026-08-03-gf5-semantic-parity.md). The source gate binds both built-in
Packs across CLI, Tauri, and MCP to 8 shared operations, 7 revision-bound operations, 5
preview/commit families, 5 read families, and the closed success/stale/replay/wrong-Pack/
wrong-context/no-mutation/recovery outcome set. Wrong-Pack calls are checked in both directions,
and rejected mutations retain the same Workspace status or authoritative revision. The remaining
148 non-shared bindings are machine-listed with typed class and Pack scope rather than hidden by a
parity claim.

This advances ordered-path item 4 from incomplete to implemented source evidence. It does not
qualify packaged binaries, real Agent hosts, or users. The next ordered P0 implementation item is
MSRV alignment plus the release/frontend non-bypass path.

M1-MSRV-001 / M1-CI-002 now have source implementation evidence in the
[toolchain and frontend source-gate record](2026-08-03-m1-msrv-frontend-source-gates.md). Cargo and
the README declare Rust 1.97, every active stable toolchain owner pins 1.97.0, and the release
source gate rejects version drift. The candidate-only source job also installs locked frontend
dependencies and reruns formatting, Svelte/TypeScript checking, UI unit tests, the production
build, and the two critical Chrome accessibility specs before release assembly.

This advances ordered-path item 5 to implemented source evidence. Exact GitHub CI on the committed
source remains required before the continuously proven MSRV checkbox can close. The next P0
implementation item was the GF5 user-documentation rewrite around Pack choice and migration.

GF5-DOC-001 now has source implementation evidence in the
[dual-Pack user documentation record](2026-08-03-gf5-user-documentation.md). The quick start,
Agent, desktop, privacy, backup, upgrade, troubleshooting, installation, guide index, README, and
new limitations guide now distinguish Generic v3 from Academic v2 compatibility before
initialization or migration. `xtask docs check` requires ten guides and their stable journey
markers, while the documented smoke runs both Packs in separate disposable Workspaces through
Academic backup/restore and Generic PDF export without submission.

This advances ordered-path item 6 to implemented source evidence. Exact native candidate and
remote CI evidence, rather than another P0 source implementation item, now form the next Alpha.6
proof boundary. GF5-SDK-001 remains a separate P1 authoring/validation deliverable.

The first Alpha.6 dry-run then exposed a direct M0-REL-001 contradiction: the roadmap documented
sequential Alpha support, but `prepare-stage v1.0.0-alpha.6` rejected Alpha→Alpha. The
[sequential Alpha transition record](2026-08-03-m0-sequential-alpha-transition.md) now documents
the corrected planner. Its 27-file dry run covers Cargo/Tauri/npm/locks, parity and package
contracts, workflow/docs, and target-bound pending readiness/freeze/feedback through one
transactional write set. No `--write`, candidate, tag, push, or release was performed.

This closes the sequential-Alpha planner source gap and proves only the dry-run half of
M2-VERSION-001. Applying the version change remains part of the explicit Alpha.6 candidate
sequence.

The [M1-CI-001 implementation record](2026-08-03-m1-fast-cross-platform-ci.md) now closes the
remaining fast-CI source gap: a pinned Ubuntu Chrome job owns the 14 critical keyboard,
accessibility, reflow, and key-visual checks, while one bounded matrix tests the core, Store, IO,
CLI, and MCP packages on Ubuntu and Windows. `release check` locks both jobs and their
non-authoritative boundary. Their exact committed remote results, and the pinned MSRV result, are
still missing evidence rather than missing source implementation.

The [M1-DEP-001 implementation record](2026-08-03-m1-dependency-assurance.md) closes the dependency
policy source gap. A dependency-change workflow now runs all four `cargo deny` classes and the
lock-bound exception validator. The review corrects two `quick-xml` entries to vulnerability status,
gates their bibliography/CSL/XML non-reachability, and gives all 23 exceptions named ownership,
seven-day review, fourteen-day expiry, removal conditions, and upstream tracking. Exact committed
workflow results and independent review remain evidence gaps.

P1 CI/dependency improvements, governance work-item linkage, independent evidence review, dogfood,
target-user validation, signing, and later Alpha.7 dual-Pack qualification also remain, but they do
not replace the P0 implementation order.

## Ordered path from current HEAD

1. Completed in source: GF5-OP-001 with M1-OP-001/002 now provides one typed canonical leaf
   registry and source-gated Clap/Tauri/MCP mappings.
2. Completed in source: ADR-RN-0019 supersedes the obsolete graph/egui authorities and the complete
   internal dependency-edge policy is release-gated (M1-ADR/GRAPH).
3. Completed in source: the shared approval broker and complete failure/concurrency suite replace
   duplicated MCP and desktop preview stores (M1-APPR/TEST).
4. Completed in source: the two-Pack semantic parity matrix qualifies shared outcomes and
   machine-lists uncovered leaves (M1-OP-003/GF5-PARITY).
5. Completed in source: Rust 1.97 alignment and the release/frontend non-bypass path are
   machine-gated; exact remote CI evidence remains required.
6. Completed in source: ten required user guides and a two-Workspace executable smoke expose Pack
   selection, exact compatibility, migration, recovery, and limitations (GF5-DOC).
7. Completed in source: sequential Alpha planning now produces one transactional, evidence-resetting
   Alpha.6 dry run; applying it remains intentionally unperformed (M0-REL-001/M2-VERSION-001).
8. Completed in source: lightweight Ubuntu/Windows core tests and the pinned Chrome critical suite
   are part of fast CI; obtain exact committed runner evidence before closing M1A (M1-CI-001).
9. Completed in source: dependency changes trigger lock-bound exception validation and live
   advisory/license/ban/source checks; obtain exact committed workflow evidence (M1-DEP-001).
10. Completed in source: structured candidate IO, Agent skill presentation state, and CLI
    performance setup cross `canisend-app`; the CLI has no Store/IO/Resources edge (M1-ARCH-003).
11. Completed in source: the Store→IO exception is explicitly accepted for Alpha.6 through its
    2026-08-17 expiry, with an injectable render boundary and named renderer/projector failure,
    stale-at-commit, CAS classification, path-conflict, `repair-required`, and convergence evidence
    (M1-ARCH-001/002/004).
12. Completed locally: the current source passes the full Workspace, frontend, Chrome accessibility,
    debug-host, dual-Pack documentation, Host Agent, Clippy, and release-source preflight. The
    preflight found and fixed a stale Agent v2 smoke that initialized generic Workspace v3 instead
    of the explicit academic compatibility authority. The Apple Silicon `release-alpha` CLI, ZIP,
    and read-only DMG package paths also build and pass their complete local smoke suites.
13. Next external boundary: create the missing GitHub milestones/roadmap work items and protected-ref
    rules, push the local commit series, and obtain exact current-commit M1A runner/dependency evidence.
    These repository mutations require explicit authority. Only after those gates pass may explicit
    release authority apply the reviewed Alpha.6 plan and start exact candidate qualification.

Only after evidence proves every applicable gate should the release status advance from Alpha.5
and the goal be considered achieved. GF4 completion alone is not an Alpha qualification event.
