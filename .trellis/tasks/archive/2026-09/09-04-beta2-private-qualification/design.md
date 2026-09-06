# Design: private Beta.2 native candidate

## Boundary

This task has one outcome: an independently verified, nonpublishing Beta.2 native candidate. It
uses the existing release workflow and verifiers. It does not add product behavior or cross the
tag/publication/qualification boundary.

## Entry truth repair

`check_active_release_truth_for_version` already owns consistency among source version, public
checkpoint, Roadmap, README, and release guide. Add the smallest source-ahead guard there and a
fixture proving both correct separation and rejection of the current false claim.

Use durable prose:

- checked-in source is stated without claiming it shares the public source commit;
- the Roadmap names Beta.2 as the active private source checkpoint and Beta.1 as the qualified
  public checkpoint; and
- completed Beta.2 source-readiness gates are checked while native candidate evidence stays
  pending.

Because `RELEASE.md`, `xtask/src/main.rs`, and both changed `.trellis/spec/` files are nonautomatic
post-freeze paths, the entry PR uses the existing two-commit exception protocol. Roadmap
documentation and `.trellis/tasks/` remain automatic classes.

## Identity chain

~~~text
protected entry merge S
  -> native-release candidate run C (future tag v1.0.0-beta.2)
  -> complete release-assets artifact A
  -> fresh temporary download D
  -> existing verifiers + GitHub attestations V
  -> protected body-free evidence merge E
~~~

The task passes only when `S/C/A/D/V` agree. There is deliberately no annotated tag, promotion
run, GitHub Release, public download, or qualification-ledger write.

## Existing workflow ownership

Candidate mode already owns source gates, dependency assurance, Windows release tests, five CLI
archives, Apple Silicon desktop ZIP/DMG, Intel macOS compilation evidence, community signatures,
archive smokes, checksums, SBOM, release manifest, and provenance attestations. Promotion jobs are
conditioned on `mode == promote`, so no wrapper or duplicated matrix is needed.

## Failure and rollback

- Entry PR failure: do not dispatch; fix the bounded truth/validator change.
- Candidate failure: do not tag; diagnose the owning job and preserve its run URL.
- Artifact mismatch: stop and retain body-free mismatch evidence; never edit the artifact.
- Source movement: discard the planned identity and re-evaluate the new protected `main`.
- Evidence PR rollback changes only documentation and task projections; candidate artifacts expire
  normally and confer no release authority.

## Compatibility

None. The candidate packages the already-reviewed Beta.2 source. Workspace v4, Agent v4, the four
Skills, both Packs, CLI/MCP contracts, and public Beta.1 history remain unchanged.
