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

## Do Not Read Unless Needed

Prefer generated evidence and metadata over raw private source files. Read full CVs, statements, references, full job adverts, source URLs, or generated application packages only when the task cannot be completed from `profile/generated/`, `job.yaml`, `parsed_job.json`, or the existing review artifacts.

Ask first before reading private materials if the user asked for general workflow help, release work, repo maintenance, or another task that does not require private content.

## Sensitive Actions Agents Must Not Do

- Do not submit an application.
- Do not create or log in to university portal accounts.
- Do not fill equality, disability, visa, right-to-work, criminal record, conflict, health, or other sensitive declarations.
- Do not scrape full job pages unless the project scope changes and the user explicitly approves it.
- Do not upload generated PDFs or application packages anywhere.

## Do Not Quote In Chat

Do not quote private materials unless the user explicitly asks. This includes full CV sections, full job adverts, cover letters, statement paragraphs, names, emails, phone numbers, reference details, source URLs, and institution-specific application strategy.

Use narrow summaries such as "the advert asks for econometrics teaching" or "the evidence file has two teaching items" when that is enough.

## Before Staging Or Commit

Run a local privacy check:

```bash
git status --short
git diff --cached --name-only
```

Only stage source code, tests, docs, prompts, templates, schemas, examples, and project skill files. If `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, or real institution-specific strategy files appear, stop and ask the user.

## Do Not Stage Or Commit

Do not stage or commit real CVs, statements, references, full job adverts, generated packages, rendered PDFs, `.env`, API keys, private source URLs, or files that reveal application strategy. If these appear in `git status --short`, leave them untouched and report the risk.
