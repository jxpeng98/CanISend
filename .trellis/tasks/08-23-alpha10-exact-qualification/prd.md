# Qualify and publish exact Alpha.10

## Goal

Qualify and publish exact Alpha.10 from the protected headless-capability source without rebuilding
promoted bytes or overstating synthetic evidence.

## Confirmed entry facts

- `M3-DEPS-001` and `M3-HEADLESS-001` are Verified through protected PR #196 at
  `549072185b5a4886a1b67c1217b79a67d237f826`; Roadmap reconciliation is protected through PR #197
  at planning baseline `79937871852904bb6667e93a21c3c95fb2ca8fa0`.
- Fast CI and dependency assurance passed for the headless change. All 23 lock-bound dependency
  exceptions were reviewed on 2026-08-24 and fail closed after 2026-09-07.
- `release status --json` is hard-consistent with no blocking drift. Its only drift is the expected
  source-ahead-of-public-Alpha.9 condition.
- Public `v1.0.0-alpha.9` remains the latest checkpoint and `v1.0.0-alpha.10` does not exist.
- The read-only sequential transition to Alpha.10 succeeds and names exactly 30 controlled files;
  it performs no write.

## Requirements

### Controlled source transition

- Start only after final planning approval. Recheck protected `main`, tag absence, clean worktree,
  and UTC-bound dependency authority immediately before each release action.
- Re-run and review the read-only 30-file transition plan before separately authorized write mode.
  Reject any extra path or digest mismatch.
- Keep planning/task-control and the mechanical version transition auditable, run the Tier 2 source
  gate once on the final branch head, and merge only through protected Fast CI.
- Freeze the resulting protected merge as source `S`. Any later product-byte change requires a new
  protected source and replacement candidate.

### Build-once candidate

- Dispatch the existing `native-release` workflow from exact `S` for future tag
  `v1.0.0-alpha.10`; do not add another workflow or local native matrix.
- Require source gates, Windows release tests, all five standalone CLI archives, the supported
  Apple Silicon App ZIP/DMG, archive smokes, accessibility/lifecycle/integrity checks, SBOM,
  checksums, provenance, and stage-appropriate community-signing evidence to pass.
- Download the complete 30-day candidate artifact and independently run the existing candidate
  verifier and provenance checks. Record `S`, candidate run `C`, artifact `A`, artifact digest,
  manifest, Pack/resource/Skill digests, and executable identities.

### Exact-host and affected-scenario evidence

- Use only extracted candidate bytes for the App-closed Codex CLI, Claude Code, Claude Desktop,
  bounded MCP-client, and affected headless scenarios. Keep the existing packaged smoke as owner of
  the full mixed-Pack workflow; real hosts need only the canonical non-mutating scenarios.
- Obtain explicit synthetic-provider and temporary-host-configuration authorization. Back up and
  restore host configuration byte-for-byte; retain no body, transcript, private path, token,
  credential, or private identifier.
- Reconcile Issue #68 only if its affected-scenario acceptance is actually met. Otherwise retain it
  as open and cross-link the evidence without overstating completion.

### Same-byte publication and reconciliation

- Only after candidate and host qualification, obtain explicit tag/publication authorization and
  create an annotated tag at `S`. The tag workflow must locate `C/A`, verify them again, and compile
  no product byte during promotion.
- Require all six draft native download smokes before publication. Then independently download all
  public assets and verify checksums, manifest, attestations, source digest, executable identity,
  starter resources, Skills, MCP inventory, and candidate/public byte identity.
- Add the dated body-free exact-host note and provider record only after the same candidate bytes
  are public and independently verified.
- Mark `M3-ALPHA10-001`, Issue #194, milestone 10, and Trellis Verified only after all identities
  agree. Rebind Issue #70 to exact public Alpha.10 but keep it open for real invited-user evidence.
- Keep public Alpha.9, its tag, release, records, notes, and artifacts immutable.

## Acceptance Criteria

- [ ] Roadmap, milestone, Issues, Trellis metadata, release notes, and machine release facts agree.
- [ ] The reviewed 30-file transition is the only version change, and exact protected source `S`
      passes the source gate and protected Fast CI.
- [ ] One nonpublishing candidate passes the five CLI-target and supported App package matrices,
      lifecycle/accessibility, integrity, SBOM, provenance, and signing gates owned by workflows.
- [ ] Independently verified candidate bytes pass App-closed Codex CLI, Claude Code, Claude
      Desktop, bounded MCP-client, and affected-scenario synthetic evidence without mutation,
      submission, retained private bodies, or unrecovered host configuration.
- [ ] The annotated tag peels to `S` and promotes the exact qualified candidate without
      recompilation.
- [ ] Independently downloaded public assets match manifests, checksums, provenance, executable
      identity, starter resources, Skill digests, MCP inventory, and headless smoke expectations.
- [ ] The body-free provider note/record binds the same `S/C/A` and public release identity.
- [ ] Authorities are reconciled only after public-byte verification; Issue #70 and Beta readiness
      remain open with zero synthetic-user claim.

## Constraints

- Planning approval authorizes task start and read-only checks only. Version write, protected
  merge, candidate dispatch, external-host configuration, and tag/publication remain explicit
  stop gates.
- The dependency policy must still be current in UTC at push and candidate time. A missed
  2026-09-07 review blocks qualification without grace.
- Alpha uses the existing `community-build` trust tier. Do not claim notarization, Developer ID,
  trusted Authenticode, public timestamping, or warning-free installation.
- Do not replace a failed artifact, move a tag, rebuild during promotion, weaken a validator, or
  infer real-user evidence from synthetic dogfood.

## Out of Scope

- Product feature work, legacy compatibility, invited-user testing, Beta.1, RC, Stable, or package
  manager publication.
- New workflows, helpers, evidence schemas, test frameworks, provider integrations, Skills, Packs,
  or direct CLI aliases for MCP mutations.

## Parent Artifacts

- `../08-18-alpha10-release-integration/prd.md`
- `../08-18-alpha10-release-integration/design.md`
- `../08-18-alpha10-release-integration/implement.md`
