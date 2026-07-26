# Stage 4B recovery and workflow-control evidence

**Date:** 2026-07-26

**Source version:** `1.0.0-alpha.2`

**Scope:** Stage 4B source implementation, hosted Fast CI, and exact Alpha.2 candidate evidence

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
| B6 accessibility qualification | `d091d14` | deterministic Tab-ring anchor and exact-package qualification |

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

The broader local equivalent of hosted Fast CI also passed:

| Check | Result | Local wall time |
|---|---|---:|
| complete workspace test suite | pass, including bounded loopback fixtures | 24.54 s |
| workspace Clippy, all targets and features | pass with warnings denied | 3.23 s |
| Alpha-profile CLI and GUI build | pass from source checkpoint `49f8526` | 10.11 s |
| local Apple Silicon package creation | pass; nested and outer ad-hoc signatures verified | 7.07 s |
| bounded packaged archive smoke | manifest, signatures, documented workflow, host-agent workflow, and GUI launch passed | 6.29 s |
| packaged recovery/control smoke | start, begin, rerun, backup, restore, repair, check, and reopen passed | pass |

The local package was
`CanISend-1.0.0-alpha.2-aarch64-apple-darwin.zip`, 48,832,099 bytes, with SHA-256
`b6c9ddd935d89c84a307e471fbbd4f032ac7cfa841db7158b85850e89b901472`. It is a disposable local
artifact under `/tmp`, not a publication candidate or release asset.

## Hosted and exact-candidate verification

Hosted Fast CI run
[`30215205425`](https://github.com/jxpeng98/CanISend/actions/runs/30215205425) passed at source
`d091d147390474514145b154b88b2cf443e6f7c1`. The quality lane completed in 50 seconds and the
complete test/development-package lane completed in 1 minute 20 seconds, keeping the critical path
below five minutes.

The non-publishing native candidate run
[`30215276643`](https://github.com/jxpeng98/CanISend/actions/runs/30215276643) then passed in 21
minutes. It built and smoked the five supported CLI targets, built the Apple Silicon desktop app
with its version-matched CLI, verified community signing policy, completed the Windows release
contracts, assembled the SBOM/manifest/checksums, and attested the resulting assets. The complete
candidate artifact was downloaded rather than rebuilt locally. `xtask release verify-candidate`
verified all 13 files against `v1.0.0-alpha.2` and exact source `d091d14`.

The downloaded desktop archive was 49,466,754 bytes with SHA-256
`1891f184583d5d221af8e02cb521c49cce0503709e53586790be257dd30e2a9f`. Its exact bytes passed:

- bounded archive, bundle manifest, nested/outer ad-hoc signature, documentation, and GUI launch;
- packaged host-agent and synthetic workflow smoke;
- English and Simplified Chinese accessibility semantics, deterministic Tab order, 200% text and
  focus visibility, and reduced-motion behavior;
- workflow start/begin/rerun followed by backup, restore, repair, check, and separate-process
  reopen; and
- terminal CLI migration, same-version update, rollback uninstall, and workspace retention.

Stage 4B is therefore qualified. No tag, GitHub Release, public asset, or update response was
created; publication remains a separate explicitly authorized action.
