# Paused workflow snapshots

The owner paused nonessential Actions on 2026-09-08 during CLI-first delivery.
Only Fast CI and dependency assurance remain enabled as repository workflows;
GitHub's managed Dependency Graph is also retained. Fast CI's desktop and browser
jobs require `CANISEND_ENABLE_DESKTOP_CI=true` (currently false).

## Snapshot

`2026-09-08/` contains byte-identical copies from source commit
`bc1076d`, except `rust-r0-spikes.yml`, recovered from its last existing version at
`83ee371ec784e8f9a18f860a604fdce05311102e`. `SHA256SUMS` records the saved bytes.
The registry publisher is reusable and has no independent trigger; its caller
`release.yml` is disabled. No publishing workflow is enabled by this archive.

Original files remain in `.github/workflows/` because release/source validators
require those paths. GitHub workflow state, not these snapshots, enforces the pause.
This archive is a historical snapshot, not a second maintained source. Do not edit
it when updating the retained workflow definitions. Files outside `.github/workflows/`
are not discovered as Actions workflows.

## Restore

1. Review the retained definition against the snapshot and current toolchains,
   credentials, release policy, and outstanding qualification gates. Do not blindly
   overwrite newer definitions. Run source checks before enabling execution.
2. Enable only the required workflow with `gh workflow enable WORKFLOW.yml`.
   For the removed Rust spike, explicitly restore and review its file first.
3. To resume Fast CI desktop jobs, set the repository variable with
   `gh variable set CANISEND_ENABLE_DESKTOP_CI --body true`. Independently enable
   `desktop-platform-qualification.yml` and `intel-gui-compile.yml` if needed.
4. Enabling a scheduled workflow restores its schedule. Formal publication still
   requires every existing release and artifact gate; the pause does not qualify
   any candidate. The npm trusted publisher remains bound to `release.yml`, whose
   disabling also pauses that publishing route.

Verify the snapshot with `cd .github/workflow-archive/2026-09-08 && shasum -a 256 -c SHA256SUMS`.
