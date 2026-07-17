# ADR-RN-0002: Use a Six-Crate Cargo Workspace with Inward Dependency Direction

**Status:** Accepted

**Date:** 2026-07-17

## Context

CanISend combines a CLI, external contracts, workflow rules, durable storage, network/document I/O, and embedded
resources. A single large crate would make boundary enforcement difficult, while many small crates would increase
compile time and coordination before stable abstractions exist.

## Decision

The initial Cargo workspace contains:

- `canisend-contracts`: pure versioned types and schemas.
- `canisend-core`: domain rules, application services, stage graph, and port traits.
- `canisend-store`: SQLite, blobs, revisions, transactions, audit, and recovery.
- `canisend-io`: HTTP, parsers, discovery, providers, and rendering.
- `canisend-resources`: embedded assets and resource manifest.
- `canisend-cli`: the `canisend` executable, command tree, presentation, and process exit policy.

An optional Rust `xtask` package owns repository automation. Dependencies point inward: contracts are foundational;
core does not depend on CLI or concrete external adapters; outer crates implement core ports.

## Consequences

- Domain tests can run without SQLite, HTTP, terminal state, or model providers.
- External integrations can change without changing domain contracts.
- Shared dependency versions and release profiles live in the root workspace manifest.
- New crates require evidence that an existing boundary is insufficient.

## Rejected alternatives

- One crate: rejected because it permits CLI, storage, and domain coupling.
- One crate per workflow stage: rejected because it creates excessive package and compile overhead.
- Dynamic plugin crates in the first release: rejected because distribution and trust policy are not yet designed.

## Revisit when

Revisit after stable profiling shows a compile-time bottleneck or a component requires an independently versioned
public library.
