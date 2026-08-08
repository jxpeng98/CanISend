---
name: canisend-review-export
description: Review, render, package, and export evidence-bound CanISend Deliverables. Use for cross-Deliverable review, evidence findings, dispositions, readiness checks, PDF rendering, package reconciliation, final checks, or local export.
---

# CanISend Review and Export

This skill covers Agent v4 tasks `review` and `export` for one exact Application.

## Review the current snapshot

1. Require `canisend.workspace/v4` and `canisend.agent/v4`, then bind the exact Application UUID,
   Pack identity and digest, revision, and snapshot digest.
2. Obtain private-read consent before reading Deliverable bodies. Inspect deterministic validation,
   evidence support, and Pack-qualified cross-Deliverable findings against that exact snapshot.
3. Explain unresolved findings. The user may approve eligible dispositions; deterministic blockers
   require correction and cannot be silently dismissed.
4. Preview each disposition, obtain approval for that exact digest, commit its single-use token,
   and verify the returned revision and audit receipt.

## Render and export locally

1. Re-orient after review and require current readiness. Readiness means only that CanISend may
   prepare local files.
2. Obtain separate private-export consent. Preview the exact Deliverables, format, destination,
   replacement behavior, and artifact graph.
3. Preserve user edits through CanISend's reconciliation path. Render only from authoritative
   structured Deliverables, never from an edited managed projection.
4. Commit the approved export preview and verify every returned artifact digest and local path.
   Confirm that `submission_performed` is `false`.

All writes follow `orient -> propose -> preview -> approve -> commit -> verify`. On stale context,
expiry, replay, consent denial, or restart, discard the preview and begin again. Never upload,
log in to a portal, or submit an Application.
