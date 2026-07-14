# ADR-018: Add Research Statement As A Guarded Document Executor

**Status:** Accepted for Task 10 of Stage 3

**Date:** 2026-07-14

## Context

ADR-016 exposes every required document and ADR-017 gives Draft and Review document-scoped run identity, but Cover
Letter is still the only available executor. Marking Research Statement available without its own schema, target,
current-basis validator, and Review behavior would let two document kinds share the wrong contract or overwrite one
another's authoritative output.

The first Research Statement slice should prove that the document-scoped runtime is reusable across document kinds.
It must preserve the existing Cover Letter wire behavior and must not imply that every required document, the full
application package, or submission is ready.

## Decision

CanISend adds `research_statement` as the second available submission-document executor. It reuses the shared Claim,
DraftBasis, DraftSection, ReviewFinding, immutable TaskSpec/result, guarded candidate validation, atomic promotion,
cache, recovery, and `(stage, document_id)` ownership boundaries.

The executor has document-specific contracts:

- route `documents.research_statement` and executor `draft.research_statement`;
- authoritative Draft target `research_statement_draft.json`;
- output schema `canisend.research-statement-draft/v1`;
- host-agent generation strategy `host_agent.research_statement`;
- deterministic Review strategy `deterministic.research_statement_review`;
- authoritative Review target `research_statement_review_findings.json`;
- required semantic sections `research_overview`, `research_contributions`, and `future_agenda`.

Research Statement Draft remains proposed. Every factual claim must resolve to current Evidence under the same support
rules as Cover Letter. Future-intent claims must use a current Criterion or confirmed Brief emphasis. Review remains
independent from generation and deterministically reports missing sections, unsupported claims, structural support
limits, exclusion conflicts, duplicate wording, and unusually long Claim blocks.

Adapter selection is based on the current Required Document Plan mapping from stable `document_id` to
`normalized_kind`. If both available document targets exist, Draft or Review without `--document-id` fails closed as
ambiguous. A single available target retains automatic selection. TaskSpec continues to expose only the stable ID;
private labels, source text, Brief content, Evidence bodies, prompts, and generated prose remain outside control
metadata.

This first Research Statement slice supports `host_agent` Draft only. `configured_provider` remains available for
Cover Letter but is rejected for Research Statement at the document adapter boundary even though both use the shared
logical Draft stage.

## Consequences

- Cover Letter and Research Statement Draft/Review records and authoritative files can coexist safely;
- the derived document execution plan truthfully exposes both executors as available;
- agents can dispatch either document through the same CLI protocol by stable document ID;
- the stage registry owns both document-specific outputs without introducing kind-specific lifecycle stages;
- user dispositions, document readiness, compatibility Markdown/Typst projection, configured-provider generation,
  cross-document consistency review, and package readiness remain Cover-Letter-only or future work.

## Rejected Alternatives

- Reuse `cover_letter_draft.json` or `review_findings.json`: rejected because authoritative ownership, cache, drift,
  and recovery would collide across documents.
- Add `research_draft` and `research_review` stages: rejected because ADR-017 defines kind as an adapter target, not a
  lifecycle stage.
- Mark Research Statement available before structured validation: rejected because a capability must describe an
  executable guarded path rather than an aspirational route.
- Enable the existing configured-provider Cover Letter prompt for Research Statement: rejected because its bounded
  output and strategy are document-specific and cannot truthfully generate a different genre.
- Extend package readiness in the same slice: rejected because reviewed document readiness and cross-document package
  policy require separate user-owned and aggregate contracts.

## Revisit When

Revisit before adding a Research Statement provider prompt, compatibility renderer, user finding dispositions,
document readiness, cross-document Review, or aggregate package gates.
