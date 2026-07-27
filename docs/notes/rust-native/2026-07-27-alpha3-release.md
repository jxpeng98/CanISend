# CanISend 1.0.0-alpha.3 release evidence

CanISend `v1.0.0-alpha.3` was published as a GitHub prerelease on 2026-07-27 from
`cebf59629d615d4b950b5823524922b56bde68ce`.

## Qualification

- Fast CI run `30305217612` passed for the exact release source.
- Non-publishing native candidate run `30305363882` passed the source gate, release-only Windows
  tests, five CLI target builds, and Apple Silicon macOS GUI package qualification.
- Candidate assembly produced the release manifest, CycloneDX SBOM, qualification evidence, and
  `SHA256SUMS`.
- Annotated-tag promotion run `30306912005` located that exact candidate, skipped every product
  build, and published only the previously qualified bytes.
- All six private-draft smoke lanes passed before publication.
- The public checksum, provenance, update-response, and candidate-to-release byte-continuity checks
  passed after publication.

The public release contains 14 assets: five standalone CLI archives, one ad-hoc-signed Apple
Silicon macOS application archive, and eight integrity, qualification, notice, and release
documents.

## Product checkpoint

Alpha.3 records the completed Stage 4 surface: all 35 GUI/CLI operation families are represented,
including schema and resource diagnostics plus bounded catalog export. The local macOS Alpha
baseline measured a 950.169 ms median GUI startup and remained within the committed startup and
package-size budgets.

The public GitHub issue audit at `2026-07-27T21:34:49Z` found no issues. The Beta readiness record
and Agent v2, public schema, and workspace contract freeze candidate now use Alpha.3 as their
immediate public baseline. This does not activate the Beta feature freeze; that remains an explicit
next release transition.
