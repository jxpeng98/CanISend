# GF4-FLOW-001 canonical generic Application flow

**Date:** 2026-08-03

**Roadmap item:** GF4-FLOW-001; contributes to M1F-DELIV and the Alpha.7 dual-Pack checkpoint

## Outcome

The verified generic starter Pack now completes a canonical v3 local flow instead of existing only
as declarative data. A new/empty Workspace can activate v3 authority, create an exact Pack-bound
Application, confirm source-bound Requirements, record a Plan, compose and approve the Pack's
`primary-document` and `supporting-document`, publish editable projections, render validated PDFs,
and export a revision-bound Manifest without submission.

## Implementation

- `canisend-store::ApplicationFlowServiceV3` is Pack-generic and validates all Pack-owned inputs.
- `canisend-app` exposes the first generic built-in facade operations and preserves explicit export
  consent.
- v3 Requirement source Blobs now enter the reference ledger, so backup and Blob audit cover both
  intake sources and Deliverable content.
- the neutral renderer projects approved Deliverable content through verified data-only Pack
  templates without a `DocumentKind` branch;
- the existing Application projection manager supplies the editable package; a bounded export
  directory receives validated PDFs and a body-free integrity Manifest.

## Defensive verification

The owned components are CanISend's v3 repository, Blob store, Pack runtime, projection manager,
embedded renderer, and local export path. Focused regressions prove:

- an end-to-end fixture reaches all nine generic Pack stages with two disjoint custom Deliverable
  kinds and four authoritative Application revisions;
- stale and wrong-Pack operations add neither an Application revision nor private Blob content;
- empty-Workspace activation refuses legacy product data and leaves v3 authority inactive;
- export requires explicit consent, stays below the Application export root, produces validated
  PDFs, and reports `submission_performed: false` in both package and render records; and
- user content containing Typst syntax is passed as literal string data to the embedded compiler.

The focused, full Workspace test, strict Clippy, formatting, and source release gates are run before
this record is committed. The existing macOS linker may emit its known `__eh_frame` size warning;
it is not a test or strict-Clippy failure.

## Remaining boundary

GF4-FLOW-001 closes the Store/shared-facade local fixture. GF4-UI-001 now supplies Pack selection,
canonical v3 intake and resume, consented review, approval, export, and digest-bound migration
surfaces. Generic context, guarded writes, resume, and recovery through Agent v3/MCP remain
GF4-AGENT-001. Synthetic scenario families and dual-Pack native qualification also remain open, so
the first usable framework Alpha is not yet qualified.
