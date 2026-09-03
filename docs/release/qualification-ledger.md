# Native Release Qualification Ledger

[`release/qualification-ledger.json`](../../release/qualification-ledger.json) is the machine-readable progress
authority between the published Alpha and Stable. It records evidence references, never certificates, private keys,
tokens, application data, or copied private issue content.

## Stage transitions

| Workspace stage | Required ledger status | Meaning |
|---|---|---|
| Alpha | `pre-beta` | Controls may be prepared, but signed Beta evidence is absent. |
| Beta | `beta-qualifying` | Signing and native/package-channel qualification may run. |
| RC | `rc-qualifying` | Feature freeze is active and clean-tag matrices are being collected. |
| Stable | `qualified` | Every mandatory evidence class below is complete. |

Changing the Cargo workspace version without changing the ledger to the corresponding state makes
`xtask release check` fail. A prerelease ledger cannot set `stable_authorized: true`.

The `beta` object is always the active/latest Beta slot. Before the first sequential Beta it holds
the qualified Beta.1 record. Preparing Beta.N+1 appends that exact record to ordered,
append-only `beta_history` and resets `beta` to canonical pending state. An absent history is
equivalent to an empty history only before that first iteration. RC, Stable, upgrade, and
package-manager qualification consume only the active qualified `beta`; they never fall back to a
historical record.

## Derived current status

Inspect the current source, latest reachable release tag, qualification stage, support surface,
contract counts, and evidence drift without creating another status authority:

```console
cargo run -p xtask --locked -- release status --json
```

The output declares `authoritative: false` and names every source from which it was derived. It
fails closed when Cargo, the public checkpoint, qualification ledger, package contract, operation
contracts, or platform target sets contradict one another. Expected work between releases remains
visible under `drift.items`; stale Beta-readiness, contract-freeze, or feedback checkpoints set
`blocks_stage_transition: true` without rewriting their historical evidence.

## Recording qualified Beta evidence

After the exact signed Beta assets have passed the nonpublishing matrix, been independently inspected, published by
the authorized tag, downloaded again, and verified with GitHub artifact attestations, preview the ledger update:

```console
cargo run -p xtask --locked -- release record-beta-qualification \
  v1.0.0-beta.1 GITHUB_RUN_ID DOWNLOADED_ASSET_DIRECTORY
```

The command re-verifies `SHA256SUMS`, the complete manifest, five archives, three canonical archive-bound signing
evidence files, contract/trust metadata, sizes, and hashes. It derives the source commit from that verified manifest
and renders the before/after ledger digest. It accepts only the exact current Beta version and canonical pending
Beta state. Beta.1 requires the planned entry freeze; a sequential Beta requires valid nonempty
history and the existing frozen baseline. Tags, source commits, and signed-matrix run IDs must be
sequential and distinct. Nothing changes unless the final `--write` flag is supplied from a clean worktree.

The run ID remains an external reference, so the command cannot prove that the number identifies the inspected
GitHub run. Retain the run URL and public `gh attestation verify` results independently; a locally assembled or
hand-edited directory is not sufficient public qualification evidence.

## Recording clean-tag RC matrices

For each public sequential RC, download and independently verify its exact assets, then preview:

```console
cargo run -p xtask --locked -- release record-rc-qualification \
  v1.0.0-rc.1 GITHUB_RUN_ID DOWNLOADED_ASSET_DIRECTORY
```

This applies the same complete signed-asset verification and clean-worktree `--write` boundary as Beta. The current
workspace version must match the tag, Beta must already be qualified, and the feature freeze must be active. Every
recorded RC tag, manifest source commit, and signed-matrix run ID must be distinct. After recording RC.1, use the
sequential stage tool to prepare RC.2, qualify its different clean tag/source/run, and retain both public
attestation reviews. Stable rejects fewer than two such records.

## Recording the native upgrade matrix

After the exact signed Beta/RC archive pair passes the five-target `native-upgrade-qualification` workflow, download
only its verified five-record evidence bundle and independently inspect the public run plus every release asset's
`gh attestation verify` result. Preview the bounded ledger change:

```console
cargo run -p xtask --locked -- release record-upgrade-qualification \
  v1.0.0-beta.1 v1.0.0-rc.1 DOWNLOADED_EVIDENCE_DIRECTORY
```

The verifier requires one GitHub run, one shared manifest pair, distinct target archives, exact platform mappings,
all lifecycle checks, and canonical body-free fields. The recorder additionally requires that exact Beta to be
qualified, the freeze to be active, and that exact RC tag to already have a successful signed matrix. It changes
nothing without `--write` from a clean worktree and updates only `upgrade_matrix`; it cannot claim package-manager or
five-target documentation/uninstall qualification.

## Recording RC documentation and uninstall evidence

The native release workflow emits a body-free record only after each target's extracted-archive quick-start,
host-agent, isolated install, uninstall, and workspace-retention smoke passes. After one signed RC run is recorded in
`release_candidates`, download that run's complete release assets and five-record evidence artifact, independently
inspect the public run and attestations, then preview:

```console
cargo run -p xtask --locked -- release record-documentation-qualification \
  v1.0.0-rc.1 DOWNLOADED_ASSET_DIRECTORY DOWNLOADED_EVIDENCE_DIRECTORY
```

The command re-verifies the complete signed release, binds every record to its manifest archive digest, and requires
all five evidence run IDs to equal the signed matrix run already recorded for that RC tag. It changes only
`documentation_uninstall`, only with `--write`, and cannot reuse Alpha preparation or a different RC run.

## Recording package-manager qualification

Hosted prequalification deliberately produces only Homebrew arm64/Intel and Scoop records plus a WinGet Sandbox
kit. After the WinGet lifecycle runs in a fresh Sandbox and all four records from the same run pass
`release verify-package-evidence`, independently inspect the run and preview:

```console
cargo run -p xtask --locked -- release record-package-qualification \
  v1.0.0-beta.1 v1.0.0-rc.1 DOWNLOADED_EVIDENCE_DIRECTORY
```

The dry-run-first command requires the ledger's exact qualified Beta, frozen RC state, and a successful recorded
matrix for that RC tag. It changes only `package_managers` with `--write`; three hosted records, mixed candidate
digests, or a WinGet record copied from another run remain insufficient. A successful write retains canonical
Beta/RC tags, run ID, and exact four-record count so Stable channel assets cannot rely on prose-only evidence.

## Recording the final RC release-notes review

After the final RC matrix is recorded, download that release's complete verified asset directory and conduct the
human review of the public issue state, release manifest, archive set, limitations, package-channel state, release
notes, and rollback guidance. Then preview the bounded evidence record with the reviewing maintainer's public GitHub
login:

```console
cargo run -p xtask --locked -- release record-release-notes-qualification \
  v1.0.0-rc.2 DOWNLOADED_ASSET_DIRECTORY REVIEWER
```

The command re-verifies the full release, requires the checked-in and published `RELEASE_NOTES.md` bytes to match,
binds the manifest source and run to the latest recorded RC, and records hashes for the stage-neutral note body and
rollback guide. It is dry-run-only without `--write`, writes only the qualification ledger from a clean worktree,
and rejects anonymous, stale, earlier-RC, invented, or noncanonical review evidence. Preparing another sequential RC
resets the earlier review because that evidence no longer describes the final candidate.

## Stable evidence requirements

Stable requires all of these in the committed ledger:

1. A frozen feature baseline commit. After that baseline, only release-blocker fixes, release evidence, and
   documentation changes are allowed.
2. The latest active qualified Beta tag/source/run with archive-bound signing evidence for macOS
   arm64, macOS Intel, and Windows x86_64; historical Betas cannot substitute for it.
3. At least two successful RC matrices with different clean tags, source commits, and GitHub Actions run IDs.
4. Passed Beta-to-RC workspace upgrade, old-binary rejection or same-schema behavior, and restore-to-new-path
   evidence.
5. Passed five-target documentation and uninstall evidence with an exact native matrix run ID.
6. Passed Homebrew Cask, Scoop, and WinGet validation plus install/upgrade/uninstall evidence.
7. A canonical, explicit maintainer review of the latest recorded RC's published release notes and rollback
   guidance, followed by final Stable release notes and explicit Stable authorization.

The ledger is necessary but not sufficient: referenced GitHub runs, public releases, checksums, attestations,
community platform-signing evidence, native signature results, and package-manager validations must still be
independently inspected. An invented run ID or status string is not qualification evidence.

## Beta boundary

Public `v1.0.0-beta.1` remains immutable and qualified while private Beta.2 source preparation is
reviewed. The policy change alone does not change the workspace version, create a tag, dispatch a
workflow, publish a release, or qualify Beta.2. Preparing Beta.2 preserves Beta.1 byte-for-byte in
`beta_history`, keeps the feature freeze active, and makes only the new active Beta slot pending.

Only a publicly qualified and independently reverified dual-Pack Alpha iteration of 7 or greater
may be refreshed into the active readiness record. `canisend.beta-readiness/v2` binds the exact tag,
source commit, successful release run, public URL, Agent/Workspace/host-resource v4 contracts,
Pack v1, both embedded Pack digests, all four Skill digests, provider-dogfood v2, both required
Codex scenarios, reviewed known limitations, and applicable P0 blocker classes. It records zero
synthetic users, invited users, and completed user flows. User evidence starts on public Beta.1 and
remains required before RC.1; it is not a Beta-entry input. Alpha.4, Alpha.5, and Alpha.6 remain
invalid Beta baselines even though their historical evidence is valid for the bytes it describes.

`prepared-local` proves only that the documentation/uninstall control exists locally; it is
deliberately weaker than `passed`, which requires the signed RC-stage matrix named by the Stable
gate. The latest active Beta signing and qualification, clean RC tags, version-pair migration,
native channel lifecycle, final RC notes review, and Stable authorization remain pending after a
new Beta source is prepared.
