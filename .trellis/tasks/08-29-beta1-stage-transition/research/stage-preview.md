# Beta.1 stage-preview research

## Observed command

`target/debug/xtask release prepare-stage v1.0.0-beta.1` completed without mutation on 2026-08-29.

- Schema: `canisend.stage-transition-plan/v1`
- From: `1.0.0-alpha.10` / `alpha`
- To: `1.0.0-beta.1` / `beta`
- Mode: `dry-run`; `writes_performed: false`
- Preserved history: readiness, contract freeze, feedback snapshot, Alpha candidate directory

## Current 18-file output

1. `Cargo.lock`
2. `Cargo.toml`
3. `apps/canisend-desktop/package.json`
4. `crates/canisend-app/Cargo.toml`
5. `crates/canisend-cli/Cargo.toml`
6. `crates/canisend-core/Cargo.toml`
7. `crates/canisend-desktop/Cargo.toml`
8. `crates/canisend-desktop/tauri.conf.json`
9. `crates/canisend-desktop/tauri.windows.conf.json`
10. `crates/canisend-io/Cargo.toml`
11. `crates/canisend-mcp/Cargo.toml`
12. `crates/canisend-resources/Cargo.toml`
13. `crates/canisend-store/Cargo.toml`
14. `fuzz/Cargo.lock`
15. `fuzz/Cargo.toml`
16. `release/RELEASE_NOTES.md`
17. `release/qualification-ledger.json`
18. `xtask/Cargo.toml`

## Confirmed drift before write

The current preview omits ten current-source projections already updated during sequential Alpha:
native-preview package, desktop fallback, CLI/GUI parity, macOS performance baseline, README,
RELEASE, bug Issue template, release workflow default, known limitations, and Alpha package
contract. The final source checker requires several of these to match `CARGO_PKG_VERSION` and also
requires a hard-coded Roadmap Alpha / `pre-beta` marker. Applying the observed plan unchanged would
therefore make the final source gate fail.

## Freshness boundary

`release/beta-readiness.json` was audited at `2026-08-29T21:32:54Z`. Write mode rejects it after
24 hours or when more than five minutes in the future. Recheck immediately before write; do not
change the clock or weaken the validator.
