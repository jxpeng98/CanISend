# R11.3 documentation and uninstall evidence

**Date:** 2026-07-18

**Roadmap item:** R11.3 documentation and uninstall qualification

**Status:** Same-run RC evidence path implemented; signed RC execution pending

## Outcome

The native release matrix now emits one canonical record from each target's actual extracted-archive smoke. The
record is written only after exact executable comparison, complete notice inventory, native version/doctor,
documented quick-start, host-agent flow, isolated install, uninstall, and retained-workspace assertions pass.

RC assembly gathers exactly five records and verifies them against the complete signed release manifest. Promotion
to `documentation_uninstall.status = passed` additionally requires the common evidence run ID to equal the
`signed_matrix_run` already committed for that exact RC tag. This closes the previous gap where Alpha preparation
could prove the mechanism but could not safely identify a qualifying RC matrix.

## Privacy and publication boundary

Only tag, target, runner environment, archive SHA-256, run ID, observed version, check booleans, and a UTC timestamp
leave the target runner. Synthetic workspace bodies, SQLite files, host packs, command output, and paths remain under
the temporary smoke directory. The evidence path does not publish a release or package channel and cannot mutate the
ledger without a clean-worktree `--write` command.

## Local verification

- `cargo test -p xtask --locked`: 39 passed, including policy, archive binding, false-check rejection, and same-run
  ledger promotion tests
- `cargo run -p xtask --locked -- release check`: five-record same-RC-run policy passed with all other release gates
- `bash -n scripts/smoke_release_archive.sh`
- a real macOS arm64 Alpha archive ran the complete quick-start, host-agent, isolated install/uninstall, and retained
  workspace smoke, then emitted a canonical nine-check JSON record
- `git diff --check`

The Alpha mechanism run is not RC qualification evidence. The roadmap checkbox remains open until a future signed RC
release run passes all five jobs, is recorded as an RC matrix, and its public assets, attestations, and evidence
artifact are independently inspected before ledger promotion.

Exact implementation commit `ea17e1e` passed all eight ordinary CI jobs in GitHub Actions run `29643087196`.
