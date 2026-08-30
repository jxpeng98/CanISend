# Beta.1 qualification contract research

## Existing owner

- CLI dispatch: `xtask/src/main.rs` routes `release record-beta-qualification` to
  `record_beta_qualification`.
- Writer: `record_beta_qualification` requires a clean worktree only for `--write`, renders the
  plan, and writes only `release/qualification-ledger.json`.
- Asset verifier: `render_beta_qualification` calls the existing complete `verify_release`, reads
  source and manifest digest from verified assets, then delegates to `beta_qualified_ledger`.
- Operations guide: `docs/release/qualification-ledger.md` requires independently downloaded
  public assets and separately retained GitHub attestation results.

## Confirmed drift

- `initial_alpha_qualification_ledger` emits `beta: {"status":"pending"}`.
- Alpha-to-Beta `prepare-stage` changes workspace stage, top status, and release-note status but
  preserves the Beta object.
- The checked-in Beta ledger therefore contains the same status-only object.
- `check_release_qualification` treats `status` as the pending authority and accepts this shape.
- `beta_qualified_ledger` alone expects four additional null/empty fields; only its unit fixture
  creates that unused shape.

The smallest root fix aligns the recorder guard and existing fixture to the generated status-only
shape. Pre-expanding the checked-in ledger would create a second manual transition and bypass the
recorder's one-file authority.

## Post-qualification source-gate drift

`check_provider_dogfood` previously required the provider candidate tag to equal the Roadmap's
current public checkpoint at every stage. That is valid during Alpha, but after Beta qualification
the checkpoint becomes Beta.1 while accepted provider evidence remains exact Alpha.10 input to
`canisend.beta-readiness/v2`. The existing `validate_provider_dogfood_readiness_binding` already
owns that tag/source/run relationship. The minimum stage-aware check keeps current-public equality
during Alpha and uses the readiness binding after Alpha; it does not falsify or repeat host evidence.

## Exact external evidence

- Tag: `v1.0.0-beta.1`
- Source: `6e1397b79031cad54e794ccdc9edca2153f23b3e`
- Candidate run: `33281162734`
- Candidate artifact: `9723581536`
- Promotion/public-verification run: `33283530240`
- Manifest SHA-256:
  `2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`
- Public files/attestations: `20` / `20`
- Candidate control merge: `a223d9e6a2cd9e9195f98fdf7a052184f71de7d0`

The next execution must redownload the public assets; no local host path is an authority.
