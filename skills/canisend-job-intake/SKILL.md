---
name: canisend-job-intake
description: Use when discovering or importing a job, merging local or host-agent results, converting a stable discovery lead into a CanISend job workspace, completing a lead-only advert, or validating source metadata before parsing.
---

# CanISend Job Intake

Focus only on moving from a job source or lead to one reviewable job folder with a complete advert. Stop before fit analysis or package drafting.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate source metadata, institutions, deadlines, advert text, criteria, or advert completeness.
- Ask before reading full private adverts, source URLs, PDFs, or other private files. State that agent-read content may be processed by the agent model provider.
- Treat RSS/Atom, Greenhouse, Lever, local export, email-alert, and host-agent search records as discovery candidates,
  never as complete adverts or application materials.
- Public adapters are read-only and identifier-only. Do not add credentials, arbitrary API URLs, application
  endpoints, adjacent-page crawling, background scheduling, account behavior, uploads, or form submission.
- Local CSV/JSON/EML/MBOX and host search imports must retain normalized published-job fields only. Raw email bodies,
  unknown vendor fields, private paths, provider/session identifiers, and credentials must not enter the catalog.
- Do not scrape arbitrary pages or bypass access controls. Use user-provided text, local files, or an explicitly approved supported URL fetch.

## Required References

Read only what the current intake requires:

- `../canisend/references/privacy.md`: source, advert, and git-safety boundaries.
- `../canisend/references/quality-gates.md`: full-advert and parsed-metadata gates.
- `../canisend/references/file-contracts.md`: lead and job-folder contracts.
- `../canisend/references/job-lifecycle.md`: intake states and next actions.

## Workflow

1. Identify the workspace and run or request `canisend doctor --workspace <private-workspace>`.
2. Choose the narrowest discovery input:
   - `discovery refresh --sources discovery-sources.yaml` for configured RSS/Atom or public Greenhouse/Lever boards;
   - `discovery import --input <csv|json|eml|mbox> --source-name <safe-label>` for a local saved export;
   - `discovery import-search --input <normalized-search.json>` for a strict
     `canisend.discovery-search/v1` host handoff; or
   - `fetch-job-feed` / `fetch-jobs-ac-uk` for the legacy-compatible feed workflow.
3. Apply only user-supplied include/exclude/source-preference terms. Review catalog counts, exclusions, match reasons,
   and aliases without treating rank as a decision to apply.
4. Let the user select a lead unless they already identified one. Prefer
   `new-job-from-lead --leads-file job_leads/catalog.json --lead-id <lead_id>` so selection survives reordering;
   retain `--lead-index` only for legacy list compatibility.
5. Create one folder with `new-job-from-lead`, or use `new-job` for manually supplied metadata or advert files.
6. If the folder contains only a lead description, obtain the full advert through user paste, a local `.md`, `.txt`,
   or `.pdf`, or an explicitly approved single URL returning HTML or PDF. Do not rely on the lead summary.
7. Verify title, institution, deadline, source URL, required documents, and the complete advert text. Leave unknown
   fields unknown.
8. Report the discovery/intake method, selected stable ID when present, job path, advert completeness, unresolved
   metadata, and the next safe command. Hand complete jobs to `$canisend-job-fit` or
   `$canisend-application-package`.
