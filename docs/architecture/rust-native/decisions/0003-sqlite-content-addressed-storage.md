# ADR-RN-0003: Use SQLite Metadata with an Immutable Content-Addressed Blob Store

**Status:** Accepted

**Date:** 2026-07-17

## Context

The product needs durable workflow state, revisions, dependency invalidation, audit history, concurrent CLI/agent
access, private document bodies, and repairable user-visible outputs. Coordinating many mutable JSON/YAML files makes
atomic state transitions and recovery expensive.

## Decision

- SQLite is the authoritative metadata, workflow-state, and audit store.
- SQLite is linked into the executable; no database service is installed.
- Private and generated content bytes are immutable SHA-256-addressed blobs.
- Database rows reference validated blob digests.
- User-visible Markdown, Typst, JSON, and PDF files are projections with manifests.
- Blob publication happens before the database transaction that references it.
- State transitions and their audit events commit in one SQLite transaction.
- Projection generation follows the authoritative commit and is explicitly repairable.
- Automatic blob garbage collection is excluded from the first release.

## Consequences

- Interrupted writes may leave an unreferenced immutable blob but cannot create a missing referenced blob by design.
- Authoritative state can be checked, backed up, queried, and migrated transactionally.
- Agents and users must not edit `.canisend/state.sqlite3` or the blob directory.
- Export edits require an explicit reconcile action and never silently replace authoritative data.
- The release must test SQLite configuration and filesystem semantics on every supported platform.

## Rejected alternatives

- Mutable JSON/YAML as authoritative state: rejected because multi-record transitions are not transactional.
- Store every body directly in ordinary SQLite rows: rejected because immutable blob verification and streaming are
  clearer for large PDF and rendering artifacts.
- External database service: rejected because it violates standalone local installation.
- Git as the state database: rejected because private content and concurrent command semantics do not fit.

## Revisit when

Revisit for remote collaboration or synchronized multi-user workspaces. Those require a different authority and
conflict model rather than a transparent replacement of local SQLite.
