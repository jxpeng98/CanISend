# Beta.1 qualification ledger evidence

Date: 2026-08-30

## Exact public identity

- Tag: `v1.0.0-beta.1`
- Annotated tag object: `96d386b136bdfddea47abeb354df1413f5c346a7`
- Source commit: `6e1397b79031cad54e794ccdc9edca2153f23b3e`
- Candidate workflow run:
  <https://github.com/jxpeng98/CanISend/actions/runs/33281162734>
- Candidate artifact: `canisend-v1.0.0-beta.1-release-assets` (`9723581536`)
- Promotion/public-verification workflow run:
  <https://github.com/jxpeng98/CanISend/actions/runs/33283530240>
- Public release:
  <https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-beta.1>
- Release-manifest SHA-256:
  `2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`
- `SHA256SUMS` SHA-256:
  `3af34e9ac644ef4dabc550b3af57c3a5dc587bcd34e35457fcc5f8ea3653950a`

The public release remained a non-draft prerelease at the immutable annotated tag. Candidate run
`33281162734` remained successful and artifact `9723581536` remained unexpired and bound to the
exact source.

## Fresh verification

One new temporary download contained all 20 public assets. The existing release verifier checked
all 19 manifest-managed files, target archives, checksums, manifest fields, community-signing
records, SBOM, and release documents. Direct GitHub verification passed for all 20 attestations
against repository `jxpeng98/CanISend`, `.github/workflows/release.yml`, and the exact source
commit. No prior local asset directory was reused or retained as authority.

The existing recorder initially disagreed with the supported stage renderer: generated pending
Beta state contains only `status: pending`, while the recorder expected unused null placeholders.
The owning guard and its existing positive/negative regression were aligned to the generated
canonical shape without a new schema, migration, compatibility branch, command, or workflow.

Qualification also advanced the public checkpoint beyond the exact Alpha.10 Codex dogfood used by
Beta readiness. The source gate now requires current-public provider equality during Alpha and the
existing exact Beta-readiness Alpha binding after Alpha. The Alpha.10 provider record was neither
rewritten as Beta.1 evidence nor rerun.

## Qualification transaction

- Plan schema: `canisend.beta-qualification-plan/v1`
- Signed matrix run: `33281162734`
- Ledger path: `release/qualification-ledger.json`
- Before SHA-256:
  `7a1599bd01b4dfe795c71cd27da36311d5020b5c3f9d19f0fa0052b7c0b71183`
- After SHA-256:
  `7bff07d6e375fe879aae4e3d5f6e65c84c8f2cdcb223508108b818047b1415d0`

The dry-run and clean-worktree write reports matched exactly apart from `mode` and
`writes_performed`. Write mode changed only the ledger. The Beta record now binds the exact tag,
source, candidate run, and the canonical Apple Silicon, Intel macOS, and Windows signing-evidence
targets. Workspace stage/status remain Beta / `beta-qualifying`; feature freeze remains planned
with a null baseline.

## Disposition

`v1.0.0-beta.1` is now the latest publicly qualified checkpoint. This record does not generate or
publish package channels, activate feature freeze, count invited users, authorize RC, authorize
Stable, upgrade community signing to public publisher trust, upload, or submit an application.

This note retains no application body, transcript, prompt, credential, private user content, or
host path.
