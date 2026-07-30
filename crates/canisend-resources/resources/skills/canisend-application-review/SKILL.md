---
name: canisend-application-review
description: Review and export a CanISend academic application package. Use for cross-document review, findings, readiness, package reconciliation, PDF rendering, final checks, or export preparation.
---

# CanISend application review

Use this workflow for review, package, render, and export.

1. Inspect `canisend_context`, `canisend_workflow_status`, and the current
   structured document set before reviewing private bodies.
2. Prepare `document-review` for the exact current revisions. Return only
   schema-valid semantic and cross-document findings with exact targets.
   CanISend supplies deterministic findings.
3. Explain each unresolved finding. The user may explicitly accept or dismiss
   eligible semantic findings; deterministic blockers require a current
   redraft and cannot be dismissed.
4. Run package readiness against the current plan, profile, evidence, document,
   and review revisions. Treat `ready-to-export` only as permission to create
   files.
5. Obtain separate private-export approval before writing an editable package
   or rendered PDFs.
6. Reconcile managed projections before replacement. Never overwrite user edits
   implicitly: use the explicit replace path to discard one edit, or copy the
   edited bytes to an unmanaged path before restoring the managed projection.
7. Build PDFs only through CanISend's trusted render path from authoritative
   structured documents. Never compile an edited managed Typst projection as a
   trusted input.
8. Inspect the final artifact graph and report unresolved blockers, warnings,
   output paths, and the exact user-owned action that remains outside CanISend.

Never interpret readiness, rendering, or export as submission consent. Never
submit the application, upload artifacts, or edit internal state directly.
