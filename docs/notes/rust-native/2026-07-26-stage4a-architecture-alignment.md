# Stage 4A architecture alignment evidence

**Date:** 2026-07-26

**Source version:** `1.0.0-alpha.2`

**Scope:** Stage 4A source implementation and local macOS Fast CI equivalent

## Outcome

Stage 4A completed the architecture-only alignment required before workspace recovery and workflow
control features are added:

- `canisend-app` is split into typed application, error, receipt, system, workspace, job, and
  workflow modules.
- `ActionReceipt<T>` can carry adapter-neutral artifacts, consents, warnings, and next actions
  without changing its empty-metadata serialization.
- `ApplicationError` has one structured Agent/GUI failure classification.
- the GUI binary entry point is 13 lines and delegates to separated desktop, state, component,
  page, dialog, and worker modules;
- GUI background work uses typed `WorkerRequest -> WorkerEvent` messages rather than page-owned
  closures; and
- the 13 overlapping Alpha CLI operation families now call `canisend-app` while preserving the
  existing Agent v2 and human-output contract.

No workspace schema, resource format, Agent protocol version, GUI operation count, or parity status
changed. CLI/GUI parity remains `17 implemented` and `18 deferred-beta`.

## Atomic commits

| Work package | Commit | Result |
|---|---|---|
| A0 execution authority | `71eddcf` | one active 1.0/Stage 4 roadmap authority |
| A1 typed application boundary | `fceacec` | modular app facade, receipts, and failures |
| A2 GUI mechanical split | `c1e0041` | launch, state, components, pages, and dialogs separated |
| A3 typed worker boundary | `4570a4d` | all current GUI background operations are typed |
| A4 CLI application adapter | `1cb8250` | 13 overlapping CLI families use the app facade |

## Verification

The following checks passed on the maintained Apple Silicon development machine:

| Check | Result | Warm local wall time |
|---|---|---:|
| `cargo test --workspace --locked` | pass | 13.72 s |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass | 2.62 s |
| `cargo build --locked -p canisend-cli -p canisend-gui` | pass | 3.60 s |
| CLI binary contract | 16 pass | included above |
| App tests | 16 pass, 1 public-network test ignored | included above |
| GUI tests | 14 pass | included above |
| host-agent smoke | pass | 4.4 s |
| `cargo run -p xtask --locked -- release check` | pass | below 1 s warm |

The complete workspace suite includes two bounded local HTTP-listener tests. The managed sandbox
does not allow binding the loopback listener, so the same suite was repeated outside that
restriction and both tests passed. No public host was contacted.

The local source loop is comfortably below five minutes. The current commits have not yet been
pushed, so their GitHub-hosted Fast CI run remains pending. The preceding warm hosted baseline was
54 seconds for the quality lane and 85 seconds for the test lane.

## Contract checks

- Existing `canisend version --json` data remains versioned and native.
- Existing `canisend doctor --json` fields and native renderer evidence remain present.
- Workspace init/status/check/backup/restore binary coverage remains green.
- Job create/import/list/show/archive binary coverage remains green.
- Workflow start/status and the existing begin/complete/rerun binary coverage remain green.
- CLI errors retain stable error codes, retryability, remediation, and exit classes.
- The GUI still requires no shell or runtime discovery.
- Page and dialog modules execute no application work directly; they emit typed worker requests.

## Next stage

Stage 4B can begin with application-level workspace restore and repair actions. Parity must not be
advanced until each new action has its CLI adapter, GUI path, focused tests, and source evidence.
