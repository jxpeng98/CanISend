# R11 package-manager qualification policy

**Date:** 2026-07-18

**Roadmap items:** R11.3 and R11.4 preparation

**Status:** Machine policy and evidence verifier implemented; native lifecycle workflow active next

## Decision

Candidate generation and candidate qualification are separate gates. Syntax checks and deterministic hashes prove
manifest construction, but do not prove that a user can install Beta, upgrade to RC, uninstall the binary, and keep
their workspace. The new policy therefore requires four native records: Homebrew on both supported macOS
architectures, Scoop on Windows, and WinGet validation plus Windows Sandbox behavior.

## Safety and publication boundary

The workflow will use only CanISend's own public signed release assets and synthetic workspaces. It is manual-only,
does not write to Homebrew, Scoop, WinGet, or `winget-pkgs`, and sets `publication_authorized: false`. This is release
qualification for an owned project, not third-party system testing.

## Qualification boundary

The current Alpha candidate remains `candidates-only`. A valid run requires a same-line signed Beta/RC pair, four
candidate-source-bound records, every official validator, exact observed versions, and workspace retention after
uninstall. Until that evidence exists, the qualification ledger and R11.3/R11.4 checkboxes remain unchanged.

`xtask release verify-package-evidence` now rejects the wrong stage pair, cross-line versions, missing or extra
records, mixed run IDs, mixed candidate pairs, unchanged Beta/RC candidate hashes, false/unknown checks, wrong native
environments, observed-version drift, and noncanonical fields. This makes the future workflow evidence reviewable
without allowing a hand-written `passed` assertion to qualify Stable.

The read-only release verifier now derives identity from the supplied tag, so RC source can independently re-verify
both historical Beta assets and current RC assets. `verify-package-candidates` then requires the same release line,
signed non-Alpha manifests, exact candidate manifest hashes, source commits, and all three package archive records.
Build, assembly, signing, and publication commands remain bound to the current workspace version.
