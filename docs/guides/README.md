# CanISend user guides

CanISend is a local-first framework with two exact-bound built-in workflow Packs:

- `org.canisend.generic-application` is the domain-neutral reference Pack.
- `org.canisend.academic-job` is the academic-job reference Pack.

The latest publicly qualified checkpoint is
[`v1.0.0-alpha.9`](https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.9). Later `main`
changes are not part of those published bytes. Always compare `canisend version --json` with the
release manifest before following a source-only command.

Initialize one neutral Workspace v4, then choose a Pack for each Application. Academic and generic
Applications may coexist in that Workspace; each Application keeps its own exact Pack identity,
digest, revisions, and associations.

Start here:

1. [Install the native binary](installation.md).
2. [Verify checksums, provenance, and release contents](release-verification.md).
3. [Create Pack-bound Applications and complete the quick start](quick-start.md).
4. [Connect Codex, Claude, or another Agent host](agent-integration.md).
5. [Understand privacy and consent boundaries](privacy-and-consent.md).
6. [Back up and restore the neutral Workspace](backup-and-recovery.md).
7. [Upgrade, migrate, roll back, or uninstall safely](upgrade-and-rollback.md).
8. [Review current product and distribution limits](known-limitations.md).
9. [Diagnose common failures](troubleshooting.md).
10. [Try the macOS-first desktop GUI](desktop-gui.md).

The active machine contract is [Agent v4](../contracts/agent-v4.md) for both built-in Packs. Earlier
Agent protocols and compatibility surfaces are historical and unsupported by Alpha.7 and later.
Security assumptions are in the [threat model](../security/threat-model.md). CanISend prepares and
exports local material; it never logs in, uploads, or submits an Application.
