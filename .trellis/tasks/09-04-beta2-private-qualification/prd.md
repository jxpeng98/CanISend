# Private Beta.2 native candidate verification

## Goal

Correct the post-transition release-truth drift, then build and independently inspect one exact
`v1.0.0-beta.2` native candidate from protected `main` without creating a tag, GitHub Release,
package publication, or Beta.2 qualification record.

## Product value

Release owners get native evidence for the actual Beta.2 bytes across the supported CLI and
desktop package surfaces while the product remains nonpublic. A failed candidate stays disposable
and cannot silently become a release.

## Confirmed facts

- Protected `main` is `4f9af8a762be8c10f56fc4eae33272a46da39f1e`; Fast CI run
  `33760915462` passed on that exact commit.
- Source and package contracts declare `1.0.0-beta.2`. Public `v1.0.0-beta.1` remains qualified;
  the active Beta ledger slot is pending and feature freeze remains active.
- No Beta.2 tag, Release, native candidate run, package publication, or qualification exists.
- Candidate mode in `.github/workflows/release.yml` is already nonpublishing. It builds the five
  CLI targets and supported macOS App artifacts, runs source/native/signing checks, assembles the
  manifest, checksums and SBOM, creates provenance attestations, and retains one release-assets
  artifact. Promotion jobs run only after a separate annotated-tag action.
- Public Beta qualification cannot be completed privately: policy requires authorized tag
  publication, a fresh public download, and independent attestation verification before
  `record-beta-qualification` may write the ledger.
- `RELEASE.md` currently says Beta.2 matches the public Beta.1 source, and the active Roadmap calls
  Beta.2 the qualified public checkpoint. Those lower-authority projections contradict the
  machine source/public split and must be corrected before candidate dispatch.

## Requirements

### R1 — Correct release truth at the owning check

- Make the current release guide distinguish checked-in source from the latest public checkpoint
  without embedding a source SHA that a later stage transition can leave stale.
- Reconcile the Roadmap and Trellis project-control status: `M4-BETA2-001` and
  `M4-BETA2-002` are complete, while `M4-BETA2-003` owns pending Beta.2 native candidate
  verification and the invited cohort remains pending.
- Extend the existing active-release-truth validator with one focused regression that rejects a
  source-ahead document claiming that the source matches or is the qualified public checkpoint.
- Do not introduce a new release workflow, status store, or documentation schema.

### R2 — Preserve the active feature freeze

- Land the truth fix through a protected entry PR. Commit changed release-blocker paths first,
  then record their exact commit and sorted nonautomatic paths in the existing freeze exception
  ledger.
- Run the focused validator test, strict relevant Clippy, the source gate once on the final PR
  head, and required protected CI.
- Dispatch only from the exact merged entry-PR commit after confirming `main` has not moved.

### R3 — Build once without publication

- Reuse `native-release` candidate mode for future tag `v1.0.0-beta.2`, cache epoch `stage4-v1`,
  and `promote_existing_tag=false`.
- Require every candidate job and `assemble-and-attest-release` to succeed for one exact source.
- Do not create or push any tag and do not invoke promotion or release-finalization paths.

### R4 — Independently inspect exact candidate bytes

- Record the workflow run URL/ID and complete release-assets artifact ID before expiry.
- Download the artifact into a fresh temporary directory and use the existing candidate/release
  verifiers plus GitHub attestation verification.
- Verify the contract-selected target set, archive contents, manifest source/tag, checksums, SBOM,
  macOS ZIP/DMG evidence, Apple ad-hoc records, Windows self-signed record, sizes, and body-free
  metadata without overstating public signing trust.

### R5 — Reconcile body-free evidence

- Add one dated note containing only source/tag/run/artifact identities, digests, counts, URLs,
  signing boundaries, and verification outcomes.
- Reconcile the Roadmap, project-control guide, parent task, and this task through a protected
  evidence PR. Keep the active Beta ledger pending.
- Preserve all existing cohort files and public Beta.1 evidence unchanged.

### R6 — Fail closed

- If any source, job, artifact, checksum, manifest, target, signature, provenance, or attestation
  check fails, retain the failure evidence and stop without tagging or publishing.
- Product or workflow fixes require a reproduced release blocker and a fresh candidate run; never
  patch downloaded artifacts or reuse a run built from different source.

## Acceptance criteria

- [x] The entry PR corrects the source/public truth, adds the focused regression, records exact
      freeze paths, and passes the source gate plus protected CI.
- [x] Candidate dispatch is bound to the exact protected entry-PR merge commit, with no existing
      Beta.2 tag or Release.
- [x] One nonpublishing native candidate run succeeds for `v1.0.0-beta.2` and produces the complete
      unexpired release-assets artifact.
- [x] Independent verification passes for checksums, manifest, target archives, App packages,
      signing evidence, SBOM, provenance, and exact source identity.
- [ ] A protected evidence PR records body-free results and the task is archived only after that
      PR merges.
- [x] Public Beta.1 remains unchanged; Beta.2 remains untagged, unpublished, unqualified, and
      absent from external package indexes.

## Out of scope

- Creating or pushing `v1.0.0-beta.2`, promoting a candidate, publishing a GitHub Release or
  package-manager entry, or writing `record-beta-qualification`.
- RC.1 preparation, Stable work, cohort claims, live provider submission, paid signing,
  notarization, or trusted Authenticode.
- New product functionality, another workflow, another verification framework, or synthetic user
  evidence.

## Key decisions

- The prior private-only decision remains binding. This task stops after nonpublishing candidate
  evidence; public Beta.2 qualification requires a later explicit authorization.
- Correct the discovered release-truth defect at the shared validator and current documents before
  consuming native CI time.
- Reuse the existing build-once candidate path and its `stage4-v1` cache epoch.
