# Academic Application Preparation Copilot

Local-first CLI tooling for preparing academic job application materials from a job advert and a Markdown-based academic profile.

This project prepares materials only. It does not submit applications, create university accounts, fill web forms, or answer sensitive declarations.

See `academic_application_prep_copilot_proposal.md` for the V1 engineering proposal.

## Complete Workflow

### 1. Install and verify the CLI

From the repository root:

```bash
uv run academic-prep --help
uv run pytest -v
```

During development, prefer `uv run academic-prep ...`. If the package is installed into an environment, the same commands are available as `academic-prep ...`.

### 2. Prepare local private profile data

Create starter profile files. The default mode is `hybrid`, which creates both Markdown evidence files and Typst-first profile sources:

```bash
uv run academic-prep init-profile
```

This creates local Markdown/YAML evidence files under `profile/`:

```text
profile/
  profile.yaml
  cv.md
  publications.md
  teaching_experience.md
  research_statement.md
  teaching_statement.md
  service_leadership.md
  grants_awards.md
  references.md
  personal_profile.yaml
  typst/
    cv.typ
    cover_letter_base.typ
    research_statement.typ
    teaching_statement.typ
  generated/
    .gitkeep
```

If you already maintain your CV and statements in Typst using `modernpro-cv` and `modernpro-coverletter`, initialize only the Typst profile scaffold:

```bash
uv run academic-prep init-profile --mode typst
```

Fill these files with your private academic profile. Typst can be the human-facing source format, but the matcher/checker should read normalized evidence from `profile/generated/`. The local `profile/profile.yaml` manifest records which Typst files correspond to CV, cover letter base, research statement, teaching statement, and generated evidence outputs.

### 3. Generate normalized profile evidence

Generate Markdown evidence files from the local profile manifest and Typst sources:

```bash
uv run academic-prep extract-profile-evidence
```

This reads `profile/profile.yaml`, extracts supported evidence from `profile/typst/*.typ`, and writes normalized files under `profile/generated/`.

Current extraction support is intentionally conservative:

- `#section("...")`
- Typst headings such as `= Research Statement`
- `#education(...)`
- `#job(...)`
- `#award(...)`
- publication references such as `+ @paper2025`

Run this again whenever the private Typst profile sources change.

### 4. Fetch jobs.ac.uk RSS leads

Open the jobs.ac.uk RSS index and copy a raw RSS Feed link from one of:

- `https://www.jobs.ac.uk/feeds/subject-areas`
- `https://www.jobs.ac.uk/feeds/locations`
- `https://www.jobs.ac.uk/feeds/type-roles`

Fetch and filter leads locally:

```bash
uv run academic-prep fetch-jobs-ac-uk \
  --feed-url "https://www.jobs.ac.uk/path/to/raw/rss/feed" \
  --include economics \
  --include finance \
  --exclude phd \
  --output job_leads/jobs_ac_uk.json
```

Filtering is local and keyword-based. V1 does not scrape individual job pages. Review `job_leads/jobs_ac_uk.json`, choose a role, then copy the advert text manually from the source page.

For offline testing, use a saved RSS XML file:

```bash
uv run academic-prep fetch-jobs-ac-uk \
  --rss-file samples/jobs_ac_uk.xml \
  --include lecturer \
  --output job_leads/jobs_ac_uk.json
```

### 5. Select one advert and create a job workspace

Create one job folder per application preparation task:

```bash
uv run academic-prep new-job \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --deadline "2026-06-15" \
  --source-url "https://www.jobs.ac.uk/job/example"
```

This creates:

```text
jobs/2026-06-15_university-x_lecturer-in-economics/
  job.yaml
  job_advert.md
```

Paste the selected advert into `job_advert.md`, or import a local Markdown/TXT advert:

```bash
uv run academic-prep new-job \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --deadline "2026-06-15" \
  --advert-file path/to/job_advert.md
```

### 6. Run the application preparation pipeline

Run the local pipeline for the selected job:

```bash
uv run academic-prep run --job jobs/2026-06-15_university-x_lecturer-in-economics
```

To use a non-default profile folder:

```bash
uv run academic-prep run \
  --job jobs/2026-06-15_university-x_lecturer-in-economics \
  --profile-dir path/to/profile
```

Current V1 foundation output:

```text
jobs/<job-slug>/
  parsed_job.json
  01_job_summary.md
  02_fit_report.md
  03_cover_letter_draft.md
  04_cv_tailoring_notes.md
  05_criteria_checklist.md
  06_final_application_package.md
  typst/
    cover_letter.typ
    application_package.typ
```

The current parser/generator is deterministic and scaffold-level. Round 2 replaces this with LLM-backed parser and generation steps while preserving the same file contracts.

### 7. Review and edit generated materials

Review outputs in this order:

1. `parsed_job.json`: confirm title, institution, criteria, required documents, and fields.
2. `05_criteria_checklist.md`: check every essential criterion.
3. `02_fit_report.md`: identify gaps and unsupported claims.
4. `03_cover_letter_draft.md`: edit claims and tone manually.
5. `04_cv_tailoring_notes.md`: apply edits to your private CV source.
6. `06_final_application_package.md`: use as the final preparation dossier.

Generated material is draft-only. Any claim about publications, teaching, service, awards, grants, or supervision must be supported by `profile/` evidence.

### 8. Render Typst outputs when needed

The project uses public Typst Universe templates:

- `@preview/modernpro-cv:1.3.0`
- `@preview/modernpro-coverletter:0.0.8`

Generate PDF outputs only when needed:

```bash
uv run academic-prep render-typst --job jobs/2026-06-15_university-x_lecturer-in-economics
```

This requires a local `typst` binary. Source generation does not require Typst; only PDF rendering does.

### 9. Submit manually outside the tool

Before submitting:

- Confirm the university portal's required documents.
- Manually review right-to-work, visa, disability, equality, criminal record, and other sensitive declarations.
- Upload final files yourself.
- Update `job.yaml` status manually if you want local tracking.

The tool prepares application materials. It does not submit anything.

## Privacy Defaults

This repository is intended to be open source. Personal application data should stay local:

- `profile/ is ignored by git` except for `.gitkeep`.
- `jobs/` generated job folders are ignored by git.
- `job_leads/` RSS outputs are ignored by git.
- API keys belong in local environment variables or `.env`, which is ignored by git.
- Do not commit real CVs, statements, references, job applications, generated PDFs, or source URLs that reveal private application strategy.

## Typst Templates

Project templates live under `templates/typst/` and import the public modernpro packages. Job-specific generated Typst files are written under each ignored `jobs/<job-slug>/typst/` folder.

The intended direction is:

- Use `modernpro-cv` for CV-style sources and later CV tailoring exports.
- Use `modernpro-coverletter` for cover letters and statement/application package style outputs.
- Keep personal content in ignored local folders.

## Project Skills and Prompts

This repository separates application prompts from agent-readable skills:

- `prompts/` contains LLM prompt files used by the application pipeline.
- `agent-skills/` contains standard `SKILL.md` directories that Codex, Claude Code, Gemini, or another agent can read as project guidance.

The main project skill is:

```text
agent-skills/academic-application-prep/
  SKILL.md
  references/
    workflow.md
    file-contracts.md
    typst-profile.md
    privacy.md
```

Agents should load this skill when working on academic application preparation, file contracts, Typst-first profile handling, or privacy-sensitive generated materials.

## Round 2 Task Queue

Round 2 should turn the current scaffold into a useful evidence-grounded preparation pipeline.

1. **LLM-backed parser**
   - Replace deterministic advert parsing with provider-based parsing.
   - Keep `parsed_job.json` schema stable.
   - Add schema validation and clear fallback errors.

2. **Profile evidence index and evidence citation**
   - Parse `profile/*.md` headings and bullet items.
   - Parse Typst-first profile sources under `profile/typst/`.
   - Build file + section/item evidence references.
   - Use evidence citation in fit reports, criteria checks, and drafts.

3. **LLM-backed profile matcher**
   - Generate `02_fit_report.md` from `parsed_job.json` and the evidence index.
   - Require explicit evidence citation for every strong-fit claim.
   - Mark missing/weak evidence instead of inventing content.

4. **Criteria coverage checker**
   - Generate `05_criteria_checklist.md` with `strong`, `partial`, `weak`, or `missing` coverage.
   - Separate essential and desirable criteria.
   - Include risk level and suggested improvement for each criterion.

5. **Cover letter and CV tailoring generation**
   - Generate evidence-grounded `03_cover_letter_draft.md`.
   - Generate actionable `04_cv_tailoring_notes.md`.
   - Keep placeholders only where manual judgement is required.

6. **jobs.ac.uk lead-to-job workflow**
   - Add a command to inspect RSS lead JSON and initialize a job workspace from a selected lead.
   - Keep advert text entry manual unless a later scope explicitly allows page scraping.

7. **Typst data mapping**
   - Map generated cover letter content into modernpro-coverletter fields more cleanly.
   - Add local-only data files for personal Typst variables if needed.
   - Keep private applicant details ignored by git.

8. **Configuration hardening**
   - Document OpenAI-compatible API settings.
   - Document local command provider usage for Codex/Gemini/Claude-style CLI access.
   - Add validation for missing provider config before LLM-backed steps run.

Suggested implementation order: parser -> evidence index -> matcher -> criteria checker -> material generation -> lead-to-job helper -> Typst mapping.
