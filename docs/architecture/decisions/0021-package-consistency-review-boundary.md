# ADR-021: Add A Receipt-Bound Aggregate Package Review

**Status:** Accepted for Task 13 of Stage 3

**Date:** 2026-07-14

## Context

ADR-013 through ADR-020 establish document-scoped Draft, deterministic Review, user-owned finding dispositions,
derived readiness, and compatibility projections for Cover Letter and Research Statement. They deliberately do not
answer whether the current set of application documents is mutually consistent. Treating either document's Review
as an aggregate verdict would hide missing required documents, stale sibling documents, and cross-document receipt
disagreements.

The aggregate operation must work in CLI, Codex, Claude, and other shell-capable hosts through the existing guarded
stage runtime. It must not give a reviewer authority to rewrite a Draft, a user-owned disposition, or a compatibility
view. It must also distinguish deterministic conflicts from semantic judgements that Claim v1 cannot prove.

## Decision

Add one core-owned Tier 2 output, `package_review_findings.json`, produced by an independent deterministic
`package_review` stage. The static stage graph places it after document Review and before package generation, while
its adapter performs dynamic fan-in over every current Required Document Plan item. Missing or unavailable Review
instances are valid aggregate inputs and become findings; they do not prevent the aggregate reviewer from running.

The output binds the exact Parsed Job, Application Brief, Required Document Plan, derived document-execution plan,
and, when present, each selected document's Draft, Review, disposition, and derived readiness hashes. Every plan item
is represented, including omitted, unavailable, optional, stale, and unsupported items. A changed bound file changes
the aggregate input fingerprint and invalidates only the aggregate result and its descendants. It does not invalidate
a still-current sibling document Review.

The deterministic reviewer may classify as blockers only conditions it can prove from structured state:

- a required document is omitted, unavailable, missing, stale, blocked, unreviewed, or marked for revision;
- the Required Document Plan itself contains unresolved blockers;
- the same normalized factual assertion appears in more than one document but its support classification or exact
  Evidence receipt set disagrees; or
- an existing deterministic document Review proves a confirmed Brief exclusion or another document blocker.

Repeated wording with identical receipts, shared Evidence used for different wording, role-context wording,
proportionality, tone, and narrative alignment remain explicit human-review findings. Claim v1 does not encode a
typed factual predicate/value pair, so the reviewer must not infer a contradiction by parsing prose. A future Claim
contract may widen deterministic comparison only after it provides an explicit value identity.

Correction proposals are optional, document- and Claim-scoped Tier 2 records. They contain an instruction, not an
authoritative replacement, and always declare `guarded_draft_candidate` as their application route. Applying a
proposal requires a new candidate for the targeted document through the existing Draft validation and promotion
boundary. The aggregate reviewer never writes a Draft or a user-owned file.

TaskSpec, workflow state, run manifests, errors, CLI text, and AgentResponse expose only paths, hashes, counts,
reason codes, stable IDs, consent requirements, and next actions. They do not copy Claim text, Evidence bodies,
finding messages, correction instructions, Brief bodies, or disposition rationale.

## Consequences

- a host can resume one aggregate consistency operation from durable receipts without chat history;
- missing and stale required documents become inspectable findings instead of making Review silently unavailable;
- exact receipt disagreements are deterministic, while semantic overclaiming is never presented as machine truth;
- one changed Cover Letter invalidates aggregate Review without invalidating a current Research Statement Review;
- package-level user decisions and aggregate readiness remain a separate Task 14 boundary.

## Rejected Alternatives

- Reuse one document's `review_findings.json`: rejected because it cannot bind sibling documents or missing plan
  items.
- Let the aggregate stage edit Draft files: rejected because generation, Review, user decision, and promotion must
  remain separate ownership boundaries.
- Parse prose to infer contradictory values: rejected because Claim v1 has no typed predicate/value contract and a
  lexical heuristic would create false certainty.
- Block stage execution until every document is reviewed: rejected because missing and unavailable required
  documents are themselves aggregate findings that must be durable and reviewable.
- Store aggregate decisions in the findings output: rejected because deterministic findings are core-owned while
  acceptance and revision decisions are user-owned.

## Revisit When

Revisit before adding typed factual assertions to Claim, allowing more than one executor instance per normalized
document kind, introducing provider-backed aggregate Review, or applying correction proposals automatically.
