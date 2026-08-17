# Design: exact Alpha.9 qualification

## Boundary

This task has one independently verifiable outcome: an exact public Alpha.9 checkpoint plus its
body-free host record. It remains one Trellis task because splitting transition, candidate,
promotion, and evidence would weaken the shared release-identity chain.

No product feature, schema, migration, release workflow, provider-record schema, helper, or test
framework is needed. The existing stage tool, native-release workflow, packaged smokes, public
verifier, and provider validator already own the required behavior.

## Release identity chain

The task carries one immutable tuple:

```text
protected source S
  -> candidate run C / artifact A
  -> annotated tag v1.0.0-alpha.9 at S
  -> promotion run P reusing A
  -> public assets and attestations
  -> provider record and dated note bound to S/C/A/P
```

The planning baseline `471dc655ea66a60fb41153fa2528a95ff8f8cdf3` is not `S`. `S` is the exact
protected merge after the controlled Alpha.9 transition. A product-byte change after `S` requires
a replacement candidate; no tag is moved and no candidate is rebuilt in place.

## Authority gates

Each gate is recorded before action:

1. Planning review authorizes `task.py start` and read-only entry checks.
2. A separate authorization permits `prepare-stage ... --write`.
3. A separate authorization permits the protected transition merge.
4. A separate authorization permits candidate workflow dispatch.
5. After candidate inspection, separate authorization permits annotated tag push and promotion.
6. A separate consent check permits synthetic-data external-host dogfood and temporary host
   configuration. One-session configuration is restored byte-for-byte.

No earlier gate implies a later one. The release remains Planned or In progress until public and
host evidence are reconciled.

## Evidence ownership

| Invariant | Existing owner |
|---|---|
| Sequential version and controlled-file mutation | `xtask release prepare-stage` |
| Source contracts and target dependency graph | final source gate and dependency check |
| Five CLI targets and macOS App packages | candidate native-release matrix |
| Both Packs and guarded Agent v4 lifecycle | packaged archive/MCP smokes |
| Render/export, stale revision, and atomic cleanup | Store/App regressions plus packaged export smoke |
| Backup, restore, repair, and integrity convergence | packaged quickstart and native recovery jobs |
| Manifest, checksums, SBOM, provenance, signatures | candidate assembly and promotion jobs |
| Exact public bytes and attestations | promotion public-download reverify |
| Codex/Claude surface wiring and consent | canonical body-free provider scenarios |

The provider scenarios stay preview/cancel and non-mutating. They do not repeat export or recovery
work that the exact packaged-binary matrix already owns. The dated note cross-links those candidate
jobs so both evidence classes bind the same release identity.

## Execution flow

1. Reconcile the Roadmap/Issue inventory and date-bound release authorities.
2. Review and apply the mechanical Alpha.9 transition on a clean branch.
3. Merge through protected CI and freeze exact source `S`.
4. Build `S` once as a nonpublishing Alpha.9 candidate and inspect every required job/artifact.
5. Tag `S`, promote artifact `A` without recompilation, and independently verify public bytes.
6. Download the public CLI, run canonical synthetic host scenarios, and restore host state.
7. Commit the body-free provider record/note and reconcile Roadmap, Issue, and Trellis state.

## Failure and rollback

- Before publication, close or abandon the candidate and retain Alpha.8 as the public checkpoint.
- A source mismatch, expired candidate, missing target, or integrity mismatch stops promotion.
- A host failure after publication keeps Issue #183 unverified and blocks cohort entry. Alpha.9
  remains immutable; a changed-byte fix requires the next sequential Alpha.
- Temporary host configuration is backed up before testing and restored byte-for-byte afterward.
- No failure permits rewriting Alpha.8, moving a tag, bypassing protected checks, or weakening an
  evidence validator.
