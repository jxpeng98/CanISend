# R3 Storage and Recovery Evidence

**Date:** 2026-07-17

**Status:** Complete

## Authoritative workspace model

Workspace format v2 now uses a root `canisend.toml` identity and a private `.canisend/` directory. The binary can
initialize a workspace, discover one from a descendant directory, resolve an explicit `--workspace`, report status,
run integrity checks, repair derived projections, create a verified backup, and restore into a new empty directory.
Configuration and database identity must match before a workspace opens.

Bundled SQLite is the authority for metadata and workflow state. Migration 1 establishes workspace metadata, jobs,
source and evidence revisions, artifacts and exact dependency edges, workflow/stage/task state, consent, audit
events, projections, discovery leads, and provider invocation records. Connections enable foreign keys, WAL mode,
full synchronous durability, and a bounded busy timeout. Migration application, workspace identity, and audit/state
transitions use transactions.

## Immutable content and artifacts

Private bodies are stored as bounded SHA-256-addressed blobs. Publication streams through a mode-0600 temporary file,
flushes it, creates the final immutable name without replacement, syncs the containing directory, and verifies the
final bytes. Reads revalidate internal directory layout and content digest. Traversal-shaped digests, symlinks,
non-file collisions, oversized streams, interrupted reads, and digest collisions fail closed.

Artifact commits publish bytes first, then atomically record the revision, blob reference, exact dependencies, head
revision, stale propagation, and audit event. A dependency conflict after publication changes no authoritative row;
the resulting unreferenced blob is reported by `workspace check` and is never deleted automatically. Updating an
upstream artifact recursively marks descendants stale.

User-visible files are derived projections. A failed projection records `repair-required` while the authoritative
artifact remains verified and readable. `workspace repair` recreates the projection from the authoritative blob.

## Backup and restore

Backup pins one SQLite read snapshot, uses SQLite's online backup API, copies only referenced and verified blobs,
includes the workspace configuration, and writes a versioned manifest containing digests for every component. The
temporary backup is fully verified before it is renamed into place. Restore accepts only a new empty destination,
verifies the source backup first, recreates private layout, and then reopens the workspace through normal identity and
integrity checks.

## Public contracts and CLI

R3 adds generated schemas for workspace status, workspace check, and backup manifest, bringing the public schema
catalog to 18 and the embedded resource catalog to 24. `workspace.lifecycle` is now truthfully marked `available` in
agent capabilities. All workspace commands support the v2 JSON envelope and grouped exit policy.

Representative commands are:

```text
canisend --workspace ./application workspace init --json
canisend --workspace ./application workspace status --json
canisend --workspace ./application workspace check --json
canisend --workspace ./application workspace backup ./backup --json
canisend workspace restore ./backup ./restored --json
canisend --workspace ./restored workspace repair --json
```

## Verification evidence

Local verification passed formatting, Clippy with warnings denied, all workspace tests, generated schema/resource
checks, release compilation, and the full packaged-binary initialization/backup/restore smoke. The suite contains 27
Rust tests, including migration rollback, corrupt database rejection, concurrent readers, writer conflict, bounded
blob interruption, internal symlink rejection, publication-before-transaction fault injection, projection failure,
backup verification, restore, and CLI process-boundary coverage.

GitHub Actions run `29612319788` repeated the clean-checkout Rust-only gate and packaged-binary smoke in 1 minute 48
seconds with no annotations. R4 can now build file, URL, HTML, and PDF intake on one durable source/artifact authority
without introducing mutable sidecar state.
