# R11 package-manager qualification recorder

**Date:** 2026-07-18

**Roadmap item:** Stable package-channel qualification preparation

**Status:** Guarded ledger path implemented; full four-record native evidence pending

## Outcome

The existing package evidence verifier now returns the one common GitHub run identity and exact four-record count.
`release record-package-qualification` consumes that verified summary and is dry-run-first: it prints the candidate
ledger's before/after digests and performs no write without an explicit final `--write` from a clean worktree.

Promotion requires the exact qualified Beta tag, current RC workspace/version, frozen feature baseline, canonical
`candidates-only` state, and an already successful signed matrix for the same RC tag. Only `package_managers` may
change. Upgrade, documentation/uninstall, release-note, and Stable authorization fields remain independent.

## External boundary

Hosted prequalification deliberately stops at two Homebrew records and one Scoop record. The fourth WinGet record
must be produced by the bundled lifecycle in a fresh Windows Sandbox and must retain the same run ID and candidate
source digests. Maintainers must inspect the public signed assets, hosted run, Sandbox output, and absence of skipped
or tolerated failures before ledger write. A locally constructed four-file directory is not qualification evidence.

## Local verification

- `cargo test -p xtask --locked`: 41 passed, including exact Beta/recorded-RC promotion guards
- `cargo run -p xtask --locked -- release check`
- `cargo clippy -p xtask --locked -- -D warnings`
- `git diff --check`

No package repository or public manifest was changed. Stable package publication remains blocked on the future real
four-record lifecycle and all other Stable gates.

Exact implementation commit `3bda1ba` passed all eight ordinary CI jobs in GitHub Actions run `29643318078`.
