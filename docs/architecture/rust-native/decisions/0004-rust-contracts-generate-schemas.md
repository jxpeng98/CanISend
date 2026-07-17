# ADR-RN-0004: Make Rust Contract Types the Source of Truth and Generate JSON Schemas

**Status:** Accepted

**Date:** 2026-07-17

## Context

CanISend accepts machine-generated candidates from host agents and configured providers. Maintaining handwritten
schemas separately from runtime models risks drift. The greenfield rebuild has no reason to preserve the earlier
Pydantic/schema generation model.

## Decision

- Public request, response, candidate, and artifact structures are Rust types in `canisend-contracts`.
- JSON Schemas are generated from those types and assigned stable schema IDs and semantic versions.
- Generated schemas are embedded in the binary and exportable for agent hosts.
- Runtime validation has two layers: JSON Schema validation followed by Rust semantic validation.
- Generated output is deterministic and checked for drift through Rust `xtask` automation.
- Machine contracts use JSON. User configuration uses TOML. Machine state does not use user-editable YAML.
- Canonical hashing uses one documented canonical JSON algorithm rather than incidental serializer output.

## Consequences

- A contract change updates code, schema, snapshots, and protocol documentation together.
- Serde deserialization alone is never treated as complete semantic validation.
- Stable error codes are maintained independently from human error messages.
- Schema generation tooling becomes a build/test dependency and must be pinned.

## Rejected alternatives

- Handwritten schemas as the only source: rejected because Rust model drift would remain possible.
- Rust deserialization without schemas: rejected because agents need an inspectable candidate contract.
- Protobuf for the first protocol: rejected because CLI agent hosts work naturally with JSON and JSON Schema.

## Revisit when

Revisit if a long-lived network service needs a binary protocol. CLI JSON remains supported for local agent hosts.
