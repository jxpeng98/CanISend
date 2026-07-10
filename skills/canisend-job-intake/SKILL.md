---
name: canisend-job-intake
description: Use when discovering or importing a job, converting an RSS or Atom lead into a CanISend job workspace, completing a lead-only advert, or validating source metadata before parsing.
---

# CanISend Job Intake

Focus only on moving from a job source or lead to one reviewable job folder with a complete advert. Stop before fit analysis or package drafting.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate source metadata, institutions, deadlines, advert text, criteria, or advert completeness.
- Ask before reading full private adverts, source URLs, PDFs, or other private files. State that agent-read content may be processed by the agent model provider.
- Treat RSS and Atom descriptions as discovery records, never as a complete advert.
- Do not scrape arbitrary pages or bypass access controls. Use user-provided text, local files, or an explicitly approved supported URL fetch.

## Required References

Read only what the current intake requires:

- `../canisend/references/privacy.md`: source, advert, and git-safety boundaries.
- `../canisend/references/quality-gates.md`: full-advert and parsed-metadata gates.
- `../canisend/references/file-contracts.md`: lead and job-folder contracts.
- `../canisend/references/job-lifecycle.md`: intake states and next actions.

## Workflow

1. Identify the workspace and run or request `canisend doctor --workspace <private-workspace>`.
2. For feeds, use `fetch-job-feed` for generic RSS or Atom sources, or `fetch-jobs-ac-uk` for the compatibility workflow. Apply only user-supplied filters.
3. Let the user select a lead unless they already identified one. Preserve source name, feed, publication date, and source URL.
4. Create one folder with `new-job-from-lead`, or use `new-job` for manually supplied metadata or advert files.
5. If the folder contains only a lead description, obtain the full advert through user paste, a local `.md`, `.txt`, or `.pdf`, or an explicitly approved single URL returning HTML or PDF. Do not rely on the lead summary.
6. Verify title, institution, deadline, source URL, required documents, and the complete advert text. Leave unknown fields unknown.
7. Report the intake method, job path, advert completeness, unresolved metadata, and the next safe command. Hand complete jobs to `$canisend-job-fit` or `$canisend-application-package`.
