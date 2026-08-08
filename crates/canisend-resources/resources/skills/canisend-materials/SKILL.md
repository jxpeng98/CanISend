---
name: canisend-materials
description: Plan and draft evidence-bound CanISend Deliverables for any workflow Pack. Use for fit analysis, Evidence associations, proceed or hold decisions, document plans, first drafts, revisions, or unsupported-claim audits.
---

# CanISend Materials

This skill covers Agent v4 tasks `fit-plan` and `drafting` for one exact Application.

## Plan from confirmed Evidence

1. Require `canisend.workspace/v4` and `canisend.agent/v4`, then bind the exact Application UUID,
   Pack ID, Pack version, Pack digest, revision, and snapshot digest.
2. Inspect confirmed Requirements and Evidence metadata. Request private bodies only with the
   exact consent CanISend requires.
3. Propose explicit Evidence-to-Application associations and a Pack-qualified Plan. Show supported
   Requirements, retained gaps, prohibited claims, and the safe hold state.
4. The user alone chooses whether and how to proceed. Preview and commit only the exact approved
   Plan and associations.

## Draft Deliverables

1. Follow the confirmed Plan and the exact Deliverable kinds declared by the selected Pack. Pack
   vocabulary can differ; the evidence and approval rules do not.
2. Ground each material claim in confirmed, associated Evidence. Keep an honest gap or placeholder
   when support is absent; never invent achievements, identities, dates, metrics, or citations.
3. Draft or revise one bounded Deliverable at a time. Run the evidence audit before presenting the
   mutation preview.
4. After explicit approval, commit the single-use preview token and verify the new revision,
   snapshot digest, audit event, and artifact references.

All writes follow `orient -> propose -> preview -> approve -> commit -> verify`. Refresh context
instead of retrying after stale revision, Pack mismatch, expiry, or restart. Never edit internal
storage, upload files, or submit an Application.
