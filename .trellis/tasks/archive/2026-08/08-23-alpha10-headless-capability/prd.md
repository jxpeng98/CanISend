# Close the App-absent CLI, Skills, and MCP journey

## Goal

Make the desktop App optional for the ordinary clean Workspace v4 journey by reusing the existing
CLI, application facade, Agent v4 Skills, and persistent MCP server.

## Requirements

- Keep direct CLI commands for bootstrap, management, recovery, and reads; keep guarded mutations
  in the same binary's persistent `mcp serve` process.
- Expose the application facade's existing project/global Skill-install scope through CLI host
  setup, status, and removal; project remains the default.
- Preserve one canonical embedded Skill source and host-specific Codex CLI, Claude Code, and
  Claude Desktop guidance. Do not create a new plugin or protocol layer.
- Prove one App-closed Workspace can hold academic and generic Applications and complete intake,
  evidence association, planning, material creation, review, export, backup, restore, and reopen.
- Preserve consent, single-use approval, audit, recovery, privacy, and no-submission invariants.

## Acceptance Criteria

- [ ] CLI initialization creates neutral Workspace v4 starter resources, Typst Profile example,
      generic examples, templates, and no Application or private Profile state.
- [ ] `host setup|status|remove --scope project|global` is explicit, idempotent, drift-detecting,
      and removes only unchanged manifest-owned resources.
- [ ] The existing headless harness completes academic and generic Applications in the same
      Workspace with the App closed, including guarded MCP work, verified export, backup, restore,
      and App-compatible reopen.
- [ ] Denial, replay, stale/wrong context, process restart, malformed input, and legacy fixtures
      fail without authority mutation or private-body disclosure.
- [ ] CLI help, Skills, MCP inventory, operation registry, and user guidance agree on Agent v4.
- [ ] Focused checks and one final Tier 2 source gate pass on the protected PR head.

## Out of Scope

- Old Skills, Agent v2/v3, Workspace v2/v3, `job` aliases, Python commands, or host layouts.
- Direct CLI aliases for every guarded MCP mutation.
- New Packs, remote MCP, provider integrations, automatic submission, or Alpha.10 publication.

## Parent Artifacts

- `../08-18-alpha10-release-integration/prd.md`
- `../08-18-alpha10-release-integration/design.md`
- `../08-18-alpha10-release-integration/research/2026-08-23-headless-capability-audit.md`
