# Qualify the next test build for App and CLI parity

## Goal

Create and merge a focused pull request, then produce an evidence-backed readiness decision for an unpublished CanISend test candidate. The result must prove the requested desktop App, CLI, v4 Skill, Workspace creation/recognition, and Application planning workflows without claiming a new public release.

## Confirmed Facts

- `fix/desktop-sidebar-copy` contains the completed sidebar and bilingual-copy cleanup. Its focused frontend checks, browser review, and `cargo run -p xtask --locked -- release check` passed before this task began.
- That branch was based on a local `main` three Trellis-only commits ahead of the recorded `origin/main`. Those unrelated commits must remain recoverable locally and must not enter this pull request.
- The checked-in product is `1.0.0-beta.1`; feature freeze is active and every nonautomatic post-baseline path requires an exact commit-bound exception.
- ADR-RN-0020 requires App, CLI, MCP, and Skills to use the same application facade and authority. It does not require identical commands: guarded writes may belong to App/MCP while CLI reads the same committed state.
- The canonical v4 Skills are `canisend-workspace`, `canisend-intake`, `canisend-materials`, and `canisend-review-export`. `apps/canisend-desktop/src/lib/views/AgentView.svelte` still maps retired pre-v4 IDs, so the installed v4 Skills fall back to generic presentation.
- Workspace marker discovery is owned by `WorkspacePaths::discover` and already has a Store regression. Desktop create/connect/reopen behavior and CLI explicit-path behavior are covered, but the compiled CLI has no end-to-end nested-directory discovery regression.
- The guarded Plan lifecycle is covered through MCP and Tauri, and CLI exposes `plan show`; a direct MCP-write-to-CLI-read assertion is still missing.
- `docs/contracts/cli-gui-parity-v1.json` is an older composite UI evidence ledger. The current exact authorities are the typed operation registry, compiled adapter inventories, and semantic-parity policy.
- Existing task `M3-EVID-005` owns invited-user evidence on exact public Beta.1. This task must not duplicate or fabricate that evidence.
- The stage policy defines Beta.1 to RC.1 and no Beta.2 iteration. The user selected an unpublished candidate, so this task does not change release stages, tags, or versions.
- GitHub ruleset `Protect main: PR + six Fast CI gates` is active with no bypass actors. It permits merge commits and requires `desktop-ui`, `browser-keyboard-accessibility`, `core-linux`, `core-windows`, `macos-quality`, and `macos-tests` against current `main`.

## Requirements

1. Rebase or transplant only the intended desktop work onto the latest `origin/main`; the pull-request diff must exclude the three unrelated local-main commits.
2. Preserve the sidebar/version/copy cleanup and fix the confirmed canonical v4 Skill titles and concise English/Simplified Chinese descriptions. Do not add aliases for retired Skills.
3. Add the smallest compiled-CLI regression proving a nested directory and an explicit `canisend.toml` marker resolve the same Workspace, while a missing marker fails without creating or mutating a Workspace.
4. Extend the existing guarded Plan lifecycle regression so state committed through MCP is read back through the standalone CLI from the same Workspace.
5. Use the existing operation-registry and semantic-parity checks as the exhaustive App/CLI/MCP inventory. Record requested workflow status, owning test, and evidence in a body-free readiness matrix.
6. Run focused tests first, one final source/release gate on the exact pull-request head, and the required protected PR checks. Missing or skipped checks are not passes.
7. Build an isolated macOS Design Preview App from the clean exact product/test commit, verify its staged bundle, and run the repository's documented quick-start, host/Skill, and Agent v4 MCP smokes against the bundled unified host.
8. Create the pull request with accurate scope, checks, candidate limitations, and freeze-exception evidence. Merge only through the protected merge path after required checks pass; preserve source commit identities required by the freeze exception.
9. Return `Ready` or `Not ready`, with blockers, warnings, residual risks, exact checks, PR/merge state, and the next step.

## Acceptance Criteria

- [ ] The PR diff contains only the intended desktop, focused regression, release-exception, and Trellis records; unrelated local-main commits are absent.
- [ ] Sidebar metadata remains removed, version remains in Settings, and English/Simplified Chinese copy checks pass.
- [ ] The Agent view presents all four canonical v4 Skills accurately and contains no retired Skill ID mapping.
- [ ] A compiled CLI regression proves nested and explicit-marker Workspace recognition plus fail-closed missing-marker behavior.
- [ ] A cross-process regression proves an MCP-confirmed Plan is visible through `canisend plan show` with the same Application and confirmed state.
- [ ] Existing desktop Workspace bootstrap/reopen, v4 Skill installation/status/removal, mixed-Pack, guarded planning, and semantic-parity tests pass.
- [ ] `operations check`, `semantics check`, desktop checks/tests/build, formatting, affected Rust Clippy/tests, and one final `release check` pass on the final PR head.
- [ ] An ad-hoc-signed, non-notarized macOS Design Preview is built from the clean exception-bound product/test commit and its integrity verification plus packaged-host quick-start, Skill, and MCP smokes pass.
- [ ] All six required protected PR checks pass against current `main`, and the PR is merged without rewriting the commit referenced by the feature-freeze exception.
- [ ] The final report labels the artifact as unpublished and does not claim Beta.2, RC.1, cohort, public-download, notarization, or five-target release qualification.

## Out of Scope

- New App or CLI capabilities not required by an accepted v4 contract or a reproduced gap.
- A one-to-one UI control for every CLI/MCP command.
- Replacing the application facade, operation registry, Skill format, test framework, or release automation.
- Rewriting the older composite CLI/GUI ledger when the current v4 authorities pass; any remaining wording drift is reported as a warning.
- Beta.2 support, RC.1 preparation, version changes, tags, public releases, package-index changes, or artifact distribution.
- Inviting users, collecting cohort evidence, or counting maintainer checks as user evidence.
- Repeating the full five-target native release matrix for an App-copy/Skill-presentation change.

## Risks and Deferred Items

- Branch cleanup changes the desktop product commit ID; the freeze-exception entry and prior Trellis journal reference must be regenerated against the final exact commit before the source gate.
- A squash or rebase merge would invalidate the commit-bound freeze exception. The protected merge method must preserve commit IDs.
- The Design Preview proves the supported local macOS App and bundled host, not notarization, public installation, Windows/Linux GUI packaging, or release qualification.
- Later Trellis-only evidence/closeout commits may advance the PR or repository head without changing candidate product bytes; the report must name both the candidate product commit and final merge commit.
- Any newly reproduced data-loss, privacy, consent, Workspace-integrity, or release-integrity defect is a blocker and is not deferred merely to obtain a green candidate.
