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
4. Respect the user's existing proceed/hold choice; ask only when it is missing. Commit associations
   separately and refresh context before Plan preview. Use `canisend_plan_propose_preview` and
   its commit to save the proposal, then `canisend_plan_confirm_preview` and its commit to confirm
   it. Each guarded commit uses `request_confirmation: true`; the user answers. A saved proposal
   is not a confirmed Plan, and confirmed hold is not permission to draft.

## Recover after an input correction

Read the current Requirements, Plan and Deliverables. Return any proposed Requirements to
`canisend-intake` for explicit decisions first. Reassess the fit and constraints, then use the
existing Plan proposal preview/commit to rebuild a `stale` Plan with the same identity. Its old
confirmation is cleared; confirm the new draft Plan through the normal native form.

When materials already exist, preserve their kinds and counts in the new Plan. Adding/removing
material kinds or changing counts is not supported by this recovery path; report that specific
gap instead of deleting history. After Plan confirmation, update each stale material through
Deliverable revision with its existing UUID and the latest Application revision. Read its old
content only with the required consent, reassess it against the corrected inputs and Evidence,
and retain only supported claims. Materials remain stale until updated and must pass review again
before a fresh export. Old exports remain historical artifacts, not current application outputs.

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
3. Prepare the complete initial material set required by the current Plan and Pack catalog before
   `canisend_deliverable_draft_preview`. First draft creation is a set operation, not an append:
   academic cover-letter and CV must be included together when both are required. The draft input
   uses kind, title, media_type and content; do not add legacy structured-claim fields or invent IDs.
   Review the candidate's evidence support before preview. `canisend_deliverable_audit` reads
   persisted drafts, so it cannot validate a not-yet-committed candidate.
4. For a Submitted local task, use `canisend_local_task_draft_preview` with exact task generation
   and candidate digest and request private-read consent. Its candidate must satisfy the same
   complete-set constraints; worker handoff does not allow per-document draft appends.
5. Initial previews use `canisend_deliverable_draft_commit`. Once materials exist, revise the exact
   Deliverable via `canisend_deliverable_revise_preview` and `canisend_deliverable_revise_commit`;
   do not retry initial draft creation. Use the returned single-use token and
   `request_confirmation: true`; the user answers. Verify returned revisions and artifact fields,
   then audit the stored result with required private-read consent before review. Missing optional
   audit receipt fields are reported as absent, not invented.

## Revise and hand off

Read feedback against the exact current draft. Distinguish editorial changes from new facts,
changed requirements or changed scope. New factual support goes back through Evidence confirmation;
a changed material set goes back through Plan review. Preserve accepted content outside the
requested edit. Re-run the affected audit and send current references to `canisend-review-export`.
Unresolved placeholders may describe a draft gap but must not be presented as final-ready output.

Follow `canisend-workspace` for MCP consent fields and preview handling. Confirmation requests
are not user approval. After denial, stop the denied operation without retry or fallback;
independent authorized checks may continue. Refresh context after stale revision, Pack mismatch, expiry, or restart. Never edit internal storage, upload, or submit.
