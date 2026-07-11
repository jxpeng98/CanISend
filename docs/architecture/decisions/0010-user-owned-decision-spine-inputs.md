# ADR-010: Keep Corrections, Decisions, And Briefs User-Owned

**Status:** Accepted

**Date:** 2026-07-11

## Context

The legacy pipeline reads language and style preferences from `job.yaml`, writes a fixed set of generated documents,
and derives match prose in the same run. That makes it difficult to distinguish an unanswered question from a
confirmed empty answer, and a regeneration can erase or obscure the human decision that justified an application.

Stage 2 needs durable corrections, an apply/hold/skip decision, and an application brief. These records can contain
private motivations, exclusions, and judgment. They must not become ordinary generated outputs owned by Parse,
Match, or a model provider.

## Decision

The following job-local files are user-owned inputs:

- `confirmed_corrections.yaml`;
- `application_decision.yaml`;
- `application_brief.yaml`.

Core services may create an unresolved template only when the file does not exist, validate its shape, stage a
scoped candidate, or apply an explicitly accepted update with an expected prior hash. They never silently regenerate,
normalize, or overwrite an existing file. Direct manual YAML edits remain valid; drift is reported and reconciled,
not rewritten.

An upstream change may make the basis of a decision or brief stale, but it does not clear the accepted value.
Orphaned corrections remain in the user-owned overlay with a reconciliation action. Generated artifacts such as
`criteria.json`, `criterion_matches.json`, and `required_document_plan.json` are core-owned and may be rebuilt from
their declared inputs.

Unknown and confirmed-empty are distinct states. Apply, hold, or skip is effective only after an explicit user
confirmation; Match may recommend an action but cannot write the decision. Required-document planning preserves
unknowns and produces actions rather than treating an empty parsed list as confirmation that no documents are
required.

Every programmatic mutation of a user-owned file will use:

1. a strict versioned model and safe YAML loading;
2. a scoped patch rather than an unrestricted whole-file replacement;
3. the expected current content hash and revision;
4. one atomic file replacement;
5. an immutable, privacy-safe mutation receipt;
6. an explicit consent boundary for an agent-assisted update.

Multi-file transactional updates are not claimed. Each user-owned update is committed independently, and derived
artifacts are refreshed afterward.

## Consequences

- Parse and Match reruns preserve corrections, application decisions, and briefs byte for byte.
- Accepted decisions survive a changed advert or evidence refresh but become review-required when their declared
  basis changes.
- Host agents cannot infer or silently write apply/hold/skip.
- `job.yaml` language and style values may bootstrap a new brief once, but there is no ongoing two-way sync.
- User-owned artifacts require compare-and-swap update services before agent mutation is exposed.
- AgentResponse 1.0 continues to return artifact references and scalar state only; private rationale and motivation
  are never copied into control records or response extensions.

## Rejected Alternatives

- Store decisions in generated Markdown: rejected because regeneration destroys machine-readable ownership.
- Keep all preferences in `job.yaml`: rejected because pipeline status writes and user decisions would share one
  last-writer-wins document.
- Let Match choose apply or skip: rejected because evidence classification is advisory and the decision belongs to
  the user.
- Replace user YAML after every validation: rejected because it destroys comments, manual edits, and conflict
  evidence.

## Revisit When

Revisit before remote workspaces, collaborative multi-user editing, or multi-file transactions. Those features need
stronger locking and event identity than the local single-file compare-and-swap contract.
