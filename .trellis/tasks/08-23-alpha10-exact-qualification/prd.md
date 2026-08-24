# Qualify and publish exact Alpha.10

## Goal

Qualify and publish exact Alpha.10 from the protected headless-capability source without rebuilding
promoted bytes or overstating synthetic evidence.

## Requirements

- Start only after `M3-HEADLESS-001` is merged and its Tier 2 gate passes.
- Use the existing sequential Alpha transition, protected metadata PR, build-once candidate, native
  matrices, exact-host dogfood, same-byte promotion, and independent public verification.
- Reuse Issue #68 for affected-scenario evidence. Rebind Issue #70 only after an exact public
  Alpha.10 exists; do not claim real-user or Beta evidence.
- Keep public Alpha.9 and its evidence immutable.

## Acceptance Criteria

- [ ] Roadmap, milestone, Issues, Trellis metadata, release notes, and machine release facts agree.
- [ ] Exact Alpha.10 source passes the source gate and protected Fast CI.
- [ ] One nonpublishing candidate passes the five CLI-target and supported App package matrices,
      lifecycle/accessibility, integrity, SBOM, provenance, and signing gates owned by workflows.
- [ ] Candidate bytes pass App-closed Codex CLI, Claude Code, Claude Desktop, bounded MCP-client,
      and affected-scenario synthetic evidence without retaining private bodies.
- [ ] The annotated tag promotes the qualified candidate without recompilation.
- [ ] Independently downloaded public assets match manifests, checksums, provenance, executable
      identity, starter resources, Skill digests, MCP inventory, and headless smoke expectations.
- [ ] Authorities are reconciled only after public-byte verification; cohort/Beta remains open.

## Out of Scope

- Product feature work, legacy compatibility, invited-user testing, Beta.1, RC, Stable, or package
  manager publication.

## Parent Artifacts

- `../08-18-alpha10-release-integration/prd.md`
- `../08-18-alpha10-release-integration/design.md`
- `../08-18-alpha10-release-integration/implement.md`
