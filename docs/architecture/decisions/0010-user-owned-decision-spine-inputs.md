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
scoped candidate, or apply an explicitly accepted update with an expected prior raw-byte hash and revision. They
never silently regenerate, normalize, or overwrite an existing file. Direct manual YAML edits remain valid; status
validates the bytes that are present and reports their baseline, while drift is reported and reconciled, not
rewritten. Users should advance the artifact `revision` when editing manually. An explicitly accepted scoped update
writes a canonical next revision and may not preserve YAML comments.

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

Initialization, update, and recovery each require `--confirm-user-owned-write`. Each semantic correction update also
requires current Parse and Confirm. An accepted correction makes Confirm stale, so Confirm must rerun before the next
correction patch; a client cannot apply a batch of semantic patches against one old Criteria projection.

The immutable mutation candidate and the user YAML are Tier 2 because they may contain correction text or rationale.
Corrected wording may also appear in the Tier 2 Criteria projection. The mutation claim and immutable receipt are
body-free control records; the receipt is Tier 1. Errors, ordinary CLI output, and AgentResponse contain only safe
paths, hashes, IDs, states, reason codes, and counts.

Multi-file transactional updates are not claimed. Each user-owned update is committed independently, and derived
artifacts are refreshed afterward.

Compare-and-swap coordinates cooperative CanISend writers and assumes the workspace and selected job-directory
topology remain stable during an operation. A normal editor saving in the final atomic-replace window can still win
after the service's last comparison, and a malicious process running as the same user can rename paths. These cases
are not linearized. Run status immediately before mutation and avoid a concurrent manual save. Remote workspaces,
multi-user locks, and hostile same-user processes are outside this local contract.

An immutable/exclusive publish may be interrupted after the complete target link is created but before CanISend
removes its private temporary link. Fresh status recognizes only the exact private, same-directory CanISend marker
with exactly two links and remains read-only; it reports recovery pending with the accepted mutation ID. Explicit
`user-mutation recover` consent removes that verified temporary link, rereads the claim/candidate/target/receipt, and
continues. Ordinary hard links, third links, non-private files, and lookalike names remain unsafe.

Semantic reset is not erasure. Correction history retains old corrected bodies, and immutable private-mode Tier 2
mutation candidates remain after reset, clear, withdraw, or supersession so accepted writes can be audited and
recovered (0600 on POSIX; inherited platform ACLs still apply). Private job folders must remain git-ignored and should enter backups only intentionally. Removing selected
private mutation events or the whole job is a separate retention decision that may make recovery impossible;
CanISend does not currently provide automatic secure erasure or deletion from backups/filesystem snapshots.

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
