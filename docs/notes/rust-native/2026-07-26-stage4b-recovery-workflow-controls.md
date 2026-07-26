# Stage 4B recovery and workflow-control evidence

**Date:** 2026-07-26

**Source version:** `1.0.0-alpha.2`

**Scope:** Stage 4B source implementation and local macOS source-gate evidence

## Outcome

Stage 4B adds five native GUI operation families through the shared Rust application boundary:

- `workspace.restore`;
- `workspace.repair`;
- `workflow.begin`;
- `workflow.complete`; and
- `workflow.rerun`.

Workspace restore verifies a backup and publishes a separate destination before the GUI changes its
registry or selection. Projection repair uses the store's managed-projection policy, preserves
edited files, reports the repaired count, and is idempotent.

The workflow timeline consumes compiled stage descriptors. Ready stages expose only supported
execution modes; completion resolves the current artifact reference from the workspace; rerun
previews the graph target, descendants, and affected current outputs before confirmation. The GUI
does not execute the copyable CLI commands it displays.

Parity is now `22 implemented` and `13 deferred-beta`. Stage-specific artifact creation, dedicated
plan confirmation, criteria, profile evidence, discovery, tasks, documents, review, package,
render, and export GUI surfaces remain later Stage 4 work.

## Atomic commits

| Work package | Commit | Result |
|---|---|---|
| B1 application recovery | `adc4650` | typed restore/repair actions and malformed-input regressions |
| B2 GUI recovery | `2767c51` | native pickers, confirmations, registry-after-success, repair health |
| B3 application workflow controls | `40e53e9` | descriptor-bound begin/complete and graph rerun preview/action |
| B4 CLI adapter alignment | `a38fabf` | five Stage 4B commands route through the application facade |
| B5 GUI workflow controls | `1a95c28` | timeline actions, artifact validation, CLI handoff text, rerun confirmation |

## Source verification

The complete B6 source gate passed on the maintained Apple Silicon development machine:

| Check | Result | Local wall time |
|---|---|---:|
| store contract | 13 passed | 8.38 s |
| application facade | 22 passed; 1 bounded public-network test ignored; 2 package-fixture tests ignored | 4.89 s |
| native GUI | 16 passed | 7.34 s |
| CLI binary contract | 16 passed | 11.66 s |
| app/GUI/CLI Clippy, all targets | pass with warnings denied | 9.82 s |
| formatter and diff check | pass | below 1 s |
| release check | pass; `22 implemented`, `13 deferred-beta` | below 1 s |

The five principal test/Clippy commands ran concurrently; their combined wall time was about 11.7
seconds rather than the sum of lane times. This remains comfortably below the five-minute Fast CI
budget.

Exact packaged macOS lifecycle, ad-hoc signature, staged-candidate download, and hosted Fast CI
evidence remain candidate qualification work and are not implied by local source completion.
