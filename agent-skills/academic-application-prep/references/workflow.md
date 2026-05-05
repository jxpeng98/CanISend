# Workflow

Use this reference for the normal installed-package workflow. From a development checkout, prefix commands with `uv run`.

## 1. Initialize Or Inspect Workspace

For a new user workspace:

```bash
academic-prep init-workspace --workspace <private-workspace>
academic-prep doctor --workspace <private-workspace>
```

For an existing workspace, start with `doctor` and resolve missing profile, prompt, skill, provider, or Typst items before generating application-facing material.

After package upgrades:

```bash
academic-prep update-workspace --workspace <private-workspace>
academic-prep doctor --workspace <private-workspace>
```

Use `--overwrite` only when the user intentionally wants packaged defaults to replace local prompt, template, or skill edits.

## 2. Prepare Profile Evidence

The user should keep real CV and statement sources in ignored `profile/`. In Typst-first workflows, these are already-written `modernpro-cv` and `modernpro-coverletter` sources.

Regenerate normalized evidence whenever profile sources change:

```bash
academic-prep extract-profile-evidence --workspace <private-workspace>
```

Agents should read generated evidence from `profile/generated/`, not directly rely on prose claims in the private CV.

## 3. Fetch And Select Leads

Fetch jobs.ac.uk RSS leads locally:

```bash
academic-prep fetch-jobs-ac-uk \
  --workspace <private-workspace> \
  --feed-url "<jobs-ac-uk-rss-url>" \
  --include "<keyword>" \
  --exclude "<keyword>"
```

RSS leads are discovery records, not full adverts. Ask the user to choose a lead index unless they already provided one.

## 4. Create One Job Workspace

From an RSS lead:

```bash
academic-prep new-job-from-lead \
  --workspace <private-workspace> \
  --lead-index <index> \
  --institution "<institution>" \
  --deadline "YYYY-MM-DD"
```

Manual job creation:

```bash
academic-prep new-job \
  --workspace <private-workspace> \
  --title "<job title>" \
  --institution "<institution>" \
  --deadline "YYYY-MM-DD" \
  --source-url "<source-url>"
```

Paste or import the full selected advert into `jobs/<job-slug>/job_advert.md` before relying on parser output.

## 5. Generate Draft Package

Deterministic baseline:

```bash
academic-prep run --workspace <private-workspace> --job jobs/<job-slug>
```

LLM-backed parse and draft generation require explicit opt-in and provider configuration:

```bash
academic-prep run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --llm-parser \
  --llm-drafts
```

Use only `--llm-parser` when the user wants structured parsing but not drafted prose. Use only `--llm-drafts` when deterministic parsing is sufficient.

## 6. Review Before Rendering

Review, in order:

1. `parsed_job.json`
2. `05_criteria_checklist.md`
3. `02_fit_report.md`
4. `03_cover_letter_draft.md`
5. `04_cv_tailoring_notes.md`
6. `typst/cover_letter_content.json`
7. `06_final_application_package.md`

Apply `quality-gates.md` before treating any output as usable.

## 7. Optional Typst Rendering

Render only when the user asks for PDFs or needs local PDF review:

```bash
academic-prep render-typst --workspace <private-workspace> --job jobs/<job-slug>
```

Rendering requires a local `typst` binary. Source generation does not.

## 8. Manual Submission

The tool stops at preparation. The user manually handles portal upload, eligibility declarations, equality monitoring, right-to-work, disability, visa, conflict, criminal record, and other sensitive form answers.
