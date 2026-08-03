# CanISend canonical Application flow v3

**Application authority:** `canisend.application-model/v3`

**Export format:** `canisend.application-flow-export/v3`

**Initial built-in surface:** `org.canisend.generic-application`

## Boundary

The canonical v3 flow advances one exact Pack-bound Application through intake, Requirement
confirmation, evidence binding, fit/Plan confirmation, Deliverable composition, explicit review,
managed package projection, embedded rendering, and consented local export. The Store service is
Pack-generic; the first shared facade binds it to the verified generic starter Pack. Pack selection
is explicit in CLI and desktop Workspace creation; the academic v2 route remains an explicit
compatibility surface.

The flow never logs in to a third-party service, uploads a file, or submits an Application. Every
read model and export Manifest reports `submission_performed: false`.

## Workspace activation

`workspace-v3.init` activates canonical v3 authority only after creating a new or empty Workspace.
The Store rejects this shortcut if any v2 Job, source, Evidence, artifact, workflow, task, profile,
discovery, Application, migration, or projection data already exists. Existing Workspaces must use
the previewed, verified-backup v2-to-v3 migration; ordinary initialization cannot relabel legacy
data as v3.

## Operations

1. `application-flow-v3.status` returns the current body-free snapshot and deterministic Pack stage
   states so a CLI or desktop session can resume without guessing.
2. `application-flow-v3.create` validates Pack metadata, stores bounded UTF-8 source content in the
   content-addressed Blob store, and creates proposed Requirements with exact byte spans.
3. `application-flow-v3.plan` requires the expected Application revision, records explicit user
   confirmation for every Requirement, and commits a user-confirmed Pack-qualified Plan.
4. `application-flow-v3.compose` validates Pack cardinality and MIME type, stores immutable content,
   and creates `review-required` Deliverables bound to the exact Plan and confirmed source
   revisions used as Evidence inputs.
5. `application-flow-v3.review` requires private-read consent and returns verified current
   Deliverable bodies for explicit local review. Routine status and list operations remain
   body-free.
6. `application-flow-v3.approve` verifies every referenced Blob and advances every current
   Deliverable to `approved` under explicit user authority.
7. `application-flow-v3.export` first publishes the existing managed Application projection, then
   renders every approved Deliverable with its verified Pack template and the embedded Typst
   compiler. It writes validated PDFs plus `render-manifest.json` only below
   `applications/APPLICATION_ID/exports/` and only after private-export consent.

The Application revision history is authoritative. Package JSON, editable content, PDFs, and the
render Manifest are recoverable projections bound to the Application revision, snapshot digest,
Pack ID/version/digest, Deliverable revisions, content hashes, output hashes, and page/byte counts.

## Stage state

Stage IDs come from the verified Pack graph. A stage is `complete` only when its declared output is
present and current; it is `ready` when every declared dependency is complete, otherwise it is
`pending`. For the generic starter Pack, one canonical fixture progresses through all nine stages:

- source-bound intake;
- user-confirmed Requirements and their exact source revisions as bounded Evidence;
- a confirmed fit decision and Plan;
- materialized Pack-cardinality-valid Deliverables;
- explicit approval;
- current managed package projections; and
- validated local PDF export.

This is a deterministic readiness view over authoritative records and the just-completed
package/render operation. It is not a second workflow authority.

## Defensive invariants

- Every mutation matches the exact Pack binding and expected Application revision before private
  Deliverable content is written.
- Metadata keys/types/options, Requirement categories, Deliverable kinds, cardinalities, template
  paths, renderer capabilities, and resource bytes come from the verified Pack.
- Source spans select valid UTF-8 byte boundaries; bodies are absent from routine read models and
  error messages.
- Pack templates remain data-only. Deliverable text is encoded as a Typst string, so Typst syntax
  in user content remains literal rather than gaining file or code authority.
- Rendering is bounded by the existing source, time, PDF-size, page-count, encryption, and PDF
  validation policies.
- A stale revision, wrong Pack, invalid cardinality, missing Blob, absent consent, unsafe path,
  symlink, unmanaged conflict, or non-approved Deliverable fails closed.

## Shared surface boundary

CLI and desktop expose Pack selection, reviewed UTF-8 intake with exact spans, list/status resume,
Plan, compose, consented private review, approval, and consented local export. Desktop fields,
categories, stages, localized labels, and Deliverable choices come from the exact verified Pack.
The v2-to-v3 Workspace migration is preview/digest/backup bound and preserves the academic Pack
presentation after activation. Direct local-file, text-PDF, and user-URL conversion into a
canonical v3 create request remains later adapter-parity work. Agent v3/MCP operation registration
is GF4-AGENT-001; no shared surface may write the database directly.
