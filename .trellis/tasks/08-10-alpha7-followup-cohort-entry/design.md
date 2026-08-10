# Design: Alpha.8 checkpoint and cohort entry

## Boundary

Alpha.8 is a replacement validation checkpoint, not a new feature milestone. It contains the
already-reviewed PR #174 onboarding improvements plus the minimum release-authority repairs needed
to qualify those bytes. Alpha.7 remains immutable and independently reproducible.

## Authority sequence

| Step | Authority | Exit evidence |
|---|---|---|
| Project control | Master Roadmap and GitHub | New `M3-ALPHA8-001` owner and Alpha.8 milestone |
| Product source | Protected PR #174 | Exact merge commit with six required checks |
| Release authority | `xtask`, transition policy, readiness script | Alpha.8 dry-run plan plus focused failures |
| Source transition | Reviewed `prepare-stage --write` PR | Controlled-file commit and green source gate |
| Candidate | Build-once workflow | Exact source, run, artifact, manifest, SBOM, provenance, signatures |
| Public checkpoint | Promotion and independent reverify | Immutable tag, identical bytes, public verification |
| Host evidence | Provider-dogfood record | Alpha.8 Agent/Skills/Pack digests and body-free outcomes |
| User evidence | Issue #70 and Beta readiness | Measured cohort record and reviewed note digest |

## Release-authority repair

The current failure is caused by exact sequential-Alpha replacements assuming that the source is
still in pre-publication wording. The minimum root fix is:

1. Accept the two valid source states for controlled active documentation: canonical development
   wording and canonical published-current wording. Render one canonical Alpha.8 development form.
2. Treat an Alpha iteration of 7 or greater as Beta-eligible only when every readiness field binds
   the exact active tag, source commit, public run/URL, v4 contracts, both Pack digests, provider
   evidence, and measured user evidence. Alpha.6 and lower remain rejected.
3. Make `refresh_beta_readiness.sh` validate the active eligible Alpha recorded by the pending
   ledger instead of a literal Alpha.7 string. It must still download and inspect the exact public
   manifest and require the provider record to match.
4. Keep historical Alpha.7 fixtures and notes unchanged; add Alpha.8-positive and mismatch-negative
   fixtures at the existing `xtask` owner.

No new configuration layer or generic release framework is required.

## Child boundaries

### Alpha.8 checkpoint qualification

Owns every repository, GitHub, CI, candidate, and public-byte action needed to establish the exact
checkpoint. It stops at explicit gates before protected merge, transition write, tag, promotion,
and publication.

### Alpha.8 cohort and Beta evidence

Consumes an immutable Alpha.8 identity. It may record measured body-free results and blocker
links, but cannot change product bytes. A confirmed blocker returns to the checkpoint task or a
new sequential-Alpha task and invalidates only affected downstream evidence.

## Rollback

- Before publication: close or revert the bounded PR/transition and retain Alpha.7 as the public
  checkpoint.
- After Alpha.8 publication: never move or rewrite the tag; publish a later sequential Alpha only
  for a confirmed blocker.
- Cohort evidence is append-only and body-free. Correct an error with a reviewed replacement record
  that names the superseded digest.
