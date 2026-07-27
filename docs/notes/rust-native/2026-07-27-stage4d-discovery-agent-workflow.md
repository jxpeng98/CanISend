# Stage 4D discovery and Agent-workflow evidence

**Date:** 2026-07-27

**Source version:** `1.0.0-alpha.2`

**Scope:** Stage 4D discovery, Agent v2 task lifecycle, and Agent integration GUI parity

**Qualification status:** Complete

## Outcome

Stage 4D implements three GUI operation families through the same Rust application and store
boundaries used by the CLI:

- `discovery.*`;
- `task.*`; and
- `agent.*`.

The Discovery surface previews reviewed local batches and user-invoked public refreshes before
commit, exposes provenance and history, suggests possible duplicates without merging, and promotes
one lead explicitly. The selected-job task panel prepares revision-bound work, exports only declared
inputs with separate consents, validates completion files before commit, and supports cancel,
stale, and prepare-again recovery. Agent integration exposes only public capability/context
metadata and exports verified Codex, Claude, or generic resource packs to a new or empty directory.

The parity authority is now `29 implemented` and `6 deferred-beta`. Only these three fully
qualified families moved to `implemented`; document, review, package, render, schema, and resource
operations remain explicitly deferred.

## Atomic commits

| Work package | Commit | Result |
|---|---|---|
| D0 tracking authority | `3b33009` | dependency order and Stage 4D invariants |
| D1 discovery application | `df779bc` | typed adapter, preview/commit, lead, and promotion actions |
| D2 discovery CLI alignment | `12ff972` | discovery commands use the application facade |
| D3 discovery GUI | `fcd9c79` | reviewed imports, refresh, history, suggestions, and promotion |
| D4 task application | `b406e87` | typed prepare/export/complete/cancel/recovery actions |
| D5 task CLI alignment | `6b4ad4e` | task commands use the application facade |
| D6 task GUI | `600f3f4` | scoped task lifecycle with independent consents |
| D7 Agent application | `cd0e270` | body-free context and verified host-pack export |
| D8 Agent CLI alignment | `55ad8de` | capability/context/assets commands use the facade |
| D9 Agent GUI | `6ebf4c7` | optional-job inspection, copy controls, and previewed export |
| D10 persistence qualification | `1014910` | discovery-to-task-to-agent reopen regression |

## Persistence qualification

The D10 GUI qualification regression creates an isolated workspace, previews and commits one
host-agent discovery batch, promotes its lead, imports a synthetic private advert, starts the
workflow, and prepares one Host-agent `job-parse` task. It reads the selected-job Agent context and
proves that the context is public, reports the persisted open task and source count, and excludes a
private sentinel plus all body-shaped fields.

The regression then reopens the workspace and proves:

- the discovery source and promoted lead retain identity and promotion linkage;
- the promoted job retains identity;
- the prepared task retains descriptor identity and status;
- the complete Agent context is unchanged and body-free; and
- a generic resource pack contains its manifest and all 31 verified resources.

This exercises the GUI worker and shared application boundary without adding a GUI-only persistence
representation.

## Verification evidence

| Check | Result |
|---|---|
| focused GUI qualification | 34 passed |
| D10 persistence regression | passed |
| affected GUI/xtask all-target Clippy | passed with warnings denied |
| formatter, shell syntax, and diff check | passed |
| source release check | passed at `29 implemented`, `6 deferred-beta` |
| macOS release build | passed for Apple Silicon CLI and GUI |
| staged bundle verification | final-byte manifest, layout, version, and nested/outer ad-hoc signatures passed |
| hosted Fast CI | run `30230838748` passed; quality 53 s, tests 2 min 37 s |
| packaged Accessibility smoke | English and Simplified Chinese semantics, exact Tab order, 200% text/focus visibility, reduced motion, and reset passed |

The packaged smoke launches the verified `.app` through macOS LaunchServices while passing an
isolated `HOME`; it therefore exercises the bundle launch contract without reading or changing the
user's GUI registry. It does not execute the inner Mach-O as an unregistered standalone process.

The staged app under `/private/tmp` is disposable local qualification output, not a release asset.
No tag, package-manager update, five-target native matrix, or public release was created.
