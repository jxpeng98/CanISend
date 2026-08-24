# Headless capability implementation

- [x] Add `--scope project|global` to CLI host setup/status/remove with project default.
- [x] Route the value through existing facade requests and receipts; add the smallest parse/routing
      and missing-home regressions at their owning layers.
- [x] Extend the existing Agent v4 headless smoke for starter resources, isolated project/global
      lifecycle, mixed Packs, guarded failure cases, export, backup/restore, and reopen.
- [x] Update canonical Skills and quick start only where executable behavior changed.
- [x] Run focused formatting, Clippy, CLI/App/resource tests, operation and semantic checks.
- [x] Run `cargo run -p xtask --locked -- release check` once on the combined local PR head.
- [ ] Merge through protected Fast CI and reconcile Issues #193 and #195.

Do not change version metadata, add a plugin layer, or restore legacy formats in this child.
