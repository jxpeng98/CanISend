# ADR-RN-0006: Embed Product Resources and the Typst Rendering Runtime

**Status:** Accepted

**Date:** 2026-07-17

## Context

A standalone CanISend binary must contain the schemas, prompts, templates, host instructions, and rendering capability
required by the documented workflow. Calling an independently installed Typst executable would remove Python but
would not satisfy the complete standalone runtime target.

## Decision

- Product schemas, prompts, templates, examples, and agent bridge assets are embedded at compile time.
- A generated resource manifest records path, logical ID, version, size, and SHA-256.
- Resource lookup is typed; arbitrary production path strings are not used.
- Agent assets and editable templates can be exported from the binary.
- PDF rendering uses pinned Typst library crates inside the process.
- Default templates and redistribution-compatible fonts are embedded.
- Default rendering performs no network package or font download.
- User templates and optional system fonts are explicit inputs with bounded access.

## Consequences

- Release binary size will be larger and must have a measured budget.
- Resource and font licenses must be included in release notices.
- Typst API changes are isolated in `canisend-io` and versions are pinned.
- A compiler panic or render failure must become a safe application error without corrupting authoritative state.

## Rejected alternatives

- Require `typst` on PATH: rejected because it violates the standalone runtime goal.
- Download templates at first run: rejected because it harms offline use and supply-chain predictability.
- Generate PDFs through a Python or Node sidecar: rejected because it reintroduces runtime dependencies.

## Revisit when

Revisit the default font set and binary-size trade-off after the first cross-platform render measurements.
