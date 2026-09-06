# Agent Session Protocol Research

Date: 2026-09-04
Status: Revised after official Codex embedding-interface verification

## Decision

Use the official Codex App Server stdio JSON-RPC interface directly for the MVP agent session. Keep
MCP as CanISend's product-tool boundary. Do not add `codex-acp` merely to translate one Codex
protocol into another, and do not add a generic provider interface while there is only one provider.

The product owner selected a post-MVP generic ACP v1 stdio channel on 2026-09-04, with Claude Agent
as the first live qualification target. This begins only after the Codex App Server MVP is complete.

## Evidence that changed the earlier decision

The official [Codex App Server documentation](https://learn.chatgpt.com/docs/app-server) explicitly
positions `codex app-server` as the interface for rich clients that embed Codex. It covers
authentication, conversation history, approvals, and streamed agent events. The default transport is
newline-delimited JSON over stdio; WebSocket transport is experimental and unsupported.

The protocol already exposes the MVP lifecycle: `initialize`/`initialized`, `thread/start`,
`thread/resume`, `turn/start`, streamed notifications and server requests, and `turn/interrupt`. The
CLI can generate TypeScript definitions or JSON Schema for the installed protocol using
`codex app-server generate-ts` and `codex app-server generate-json-schema`.

The official [Codex authentication documentation](https://learn.chatgpt.com/docs/auth) says local
Codex clients support ChatGPT subscription sign-in and API-key sign-in. CanISend should reuse the
installed Codex client's sign-in state and surface an authentication-required state; it must not
collect, copy, log, or persist provider tokens.

Local verification on 2026-09-04 found `codex-cli 0.151.0` and a working
`codex app-server --help`, whose default listener is `stdio://`. The local help still labels App
Server experimental, so release support requires a tested version range, capability probe, schema
fixture, native smoke, and external-handoff rollback. This observation proves availability on the
development host, not the minimum supported version.

## Why direct App Server is smaller

The earlier plan selected ACP v1 and the ACP organization's `codex-acp` adapter to obtain a
provider-neutral boundary. With only Codex in the MVP, that adds another executable, distribution
decision, version matrix, translation layer, and failure mode while forwarding to App Server
underneath. Direct App Server provides the required lifecycle without that cost.

ACP remains a valid interoperability option, especially when integrating a second provider, but it
does not need to exist in the first implementation. Product workflows stay provider-independent by
normalizing the small UI event model in Rust and keeping Codex identifiers body-free; a shared
provider trait is added only when a second concrete implementation proves its shape.

The official [ACP Registry](https://agentclientprotocol.com/get-started/registry) currently includes
multiple native agents and wrappers, including Claude Agent, Gemini CLI, GitHub Copilot, Cursor,
Goose, and OpenCode. Registry presence is discovery evidence, not CanISend qualification: each Agent
still needs version, authentication, required-capability, permission, provenance, packaging, and
native-runtime checks.

Anthropic's official [Agent SDK documentation](https://code.claude.com/docs/en/agent-sdk/overview)
documents sessions, streaming, permissions, and MCP, and directs non-Python/TypeScript hosts to its
CLI subprocess interface. A Claude ACP wrapper can therefore be assessed as the first common-channel
implementation without embedding a Node or Python runtime in the Tauri process.

Harness's current
[Agent documentation](https://developer.harness.io/docs/platform/harness-ai/core-capabilities/in-your-pipelines/harness-agents-references/)
describes MCP connectors rather than an ACP Agent session surface. Treat Harness as an MCP consumer
or external integration until an embedded-session protocol is documented and qualified.
Cloud/pipeline access to a private local Workspace would additionally require a separately approved
remote transport and authentication boundary.

## App Server and MCP responsibility split

| Concern | Protocol/owner |
| --- | --- |
| Codex initialize/auth/thread/turn/update/interrupt | Codex App Server |
| Host file/command/network approval request | Codex App Server plus desktop policy |
| CanISend capabilities and domain operations | MCP |
| Mutation preview/approve/commit/verify | Existing CanISend application facade |
| Evidence, consent, audit, recovery, export truth | CanISend Rust core |
| Conversation history | Codex thread in the MVP |

Passing a Workspace/Application-bound CanISend MCP definition to Codex is the integration point.
Neither App Server nor ACP replaces the existing domain-tool boundary.

## MVP compatibility policy

- Resolve an explicitly configured `codex` path first, then reuse the desktop runtime's bounded
  installed-executable candidates. Never shell-interpolate a path.
- Record `codex --version`; require a release-tested range plus successful initialization and the
  exact methods needed by the MVP. Executable presence alone is not readiness.
- Generate and review a schema fixture from the tested CLI during development. Do not generate code
  or schemas on every App launch.
- Ignore unknown optional notifications only when correlation and required fields remain valid;
  reject invalid framing, oversized messages, unknown required requests, or missing required fields.
- Use a deterministic fake App Server for CI. Provider login and network access are not test
  prerequisites.
- Treat missing executable, authentication required, incompatible capability, process exit, and
  malformed output as actionable diagnostic states, not failed product mutations.
- Use stdio only. Do not bundle, download, update, or authenticate Codex on the user's behalf in the
  MVP.
