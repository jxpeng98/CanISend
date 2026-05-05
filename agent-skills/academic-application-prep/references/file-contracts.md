# File Contracts

Use this reference when reading, writing, or validating project files.

## Workspace Root

User workspaces are initialized with `academic-prep init-workspace --workspace <private-workspace>` and contain:

```text
academic-prep.yaml
.env.example
.gitignore
profile/
jobs/
job_leads/
prompts/
templates/
schemas/
agent-skills/
```

CLI commands read `academic-prep.yaml` from `--workspace`; configured relative paths are resolved inside that workspace so agents can run from any current directory.

Default config keys:

```yaml
profile_dir: profile
jobs_dir: jobs
job_leads_dir: job_leads
prompt_dir: prompts
template_dir: templates
schema_dir: schemas
agent_skills_dir: agent-skills
```

## Resource Overrides

Application prompts live in `prompts/`. Workspace-local prompts override packaged defaults; missing prompt files fall back to packaged copies.

Agent-readable project skills live in `agent-skills/`. Workspace-local skills are copied defaults that can be edited by the user.

Project-managed Typst templates live in `templates/typst/`. Job-specific generated Typst sources live under each job folder.

## Private Profile

Private local profile data lives in ignored `profile/`.

Expected Typst-first profile files:

```text
profile/profile.yaml
profile/typst/cv.typ
profile/typst/cover_letter_base.typ
profile/typst/research_statement.typ
profile/typst/teaching_statement.typ
profile/generated/*.evidence.md
```

Generated evidence citations use `profile/generated/file.evidence.md#Section`.

Generated evidence items should have stable local IDs:

```markdown
## Teaching

- [cv-001] `job`: position: Teaching Assistant, institution: University X
```

New materials should cite item-level evidence as `profile/generated/file.evidence.md#Section/item-id`, for example `profile/generated/cv.evidence.md#Teaching/cv-001`. Section-level citations remain a compatibility fallback, not the preferred new output.

## Job Folder

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

## Output Contracts

- `job.yaml`: lightweight tracking fields, including `title`, `institution`, `deadline`, `source_url`, `status`, `created_at`, and `updated_at`.
- `job_advert.md`: full advert text. RSS-created jobs start with lead metadata and require manual full advert paste.
- `parsed_job.json`: structured advert data. Missing fields should remain empty or unknown; do not invent.
- `02_fit_report.md`, `03_cover_letter_draft.md`, `04_cv_tailoring_notes.md`, `05_criteria_checklist.md`: evidence-grounded Markdown review artifacts.
- `typst/cover_letter_content.json`: structured content consumed by `cover_letter.typ`.
- `typst/application_package_content.json`: structured content consumed by `application_package.typ`.
