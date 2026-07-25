# ADR-RN-0014: Start the unified CLI and desktop product at 1.0 Alpha

**Status:** Accepted

**Date:** 2026-07-25

## Context

CanISend has a complete Rust-native CLI and Agent v2 application-preparation pipeline. A macOS-first
desktop adapter now shares the same typed application services and can manage workspaces, intake
jobs, import supplied URLs and local files/PDFs, start workflows, inspect diagnostics, install the
version-matched CLI, and check for public updates.

The existing workspace version is `0.7.0-rc.2`. Its qualification ledger, Beta evidence, two RC
matrices, feedback snapshot, support policy, release notes, and Alpha package candidates describe
the historical `0.7` release line. They cannot truthfully authorize a new release line or a desktop
release unit.

The earlier GUI roadmap provisionally named `0.8` as the next line. The product owner has instead
decided that the next public product line will be `1.0`.

## Decision

- The next public CanISend version will be `1.0.0-alpha.1`.
- CanISend will not publish a `0.8` product line.
- The project will not enter the `1.0` line at RC. `1.0.0-rc.1` is reserved for a feature-frozen
  CLI and GUI release unit that has already passed Alpha/Beta feedback and qualification gates.
- Beginning with `1.0.0-alpha.1`, the macOS GUI and its version-matched CLI are one product release
  unit. The five-target standalone CLI matrix remains supported; macOS is the first claimed GUI
  platform.
- The current `0.7.0-rc.2` version remains in the workspace until a fail-closed release-line
  activation change can archive the current release authority and atomically initialize the new
  `1.0` Alpha state.
- `0.7` qualification evidence remains immutable historical evidence. It may be copied into or
  referenced from a versioned history directory, but its tags, commits, runs, hashes, and measured
  public metadata must not be rewritten to say `1.0`.
- Release-line activation must update the workspace version, exact internal dependency pins,
  lockfiles, active support policy, active qualification ledger, release notes, and stage-aware
  release checks as one reviewed change.
- `1.0.0-alpha.1` may use the approved free macOS ad-hoc integrity signature. It must clearly state
  that it is not Developer ID identity and is not notarized.
- Windows and Linux GUI publication remain deferred. This does not reduce the existing five-target
  standalone CLI matrix.

## Why Alpha rather than RC

The GUI currently has an implemented vertical slice, not full CLI-to-GUI workflow parity. Restore,
repair, workflow transition controls, evidence, discovery, agent-task, document, review, package,
render, and export surfaces remain incomplete. Packaged disposable-user lifecycle, accessibility,
Intel macOS, and final release-integrity evidence also remain open.

Alpha accurately communicates that the native product can be exercised by early users while its
surface, packaging, and interaction details can still change. RC will mean that the intended 1.0
feature set is frozen and only release blockers, evidence, and documentation changes remain.

## Consequences

- Version communication becomes simpler: the next public product is `1.0`, with explicit maturity
  expressed by SemVer prerelease identifiers.
- The `0.7` RC history remains auditable rather than being silently reused as 1.0 evidence.
- A release-line activation tool or equivalent bounded transaction must be implemented before the
  workspace can be changed to `1.0.0-alpha.1`.
- The current update checker can compare `1.0` prereleases without Python or a package manager.
- Alpha publication is blocked until an exact macOS app bundle and the standalone CLI artifacts
  are represented in a machine-checkable release manifest and pass the defined Alpha gate.

## Rejected alternatives

- **Publish `0.8.0-alpha.1`:** rejected by the product-version decision; there will be no public
  0.8 line.
- **Change only `Cargo.toml` to `1.0.0-alpha.1`:** rejected because it would leave the active
  ledger, support policy, notes, and release checks describing a different release.
- **Rewrite all `0.7` evidence as `1.0`:** rejected because public tags, commits, runs, hashes, and
  measured metadata are historical facts.
- **Publish `1.0.0-rc.1` immediately:** rejected because the GUI feature set and native release
  qualification are not frozen.

## Promotion policy

The intended forward sequence is:

```text
1.0.0-alpha.1 → optional later Alpha → 1.0.0-beta.1
→ 1.0.0-rc.1 → sequential RC if required → 1.0.0
```

Promotion is evidence-driven, not calendar-driven. No stage is skipped merely to obtain the `1.0`
label.
