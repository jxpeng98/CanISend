# ADR-006: Preserve Text Compatibility And Limit Phase 1 Platform Claims

**Status:** Accepted

**Date:** 2026-07-10

## Context

CanISend already has user workspaces, a text CLI, job status values, feed files, skills, and packaged defaults. Phase 1
adds an agent contract but no persistent stage state or MCP server. Overstating platform support or using resource
refresh as data migration would create unsafe upgrade expectations.

## Decision

- Existing command names, text defaults, job slugs, `job.yaml status`, and legacy lead fields remain readable.
- `--format json` is additive.
- Missing future workflow state is conservatively derived from existing artifacts.
- `update-workspace` refreshes packaged resources and preserves local edits by default; it is not a user-data migration
  engine.
- Every persistent workspace or artifact schema introduced later ships with an old-workspace fixture, upgrade test,
  and rollback behavior in the same phase.
- `skills/` becomes the canonical distributed skill pack. Initialized workspaces receive a self-contained copy under
  `agent-skills/`; the existing main-skill mirror remains for a compatibility release.
- Phase 1 supports shell-capable Codex CLI/App sessions, Claude Code, and IDE agents.
- Claude Desktop/App without a local command bridge becomes supported only after the Phase 2 local MCP slice.
- The current orchestrator remains experimental and is not reused as a trusted agent runtime in Phase 1.

## Consequences

- Existing users can upgrade without rewriting private job folders.
- Workspace-local skill edits remain protected by the existing overwrite policy.
- Fresh-session fixtures in Phase 1 prove durable state, not real cross-platform adapter conformance.
- Platform claims match capabilities that actually ship.

## Rejected Alternatives

- Add workflow state directly to every existing job in Phase 1: rejected because the stage contract is not ready.
- Claim Claude Desktop support through documentation alone: rejected because no tool bridge exists.
- Reuse the orchestrator for the agent protocol: rejected until outputs and writes are technically enforced.

## Revisit When

Revisit when Phase 2 adds persistent stage state and local MCP, and whenever a new persistent schema or supported host
is introduced.
