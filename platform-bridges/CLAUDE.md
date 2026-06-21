# CanISend Claude Bridge

@agent-skills/canisend/SKILL.md

Start every workflow by checking the private workspace:

```bash
canisend doctor --workspace .
```

If running from outside the workspace, replace `.` with the workspace path.

Agent boundaries:

- Direct CLI deterministic mode can be local-only. Agent-assisted mode is not local-only: any file, PDF, webpage, or generated material you read or summarize may be processed by the agent model provider.
- LLM-backed CLI mode (`extract-profile-evidence --llm-augment`, `--llm-parser`, `--llm-drafts`, or command providers) may transmit selected private advert, profile, evidence, and draft context to the configured provider.
- Allowed by default: run local deterministic `canisend` commands, inspect generated evidence, and review current job metadata/artifacts needed for the task.
- Ask first: before reading full private CVs, statements, full job adverts, references, PDFs, source URLs, generated packages, or before enabling LLM-backed CLI flags/providers.
- Never do: submit applications, fill portals, create accounts, scrape full job pages, answer sensitive declarations, upload packages, or fabricate evidence.
- Do not quote private materials in chat unless the user explicitly asks.
- Do not stage private files such as `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, or real application packages.
- Prefer generated evidence from `profile/generated/`; report gaps rather than inventing claims.

For cross-platform details, read @agent-skills/canisend/references/platforms.md.
For privacy details, read @agent-skills/canisend/references/privacy.md.
