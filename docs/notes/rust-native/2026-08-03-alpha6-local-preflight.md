# Alpha.6 local source preflight

Date: 2026-08-03

## Scope

This preflight exercised every current-HEAD fast-CI owner available on the Apple Silicon development
machine before any Alpha.6 version write, push, candidate workflow, tag, or publication. The source
implementation commit after the discovered smoke fix is
`92ed8bb3bdd4974b6f817714226d03c6ed9c525e`.

## Frontend and browser results

- locked pnpm installation: current lockfile already satisfied;
- Prettier check: passed;
- Svelte/TypeScript check: zero errors and zero warnings;
- Vitest: 13 files and 72 tests passed;
- native-preview evidence policy: 4 tests passed;
- production Vite build: passed;
- pinned-browser accessibility path: 14 Playwright tests passed, covering automated accessibility,
  keyboard traversal, focus/navigation state, reduced-density controls, bilingual routes, and
  Simplified Chinese reflow at 200% text.

## Rust, contracts, and workflow results

- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: passed;
- `cargo test --workspace --locked`: passed across contracts, Core, Store, IO, App, CLI, MCP,
  desktop, resources, xtask, generic examples, migration/fault-injection, semantic parity, and
  recovery tests;
- expected release-binary, signed-fixture, network, and scheduled-performance tests remained
  ignored under their declared owners;
- debug CLI and Tauri unified host build with `canisend-gui/custom-protocol`: passed;
- dual-Pack documented quick-start smoke: passed;
- bounded Agent v2 Host Agent smoke: passed after the compatibility correction;
- `cargo run -p xtask --locked -- release check`: passed, including the 26-edge actual graph,
  25-edge target, and one expiring Store→IO exception.

macOS emitted its known debug-linker compact-unwind size warning while building the large test and
debug binaries. It did not affect compilation, Clippy, runtime smoke, or release-profile evidence;
release candidate size/signature behavior remains owned by the exact native matrix.

## Blocking regression found and fixed

`scripts/smoke_host_agent.sh` previously used default `workspace init`. The canonical default now
creates a generic Workspace v3 authority, where legacy Agent v2 `job.create` correctly fails with
`compatibility.unavailable`. The smoke therefore failed before creating its first Job.

Commit `92ed8bb` makes the legacy smoke select `--pack academic-job`, which creates the intentional
academic v2 compatibility authority. `xtask release check` now rejects removal of that explicit
selection. The generic default remains unchanged and fail-closed; the dual-Pack quick-start owns the
canonical generic v3 journey.

## Boundary

This is strong local source evidence, not release evidence. The implementation commit remains
version `1.0.0-alpha.5` and was 36 commits ahead of remote `main` at
`cb2db0f772ff1931c84427becd4674c59acf9028` when inspected. Linux/Windows fast CI, dependency assurance, the exact
Alpha.6 native candidate matrix, packaged lifecycle, real Codex/Claude dogfood, tag promotion, and
public-byte re-verification have not run on this source commit.
