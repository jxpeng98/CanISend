# ADR-022: Separate Package Findings, User Decisions, And Readiness

**Status:** Accepted for Task 14 of Stage 3

**Date:** 2026-07-14

## Context

ADR-021 adds a deterministic, core-owned aggregate Review over the exact Required Document Plan and current
document receipts. Its findings cannot also store acceptance or revision decisions: that would let a deterministic
rerun overwrite user intent and would make a stale decision appear current after any document changes.

Per-document `reviewed` is necessary but does not establish that all required documents exist or that
cross-document findings were dispositioned. Conversely, application-package readiness must not be presented as
rendering readiness, portal readiness, submission, or proof that an employer received an application.

## Decision

Add one independent Tier 2 user-owned file, `package_review_dispositions.yaml`. It binds the exact SHA-256 of
`package_review_findings.json`, uses revision/hash compare-and-swap, requires explicit write consent, and reuses the
existing immutable mutation claim, private candidate, receipt, and interrupted-publication recovery boundary.
Changing aggregate Review preserves the older file but makes its basis stale. Only the explicit
`reset_for_current_package_review` patch can bind a new aggregate Review and clear the earlier decisions.

Each package finding decision is either `accepted` or `revision_required`. A deterministic blocker cannot be
accepted or waived. A patch changes one finding at a time and must target a current finding ID. Package-level patch
operation names are distinct from document Review operations so a host cannot accidentally route a decision to the
wrong user-owned file.

Add a strict derived `ApplicationPackageReadinessV1` contract. It binds:

- the exact Required Document Plan and derived execution-plan hashes;
- every required document's aggregate state and Draft/Review/disposition/readiness receipts;
- the exact package Review hash; and
- the package disposition hash only when it matches that Review.

The derived states are `blocked`, `review_required`, `revision_required`, and `reviewed`. Any required document that
is omitted, unavailable, missing, stale, blocked, unreviewed, or marked for revision prevents `reviewed`. Every
current non-blocker package finding needs an explicit decision. Blockers remain non-waivable. Optional documents
are reported separately and never become required merely because a Draft exists.

No mutable readiness file is authoritative. Status and `check-package` independently rederive readiness from exact
current inputs. APP-Q fails closed when aggregate findings or dispositions are absent, invalid, stale, or incomplete.
Legacy package files remain readable, but Cover Letter readiness alone can no longer pass the application-package
gate.

CLI and AgentResponse expose paths, hashes, revisions, counts, states, reason codes, consent requirements, and next
actions only. Finding messages, Claim text, correction instructions, and user rationale remain in the Tier 2 data
plane. `reviewed` means only that the current application-document package completed this review boundary.

## Consequences

- deterministic findings can be regenerated without overwriting a user decision;
- a fresh Codex, Claude, CLI, or other shell-capable host can resume from durable local receipts;
- cross-document mutation histories cannot collide with Cover Letter or Research Statement histories;
- APP-Q can prove that every required document and package finding belongs to one exact current basis; and
- rendering, manual submission, and submission evidence remain later, independent gates.

## Rejected Alternatives

- Store decisions in `package_review_findings.json`: rejected because deterministic reruns must not own user intent.
- Reuse either document's disposition file: rejected because document and aggregate finding identities have
  different bases and mutation ownership.
- Treat all package findings as waivable: rejected because proven missing required documents and receipt conflicts
  are executable blockers.
- Persist one mutable readiness verdict: rejected because a verdict can become stale independently of its receipts.
- Let optional standalone documents silently enter APP-Q: rejected because the confirmed Required Document Plan is
  the only source of requiredness.

## Revisit When

Revisit before introducing rendering approval, portal/upload state, submission evidence, automatic application of
correction proposals, or multiple executor instances for one normalized document kind.
