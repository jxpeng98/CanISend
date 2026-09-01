# Validate macOS feature completeness before Windows integration

## Goal

Establish whether the current protected `main` provides the previously agreed CanISend 1.0
end-to-end experience on Apple Silicon macOS, using repository-owned checks and an isolated
temporary App. Record concrete gaps without changing product code.

## Background

- The future Windows branch is still being completed remotely. It will be reviewed and merged into
  `main` later, after which Windows integration and macOS-to-Windows cross-build work can resume.
- Current `main` and `origin/main` resolve to
  `0d11c456d726990ff9940f404a6aebad24cf72fc`; Fast CI run `33291606152` passed on that revision.
- The 1.0 product contract is a local-first evidence-bound workflow: Workspace initialization,
  mixed generic and academic Applications, Profile and Evidence management, reviewable workflow
  artifacts, guarded export, backup/restore, and consent-bound Codex/Claude integrations.
- The App must support an ordinary-user local flow, while the CLI, MCP server, Skills, and host
  setup remain usable without the App.
- CanISend does not log in, upload, or submit to third-party services on the user's behalf.
- Beta.1 feature freeze is active. This task is validation and evidence only; any confirmed defect
  requiring a product change must be scoped separately under the release-control policy.

## Requirements

- Validate the exact current `main` revision on Apple Silicon macOS and record the Rust, Node, pnpm,
  and OS/architecture context used.
- Reuse existing repository checks and scripts; do not add another test harness or duplicate a gate
  already owned by Fast CI.
- Cover the native Rust workspace and the user-visible CLI quickstart: version/doctor, Workspace v4,
  mixed generic and academic Applications, Profile import, check, backup, restore, and repair.
- Cover Codex and Claude host lifecycle plus the MCP application flow, including evidence,
  requirements, plans, deliverables, guarded writes, export, and restored-workspace validation.
- Build the existing isolated macOS Design Preview App without publishing it. Verify its frontend,
  native host, staged bundle, ad-hoc signature, seeded mixed-pack Workspace, and build receipt.
- Inspect the temporary App's primary navigation and representative workflow screens on macOS using
  only synthetic/local fixture data. Do not contact a real provider or external application site.
- Validate the macOS GUI-to-CLI lifecycle with the existing repository smoke when the local
  environment permits it, including replacement, rollback, uninstall, and Workspace retention.
- Distinguish implemented behavior, automated evidence, observed GUI behavior, and anything not
  exercised. A passing build alone is not a completeness claim.
- Preserve the worktree outside the Trellis task records. Do not modify product code, accept SDK
  licences, install global tools, sign with release credentials, publish, or mutate protected remote
  state.

## Out of Scope

- Integrating or fixing the unfinished remote Windows branch.
- Running `cargo-xwin`, Windows guest/runtime checks, Windows packaging, or claiming Windows support
  from this macOS validation.
- Release packaging, notarization, Authenticode, publication, or full native release qualification.
- Sending real Codex/Claude prompts or importing private user data.
- Automatically repairing any gap found by validation.

## Acceptance Criteria

- [x] Exact source revision, host architecture, and tool versions are captured; the repository's
      pinned Rust toolchain and supported Node/pnpm invocation are used or any mismatch is explicit.
- [x] The Rust workspace test suite passes, or every failure is reproduced and classified without a
      waiver.
- [x] The documented CLI quickstart passes against a temporary Workspace containing both generic and
      academic Applications.
- [x] Codex/Claude host lifecycle and the MCP v4 smoke pass against isolated temporary homes and
      workspaces.
- [x] The macOS Design Preview script produces a temporary staged App and receipt, with frontend
      checks, Playwright visual tests, native build, bundle verification, and seeded data passing.
- [x] The temporary App launches on macOS and its primary navigation plus representative
      Workspace/Application/Profile/Evidence/Review/Settings surfaces are observed, or the exact UI
      blocker is recorded.
- [x] The existing macOS GUI-to-CLI lifecycle smoke passes, or its exact environment/product blocker
      is recorded.
- [x] A final completeness matrix states pass/fail/not-exercised for each agreed 1.0 capability and
      separates product gaps from environment or release-owned checks.
- [x] No product source, release metadata, global tool installation, protected branch, or remote
      state is changed.
- [x] Windows branch integration and the `cargo-xwin` loop remain explicitly deferred until the
      Windows work is complete and available for review.
