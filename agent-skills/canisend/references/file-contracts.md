# File Contracts

Use this reference when reading, writing, or validating project files.

## Workspace Root

User workspaces are initialized with `canisend init-workspace --workspace <private-workspace>` and contain:

```text
canisend.yaml
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

CLI commands read `canisend.yaml` from `--workspace`; configured relative paths are resolved inside that workspace so agents can run from any current directory.

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
00_preparation_questions.md
01_job_summary.md
02_fit_report.md
03_cover_letter_draft.md
04_cv_tailoring_notes.md
05_criteria_checklist.md
06_final_application_package.md
07_material_review_checklist.md
typst/
  cover_letter.typ
  application_package.typ
```

RSS and Atom lead outputs live in ignored `job_leads/`.

## Output Contracts

- `job.yaml`: lightweight tracking fields, including `title`, `institution`, `deadline`, `source_url`, `status`, `english_variant`, `writing_style`, `created_at`, and `updated_at`.
- `job_advert.md`: full advert text. Feed-created jobs start with lead metadata and require manual full advert paste or
  an explicit one-URL import.
- `parsed_job.json`: structured advert data. Missing fields should remain empty or unknown; do not invent.
- `00_preparation_questions.md`: grill-me checklist for confirming US English vs UK English, writing style, specific motivation, emphasis, risks, and excluded details before treating materials as final.
- `02_fit_report.md`, `03_cover_letter_draft.md`, `04_cv_tailoring_notes.md`, `05_criteria_checklist.md`: evidence-grounded Markdown review artifacts.
- `07_material_review_checklist.md`: management artifact for cover letter draft, CV tailoring notes, placeholders, item-level citations, and manual follow-up actions.
- `typst/cover_letter.typ`: editable Typst source for the final cover letter, with stable `// CANISEND: section ...` markers.
- `typst/application_package.typ`: editable Typst source for the final package, including remaining actions and review sections.
- `typst/.canisend-generated.json`: generated-hash metadata used to avoid overwriting user-edited Typst files.
- `typst/*.generated.typ`: candidate regeneration written only when the corresponding editable `.typ` has diverged
  from its generated baseline.
- `application_gate_report.json`: optional machine-readable `APP-Q*` report written only by an explicit
  `check-package --write-report` request.

The pipeline may emit content JSON compatibility/debug artifacts under `typst/`, but agents should treat the `.typ` files as the editing contract.
