# ADR-019: Scope Review Dispositions And Readiness By Document Identity

**Status:** Accepted for Task 11 of Stage 3

**Date:** 2026-07-14

## Context

ADR-018 adds a guarded Research Statement Draft and deterministic Review, but only Cover Letter findings can enter
the user-owned disposition and derived readiness path. Reusing `review_dispositions.yaml` for Research Statement
would let two documents overwrite the same CAS baseline, mutation claims, recovery evidence, and readiness receipts.

The next slice must let an agent or CLI session resume finding decisions for either document without inferring which
private Review is meant. It must preserve existing Cover Letter YAML and compatibility projection behavior and must
not turn per-document readiness into application-package readiness.

## Decision

Review disposition operations become document-scoped through the stable Required Document Plan `document_id`.
`review-dispositions status`, `init`, and `update` accept `--document-id`. When exactly one supported document target
exists it may be resolved automatically; when Cover Letter and Research Statement both exist, omission fails closed
as ambiguous.

Each document kind owns a separate user artifact and mutation-control namespace:

- Cover Letter keeps `review_dispositions.yaml` and artifact identity `review_dispositions`;
- Research Statement uses `research_statement_review_dispositions.yaml` and artifact identity
  `research_statement_review_dispositions`.

Both artifacts use `ReviewDispositionsV1`, guarded revision/hash compare-and-swap, immutable candidate/claim/receipt
records, interrupted-publication recovery, non-waivable blocker rules, and explicit reset against a changed Review.
The contract gains a defaulted `document_kind`; missing values continue to mean `cover_letter`, so existing valid
Cover Letter YAML remains readable. Research Statement artifacts must declare `research_statement` and cannot be
loaded through the Cover Letter artifact identity.

`DocumentReadinessV1` and `derive_document_readiness` support both document kinds. Readiness remains a deterministic,
body-free projection bound to the exact Draft, Review, and disposition hashes. A document becomes `reviewed` only
when every current non-blocker finding is explicitly accepted, no revision is requested, and no blocker remains.

Agent responses expose only selected document identity/kind, artifact paths and hashes, revision, states, counts,
reason codes, consents, and actions. Finding messages, Claim text, Evidence bodies, Brief content, and disposition
bodies remain outside the control response.

## Consequences

- Cover Letter and Research Statement disposition histories, CAS baselines, recovery controls, and readiness cannot
  collide;
- agents reuse the existing review-disposition operations and pass the same stable ID used for Draft and Review;
- legacy sole-Cover-Letter calls and YAML remain readable and automatically resolvable;
- changing one document makes only that document's disposition basis stale;
- Research Statement can reach per-document `reviewed` without changing compatibility rendering or package gates.

## Rejected Alternatives

- Reuse `review_dispositions.yaml`: rejected because two documents would share one user-owned revision and recovery
  namespace.
- Store dispositions inside `review_findings.json`: rejected because deterministic Review output is core-owned while
  finding decisions are explicit user-owned state.
- Infer the target from whichever Review file exists: rejected because behavior would change as another Review is
  generated and could silently select the wrong private document.
- Add a separate Research-specific CLI command tree: rejected because document identity already provides a common
  host-neutral dispatch contract.
- Extend package readiness or Markdown/Typst projection now: rejected because per-document acceptance is not an
  aggregate application-package policy or rendering approval.

## Revisit When

Revisit before allowing more than one document of the same normalized kind, adding aggregate cross-document Review,
or deriving application-package readiness from multiple reviewed documents.
