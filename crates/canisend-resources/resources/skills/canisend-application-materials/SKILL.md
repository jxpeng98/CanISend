---
name: canisend-application-materials
description: Build evidence-backed academic application materials in CanISend. Use for profile evidence, requirement matching, apply or hold decisions, cover letters, CV tailoring, research statements, teaching statements, or other drafts.
---

# CanISend application materials

Use this workflow for evidence, matching, planning, and drafting.

1. Read the selected job's `canisend_context` and
   `canisend_workflow_status`. Follow the exact returned `next_actions`; do not
   skip an incomplete prerequisite.
2. Inspect profile-source metadata without exposing bodies. For
   `evidence-normalize`, export only declared inputs after private-read and
   provider-send consent. Propose traceable evidence with source spans and
   sensitivity; never invent achievements, dates, publications, or metrics.
3. Let the user correct, exclude, and classify evidence before confirmation.
   Use only confirmed, non-excluded evidence revisions downstream.
4. For `evidence-match`, produce one revision-bound proposal per exact
   criterion. State gaps and prohibited claims instead of smoothing them over.
5. Export the application plan. Explain CanISend's blockers and safe `hold`
   default. The user alone chooses `apply`, `hold`, or `skip` and approves the
   document plan.
6. Draft only when the confirmed decision is `apply` and CanISend reports no
   blocking evidence gap. Prepare the exact next planned `*-draft` task and
   complete documents sequentially.
7. Ground every material claim in confirmed evidence. Keep honest
   placeholders where evidence is missing. Respect the task's requested
   document kind, prompt, schema, revisions, and mode.
8. Preview and commit each task completion through MCP when available, or the
   equivalent versioned CLI flow. Refresh workflow status after every commit.
9. Finish with a brief coverage summary: requirements supported, gaps retained,
   drafts completed, and the next review action.

Never confirm evidence or an application decision on the user's behalf. Never
send private content without the scoped consents returned by CanISend.
