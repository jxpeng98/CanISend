---
name: canisend-review-export
description: Review, render, package, and export evidence-bound CanISend Deliverables. Use for cross-Deliverable review, evidence findings, dispositions, readiness checks, PDF rendering, package reconciliation, final checks, or local export.
---

# CanISend Review and Export

This skill covers Agent v4 tasks `review` and `export` for one exact Application.

## Review substance and consistency

Compare the entire material set with the confirmed Requirements and Plan. Check whether each
mandatory criterion is addressed, each factual claim has the right Evidence and its wording does
not overstate that evidence. Separate deterministic failures from editorial recommendations.
Check relevance, specificity, readable structure, repetition and whether the intended audience
can find the argument. A passing validator is not a judgement of persuasive quality.

Cross-check names, dates, roles, publication status, metrics and future/completed work across
materials. Check the source-backed length, language and format constraints. Route missing facts
to `canisend-workspace`, source ambiguity to `canisend-intake`, and content or Plan revisions to
`canisend-materials`; refresh affected review state after a change. Never dismiss a blocker merely
to reach export, and do not require a separate approval for every non-mutating editorial comment.

## Review the current snapshot

1. Require `canisend.workspace/v4` and `canisend.agent/v4`, then bind the exact Application UUID,
   Pack identity and digest, revision, and snapshot digest.
2. Obtain private-read consent before reading Deliverable bodies. Inspect deterministic validation,
   evidence support, and Pack-qualified cross-Deliverable findings against that exact snapshot.
3. Explain unresolved findings. The user may approve eligible dispositions; deterministic blockers
   require correction and cannot be silently dismissed.
4. Preview each disposition, call its guarded commit with `request_confirmation: true`, and
   let the user answer the native form. Verify the returned revision and any returned audit receipt; report an absent receipt as absent.

## Render and export locally

1. Re-orient after review and require current readiness. Readiness means only that CanISend may
   prepare local files.
2. Request separate private-export consent with `request_private_export`. Preview the exact
   Deliverables, format, destination, replacement behavior, and artifact graph.
3. Preserve user edits through CanISend's reconciliation path. Render only from authoritative
   structured Deliverables, never from an edited managed projection.
4. Request native confirmation for the export preview with `request_confirmation: true`; only
   the user may approve. Verify every returned artifact digest and local path, and confirm that
   `submission_performed` is `false`.

## Deliver and resume

Inspect generated documents when a suitable local viewer/parser is available: missing sections,
clipped text, blank pages, unresolved placeholders, bad glyphs and broken links. Report when only
machine integrity checks were performed; a PDF hash does not prove visual correctness. Correct
problems through authoritative Deliverables/templates and the supported reconciliation path,
then render and verify again with the required consent.

Deliver the actual local paths, document inventory, verification results and remaining limitations.
Distinguish draft, reviewed and exported states; never call a hold Plan a finished application.
If materials or sources change after export, use a fresh review/readiness check and export rather
than treating old files as current. Restored Workspace drafts can be authoritative while scoped
export directories are absent; inspect first and request a fresh export instead of declaring data
loss or silently copying stale output. Portal submission remains the user's separate action.

Follow `canisend-workspace` for MCP consent fields and preview handling. Confirmation requests
are not user approval. After denial, stop the denied operation without retry or fallback;
independent authorized checks may continue. On stale context, expiry,
replay, or restart, discard the preview and re-orient. Never upload, log in to a portal, or submit.
