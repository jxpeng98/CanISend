# Qualify the Alpha.9 architecture checkpoint

## Goal

Qualify and publicly reverify one exact `v1.0.0-alpha.9` checkpoint containing the accepted
`M3-ARCH-001` Store-to-IO dependency correction, then bind body-free provider/host evidence to
those public bytes before invited-user evidence resumes.

## Current state

- Roadmap item `M3-ALPHA9-001` entered In progress after explicit authorization on 2026-08-12;
  GitHub Issue #183 is its public projection under the `Alpha.9 — Architecture checkpoint`
  milestone.
- Protected `main` at planning time is `471dc655ea66a60fb41153fa2528a95ff8f8cdf3`; this is a
  planning baseline, not the eventual release source.
- `v1.0.0-alpha.8` remains the immutable latest public checkpoint. No Alpha.9 tag exists.
- `M3-ARCH-001` is Verified at protected merge `db966a34`. Its production dependency correction
  has not yet appeared in a qualified public artifact.
- A read-only Alpha.8-to-Alpha.9 transition preview succeeds and identifies 30 controlled files.

## Requirements

1. Confirm the Alpha.9 entry inventory against the Master Roadmap and public Issues. Include only
   `M3-ARCH-001` and any separately accepted changed-byte P0/P1 blocker that exists before source
   freeze; do not add feature work to the release task.
2. Preview the next-sequential Alpha transition from a clean, updated protected source. Apply
   `prepare-stage v1.0.0-alpha.9 --write` only after separate explicit authorization, and require
   the written file set and digests to match the reviewed plan.
3. Merge the controlled transition through protected review and freeze its exact merge commit as
   the sole Alpha.9 candidate source. Any later product-byte change invalidates the candidate and
   requires a new reviewed source identity.
4. Use the existing native-release workflow to build the future Alpha.9 tag once. The candidate
   must bind five CLI targets, the supported macOS App packages, manifest, checksums, SBOM,
   provenance, qualification records, and stage-appropriate signing evidence to one source.
5. Use existing source and packaged-binary checks to prove the changed architecture boundary:
   the target dependency graph has no Store-to-IO exception, and both Packs, rendering/export,
   backup/restore, repair, stale revision, and failure-atomicity paths pass at their owning layer.
6. After candidate qualification and separate release authorization, create an annotated Alpha.9
   tag at the frozen source and promote the same unexpired candidate without recompilation.
   Independently download and verify every public asset and attestation.
7. With separately confirmed synthetic-data consent, exercise the exact public CLI through Codex
   CLI, Claude Code, Claude Desktop, and the bounded MCP path. Retain only canonical preview/cancel
   outcomes, versions, identities, digests, and body-free state; never retain content or secrets.
8. Update the canonical provider record and dated evidence note to bind the Alpha.9 source,
   candidate, public release, Agent/Workspace v4 contracts, Skills/resources, and both Pack
   digests. Reconcile the Roadmap and Issue only after all exact-byte evidence passes.

## Acceptance Criteria

- [ ] The reviewed Alpha.9 transition contains only its controlled files and is merged through
      protected CI from an explicitly authorized write.
- [ ] One exact protected source commit binds the candidate run, artifact ID/name, annotated tag,
      promotion run, public release, manifest, checksums, SBOM, provenance, and signing evidence.
- [ ] All five CLI targets and supported macOS App packages pass the existing exact-package and
      public-download verification gates without rebuilding between candidate and promotion.
- [ ] Locked dependency metadata proves the accepted graph with zero temporary Store-to-IO
      exception, while existing exact-package checks cover both Packs and affected render/recovery
      paths.
- [ ] Codex CLI, Claude Code, and Claude Desktop canonical preview/cancel scenarios pass against
      the exact public Alpha.9 CLI without mutation or submission; the bounded MCP lifecycle also
      passes.
- [ ] `release/provider-dogfood.json` and its dated note contain only body-free synthetic evidence
      and pass the repository release validator.
- [ ] Alpha.8 history and tag remain unchanged, and any failed post-publication Alpha.9 blocker is
      resolved only by a later sequential Alpha.
- [ ] Roadmap item `M3-ALPHA9-001` and Issue #183 are marked Verified only after public and host
      evidence agree; Issue #70 remains separate invited-user work on the exact Alpha.9 bytes.

## Constraints

- Task creation and implementation-start approval do not authorize release transitions. Stop for
  explicit authorization before `--write`, protected merge, candidate dispatch, tag push,
  promotion/publication, and external-host dogfood.
- Use Tier 3 only for the exact candidate. Do not reproduce the five-target matrix locally or add
  a parallel release workflow, evidence schema, helper, dependency, or test framework.
- Treat Alpha signing as integrity evidence, not a trusted publisher identity. Do not claim Apple
  notarization, Developer ID, public timestamping, or SmartScreen reputation.
- Do not use real applicant content, provider tokens, transcripts, credentials, private paths, or
  invited-user evidence in this task.

## References

- Parent task `08-10-1-0-roadmap-trellis-control`
- Roadmap item `M3-ALPHA9-001`
- GitHub Issue #183
- Prior task `08-10-alpha8-checkpoint-qualification`
- `research/2026-08-12-alpha9-qualification-boundary.md`

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
