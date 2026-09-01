# Integrate Windows fixes and verify multi-platform builds

## Goal

Complete the five reproduced Windows validation fixes on their existing branch, integrate the
result through the protected `main` path without weakening feature-freeze auditing, and then verify
the exact merged source on macOS, through macOS-to-Windows x64 MSVC cross-compilation, and on a
native Windows runner where runtime behavior matters.

## Background and Evidence

- Local `main` is `aaa4e98a8e788fc34d86357efc9749e795e5bd4f`, one documentation-only
  macOS validation commit ahead of `origin/main@0d11c456d726990ff9940f404a6aebad24cf72fc`.
- `origin/fix/windows-desktop-validation@42036258fa07fe25e9b42a358b7e64088cbd502e`
  is based directly on the same `origin/main` and contains one documentation-only Windows
  validation checkpoint. It has no PR or check runs and its Trellis task remains `in_progress`.
- The Windows checkpoint proved the CLI/MCP/core flow, release GUI build and launch, PDF and
  embedded resources, frontend unit/build, and the accessibility suite when Vite was started
  directly.
- It reproduced five gaps: unconditional retired desktop reads, macOS-only PATH copy on Windows,
  Windows-only Clippy warnings, an unbounded malformed `.exe` version probe, and a nonportable
  Playwright server command.
- Feature freeze is active at
  `acf25dc483643ca9be0210320775708da116b715`. Product and release-control paths require exact
  commit-bound exceptions.
- The current freeze verifier also counts a protected GitHub merge commit as a second source
  change even when that merge's tree exactly equals the already-audited PR head. This prevents a
  post-freeze product PR from passing the merge-ref source gate and must be repaired without
  exempting conflict resolutions or new merge content.
- On 2026-09-01 the maintainer selected **complete and verify before merge**. The evidence-only
  Windows checkpoint must not be merged into `main` as a completed product result.

## In Scope

### Branch and history integration

- Continue `fix/windows-desktop-validation`; do not replace it with a new implementation branch.
- Preserve the exact Windows checkpoint and local macOS validation commit in reviewable history.
- Merge histories rather than rebase or cherry-pick those two evidence commits.
- Re-fetch before integration and stop for replanning if protected `main` gains overlapping or
  nonautomatic feature-freeze changes.

### Five Windows blocker repairs

1. Remove active `listJobs`/`showJob` reads from the Svelte shell and project selection from the
   supported Application dossier collection. Keep legacy bridge operations fail closed.
2. Render English and Simplified Chinese PATH guidance from backend `target_os`: Windows registry,
   macOS `.zprofile`, and a neutral fallback.
3. Gate Unix-only imports and shell-profile constants with `#[cfg(unix)]`.
4. Reject obviously malformed Windows executables before version probing and make stdout delivery
   deadline-aware with the Rust standard library while preserving no-downgrade and rollback
   behavior.
5. Start Playwright's existing package-local Vite binary directly.

### Protected feature-freeze integration

- Teach the freeze verifier to avoid duplicating an exception only for a merge commit whose full
  tree exactly equals a non-first parent already traversed and audited.
- Continue validating every source commit and every merge that adds conflict-resolution or other
  new content.
- Add focused positive and negative merge-history regressions and document the rule.
- Commit release-control and Windows product repairs separately, then record their exact sorted
  nonautomatic paths as `release-blocker` exceptions in a following ledger-only commit.

### Verification cadence and merge

- Use macOS as the primary implementation host. During code work, batch related edits and run only
  the smallest existing owning regression for each changed invariant; do not run a workspace/full
  frontend suite or `cargo-xwin` after each fix.
- At the completed macOS implementation boundary, run formatting, affected-package Clippy/Svelte
  checking, the small focused regressions, one Tier 2 source gate, and one focused macOS GUI smoke.
  Broader suites remain deferred to protected Fast CI.
- Obtain targeted native Windows branch evidence only for behavior macOS cannot prove: strict
  Clippy, malformed `.exe` and replacement/restore, stock Playwright startup, current-user PATH
  registry behavior, and rebuilt WebView2 GUI behavior. Native Windows closure is a merge gate.
- Open one protected PR and let its six Fast CI jobs own the full source test boundary. Do not run
  another local copy of those full suites. Merge only the reviewed, up-to-date green branch.
- On exact merged `main`, verify the required merge-commit Fast CI run instead of repeating it
  locally, then run `cargo-xwin` once for the Windows x64 MSVC test graph and release binaries,
  inspect PE32+ x86-64 outputs, record SHA-256 hashes, and dispatch the existing desktop-platform
  qualification workflow for native Windows/Linux package/runtime evidence.
- Record task closure through documentation-only protected history after post-merge evidence is
  complete.

## Out of Scope

- Merging the evidence-only checkpoint before its five product blockers are closed.
- Re-enabling or translating retired Workspace v3 bridge operations.
- A compatibility layer, new runtime dependency, new CI tier, or speculative UI abstraction.
- Promoting the Windows GUI candidate to a public supported target.
- Production signing, notarization, trusted Authenticode, release tagging, publishing, or native
  Windows x64 hardware certification. Existing self-signed qualification remains nonpublishing.
- Accepting Microsoft licence terms, downloading SDK material, installing global tools, or
  creating a replacement Windows task without action-time authorization.
- Repeating five-target release matrices already owned by release workflows.
- Running the full Rust/frontend suites or Windows cross-build after every implementation slice,
  or adding redundant test files when an existing owning regression can express the invariant.

## Acceptance Criteria

- [ ] `42036258fa07fe25e9b42a358b7e64088cbd502e` and
      `aaa4e98a8e788fc34d86357efc9749e795e5bd4f` both remain reachable in the integrated history.
- [ ] A clean Workspace v4 activation, selection, and refresh path contains no active
      `job.list`/`job.show` call, while the bridge still rejects those operations.
- [ ] The active selected-Application projection uses dossier/content-catalog data without
      inventing an empty legacy source list, and affected navigation/application regressions pass.
- [ ] Settings presents accurate Windows, macOS, and neutral PATH guidance in English and
      Simplified Chinese from backend `target_os`.
- [ ] Strict Windows Clippy reports no unused Unix-only CLI-install items.
- [ ] Malformed pre-existing Windows `canisend.exe` input returns within the owned deadline; valid
      version comparison, no-downgrade, replacement, rollback, and output limits remain intact.
- [ ] Stock `pnpm test:accessibility` starts its own Vite server and passes in protected CI and on
      the targeted native Windows check.
- [ ] A content-preserving protected merge is not double-counted by the freeze verifier, while a
      merge containing new content remains rejected without an exact exception.
- [ ] macOS implementation uses focused owning regressions only; affected formatting/type/Clippy
      checks and one `xtask release check` pass once at the completed implementation boundary.
- [ ] Required native Windows branch checks pass on the exact reviewed head before merge.
- [ ] The up-to-date PR and exact merge commit pass the repository-owned Fast CI suites without a
      redundant local full-suite run.
- [ ] On merged `main`, `cargo xwin test --workspace --no-run` and applicable Windows release
      binaries build in one consolidated cross-build pass, or a concrete third-party toolchain
      blocker is recorded without claiming a runtime pass.
- [ ] Cross-built executables are identified as PE32+ x86-64 and recorded with SHA-256 hashes;
      native Windows package/runtime evidence is attributed separately.
- [ ] No unapproved installation, SDK licence acceptance, publication, support-policy expansion,
      credential/private-data inclusion, generated artifact commit, or unrelated change occurs.

## Key Decisions

- **Order:** complete and verify the Windows branch, merge through protected review, then perform
  the requested post-merge multi-platform build verification.
- **Development host and test budget:** macOS owns implementation. Each code batch gets only its
  smallest existing focused regression; full suites run only at the protected integration
  boundary, and `cargo-xwin` runs once on merged `main` rather than after each fix.
- **Legacy boundary:** fix active callers; keep retired bridge commands fail closed.
- **Selection model:** use the existing dossier/content-catalog contracts instead of synthesizing a
  legacy `JobDetailReadModel`.
- **Probe implementation:** Rust standard library only; one Windows executable-prefix preflight and
  one deadline-aware stdout result path.
- **Freeze integrity:** allow only tree-identical, content-preserving merge integration to avoid a
  duplicate exception; do not exempt merge commits generally.
- **Evidence:** `cargo-xwin` proves compilation and artifact architecture, not Windows execution.
  Native Windows evidence remains mandatory for registry, process, Playwright, and WebView2 claims.
- **CI:** reuse Fast CI and desktop-platform qualification as the broad-suite owners; add no
  workflow, matrix, or duplicate local full-suite execution.

## Risks and Deferred Items

- `cargo-xwin` may need Microsoft SDK material. If it is not already cached, execution pauses for
  explicit licence/download authorization.
- No reusable Windows Codex task is currently visible. If the validated Windows checkout/runner
  cannot be reconnected, native closure blocks the merge rather than being replaced by
  cross-compilation.
- If `origin/main` advances with nonautomatic changes, the exact exception order and merge-history
  design must be recalculated before editing or merging.
- Windows 11 Arm x64 emulation, if used later, is not native x64 hardware certification. Hosted
  `windows-2025` evidence is attributed to that runner and current support policy remains unchanged.
