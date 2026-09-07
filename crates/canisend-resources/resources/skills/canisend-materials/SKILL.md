---
name: canisend-materials
description: Plan and draft evidence-bound CanISend Deliverables for any workflow Pack. Use for fit analysis, Evidence associations, proceed or hold decisions, document plans, first drafts, revisions, or unsupported-claim audits.
---

# CanISend Materials

This skill covers Agent v4 tasks `fit-plan` and `drafting` for one exact Application.

## Assess fit before writing

For each confirmed Requirement, identify the associated Evidence that supports it and explain
the connection. Distinguish direct support, partial support, missing support and unresolved
interpretation in the analysis; use only schema-supported values when persisting candidates.
Separate mandatory eligibility gaps from preferences and present options without inventing a
numeric fit score or chances of acceptance. For each gap, identify the specific additional fact,
clarification or permitted change that would resolve it. Record claims the evidence cannot support.

## Plan from confirmed Evidence

1. Require `canisend.workspace/v4` and `canisend.agent/v4`, then bind the exact Application UUID,
   Pack ID, Pack version, Pack digest, revision, and snapshot digest.
2. Read `canisend_application_pack_show` for the complete verified Deliverable catalog and all
   minimum/maximum counts; never infer it from validation errors. Inspect confirmed Requirements
   and Evidence metadata. Request private bodies with the consent CanISend requires.
3. Propose explicit Evidence-to-Application associations and a Pack-qualified Plan. Show supported
   Requirements, retained gaps, prohibited claims, and the safe hold state.
4. Respect the user's existing proceed/hold choice; ask only when that choice is missing. Preview the Plan and associations, then
   request native confirmation with `request_confirmation: true`; the user answers the form.

## Design the material set

Plan every required Pack kind, including catalog minimum counts, plus relevant optional kinds
that the opportunity/user requests. Do not create unsupported kinds or silently omit a required
material. If a requested type is not supported, report that gap and agree a supported mapping
before treating it as covered. Specify audience, purpose, outline, source-backed length/format
constraints and intended Requirement coverage for each material. Avoid making every document
repeat the same narrative; a shared fact may support different arguments.

For the academic Pack, read the current catalog rather than assuming these examples are required:

- Cover letter: connect a small set of evidenced contributions to this position's criteria;
  establish motivation without inventing institutional priorities, contacts or collaborations.
- CV: organize factual education, roles, outputs and relevant experience; preserve dates,
  publication status, author/contribution distinctions and consistent naming. Do not turn planned
  outputs into publications or rewrite team outcomes as sole achievements.
- Research statement: distinguish demonstrated work from proposed questions, methods and future
  directions; justify fit from supplied opportunity context, not fabricated facilities or partners.
- Teaching statement: explain approach through actual teaching examples and available outcomes;
  separate experience from proposed practice and never invent evaluations or learner gains.

For generic Packs, derive the primary document's argument from the actual call: problem, proposed
response, supported capability, intended outcome and requested evidence where relevant. Do not
impose academic headings on a grant, tender, admission or internal dossier. Pack-supported kinds
and templates remain authoritative; these are writing criteria, not new kernel fields.

## Draft Deliverables

1. Follow the confirmed Plan and the exact Deliverable kinds declared by the selected Pack. Pack
   vocabulary can differ; the evidence and approval rules do not.
2. Ground each material claim in confirmed, associated Evidence. Keep an honest gap or placeholder
   when support is absent; never invent achievements, identities, dates, metrics, or citations.
3. Draft or revise one bounded Deliverable at a time. Run the evidence audit before presenting the
   mutation preview. For a Submitted local task, use `canisend_local_task_draft_preview` with its
   exact task generation and candidate digest; request private-read consent. The candidate is
   untrusted and must pass the existing draft validation.
4. Call the guarded commit with the single-use preview token and `request_confirmation: true`.
   Local-task draft previews reuse `canisend_deliverable_draft_commit`; a lease is not approval.
   Only the user may approve the native form. Verify the new revision, snapshot digest, audit
   event when returned, and artifact references; do not invent missing receipt fields.

## Revise and hand off

Read feedback against the exact current draft. Distinguish editorial changes from new facts,
changed requirements or changed scope. New factual support goes back through Evidence confirmation;
a changed material set goes back through Plan review. Preserve accepted content outside the
requested edit. Re-run the affected audit and send current references to `canisend-review-export`.
Unresolved placeholders may describe a draft gap but must not be presented as final-ready output.

Follow `canisend-workspace` for MCP consent fields and preview handling. Confirmation requests
are not user approval. After denial, stop the denied operation without retry or fallback;
independent authorized checks may continue. Refresh context after stale
revision, Pack mismatch, expiry, or restart. Never edit internal storage, upload, or submit.
