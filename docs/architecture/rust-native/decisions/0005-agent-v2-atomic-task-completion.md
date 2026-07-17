# ADR-RN-0005: Use Agent Protocol v2 with Atomic Task Completion

**Status:** Accepted

**Date:** 2026-07-17

## Context

Codex, Claude, IDE agents, and custom workers need a stable way to inspect context, obtain bounded work, and return a
candidate without editing internal state. The greenfield rebuild can remove the old multi-command compatibility
boundary while preserving explicit validation and consent.

## Decision

- Agent-facing JSON identifies `canisend.agent/v2`.
- JSON-mode commands emit exactly one response envelope on stdout; logs use stderr.
- `task prepare` creates a bounded task with exact input revisions, allowed output kind, schema, consent scopes, and
  lease metadata.
- `task complete` accepts a candidate through stdin or a safe regular file, rechecks inputs, validates the candidate,
  stores its immutable bytes, and commits the result atomically.
- Invalid candidates do not change authoritative workflow state and leave the task available for correction.
- A changed input makes the task stale.
- Identical repeated completion is idempotent when the recorded result permits it.
- Agents never write `.canisend/` or authoritative projection paths directly.

## Consequences

- Host integrations need only a CLI and JSON parser.
- The task service, not the host prompt, is the authority for allowed reads and writes.
- Protocol snapshots and error codes become release contracts inside the Rust v2 line.
- Candidate size, file type, symlink, and path checks must run before parsing private data.

## Rejected alternatives

- Let agents edit workflow files directly: rejected because validation and atomicity would be advisory.
- Preserve separate submit/apply commands: rejected because the new storage layer can validate and commit one result
  transactionally.
- Require a daemon or MCP server for the first release: rejected because it increases installation and lifecycle
  complexity without being necessary for Codex/Claude CLI use.

## Revisit when

Add a stdio server only after CLI protocol v2 is stable and the same task service can be reused without creating a
second authority.
