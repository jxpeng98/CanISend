# Platform Compatibility

Use this reference when setting up the same CanISend skill across different CLI and IDE agents.

## Portable Core

The portable contract is:

```text
agent-skills/canisend/SKILL.md
agent-skills/canisend/references/*.md
canisend doctor --workspace <private-workspace>
```

Any platform can use the skill by reading `SKILL.md`, then loading only the needed one-level reference files. The CLI remains the source of truth for workspace actions.

## Workspace Bridge Files

`canisend init-workspace` copies these bridge files into the workspace root:

```text
AGENTS.md
CLAUDE.md
GEMINI.md
```

They all point agents to `agent-skills/canisend/SKILL.md` and start with `canisend doctor --workspace <private-workspace>`.

Existing bridge files are not overwritten unless the user passes `--overwrite`.

## Codex And AGENTS.md

Codex and other AGENTS.md-aware agents should read root `AGENTS.md`. The bridge file summarizes privacy boundaries and points to the project skill. If a Codex runtime supports native skills, prefer invoking `$canisend`; otherwise read the skill folder directly.

## Claude Code And CLAUDE.md

Claude Code can use root `CLAUDE.md` as project memory. The bridge uses `@agent-skills/canisend/SKILL.md` import syntax so Claude can load the skill body and references through normal file access.

If imports are disabled or unavailable, ask Claude to read `agent-skills/canisend/SKILL.md` manually.

## Gemini CLI And GEMINI.md

Gemini CLI can use root `GEMINI.md` context files and import syntax. The bridge imports the skill and tells Gemini to use the workspace CLI rather than hidden state.

If a Gemini setup customizes context file names, configure it to include `GEMINI.md` or `AGENTS.md`.

## IDE Agents

For VS Code, Cursor, Zed, JetBrains, Copilot-style chat, and other IDE agents:

1. Open the private workspace as the project root.
2. Ask the agent to read `AGENTS.md`, `CLAUDE.md`, or `GEMINI.md` if the IDE does not load one automatically.
3. Ask the agent to use `agent-skills/canisend/SKILL.md`.
4. Keep all actions routed through `canisend --workspace <private-workspace>`.

## Compatibility Checklist

- `canisend doctor --workspace <private-workspace>` reports workspace readiness.
- Root bridge file exists for the target platform.
- The bridge points to `agent-skills/canisend/SKILL.md`.
- The agent understands private paths: `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs.
- The agent reads `quality-gates.md` before presenting generated materials as ready.
