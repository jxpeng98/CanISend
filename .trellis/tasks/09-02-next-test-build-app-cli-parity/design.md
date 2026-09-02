# Design: unpublished App and CLI parity candidate

## Approach

Use existing authorities and test seams. Make one narrow product correction for canonical Skill presentation, add two missing cross-surface regressions, and qualify an isolated macOS Design Preview from the exact PR head. No new abstraction, command, schema, version, or release workflow is needed.

This remains one task because the product fix, parity regressions, PR, and preview all contribute to one independently verifiable outcome: an unpublished candidate readiness decision.

## Boundaries

- `canisend-app` remains the single business facade.
- `canisend-store` remains the Workspace marker/discovery owner.
- CLI, Tauri, and MCP remain adapters; none may write `.canisend` directly.
- Canonical Skill IDs remain owned by `canisend-resources`; the desktop only presents returned IDs.
- Guarded planning remains App/MCP write plus CLI read. No unsafe CLI mutation shortcut is introduced.
- `OperationRegistry` plus semantic-parity policy are the current inventory authorities. The older composite UI ledger is not expanded.
- The candidate is an isolated, ad-hoc-signed Design Preview and is never tagged, uploaded, or distributed.

## Data Flows

| Workflow | Flow | Validation owner |
|---|---|---|
| Workspace creation | App/CLI → application facade → Store initialization → `canisend.toml` + SQLite | desktop command tests, CLI binary contract |
| Workspace recognition | CLI cwd or explicit marker → `WorkspacePaths::discover` → v4 open-before-mutation | Store contract plus new compiled-CLI regression |
| Skill setup | embedded canonical resources → facade install/status → CLI/Tauri read model → Agent view | resource manifest tests, host smoke, frontend ID contract |
| Application planning | Tauri/MCP preview → approval → commit → shared SQLite → CLI `plan show` | desktop lifecycle, MCP lifecycle, new CLI readback assertion |
| Surface inventory | compiled Clap/Tauri/MCP leaves → typed registry → semantic fixtures | `xtask operations check`, `xtask semantics check` |
| Test candidate | exact clean product/test commit → existing Design Preview builder → staged `.app` → bundled-host smokes | bundle verifier and repository smoke scripts |

## Planned Changes

1. Move the intended desktop changes to a clean PR branch based on the latest remote main while retaining the original local branch as recovery evidence.
2. Replace retired Skill ID branches in `AgentView.svelte` with the four v4 IDs and align English/Chinese copy keys and descriptions with the actual task groups.
3. Extend the existing static frontend contract so canonical IDs are required and retired IDs are rejected.
4. Add one CLI binary-contract test for nested/marker discovery and fail-closed absence.
5. Add one assertion to the existing MCP lifecycle proving CLI Plan readback after confirmation.
6. Record one exact feature-freeze exception for the final product/test commit and its sorted nonautomatic paths.
7. Fill a Trellis readiness matrix with requested workflows, source owners, fresh checks, and result classification.
8. Build and inspect one local Design Preview, run packaged-host smokes, create the PR, wait for the six ruleset checks, and merge with commit identities preserved.

## Compatibility

- Workspace remains `canisend.workspace/v4`; Agent remains `canisend.agent/v4`.
- No migration, compatibility alias, operation ID, Pack, manifest, or serialized payload changes.
- Retired pre-v4 Skill IDs remain rejected by the existing host-resource boundary rather than being translated in the UI.
- Existing user-modified and unmanaged Skill preservation behavior is unchanged.

## Validation Strategy

1. Reproduce the Skill-label gap from source and make the focused frontend regression fail before the fix.
2. Run the new CLI discovery and Plan readback regressions at their owning binary boundary.
3. Run existing resource, desktop bootstrap, mixed-Pack, guarded Plan, operation-registry, and semantic-parity checks.
4. Run desktop formatting/type/unit/visual/build checks, affected Rust formatting/Clippy/tests, and one final release source gate.
5. Build the isolated App and run quick-start, host/Skill, and MCP smokes against the host inside the staged bundle.
6. Treat protected CI as required PR evidence. Do not substitute local output for a missing protected check.

## Rollback

- Before branch cleanup, preserve the existing branch reference; local `main` retains its three unrelated commits.
- Product rollback is the single product/test commit; the following exception record is removed with it.
- Do not merge if the clean diff, source gate, preview, packaged-host smoke, or required PR check fails.
- If a merged regression is found, revert the product commit and its exact exception through a new protected PR; never rewrite Beta.1 history.
