# CanISend Stage 4I–4M content integration and experience plan

**Status:** Stage 4I, Stage 4J, Stage 4K, and Stage 4L complete in source; Stage 4M is the
next follow-on slice.

**Product decision:** CanISend remains the local-first application state, validation, and
visualization layer. External Codex, Claude, or another host remains the primary Agent runtime.
Content is integrated through stable local records and body-free coordination models rather than
by moving conversations or provider credentials into CanISend.

## Goal

Turn the current collection of capable tabs into one continuous application-preparation workspace:
every application has one understandable state, every stored item has visible provenance and
relationships, every screen continues the same workflow, and Agent assistance starts from the
same authoritative next action as the user interface.

## Product invariants

- Job, Discovery, Profile, Workflow, Task, Document, Review, Package, and Render records remain
  authoritative in their existing services.
- Read models may compose authoritative state but must not create a second mutable truth.
- Routine list, overview, diagnostic, handoff, and Agent-context responses remain body-free.
- Private bodies are read only through an explicit, scoped operation and existing consent rules.
- CanISend never owns external-host transcripts, credentials, plugins, or search sessions.
- Every proposed mutation remains previewable, revision-bound, and user-confirmed where required.
- Imported URLs, PDFs, CSV, JSON, text, and Agent candidates remain untrusted input.

## Stage 4I — Application Dossier foundation

### I1. Unified read contract

Create one `ApplicationDossierReadModel` that composes:

- canonical job identity and archive state;
- direct or promoted-discovery origin;
- location, deadline, source URL, freshness, and last-seen metadata;
- job-source and reusable-profile-source readiness;
- workflow state, current stage, completed/total stages;
- only the blocker relevant to the current stage; and
- the exact authoritative `next_actions`.

The Dossier stores no bodies and introduces no schema migration. Promoted-lead metadata is resolved
through the existing `promoted_job_id` relationship rather than copied into the Job record.

### I2. Shared product boundary

Expose the same model through:

- the Rust application facade;
- `canisend application list --json`;
- `canisend application show --job JOB_ID --json`;
- Tauri commands and the TypeScript bridge; and
- selected-job Agent Context.

CLI and GUI parity ledgers must name the same operation family and tests must prove that private
source text is absent.

### I3. Connected overview

Use the Dossier to replace placeholder and reconstructed state:

- Today shows real upcoming deadlines and the selected application's next action.
- Applications shows progress, current stage, location, deadline, relevant blocker, and a
  route-aware Continue action.
- State and blockers use text and icons as well as colour.
- Changing a job, completing a workflow mutation, importing profile evidence, reviewing, packaging,
  or rendering refreshes the same selected Dossier.

### I4. Stage 4I exit

- [x] Dossier contract and metadata projection implemented.
- [x] Rust, CLI, Tauri, TypeScript, Today, Applications, and Agent Context connected.
- [x] English and Simplified Chinese copy added.
- [x] Focused body-free, metadata, bridge, deadline, and UI tests added.
- [x] Complete the final source gate and record
  [evidence](../../notes/rust-native/2026-07-30-stage4i-application-dossier.md).

## Stage 4J — Content Catalog and local search

### J1. Catalog

Introduce a read-oriented catalog entry for user-visible content:

- [x] Derive source adverts, normalized text, profile sources, evidence, criteria, matches, plans,
  documents, review findings, package state, and render outputs from current artifact heads.
- [x] Expose type, title, subject jobs, revision/hash, recursively conservative privacy,
  provenance, freshness, lifecycle status, and exact upstream relationships.
- [x] Keep the catalog read-only and rebuildable without a database migration or copied bodies.

Each entry must expose type, title, subject job, revision, privacy classification, provenance,
freshness, generated/confirmed status, and relationships. Mutable state remains in the owning
service.

### J2. Search

Add local indexed search with:

- [x] Provide title/metadata search without private-body consent.
- [x] Require an explicit non-empty query and `read-private-inputs` consent before any private
  workspace read.
- [x] Filter by application, content category, stage, status, privacy, and UTC date bounds.
- [x] Return exact provenance, match field, bounded private snippet, and route-aware deep links.
- [x] Rebuild a deterministic bounded inverted index in memory and discard it after each search.

Index data is a repairable projection, not an authoritative copy. Search snippets must obey the
same privacy boundary as the query.

### J3. Intake convergence

Make URL, PDF, local file, CSV, JSON, and Agent-provided intake converge on a shared preview:

- [x] Present source identity, detected type, extraction result, duplicate signal, target,
  intended mutations, and the exact confirmed consent scope through one typed review model.
- [x] Reuse one bilingual Svelte review component for direct job intake and discovery intake.
- [x] Commit direct intake from the exact prepared bytes and discovery intake from the exact
  normalized report held in bounded Rust preview state; never reread on confirmation.
- [x] Keep duplicate review explicit and disable automatic merge.

### J4. Stage 4J exit

- [x] Rust store, application, CLI, Tauri, and TypeScript content boundaries implemented.
- [x] Content Library embedded in Application Overview with English and Simplified Chinese copy.
- [x] Shared intake review implemented across file, URL/PDF, CSV, JSON, Agent, and network sources.
- [x] Focused privacy, rebuild, consent-ordering, bridge, and navigation tests implemented.
- [x] Complete the final source gate and record
  [Stage 4J evidence](../../notes/rust-native/2026-07-30-stage4j-content-catalog.md).

## Stage 4K — Application Workspace information architecture

Replace independent feature tabs as the primary journey with a selected-application workspace:

- [x] Overview
- [x] Job & criteria
- [x] Evidence & fit
- [x] Materials
- [x] Review & export

Global Workspaces, Opportunities, Profile, Agent Integration, and Settings remain supporting
surfaces. A persistent context bar shows workspace, application, deadline, current stage, blocker,
and next action. Every receipt deep-links to its affected content and recommended continuation.

### K1. Navigation convergence

- [x] Remove Workflow and Documents & delivery from the primary sidebar without removing their
  underlying revision-bound controls.
- [x] Map legacy detail routes and Content Catalog deep links into the five workspace sections.
- [x] Keep internal Workflow and Delivery tab changes synchronized with global navigation memory.
- [x] Restore the selected workspace, application, section, detail, and last successful action
  across normal restarts.

### K2. Persistent application context

- [x] Show the selected workspace and application plus Dossier deadline, current stage, progress,
  lifecycle state, first relevant blocker, and authoritative next action.
- [x] Keep the five application sections keyboard reachable with explicit text and mature Lucide
  icons; retain the existing skip link and focus indicators.
- [x] Support the 960-pixel minimum desktop window without overflowing the context controls.
- [x] Increase deep-link scroll offsets so fixed navigation never obscures the destination.

### K3. Continuation and performance

- [x] Bind successful creation, promotion, intake, workflow, review, package, and render receipts
  to their affected application route.
- [x] Select a newly created or promoted application before recording its continuation route.
- [x] Lazy-load Workflow, Delivery, Agent, and Content Library surfaces with visible,
  reduced-motion-safe loading and retry states.
- [x] Complete the source gate and record
  [Stage 4K evidence](../../notes/rust-native/2026-07-30-stage4k-application-workspace.md).

## Stage 4L — Contextual Agent assistance

- [x] Generate a bounded body-free context packet from the Dossier and sanitized Content Catalog
  relationships.
- [x] Recommend the smallest applicable project skill, application section, and exact CanISend
  action.
- [x] Keep external-host handoff primary and the in-App runtime bridge optional/read-only.
- [x] Represent revision-bound proposal state for criteria, evidence, matches, plans, and drafts.
- [x] Show proposal diff, revision provenance, validation, commit boundary, and intended state
  change before confirmation.
- [x] Refresh the Dossier and Catalog after commit, and invalidate stale Agent guidance so UI and
  Agent continue from the same next action.
- [x] Keep the 13-tool MCP surface and 40 generated schemas frozen; expose the additive read model
  through the application facade, CLI, Tauri, and TypeScript bridge.
- [x] Complete focused/source gates and record
  [Stage 4L evidence](../../notes/rust-native/2026-07-30-stage4l-contextual-agent-assistance.md).

CanISend does not store a parallel chat history. Codex/Claude session continuity and host tools
remain owned by those hosts.

## Stage 4M — Qualification and product hardening

- Add catalog rebuild/reopen, stale revision, concurrent edit, and malformed-input regressions.
- Prove no private bodies enter Dossier, registry, diagnostics, handoff, or routine search metadata.
- Add accessibility, keyboard, bilingual, empty/error/loading, and navigation-continuity coverage.
- Measure Dossier list and indexed-search latency on bounded large local fixtures.
- Update migration, backup, recovery, and rollback documentation for any new projection.
- Run native package qualification only at an explicitly authorized release checkpoint.

## Ordered delivery

1. Finish Stage 4I source gates and evidence.
2. Complete consented real-provider dogfood and the already-required Alpha.5 qualification.
3. Implement Stage 4J Catalog before adding broad new navigation, so the UI has one content model.
4. Implement Stage 4K on top of Dossier + Catalog rather than adding more isolated tabs.
5. Add Stage 4L proposal UX only after authoritative relationships and deep links are stable.
6. Complete Stage 4M, then decide whether the result is an additional Alpha or the Beta feature
   freeze candidate.

## Definition of done

The integration programme is complete when a user can import or discover an opportunity, see one
coherent application state, find every related source and generated artifact with provenance,
continue the exact next action from either CanISend or an external Agent, review every mutation,
and return to the same current application without reconstructing context across tabs.
