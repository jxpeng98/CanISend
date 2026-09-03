# Unpublished desktop shell and Workspace migration verification

## Candidate identity

| Fact | Value |
|---|---|
| Candidate source commit | `23d884b8a6aa4e559a8d8995d74acf47721d3f55` |
| Product commit | `5734ffac8138b8f1690db4161c3af4959cde8669` |
| UI contract correction | `ee617c08ff24fbb5567afda8a42cbefd197a3b9c` |
| Candidate type | Local macOS Design Preview |
| Product version | `1.0.0-beta.1` (unchanged) |
| Pull request | [#217](https://github.com/jxpeng98/CanISend/pull/217) |
| Final PR head | `d8f017fe3b79ee3c7ef2639267790d354b9ad24d` |
| Merge commit | `4d90be6ae2651afe61d4acec9a7d4363cdd0e5ca` |
| Publication authorization | No |
| Current decision | Merged after all protected checks passed |

## Fresh verification

| Scope | Command or owner | Result |
|---|---|---|
| Desktop translation and shell contracts | `pnpm test` | Passed: 85 tests |
| Desktop accessibility and reflow | `pnpm test:accessibility` | Passed: 15 tests |
| Desktop production assets | `pnpm build` | Passed |
| Svelte types | `pnpm check` | Passed: 0 errors, 0 warnings |
| UI design lint | Impeccable detector | Passed: 0 findings |
| Store migration and recovery | `cargo test -p canisend-store --locked` | Passed: 52 unit + 17 contract tests |
| App Workspace recovery | `cargo test -p canisend-app --locked workspace::tests` | Passed: 8 tests |
| Strict affected Rust lint | Store and GUI Clippy with `-D warnings` | Passed |
| Source and release policy | `cargo run -p xtask --locked -- release check` | Passed |
| Packaged visual suite | Design Preview builder | Passed: 18 tests |
| Documented CLI journey | `smoke_documented_quickstart.sh` | Passed: two Packs, data, backup, restore |
| Project/global v4 Skills | `smoke_host_v4.sh` | Passed: Codex/Claude lifecycle and legacy refusal |
| Guarded MCP lifecycle | `smoke_agent_v4_mcp.sh` | Passed: dual-Pack lifecycle, backup, restore, reopen |
| Native App inspection | Isolated Design Preview | Passed: Workspace, both Packs, four Skills, bilingual shell, Settings, notification, diagnostics |
| Protected pull-request CI | Fast CI run `33699412891` | Passed: all six jobs |

## Candidate artifact evidence

| Fact | Value |
|---|---|
| Receipt status | `ready-local-design-review` |
| Receipt SHA-256 | `28889e2ce307ef09a560f3d0b68ccfbd83b7822be600d59808e17c74d8d2fbe9` |
| Integrity manifest SHA-256 | `5393f785d84fb982b4975e8fee62e72b1f32872abe83693b792e7ed76f54addf` |
| Bundled host SHA-256 | `f0509e2885b1fd08c03a7d947ed887b11853e5fdebcc8753430781f7f345d225` |
| Signing | Apple ad hoc |
| Notarized | No |
| Publication allowed | No |

## Migration boundary

- The candidate keeps Workspace format `canisend.workspace/v4` and database schema version `20`.
- There is no schema delta in this candidate, so this evidence does not claim a cross-version
  Beta-to-RC migration qualification.
- The Store regression proves all pending schema versions share one immediate transaction, a
  middle failure restores the original schema/history/version, and a corrected retry succeeds.
- Older incomplete history and newer schemas are rejected before pending schema SQL runs.
- Existing v4 recovery tests prove verified backup and restore to a new path, malformed/legacy
  refusal, occupied-destination preservation, reopen, and integrity checks.

## Warnings and exclusions

- The first preview attempt could not verify the pnpm registry signature inside the restricted
  network sandbox. The approved unrestricted retry verified the package-manager signature and
  completed without changing dependencies or source.
- macOS debug linking emitted the existing compact-unwind size warning during App tests; all owning
  tests and the optimized candidate build passed.
- No tag, GitHub release, notarization, public artifact, downgrade path, or retired v2/v3 desktop
  migration surface was created or claimed.
- The newest remote tag and GitHub Release remain `v1.0.0-beta.1` after the merge.
