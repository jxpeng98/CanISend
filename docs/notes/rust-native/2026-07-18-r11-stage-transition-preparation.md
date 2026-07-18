# R11 native stage-transition preparation

**Date:** 2026-07-18

**Roadmap items:** R11.2 through R11.4 transition control

**Status:** Dry-run and guarded write implementation complete; Alpha remains current

## Problem

The string `0.7.0-alpha.1` appears in both current product state and immutable historical evidence. A global version
replacement would corrupt the public Alpha identity, Beta contract-freeze source, feedback snapshot, and generated
Alpha package candidates. Manual edits could also leave Cargo's exact internal dependencies, lock file, release
notes, and qualification ledger on different stages.

## Decision

`xtask release prepare-stage TAG` now renders a read-only, digest-bound plan by default. It discovers workspace
packages, changes only the explicit current-state surfaces, reports every before/after SHA-256, and preserves the
historical records. `--write` is explicit and refuses a dirty worktree. Forward motion is limited to Alpha→Beta.1,
Beta→RC.1, and RC→Stable on the same release line; RC also requires qualified signed Beta evidence plus an active
feature freeze, while Stable requires the complete qualification ledger.

Product-version assertions and agent snapshots now resolve the Cargo package version dynamically. They continue to
verify the public JSON field but no longer create unrelated snapshot churn at each release stage.

## Current preview

The Alpha→Beta dry run succeeds and identifies exactly ten changed files: workspace/lock manifests, five manifests
with internal exact dependencies, the `xtask` manifest, qualification ledger, and release-note heading. It performs
no writes. The policy and temporary-repository tests prove stage skipping and Beta.2 are rejected, dry-run is
nonmutating, controlled files reach Beta state, and all four Alpha history surfaces remain byte-identical.

## Boundary and next action

The workspace deliberately remains `0.7.0-alpha.1`. The tool does not create tags, start workflows, publish releases,
or manufacture signing evidence. Before using `--write`, refresh `release/beta-readiness.json`, provision the named
Apple and Azure signing configuration from the signing runbook, review the dry-run file set, and pass the Alpha
source gate from a clean worktree.
