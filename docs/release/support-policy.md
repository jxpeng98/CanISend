# CanISend 1.0 Support Policy

This is the pre-Stable support policy for the Rust-native `1.0` line. The machine-readable authority is
[`release/support-policy.json`](../../release/support-policy.json). It remains `pre-stable-draft` during Alpha, Beta,
and RC; the Stable source gate requires it to become `published` when the workspace version loses its prerelease
suffix.

## Version support

- A prerelease is **current-only-until-superseded** by the next Alpha, Beta, RC, or Stable build. It is not an LTS
  channel.
- After Stable, support is **current-minor-latest-patch**: only the latest `1.0.x` patch is maintained and users may
  need to update before a defect can be reproduced or fixed.
- The Python `0.6` line is archived and unsupported. It is a historical source reference, not a runtime,
  compatibility, migration, or security-maintenance channel.
- There is no long-term-support release and **No service-level agreement**. Security and data-loss reports are still
  triaged through [`SECURITY.md`](../../SECURITY.md) and may block a release.

Any later stable line must publish its own policy and release-specific migration handoff before it
supersedes 1.0. A Git tag or locally rebuilt binary outside the declared release channels does not
create support status.

## Supported native targets

The exact machine authority is [`release/targets.json`](../../release/targets.json). The `1.0` line qualifies five
archives:

| Platform | Target | Archive |
|---|---|---|
| macOS arm64 | `aarch64-apple-darwin` | `.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` |
| Linux x86_64 glibc | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux x86_64 static musl | `x86_64-unknown-linux-musl` | `.tar.gz` |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `.zip` |

Linux arm64 is unsupported in `1.0`. A target triple means the published archive and its native release-matrix
runner are qualified; it does not imply support for every historical operating-system version, alternative libc,
emulator, compatibility layer, or modified executable. The installation guide explains archive selection and the
release-verification guide defines the required checksum, provenance, and signing checks.

The desktop application is distributed for Apple Silicon macOS during Alpha. Beta and later also
require exact-candidate Intel macOS GUI compilation evidence, but that compile-only record is not a
native Intel desktop runtime claim. Windows and Linux GUI packages remain outside the 1.0 support
line until separately implemented and qualified; the standalone CLI archives above remain the
cross-platform surface.

End users need no Python, Node.js, Java, external Typst executable, external SQLite installation, Rust toolchain, or
runtime package download. Building from source is a development path and is not equivalent to a supported archive.

## Contract support

| Surface | Supported `1.0` contract | Compatibility boundary |
|---|---|---|
| Agent protocol | `canisend.agent/v4` | Earlier Agent requests and Skills are unsupported and fail before mutation. |
| Public JSON Schema | `4.0.0` | The Beta freeze binds all public v2, Application v3, Agent v4, and Pack v1 schemas. |
| Host resources | `canisend.agent-host-resources/v4` | The installed binary verifies every manifest-owned Skill before use. |
| Workspace | `canisend.workspace/v4` | Alpha.7 initializes clean v4 authority; v2/v3 import and compatibility are outside 1.0. |

Machine-readable JSON envelopes, schemas, stable error codes, capability/context snapshots, and task validation are
the integration contract. Human-readable command prose, whitespace, progress text, and diagnostic wording are not a
machine API. Integrations must use `--json`, advertised capabilities, and generated schemas rather than scrape human
output.

Codex and Claude Code Skills are generated from one canonical Agent v4 resource source. Claude
Desktop can consume the same local stdio MCP server from its separate user-level configuration,
but does not consume the project-local Claude Code Skills. Use `host setup`, `host status`, and
`host remove` with the matching CanISend binary; never merge an earlier Skill layout into v4.
Generated resources exclude private Workspace bodies and host configuration is not silently
rewritten.

## Workspace support and rollback

Migrations 1 through 20 form the immutable Beta baseline. Any later `1.0` migration must be
contiguous and append-only. CanISend rejects earlier Workspace formats,
a future database schema, or incomplete migration history before mutation; it does not silently
import, repair, delete, or rewrite unsupported authority.

The future-schema check runs before connection configuration and returns a stable
`upgrade-required` application failure with the found/supported schema versions and the verified
pre-upgrade-backup recovery action.

There is no supported in-place downgrade. Before upgrading, check and back up every workspace. If an older binary
cannot accept a workspace opened by a newer binary, restore the verified pre-upgrade backup **restore into a new path**
and keep the upgraded workspace for diagnosis. The full procedure is in the
[upgrade, rollback, and uninstall guide](../guides/upgrade-and-rollback.md).

## Supported input and security boundary

The `1.0` line supports local text, text-extractable PDF, supplied URL/HTML, reviewed CSV/JSON,
the clean Workspace v4 CLI and MCP operations returned by `tools/list`, current Agent v4 host
resources, and the Apple Silicon macOS desktop workflow described above. Scanned/image-only PDFs
without extractable text, browser/portal automation, automatic application submission,
Windows/Linux desktop packages, and Linux arm64 archives are outside this support line.

External Codex, Claude Code, and Claude Desktop handoff is the primary reasoning surface. Codex
CLI is the required real-host qualification surface for Beta entry and must pass the canonical
Academic and Generic Pack scenarios on the exact checkpoint. Claude resources remain generated
and checked from the same Agent v4 source; Claude Code/Desktop real-host sessions are non-blocking
compatibility observations and are never reported as passed when skipped or unauthenticated.

Host credentials, conversations, provider entitlements, search, plugins, connectors, and
retention remain owned by those hosts. CanISend supplies body-free context and guarded tools but
does not promise that every host exposes identical capabilities.

CanISend has no default telemetry. Public issues must not contain private advert, profile, application, workspace,
provider, or credential content. Provider send and private export remain explicit consent boundaries; installation
or support status never grants a provider permission to receive data.
