# R11 package-manager qualification policy

**Date:** 2026-07-18

**Roadmap items:** R11.3 and R11.4 preparation

**Status:** Policy, verifier, native prequalification workflow, and Sandbox kit implemented

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

The manual prequalification workflow now executes Homebrew on both release architectures and Scoop on Windows 2025,
after binding checked-in candidates to both complete public signed releases. It also runs official WinGet manifest
validation and packages the exact candidates, run identity, nonpublishing lifecycle script, and guide for a fresh
Windows Sandbox. The hosted workflow intentionally produces only three evidence records; it cannot claim the fourth
until the Sandbox lifecycle result is returned and the four-record verifier passes.

Ordinary CI run `29640846261` passed all eight jobs at exact commit
`9da2507856ab2de955e224d8685ee9c3fbaac933`. In particular, the Windows job parsed the release-signing verifier and
the new Scoop/WinGet lifecycle verifier successfully before running the native build and render gates. This
qualifies workflow/script portability, not the still-missing signed Beta/RC lifecycle evidence.
