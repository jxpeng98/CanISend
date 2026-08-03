# CanISend user guides

CanISend source is a local-first framework with two exact-bound built-in workflow Packs:

- `org.canisend.generic-application` is the domain-neutral default and uses Workspace/Agent v3.
- `org.canisend.academic-job` is the academic-job reference Pack and retains the Workspace/Agent
  v2 compatibility journey.

The latest publicly qualified checkpoint is `v1.0.0-alpha.5`. The post-tag `main` source contains
additional roadmap work; it is not a published Alpha.6 or Alpha.7 until exact artifacts pass the
release gates. Always compare `canisend version --json` with the release manifest before following
a source-only command.

Choose a Pack before creating a Workspace. A Workspace is then bound to that exact Pack identity
and digest; it is not a domain selector that can be changed later. A v2→v3 migration preserves the
academic Pack and does not convert academic records into generic Applications.

Start here:

1. [Install the native binary](installation.md).
2. [Verify checksums, provenance, and release contents](release-verification.md).
3. [Choose a Pack and complete its quick start](quick-start.md).
4. [Connect Codex, Claude, or another Agent host](agent-integration.md).
5. [Understand privacy and consent boundaries](privacy-and-consent.md).
6. [Back up and restore the exact Pack-bound Workspace](backup-and-recovery.md).
7. [Upgrade, migrate, roll back, or uninstall safely](upgrade-and-rollback.md).
8. [Review current product and distribution limits](known-limitations.md).
9. [Diagnose common failures](troubleshooting.md).
10. [Try the macOS-first desktop GUI](desktop-gui.md).

The machine contracts are [Agent protocol v2](../contracts/agent-protocol-v2.md) for the academic
compatibility surface and [Agent v3/MCP](../contracts/agent-v3-mcp.md) for canonical generic
operations.
Security assumptions are in the [threat model](../security/threat-model.md). CanISend prepares and
exports local material; it never logs in, uploads, or submits an Application.
