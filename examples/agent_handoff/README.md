# Host-Neutral Agent Handoff

This fake-data contract example demonstrates how two shell-capable hosts can resume the same CanISend application
workflow from durable workspace state. It does not copy a prompt, transcript, or provider session between hosts.

Host A and host B both request the current context with the same command:

```bash
canisend agent context \
  --workspace <workspace> \
  --job jobs/<job-id> \
  --format json
```

The response contains versioned protocol metadata, a safe job summary, a derived workflow snapshot, relative or
opaque artifact references, missing fields, consent requirements, blockers, and next actions. It does not contain the
full advert, profile, or package body.

`expected_capabilities.json` records the stable Phase 1 capability subset. `expected_context_shape.json` records the
required response shape and body fields that must never appear. Dynamic values such as `request_id`, package version,
job ID, hashes, and derived readiness are intentionally not frozen in the fixtures.

Phase 1 proves host-neutral inspection through a local command bridge for Codex CLI/App, Claude Code, and IDE agents.
Semantic task preparation, result application, candidate promotion, and local MCP transport begin in Phase 2.
