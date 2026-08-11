# Changelog

## Published `v1.0.0-alpha.8`

- Moved active Workspace and Application context into the desktop sidebar.
- Added project or global installation scope for managed Agent Skills while keeping MCP
  configuration project-local.
- Added Typst Profile import and verified starter resources for new Workspace directories.
- Preserved exact Alpha provider evidence across sequential source transitions and made Alpha.8
  eligible for the same fail-closed Beta-readiness checks as Alpha.7.

Published from `35e7c822ea2f469ab726a31b5d08e622f6810c55` on 2026-08-10 after build-once native
qualification, promotion without recompilation, and independent public-download verification. The
immutable release unit is at
https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.8.

## Published `v1.0.0-alpha.7`

- Replaced Workspace modes with clean `canisend.workspace/v4`: one neutral Workspace can hold
  independently Pack-bound academic and generic Applications.
- Added explicit Application associations for shared Profile Sources and Evidence, connected
  source intake, and guarded Requirement, Plan, and Deliverable operations.
- Added App-led atomic bootstrap plus standalone CLI initialization, host management, basic-data
  import/read, recovery, and persistent MCP stdio workflows.
- Added `canisend.agent/v4`, schema version `4.0.0`, and one canonical source for
  integrity-managed Codex and Claude Code Skills.
- Removed supported Alpha.6 aliases, Agent v2/v3 requests, old Skill layouts, and Workspace v2/v3
  migration from the Alpha.7 surface; unsupported legacy inputs fail before mutation.
- Added packaged host and full guarded dual-Pack MCP lifecycle smoke on Linux, Windows, and macOS.

Published from `9986a6a63b596b7760b4721a7e97c36aedce6d51` on 2026-08-10 after build-once native
qualification and independent public-download verification. The immutable release unit is at
https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.7.

## Published `v1.0.0-alpha.6`

`v1.0.0-alpha.6` is the previous publicly qualified checkpoint. Its Workspace v3/Agent v2-v3
contracts, release notes, manifest, artifacts, and qualification evidence remain immutable at the
tag.

## Historical Rust-native `0.7.0-alpha.1` development

- Started a greenfield Rust replacement with no Python runtime, Pytest, old-workspace, or agent-v1 compatibility.
- Archived the final Python implementation at `archive/python-v0.6.0b1-final`.
- Added the six-crate Cargo workspace and Rust `xtask` automation.
- Added the `canisend.agent/v2` response envelope and truthful compiled capability registry.
- Added native `version`, `doctor`, and `agent capabilities` commands.
- Added an embedded resource manifest with SHA-256 verification.
- Added Rust-native dependency spikes for bundled SQLite, generated schemas, PDF extraction, embedded Typst, and
  Rustls. The complete spike passed on Ubuntu, macOS, and Windows in GitHub Actions run `29608591519`.
- Replaced the active Python package, Pytest suite, schemas, resources, and publication automation with the
  Rust-native product foundation.

## Python release history

The complete historical changelog is available from the archive tag:

```text
git show archive/python-v0.6.0b1-final:CHANGELOG.md
```

The previous published beta was `v0.6.0b1`. It is not compatible with the Rust-native workspace or agent protocol.
