# ADR-009: Derive Stable Semantic Identifiers From Source Receipts

**Status:** Accepted

**Date:** 2026-07-11

## Context

`parsed_job.json` currently stores criteria as display text plus a source-text receipt. The matcher returns transient
Python objects keyed by that display text, while profile evidence exposes positional item labels such as `cv-001`.
Those representations are sufficient for one process that immediately renders Markdown, but they cannot safely
carry confirmations, corrections, evidence gaps, or review decisions across reparses and host sessions.

Extending Parsed Job v1 in place would break its strict schema and change the already accepted Parse contract.
Using list positions, source line numbers, or editable display text as identity would also make harmless reordering
or user corrections detach downstream decisions.

## Decision

Stage 2 introduces separate, versioned semantic projections rather than changing Parsed Job v1 in place.

`criteria.json` is a core-owned, regenerable projection of the criteria receipts in `parsed_job.json`. Each criterion
has:

- a stable `criterion_<digest>` identifier derived from the job identity, importance, normalized source receipt, and
  normalized parser interpretation, but never user-corrected text;
- essential or desirable importance;
- display text and the original source-text receipt;
- either a job-relative source span with one-based line numbers and receipt/context hashes, or explicit unknown
  source state with candidate spans when the receipt is missing or ambiguous;
- categorical extraction confidence;
- an explicit `unknown`, `unconfirmed`, `confirmed`, or `corrected` review state.

Identifiers do not include list position, line number, sibling count, or corrected display text. Exact duplicate
criterion projections are deduplicated. Reordering criteria, adding or removing a sibling that shares a receipt, or
inserting unrelated advert lines therefore preserves identity. A changed source receipt or parser interpretation
creates a new identity. Any old confirmation is retained in its user-owned file and reported as orphaned for
reconciliation.

Ambiguous or unresolved source receipts are never silently assigned to the first match. Candidate spans carry a
context anchor, and an explicit occurrence selection is accepted only when that anchor is unique in the current
candidate set. Otherwise the source remains unknown and the correction becomes reconciliation work. Stable evidence
references and durable criterion matches will use content-derived semantic identity, while paths and human-readable
citations remain locators rather than identity.

Generated semantic artifacts may contain reviewed source text because they are application artifacts. Workflow
control records, manifests, and AgentResponse extensions contain only paths, hashes, identifiers, counts, and other
privacy-safe scalars.

## Consequences

- Parsed Job v1 and the legacy pipeline remain compatible.
- User corrections can change criterion text without changing criterion identity.
- Advert source changes create explicit reconciliation work instead of attaching an old decision to a different
  requirement.
- Duplicate source text requires an explicit occurrence plus unique context anchor, otherwise it remains ambiguous.
- Markdown fit reports become views of structured artifacts in later slices; they are not machine state.
- Public semantic schemas must be packaged and versioned independently from the agent envelope and Parse schema.

## Rejected Alternatives

- Add IDs directly to Parsed Job v1: rejected because it changes the accepted strict Parse contract without a
  migration.
- Use array indexes or source line numbers: rejected because harmless reordering would invalidate identity.
- Hash only the source receipt: rejected because one receipt can encode more than one parsed criterion.
- Hash user-corrected text: rejected because confirmation itself would detach the record it confirms.
- Treat existing evidence item labels as permanent identity: rejected because positional extraction renumbers later
  items after an insertion.

## Revisit When

Revisit when Parsed Job v2 is introduced, source documents have stable structural anchors, or evidence extraction
stores persistent IDs in the profile source rather than deriving them.
