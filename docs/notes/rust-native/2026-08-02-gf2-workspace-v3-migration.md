# GF2-MIG-001 — Workspace v2→v3 migration implementation record

**Date:** 2026-08-02

**Status:** Implemented locally; work-item linkage and independent review remain roadmap
governance steps.

## Delivered

- Added append-only database migration 15 with a migration ledger, deterministic Job/Application
  links, and immutable digest-only legacy bindings.
- Added a body-free, deterministic dry-run report over the complete frozen v2 table inventory.
- Required a verified `org.canisend.academic-job` Pack with the full legacy Requirement and
  Deliverable vocabulary.
- Mapped every Job to a neutral Opportunity/Application snapshot and mapped current Criteria,
  Plans, planned documents, materialized documents, source spans, and evidence citations.
- Bound every legacy source row to the exact Pack digest without rewriting legacy tables,
  artifact digests, Blob bytes, or projections.
- Required the exact reviewed plan digest and created a verified pre-migration backup before the
  single authority/data transaction.
- Rechecked source inventory, referenced Blob identities, projection state, v3 authority, and
  Application count before commit.
- Added neutral app-service preview and migrate entry points; no CLI, network, provider, upload, or
  submission behavior was introduced.

## Focused evidence

- Migration dry runs omit a private sentinel and stale previews fail before backup creation.
- Repeated unchanged dry runs are byte-equivalent; an invalid referenced Blob rejects preview and
  leaves v3 authority absent.
- A Workspace containing a Job, source, workflow, Agent task, parsed artifact, and confirmed
  Criteria migrates with identical legacy-inventory digest/counts and referenced Blob set.
- The migrated v3 Requirement preserves confirmed user authority and exact source provenance.
- The verified pre-migration backup restores to a separate v2 Workspace with v3 authority absent.
- Existing Store and App suites pass with database schema 15.

## Remaining boundary

GF2-MIG-002 failure injection, low-space/DB-busy qualification, and stable old-binary remediation
are recorded in the [failure qualification](2026-08-02-gf2-workspace-v3-failure-qualification.md).
GF2-PROJ-001 owns `applications/` projections and legacy path recognition. GF3-PACK-001 must
provide the reviewed built-in academic Pack consumed by these app services before ordinary users
can invoke migration.
