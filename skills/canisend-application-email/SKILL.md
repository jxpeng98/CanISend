---
name: canisend-application-email
description: Use when drafting or revising job application emails, follow-ups, referee requests, materials handoff notes, or polite hiring-process messages using CanISend context.
---

# CanISend Application Email

Focus only on short job-application communication around the materials. Do not draft or submit portal answers unless the user explicitly asks for another CanISend workflow.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, relationships, promises, availability, referee agreement, institutional contacts, or hiring-process facts.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, PDFs, or enabling LLM-backed commands.
- Prefer generated job metadata, application package paths, user-provided recipient context, and explicit instructions over raw private files.
- Keep emails truthful, brief, and scoped to what the user has actually prepared or confirmed.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: readiness gates before implying materials are final.
- `../canisend/references/file-contracts.md`: generated package and review artifact paths.
- `../canisend/references/workflow.md`: manual submission boundary.

## Workflow

1. Identify the communication type: application email, follow-up, referee request, materials handoff, clarification request, or thank-you note.
2. Confirm recipient, relationship, desired tone, deadline, and what attachments or materials are actually ready.
3. Prefer `parsed_job.json`, review checklists, package paths, and user-provided recipient context.
4. State the purpose in the first sentence and keep the body specific.
5. Avoid overexplaining qualifications unless the email is part of the application itself.
6. Do not claim submission, attachment, referee consent, or availability unless the user has confirmed it.
7. Before saying materials can be sent, check `../canisend/references/quality-gates.md`.

## Output

For simple emails, output only the email.

For uncertain or sensitive messages, include:

- Draft email
- Assumptions to confirm
- Attachment or readiness notes
