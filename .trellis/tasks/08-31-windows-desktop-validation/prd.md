# Fix Windows desktop validation gaps

## Goal

Publish the reproducible gaps found while validating commit
`0d11c456d726990ff9940f404a6aebad24cf72fc` on Windows as an auditable review checkpoint on a
dedicated remote branch, then continue product changes and runnable verification from macOS.

This task validates the existing Windows CLI and nonpublishing Windows desktop candidate. It does
not promote Windows GUI packaging into the public support policy or infer macOS qualification from
Windows results.

The current Windows session ends after the review-only checkpoint is pushed. It must not edit
product source or claim that the observed defects are fixed. The same branch can then be fetched on
macOS for implementation and macOS-owned verification. Windows-only runtime closure still requires
a later native Windows/CI result; macOS cannot by itself prove registry mutation, `.exe` process
behavior, or a Windows GUI smoke.

## Background

The initial Windows run proved that the release GUI builds and launches, embedded resources and
PDF rendering verify, the CLI/MCP/workspace/backup/host-integration basics work, and the frontend
suite is healthy. It also exposed four product defects and one validation-harness defect:

1. Workspace activation calls retired `job.list` and `job.show` bridge operations even though the
   supported Application dossier projection already carries the selection data used by the shell.
2. Settings describes the macOS `.zprofile` mutation while running on Windows, where the backend
   actually manages `HKCU\\Environment\\Path`.
3. Unix-only PATH helpers are compiled as unused items on Windows, so strict Clippy fails.
4. CLI inspection can hang on a malformed pre-existing Windows `.exe` while probing its version;
   the replacement/restore regression therefore never completes.
5. Playwright recursively invokes `pnpm exec vite`; the stock accessibility command cannot start
   its server in this Windows toolchain, although the same local Vite binary works directly.

The complete baseline evidence is in `research/windows-validation.md`.

## Requirements

### WIN-VAL-1: Preserve exact validation evidence

- Record the source commit, Windows environment, commands, pass/fail counts, manual smoke coverage,
  support-policy boundary, cleanup state, and every reproducible gap.
- Distinguish a locally working candidate from an officially supported or published desktop target.

### WIN-CHECKPOINT-1: Publish a review-only checkpoint

- The first branch commit must contain only Trellis task metadata, planning, review evidence, and a
  macOS handoff; it must not change product source, tests, release metadata, or artifacts.
- Push `fix/windows-desktop-validation` to `origin` so the macOS environment can resume from the
  exact reviewed state.
- Keep the task open after this checkpoint; review publication is not product-fix completion.

### MAC-HANDOFF-1: Make the follow-up executable on macOS

- Record checkout prerequisites, implementation order, macOS commands, expected results, cleanup,
  and Windows-only evidence that remains impossible to prove locally on macOS.
- Product edits and runnable follow-up gates start only after the branch is resumed on macOS.

### WIN-GUI-1: Remove retired read operations from the active desktop shell

- The active Svelte application must not call `job.list` or `job.show`.
- Workspace Application selection must be derived from the supported Application dossier list and
  refreshed through the supported single-dossier operation.
- The bridge must continue to reject retired operations before invoking the native host.
- Opening or creating a clean Workspace v4 must not raise the retired-operation error banner.

### WIN-COPY-1: Render platform-accurate PATH guidance

- Windows must explain that CanISend updates the current user's persistent PATH setting and must
  not mention `.zprofile` or a shell profile.
- macOS must retain explicit `.zprofile` guidance, and an unknown platform must receive neutral
  PATH guidance.
- English and Simplified Chinese copy must remain aligned.

### WIN-RUST-1: Keep platform-specific Rust items platform-specific

- Imports and constants used only by Unix shell-profile handling must be gated to Unix builds.
- Strict workspace Clippy with warnings denied must pass on Windows.

### WIN-PROBE-1: Bound unmanaged CLI version inspection

- Obvious malformed Windows executable bytes must be rejected before process launch.
- The version probe must never wait indefinitely for either child termination or captured stdout;
  timeout/error paths return an unknown version without mutating the candidate installation.
- Existing output-size, version-parsing, no-downgrade, replacement-preservation, and restore
  behavior must remain intact.
- A Windows regression must prove that malformed pre-existing bytes do not hang inspection or the
  replacement/restore lifecycle.

### WIN-A11Y-1: Make the stock accessibility command portable

- Playwright must launch the repository-local Vite executable without recursively resolving a
  second package-manager process.
- `pnpm test:accessibility` must start its own server and pass on Windows without a temporary config.

### WIN-DELIVERY-1: Complete the cross-host branch

- Create `fix/windows-desktop-validation` from the exact current `main` baseline.
- First publish the review-only checkpoint; later include regression tests and bounded fixes from
  the macOS continuation in separate reviewed commits.
- Push the branch to `origin` without changing release metadata or publishing artifacts.

## Acceptance Criteria

### Review checkpoint (current Windows session)

- [x] `research/windows-validation.md` captures the exact baseline and support boundary.
- [x] `research/macos-handoff.md` identifies the exact resume workflow and the native Windows
      evidence that macOS cannot replace.
- [x] The checkpoint diff contains no product source, test, release-metadata, or artifact changes.
- [ ] The review commit exists on `origin/fix/windows-desktop-validation` and the task remains open.

### Product follow-up (macOS continuation)

- [ ] The active desktop shell contains no `listJobs`/`showJob` call, while bridge tests still prove
      that `job.list`/`job.show` fail closed.
- [ ] A freshly created/opened Workspace v4 shows no retired-operation error and current
      Application selection still works through dossier projections.
- [ ] Windows Settings contains no `.zprofile`/shell-profile claim; macOS and neutral variants are
      covered in both languages.
- [ ] The malformed-Windows-executable regression is added, runnable macOS probe/install tests
      terminate and pass, and native Windows execution is reported separately.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] Strict workspace Clippy with warnings denied passes on macOS; the Windows result is supplied
      by native CI or remains explicitly pending.
- [ ] Frontend formatting, Svelte checking, Vitest, production build, and the stock Playwright
      accessibility suite pass.
- [ ] The final source gate (`cargo run -p xtask --locked -- release check` plus the repository's
      fast workspace CI commands) passes at the final branch head.
- [ ] A rebuilt macOS host passes a focused GUI smoke covering workspace activation, Applications,
      Settings/PATH copy, diagnostics, language switching, and cleanup.
- [ ] Windows-only tests/checks are either green in native CI or explicitly remain pending; a macOS
      pass is never reported as complete Windows runtime qualification.
- [ ] The completed follow-up commits exist on `origin/fix/windows-desktop-validation` and the
      worktree has no unintended changes.

## Out of Scope

- Publishing, signing, or promoting Windows/Linux desktop installers.
- Claiming macOS runtime qualification from this Windows run.
- Editing product source or running new product validation during the current Windows checkpoint.
- Resolving unrelated release-status drift or updating stale public release-version prose.
- Re-enabling retired Workspace/job operations or adding a Workspace v3 compatibility layer.
- Broad redesign of legacy-named internal UI state beyond the supported read-path repair.
- Extended fuzzing, advisory/license sweeps, notarization, Authenticode, or clean-tag release
  qualification owned by scheduled/native release gates.

## Authority and Verification Tier

- Product/release truth remains with the accepted ADRs, manifests, release contracts, tags, and
  roadmap named in `.trellis/spec/guides/project-control.md`.
- The current checkpoint is documentation-only and runs documentation/task consistency checks.
  The macOS continuation uses focused owning-layer regressions while editing, then one Tier 2
  source gate at the final head. A native smoke is qualification evidence for its own host only,
  not a publication or cross-platform support claim.
