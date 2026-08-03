# M1F workflow-pack framework verification

Date: 2026-08-03

## Decision

The M1F source implementation now satisfies the eleven Roadmap acceptance contracts represented
by GitHub Issues #19–#29. The last functional gap was that canonical Agent v3 and Application v3
operations admitted only `org.canisend.generic-application`; a Workspace v2 migrated to v3 was
therefore bound correctly to `org.canisend.academic-job` but could not complete the same neutral
flow through Agent v3, CLI, MCP, or desktop.

The canonical application facade now resolves the exact Workspace or Application Pack tuple
`(id, version, digest)` through the verified built-in registry. Generic v3 and migrated Academic
v3 Applications use one plan, compose, review, approval, render, and export engine. The old
`generic-*` command spellings remain compatibility aliases; they no longer select or authorize a
Pack. Workspace v2 still fails closed until the explicit dry-run-first migration is committed.

## Protected integration evidence

M1F is `Verified`. [PR #98](https://github.com/jxpeng98/CanISend/pull/98) merged through protected
`main` as commit
[`48da419c1ff54e076e428a658eee8dc8836a8dbe`](https://github.com/jxpeng98/CanISend/commit/48da419c1ff54e076e428a658eee8dc8836a8dbe).
The exact merge commit passed [Fast CI run
`30849789890`](https://github.com/jxpeng98/CanISend/actions/runs/30849789890) and
[dependency-assurance run
`30849787300`](https://github.com/jxpeng98/CanISend/actions/runs/30849787300). Issues #19–#29 carry
those links, are labeled `state:verified`, and are closed. GitHub milestone M1 closed with 26/26
items complete on 2026-08-03.

This is framework/source verification, not release evidence. Alpha.6 package, native lifecycle,
Host Agent, tag, and public-download qualification remain owned by M2.

## Closure matrix

| Roadmap item | Source evidence and acceptance result |
|---|---|
| M1F-PACK-001 / #19 | Pack v1 schema, bounded manifests/resources, data-only capabilities, digest checks, and exact built-in/external-origin registry admission pass malformed, oversize, unsafe-path, executable, unknown-capability, incompatibility, and digest-mismatch tests before mutation. This does not claim that an external Pack installer ships in Alpha.6. |
| M1F-DAG-001 / #20 | Pack-qualified stage IDs and validated DAGs reject duplicate/missing IDs, cycles, unreachable terminals, illegal modes/outputs, and bounds violations; dependency-scoped stale propagation and rerun remain deterministic. |
| M1F-DELIV-001 / #21 | Academic and Generic Packs have disjoint Deliverable catalogs and both complete plan, compose, review, managed projection, Typst render, and consented PDF export through the same Application v3 facade. |
| M1F-MODEL-001 / #22 | Opportunity, Application, Requirement, Plan, Deliverable, and derived boundaries are neutral and carry exact Pack identity. Existing Applications resolve their recorded tuple; new v3 Applications resolve the Workspace tuple; v2 returns migration remediation. |
| M1F-MIG-001 / #23 | Immutable Workspace v2 fixtures migrate Jobs, stages, artifacts, revisions, dependencies, review/package/render/audit authority to Academic v3 with backup and referenced Blob identity preserved. |
| M1F-MIG-002 / #24 | Interruption, retry, corrupt input, low space, busy database, old/downgrade binary, restore, and projection-conflict fixtures fail atomically to recoverable v2 or valid v3 state. Exact packaged-binary lifecycle repetition remains M2-LIFE-001. |
| M1F-PROJ-001 / #25 | Pack-neutral `applications/APPLICATION_ID/` projections and bounded migrated `jobs/JOB_ID/` recognition cover edited, unmanaged, symlink, conflict, copy/replace, repair, and staged-restore paths without overwriting authority. |
| M1F-INVALID-001 / #26 | Same-Pack change review preserves label-only outputs and scopes Requirement, workflow, template, renderer, validator, and Evidence invalidation to dependency-reached state; failure remains atomic. |
| M1F-ACADEMIC-001 / #27 | The Academic Pack supplies ten stages, four Deliverables, templates, prompts, validators, discovery references, and deterministic neutral rendering. A migrated Academic v3 Application now completes all ten stages and exports two selected PDFs without a legacy business-engine branch. |
| M1F-COMPAT-001 / #28 | Agent v2, `job` CLI, and `jobs/JOB_ID` remain bounded to the Academic Pack, identify canonical v3 remediation, and reject Generic mappings instead of guessing. |
| M1F-SURFACE-001 / #29 | App facade, CLI, MCP, Agent v3, and desktop resolve the exact Pack. Academic lifecycle fixtures now cover CLI, MCP approval/recovery, and desktop migration/resume; operation and semantic registries classify the Pack-neutral bindings. English/Chinese, keyboard, 200% reflow, and automated accessibility gates pass. |

## Implementation completed

- Added Pack-neutral `application_flow_v3`, create, plan, compose, review, approve, and export
  facade methods. Deprecated generic-named facade methods delegate without changing behavior.
- Changed Agent v3 capability, context, creation, mutation, and export paths to bind the exact
  Workspace/Application Pack rather than rejecting migrated Academic Workspaces.
- Changed CLI, MCP, and Tauri canonical v3 adapters to use the neutral facade while retaining
  compatibility command and handler names required by the frozen operation registry.
- Added an Academic Agent v3 migration/admission fixture and complete migrated Academic lifecycle
  fixtures for the facade, real CLI binary, MCP approval broker, and Tauri command implementation.
- Added deterministic `generated-date: none` to the neutral Typst projection so the Academic
  templates render without consulting the clock or private metadata.
- Expanded semantic parity to Academic v3 CLI, Tauri, and MCP cases and changed canonical
  application/Agent-v3 Pack scope from generic-only to exact-Pack `any`.
- Refreshed the fail-closed domain-coupling inventory after the new Academic compatibility
  fixtures changed its exact file/classification digest.

## Local verification

The working-tree candidate passed:

- `cargo fmt --all -- --check`;
- `cargo test --workspace --all-targets --locked` (all product suites passed; 85/85 `xtask`
  policy tests passed after the two intended policy-baseline updates);
- strict `cargo clippy` with all targets/features and `-D warnings` for Contracts, Core, IO,
  Store, App, CLI, MCP, desktop, and `xtask`;
- `xtask schemas check`: 40 public, 7 Application v3, and 1 Workflow Pack schema;
- `xtask resources check`: 4 exact template resources;
- `xtask operations check`: 86 CLI, 111 Tauri, and 22 MCP leaves;
- `xtask semantics check`: 8 shared operations, 5 preview/commit pairs, 5 read families, 71
  qualified and 148 explicitly uncovered bindings;
- `xtask scope check`: 189 files, 4 classifications, and 8 required areas;
- `xtask release check` with the current Alpha.5 source/public truth intact;
- Svelte/TypeScript check with 0 errors and 0 warnings, 72/72 UI unit tests, production build,
  and 14/14 pinned-Chrome accessibility/keyboard/reflow tests.

Focused acceptance fixtures also pass for Workflow Pack validation, Workspace migration and
failure recovery, Application Pack invalidation, neutral Generic and Academic Application flows,
Academic Agent v3 admission, real CLI lifecycle, MCP review/approval/export recovery, and desktop
migration/resume behavior.

## Remaining boundary

1. Commit and merge this exact candidate through the protected `main` branch.
2. Inspect and link required Fast CI, cross-platform core, browser, and dependency results on the
   exact merge commit.
3. Transition Issues #19–#29 from `state:in-progress` to `state:verified`, close the M1 milestone,
   and record the merge/run identities in committed evidence.
4. Begin M2 with Alpha.6 scope freeze and the atomic version/package-contract/GPL transition.
5. Build once and qualify the exact five-target/native candidate, migration/rollback lifecycle,
   Codex and Claude Academic Pack runs, promotion, publication, and independent public download.

No tag, release asset, public metadata, or package-manager entry is changed by M1F verification.
