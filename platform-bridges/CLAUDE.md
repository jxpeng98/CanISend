# CanISend Claude Bridge

@agent-skills/canisend/SKILL.md

Start every workflow by checking the private workspace:

```bash
canisend doctor --workspace .
```

If running from outside the workspace, replace `.` with the workspace path.

Agent boundaries:

- Allowed by default: run local `canisend` commands, inspect generated evidence, and review job artifacts needed for the current task.
- Ask first: before reading full private CVs, statements, full job adverts, references, source URLs, or before enabling LLM-backed flags.
- Never do: submit applications, fill portals, create accounts, scrape full job pages, answer sensitive declarations, upload packages, or fabricate evidence.
- Do not quote private materials in chat unless the user explicitly asks.
- Do not stage private files such as `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, or real application packages.

For cross-platform details, read @agent-skills/canisend/references/platforms.md.
