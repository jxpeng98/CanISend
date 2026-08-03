# M1-CI-001 — Fast cross-platform and browser gates

Date: 2026-08-03

## Outcome

`.github/workflows/fast-ci.yml` now provides the pre-release feedback required by M1-CI-001
without turning development jobs into release evidence:

- `browser-keyboard-accessibility` runs the pinned Node.js, pnpm, Playwright, and Chrome versions
  on `ubuntu-24.04`, then executes the 14 critical browser checks;
- `cross-platform-core` runs one lightweight matrix on `ubuntu-24.04` and `windows-2025` for
  `canisend-core`, `canisend-store`, `canisend-io`, `canisend-cli`, and `canisend-mcp`; and
- the existing three Apple Silicon jobs retain full formatting, Clippy, workspace, property,
  desktop-build, and development-smoke ownership.

The browser suite covers automated accessibility, keyboard dismissal and focus restoration,
active navigation and tab styling, English and Simplified Chinese, 200% text reflow, reduced
motion, compact/dark variants, and layout overflow. The core matrix deliberately excludes release
profiles, packaging, signing, publication, and whole-workspace repetition.

## Machine-enforced ownership

`release/native-test-ownership.json` now names all three runner classes, the five logical jobs,
the exact browser and core commands, and their non-authoritative scope. `xtask release check`
rejects removal or drift of:

- either Ubuntu or Windows matrix member;
- Rust `1.97.0`, Node.js `26.5.0`, pnpm `11.17.0`, or pinned Chrome setup;
- any of the five named Rust packages;
- the critical browser command; or
- the no-release-profile boundary.

This keeps fast feedback distinct from the five-target packaged-binary matrix. A green PR job
cannot authorize an Alpha artifact, and native candidate qualification cannot be replaced by this
development matrix.

## Local verification

- `pnpm --dir apps/canisend-desktop test:accessibility` — 14 Chrome tests passed
- `cargo test --locked -p canisend-core -p canisend-store -p canisend-io -p canisend-cli -p canisend-mcp`
- `cargo test -p xtask --locked` — 77 tests passed
- `cargo run -p xtask --locked -- release check`
- `cargo fmt --all -- --check`
- `git diff --check`
- Ruby/Psych parse of `.github/workflows/fast-ci.yml`

The local core command ran on macOS and proves the selected package suite and command. It does not
claim Ubuntu or Windows behavior.

## Remaining boundary

M1-CI-001 is implemented in source, but the M1A exit checkbox remains open until the committed
workflow completes on GitHub's exact Ubuntu, Windows, and Chrome runners. M1-MSRV-001 likewise
still requires an exact current-commit remote result. Release qualification, Alpha.6 version
application, candidate creation, tagging, pushing, and publication were not performed here.
