# Private Beta.2 candidate evidence — 2026-09-04

## Machine and remote state

- Protected `main`: `4f9af8a762be8c10f56fc4eae33272a46da39f1e`.
- Fast CI run `33760915462` passed on that exact commit.
- Source/package identity: `1.0.0-beta.2` / `v1.0.0-beta.2`.
- Public checkpoint: qualified `v1.0.0-beta.1` from
  `6e1397b79031cad54e794ccdc9edca2153f23b3e`, candidate run `33281162734`.
- Ledger: active Beta pending, one exact qualified Beta.1 history record, feature freeze frozen at
  `acf25dc483643ca9be0210320775708da116b715`, no RC, Stable unauthorized.
- Remote inspection found no Beta.2 tag, Release, or native-release run.

## Existing candidate path

`.github/workflows/release.yml` candidate mode is selected by manual dispatch with
`promote_existing_tag=false`. It fails if the future tag already exists, binds the run to
`GITHUB_SHA`, and runs source gates, dependency assurance, Windows tests, five-target CLI builds,
macOS App packaging/smokes, signing checks, manifest/checksum/SBOM assembly, and provenance
attestation. It uploads `canisend-v1.0.0-beta.2-release-assets` for 30 days. All tag, draft,
promotion, publication, and public-reverification jobs require promote mode and are skipped.

Independent review can reuse the Beta.1 candidate pattern and the existing
`release verify-candidate`, `release verify`, and `gh attestation verify` paths. No new workflow or
verification utility is needed.

## Qualification boundary

`RELEASE.md` requires exact signed Beta assets to be published by an authorized annotated tag,
downloaded again, and independently attestation-verified before `record-beta-qualification` can
write the ledger. Therefore a private candidate can pass Tier 3 but cannot truthfully become a
qualified Beta record.

## Reproduced release-truth drift

- `RELEASE.md` says checked-in Beta.2 matches public Beta.1 source
  `6e1397b79031cad54e794ccdc9edca2153f23b3e`.
- The Roadmap says Beta.2 remains the qualified public checkpoint even though its own next sentence
  and machine facts say no Beta.2 tag, Release, package, or qualification exists.
- The Roadmap still marks `M4-BETA2-001` and `M4-BETA2-002` incomplete after protected PRs #219 and
  #220 merged and the Trellis task was archived by #221.
- `check_active_release_truth_for_version` checks individual version/public markers but does not
  reject their false conflation. A focused shared guard is the smallest durable fix.

## Relevant authorities

- `.trellis/spec/guides/project-control.md`
- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md`
- `RELEASE.md`
- `release/qualification-ledger.json`
- `release/feature-freeze-exceptions.json`
- `.github/workflows/release.yml`
- `.trellis/tasks/archive/2026-08/08-30-beta1-candidate/`
