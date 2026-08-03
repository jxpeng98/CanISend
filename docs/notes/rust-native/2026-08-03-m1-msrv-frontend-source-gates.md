# M1-MSRV-001 / M1-CI-002 — Toolchain and frontend source gates

Date: 2026-08-03

## Outcome

CanISend now declares the Rust version that it actually pins and continuously configures. The
Workspace manifest and README declare Rust 1.97, while `rust-toolchain.toml`, Clippy, all stable
workflow actions, compiler-cache namespaces, and release test ownership use Rust 1.97.0.

The native release candidate's `source-gates` job now owns the frontend source suite instead of
only building UI assets later in platform packaging jobs. A manual candidate dispatch cannot reach
release assembly unless the same candidate commit passes locked dependency installation, Svelte
formatting, Svelte and TypeScript checking, unit tests, production build, Chrome installation, and
the bounded accessibility/browser suite.

The first activation established one Prettier baseline across 120 previously drifting UI source
and configuration files. The rewrite was mechanical; one source-text assertion was made
format-insensitive, and the complete type, unit, build, and browser suite passed afterward.

## Source enforcement

`xtask release check` now validates:

- Workspace `rust-version = "1.97"`;
- exact `rust-toolchain.toml` and Clippy `1.97.0` pins;
- the README Rust 1.97+ badge;
- every `dtolnay/rust-toolchain` use across active workflows;
- the release ownership ledger's exact Rust, Node, pnpm, browser, and frontend commands;
- presence of every frontend command inside the candidate-only Ubuntu `source-gates` job;
- the exact two accessibility spec files behind `test:accessibility`; and
- the Playwright Chrome channel binding.

Historical excluded spikes and dated implementation notes retain their original Rust facts; they
are not active product or release claims.

## Local verification

- `cargo test -p xtask --locked native_test_ownership_runs_the_source_suite_once`
- `cargo run -p xtask --locked -- release check`
- `pnpm --dir apps/canisend-desktop format:check`
- `pnpm --dir apps/canisend-desktop check`
- `pnpm --dir apps/canisend-desktop test` — 72 tests
- `pnpm --dir apps/canisend-desktop build`
- `pnpm --dir apps/canisend-desktop test:accessibility` — 13 Chrome tests
- YAML parse of `.github/workflows/release.yml`

## Remaining boundary

This completes the M1-MSRV-001 and M1-CI-002 source implementation. The changed commit still needs
an exact remote CI/candidate result before the M1A evidence checkbox for continuously proven MSRV
can be marked complete. M1-CI-001's lightweight Linux/Windows PR jobs and fast-CI browser job are
separate P1 work, and native release qualification remains mandatory.
