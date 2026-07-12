# Platform Compatibility

Use this reference when setting up the same CanISend skill across different CLI and IDE agents.

## Portable Core

The portable contract is:

```text
agent-skills/canisend/SKILL.md
agent-skills/canisend/references/*.md
agent-skills/canisend-*/SKILL.md
canisend agent context --workspace <private-workspace> --format json
```

Any platform can use the skill by reading `SKILL.md`, then loading only the needed one-level reference files. The CLI remains the source of truth for workspace actions.

## Mode Transparency

All platforms must distinguish direct CLI work from agent-assisted work:

- Direct CLI deterministic mode can be local-only when no LLM flags or providers are used.
- Agent-assisted mode is not local-only; content read by Codex, Claude Code, or another AI agent may be processed by that agent's model provider.
- LLM-backed CLI mode sends selected context to the configured provider or command when explicit flags are enabled.

Bridge files should make this distinction visible before an agent reads full private materials.

## Workspace Bridge Files

`canisend init-workspace` copies these bridge files into the workspace root:

```text
AGENTS.md
CLAUDE.md
```

They all point agents to `agent-skills/canisend/SKILL.md` and start with `canisend agent context --workspace <private-workspace> --format json`.

Existing bridge files are not overwritten unless the user passes `--overwrite`.

## Codex And AGENTS.md

Codex and other AGENTS.md-aware agents should read root `AGENTS.md`. The bridge file summarizes mode boundaries and points to the project skill. If a Codex runtime supports native skills, prefer invoking `$canisend`; otherwise read the skill folder directly.

## Claude Code And CLAUDE.md

Claude Code can use root `CLAUDE.md` as project memory. The bridge uses `@agent-skills/canisend/SKILL.md` import syntax so Claude can load the skill body and references through normal file access.

If imports are disabled or unavailable, ask Claude to read `agent-skills/canisend/SKILL.md` manually.

## IDE Agents

For VS Code, Cursor, Zed, JetBrains, Copilot-style chat, and other IDE agents:

1. Open the private workspace as the project root.
2. Ask the agent to read `AGENTS.md` if the IDE does not load one automatically.
3. Ask the agent to use `agent-skills/canisend/SKILL.md`.
4. Keep all actions routed through `canisend --workspace <private-workspace>`.

## Compatibility Checklist

- `canisend agent capabilities --format json` reports the supported protocol and operations.
- `canisend agent context --workspace <private-workspace> --format json` reports safe workspace state.
- `canisend stage status --workspace <private-workspace> --job jobs/<job-slug> --format json` reconstructs current
  Parse, Confirm, Evidence, Match, and Brief-plan state from durable job files.
- `canisend corrections status`, `canisend decision status`, and `canisend brief status` reconstruct user-owned state
  without returning private bodies; agents use their init/update/recover operations rather than writing YAML directly.
- Agents ask before reading Tier 2 `application_brief.yaml` or `required_document_plan.json`; body-free status is the
  default cross-platform handoff.
- `canisend doctor --workspace <private-workspace>` remains available for human-readable diagnostics.
- Root bridge file exists for the target platform.
- The bridge points to `agent-skills/canisend/SKILL.md`.
- The agent understands private paths: `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs.
- The agent understands that agent-read private content may enter the agent model context.
- The agent reads `quality-gates.md` before presenting generated materials as ready.

## Fresh-Session Handoff

A new shell-capable host resumes from workspace files, not a previous chat transcript. Host A and host B should run
the same `agent context` command with the workspace and optional job path. Apart from dynamic `request_id` values, an
unchanged workspace yields the same semantic context, artifact hashes, blockers, consents, and next actions.

The accepted shell contract covers Codex CLI/App sessions with command access, Claude Code, and IDE shell agents.
Parse supports deterministic and approved host-agent execution; Confirm, Evidence, Match, and Brief planning are
deterministic-only.
Evidence materializes its cross-directory profile inputs into a job-local immutable snapshot, so TaskSpec v1 remains
truthful on every host. Its safe-read implementation uses descriptor-relative protection where available and a
portable pre/post identity fallback where it is not.

No platform adapter, SDK, MCP transport, hosted service, or second provider is required for these slices. A fresh
host can run `extract-profile-evidence`, deterministic Evidence/Parse/Confirm/Match/Brief, and the same
status/scoped-patch corrections, Decision, and Brief operations through the CLI. Match output is proposed review
data; only explicit user-owned updates record apply/hold/skip and Brief choices.

Codex, Claude Code, and IDE agents must create one bounded strict patch in safe scratch space and pass it to the
Agent operation with the current revision/hash and explicit consent. They must not overwrite or normalize
`confirmed_corrections.yaml`, `application_decision.yaml`, or `application_brief.yaml`, and must never edit
`required_document_plan.json`. One correction requires one subsequent Confirm rerun before the next correction.
Brief requires a current confirmed apply Decision; empty requirements are not `confirmed_empty`, and unresolved,
required/omit, missing-action, or orphan blockers remain visible. Private patch/YAML/plan/candidate bodies are Tier 2;
receipts and AgentResponse remain body-free. CAS assumes a stable local job directory and cooperative CanISend
writers, so avoid concurrent manual editor saves. Task 6 is locally accepted; Task 7 and Stage 2 remain open.
