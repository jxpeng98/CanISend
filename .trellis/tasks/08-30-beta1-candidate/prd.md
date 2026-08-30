# Build and publicly verify Beta.1 candidate

## Goal

Build `v1.0.0-beta.1` once from exact protected source
`6e1397b79031cad54e794ccdc9edca2153f23b3e`, inspect the complete native release artifact,
promote those same bytes through an annotated tag, and independently reverify the public
prerelease without recording Beta qualification or changing product code.

## Background

- The Master Roadmap assigns this outcome to `M4-CANDIDATE-001` / Issue #74, which is Ready.
- Remote `main` and the staged Beta.1 source both resolve to `6e1397b79031cad54e794ccdc9edca2153f23b3e`.
  The source declares `1.0.0-beta.1`, Workspace stage `beta`, and ledger status
  `beta-qualifying` with Beta qualification still pending.
- Dependency assurance run `33280629375` and all six jobs in Fast CI run `33280629465` passed
  on that commit.
- Exact public `v1.0.0-beta.1` now exists at the protected source. Alpha.10 remains immutable.
- `.github/workflows/release.yml` already owns the candidate build, five CLI targets, supported
  macOS App packages, community signing, SBOM, checksums, manifest, provenance, candidate lookup,
  same-byte promotion, draft smokes, publication, and public reverification. No new release
  workflow or product implementation is required.

## Requirements

### Entry and exact identity

- Require remote `main`, the workflow-dispatch ref, candidate manifest source, and future annotated
  tag to resolve to exact commit `6e1397b79031cad54e794ccdc9edca2153f23b3e`.
- Require protected Fast CI and dependency assurance to be successful for that exact commit before
  dispatching the native candidate.
- Fail closed if the future tag or release already exists, the source moves, controlled release
  state disagrees, an exception expires, or any required check is incomplete.

### Build-once candidate

- Dispatch the existing `native-release` workflow on exact `main` for future tag
  `v1.0.0-beta.1` in candidate mode. Do not create or push the tag first.
- Require every candidate job to succeed and retain the unexpired
  `canisend-v1.0.0-beta.1-release-assets` artifact plus exact run and artifact IDs.
- Independently download and verify the complete candidate: checksums, release manifest, five CLI
  archives, supported macOS App ZIP/DMG, SBOM, three canonical native-signing records, and GitHub
  attestations bound to the exact source commit.
- Review the community-signing limitations truthfully; do not claim Apple notarization, Developer
  ID, trusted Authenticode, public timestamping, or warning-free installation.

### Same-byte promotion and public verification

- Only after the candidate succeeds and its complete artifact is reviewed, create one annotated
  `v1.0.0-beta.1` tag at the exact candidate commit and push only that tag.
- Let the tag-triggered workflow locate the successful candidate artifact. Promotion must not
  compile, sign, package, or attest new product bytes.
- Require all draft download smokes, public prerelease publication, public checksums, update
  response, and public attestations to pass.
- Independently download every public asset into a fresh temporary directory and prove candidate
  and public manifest/checksum bytes agree.

### Evidence and project control

- Retain only body-free release identity, run URLs/IDs, artifact ID, tag object/source, manifest
  digest, asset count, verification outcomes, and public release URL. Retain no credentials,
  private application bodies, prompts, or host paths.
- Add one dated evidence note and reconcile the Master Roadmap, Trellis project-control guide,
  parent/current tasks, Issue #74, and milestone state through a protected documentation/control PR.
- Mark `M4-CANDIDATE-001` Verified only after the public bytes pass independent verification and
  the protected reconciliation merges.
- Hand off exact downloaded public assets and run identity to `M4-LEDGER-001` / Issue #75 without
  writing the qualification ledger in this task.

## Acceptance Criteria

- [x] Exact source `6e1397b79031cad54e794ccdc9edca2153f23b3e` has successful protected Fast CI and dependency
      assurance before candidate dispatch.
- [x] One nonpublishing `v1.0.0-beta.1` candidate run succeeds and yields one complete, unexpired
      release-assets artifact for the exact source.
- [x] Independent inspection verifies all candidate checksums, manifest fields, target archives,
      App packages, signing records, SBOM, and attestations without a public-trust overclaim.
- [x] One annotated tag peels to the exact candidate source and promotion reuses the candidate
      artifact without recompilation or replacement.
- [x] All draft smokes and public verification jobs pass; the GitHub release is a non-draft
      prerelease and every public asset re-download verifies.
- [x] Candidate and public `SHA256SUMS` and manifest bytes agree, and retained evidence contains
      only body-free identities and outcomes.
- [ ] A protected control PR records the exact evidence; Issue #74 becomes `state:verified` only
      after merge, while Beta qualification, channel generation, feature freeze, and cohort
      evidence remain pending.

## Technical Notes

- Reuse `.github/workflows/release.yml`; no new workflow, helper, dependency, schema, or test suite.
- Use `.github/workflows/finalize-verified-release.yml` only for its narrowly validated recovery
  case: the tag run failed at publication after every required draft gate succeeded.
- A recoverable infrastructure failure may resume the immutable tag with the existing workflow's
  `promote_existing_tag=true` path only when source, tag, candidate, and artifact identities still
  match. Never replace or rebuild candidate bytes behind the tag.

## Out of Scope

- Product, CLI, App, MCP, Pack, Skill, Workspace, or compatibility changes.
- Repeating Codex or Claude host dogfood already owned by Alpha.10; Claude remains non-blocking.
- `record-beta-qualification`, package-channel candidate generation or publication, feature-freeze
  activation, invited-user evidence, RC work, Stable authorization, notarization, or paid signing.
- Moving, deleting, or rewriting any existing tag, public release, historical note, or artifact.
