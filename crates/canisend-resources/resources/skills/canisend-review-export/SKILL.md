---
name: canisend-review-export
description: Review the current CanISend material set, record dispositions, check readiness, render and verify local exports. Use Materials for substantive draft or Plan revisions.
---

# CanISend Review and Export

Tasks: `review`, `export`. Use [Workspace](../canisend-workspace/SKILL.md) for shared
state, consent and recovery rules. Bind review to the current Application snapshot.

## Review

Compare the set against confirmed Requirements, Evidence and Plan: criterion coverage,
supported wording, source-backed length/language/format limits, and consistent names, dates,
roles and publication status. Separate deterministic blockers from editorial recommendations;
a passing validator does not establish persuasive quality. Inspect private bodies only with
required consent. Route missing facts to Workspace, source corrections to Intake, and
substantive edits to Materials; refresh review after a change.

`canisend_review_disposition_preview` reviews the current material set using the Application
revision and required private-read request. It does not take an arbitrary finding ID or waive
individual blockers. Correct blockers, preview, then call its commit with the actual schema's
private-read and `request_confirmation: true` fields. Use individual or active Auto approval
under Workspace's shared rules; verify returned state
and report optional receipt fields as absent when they are not returned.

## Local export and verification

Require current readiness. `request_private_export` requests separate export consent; preview
the exact material set, formats, destination and replacement behavior, then request native
confirmation. Preserve user-edited projections: render from authoritative Deliverables and
use a discovered reconciliation/revision operation for edits, or report its absence.

Verify returned paths/digests and `submission_performed: false`. When a suitable local viewer
is available, inspect exported documents for missing/clipped content, blank pages, bad glyphs
and unresolved placeholders. Stay within the granted read scope; export consent does not
authorize an external viewer/model upload. If visual inspection is unavailable, report the
machine checks performed without claiming visual correctness.

Deliver the local file inventory and remaining limitations. Correct problems through the
owning Deliverable/template operation and re-review/re-export affected content. After input
changes or restore, inspect current state before reusing old exports. Local export is never
portal submission.
