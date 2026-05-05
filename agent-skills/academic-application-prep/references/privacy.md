# Privacy Rules

Use this reference before reading, writing, staging, committing, or quoting user application data.

## Private By Default

Do not commit real applicant data.

Ignored private paths:

- `profile/`
- `jobs/`
- `job_leads/`
- `.env`

Safe-to-commit paths:

- `prompts/`
- `agent-skills/`
- `templates/`
- `schemas/`
- tests and source code

Never fabricate applicant evidence. Missing evidence should be reported as a gap.

## Sensitive Actions Agents Must Not Do

- Do not submit an application.
- Do not create or log in to university portal accounts.
- Do not fill equality, disability, visa, right-to-work, criminal record, conflict, health, or other sensitive declarations.
- Do not scrape full job pages unless the project scope changes and the user explicitly approves it.
- Do not upload generated PDFs or application packages anywhere.

## Before Staging Or Commit

Run a local privacy check:

```bash
git status --short
git diff --cached --name-only
```

Only stage source code, tests, docs, prompts, templates, schemas, examples, and project skill files. If `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, or real institution-specific strategy files appear, stop and ask the user.

## Quoting And Summaries

When discussing private materials in chat, summarize narrowly. Avoid pasting full CV sections, full job adverts, full cover letters, names, emails, phone numbers, reference details, or source URLs unless the user explicitly asks.
