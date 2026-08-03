# GF5-PARITY-001 — Two-Pack semantic outcome matrix

Date: 2026-08-03

## Outcome

CanISend now source-gates semantic outcomes for both built-in Packs on CLI, Tauri, and MCP. The
machine-readable policy binds the typed operation registry to exact test paths and markers rather
than treating matching command names or response envelopes as proof of equivalent behavior.

The canonical generic lifecycle fixtures create and resume an Application, plan, compose, review,
approve, and export it without submission. They also exercise stale revisions, denied consent,
single-use replay, recovery, wrong Pack calls, and unchanged authority after rejected mutations.
Academic fixtures cover the existing job, Profile, task, workflow, intake, and Agent v2
compatibility families. Surface-specific tests prove the Pack boundary in both directions.

## Enforced inventory

- 8 exact shared operations from `OperationRegistry`;
- 6 exact Pack/surface cases;
- 7 revision-bound operations;
- 5 preview/commit families;
- 5 read families;
- the closed success, stale, replay, wrong-Pack, wrong-context, no-mutation, and recovery outcome
  set; and
- 71 qualified bindings plus 148 explicitly uncovered, typed non-shared bindings.

`xtask semantics check` rejects policy shape drift, a missing source marker, an incomplete shared
operation set, missing success/no-mutation/stale coverage, a missing Pack/surface cell, an
unqualified shared binding, incomplete preview or read families, and an unapproved uncovered
class. `xtask semantics uncovered` emits the remaining bindings as structured JSON for subsequent
planning.

The release source gate now invokes the semantic check after the operation-registry and shared
approval-broker checks.

## Focused verification

- `cargo test -p canisend-cli --locked
  canonical_v3_cli_preserves_full_semantic_lifecycle_and_failures`
- `cargo test -p canisend-gui --locked
  generic_desktop_commands_preserve_full_semantic_lifecycle_and_failures`
- `cargo test -p canisend-gui --locked
  desktop_pack_selection_creates_v3_and_resolves_generic_labels`
- `cargo test -p canisend-gui --locked
  shared_registry_and_job_commands_cover_the_local_ts2_slice`
- `cargo test -p canisend-mcp --locked
  agent_v3_runs_new_resume_review_approval_and_stale_recovery`
- `cargo test -p xtask --locked
  semantic_parity_policy_rejects_missing_markers_outcomes_and_shared_bindings`
- `cargo run -p xtask --locked -- semantics check`

All fixtures use disposable local Workspaces and synthetic content. Wrong-Pack and stale paths
compare Workspace status or authoritative revision before and after rejection. Export assertions
also require that no stale destination exists and that successful rendering reports
`submission_performed: false`.

## Remaining boundary

This closes the M1-OP-003 / GF5-PARITY-001 minimum source scope. The 148 explicitly uncovered
bindings are primarily adapter-only or bounded compatibility leaves and are deliberately visible;
they are not falsely reported as shared parity. MSRV alignment, release/frontend non-bypass,
GF5 user documentation, native qualification, real Codex/Claude sessions, and user validation
remain before a new Alpha or Beta claim.
