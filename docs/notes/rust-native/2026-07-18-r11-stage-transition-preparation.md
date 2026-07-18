# R11 native stage-transition preparation

**Date:** 2026-07-18

**Roadmap items:** R11.2 through R11.4 transition control

**Status:** Guarded Alpha-to-Beta transition executed

## Problem

The string `0.7.0-alpha.1` appears in both current product state and immutable historical evidence. A global version
replacement would corrupt the public Alpha identity, Beta contract-freeze source, feedback snapshot, and generated
Alpha package candidates. Manual edits could also leave Cargo's exact internal dependencies, lock file, release
notes, and qualification ledger on different stages.

## Decision

`xtask release prepare-stage TAG` now renders a read-only, digest-bound plan by default. It discovers workspace
packages, changes only the explicit current-state surfaces, reports every before/after SHA-256, and preserves the
historical records. `--write` is explicit and refuses a dirty worktree. Forward motion is limited to Alpha→Beta.1,
Beta→RC.1, sequential RC iteration, and RC→Stable on the same release line; RC also requires qualified signed Beta
evidence plus an active feature freeze, while Stable requires the complete qualification ledger. RC.N may advance
only to RC.(N+1), preserving recorded clean-tag evidence; number skipping and Beta iteration are rejected. An
Alpha→Beta write also rejects a readiness audit older than 24 hours or unreasonably dated in the future.

The companion refresh script is also dry-run first. Its GitHub query deliberately retains only public issue
number/state and public Alpha release identity. Any open issue stops automation for maintainer triage; when none are
open, an arbitrary-file `xtask` verifier validates the candidate before a clean-worktree `--write` can replace the
ledger.

Product-version assertions and agent snapshots now resolve the Cargo package version dynamically. They continue to
verify the public JSON field but no longer create unrelated snapshot churn at each release stage.

## Executed Beta transition

The final Alpha→Beta dry run identified exactly ten controlled files: workspace/lock manifests, five manifests with
internal exact dependencies, the `xtask` manifest, qualification ledger, and release-note heading. After the public
issue snapshot was refreshed at `2026-07-18T14:26:32Z` with zero total/open issues, the guarded write advanced those
files to `0.7.0-beta.1`. All four immutable Alpha history surfaces remained byte-identical.

The first Beta test run exposed two test-only assumptions that the current workspace would always be Alpha. Their
assertions now derive the exact current SemVer and release-note heading while retaining fixed historical Alpha,
Beta, and RC parsing coverage. Focused tests, Clippy, property contracts, and the complete source release check pass
in Beta state.

Ordinary CI run `29641475661` passed all eight jobs at exact implementation commit
`f81a262b763e8866f56a543d42d42fbae4683d94`. The run covered dependency policy, complete Rust/property/source gates,
three-platform recovery, and three-platform native render/documentation/archive smokes. Windows also parsed the
release-signing and package-lifecycle PowerShell verifiers before completing its native job.

At `2026-07-18T10:54:05Z`, the new readiness refresher completed its earlier live dry run against `jxpeng98/CanISend`: the
public Alpha release identity matched, the privacy-minimized issue audit returned zero total/open issues, and the
candidate passed `xtask release verify-beta-readiness`. The candidate was intentionally not written because the
24-hour snapshot is reserved for the actual credential-ready transition window.

The freshness gate and refresher then passed all eight jobs in ordinary CI `29641728157` at exact commit
`e064eb6b6457f34542c1a603f48dbfb9e4f60938`, including all three recovery and all three native
render/documentation/archive jobs.

## Boundary and next action

The workspace is now `0.7.0-beta.1` with the ledger at `beta-qualifying`; no tag or release was created by the
transition. The next step is a clean nonpublishing native release matrix using the Community signing policy. Its two
macOS ad-hoc records and one Windows self-signed record must be independently verified before any Beta tag is
created.
