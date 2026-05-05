# File Contracts

Application prompts live in `prompts/`.

Codex-readable project skills live in `agent-skills/`.

Private local profile data lives in ignored `profile/`.

Each application task lives in ignored `jobs/<job-slug>/` and contains:

```text
job.yaml
job_advert.md
parsed_job.json
01_job_summary.md
02_fit_report.md
03_cover_letter_draft.md
04_cv_tailoring_notes.md
05_criteria_checklist.md
06_final_application_package.md
typst/
  cover_letter_content.json
  cover_letter.typ
  application_package_content.json
  application_package.typ
```

RSS lead outputs live in ignored `job_leads/`.
