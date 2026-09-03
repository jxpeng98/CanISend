# Current-state evidence

## Authorities

- `release/qualification-ledger.json`: public `v1.0.0-beta.1` is qualified from
  `6e1397b79031cad54e794ccdc9edca2153f23b3e`; feature freeze is active at
  `acf25dc483643ca9be0210320775708da116b715`.
- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md`: `M3-CLI-001`,
  `M3-AGENT-RESOURCES-001`, and `M3-HEADLESS-001` define the supported CLI/Skills/MCP split.
- `docs/contracts/operation-registry-v1.md`: exact compiled adapter inventory is 31 CLI, 129 Tauri,
  and 36 MCP leaves.
- `docs/contracts/agent-v4.md`: ten canonical Agent tasks and the guarded
  `orient -> propose -> preview -> approve -> commit -> verify` sequence.
- `docs/guides/agent-integration.md`: four canonical Skills, Codex/Claude project paths, and the
  persistent MCP setup contract.

## Existing automated coverage

- `crates/canisend-cli/tests/binary_contract.rs` covers version/doctor, stable JSON failures,
  legacy refusal, Workspace initialization/discovery/recovery, host setup/status/remove,
  mixed-Pack Applications, and Profile Source handling.
- `crates/canisend-cli/tests/mcp_protocol.rs` covers protocol negotiation, all 36 tools, guarded
  requirement/Plan/Deliverable/review/export mutations, associations, replay, private-read
  consent, wrong input, and legacy-tool refusal.
- `crates/canisend-resources/tests/manifest.rs` covers canonical task-to-Skill ownership, resource
  integrity, clean installation/update/removal, drift, symlinks, and unsupported versions.
- `scripts/smoke_agent_v4_mcp.sh` covers a packaged-style dual-Pack CLI/MCP lifecycle, guarded
  positive and negative paths, export, Workspace check, backup, restore, and reopen.
- `crates/canisend-store/src/database.rs` contains focused transaction tests for complete pending
  migration-chain rollback and retry.

## Fresh baseline results — 2026-09-03

- `cargo run -p xtask --locked -- operations check`: passed; 31 CLI, 129 Tauri, 36 MCP leaves,
  99 adapter-only, zero compatibility bindings.
- `cargo test --locked -p canisend-cli --test binary_contract --test mcp_protocol`: passed;
  16 tests total.
- `cargo test --locked -p canisend-resources --test manifest`: passed; 16 tests.
- `cargo test --locked -p canisend-app agent_facade_is_typed_body_free_and_exports_verified_host_packs`:
  passed; one focused test.
- `./scripts/smoke_agent_v4_mcp.sh target/debug/canisend <disposable-root>`: passed; guarded
  dual-Pack lifecycle, backup, restore, and reopen.

## Verified gap

The packaged smoke does not currently include project-scoped `host setup` / `host status` or bind
the four installed Skill files inside the same end-to-end fixture. Unit and binary-contract tests
cover these separately. Extending the existing smoke is the smallest useful closure; broader
product changes require a failing reproduction first.

## Release-identity constraint and resolved direction

- ADR-RN-0014 defines `beta.1 -> rc.1 -> sequential RC if required -> stable`.
- `release/stage-transition-policy.json` permits sequential Alpha and RC iterations but contains
  no sequential Beta rule.
- `xtask` validates that exact policy and has a regression requiring `beta.1 -> beta.2` to fail.
- A real Beta.2 therefore needs new append-only qualification and cohort semantics; changing only
  package versions would violate release authority.
- On 2026-09-03 the product owner chose private `v1.0.0-beta.2` source preparation and deferred
  RC.1. The plan adds only exact sequential-Beta policy, append-only Beta history, compatible
  qualification validation, and focused regressions before using the existing transition command.
- Publication, qualification, cohort rebasing, tagging, and workflow dispatch remain separate and
  are not authorized by this planning decision.
