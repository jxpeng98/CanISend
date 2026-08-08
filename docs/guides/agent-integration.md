# Agent integration

CanISend owns validation, Pack identity, revisions, consent, storage, review, rendering, recovery,
and audit state. Codex, Claude Code, or another Agent host owns conversation and bounded semantic
reasoning. A host must never edit `.canisend`, SQLite, immutable Blobs, or managed projections.

## Clean v4 boundary

Alpha.7 host resources require `canisend.workspace/v4` and `canisend.agent/v4`. One Workspace can
contain academic, generic, or other Pack-bound Applications together. Select one exact Application
and preserve its Pack ID, Pack version, Pack digest, revision, and snapshot digest; never select a
Workspace mode.

Earlier Skills, protocol requests, command aliases, and host-resource layouts are unsupported.
CanISend detects those resources before installation and returns clean-install guidance without
modifying either the old files or authoritative Workspace state.

## Install the v4 host resources

The desktop initialization and Agent setup journey installs four version-matched Skills from one
embedded, integrity-checked source:

| Skill | Canonical Agent v4 tasks |
| --- | --- |
| `canisend-workspace` | orientation, Profile/Evidence, Application creation, recovery |
| `canisend-intake` | Source intake and Requirements |
| `canisend-materials` | fit/Plan and Deliverable drafting |
| `canisend-review-export` | review and local export |

Codex receives the Skills under `.agents/skills` with generated `agents/openai.yaml` metadata.
Claude Code receives the same `SKILL.md` bytes under `.claude/skills`. The host-specific manifest
is `.agents/canisend-agent-v4.json` or `.claude/canisend-agent-v4.json`; it binds the product,
protocol, Workspace format, task-model digest, every file path, and every file digest.

Install and update are idempotent within v4. CanISend replaces only bytes recorded by the current
manifest, refuses user-modified or unmanaged paths, and performs a complete digest preflight before
uninstalling. Host setup never writes inside `.canisend`.

## Connect the MCP adapter

The native binary serves MCP `2025-11-25` over stdio and does not require the App to be open:

```console
canisend --workspace /absolute/path/to/workspace mcp serve
```

Codex project configuration:

```toml
[mcp_servers.canisend]
command = "/absolute/path/to/canisend"
args = ["--workspace", "/absolute/path/to/workspace", "mcp", "serve"]
enabled = true
default_tools_approval_mode = "writes"
```

Claude Code project configuration:

```json
{
  "mcpServers": {
    "canisend": {
      "type": "stdio",
      "command": "/absolute/path/to/canisend",
      "args": ["--workspace", "/absolute/path/to/workspace", "mcp", "serve"]
    }
  }
}
```

List tools at the start of every session. The current clean-v4 MCP adapter exposes these body-free
read operations:

- `canisend_workspace_status`;
- `canisend_workspace_check`;
- `canisend_application_list`; and
- `canisend_application_show`.

Do not infer that an operation from the canonical registry is callable when it is absent from the
runtime tool list. Use a native CLI operation only when it exposes the same operation ID, or stop
with the missing capability as a blocker.

## Canonical task sequence

Orientation is read-only and follows `orient -> verify`. Every mutation follows:

```text
orient -> propose -> preview -> approve -> commit -> verify
```

The proposal and preview bind the exact Workspace, Application, Pack, revision, snapshot, schema,
operation, candidate digest, required consents, and expiry. Approval belongs to the user and binds
the exact preview digest. Commit consumes one opaque, process-bounded token. Expiry, replay, stale
revision, wrong Pack, denied consent, or host restart requires a fresh orientation and preview.

The canonical task model, operation registry, seven schemas, two examples, host guide, and Skills
ship in the Agent v4 export pack. The Codex pack has 20 files; Claude and generic-host packs have
16 because OpenAI UI metadata is Codex-specific.

## User-only boundaries

An Agent may propose Requirements, Evidence relationships, Plans, Deliverables, and review
findings. The user confirms sources and Evidence, chooses whether to proceed, grants private,
provider, network, and export consent, approves exact previews, reviews final artifacts, and
submits outside CanISend. No tool, Skill, receipt, export, or readiness state authorizes login,
upload, portal automation, or submission. Every export receipt confirms that
`submission_performed` is `false`.

See [Agent v4](../contracts/agent-v4.md) and
[Privacy and consent](privacy-and-consent.md).
