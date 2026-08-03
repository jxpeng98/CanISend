# GF3-COMPAT-001 — bounded academic v2 compatibility

**Date:** 2026-08-03

**Status:** Implemented locally; work-item linkage, committed source-gate inspection, canonical
Agent v3 write operations, and native qualification remain roadmap steps.

## Delivered

- Added a Store read boundary that distinguishes Workspace v2 from active v3 authority and reads
  deterministic legacy Job→Application→exact Pack bindings.
- Added one typed legacy-operation registry with explicit canonical v3 targets; mappings are never
  derived from labels, IDs, or a latest Pack version.
- Added optional typed compatibility metadata to application receipts and Agent v2 CLI responses
  without changing ordinary `data` payloads.
- Bound Agent v2 MCP reads and guarded writes, Agent context/capabilities, task operations,
  workflow status, profile-source listing, and all `job` CLI commands to the verified academic
  Pack compatibility policy.
- Preserved Workspace v2 behavior as implicit academic compatibility.
- Allowed exact migrated-academic reads, rejected generic/mismatched/unmapped v3 Applications,
  and rejected every migrated legacy write before Workspace mutation.
- Added stable `compatibility.unavailable` classification with detected Pack context,
  `workspace_mutated: false`, and the canonical v3 remediation action.
- Deliberately reviewed and advanced the pre-Beta Agent v2 schema/snapshot freeze digests for the
  optional compatibility member and stable error code required by ADR-RN-0018; protocol and public
  schema versions remain unchanged, and the generated candidate differs only in those Agent hashes.

## Defensive invariant

The owned invariant is single Workspace authority. A legacy operation must not update only v2
tables after Workspace v3 activation because the neutral Application snapshot and dependency
ledger would not advance in the same transaction. The bounded fixture migrates a local academic
Workspace, attempts a legacy archive, and proves the Job remains unchanged.

## Focused evidence

- Operation-registry coverage asserts unique legacy keys and a nonempty explicit v3 target.
- Exact-Pack tests reject a digest mismatch even when the Pack ID is academic.
- Synthetic generic-Pack context fails with `compatibility.unavailable` and
  `application.show` remediation.
- Migrated academic `job.show` succeeds as read-only; `job.archive` fails before mutation.
- CLI binary contracts preserve current v2 fixtures and assert `job-cli` metadata.
- MCP read tests assert `agent-v2`, exact Pack identity, and canonical target on every response.

## Verification

```console
cargo test -p canisend-store compatibility_v3 --locked
cargo test -p canisend-app compatibility --locked
cargo test -p canisend-cli --test binary_contract --locked
cargo test -p canisend-mcp --locked
```

## Remaining boundary

GF3-UI-001 must remove hard-coded academic presentation from shared desktop surfaces. GF5 owns the
canonical Agent v3/CLI/MCP operation registry and may implement the write targets named here. Until
then, migrated v3 legacy writes remain unavailable rather than dual-writing mixed authority.
