# Workflow

Use this reference for the normal installed-package workflow. From a development checkout, prefix commands with `uv run`.

## 1. Initialize Or Inspect Workspace

For a new user workspace:

```bash
canisend init-workspace --workspace <private-workspace>
canisend doctor --workspace <private-workspace>
```

For an existing workspace, start with `doctor` and resolve missing profile, prompt, skill, provider, or Typst items before generating application-facing material.

After package upgrades:

```bash
canisend update-workspace --workspace <private-workspace>
canisend doctor --workspace <private-workspace>
```

Use `--overwrite` only when the user intentionally wants packaged defaults to replace local prompt, template, or skill edits.

## 2. Prepare Profile Evidence

The user should keep real CV and statement sources in ignored `profile/`. In Typst-first workflows, these are already-written `modernpro-cv` and `modernpro-coverletter` sources.

Regenerate normalized evidence whenever profile sources change:

```bash
canisend extract-profile-evidence --workspace <private-workspace>
```

Agents should read generated evidence from `profile/generated/`, not directly rely on prose claims in the private CV. New claims should cite item-level citations such as `profile/generated/cv.evidence.md#Teaching/cv-001`.

If generated evidence is incomplete, first report the gap. Read raw profile sources only with user approval, because in agent-assisted mode the content read by the agent may be processed by the agent model provider. `extract-profile-evidence --llm-augment` must also be explicit opt-in; it rejects augmented items that do not cite a local source chunk.

## 3. Fetch And Select Leads

Fetch jobs.ac.uk RSS leads locally:

```bash
canisend fetch-jobs-ac-uk \
  --workspace <private-workspace> \
  --feed-url "<jobs-ac-uk-rss-url>" \
  --include "<keyword>" \
  --exclude "<keyword>"
```

RSS leads are discovery records, not full adverts. Ask the user to choose a lead index unless they already provided one.

## 4. Create One Job Workspace

From an RSS lead:

```bash
canisend new-job-from-lead \
  --workspace <private-workspace> \
  --lead-index <index> \
  --institution "<institution>" \
  --deadline "YYYY-MM-DD"
```

Manual job creation:

```bash
canisend new-job \
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
canisend run --workspace <private-workspace> --job jobs/<job-slug>
```

LLM-backed parse and draft generation require explicit opt-in and provider configuration:

```bash
canisend run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --llm-parser \
  --llm-drafts
```

Use only `--llm-parser` when the user wants structured parsing but not drafted prose. Use only `--llm-drafts` when deterministic parsing is sufficient.

Always ask before enabling LLM-backed flags or a command provider for a real workspace, because those modes can send selected private advert, profile, evidence, and draft context to the configured provider. If the user has not opted in, run the deterministic baseline and report any gaps for manual review.

## 6. Review Before Rendering

Review, in order:

1. `parsed_job.json`
2. `05_criteria_checklist.md`
3. `02_fit_report.md`
4. `03_cover_letter_draft.md`
5. `04_cv_tailoring_notes.md`
6. `07_material_review_checklist.md`
7. `typst/cover_letter.typ`
8. `typst/application_package.typ`
9. `06_final_application_package.md`

Apply `quality-gates.md` before treating any output as usable.
In particular, check item-level citations, unsupported claims, required-document coverage, and private-file safety before presenting a package as ready.

In agent-assisted mode, also report which private sources were read directly, which LLM-backed CLI flags were used, and which remaining claims need manual confirmation.

## 7. Optional Typst Rendering

Render only when the user asks for PDFs or needs local PDF review:

```bash
canisend render-typst --workspace <private-workspace> --job jobs/<job-slug>
```

Rendering requires a local `typst` binary. Source generation does not.

## 8. Manual Submission

The tool stops at preparation. The user manually handles portal upload, eligibility declarations, equality monitoring, right-to-work, disability, visa, conflict, criminal record, and other sensitive form answers.
