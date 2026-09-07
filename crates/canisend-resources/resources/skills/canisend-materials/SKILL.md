---
name: canisend-materials
description: Plan and draft evidence-bound CanISend Deliverables for any workflow Pack. Use for fit analysis, Evidence associations, proceed or hold decisions, document plans, first drafts, revisions, or unsupported-claim audits.
---

# CanISend Materials

This skill covers Agent v4 tasks `fit-plan` and `drafting` for one exact Application.

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

Follow `canisend-workspace` for MCP consent fields and preview handling. Confirmation requests
are not user approval. After denial, stop the denied operation without retry or fallback;
independent authorized checks may continue. Refresh context after stale
revision, Pack mismatch, expiry, or restart. Never edit internal storage, upload, or submit.
