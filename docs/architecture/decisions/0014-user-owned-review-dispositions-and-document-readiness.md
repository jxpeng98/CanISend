# ADR-014: Keep Review Dispositions User-Owned And Derive Document Readiness

**Status:** Accepted for the next Stage 3 Cover Letter slice

**Date:** 2026-07-14

## Context

ADR-013 separates a promoted structured Draft from deterministic Review findings. The first Cover Letter slice keeps
both artifacts immutable and `proposed`, so an open semantic finding cannot silently become reviewed and the
compatibility projection cannot participate in package readiness.

The next slice needs a durable user decision without allowing an agent to rewrite Review, waive an executable
blocker, or claim that one reviewed Cover Letter makes the complete application package ready. The decision must
also survive a fresh host session and use the existing explicit-consent, raw-byte revision/hash compare-and-swap and
recovery boundary for user-owned files.

## Decision

CanISend adds `review_dispositions.yaml` as a user-owned Tier 2 artifact for the current Cover Letter. It binds the
exact promoted Draft and deterministic Review hashes. Each entry targets one stable finding ID and records exactly
one body-free disposition:

- `accepted`: the user completed the semantic check and accepts the current wording or warning;
- `revision_required`: the finding must be addressed by a new Draft.

Executable blocker findings are non-waivable. They may be marked only `revision_required`; an `accepted` blocker is
ignored by readiness inspection and rejected by guarded updates. A changed Draft or Review preserves the user file
but changes its basis status to `review_required`. An explicit `reset_for_current_review` update rebinds the file to
the new hashes and clears old dispositions; findings are never matched across Review hashes by position or text.

### Mutation and privacy boundary

- status is read-only and body-free;
- initialization and each single-finding update require explicit write consent and the current raw-byte SHA-256 and
  revision baseline;
- accepted mutations use the existing private candidate, immutable claim, atomic one-file replacement, immutable
  receipt, and recovery path;
- direct user edits remain supported but are validated before they affect readiness;
- finding messages, next actions, Claim text, and Evidence bodies do not enter receipts, errors, ordinary output, or
  AgentResponse.

### Derived document readiness

Cover Letter readiness is a deterministic projection, not a mutable status on Draft or Review. Its states are:

- `blocked`: the current Review contains an executable blocker;
- `review_required`: dispositions are missing, stale, invalid for the current Review, or incomplete;
- `revision_required`: at least one current non-blocker finding is explicitly marked for revision;
- `reviewed`: the Review is current and blocker-free and every current finding is explicitly `accepted`.

The compatibility content projection records the exact disposition hash, which binds its validated revision, and
the derived state. Package
checking may accept the Cover Letter review gate only when those receipts still match and the state independently
re-derives as `reviewed`. Draft and Review remain `proposed`; no file is rewritten to manufacture a reviewed state.
This slice establishes document readiness only. Required-document completeness, other package gates, verification,
rendering, and manual submission remain separate.

## Consequences

- semantic review decisions persist across Codex, Claude Code, and plain CLI sessions;
- agents cannot self-certify their own Draft or deterministic Review output;
- blocker findings cannot be converted into readiness by a waiver-like patch;
- a stale disposition file fails closed without erasing user history;
- compatibility files can carry a verifiable reviewed Cover Letter receipt while the whole package can still fail.

## Rejected Alternatives

- Change Draft or Review `review_state` in place: rejected because those artifacts are core-owned immutable stage
  outputs and their generators must not certify downstream human judgment.
- Allow arbitrary waiver text: rejected because it adds private bodies to the control surface and lets executable
  contradictions appear resolved without a new Draft.
- Carry dispositions forward by finding list position or message similarity: rejected because both are unstable and
  can attach a decision to different wording.
- Persist a second mutable readiness flag: rejected because it can drift from Review and dispositions; readiness is
  always re-derived.
- Treat reviewed Cover Letter as package readiness: rejected because required documents and later gates remain
  independent.

## Revisit When

Revisit before cross-document disposition reuse, multi-user review, signed approvals, package-stage aggregation, or
automatic portal submission.
