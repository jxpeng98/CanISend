# Alpha.9 qualification boundary research

Date: 2026-08-12

## Authority snapshot

- Master Roadmap: `M3-ARCH-001` is Verified; entry for P0 `M3-ALPHA9-001` was explicitly
  authorized on 2026-08-12; `M3-EVID-005` / Issue #70 waits for an exact qualified Alpha.9.
- GitHub projection: Issue #183 is Open under `Alpha.9 — Architecture checkpoint` with release
  ownership and a separate-authorization boundary for every release transition.
- Planning baseline: protected `main` merge `471dc655ea66a60fb41153fa2528a95ff8f8cdf3`.
- Latest public release: immutable `v1.0.0-alpha.8` at
  `35e7c822ea2f469ab726a31b5d08e622f6810c55`.
- Accepted changed-byte dependency: Store-to-IO removal merged at `db966a34` under
  `M3-ARCH-001`; no Alpha.9 tag exists.

## Entry verification

- Remote `main`, local `main`, `origin/main`, and the task branch base all resolve to
  `471dc655ea66a60fb41153fa2528a95ff8f8cdf3`.
- The Alpha.9 milestone contains only Verified Issue #182 and active Issue #183; no additional
  accepted changed-byte P0/P1 blocker was found.
- The dependency authority contains 28 actual and 28 target edges, with zero temporary exception,
  planned removal, or planned addition. Store depends on IO only for tests.
- `xtask dependencies check` passes with 23 reviewed advisory exceptions and two known
  vulnerabilities; all review and expiry dates remain 2026-08-17.
- Alpha signing readiness passes under the community-build policy; unsigned Alpha is permitted.
- `xtask release status --json` reports no blocking drift and the expected source-ahead state over
  immutable Alpha.8.

## Source delta that needs qualification

The relevant post-Alpha.8 product commits move the render seam to Core, compose its IO
implementation in App, remove Store's production IO edge, validate render output, clean partial
export batches, and reject post-render stale export. Existing regressions cover invalid output,
atomic cleanup, stale revision, projection, restore, repair convergence, and dependency shape.

No new Pack schema, Workspace schema, Agent protocol, operation schema, database migration, or
dependency is required for Alpha.9.

## Existing release coverage

- `release.yml` already builds future tags once, then promotes the exact unexpired candidate
  without rebuilding.
- `smoke_release_archive.sh` already invokes documented quickstart, host v4, and Agent v4 MCP
  smokes.
- The documented quickstart initializes one Workspace with both Packs, profile source, backup,
  restore to a new path, repair, and integrity check.
- The Agent v4 MCP smoke reaches guarded dual-Pack export preview/commit and verifies documents,
  revisions, and no submission.
- Native jobs already own Windows render/recovery checks, while assembly owns manifests,
  checksums, SBOM, provenance, and stage-appropriate signing evidence.
- `provider-dogfood.json` already requires exactly the canonical Codex CLI, Claude Code, and
  Claude Desktop scenarios plus the rejected stale-host attempt.

Therefore the minimum sufficient implementation reuses these owners. Alpha.9 needs a new exact
release identity and body-free evidence record, not a second workflow or additional test suite.

## Transition preview

The read-only command below succeeded with `writes_performed: false`:

```text
cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.9
```

It reported 30 controlled files spanning workspace/internal versions and locks, desktop/native
preview versions, release workflow default, CLI/GUI parity, Alpha package contract, active docs,
performance baseline, pending Beta/freeze/feedback identities, and release notes. Before write,
the preview must be rerun from clean current protected source and compared in full.

The final clean-branch preview again reported `alpha.8 → alpha.9`, 30 files, and
`writes_performed: false`. The SHA-256 of its canonical, ordered `.files` array is
`09476c1a6a3a2b8a495bc51f95568cfce82150d93566c8a9c703d93052980bbf`. This binds every reviewed
path plus before/after digest without copying the release tool's authority into a second format.

## Rejected first candidate

The first nonpublishing candidate completed successfully as run `31600099628` from protected
source `8ca0c0a47dcb600d8933168f168543c1388345e8`. Artifact `9143803260`, named
`canisend-v1.0.0-alpha.9-release-assets`, had GitHub artifact digest
`sha256:eee245841bd105df62de41ff3d1ce92c94426fc502d3eb117a430e819bfc4b1a` and expiry
`2026-09-11T13:42:00Z`. All source, native, desktop, packaged-smoke, assembly, and attestation jobs
passed; an independent download verified 15 checksum-listed files and provenance bound 16
subjects to the exact source and release workflow.

Manual artifact inspection rejected the candidate before tagging. Its active release manifest and
SBOM declared legacy Agent, Workspace, resource, and schema v2 metadata while
`release/support-policy.json` and `release/alpha-package-contract.json` require v4. The assembler,
SBOM writer, and verifier all used the same compatibility constants, so the stale metadata could
self-validate. The repair must make every active release projection consume one supported v4
tuple and leave historical v2 evidence unchanged. Candidate run `31600099628` is diagnostic only;
a protected changed-byte repair requires a new source and replacement candidate.

## Qualified replacement candidate

PR #187 passed all protected Fast CI and dependency-policy checks and merged on 2026-08-12 as
replacement source `S2` `4876c5669b7ae48ca053b5e06e0005419d2051f6`. Protected `main` resolved to
the same commit, no `v1.0.0-alpha.9` tag existed, and no other release workflow was active before
the separately authorized replacement dispatch.

Replacement candidate run `C2` `31609344160` completed successfully from `S2`. Its release asset
`A2` is artifact `9147597003`, named `canisend-v1.0.0-alpha.9-release-assets`, with GitHub digest
`sha256:da3c6a5c0aab4cc7f41c2fb1a33fc3c2769232ed74d0333e73f0a33cd5d489d9` and expiry
`2026-09-11T15:23:46Z`. Release identity, signing readiness, source gates, Windows release tests,
all five CLI targets, the supported macOS App package, assembly, packaged smokes, and attestation
passed. Promotion and publication jobs were skipped by candidate mode.

An independent download contained 16 files. `xtask release verify` passed all 15 checksum-listed
files. The manifest binds `S2` and the exact active tuple `canisend.agent/v4`, schema `4.0.0`,
`canisend.agent-host-resources/v4`, and `canisend.workspace/v4`; the SBOM exposes the same Agent,
Workspace, and schema values. The manifest, SBOM, and `SHA256SUMS` digests are respectively
`6d3e5e64dcb6663b5122c70420dc3e16d8c8e3aed8c3bcec35b4ba101537ba5b`,
`2d513827ef284cf214124ad6909e1176502acda4ba59c8b18e659560708673e9`, and
`13e9de63e54fa0a011d58146fd83ae4ee0cdd05cc25b6dba0d2d2ba59202573f`.

GitHub OIDC provenance verified against `jxpeng98/CanISend/.github/workflows/release.yml`, source
digest `S2`, and `refs/heads/main`. Its signed statement contains all 16 subjects; recomputing every
local SHA-256 produced zero mismatches and zero missing files. Community Alpha signing remains
stage-accurate: Apple artifacts are ad-hoc signed, the Windows CLI carries self-signed Authenticode,
Linux relies on checksums and provenance, and no trusted-publisher or notarization claim is made.

## Public Alpha.9 promotion

After separate publication authorization, annotated tag object
`faba63fe5ccd89ae0aaf587d4db12a19e74271c2` was pushed as `v1.0.0-alpha.9` and peeled to exact
replacement source `S2`. Promotion run `P` `31618836210` selected candidate run `C2` and artifact
`A2`, skipped every product build job, passed the six exact-draft native verification jobs, and
published the prerelease at
`https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.9` on
`2026-08-12T16:51:01Z`.

The public release contains 16 assets. Independent download verification passed all 15
checksum-listed files, the release manifest digest
`6d3e5e64dcb6663b5122c70420dc3e16d8c8e3aed8c3bcec35b4ba101537ba5b`, and GitHub OIDC
attestation verification for every public asset. A recursive candidate-to-public comparison found
no byte difference. Promotion evidence artifact `9150636527` has GitHub digest
`sha256:d2dcdcae78b4b5122adc8d24752a5cbff0b5b76a2f7008587a47c5895442bd45`; public verification
artifact `9150717059` has GitHub digest
`sha256:c00f781a011dfb331e12fc84b1f3e2524fe34f795550da24ae810032c8c20586`.

The prior public Alpha.8 tag still peels to
`35e7c822ea2f469ab726a31b5d08e622f6810c55`, and its release remains the unchanged public
prerelease published at `2026-08-10T23:19:31Z`. Public promotion closes only release-plan section
5; external-host dogfood still requires separate synthetic-data and temporary-configuration
consent before provider evidence or Roadmap state changes.

## Exact public Alpha.9 external-host dogfood

On 2026-08-16 the maintainer separately authorized synthetic-metadata provider dogfood and
temporary one-session host configuration. The downloaded public Apple Silicon Alpha.9 CLI matched
the public checksum manifest, reported exact source `S2`, and passed the full guarded dual-Pack MCP
lifecycle. Canonical Requirement preview/cancel scenarios passed on Codex CLI `0.147.0`, Claude
Code `2.1.231`, and Claude Desktop `1.26832.0` with zero mutation and zero submission. Final
Workspace integrity was clean.

Claude Desktop used a new incognito chat and one-session approvals. After the App was closed, its
pre-existing configuration was restored byte-for-byte; the original, backup, and restored SHA-256
was `8281e2dfda423041cc5fd1eb93a6a2dd1fdf9b5dd82a8c0aa305ede83fb32cd4`. The
standard-chat stale-memory attempt remains rejected under Issue #67. The dated body-free evidence
note is `docs/notes/rust-native/2026-08-16-alpha9-exact-public-host-dogfood.md`; Roadmap and Issue
reconciliation still require the protected evidence PR to pass and merge.

## Protected host-evidence merge

PR #190 passed all six Fast CI jobs at exact head
`cabae97797919fbf31024bfc634e3ac56de764ea` with no review thread or changed head, then merged
through protected `main` as `91520f02cfce970afdd9f54636a713871ef9d002` at
`2026-08-17T01:19:02Z`. The exact public, candidate, provider, Pack, host, and protected-source
identities now agree. Final Roadmap/GitHub/Trellis reconciliation must not claim invited-user or
Beta evidence.
