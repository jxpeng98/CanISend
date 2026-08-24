# Exact Alpha.10 qualification design

## Boundary

This is one release-identity pipeline, not a feature branch. Existing stage, candidate, packaged
smoke, provider-record, promotion, and verification machinery already owns the outcome; no new
abstraction or workflow is required.

## Identity chain

```text
planning baseline B
  -> protected Alpha.10 source S
  -> candidate run C / complete artifact A
  -> exact candidate host evidence H
  -> annotated tag T at S
  -> promotion run P reusing A
  -> independent public verification V
  -> body-free record and authority reconciliation R
```

`B` is `79937871852904bb6667e93a21c3c95fb2ca8fa0`; it is not the release source. `S` is the exact
protected merge after the controlled Alpha.10 transition. `C/A/H/T/P/V/R` must all bind `S` and
`v1.0.0-alpha.10`. A changed product byte invalidates the chain before publication.

## Existing owners

| Invariant | Owner |
|---|---|
| Sequential 30-file version mutation | `xtask release prepare-stage` |
| Source and dependency consistency | `xtask release check`, Fast CI, dependency assurance |
| Five CLI archives and Apple Silicon App packages | `.github/workflows/release.yml` |
| App-closed mixed-Pack lifecycle, export, backup/restore/reopen | existing release archive and Agent v4 smokes |
| Codex CLI, Claude Code, Claude Desktop, bounded MCP consent | canonical synthetic host scenarios |
| Manifest, checksums, SBOM, provenance, signatures | candidate assembly and verifier |
| Same-byte promotion and native draft smokes | candidate locator and tag-triggered promotion |
| Public checksums, provenance, update identity | published-release workflow plus independent download |
| Public project state | Roadmap, Issues #68/#70/#194, milestone 10, Trellis task |

## Stop gates

1. Final plan approval starts the task and permits read-only preflight only.
2. Separate approval permits `prepare-stage v1.0.0-alpha.10 --write`.
3. Separate approval permits protected metadata merge after Tier 2 and PR checks pass.
4. Separate approval permits the nonpublishing candidate dispatch from exact `S`.
5. Separate approval permits synthetic provider use and temporary host configuration.
6. Exact candidate evidence is presented before separate annotated-tag/publication approval.
7. Public verification and evidence PR must pass before any Verified or cohort-baseline claim.

The candidate dispatch uses the existing workflow with tag `v1.0.0-alpha.10`, a body-free cache
epoch, and `promote_existing_tag=false`. An annotated-tag push is the normal promotion trigger;
`promote_existing_tag=true` is only the repository's bounded recovery path for an already-pushed
immutable tag.

## Host-evidence boundary

The full guarded dual-Pack workflow is not repeated manually because the exact archive smoke owns
it. The external hosts run the smallest canonical preview/cancel scenarios against extracted
candidate bytes, with the App closed, then integrity is rechecked. After promotion, independent
digest equality proves those tested bytes are the published bytes; only then is the provider note
finalized. Synthetic evidence never increments invited-user counts.

## Failure and rollback

- Before tag creation, abandon a failed or expired candidate, fix protected source, and produce a
  new `S/C/A`; never replace files inside `A`.
- Tag/source, artifact, digest, target, provenance, signing, host-restore, or date-policy mismatch
  stops the next gate.
- A promotion failure leaves a draft or failed run for diagnosis. Use the existing finalize path
  only when its immutable-tag and successful-draft-gate preconditions are exactly satisfied.
- A failure after publication leaves Alpha.10 immutable and unverified; changed bytes require the
  next sequential Alpha. Alpha.9 history is never rewritten.
- Host configuration is restored byte-for-byte even on failure. No failure permits weakening
  evidence, consent, privacy, path, or no-submission controls.
