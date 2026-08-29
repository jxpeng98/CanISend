# Integrate and release Alpha.10

## Goal

Make the desktop App optional for the complete ordinary CanISend workflow: a fresh user can
initialize Workspace v4, install current host resources, connect supported AI hosts through MCP,
and complete both built-in Pack journeys through the standalone product surface before exact
`v1.0.0-alpha.10` qualification and publication.

## Background

- The user requires the capability coverage that made the 0.x CLI/Skills workflow useful, now
  implemented on the generic 1.x architecture and available before further App expansion.
- The recalled 2026-08-09 product decision explicitly rejects compatibility with old Skills,
  Agent v2/v3, `job` aliases, host layouts, and Workspace v2/v3. The required compatibility is
  user-outcome coverage, not old commands, files, or protocols.
- Current Workspace v4 initialization already creates verified starter guidance, a Typst Profile
  example, generic examples, and Typst templates while preserving user-owned existing files.
- Current host resources contain four integrity-managed Agent v4 Skills generated for Codex and
  Claude Code. The App supports project/global Skill scope; the CLI host commands currently force
  project scope.
- Current standalone CLI has 31 leaves for initialization, health/recovery, Application creation
  and reads, Profile Source import, host setup/status/removal, and MCP stdio.
- Current MCP server has 36 tools and owns persistent-process guarded Requirement, Plan,
  Deliverable, association, review, and export mutations. The CLI intentionally exposes only the
  corresponding reads because approval tokens are process-bound.
- Alpha.7 through Alpha.9 recorded App-closed Codex CLI, Claude Code, Claude Desktop, and bounded
  MCP-client evidence, but the Roadmap still leaves final App/CLI initialization and
  Agent/Skills/MCP integration evidence incomplete.
- Public Alpha.10 is immutable from protected source
  `cd40180f2ff8ac957276f1948ba88da428511a82`; its native candidate, same-byte promotion, public
  assets, and Codex Generic scenario already passed. The remaining qualification work is the
  Academic Codex scenario and body-free policy reconciliation.

## Requirements

### Capability boundary

- Preserve the clean Workspace v4, Agent v4, Application-level Pack binding, consent,
  single-use approval, audit, recovery, and no-submission invariants.
- Provide one documented App-absent path from an empty directory through Workspace initialization,
  starter/Profile inspection, host setup, mixed academic/generic Applications, guarded work,
  review, export, backup, restore, and reopen.
- Reuse the application facade and existing persistent MCP approval broker. Do not duplicate
  business logic in CLI, Skills, plugins, or host-specific adapters.

### Workspace initialization

- `canisend workspace init` must create the same neutral authoritative Workspace and verified
  starter resources as App initialization, without creating an Application, private Profile body,
  or Workspace mode.
- Initialization must remain failure-atomic, preserve existing user files under the approved
  policy, and provide actionable next steps for Profile import, both Packs, host setup, and MCP.

### CLI and headless workflow

- The standalone product must work while the App is closed and expose every ordinary workflow
  outcome through a reviewed combination of direct CLI operations and the bundled MCP server.
- Human output and `canisend.agent/v4` JSON must retain stable error, privacy, next-action, and
  no-mutation behavior.
- Five supported CLI targets must exercise the same Workspace/Pack facts without Python or App
  runtime dependencies.

### Skills, plugin, and MCP coordination

- Project and global installation must be explicit, inspectable, idempotent, drift-detecting,
  safely removable, and limited to manifest-owned resources.
- Codex CLI, Claude Code, and Claude Desktop must receive host-correct setup guidance from one
  canonical embedded resource set. Codex is the required real-host release surface; Claude
  real-host sessions are non-blocking compatibility observations. Any Codex plugin packaging must
  reuse those resources rather than create a second Skill source.
- Host configuration must remain user-reviewed: CanISend may generate exact commands/snippets and
  project-local files where already authorized, but must not overwrite unrelated global host
  configuration.
- MCP must remain the structured persistent transport for guarded mutations and must not require
  the App to run or allow an Agent to write `.canisend`, SQLite, Blobs, or projections directly.

### Release

- Reconcile Roadmap, GitHub Issue/milestone, Trellis metadata, and machine release facts before
  starting an Alpha.10 candidate.
- Reuse the completed sequential-Alpha build-once promotion and independent public verification.
  Exact host evidence must bind the unchanged public bytes and must not trigger a rebuild.

## Acceptance Criteria

- [ ] A clean CLI-only setup creates Workspace v4 with verified README, Typst Profile example,
      generic examples, templates, both embedded Packs, and no private or Application state.
- [ ] CLI host setup supports the approved project/global scope for Codex and Claude resources,
      reports drift, and emits exact MCP registration/removal guidance without overwriting unrelated
      host configuration.
- [ ] With the App closed, the supported headless surface completes one academic and one generic
      Application in the same Workspace through intake, Profile/Evidence association,
      Requirements, Plan, Deliverables, review, verified export, backup, restore, and reopen.
- [ ] Denial, replay, stale revision, wrong Workspace/Application/Pack, host restart, malformed
      input, and unsupported legacy fixtures fail without mutation or private-body disclosure.
- [ ] App reopen observes the exact headless receipts and revisions without reconciliation drift.
- [ ] Current generated Skills/plugin resources, CLI help, MCP inventory, operation registry,
      guides, and release notes describe one Agent v4 workflow with no legacy compatibility claim.
- [ ] Exact Alpha.10 source, native candidate, promoted public bytes, and App-closed Academic and
      Generic Codex evidence agree before Alpha.10 is called qualified for Beta entry.

## Out of Scope

- Compatibility with old Skills, plugin layouts, Agent v2/v3, `job` aliases, Workspace v2/v3, or
  the old Python implementation.
- Restoring domain-specific 0.x command names when a neutral v4 operation owns the same outcome.
- Agent-owned authority writes, automatic login/upload/submission, provider credentials, or
  private conversation/body retention.
- Real invited-user cohort evidence, Beta.1, feature freeze, RC, Stable, or package-manager
  publication. The cohort runs on public Beta.1 and remains required before RC planning.
- New workflow Packs, OCR, remote MCP, or speculative provider integrations.

## Technical Notes

- The user approved direct CLI commands for initialization, management, and reads plus the same
  binary's persistent MCP server for guarded mutations. Separate direct CLI aliases for every MCP
  mutation are not required.
- A new Codex plugin distribution is not required while canonical Agent v4 Skills and MCP satisfy
  the supported host journey; any future plugin must package the same embedded resources.
