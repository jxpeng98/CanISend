# Unpublished App and CLI parity readiness matrix

## Candidate identity

| Fact | Value |
|---|---|
| Candidate source commit | `0f88a1f15a620b00c1d6ae4c4b8f03b6b08faeba` |
| Product/test commit | `f03cb412b20f37f3a0ff5ad3af14c39a3ab66845` |
| Coupling-inventory commit | `1fa42081f17705de9b309297f485932672a410be` |
| Visual-regression commit | `8959cc16e23c3a28b7026500a89adad84702ab46` |
| Feature-freeze record commits | `f32afb55efcb642672f4820e012dd77d01550ccb`, `0f88a1f15a620b00c1d6ae4c4b8f03b6b08faeba` |
| Candidate type | Unpublished macOS Design Preview |
| Product version | `1.0.0-beta.1` (unchanged) |
| Publication authorization | No (`publication_allowed == false`) |

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
| Final source and release gate | Repository release policy | diff check; desktop format, 85 unit tests, 17 Playwright tests, Svelte check; Rust format and strict affected-package Clippy; final `release check` | Passed |
| Packaged App and bundled-host smokes | macOS Design Preview builder | Clean-source staged bundle verification; documented quick-start; project/global Skill lifecycle; guarded dual-Pack MCP lifecycle; backup, restore, and reopen | Passed |
| Packaged App inspection | Isolated macOS Design Preview | Workspace create/connect/reopen; both Packs; four canonical Skills; MCP-created Revision 7 Application; Settings version; English and Simplified Chinese layouts | Passed |
| Protected pull request | GitHub ruleset | Six required Fast CI checks and commit-preserving merge | Pending |

## Candidate artifact evidence

| Fact | Value |
|---|---|
| Receipt status | `ready-local-design-review` |
| Receipt SHA-256 | `f862a7d4b5ca24724f57a7bd20bd2342ac3bd6df931cc219e00a701756e0cb05` |
| Bundled host SHA-256 | `de0b8972bd4fd35ef2137d07bc7edf4f58a3b9c5c8567367e58be660b79a7edf` |
| Integrity manifest SHA-256 | `b13b0cfb7b5d0fe7db30630f3783529dadb639f4062201b7a38dcac03a2280ca` |
| Signing and distribution | Apple ad hoc; not notarized; unpublished |

## Warnings and blockers

- Warning: macOS debug linking emitted the existing compact-unwind size warning; all owning checks passed.
- The first preview build could not fetch the pnpm signature inside the restricted sandbox; the approved network retry completed without changing source or dependencies.
- The first preview run exposed stale Playwright copy expectations. They were corrected, exception-bound, and the complete 17-test browser suite then passed.
- Pending protected-PR evidence is not counted as passed.
- No release tag, public artifact, notarization, cohort result, or five-target qualification is claimed.
- Current blocker: none reproduced; the decision remains pending until the protected PR is green and merged.
