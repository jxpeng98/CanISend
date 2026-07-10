# ADR-001: Keep The Python Service Layer And File Workspace Authoritative

**Status:** Accepted

**Date:** 2026-07-10

## Context

CanISend is used through a CLI, Codex and Claude skills, local agent CLIs, and eventually MCP. If each surface owns
workflow rules, state derivation, or privacy decisions, the same workspace can produce different answers depending on
the host platform.

## Decision

The Python service layer is the only implementation of application workflow business rules. The local workspace is
the durable source of truth for user inputs, source provenance, evidence, decisions, authoritative artifacts, and run
state.

CLI commands, text presenters, JSON presenters, skills, provider adapters, and MCP handlers call the same services.
Skills may explain policy and route work, but they do not redefine state transitions or validation rules.

Chat history, provider sessions, and platform-local memory are useful context but are never authoritative workflow
state.

## Consequences

- Cross-platform resume is derived from workspace artifacts rather than copied chat history.
- New transports require conformance tests against the service layer.
- Platform-only behavior is treated as an adapter defect unless explicitly documented as a capability limitation.
- The workspace remains usable without MCP or a particular model provider.

## Rejected Alternatives

- Platform-specific workflows: rejected because they create drift and vendor lock-in.
- MCP as the business-logic core: rejected because CLI-only and offline use must remain first class.
- Chat history as state: rejected because it is not portable, deterministic, or reliably recoverable.

## Revisit When

Revisit only if CanISend becomes a hosted multi-user service whose authoritative state is intentionally migrated to a
server-side database with an explicit local-workspace compatibility strategy.
