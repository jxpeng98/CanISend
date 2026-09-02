# Implementation Plan

## 1. Prepare the clean PR branch

- [x] Fetch current `origin/main` and inspect remote divergence before changing branch history.
- [x] Preserve the existing `fix/desktop-sidebar-copy` reference and local `main` commits.
- [x] Create a clean PR branch from current `origin/main`, apply only the intended desktop product diff, and update this task's branch field.
- [x] Commit the converged Trellis planning artifacts on that clean branch before product editing.
- [x] Verify `git log origin/main..HEAD` and `git diff --name-status origin/main...HEAD` exclude the three unrelated local-main commits.

Rollback point: stop before product editing if the intended diff cannot be separated without losing user work.

## 2. Fix the confirmed Skill presentation gap

- [x] Add a failing frontend source-contract assertion for the four canonical v4 Skill IDs and absence of retired IDs.
- [x] Update `AgentView.svelte` to map only `canisend-workspace`, `canisend-intake`, `canisend-materials`, and `canisend-review-export`.
- [x] Rename and tighten the corresponding English/Simplified Chinese copy keys and descriptions so they match Workspace, Intake, Materials, and Review & Export tasks.
- [x] Run:

```console
pnpm --dir apps/canisend-desktop exec vitest run src/lib/i18n.test.ts src/lib/accessibility-contract.test.ts
pnpm --dir apps/canisend-desktop check
```

## 3. Add the two missing parity regressions

- [x] Add one compiled CLI test covering nested cwd discovery, explicit `canisend.toml` selection, and missing-marker failure without mutation.
- [x] Extend `completes_the_guarded_requirement_plan_and_deliverable_lifecycle` so an MCP-confirmed Plan is read by a separate CLI process and remains `confirmed`.
- [x] Run:

```console
cargo test -p canisend-store --locked --test store_contract workspace_init_discovery_status_and_check_are_consistent
cargo test -p canisend-cli --locked --test binary_contract workspace_v4_discovers_parent_and_explicit_marker_without_mutation_on_missing
cargo test -p canisend-cli --locked --test mcp_protocol completes_the_guarded_requirement_plan_and_deliverable_lifecycle
```

## 4. Verify existing App, CLI, Skill, and Plan owners

- [x] Run:

```console
cargo test -p canisend-resources --locked --test manifest agent_v4_skills_cover_the_canonical_tasks_once_without_host_drift
cargo test -p canisend-resources --locked --test manifest agent_skills_install_is_idempotent_upgradeable_and_edit_safe
cargo test -p canisend-resources --locked --test manifest unsupported_host_resources_and_wrong_versions_fail_before_mutation
cargo test -p canisend-gui --locked shared_registry_commands_create_and_reopen_neutral_workspace_v4
cargo test -p canisend-gui --locked desktop_bootstrap_installs_selected_v4_hosts_and_records_only_basic_boundaries
cargo test -p canisend-gui --locked pack_driven_desktop_commands_preserve_full_semantic_lifecycle_and_failures
cargo run -p xtask --locked -- operations check
cargo run -p xtask --locked -- semantics check
```

- [x] Record every requested workflow, source owner, test owner, fresh result, and any warning/blocker in `research/readiness-matrix.md`.

## 5. Commit the bounded product/test change and freeze exception

- [x] Review the complete product/test diff and commit it as one auditable Conventional Commit.
- [x] Resolve its full commit ID, regenerate the prior Trellis journal reference, and add one sorted feature-freeze exception covering every changed nonautomatic path.
- [x] Commit the exception separately so it binds the unchanged product/test commit identity.
- [x] Commit current Trellis progress separately so the candidate build starts from a clean worktree; later evidence-only commits must record, not obscure, the candidate product commit.
- [ ] Ensure the eventual merge method preserves those commit IDs; do not squash or rebase-merge.

Rollback point: if the exception cannot be exact or the branch requires identity rewriting, regenerate it before any source gate or push.

## 6. Run final source checks once

- [x] Run:

```console
git diff --check origin/main...HEAD
pnpm --dir apps/canisend-desktop format:check
pnpm --dir apps/canisend-desktop test
cargo fmt --all -- --check
cargo clippy -p canisend-cli -p canisend-gui --all-targets --locked -- -D warnings
cargo run -p xtask --locked -- release check
```

- [x] Re-read the final diff and `git status`; no unrelated, generated, secret, private-body, or local-path content may remain.

## 7. Build and smoke the unpublished candidate

- [x] From a clean exact commit, run the existing Design Preview builder without skipping its UI checks:

```console
./scripts/build_macos_design_preview.sh
```

- [x] Verify the receipt reports the exact commit, `source.dirty == false`, ad-hoc signing, no notarization, and `publication_allowed == false`.
- [x] Run against `CanISend Design Preview.app/Contents/MacOS/canisend-gui` using fresh temporary directories:

```console
./scripts/smoke_documented_quickstart.sh BUNDLED_HOST NEW_TEMP_DIRECTORY
./scripts/smoke_host_v4.sh BUNDLED_HOST NEW_TEMP_DIRECTORY
./scripts/smoke_agent_v4_mcp.sh BUNDLED_HOST NEW_TEMP_DIRECTORY
```

- [x] Launch the isolated App and inspect Workspace create/connect/reopen, both Packs, canonical Skill labels, Plan state, Settings version, and English/Simplified Chinese layout. Retain only body-free paths/digests/results.

## 8. Create, verify, and merge the PR

- [ ] Push the clean branch and create a PR describing scope, known fixes, exact checks, freeze exception, candidate boundary, and rollback.
- [ ] Wait for `desktop-ui`, `browser-keyboard-accessibility`, `core-linux`, `core-windows`, `macos-quality`, and `macos-tests`; investigate failures at the owning layer, then rerun only invalidated evidence.
- [ ] Merge through the protected commit-preserving method only when local, candidate, and required CI gates are green.
- [ ] Confirm the merged tree contains the exact exception-bound product/test commit and no public release/tag/workflow was created.

## 9. Finish the task

- [ ] Update `research/readiness-matrix.md` and the final task evidence with PR URL, candidate product commit, merge commit, checks, candidate receipt/digests, blockers/warnings, and `Ready` or `Not ready`.
- [ ] Run `trellis-check`, update any durable spec only if a new reusable invariant was learned, commit Trellis records, archive the task, and record the session.
