# ADR-002: Version The Agent Envelope Separately From The Package

**Status:** Accepted

**Date:** 2026-07-10

## Context

Host agents need stable machine output. The Python package version, protocol compatibility, response schema,
workspace schema, and artifact schemas evolve at different rates.

## Decision

Phase 1 introduces:

- protocol major: `canisend.agent/v1`;
- response schema: `1.0.0`;
- dotted operation names such as `workspace.inspect` and `job.intake`;
- `agent capabilities` as the discovery point for supported protocol and schema versions.

Public producer models reject undeclared top-level and nested fields. Optional experimental scalar metadata may appear
only inside a namespaced `extensions` mapping. A new core field requires a response schema version change. A breaking
semantic change requires a new protocol major.

Phase 1 emits only schema `1.0.0`; multi-schema response negotiation may be added when a second schema exists.

## Consequences

- Upgrading the Python package does not imply a breaking agent protocol.
- Clients can branch on capabilities instead of parsing `--help` output.
- Strict producer models reduce accidental private-data leakage.
- Extension values are limited to JSON scalars; structured additions use a reviewed schema revision.

## Rejected Alternatives

- Use the package version as the protocol version: rejected because release and protocol compatibility differ.
- Permit arbitrary unknown fields: rejected because it weakens leak prevention and validation.
- Freeze every future field in Phase 1: rejected because stage execution does not yet exist.

## Revisit When

Revisit when schema `1.1.0` or protocol `v2` is proposed, at which point compatibility fixtures and negotiation behavior
must be implemented with the change.
