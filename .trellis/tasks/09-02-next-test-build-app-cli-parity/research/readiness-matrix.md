# Unpublished App and CLI parity readiness matrix

## Candidate identity

| Fact | Value |
|---|---|
| Product/test commit | `f03cb412b20f37f3a0ff5ad3af14c39a3ab66845` |
| Feature-freeze record commit | `f32afb55efcb642672f4820e012dd77d01550ccb` |
| Candidate type | Unpublished macOS Design Preview |
| Publication authorization | No |

## Fresh source evidence

| Requested workflow | Authority or owner | Fresh evidence | Result |
|---|---|---|---|
| Sidebar metadata removal and Settings version | Desktop presentation | `accessibility-contract.test.ts` | Passed |
| Concise English and Simplified Chinese copy | Desktop i18n | `i18n.test.ts`; `svelte-check`; Prettier check | Passed |
| Four canonical v4 Skill labels | Resources plus desktop presentation | canonical resource manifest test; desktop Skill ID source contract | Passed |
| Skill install, upgrade, removal safety | `canisend-resources` | idempotent/edit-safe and unsupported-version regressions | Passed |
| Workspace create and reopen | App facade plus desktop adapter | shared-registry desktop command regression | Passed |
| Nested and explicit-marker Workspace discovery | Store plus compiled CLI | Store discovery contract; compiled CLI binary contract | Passed |
| Missing Workspace marker fails without mutation | Store plus compiled CLI | compiled CLI binary contract with unchanged snapshot | Passed |
| Generic and academic Pack lifecycle | App facade plus desktop adapter | Pack-driven desktop semantic lifecycle regression | Passed |
| Guarded Plan commit and CLI readback | MCP plus compiled CLI | MCP lifecycle with separate `plan show` process | Passed |
| App, CLI, and MCP operation inventory | Typed operation registry | `xtask operations check` (`31` CLI, `129` Tauri, `36` MCP leaves) | Passed |
| Cross-surface operation semantics | Semantic parity policy | `xtask semantics check` (`38` shared operations) | Passed |
| Final source and release gate | Repository release policy | Final checks on clean PR head | Pending |
| Packaged App and bundled-host smokes | macOS Design Preview builder | Bundle receipt, quick-start, host, and MCP smokes | Pending |
| Protected pull request | GitHub ruleset | Six required Fast CI checks and commit-preserving merge | Pending |

## Warnings and blockers

- Warning: macOS debug linking emitted the existing compact-unwind size warning; the focused desktop tests passed.
- Pending evidence is not counted as passed.
- No release tag, public artifact, notarization, cohort result, or five-target qualification is claimed.
- Current blocker: none reproduced; readiness remains undecided until every pending row is resolved.
