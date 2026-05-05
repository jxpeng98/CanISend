# Academic Application Preparation Copilot

Local-first CLI tooling for preparing academic job application materials from a job advert and a private academic profile.

This project prepares materials only. It does not submit applications, create university accounts, fill web forms, or answer sensitive declarations.

See `academic_application_prep_copilot_proposal.md` for the V1 engineering proposal.

## Installation Model

Normal users should install the package and create a separate private workspace. They do not need to fork this repository.

```bash
uv tool install academic-application-prep
academic-prep init-workspace --workspace ~/AcademicApplications
academic-prep doctor --workspace ~/AcademicApplications
```

The workspace contains private profile data, job leads, job folders, editable prompt copies, Typst templates, schemas, and agent-readable skills:

```text
~/AcademicApplications/
  academic-prep.yaml
  .env.example
  .gitignore
  AGENTS.md
  CLAUDE.md
  GEMINI.md
  profile/
  jobs/
  job_leads/
  prompts/
  templates/
  schemas/
  agent-skills/
```

The package keeps built-in defaults for prompts, schemas, templates, examples, agent skills, and platform bridge files. If a workspace-local default file is missing, the CLI falls back to the packaged copy. If a local file exists, it is treated as the user's editable override.

To update:

```bash
uv tool upgrade academic-application-prep
academic-prep update-workspace --workspace ~/AcademicApplications
academic-prep doctor --workspace ~/AcademicApplications
```

`update-workspace` preserves local prompt/template/skill edits by default. Use `--overwrite` only when you intentionally want to replace local default-resource copies with the package version.

`AGENTS.md`, `CLAUDE.md`, and `GEMINI.md` are lightweight bridge files for Codex-style agents, Claude Code, Gemini CLI, and IDE agents. They all point to `agent-skills/academic-application-prep/SKILL.md` so the same workflow is usable across platforms.

Developers who want to change the tool itself should fork or clone the repository and use `uv run academic-prep ...`.

## Release And Update Workflow

Releases are designed for normal users to consume as an installed CLI, not as a forked repository.

Maintainer release checks:

```bash
uv run pytest -v
uv build
uv run python -m academic_prep.package_check dist/*.whl
```

The package check verifies that runtime resources are present in the wheel, including prompts, Typst templates, schemas, examples, `.env.example`, and `agent-skills/`.

CI runs the same test/build/resource-check sequence on pushes and pull requests. The release workflow builds distributions once, checks packaged resources, and publishes through PyPI Trusted Publishing with OIDC:

- Manual `workflow_dispatch` with `publish_target=testpypi` publishes to TestPyPI.
- A published GitHub Release publishes to PyPI.
- The workflow uses `pypa/gh-action-pypi-publish@release/v1`; no PyPI API token should be stored in the repository.

Before the first publish, configure Trusted Publishing on TestPyPI and PyPI for this repository and the `.github/workflows/release.yml` workflow. Use GitHub environments named `testpypi` and `pypi` so releases can require manual approval if desired.

Version updates should change both:

- `pyproject.toml` project version
- `src/academic_prep/__init__.py` `__version__`

After upgrading, users should refresh default workspace resources without overwriting local edits:

```bash
uv tool upgrade academic-application-prep
academic-prep update-workspace --workspace ~/AcademicApplications
academic-prep doctor --workspace ~/AcademicApplications
```

## Example

The repository includes a fully local, fake-data workflow under `examples/end_to_end/`. It demonstrates:

- jobs.ac.uk RSS lead import
- `new-job-from-lead`
- Typst-first profile evidence extraction
- command-provider LLM parser and draft generation
- structured `modernpro-coverletter` outputs via `cover_letter_content.json`

Run or inspect `examples/end_to_end/README.md` before adapting the workflow to private profile and job data.

## Complete Workflow

### 1. Install and verify the CLI

For normal use:

```bash
uv tool install academic-application-prep
academic-prep --help
```

From a development checkout:

```bash
uv run academic-prep --help
uv run pytest -v
```

During development, prefer `uv run academic-prep ...`. If the package is installed into an environment, the same commands are available as `academic-prep ...`.

### 2. Initialize a private workspace

Create a user workspace. The default profile mode is Typst-first because the intended workflow starts from an already-written `modernpro-cv` CV and `modernpro-coverletter` cover letter or statements:

```bash
academic-prep init-workspace --workspace ~/AcademicApplications
```

Check local readiness:

```bash
academic-prep doctor --workspace ~/AcademicApplications
```

The CLI reads `academic-prep.yaml` from `--workspace` and resolves configured relative paths inside that workspace. The examples below keep `--workspace ~/AcademicApplications` so they can run from any current directory. If you `cd ~/AcademicApplications`, you can omit `--workspace`.

### 3. Prepare local private profile data

Create starter profile files. The default mode is `hybrid`, which creates both Markdown evidence files and Typst-first profile sources:

```bash
academic-prep init-profile --workspace ~/AcademicApplications --mode hybrid
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
academic-prep init-profile --workspace ~/AcademicApplications --mode typst
```

Fill these files with your private academic profile. In normal use, `profile/typst/cv.typ`, `profile/typst/research_statement.typ`, `profile/typst/teaching_statement.typ`, and `profile/typst/cover_letter_base.typ` should be your already-written modernpro-based sources. Typst can be the human-facing source format, but the matcher/checker should read normalized evidence from `profile/generated/`. The local `profile/profile.yaml` manifest records which Typst files correspond to CV, cover letter base, research statement, teaching statement, and generated evidence outputs.

### 4. Generate normalized profile evidence

Generate Markdown evidence files from the local profile manifest and Typst sources:

```bash
academic-prep extract-profile-evidence --workspace ~/AcademicApplications
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

### 5. Fetch jobs.ac.uk RSS leads

Open the jobs.ac.uk RSS index and copy a raw RSS Feed link from one of:

- `https://www.jobs.ac.uk/feeds/subject-areas`
- `https://www.jobs.ac.uk/feeds/locations`
- `https://www.jobs.ac.uk/feeds/type-roles`

Fetch and filter leads locally:

```bash
academic-prep fetch-jobs-ac-uk \
  --workspace ~/AcademicApplications \
  --feed-url "https://www.jobs.ac.uk/path/to/raw/rss/feed" \
  --include economics \
  --include finance \
  --exclude phd
```

Filtering is local and keyword-based. V1 does not scrape individual job pages. Review `job_leads/jobs_ac_uk.json`, choose a role, then copy the advert text manually from the source page.

For offline testing, use a saved RSS XML file:

```bash
academic-prep fetch-jobs-ac-uk \
  --workspace ~/AcademicApplications \
  --rss-file samples/jobs_ac_uk.xml \
  --include lecturer
```

### 6. Select one advert and create a job workspace

Create one job folder per application preparation task. If the role came from `job_leads/jobs_ac_uk.json`, initialize the folder from the selected zero-based lead index:

```bash
academic-prep new-job-from-lead \
  --workspace ~/AcademicApplications \
  --lead-index 0 \
  --institution "University X" \
  --deadline "2026-06-15"
```

This writes the RSS title, source URL, published date, and RSS description into `job_advert.md` with a clear `RSS lead only` notice. It does not fetch or scrape the full advert page; paste the complete advert manually before relying on parsed criteria or generated drafts.

You can also create a job manually:

```bash
academic-prep new-job \
  --workspace ~/AcademicApplications \
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
academic-prep new-job \
  --workspace ~/AcademicApplications \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --deadline "2026-06-15" \
  --advert-file path/to/job_advert.md
```

### 7. Run the application preparation pipeline

Run the local pipeline for the selected job:

```bash
academic-prep run \
  --workspace ~/AcademicApplications \
  --job jobs/2026-06-15_university-x_lecturer-in-economics
```

This default run uses the deterministic local parser. To use the LLM-backed job parser, opt in explicitly and configure a provider:

```bash
ACADEMIC_PREP_LLM_PROVIDER=openai-compatible
OPENAI_API_KEY=...
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=...

academic-prep run \
  --workspace ~/AcademicApplications \
  --job jobs/2026-06-15_university-x_lecturer-in-economics \
  --llm-parser
```

For local CLI model access, use the generic command provider instead of hardcoding a vendor adapter:

```bash
ACADEMIC_PREP_LLM_PROVIDER=command
ACADEMIC_PREP_LLM_COMMAND="codex exec --json"
ACADEMIC_PREP_LLM_TIMEOUT_SECONDS=300

academic-prep run \
  --workspace ~/AcademicApplications \
  --job jobs/2026-06-15_university-x_lecturer-in-economics \
  --llm-parser
```

To keep deterministic parsing but use provider-backed evidence-grounded drafts, opt in with `--llm-drafts`:

```bash
academic-prep run \
  --workspace ~/AcademicApplications \
  --job jobs/2026-06-15_university-x_lecturer-in-economics \
  --llm-drafts
```

You can combine both switches when provider-backed parsing and drafting are both desired:

```bash
academic-prep run \
  --workspace ~/AcademicApplications \
  --job jobs/2026-06-15_university-x_lecturer-in-economics \
  --llm-parser \
  --llm-drafts
```

The LLM parser must return JSON matching the `parsed_job.json` contract. Invalid JSON, missing required fields, or criteria without source text fail clearly instead of silently inventing data.

The LLM draft generator writes `02_fit_report.md`, `03_cover_letter_draft.md`, `04_cv_tailoring_notes.md`, and `05_criteria_checklist.md`. Draft outputs must cite profile evidence as backticked `profile/generated/file.evidence.md#Section` references; unknown citations fail validation.

To use a non-default profile folder:

```bash
academic-prep run \
  --workspace ~/AcademicApplications \
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
    cover_letter_content.json
    cover_letter.typ
    application_package_content.json
    application_package.typ
```

The default generator remains deterministic and scaffold-level. `--llm-drafts` replaces matching and drafting steps with provider-backed generation while preserving the same file contracts.

Typst generation is structured. `cover_letter_content.json` maps job-specific opening, fit sections, closing, and recipient fields into `modernpro-coverletter`; `cover_letter.typ` reads that data file. It is not a line-by-line Markdown-to-Typst conversion.

### 8. Review and edit generated materials

Review outputs in this order:

1. `parsed_job.json`: confirm title, institution, criteria, required documents, and fields.
2. `05_criteria_checklist.md`: check every essential criterion.
3. `02_fit_report.md`: identify gaps and unsupported claims.
4. `03_cover_letter_draft.md`: edit claims and tone manually.
5. `04_cv_tailoring_notes.md`: apply edits to your private CV source.
6. `06_final_application_package.md`: use as the final preparation dossier.

Generated material is draft-only. Any claim about publications, teaching, service, awards, grants, or supervision must be supported by `profile/` evidence.

### 9. Render Typst outputs when needed

The project uses public Typst Universe templates:

- `@preview/modernpro-cv:1.3.0`
- `@preview/modernpro-coverletter:0.0.8`

Generate PDF outputs only when needed:

```bash
academic-prep render-typst \
  --workspace ~/AcademicApplications \
  --job jobs/2026-06-15_university-x_lecturer-in-economics
```

This requires a local `typst` binary. Source generation does not require Typst; only PDF rendering does.

### 10. Submit manually outside the tool

Before submitting:

- Confirm the university portal's required documents.
- Manually review right-to-work, visa, disability, equality, criminal record, and other sensitive declarations.
- Upload final files yourself.
- Update `job.yaml` status manually if you want local tracking.

The tool prepares application materials. It does not submit anything.

## Privacy Defaults

This repository is intended to be open source. Personal application data should stay local:

- `profile/` is ignored by git except for `.gitkeep`.
- `jobs/` generated job folders are ignored by git.
- `job_leads/` RSS outputs are ignored by git.
- API keys belong in local environment variables or `.env`, which is ignored by git.
- Do not commit real CVs, statements, references, job applications, generated PDFs, or source URLs that reveal private application strategy.

## Typst Templates

Project templates live under `templates/typst/` and import the public modernpro packages. Job-specific generated Typst files are written under each ignored `jobs/<job-slug>/typst/` folder.

The intended direction is:

- Use `modernpro-cv` for CV-style sources and later CV tailoring exports.
- Use `modernpro-coverletter` for cover letters and statement/application package style outputs.
- Keep user-authored CV and statement Typst sources in `profile/typst/`; the pipeline should not rewrite them.
- Generate job-specific `cover_letter_content.json` and `application_package_content.json` for modernpro rendering.
- Keep personal content in ignored local folders.

## Project Skills and Prompts

This repository separates application prompts from agent-readable skills:

- `prompts/` contains LLM prompt files used by the application pipeline.
- `agent-skills/` contains standard `SKILL.md` directories that Codex, Claude Code, Gemini, or another agent can read as project guidance.

The main project skill is:

```text
agent-skills/academic-application-prep/
  SKILL.md
  agents/
    openai.yaml
  references/
    workflow.md
    job-lifecycle.md
    platforms.md
    file-contracts.md
    typst-profile.md
    provider-config.md
    quality-gates.md
    agent-orchestration.md
    privacy.md
```

Agents should load this skill when working on academic application preparation, file contracts, Typst-first profile handling, provider setup, evidence quality gates, agent orchestration, or privacy-sensitive generated materials.

Codex, Claude Code, Gemini, or another local agent should coordinate through the skill and CLI: fetch RSS leads, create a job from a chosen lead, ensure the full advert is present, extract profile evidence, run parser/draft generation, review citations, and optionally render Typst. Agents must not scrape pages, submit applications, or commit private profile/job data.

`platform-bridges/` contains the bridge templates copied into user workspaces. `references/platforms.md` explains how to use the same skill through Codex/AGENTS.md, Claude Code/CLAUDE.md, Gemini CLI/GEMINI.md, and IDE agents that only read project instruction files.

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
   - Use `new-job-from-lead` to initialize a job workspace from selected RSS lead JSON.
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
